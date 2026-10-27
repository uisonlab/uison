//! AppKit window: NSApplication + NSWindow + our own NSView subclass,
//! registered at runtime via objc_allocateClassPair.

use std::cell::{Cell, RefCell};
use std::ffi::{CStr, c_char, c_void};
use std::ptr::null;
use std::sync::OnceLock;
use std::time::Instant;

use uison_core::{App, Cursor, Event, Key, KeyCode, MouseButton, Point, Rect, Size};
use uison_frame::script::{Action, Player};
use uison_frame::{Damage, Runtime};

use uison_apple::ffi::*;
use uison_apple::{CgCanvas, Text, class, save_image_png, sel};

use crate::host::{MacHost, clipboard, mods};

#[link(name = "AppKit", kind = "framework")]
unsafe extern "C" {}

#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    /// Window screenshot. Our own window can be captured without screen recording permission.
    fn CGWindowListCreateImage(bounds: CGRect, options: u32, window: u32, image_options: u32) -> CFTypeRef;
}

// libdispatch (in libSystem): wakeup from other threads is a task on the main queue.
unsafe extern "C" {
    static _dispatch_main_q: c_void;
    fn dispatch_async_f(queue: *const c_void, ctx: *mut c_void, work: extern "C" fn(*mut c_void));
}

/// On the main thread: `ctx` is our view.
extern "C" fn wake(ctx: *mut c_void) {
    send(ctx.cast(), Event::Wake);
}

thread_local! {
    /// The app is handling an event. A system modal loop inside a handler (NSOpenPanel,
    /// NSAlert) delivers its events to the window; they wait here until handling ends instead of
    /// re-entering the app (a second `&mut State` on top of the first).
    static BUSY: Cell<bool> = const { Cell::new(false) };
    static DEFERRED: RefCell<Vec<Event>> = const { RefCell::new(Vec::new()) };
}

/// An event to the app, or into the queue if it is busy with the previous one.
fn send(view: Id, ev: Event) {
    if BUSY.get() {
        DEFERRED.with_borrow_mut(|d| match (d.last_mut(), &ev) {
            // Of mouse moves only the last one matters.
            (Some(Event::MouseMove(last)), Event::MouseMove(p)) => *last = *p,
            _ => d.push(ev),
        });
        return;
    }
    BUSY.set(true);
    let mut next = Some(ev);
    while let Some(ev) = next {
        unsafe { state(view).dispatch(ev) };
        next = DEFERRED.with_borrow_mut(|d| (!d.is_empty()).then(|| d.remove(0)));
    }
    BUSY.set(false);
}

const LIST_INCLUDING_WINDOW: u32 = 1 << 3;
const IMAGE_IGNORE_FRAMING: u32 = 1 << 0;

struct State {
    rt: Runtime,
    text: Text,
    /// The app requested paste (`Host::request_paste`); perform it after the event.
    paste: Cell<bool>,
    view: Id,
    /// Highlight redrawn regions (UISON_FLASH=1).
    flashing: bool,
    flash_n: u32,
    /// Print timings (UISON_STATS=1).
    stats: bool,
    /// When handling of the last event started, for latency up to the end of drawing.
    pending_since: Option<Instant>,
    /// Regions requested by diff since the last draw. AppKit merges them
    /// into one bounding rectangle; we draw only them (see draw_rect).
    requested: Vec<Rect>,
    requested_full: bool,
    /// Current mouse cursor shape.
    cursor: Cursor,
    /// Deadline of the nearest scheduled `uisonTick:`.
    tick_at: Option<Instant>,
    /// File for the hit area map (UISON_HITMAP).
    hitmap: Option<String>,
    /// Save a window screenshot (as WindowServer shows it) after draws (UISON_CAPTURE=folder).
    capture: Option<Capture>,
    /// Test scenario (UISON_SCRIPT=file).
    player: Option<Player>,
}

struct Capture {
    dir: String,
    n: u32,
    scheduled: bool,
    /// Screenshot from a scenario step: a file name instead of a number.
    named: Option<String>,
}

