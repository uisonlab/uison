//! UIKit: app delegate, scene delegate (UIWindowScene) and our own UIView subclass,
//! registered at runtime via objc_allocateClassPair, like UisonView on macOS.

use std::cell::{Cell, RefCell};
use std::ffi::{CStr, c_char, c_void};
use std::ptr::null_mut;
use std::sync::OnceLock;
use std::time::Instant;

use uison_apple::ffi::*;
use uison_apple::{CgCanvas, Text, class, sel};
use uison_core::{App, Event, Insets, Key, KeyCode, Keyboard, MouseButton, Point, Rect, Size};
use uison_frame::script::{Action, Player};
use uison_frame::{Damage, Runtime};

use crate::host::{IosHost, clipboard};

#[link(name = "UIKit", kind = "framework")]
unsafe extern "C" {
    fn UIApplicationMain(argc: i32, argv: *mut *mut c_char, principal: Id, delegate: Id) -> i32;
    fn UIGraphicsGetCurrentContext() -> CGContextRef;
    static UIKeyboardWillChangeFrameNotification: Id;
    static UIKeyboardFrameEndUserInfoKey: Id;
}

// libdispatch: wakeup from other threads is a task on the main queue.
unsafe extern "C" {
    static _dispatch_main_q: c_void;
    fn dispatch_async_f(queue: *const c_void, ctx: *mut c_void, work: extern "C" fn(*mut c_void));
}

/// On the main thread: `ctx` is our view.
extern "C" fn wake(ctx: *mut c_void) {
    unsafe { state(ctx.cast()).dispatch(Event::Wake) }
}

#[link(name = "QuartzCore", kind = "framework")]
unsafe extern "C" {}

#[link(name = "Foundation", kind = "framework")]
unsafe extern "C" {
    static NSRunLoopCommonModes: Id;
}

/// Start of `run`, to measure "launch → first frame" (UISON_STATS).
static STARTED: OnceLock<Instant> = OnceLock::new();

thread_local! {
    /// The app from `run`, until the scene connects.
    static PENDING: RefCell<Option<Box<dyn App>>> = const { RefCell::new(None) };
}

struct State {
    rt: Runtime,
    text: Text,
    /// The app requested paste; perform it after the event.
    paste: Cell<bool>,
    view: Id,
    /// View size and safe area as known to the app.
    size: Size,
    safe: Insets,
    /// Safe area from the system and how much the on-screen keyboard covers at the bottom:
    /// the app gets the area raised above the keyboard.
    system_safe: Insets,
    keyboard_h: f32,
    /// The keyboard is shown (the view is first responder).
    keyboard: Option<Keyboard>,
    keyboard_sync: bool,
    /// System theme as known to the app.
    dark: Option<bool>,
    /// Highlight redrawn regions (UISON_FLASH=1).
    flashing: bool,
    flash_n: u32,
    /// Print timings (UISON_STATS=1).
    stats: bool,
    pending_since: Option<Instant>,
    /// Regions requested by diff since the last draw (see draw_rect).
    requested: Vec<Rect>,
    requested_full: bool,
    /// Deadline of the nearest scheduled `uisonTick:`.
    tick_at: Option<Instant>,
    /// Frequent ticks (animation, inertia) come from CADisplayLink, in sync with the display
    /// (UISON_TIMER=1 uses a timer, for comparison).
    link: Id,
    link_active: bool,
    /// Intervals between draws during animation (UISON_STATS).
    last_draw: Option<Instant>,
    gaps: Vec<f32>,
    /// Test scenario (UISON_SCRIPT=file, path relative to the .app).
    player: Option<Player>,
}

