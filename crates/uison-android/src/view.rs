//! Window state and native methods of `dev.uison.UisonView`: events → `Runtime`,
//! drawing into `Canvas`, callbacks into Java (keyboard, frames, timers).

use std::cell::{Cell, RefCell};
use std::ffi::c_void;
use std::time::{Duration, Instant};

use uison_core::{App, Canvas, Color, Event, Insets, Key, KeyCode, Keyboard, Mods, MouseButton, Paint, Point, Rect, Size};
use uison_frame::script::{Action, Player};
use uison_frame::{Damage, Runtime};

use crate::canvas::{JCanvas, Paints};
use crate::host::{AndroidHost, clipboard};
use crate::java::j;
use crate::jni::*;
use crate::text::AndroidText;
use crate::{env, set_env};

thread_local! {
    /// The app from `run` until the window is created.
    static PENDING: RefCell<Option<Box<dyn App>>> = const { RefCell::new(None) };
    static STATE: RefCell<Option<State>> = const { RefCell::new(None) };
}

/// Remember the app; `UisonActivity` creates the window (called from `main` in `JNI_OnLoad`).
pub fn run(app: impl App + 'static, _title: &str) {
    PENDING.with(|p| *p.borrow_mut() = Some(Box::new(app)));
}

// ALooper of the main thread: alarm for Handle::post — a byte into a pipe the loop waits on.
#[link(name = "android")]
unsafe extern "C" {
    fn ALooper_forThread() -> *mut c_void;
    fn ALooper_addFd(
        looper: *mut c_void,
        fd: i32,
        ident: i32,
        events: i32,
        callback: extern "C" fn(i32, i32, *mut c_void) -> i32,
        data: *mut c_void,
    ) -> i32;
}

unsafe extern "C" {
    fn pipe(fds: *mut i32) -> i32;
    fn read(fd: i32, buf: *mut c_void, n: usize) -> isize;
    fn write(fd: i32, buf: *const c_void, n: usize) -> isize;
    fn fcntl(fd: i32, cmd: i32, arg: i32) -> i32;
}

/// On the main thread: read everything from the pipe and run deferred work.
extern "C" fn wake(fd: i32, _events: i32, _data: *mut c_void) -> i32 {
    let mut buf = [0u8; 64];
    while unsafe { read(fd, buf.as_mut_ptr().cast(), buf.len()) } > 0 {}
    with(env().0, |st, out| {
        st.dispatch(Event::Wake, out);
    });
    1 // stay registered
}

/// Pipe + ALooper; writing to the pipe (from any thread) wakes the main loop.
fn install_waker(app: &mut dyn App) {
    let mut fds = [0i32; 2];
    if unsafe { pipe(fds.as_mut_ptr()) } != 0 {
        return;
    }
    const F_SETFL: i32 = 4;
    const O_NONBLOCK: i32 = 0o4000;
    unsafe {
        fcntl(fds[0], F_SETFL, O_NONBLOCK);
        fcntl(fds[1], F_SETFL, O_NONBLOCK);
        // ALOOPER_POLL_CALLBACK, ALOOPER_EVENT_INPUT.
        ALooper_addFd(ALooper_forThread(), fds[0], -2, 1, wake, std::ptr::null_mut());
    }
    let w = fds[1];
    app.set_waker(std::sync::Arc::new(move || {
        let b = 1u8;
        unsafe { write(w, (&b as *const u8).cast(), 1) };
    }));
}

/// Ticks more frequent than this follow screen frames (Choreographer), rarer ones use a timer.
const FRAME_TICK: Duration = Duration::from_millis(20);