impl State {
    fn dispatch(&mut self, ev: Event) {
        if ev == Event::Tick {
            self.tick_at = None;
        }
        let pos = ev.pos();
        let leave = ev == Event::MouseLeave;
        self.apply(ev);
        if self.paste.take()
            && let Some(t) = clipboard()
        {
            self.apply(Event::Paste(t.into()));
        }
        if let Some(p) = pos {
            self.set_cursor(uison_frame::cursor_at(&self.rt.frame, p));
        } else if leave {
            self.set_cursor(Cursor::Arrow);
        }
        self.schedule_tick();
    }

    fn set_cursor(&mut self, c: Cursor) {
        if c == self.cursor {
            return;
        }
        self.cursor = c;
        unsafe {
            let cls = class!("NSCursor");
            let cursor: Id = match c {
                Cursor::Arrow => msg(cls, sel!("arrowCursor"), ()),
                Cursor::Pointer => msg(cls, sel!("pointingHandCursor"), ()),
                Cursor::Text => msg(cls, sel!("IBeamCursor"), ()),
            };
            msg::<(), _>(cursor, sel!("set"), ());
        }
    }

    /// The app asks for a timer: a delayed `uisonTick:` call. If the new deadline is
    /// earlier than the scheduled one (animation while the caret blinks), schedule one more;
    /// an extra tick is harmless: the app checks the time itself.
    fn schedule_tick(&mut self) {
        let Some(d) = self.rt.app.wants_tick() else { return };
        let at = Instant::now() + d;
        if self.tick_at.is_some_and(|t| t <= at) {
            return;
        }
        self.tick_at = Some(at);
        unsafe {
            msg::<(), _>(self.view, sel!("performSelector:withObject:afterDelay:"), (sel!("uisonTick:"), NIL, d.as_secs_f64()));
        }
    }

    /// UISON_HITMAP=file: hit area map of the current frame (`id x y w h` per line),
    /// so automation clicks elements by id rather than by coordinates.
    fn write_hitmap(&self) {
        if let Some(path) = &self.hitmap {
            let mut s = String::new();
            // Whole points: f32 formatting from std weighs ~16 KB.
            let i = |v: f32| v.round() as i64;
            for (id, r) in uison_frame::hit_areas(&self.rt.frame) {
                s.push_str(&format!("{id}\t{}\t{}\t{}\t{}\n", i(r.x), i(r.y), i(r.w), i(r.h)));
            }
            let _ = std::fs::write(path, s);
        }
    }

    fn apply(&mut self, ev: Event) {
        let t0 = Instant::now();
        let host = MacHost { text: &self.text, paste: &self.paste };
        let up = match self.rt.dispatch(&ev, &host) {
            Ok(Some(up)) => up,
            Ok(None) => return,
            Err(e) => {
                eprintln!("uison: bad frame: {e}");
                return;
            }
        };
        self.write_hitmap();
        // Full is the whole view: it can be larger than the frame (the strip under the title bar).
        if up.damage == Damage::Full {
            self.requested_full = true;
            unsafe { msg::<(), _>(self.view, sel!("setNeedsDisplay:"), (true,)) };
        } else {
            for r in up.damage.rects(self.rt.frame.rect()) {
                self.requested.push(r);
                unsafe { msg::<(), _>(self.view, sel!("setNeedsDisplayInRect:"), (CGRect::from(r),)) };
            }
        }
        if up.damage != Damage::None {
            self.pending_since = Some(t0);
        }
        if self.stats {
            let frame = self.rt.frame.rect();
            eprintln!(
                "{}: app {}us, parse {}us ({} B), diff {}us → {} rect(s), {}%",
                ev.name(),
                up.app.as_micros(),
                up.load.as_micros(),
                up.bytes,
                up.diff.as_micros(),
                up.damage.rects(frame).count(),
                percent(up.damage.ratio(frame)),
            );
        }
    }
}

/// Share in whole percent: statistics without f32 formatting (it weighs ~16 KB).
fn percent(ratio: f32) -> u32 {
    (ratio * 100.0).round() as u32
}

// ---------------------------------------------------------------- classes

struct Classes {
    view: usize,
    delegate: usize,
    state_offset: isize,
}