impl State {
    fn dispatch(&mut self, ev: Event) {
        let t0 = Instant::now();
        let name = ev.name();
        let host = IosHost { text: &self.text, paste: &self.paste };
        match self.rt.dispatch(&ev, &host) {
            Ok(Some(up)) => {
                if up.damage == Damage::Full {
                    self.requested_full = true;
                    unsafe { msg::<(), _>(self.view, sel!("setNeedsDisplay"), ()) };
                } else {
                    for r in up.damage.rects(self.rt.frame.rect()) {
                        self.requested.push(r);
                        unsafe { msg::<(), _>(self.view, sel!("setNeedsDisplayInRect:"), (CGRect::from(r),)) };
                    }
                }
                if up.damage != Damage::None && self.pending_since.is_none() {
                    self.pending_since = Some(t0);
                }
                if self.stats {
                    let frame = self.rt.frame.rect();
                    eprintln!(
                        "{name}: app {}us, parse {}us ({} B), diff {}us → {} rect(s), {}%",
                        up.app.as_micros(),
                        up.load.as_micros(),
                        up.bytes,
                        up.diff.as_micros(),
                        up.damage.rects(frame).count(),
                        (up.damage.ratio(frame) * 100.0).round() as u32,
                    );
                }
            }
            Ok(None) => {}
            Err(e) => eprintln!("uison: bad frame: {e}"),
        }
        if self.paste.take()
            && let Some(t) = clipboard()
        {
            self.dispatch(Event::Paste(t.into()));
        }
        self.schedule_tick();
        // The keyboard is handled after the event, deferred: showing it sends notifications,
        // and their handler calls dispatch again.
        if self.rt.app.wants_keyboard() != self.keyboard && !self.keyboard_sync {
            self.keyboard_sync = true;
            after(self.view, sel!("uisonKeyboardSync:"), 0.0);
        }
    }

    /// Area for the app: the system one, and at the bottom at least the keyboard height.
    fn update_safe(&mut self) {
        let mut safe = self.system_safe;
        safe.bottom = safe.bottom.max(self.keyboard_h);
        if safe != self.safe {
            self.safe = safe;
            self.dispatch(Event::SafeArea(safe));
        }
    }

    /// As on macOS: a new deadline earlier than the scheduled one means one more delayed call.
    fn schedule_tick(&mut self) {
        let Some(d) = self.rt.app.wants_tick() else { return };
        if d <= FRAME_TICK && !self.link.is_null() {
            if !self.link_active {
                self.link_active = true;
                unsafe { msg::<(), _>(self.link, sel!("setPaused:"), (false,)) };
            }
            return;
        }
        let at = Instant::now() + d;
        if self.tick_at.is_some_and(|t| t <= at) {
            return;
        }
        self.tick_at = Some(at);
        unsafe {
            msg::<(), _>(self.view, sel!("performSelector:withObject:afterDelay:"), (sel!("uisonTick:"), NIL, d.as_secs_f64()));
        }
    }

    /// Animation ended: intervals between frames (p50, p95, max, how many longer than 20 ms).
    fn report_gaps(&mut self) {
        let wants_frames = self.rt.app.wants_tick().is_some_and(|d| d <= FRAME_TICK);
        if !self.stats || wants_frames || self.gaps.len() < 5 {
            return;
        }
        let mut g = std::mem::take(&mut self.gaps);
        g.sort_by(f32::total_cmp);
        let at = |q: f32| g[((g.len() - 1) as f32 * q) as usize];
        let slow = g.iter().filter(|&&x| x > 20.0).count();
        eprintln!(
            "animation: {} frames, gap p50 {:.1} ms, p95 {:.1} ms, max {:.1} ms, >20 ms: {slow}",
            g.len(),
            at(0.5),
            at(0.95),
            g[g.len() - 1]
        );
    }
}

/// Ticks more frequent than this come from CADisplayLink (display frames).
const FRAME_TICK: std::time::Duration = std::time::Duration::from_millis(20);

// ---------------------------------------------------------------- classes

struct Classes {
    view: usize,
    state_offset: isize,
    window_offset: isize,
}

fn classes() -> &'static Classes {
    static C: OnceLock<Classes> = OnceLock::new();
    C.get_or_init(|| unsafe { register_classes() })
}

unsafe fn add(cls: Class, sel: Sel, imp: *const c_void, types: &CStr) {
    let ok = unsafe { class_addMethod(cls, sel, imp, types.as_ptr()) };
    assert!(ok, "class_addMethod failed");
}

unsafe fn protocol(cls: Class, name: &CStr) {
    unsafe {
        let p = objc_getProtocol(name.as_ptr());
        if !p.is_null() {
            class_addProtocol(cls, p);
        }
    }
}

unsafe fn ivar_offset(cls: Class, name: &CStr) -> isize {
    unsafe { ivar_getOffset(class_getInstanceVariable(cls, name.as_ptr())) }
}