struct State {
    rt: Runtime,
    text: AndroidText,
    /// The app asked for a paste — perform it after the event.
    paste: Cell<bool>,
    paints: Paints,
    /// Global reference to UisonView.
    view: jobject,
    /// Physical pixels per point.
    density: f32,
    size: Size,
    system_safe: Insets,
    ime: f32,
    safe: Insets,
    dark: Option<bool>,
    /// The keyboard being shown.
    keyboard: Option<Keyboard>,
    /// Regions of the last change — for highlighting (UISON_FLASH).
    last_damage: Vec<Rect>,
    flashing: bool,
    flash_n: u32,
    stats: bool,
    pending_since: Option<Instant>,
    frame_pending: bool,
    player: Option<Player>,
    started: Instant,
    first_drawn: bool,
    last_draw: Option<Instant>,
    gaps: Vec<f32>,
    /// Animation ticks by timer rather than screen frames (UISON_TIMER=1, for comparison).
    timer_only: bool,
}

/// Java call after the state is released (Java may synchronously call us again).
enum Out {
    Keyboard(i32),
}

fn with<R>(e: *mut JNIEnv, f: impl FnOnce(&mut State, &mut Vec<Out>) -> R) -> Option<R> {
    set_env(e);
    let mut out = Vec::new();
    let r = STATE.with(|s| s.borrow_mut().as_mut().map(|st| f(st, &mut out)));
    let view = STATE.with(|s| s.borrow().as_ref().map(|st| st.view));
    if let Some(view) = view {
        for o in out {
            match o {
                Out::Keyboard(k) => unsafe { env().call_void(view, j().view_keyboard, &[i(k)]) },
            }
        }
    }
    r
}

fn env_flag(name: &str) -> bool {
    std::env::var(name).is_ok_and(|v| v != "0" && !v.is_empty())
}

fn keyboard_kind(k: Keyboard) -> i32 {
    match k {
        Keyboard::Text => 0,
        Keyboard::Email => 1,
        Keyboard::Number => 2,
        Keyboard::Decimal => 3,
        Keyboard::Phone => 4,
        Keyboard::Url => 5,
        Keyboard::Password => 6,
    }
}