fn classes() -> &'static Classes {
    static C: OnceLock<Classes> = OnceLock::new();
    C.get_or_init(|| unsafe { register_classes() })
}

unsafe fn add(cls: Class, sel: Sel, imp: *const c_void, types: &CStr) {
    let ok = unsafe { class_addMethod(cls, sel, imp, types.as_ptr()) };
    assert!(ok, "class_addMethod failed");
}

unsafe fn register_classes() -> Classes {
    type Ev = extern "C" fn(Id, Sel, Id);
    type B = extern "C" fn(Id, Sel) -> bool;
    unsafe {
        let view = objc_allocateClassPair(class!("NSView"), c"UisonView".as_ptr(), 0);
        assert!(!view.is_null(), "UisonView already registered");
        class_addIvar(view, c"uison_state".as_ptr(), 8, 3, c"^v".as_ptr());
        add(view, sel!("drawRect:"), draw_rect as extern "C" fn(Id, Sel, CGRect) as _, c"v@:{CGRect={CGPoint=dd}{CGSize=dd}}");
        add(view, sel!("isFlipped"), yes as B as _, c"B@:");
        add(view, sel!("isOpaque"), yes as B as _, c"B@:");
        add(view, sel!("acceptsFirstResponder"), yes as B as _, c"B@:");
        add(view, sel!("acceptsFirstMouse:"), yes1 as extern "C" fn(Id, Sel, Id) -> bool as _, c"B@:@");
        add(view, sel!("mouseDown:"), mouse_down as Ev as _, c"v@:@");
        add(view, sel!("mouseUp:"), mouse_up as Ev as _, c"v@:@");
        add(view, sel!("rightMouseDown:"), right_down as Ev as _, c"v@:@");
        add(view, sel!("rightMouseUp:"), right_up as Ev as _, c"v@:@");
        add(view, sel!("mouseMoved:"), mouse_moved as Ev as _, c"v@:@");
        add(view, sel!("mouseDragged:"), mouse_moved as Ev as _, c"v@:@");
        add(view, sel!("mouseExited:"), mouse_exited as Ev as _, c"v@:@");
        add(view, sel!("scrollWheel:"), scroll_wheel as Ev as _, c"v@:@");
        add(view, sel!("keyDown:"), key_down as Ev as _, c"v@:@");
        add(view, sel!("uisonCapture:"), capture_window as Ev as _, c"v@:@");
        add(view, sel!("uisonTick:"), tick as Ev as _, c"v@:@");
        add(view, sel!("uisonStep:"), script_step as Ev as _, c"v@:@");
        add(view, sel!("viewDidChangeEffectiveAppearance"), appearance_changed as extern "C" fn(Id, Sel) as _, c"v@:");
        objc_registerClassPair(view);
        let ivar = class_getInstanceVariable(view, c"uison_state".as_ptr());
        let state_offset = ivar_getOffset(ivar);

        let delegate = objc_allocateClassPair(class!("NSObject"), c"UisonDelegate".as_ptr(), 0);
        add(
            delegate,
            sel!("applicationShouldTerminateAfterLastWindowClosed:"),
            yes1 as extern "C" fn(Id, Sel, Id) -> bool as _,
            c"B@:@",
        );
        add(delegate, sel!("windowDidResize:"), window_did_resize as Ev as _, c"v@:@");
        objc_registerClassPair(delegate);

        Classes { view: view as usize, delegate: delegate as usize, state_offset }
    }
}

unsafe fn state<'a>(view: Id) -> &'a mut State {
    unsafe {
        let slot = view.cast::<u8>().offset(classes().state_offset).cast::<*mut State>();
        &mut **slot
    }
}

extern "C" fn yes(_: Id, _: Sel) -> bool {
    true
}

extern "C" fn yes1(_: Id, _: Sel, _: Id) -> bool {
    true
}

// ---------------------------------------------------------------- drawing