unsafe fn register_classes() -> Classes {
    type Touches = extern "C" fn(Id, Sel, Id, Id);
    type Ev = extern "C" fn(Id, Sel, Id);
    type V = extern "C" fn(Id, Sel);
    type B = extern "C" fn(Id, Sel) -> bool;
    type Q = extern "C" fn(Id, Sel) -> isize;
    unsafe {
        let view = objc_allocateClassPair(class!("UIView"), c"UisonView".as_ptr(), 0);
        assert!(!view.is_null(), "UisonView already registered");
        class_addIvar(view, c"uison_state".as_ptr(), 8, 3, c"^v".as_ptr());
        add(view, sel!("drawRect:"), draw_rect as extern "C" fn(Id, Sel, CGRect) as _, c"v@:{CGRect={CGPoint=dd}{CGSize=dd}}");
        add(view, sel!("layoutSubviews"), layout_subviews as V as _, c"v@:");
        add(view, sel!("safeAreaInsetsDidChange"), safe_area_changed as V as _, c"v@:");
        add(view, sel!("traitCollectionDidChange:"), trait_changed as Ev as _, c"v@:@");
        add(view, sel!("isOpaque"), yes as B as _, c"B@:");
        add(view, sel!("touchesBegan:withEvent:"), touches_began as Touches as _, c"v@:@@");
        add(view, sel!("touchesMoved:withEvent:"), touches_moved as Touches as _, c"v@:@@");
        add(view, sel!("touchesEnded:withEvent:"), touches_ended as Touches as _, c"v@:@@");
        add(view, sel!("touchesCancelled:withEvent:"), touches_cancelled as Touches as _, c"v@:@@");
        add(view, sel!("uisonTick:"), tick as Ev as _, c"v@:@");
        add(view, sel!("uisonFrame:"), display_frame as Ev as _, c"v@:@");
        add(view, sel!("uisonStep:"), script_step as Ev as _, c"v@:@");
        // On-screen keyboard: UIKeyInput (+ UITextInputTraits) and notifications about its frame.
        protocol(view, c"UIKeyInput");
        add(view, sel!("canBecomeFirstResponder"), yes as B as _, c"B@:");
        add(view, sel!("hasText"), yes as B as _, c"B@:");
        add(view, sel!("insertText:"), insert_text as Ev as _, c"v@:@");
        add(view, sel!("deleteBackward"), delete_backward as V as _, c"v@:");
        add(view, sel!("keyboardType"), keyboard_type as Q as _, c"q@:");
        add(view, sel!("autocapitalizationType"), autocapitalization as Q as _, c"q@:");
        add(view, sel!("autocorrectionType"), no_correction as Q as _, c"q@:");
        add(view, sel!("spellCheckingType"), no_correction as Q as _, c"q@:");
        add(view, sel!("isSecureTextEntry"), secure as B as _, c"B@:");
        add(view, sel!("uisonKeyboardSync:"), keyboard_sync as Ev as _, c"v@:@");
        add(view, sel!("uisonKeyboardFrame:"), keyboard_frame as Ev as _, c"v@:@");
        objc_registerClassPair(view);
        let state_offset = ivar_offset(view, c"uison_state");

        let app = objc_allocateClassPair(class!("UIResponder"), c"UisonAppDelegate".as_ptr(), 0);
        protocol(app, c"UIApplicationDelegate");
        add(
            app,
            sel!("application:didFinishLaunchingWithOptions:"),
            did_finish as extern "C" fn(Id, Sel, Id, Id) -> bool as _,
            c"B@:@@",
        );
        objc_registerClassPair(app);

        // Scene delegate: creates the window for the UIWindowScene and holds it (the `window` property).
        let scene = objc_allocateClassPair(class!("UIResponder"), c"UisonSceneDelegate".as_ptr(), 0);
        protocol(scene, c"UIWindowSceneDelegate");
        class_addIvar(scene, c"uison_window".as_ptr(), 8, 3, c"@".as_ptr());
        add(
            scene,
            sel!("scene:willConnectToSession:options:"),
            will_connect as extern "C" fn(Id, Sel, Id, Id, Id) as _,
            c"v@:@@@",
        );
        add(scene, sel!("window"), scene_window as extern "C" fn(Id, Sel) -> Id as _, c"@@:");
        add(scene, sel!("setWindow:"), set_scene_window as Ev as _, c"v@:@");
        objc_registerClassPair(scene);
        let window_offset = ivar_offset(scene, c"uison_window");

        Classes { view: view as usize, state_offset, window_offset }
    }
}