impl State {
    fn dispatch(&mut self, ev: Event, out: &mut Vec<Out>) -> bool {
        let t0 = Instant::now();
        let name = ev.name();
        let e = env();
        let host = AndroidHost { text: &self.text, view: self.view, paste: &self.paste };
        let changed = match self.rt.dispatch(&ev, &host) {
            Ok(Some(up)) => {
                let d = self.density;
                match &up.damage {
                    Damage::None => {}
                    Damage::Full => unsafe { e.call_void(self.view, j().view_damage, &[i(0), i(0), i(0), i(0)]) },
                    Damage::Rects(rs) => {
                        for r in rs {
                            let (l, t) = ((r.x * d).floor() as i32, (r.y * d).floor() as i32);
                            let (rr, b) = ((r.right() * d).ceil() as i32, (r.bottom() * d).ceil() as i32);
                            unsafe { e.call_void(self.view, j().view_damage, &[i(l), i(t), i(rr), i(b)]) };
                        }
                    }
                }
                if up.damage != Damage::None {
                    self.last_damage = up.damage.rects(self.rt.frame.rect()).collect();
                    if self.pending_since.is_none() {
                        self.pending_since = Some(t0);
                    }
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
                true
            }
            Ok(None) => false,
            Err(err) => {
                eprintln!("uison: bad frame: {err}");
                false
            }
        };
        let changed = match self.paste.take().then(|| clipboard(self.view)).flatten() {
            Some(t) => self.dispatch(Event::Paste(t.into()), out) | changed,
            None => changed,
        };
        self.schedule_tick();
        let want = self.rt.app.wants_keyboard();
        // The keyboard may have been hidden with the system Back — show it again when the field is touched.
        let touched = matches!(ev, Event::MouseUp(_, MouseButton::Touch));
        if want != self.keyboard || (touched && want.is_some()) {
            self.keyboard = want;
            out.push(Out::Keyboard(want.map_or(-1, keyboard_kind)));
        }
        changed
    }

    fn schedule_tick(&mut self) {
        let Some(d) = self.rt.app.wants_tick() else { return };
        let e = env();
        unsafe {
            if d <= FRAME_TICK && !self.timer_only {
                if !self.frame_pending {
                    self.frame_pending = true;
                    e.call_void(self.view, j().view_frame, &[]);
                }
            } else {
                e.call_void(self.view, j().view_tick, &[long(d.as_millis() as i64)]);
            }
        }
    }

    /// Safe area for the app: the system one; at the bottom — no less than the keyboard (as on iOS).
    fn update_safe(&mut self, out: &mut Vec<Out>) {
        let mut safe = self.system_safe;
        safe.bottom = safe.bottom.max(self.ime);
        if safe != self.safe {
            self.safe = safe;
            self.dispatch(Event::SafeArea(safe), out);
        }
    }

    fn report_gaps(&mut self) {
        if !self.stats || self.gaps.len() < 5 {
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

    fn draw(&mut self, c: jobject) {
        let t0 = Instant::now();
        let e = env();
        let d = self.density;
        if self.stats {
            if let Some(t) = self.last_draw.filter(|t| t.elapsed().as_millis() < 100) {
                self.gaps.push(t.elapsed().as_secs_f32() * 1000.0);
            }
            self.last_draw = Some(t0);
        }
        let dirty;
        unsafe {
            e.push_frame(16);
            e.call_int(c, j().save, &[]);
            e.call_void(c, j().scale, &[f(d), f(d)]);
            // What the system asks for (after scale — already in points): with hardware acceleration — the whole screen.
            let clip = clip_bounds(e, c);
            let frame = self.rt.frame.rect();
            dirty = clip.map_or(frame, |r| r.intersect(&frame));
            let mut canvas = JCanvas::new(e, c, &self.text, &self.paints, d);
            uison_frame::paint(&self.rt.frame, &mut canvas, dirty);
            if self.flashing {
                for r in &self.last_damage {
                    self.flash_n = self.flash_n.wrapping_add(1);
                    let hue = [(255, 60, 60), (60, 200, 60), (60, 120, 255), (230, 180, 0)][self.flash_n as usize % 4];
                    canvas.rect(*r, 0.0, Paint::fill(Color::rgba(hue.0, hue.1, hue.2, 70)));
                }
            }
            e.call_void(c, j().restore, &[]);
            e.pop_frame();
            e.check();
        }
        if self.stats {
            if !self.first_drawn {
                eprintln!("launch → first draw: {} ms", self.started.elapsed().as_millis());
            }
            let latency = self.pending_since.take().map(|t| t.elapsed().as_micros());
            eprintln!(
                "draw: {}% of frame, {}us{}",
                (100.0 * dirty.area() / self.rt.frame.rect().area().max(1.0)).round() as u32,
                t0.elapsed().as_micros(),
                latency.map(|l| format!(", event→drawn {l}us")).unwrap_or_default(),
            );
        }
        self.first_drawn = true;
    }
}

/// Bounds of the canvas clip in its current coordinates (`Canvas.getClipBounds`).
unsafe fn clip_bounds(e: Env, c: jobject) -> Option<Rect> {
    let j = j();
    unsafe {
        let r = e.new_object(j.rect_cls, j.rect_new, &[]);
        let ok = e.call_bool(c, j.get_clip_bounds, &[l(r)]);
        let get = |fid| e.int_field(r, fid) as f32;
        let rect = ok.then(|| {
            let (l, t, rr, b) = (get(j.rect_left), get(j.rect_top), get(j.rect_right), get(j.rect_bottom));
            Rect::new(l, t, rr - l, b - t)
        });
        e.delete_global(r);
        rect
    }
}

// ---------------------------------------------------------------- native methods

extern "C" fn native_create(e: *mut JNIEnv, _: jclass, view: jobject, density: f32, env_s: jstring, script: jstring) {
    set_env(e);
    let env = env();
    unsafe {
        // Launch parameters (am start --es KEY value) — environment variables.
        for line in env.string(env_s).lines() {
            if let Some((k, v)) = line.split_once('=') {
                std::env::set_var(k, v);
            }
        }
        let script = (!script.is_null()).then(|| env.string(script));
        let view = env.global(view);
        // Window recreated (activity restarted) — same state, new View.
        if let Some(()) = STATE.with(|s| {
            s.borrow_mut().as_mut().map(|st| {
                env.delete_global(st.view);
                st.view = view;
                st.density = density;
            })
        }) {
            return;
        }
        let Some(app) = PENDING.with(|p| p.borrow_mut().take()) else {
            eprintln!("uison: no app — call uison::run(app, title) in main()");
            return;
        };
        let started = Instant::now();
        let text = AndroidText::new();
        let paste = Cell::new(false);
        let rt = match Runtime::new(app, &AndroidHost { text: &text, view, paste: &paste }) {
            Ok(rt) => rt,
            Err(err) => {
                eprintln!("uison: bad frame: {err}");
                return;
            }
        };
        let stats = env_flag("UISON_STATS");
        if stats {
            eprintln!("first frame: {} items, app + load {}us", rt.frame.items.len(), started.elapsed().as_micros());
        }
        let player = script.and_then(|src| Player::new(&src, true).map_err(|err| eprintln!("uison: {err}")).ok());
        let has_player = player.is_some();
        let st = State {
            size: rt.frame.size,
            rt,
            text,
            paste,
            paints: Paints::new(env),
            view,
            density,
            system_safe: Insets::default(),
            ime: 0.0,
            safe: Insets::default(),
            dark: None,
            keyboard: None,
            last_damage: Vec::new(),
            flashing: env_flag("UISON_FLASH"),
            flash_n: 0,
            stats,
            pending_since: None,
            frame_pending: false,
            player,
            started,
            first_drawn: false,
            last_draw: None,
            gaps: Vec::new(),
            timer_only: env_flag("UISON_TIMER"),
        };
        let mut st = st;
        install_waker(&mut *st.rt.app);
        STATE.with(|s| *s.borrow_mut() = Some(st));
        if has_player {
            env.call_void(view, j().view_step, &[long(1000)]);
        }
    }
}

extern "C" fn native_resize(e: *mut JNIEnv, _: jclass, w: i32, h: i32) {
    with(e, |st, out| {
        let size = Size::new(w as f32 / st.density, h as f32 / st.density);
        if size != st.size && size.w > 0.0 && size.h > 0.0 {
            st.size = size;
            st.dispatch(Event::Resize(size), out);
        }
    });
}

extern "C" fn native_insets(e: *mut JNIEnv, _: jclass, top: i32, right: i32, bottom: i32, left: i32, ime: i32) {
    with(e, |st, out| {
        let d = st.density;
        st.system_safe = Insets { top: top as f32 / d, right: right as f32 / d, bottom: bottom as f32 / d, left: left as f32 / d };
        st.ime = ime as f32 / d;
        st.update_safe(out);
    });
}

extern "C" fn native_draw(e: *mut JNIEnv, _: jclass, canvas: jobject) {
    with(e, |st, _| st.draw(canvas));
}

/// MotionEvent: ACTION_DOWN 0, UP 1, MOVE 2, CANCEL 3, HOVER_MOVE 7, HOVER_EXIT 10;
/// tool: finger 1, stylus 2, mouse 3.
extern "C" fn native_pointer(e: *mut JNIEnv, _: jclass, action: i32, x: f32, y: f32, tool: i32, buttons: i32) {
    with(e, |st, out| {
        let p = Point::new(x / st.density, y / st.density);
        let b = match (tool, buttons) {
            (3, 2) => MouseButton::Right,
            (3, _) => MouseButton::Left,
            _ => MouseButton::Touch,
        };
        let ev = match action {
            0 => Event::MouseDown(p, b),
            1 => Event::MouseUp(p, b),
            2 | 7 => Event::MouseMove(p),
            // The system took the touch away (Back gesture, notification) — release outside, no click.
            3 => Event::MouseUp(Point::new(-1.0, -1.0), b),
            10 => Event::MouseLeave,
            _ => return,
        };
        st.dispatch(ev, out);
    });
}

extern "C" fn native_scroll(e: *mut JNIEnv, _: jclass, x: f32, y: f32, dy: f32) {
    with(e, |st, out| {
        let p = Point::new(x / st.density, y / st.density);
        // Wheel: step ~ a line; sign as scrollingDeltaY on macOS (up — content moves down).
        st.dispatch(Event::Scroll(p, 0.0, dy * 48.0), out);
    });
}

/// Android key codes (KeyEvent.KEYCODE_*); meta: Shift 1, Alt 2, Ctrl 0x1000, Meta 0x10000.
extern "C" fn native_key(e: *mut JNIEnv, _: jclass, code: i32, unicode: i32, meta: i32) -> jboolean {
    let code_uison = match code {
        67 => KeyCode::Backspace,
        112 => KeyCode::Delete,
        66 | 160 => KeyCode::Enter,
        61 => KeyCode::Tab,
        111 => KeyCode::Escape,
        21 => KeyCode::Left,
        22 => KeyCode::Right,
        19 => KeyCode::Up,
        20 => KeyCode::Down,
        122 => KeyCode::Home,
        123 => KeyCode::End,
        92 => KeyCode::PageUp,
        93 => KeyCode::PageDown,
        82 => KeyCode::Menu,
        140 => KeyCode::F10,
        _ => KeyCode::Char,
    };
    let ch = char::from_u32(unicode as u32).filter(|c| !c.is_control());
    if code_uison == KeyCode::Char && ch.is_none() {
        return 0; // Back, volume and the rest go to the system
    }
    // Ctrl is the primary shortcut modifier (`cmd`), as in a browser not on a Mac:
    // `ui.shortcut("cmd+n")` also compares `ctrl`, so both can't be set at once.
    let mods = Mods { shift: meta & 1 != 0, alt: meta & 2 != 0, ctrl: false, cmd: meta & 0x1000 != 0 };
    let mut buf = [0u8; 4];
    let text = if code_uison == KeyCode::Char { ch.map(|c| c.encode_utf8(&mut buf).into()) } else { None };
    let key = Key { code: code_uison, text, mods };
    with(e, |st, out| {
        st.dispatch(Event::Key(key), out);
    });
    1
}

/// Text from the on-screen keyboard (InputConnection.commitText): characters, `\n` — Enter.
extern "C" fn native_text(e: *mut JNIEnv, _: jclass, s: jstring) {
    set_env(e);
    let s = unsafe { env().string(s) };
    with(e, |st, out| {
        for part in s.split_inclusive('\n') {
            let (t, enter) = part.strip_suffix('\n').map_or((part, false), |t| (t, true));
            if !t.is_empty() {
                st.dispatch(Event::Key(Key::char(t)), out);
            }
            if enter {
                st.dispatch(Event::Key(Key::new(KeyCode::Enter)), out);
            }
        }
    });
}

extern "C" fn native_frame(e: *mut JNIEnv, _: jclass, _nanos: i64) {
    with(e, |st, out| {
        st.frame_pending = false;
        st.dispatch(Event::Tick, out);
        if !st.frame_pending {
            st.report_gaps();
        }
    });
}

extern "C" fn native_tick(e: *mut JNIEnv, _: jclass) {
    with(e, |st, out| {
        st.dispatch(Event::Tick, out);
    });
}

extern "C" fn native_appearance(e: *mut JNIEnv, _: jclass, dark: jboolean) {
    with(e, |st, out| {
        let dark = dark != 0;
        if st.dark != Some(dark) {
            st.dark = Some(dark);
            st.dispatch(Event::Appearance(dark), out);
        }
    });
}

/// A message for the app from the Java side (the chosen file: `{"action":…,"key":…}`).
extern "C" fn native_message(e: *mut JNIEnv, _: jclass, s: jstring) {
    set_env(e);
    let s = unsafe { env().string(s) };
    with(e, |st, out| st.dispatch(Event::Message(s.into()), out));
}

/// System Back: `true` — the app handled it (something changed), otherwise it goes to the system.
extern "C" fn native_back(e: *mut JNIEnv, _: jclass) -> jboolean {
    with(e, |st, out| st.dispatch(Event::Key(Key::new(KeyCode::Back)), out)).unwrap_or(false) as jboolean
}

/// Check scenario step (UISON_SCRIPT — a file in the APK assets).
extern "C" fn native_step(e: *mut JNIEnv, _: jclass) {
    with(e, |st, out| {
        let Some(player) = st.player.as_mut() else { return };
        let Some((action, pause)) = player.next(&st.rt.frame) else { return };
        match action {
            Action::Event(ev) => {
                st.dispatch(ev, out);
            }
            Action::Capture(name) => eprintln!("uison: capture {name}"),
            Action::Log(m) => eprintln!("uison: {m}"),
            Action::Wait => {}
            Action::Done => {
                crate::log_now("uison: done");
                std::process::exit(0);
            }
        }
        unsafe { env().call_void(st.view, j().view_step, &[long(pause.as_millis() as i64)]) };
    });
}

/// Register the native methods of `UisonView` (from `JNI_OnLoad`).
pub unsafe fn register(e: Env, cls: jclass) {
    macro_rules! m {
        ($name:literal, $sig:literal, $f:expr) => {
            NativeMethod { name: $name.as_ptr(), signature: $sig.as_ptr(), fn_ptr: $f as *const c_void }
        };
    }
    type V = extern "C" fn(*mut JNIEnv, jclass);
    let methods = [
        m!(c"nativeCreate", c"(Ldev/uison/UisonView;FLjava/lang/String;Ljava/lang/String;)V",
           native_create as extern "C" fn(*mut JNIEnv, jclass, jobject, f32, jstring, jstring)),
        m!(c"nativeResize", c"(II)V", native_resize as extern "C" fn(*mut JNIEnv, jclass, i32, i32)),
        m!(c"nativeInsets", c"(IIIII)V", native_insets as extern "C" fn(*mut JNIEnv, jclass, i32, i32, i32, i32, i32)),
        m!(c"nativeDraw", c"(Landroid/graphics/Canvas;)V", native_draw as extern "C" fn(*mut JNIEnv, jclass, jobject)),
        m!(c"nativePointer", c"(IFFII)V", native_pointer as extern "C" fn(*mut JNIEnv, jclass, i32, f32, f32, i32, i32)),
        m!(c"nativeScroll", c"(FFF)V", native_scroll as extern "C" fn(*mut JNIEnv, jclass, f32, f32, f32)),
        m!(c"nativeKey", c"(III)Z", native_key as extern "C" fn(*mut JNIEnv, jclass, i32, i32, i32) -> jboolean),
        m!(c"nativeText", c"(Ljava/lang/String;)V", native_text as extern "C" fn(*mut JNIEnv, jclass, jstring)),
        m!(c"nativeFrame", c"(J)V", native_frame as extern "C" fn(*mut JNIEnv, jclass, i64)),
        m!(c"nativeTick", c"()V", native_tick as V),
        m!(c"nativeAppearance", c"(Z)V", native_appearance as extern "C" fn(*mut JNIEnv, jclass, jboolean)),
        m!(c"nativeBack", c"()Z", native_back as extern "C" fn(*mut JNIEnv, jclass) -> jboolean),
        m!(c"nativeStep", c"()V", native_step as V),
        m!(c"nativeMessage", c"(Ljava/lang/String;)V", native_message as extern "C" fn(*mut JNIEnv, jclass, jstring)),
    ];
    unsafe { e.register(cls, &methods) };
}