extern "C" fn draw_rect(this: Id, _: Sel, _dirty: CGRect) {
    let t0 = Instant::now();
    unsafe {
        let st = state(this);
        let gc: Id = msg(class!("NSGraphicsContext"), sel!("currentContext"), ());
        let ctx: CGContextRef = msg(gc, sel!("CGContext"), ());
        let mut rects: *const CGRect = null();
        let mut n: isize = 0;
        msg::<(), _>(this, sel!("getRectsBeingDrawn:count:"), (&mut rects, &mut n));
        let asked: Vec<Rect> = std::slice::from_raw_parts(rects, n as usize).iter().map(|&r| Rect::from(r)).collect();
        // AppKit (layer-backed view) merges our regions into one bounding
        // rectangle. If exactly that is requested, draw only our regions:
        // the rest of the layer content is kept. If more is requested (resize,
        // window overlap), draw everything requested.
        let bbox = st.requested.iter().fold(Rect::default(), |acc, r| acc.union(r)).inflate(0.5);
        let own = !st.requested_full && !st.requested.is_empty() && asked.iter().all(|a| bbox.covers(a));
        let rects: Vec<Rect> = if own {
            st.requested
                .iter()
                .flat_map(|r| asked.iter().map(move |a| r.intersect(a)))
                .filter(|r| !r.is_empty())
                .collect()
        } else {
            asked
        };
        st.requested.clear();
        st.requested_full = false;
        let mut canvas = CgCanvas::new(ctx, &st.text);
        let mut area = 0.0;
        for &r in &rects {
            // AppKit may also request the strip under the title bar (y < 0); don't count it.
            area += r.intersect(&st.rt.frame.rect()).area();
            uison_frame::paint(&st.rt.frame, &mut canvas, r);
            if st.flashing {
                st.flash_n = st.flash_n.wrapping_add(1);
                let hue = [(255, 60, 60), (60, 200, 60), (60, 120, 255), (230, 180, 0)][st.flash_n as usize % 4];
                use uison_core::Canvas;
                canvas.rect(r, 0.0, uison_core::Paint::fill(uison_core::Color::rgba(hue.0, hue.1, hue.2, 70)));
            }
        }
        // Screenshot with a delay: WindowServer must get the frame on screen first.
        if let Some(c) = &mut st.capture
            && !c.scheduled
            && st.player.is_none()
        {
            c.scheduled = true;
            msg::<(), _>(this, sel!("performSelector:withObject:afterDelay:"), (sel!("uisonCapture:"), NIL, 0.2f64));
        }
        if st.stats {
            let latency = st.pending_since.take().map(|t| t.elapsed().as_micros());
            eprintln!(
                "draw: {} rect(s), {}% of frame, {}us{}",
                rects.len(),
                percent(area / st.rt.frame.rect().area().max(1.0)),
                t0.elapsed().as_micros(),
                latency.map(|l| format!(", event→drawn {l}us")).unwrap_or_default(),
            );
        }
    }
}

extern "C" fn capture_window(this: Id, _: Sel, _: Id) {
    unsafe {
        let st = state(this);
        let Some(c) = &mut st.capture else { return };
        c.scheduled = false;
        let window: Id = msg(this, sel!("window"), ());
        let number: isize = msg(window, sel!("windowNumber"), ());
        let img = CGWindowListCreateImage(
            CGRectNull,
            LIST_INCLUDING_WINDOW,
            number as u32,
            IMAGE_IGNORE_FRAMING,
        );
        if img.is_null() {
            eprintln!("uison: capture failed");
            return;
        }
        let path = match c.named.take() {
            Some(name) => format!("{}/{name}.png", c.dir),
            None => {
                c.n += 1;
                format!("{}/{:04}.png", c.dir, c.n - 1)
            }
        };
        match save_image_png(img, &path) {
            Ok(()) => eprintln!("capture: {path}"),
            Err(e) => eprintln!("uison: {e}"),
        }
        CGImageRelease(img);
    }
}

// ---------------------------------------------------------------- scenario

/// UISON_SCRIPT: JSON scenario (see `uison_frame::script`) with the mouse; UISON_TOUCH=1 with touches.
fn load_script() -> Option<Player> {
    let path = std::env::var("UISON_SCRIPT").ok().filter(|s| !s.is_empty())?;
    let src = std::fs::read_to_string(&path).map_err(|e| eprintln!("uison: script {path}: {e}")).ok()?;
    Player::new(&src, env_flag("UISON_TOUCH")).map_err(|e| eprintln!("uison: {e}")).ok()
}