unsafe fn state<'a>(view: Id) -> &'a mut State {
    unsafe {
        let slot = view.cast::<u8>().offset(classes().state_offset).cast::<*mut State>();
        &mut **slot
    }
}

unsafe fn window_slot(delegate: Id) -> *mut Id {
    unsafe { delegate.cast::<u8>().offset(classes().window_offset).cast::<Id>() }
}

extern "C" fn yes(_: Id, _: Sel) -> bool {
    true
}

// ---------------------------------------------------------------- app and scene

extern "C" fn did_finish(_: Id, _: Sel, _: Id, _: Id) -> bool {
    true
}

extern "C" fn scene_window(this: Id, _: Sel) -> Id {
    unsafe { *window_slot(this) }
}

extern "C" fn set_scene_window(this: Id, _: Sel, w: Id) {
    unsafe { *window_slot(this) = w }
}

fn env_flag(name: &str) -> bool {
    std::env::var(name).is_ok_and(|v| v != "0" && !v.is_empty())
}

extern "C" fn will_connect(this: Id, _: Sel, scene: Id, _: Id, _: Id) {
    let Some(app) = PENDING.with(|p| p.borrow_mut().take()) else {
        eprintln!("uison: no app — call uison::run(app, title) in main()");
        return;
    };
    let cls = classes();
    unsafe {
        let t0 = Instant::now();
        let text = Text::new();
        let paste = Cell::new(false);
        let rt = Runtime::new(app, &IosHost { text: &text, paste: &paste }).unwrap_or_else(|e| {
            eprintln!("uison: bad frame: {e}");
            std::process::exit(1)
        });
        let load_us = t0.elapsed().as_micros();

        let window: Id = msg(class!("UIWindow"), sel!("alloc"), ());
        let window: Id = msg(window, sel!("initWithWindowScene:"), (scene,));
        let bounds: CGRect = msg(window, sel!("bounds"), ());

        let view: Id = msg(cls.view as Class, sel!("alloc"), ());
        let view: Id = msg(view, sel!("initWithFrame:"), (bounds,));
        // White background under the frame if it is smaller than the screen.
        let white: Id = msg(class!("UIColor"), sel!("whiteColor"), ());
        msg::<(), _>(view, sel!("setBackgroundColor:"), (white,));

        let st = Box::new(State {
            size: rt.frame.size,
            safe: Insets::default(),
            system_safe: Insets::default(),
            keyboard_h: 0.0,
            keyboard: None,
            keyboard_sync: false,
            dark: None,
            rt,
            text,
            paste,
            view,
            flashing: env_flag("UISON_FLASH"),
            flash_n: 0,
            stats: env_flag("UISON_STATS"),
            pending_since: None,
            requested: Vec::new(),
            requested_full: false,
            tick_at: None,
            link: NIL,
            link_active: false,
            last_draw: None,
            gaps: Vec::new(),
            player: load_script(),
        });
        if st.stats {
            eprintln!("first frame: {} items, app + load {load_us}us", st.rt.frame.items.len());
        }
        *view.cast::<u8>().offset(cls.state_offset).cast::<*mut State>() = Box::into_raw(st);
        // Wakeup for Handle::post from other threads (the view lives until the process ends).
        let view_addr = view as usize;
        state(view).rt.app.set_waker(std::sync::Arc::new(move || {
            dispatch_async_f(&raw const _dispatch_main_q, view_addr as *mut c_void, wake)
        }));

        if !env_flag("UISON_TIMER") {
            let link: Id = msg(class!("CADisplayLink"), sel!("displayLinkWithTarget:selector:"), (view, sel!("uisonFrame:")));
            msg::<(), _>(link, sel!("setPaused:"), (true,));
            let rl: Id = msg(class!("NSRunLoop"), sel!("mainRunLoop"), ());
            msg::<(), _>(link, sel!("addToRunLoop:forMode:"), (rl, NSRunLoopCommonModes));
            state(view).link = link;
        }
        let nc: Id = msg(class!("NSNotificationCenter"), sel!("defaultCenter"), ());
        msg::<(), _>(
            nc,
            sel!("addObserver:selector:name:object:"),
            (view, sel!("uisonKeyboardFrame:"), UIKeyboardWillChangeFrameNotification, NIL),
        );

        let vc: Id = msg(class!("UIViewController"), sel!("alloc"), ());
        let vc: Id = msg(vc, sel!("init"), ());
        msg::<(), _>(vc, sel!("setView:"), (view,));
        msg::<(), _>(window, sel!("setRootViewController:"), (vc,));
        release(vc);
        release(view);
        *window_slot(this) = window; // +1 from alloc: the window lives as long as the scene
        msg::<(), _>(window, sel!("makeKeyAndVisible"), ());
        if state(view).player.is_some() {
            after(view, sel!("uisonStep:"), 1.0); // let the app show up first
        }
    }
}