/// Scenario step: events go the same path as real ones, then a timer until the next step.
extern "C" fn script_step(this: Id, _: Sel, _: Id) {
    unsafe {
        if BUSY.get() {
            msg::<(), _>(this, sel!("performSelector:withObject:afterDelay:"), (sel!("uisonStep:"), NIL, 0.05f64));
            return;
        }
        let st = state(this);
        let Some(player) = st.player.as_mut() else { return };
        let Some((action, pause)) = player.next(&st.rt.frame) else { return };
        match action {
            Action::Event(e) => send(this, e),
            Action::Capture(name) => match &mut st.capture {
                // WindowServer doesn't show the frame immediately, so take the screenshot a bit later.
                Some(c) => {
                    c.named = Some(name);
                    msg::<(), _>(this, sel!("performSelector:withObject:afterDelay:"), (sel!("uisonCapture:"), NIL, 0.25f64));
                }
                None => eprintln!("uison: capture {name} (UISON_CAPTURE not set)"),
            },
            Action::Log(m) => eprintln!("uison: {m}"),
            Action::Wait => {}
            Action::Done => {
                eprintln!("uison: done");
                std::process::exit(0);
            }
        }
        msg::<(), _>(this, sel!("performSelector:withObject:afterDelay:"), (sel!("uisonStep:"), NIL, pause.as_secs_f64()));
    }
}

// ---------------------------------------------------------------- events

unsafe fn location(view: Id, event: Id) -> Point {
    unsafe {
        let p: CGPoint = msg(event, sel!("locationInWindow"), ());
        let p: CGPoint = msg(view, sel!("convertPoint:fromView:"), (p, NIL));
        Point::new(p.x as f32, p.y as f32)
    }
}

fn mouse(this: Id, event: Id, f: impl FnOnce(Point) -> Event) {
    unsafe {
        let p = location(this, event);
        send(this, f(p));
    }
}

extern "C" fn mouse_down(this: Id, _: Sel, e: Id) {
    mouse(this, e, |p| Event::MouseDown(p, MouseButton::Left));
}

extern "C" fn mouse_up(this: Id, _: Sel, e: Id) {
    mouse(this, e, |p| Event::MouseUp(p, MouseButton::Left));
}

extern "C" fn right_down(this: Id, _: Sel, e: Id) {
    mouse(this, e, |p| Event::MouseDown(p, MouseButton::Right));
}

extern "C" fn right_up(this: Id, _: Sel, e: Id) {
    mouse(this, e, |p| Event::MouseUp(p, MouseButton::Right));
}

extern "C" fn mouse_moved(this: Id, _: Sel, e: Id) {
    mouse(this, e, Event::MouseMove);
}

extern "C" fn mouse_exited(this: Id, _: Sel, _: Id) {
    send(this, Event::MouseLeave);
}

extern "C" fn scroll_wheel(this: Id, _: Sel, e: Id) {
    unsafe {
        let p = location(this, e);
        let mut dx: f64 = msg(e, sel!("scrollingDeltaX"), ());
        let mut dy: f64 = msg(e, sel!("scrollingDeltaY"), ());
        let precise: bool = msg(e, sel!("hasPreciseScrollingDeltas"), ());
        if !precise {
            // The mouse wheel reports "lines".
            dx *= 16.0;
            dy *= 16.0;
        }
        send(this, Event::Scroll(p, dx as f32, dy as f32));
    }
}

extern "C" fn tick(this: Id, _: Sel, _: Id) {
    send(this, Event::Tick);
}