fn after(view: Id, sel: Sel, secs: f64) {
    unsafe { msg::<(), _>(view, sel!("performSelector:withObject:afterDelay:"), (sel, NIL, secs)) }
}

/// UISON_SCRIPT: JSON scenario (see `uison_frame::script`); a relative path is from the .app folder.
fn load_script() -> Option<Player> {
    let name = std::env::var("UISON_SCRIPT").ok().filter(|s| !s.is_empty())?;
    let path = match std::env::current_exe() {
        Ok(exe) if !name.starts_with('/') => exe.with_file_name(&name),
        _ => name.into(),
    };
    let src = std::fs::read_to_string(&path).map_err(|e| eprintln!("uison: script {}: {e}", path.display())).ok()?;
    Player::new(&src, true).map_err(|e| eprintln!("uison: {e}")).ok()
}

/// Scenario step: an action goes the same path as real touches, then a timer until the next step.
extern "C" fn script_step(this: Id, _: Sel, _: Id) {
    unsafe {
        let st = state(this);
        let Some(player) = st.player.as_mut() else { return };
        let Some((action, pause)) = player.next(&st.rt.frame) else { return };
        match action {
            Action::Event(e) => st.dispatch(e),
            Action::Capture(name) => eprintln!("uison: capture {name}"),
            Action::Log(m) => eprintln!("uison: {m}"),
            Action::Wait => {}
            Action::Done => {
                // Scenarios are for testing: exit so the outside script finishes reading the output.
                eprintln!("uison: done");
                std::process::exit(0);
            }
        }
        after(this, sel!("uisonStep:"), pause.as_secs_f64());
    }
}

// ---------------------------------------------------------------- drawing and size

extern "C" fn layout_subviews(this: Id, _: Sel) {
    unsafe {
        msg_super::<(), _>(this, class!("UIView"), sel!("layoutSubviews"), ());
        let b: CGRect = msg(this, sel!("bounds"), ());
        let size = Size::new(b.size.width as f32, b.size.height as f32);
        let st = state(this);
        if size != st.size && size.w > 0.0 && size.h > 0.0 {
            st.size = size;
            st.dispatch(Event::Resize(size));
        }
        let i: [f64; 4] = msg(this, sel!("safeAreaInsets"), ()); // UIEdgeInsets: top, left, bottom, right
        check_appearance(this);
        let st = state(this);
        st.system_safe = Insets { top: i[0] as f32, left: i[1] as f32, bottom: i[2] as f32, right: i[3] as f32 };
        st.update_safe();
    }
}

/// Whether the system theme is dark (UIUserInterfaceStyle: 2 = dark) → Event::Appearance on change.
fn check_appearance(this: Id) {
    unsafe {
        let traits: Id = msg(this, sel!("traitCollection"), ());
        let style: isize = msg(traits, sel!("userInterfaceStyle"), ());
        let dark = style == 2;
        let st = state(this);
        if st.dark != Some(dark) {
            st.dark = Some(dark);
            st.dispatch(Event::Appearance(dark));
        }
    }
}

extern "C" fn trait_changed(this: Id, _: Sel, prev: Id) {
    unsafe { msg_super::<(), _>(this, class!("UIView"), sel!("traitCollectionDidChange:"), (prev,)) };
    check_appearance(this);
}

/// The area changes even without a size change (status bar on rotation); recompute the layout.
extern "C" fn safe_area_changed(this: Id, _: Sel) {
    unsafe {
        msg_super::<(), _>(this, class!("UIView"), sel!("safeAreaInsetsDidChange"), ());
        msg::<(), _>(this, sel!("setNeedsLayout"), ());
    }
}

extern "C" fn draw_rect(this: Id, _: Sel, dirty: CGRect) {
    let t0 = Instant::now();
    unsafe {
        let st = state(this);
        let ctx = UIGraphicsGetCurrentContext();
        let asked = Rect::from(dirty);
        // As on macOS: if UIKit requests the bounding rectangle of our regions,
        // draw only them (experiment 1: whether the rest of the layer content is kept).
        let bbox = st.requested.iter().fold(Rect::default(), |acc, r| acc.union(r)).inflate(0.5);
        let own = !st.requested_full && !st.requested.is_empty() && bbox.covers(&asked);
        let rects: Vec<Rect> = if own {
            st.requested.iter().map(|r| r.intersect(&asked)).filter(|r| !r.is_empty()).collect()
        } else {
            vec![asked]
        };
        st.requested.clear();
        st.requested_full = false;
        if st.stats {
            if let Some(t) = st.last_draw.filter(|t| t.elapsed().as_millis() < 100) {
                st.gaps.push(t.elapsed().as_secs_f32() * 1000.0);
            }
            st.last_draw = Some(t0);
        }
        let mut canvas = CgCanvas::new(ctx, &st.text);
        let mut area = 0.0;
        for &r in &rects {
            area += r.intersect(&st.rt.frame.rect()).area();
            uison_frame::paint(&st.rt.frame, &mut canvas, r);
            if st.flashing {
                st.flash_n = st.flash_n.wrapping_add(1);
                let hue = [(255, 60, 60), (60, 200, 60), (60, 120, 255), (230, 180, 0)][st.flash_n as usize % 4];
                use uison_core::Canvas;
                canvas.rect(r, 0.0, uison_core::Paint::fill(uison_core::Color::rgba(hue.0, hue.1, hue.2, 70)));
            }
        }
        static FIRST: std::sync::Once = std::sync::Once::new();
        if st.stats && !FIRST.is_completed() {
            FIRST.call_once(|| {});
            eprintln!("launch → first draw: {} ms", STARTED.get().map_or(0, |t| t.elapsed().as_millis()));
        }
        if st.stats {
            let latency = st.pending_since.take().map(|t| t.elapsed().as_micros());
            eprintln!(
                "draw: {} rect(s), {}% of frame, {}us{}",
                rects.len(),
                (100.0 * area / st.rt.frame.rect().area().max(1.0)).round() as u32,
                t0.elapsed().as_micros(),
                latency.map(|l| format!(", event→drawn {l}us")).unwrap_or_default(),
            );
        }
    }
}

// ---------------------------------------------------------------- touches

/// Touch → pointer events with `MouseButton::Touch`. The view accepts one touch (multipleTouchEnabled = NO).
fn touch(this: Id, touches: Id, f: impl FnOnce(Point) -> Event) {
    unsafe {
        let t: Id = msg(touches, sel!("anyObject"), ());
        if t.is_null() {
            return;
        }
        let p: CGPoint = msg(t, sel!("locationInView:"), (this,));
        state(this).dispatch(f(Point::new(p.x as f32, p.y as f32)));
    }
}

extern "C" fn touches_began(this: Id, _: Sel, touches: Id, _: Id) {
    touch(this, touches, |p| Event::MouseDown(p, MouseButton::Touch));
}

extern "C" fn touches_moved(this: Id, _: Sel, touches: Id, _: Id) {
    touch(this, touches, Event::MouseMove);
}

extern "C" fn touches_ended(this: Id, _: Sel, touches: Id, _: Id) {
    touch(this, touches, |p| Event::MouseUp(p, MouseButton::Touch));
}

extern "C" fn touches_cancelled(this: Id, _: Sel, _: Id, _: Id) {
    // The system took the touch away: release outside the view, no click.
    unsafe { state(this).dispatch(Event::MouseUp(Point::new(-1.0, -1.0), MouseButton::Touch)) }
}

// ---------------------------------------------------------------- keyboard