extern "C" fn key_down(this: Id, _: Sel, e: Id) {
    unsafe {
        let code: u16 = msg(e, sel!("keyCode"), ());
        let mods = mods(msg(e, sel!("modifierFlags"), ()));
        // macOS virtual key codes (Carbon kVK_*).
        let code = match code {
            51 => KeyCode::Backspace,
            117 => KeyCode::Delete,
            36 | 76 => KeyCode::Enter,
            48 => KeyCode::Tab,
            53 => KeyCode::Escape,
            123 => KeyCode::Left,
            124 => KeyCode::Right,
            125 => KeyCode::Down,
            126 => KeyCode::Up,
            115 => KeyCode::Home,
            119 => KeyCode::End,
            116 => KeyCode::PageUp,
            121 => KeyCode::PageDown,
            109 => KeyCode::F10,
            110 => KeyCode::Menu,
            _ => KeyCode::Char,
        };
        // Text is printable characters only: no control characters and no
        // function keys (U+F700…U+F8FF in AppKit).
        let s: Id = msg(e, sel!("characters"), ());
        let c: *const c_char = if s.is_null() { null() } else { msg(s, sel!("UTF8String"), ()) };
        let text = if c.is_null() { String::new() } else { CStr::from_ptr(c).to_string_lossy().into_owned() };
        let printable = !text.is_empty() && text.chars().all(|ch| !ch.is_control() && !('\u{F700}'..='\u{F8FF}').contains(&ch));
        let key = Key { code, text: (code == KeyCode::Char && printable).then(|| text.into()), mods };
        send(this, Event::Key(key));
    }
}

/// Whether the view has a dark theme (appearance name NSAppearanceNameDarkAqua etc.).
unsafe fn is_dark(view: Id) -> bool {
    unsafe {
        let appearance: Id = msg(view, sel!("effectiveAppearance"), ());
        let name: Id = msg(appearance, sel!("name"), ());
        let c: *const c_char = if name.is_null() { null() } else { msg(name, sel!("UTF8String"), ()) };
        !c.is_null() && CStr::from_ptr(c).to_bytes().windows(4).any(|w| w == b"Dark")
    }
}

extern "C" fn appearance_changed(this: Id, _: Sel) {
    unsafe {
        let dark = is_dark(this);
        send(this, Event::Appearance(dark));
    }
}

extern "C" fn window_did_resize(_: Id, _: Sel, note: Id) {
    unsafe {
        let window: Id = msg(note, sel!("object"), ());
        let view: Id = msg(window, sel!("contentView"), ());
        let b: CGRect = msg(view, sel!("bounds"), ());
        send(view, Event::Resize(Size::new(b.size.width as f32, b.size.height as f32)));
    }
}

// ---------------------------------------------------------------- startup

unsafe fn new_obj(cls: Class) -> Id {
    unsafe {
        let o: Id = msg(cls, sel!("alloc"), ());
        msg(o, sel!("init"), ())
    }
}

unsafe fn install_menu(ns_app: Id, title: &str) {
    unsafe {
        let bar = new_obj(class!("NSMenu"));
        let item = new_obj(class!("NSMenuItem"));
        msg::<(), _>(bar, sel!("addItem:"), (item,));
        msg::<(), _>(ns_app, sel!("setMainMenu:"), (bar,));
        let menu = new_obj(class!("NSMenu"));
        let quit_title = nsstring(&format!("Quit {title}"));
        let key = nsstring("q");
        let quit: Id = msg(class!("NSMenuItem"), sel!("alloc"), ());
        let quit: Id = msg(quit, sel!("initWithTitle:action:keyEquivalent:"), (quit_title, sel!("terminate:"), key));
        msg::<(), _>(menu, sel!("addItem:"), (quit,));
        msg::<(), _>(item, sel!("setSubmenu:"), (menu,));
        // Containers retain their elements; our references are no longer needed.
        for o in [bar, item, menu, quit, quit_title, key] {
            release(o);
        }
    }
}

fn env_flag(name: &str) -> bool {
    std::env::var(name).is_ok_and(|v| v != "0" && !v.is_empty())
}