extern "C" fn keyboard_sync(this: Id, _: Sel, _: Id) {
    unsafe {
        let st = state(this);
        st.keyboard_sync = false;
        let want = st.rt.app.wants_keyboard();
        let was = std::mem::replace(&mut st.keyboard, want);
        match (was, want) {
            (None, Some(_)) => {
                msg::<bool, _>(this, sel!("becomeFirstResponder"), ());
            }
            (Some(_), None) => {
                msg::<bool, _>(this, sel!("resignFirstResponder"), ());
            }
            // Another field with another keyboard: re-read the properties (keyboardType…).
            (Some(a), Some(b)) if a != b => msg::<(), _>(this, sel!("reloadInputViews"), ()),
            _ => {}
        }
    }
}

/// Keyboard frame (in screen coordinates) → how much it covers at the bottom.
extern "C" fn keyboard_frame(this: Id, _: Sel, note: Id) {
    unsafe {
        let info: Id = msg(note, sel!("userInfo"), ());
        let value: Id = msg(info, sel!("objectForKey:"), (UIKeyboardFrameEndUserInfoKey,));
        if value.is_null() {
            return;
        }
        let kb: CGRect = msg(value, sel!("CGRectValue"), ());
        let kb: CGRect = msg(this, sel!("convertRect:fromView:"), (kb, NIL));
        let b: CGRect = msg(this, sel!("bounds"), ());
        let covered = (b.origin.y + b.size.height - kb.origin.y).clamp(0.0, b.size.height) as f32;
        let st = state(this);
        st.keyboard_h = covered;
        st.update_safe();
    }
}

extern "C" fn insert_text(this: Id, _: Sel, text: Id) {
    unsafe {
        let c: *const c_char = msg(text, sel!("UTF8String"), ());
        if c.is_null() {
            return;
        }
        let s = CStr::from_ptr(c).to_string_lossy();
        let st = state(this);
        for part in s.split_inclusive('\n') {
            let (t, enter) = part.strip_suffix('\n').map_or((part, false), |t| (t, true));
            if !t.is_empty() {
                st.dispatch(Event::Key(Key::char(t)));
            }
            if enter {
                st.dispatch(Event::Key(Key::new(KeyCode::Enter)));
            }
        }
    }
}

extern "C" fn delete_backward(this: Id, _: Sel) {
    unsafe { state(this).dispatch(Event::Key(Key::new(KeyCode::Backspace))) }
}

fn current_keyboard(this: Id) -> Keyboard {
    unsafe { state(this).rt.app.wants_keyboard().unwrap_or_default() }
}

/// UIKeyboardType.
extern "C" fn keyboard_type(this: Id, _: Sel) -> isize {
    match current_keyboard(this) {
        Keyboard::Text | Keyboard::Password => 0,
        Keyboard::Url => 3,
        Keyboard::Number => 4,
        Keyboard::Phone => 5,
        Keyboard::Email => 7,
        Keyboard::Decimal => 8,
    }
}

/// UITextAutocapitalizationType: sentences for text, otherwise no capitals.
extern "C" fn autocapitalization(this: Id, _: Sel) -> isize {
    if current_keyboard(this) == Keyboard::Text { 2 } else { 0 }
}

/// Autocorrection can't work without UITextInput, so it is off.
extern "C" fn no_correction(_: Id, _: Sel) -> isize {
    1
}

extern "C" fn secure(this: Id, _: Sel) -> bool {
    current_keyboard(this) == Keyboard::Password
}

extern "C" fn tick(this: Id, _: Sel, _: Id) {
    unsafe {
        let st = state(this);
        st.tick_at = None;
        st.dispatch(Event::Tick);
        st.report_gaps();
    }
}

/// Display frame (CADisplayLink): an animation tick; pause when no longer needed.
extern "C" fn display_frame(this: Id, _: Sel, _: Id) {
    unsafe {
        let st = state(this);
        st.link_active = false;
        st.dispatch(Event::Tick);
        if !st.link_active {
            msg::<(), _>(st.link, sel!("setPaused:"), (true,));
            st.report_gaps();
        }
    }
}

// ---------------------------------------------------------------- startup

/// Store the app and enter UIApplicationMain. Does not return.
/// The window is created when the system connects the scene (`scene:willConnectToSession:options:`).
pub fn run(app: impl App + 'static, _title: &str) {
    STARTED.get_or_init(Instant::now);
    classes();
    PENDING.with(|p| *p.borrow_mut() = Some(Box::new(app)));
    unsafe {
        let delegate = nsstring("UisonAppDelegate");
        UIApplicationMain(0, null_mut(), NIL, delegate);
    }
}