/// Open a window with the app and enter the event loop. Does not return.
pub fn run(app: impl App + 'static, title: &str) {
    let cls = classes();
    unsafe {
        objc_autoreleasePoolPush();
        let ns_app: Id = msg(class!("NSApplication"), sel!("sharedApplication"), ());
        msg::<bool, _>(ns_app, sel!("setActivationPolicy:"), (0isize,)); // Regular
        // Dock icon from `uison::set_icon` (a bundled .app already has one, the same picture).
        if let Some(png) = uison_core::icon::icon() {
            let data: Id = msg(class!("NSData"), sel!("dataWithBytes:length:"), (png.as_ptr(), png.len()));
            let image: Id = msg(class!("NSImage"), sel!("alloc"), ());
            let image: Id = msg(image, sel!("initWithData:"), (data,));
            if !image.is_null() {
                msg::<(), _>(ns_app, sel!("setApplicationIconImage:"), (image,));
                release(image);
            }
        }
        install_menu(ns_app, title);

        let text = Text::new();
        let t0 = Instant::now();
        let paste = Cell::new(false);
        let rt = Runtime::new(Box::new(app), &MacHost { text: &text, paste: &paste }).unwrap_or_else(|e| {
            eprintln!("uison: bad frame: {e}");
            std::process::exit(1)
        });
        let load_us = t0.elapsed().as_micros();
        let frame = &rt.frame;
        let rect = CGRect::new(0.0, 0.0, frame.size.w as f64, frame.size.h as f64);

        let window: Id = msg(class!("NSWindow"), sel!("alloc"), ());
        // Titled | Closable | Miniaturizable | Resizable; Buffered.
        let window: Id = msg(window, sel!("initWithContentRect:styleMask:backing:defer:"), (rect, 15usize, 2usize, false));
        let t = nsstring(title);
        msg::<(), _>(window, sel!("setTitle:"), (t,));
        release(t);

        let view: Id = msg(cls.view as Class, sel!("alloc"), ());
        let view: Id = msg(view, sel!("initWithFrame:"), (rect,));
        let st = Box::new(State {
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
            cursor: Cursor::Arrow,
            tick_at: None,
            hitmap: std::env::var("UISON_HITMAP").ok().filter(|p| !p.is_empty()),
            capture: std::env::var("UISON_CAPTURE").ok().filter(|d| !d.is_empty()).map(|dir| {
                let _ = std::fs::create_dir_all(&dir);
                Capture { dir, n: 0, scheduled: false, named: None }
            }),
            player: load_script(),
        });
        if st.stats {
            eprintln!("first frame: {} items, app + load {load_us}us", st.rt.frame.items.len());
        }
        st.write_hitmap();
        *view.cast::<u8>().offset(cls.state_offset).cast::<*mut State>() = Box::into_raw(st);
        // Wakeup for Handle::post from other threads (the view lives until the process ends).
        let view_addr = view as usize;
        state(view).rt.app.set_waker(std::sync::Arc::new(move || {
            dispatch_async_f(&raw const _dispatch_main_q, view_addr as *mut c_void, wake)
        }));

        // MouseEnteredAndExited | MouseMoved | ActiveInKeyWindow | InVisibleRect.
        let area: Id = msg(class!("NSTrackingArea"), sel!("alloc"), ());
        let area: Id =
            msg(area, sel!("initWithRect:options:owner:userInfo:"), (CGRect::default(), 0x223usize, view, NIL));
        msg::<(), _>(view, sel!("addTrackingArea:"), (area,));

        msg::<(), _>(window, sel!("setContentView:"), (view,));
        let dark = is_dark(view);
        if dark {
            send(view, Event::Appearance(true));
        }
        msg::<bool, _>(window, sel!("makeFirstResponder:"), (view,));
        let delegate = new_obj(cls.delegate as Class);
        msg::<(), _>(window, sel!("setDelegate:"), (delegate,));
        msg::<(), _>(ns_app, sel!("setDelegate:"), (delegate,));
        msg::<(), _>(window, sel!("center"), ());
        msg::<(), _>(window, sel!("makeKeyAndOrderFront:"), (NIL,));
        if state(view).player.is_some() {
            msg::<(), _>(view, sel!("performSelector:withObject:afterDelay:"), (sel!("uisonStep:"), NIL, 1.0f64));
        }
        msg::<(), _>(ns_app, sel!("activateIgnoringOtherApps:"), (true,));
        msg::<(), _>(ns_app, sel!("run"), ());
    }
}
