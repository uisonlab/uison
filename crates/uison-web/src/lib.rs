//! Web backend: canvas 2D in the browser. No wasm-bindgen: drawing and metrics are
//! imports from `web/uison.js` (the `uison` module), events are `uison_*` exports.
//!
//! An app is written as usual: `fn main() { uison::run(app(), "Title") }`.
//! In WASM `run` only remembers the app; after that JS calls the exports.

#![cfg(target_arch = "wasm32")]

mod canvas;
mod js;
mod text;

use std::cell::RefCell;
use std::time::Duration;

use uison_core::{App, Cursor, Event, Key, KeyCode, Mods, MouseButton, Point, Rect, Size};
use uison_frame::{Damage, Runtime};

use crate::canvas::WebCanvas;
use crate::text::WebText;

/// A message to the browser console.
pub fn log(s: &str) {
    unsafe { js::log(s.as_ptr(), s.len(), 0) }
}

fn error(s: &str) {
    unsafe { js::log(s.as_ptr(), s.len(), 2) }
}

/// Register the app. After `main()` JS calls `uison_init`.
pub fn run(app: impl App + 'static, title: &str) {
    std::panic::set_hook(Box::new(|info| error(&format!("uison: {info}"))));
    uison_core::time::set_clock(|| Duration::from_secs_f64(unsafe { js::now() } / 1000.0));
    #[cfg(target_os = "emscripten")]
    {
        // Exports for JS go through the function table: emcc needs no --export flags, and its
        // minifier won't drop functions that only JS calls.
        macro_rules! table {
            ($($f:ident),*) => { (concat!($(stringify!($f), ","),*), [$($f as *const () as usize as u32),*]) };
        }
        let (names, fns) = table!(
            uison_init, uison_resize, uison_appearance, uison_pointer, uison_wheel, uison_input_buffer,
            uison_key, uison_input, uison_paste, uison_tick, uison_image_loaded, uison_frame, uison_bench,
            uison_draws, uison_hitmap, uison_hitmap_len, uison_wake
        );
        unsafe { js::boot(names.as_ptr(), names.len(), fns.as_ptr(), fns.len()) };
    }
    unsafe { js::set_title(title.as_ptr(), title.len()) };
    PENDING.with(|p| *p.borrow_mut() = Some(Box::new(app)));
    // Emscripten: `main` doesn't return and the stack isn't unwound — main's locals
    // referenced by C/C++ handlers (`[&]`) stay alive; further events come from JS.
    // Exit is a wasm trap: neither catch_all nor C++ cleanup catches it (with -fwasm-exceptions
    // the JS exception `unwind` from emscripten_exit_with_live_runtime would run main's destructors),
    // and the glue takes it as a normal exit from main (uison_js_park).
    #[cfg(target_os = "emscripten")]
    unsafe {
        js::park();
        core::arch::wasm32::unreachable()
    }
}

thread_local! {
    static PENDING: RefCell<Option<Box<dyn App>>> = const { RefCell::new(None) };
    static STATE: RefCell<Option<State>> = const { RefCell::new(None) };
    /// Buffer for strings from JS (key text).
    static INPUT: RefCell<Vec<u8>> = RefCell::new(vec![0; 256]);
    static HITMAP: RefCell<String> = const { RefCell::new(String::new()) };
}

const FLAG_STATS: u32 = 1;
const FLAG_FLASH: u32 = 2;


struct State {
    rt: Runtime,
    text: WebText,
    dpr: f32,
    /// Regions to draw in the next browser frame (requestAnimationFrame).
    dirty: Vec<Rect>,
    full: bool,
    frame_requested: bool,
    cursor: Cursor,
    /// Deadline of the nearest requested tick.
    tick_at: Option<Duration>,
    stats: bool,
    flash: bool,
    flash_n: u32,
    /// How many times we drew (measurements).
    draws: u32,
    /// Start of handling an event not yet drawn (for latency).
    pending_since: Option<Duration>,
}

impl State {
    fn invalidate(&mut self, d: &Damage) {
        match d {
            Damage::None => return,
            Damage::Full => self.full = true,
            Damage::Rects(rs) => self.dirty.extend(rs.iter().copied()),
        }
        // Draw once per browser frame: events more frequent than frames merge into one draw.
        if !self.frame_requested {
            self.frame_requested = true;
            unsafe { js::request_frame() };
        }
    }

    fn dispatch(&mut self, ev: Event) -> bool {
        let t0 = uison_core::time::now();
        let pos = ev.pos();
        let name = ev.name();
        let changed = match self.rt.dispatch(&ev, &self.text) {
            Ok(Some(up)) => {
                if self.stats {
                    let r = self.rt.frame.rect();
                    log(&format!(
                        "{name}: app {}us, parse {}us ({} B), diff {}us → {} rect(s), {}%",
                        up.app.as_micros(),
                        up.load.as_micros(),
                        up.bytes,
                        up.diff.as_micros(),
                        up.damage.rects(r).count(),
                        (up.damage.ratio(r) * 100.0).round()
                    ));
                }
                if self.pending_since.is_none() && up.damage != Damage::None {
                    self.pending_since = Some(t0);
                }
                self.invalidate(&up.damage);
                true
            }
            Ok(None) => false,
            Err(e) => {
                error(&format!("uison: bad frame: {e}"));
                false
            }
        };
        if let Some(p) = pos {
            self.set_cursor(uison_frame::cursor_at(&self.rt.frame, p));
        }
        self.schedule_tick();
        changed
    }

    fn set_cursor(&mut self, c: Cursor) {
        if c != self.cursor {
            self.cursor = c;
            let k = match c {
                Cursor::Arrow => 0,
                Cursor::Pointer => 1,
                Cursor::Text => 2,
            };
            unsafe { js::set_cursor(k) };
        }
    }

    /// As in the macOS backend: a new deadline earlier than the scheduled one — one more timer.
    fn schedule_tick(&mut self) {
        let Some(d) = self.rt.app.wants_tick() else { return };
        let at = uison_core::time::now() + d;
        if self.tick_at.is_some_and(|t| t <= at) {
            return;
        }
        self.tick_at = Some(at);
        unsafe { js::request_tick(d.as_secs_f64() * 1000.0) };
    }

    fn draw(&mut self) {
        self.frame_requested = false;
        let t0 = uison_core::time::now();
        let frame = &self.rt.frame;
        let rects: Vec<Rect> =
            if self.full { vec![frame.rect()] } else { std::mem::take(&mut self.dirty) };
        self.full = false;
        self.dirty.clear();
        self.draws += 1;
        let mut c = WebCanvas::new(&self.text, self.dpr);
        let mut area = 0.0;
        for r in &rects {
            area += r.area();
            uison_frame::paint(frame, &mut c, *r);
            if self.flash {
                self.flash_n = self.flash_n.wrapping_add(1);
                let colors = [0xFF3C3C46, 0x3CC83C46, 0x3C78FF46, 0xE6B40046];
                c.flash(*r, colors[self.flash_n as usize % 4]);
            }
        }
        if self.stats {
            let latency = self.pending_since.take().map(|t| (uison_core::time::now() - t).as_micros());
            log(&format!(
                "draw: {} rect(s), {}% of frame, {}us{}",
                rects.len(),
                (100.0 * area / frame.rect().area().max(1.0)).round(),
                (uison_core::time::now() - t0).as_micros(),
                latency.map(|l| format!(", event→drawn {l}us")).unwrap_or_default()
            ));
        }
        self.pending_since = None;
    }
}

fn with<R>(f: impl FnOnce(&mut State) -> R) -> Option<R> {
    STATE.with(|s| s.borrow_mut().as_mut().map(f))
}

// ---------------------------------------------------------------- exports

/// Canvas is ready: create the runtime, fit the first frame to the canvas size.
#[unsafe(no_mangle)]
pub extern "C" fn uison_init(w: f32, h: f32, dpr: f32, flags: u32) {
    let Some(app) = PENDING.with(|p| p.borrow_mut().take()) else {
        error("uison: no app — call uison::run(app, title) in main()");
        return;
    };
    let text = WebText::new();
    let mut rt = match Runtime::new(app, &text) {
        Ok(rt) => rt,
        Err(e) => return error(&format!("uison: bad first frame: {e}")),
    };
    // Alarm for Handle::post (the browser has one thread — post from handlers, JS timers).
    rt.app.set_waker(std::sync::Arc::new(|| unsafe { js::wake() }));
    let mut st = State {
        rt,
        text,
        dpr,
        dirty: Vec::new(),
        full: true,
        frame_requested: false,
        cursor: Cursor::Arrow,
        tick_at: None,
        stats: flags & FLAG_STATS != 0,
        flash: flags & FLAG_FLASH != 0,
        flash_n: 0,
        draws: 0,
        pending_since: None,
    };
    st.dispatch(Event::Resize(Size::new(w, h)));
    st.invalidate(&Damage::Full);
    STATE.with(|s| *s.borrow_mut() = Some(st));
}

/// Size or pixel density changed (canvas cleared — draw everything).
#[unsafe(no_mangle)]
pub extern "C" fn uison_resize(w: f32, h: f32, dpr: f32) {
    with(|st| {
        st.dpr = dpr;
        st.dispatch(Event::Resize(Size::new(w, h)));
        st.invalidate(&Damage::Full);
    });
}

/// System theme: 1 — dark.
#[unsafe(no_mangle)]
pub extern "C" fn uison_appearance(dark: u32) {
    with(|st| st.dispatch(Event::Appearance(dark != 0)));
}

/// kind: 0 — down, 1 — up, 2 — move, 3 — leave. button: 0 — left, 2 — right.
#[unsafe(no_mangle)]
pub extern "C" fn uison_pointer(kind: u32, x: f32, y: f32, button: u32) -> u32 {
    let p = Point::new(x, y);
    let b = match button {
        0 => MouseButton::Left,
        2 => MouseButton::Right,
        9 => MouseButton::Touch,
        _ => MouseButton::Other,
    };
    let ev = match kind {
        0 => Event::MouseDown(p, b),
        1 => Event::MouseUp(p, b),
        2 => Event::MouseMove(p),
        _ => Event::MouseLeave,
    };
    with(|st| st.dispatch(ev) as u32).unwrap_or(0)
}

/// dy > 0 — content moves down (like scrollingDeltaY on macOS).
#[unsafe(no_mangle)]
pub extern "C" fn uison_wheel(x: f32, y: f32, dx: f32, dy: f32) -> u32 {
    with(|st| st.dispatch(Event::Scroll(Point::new(x, y), dx, dy)) as u32).unwrap_or(0)
}

/// Buffer for key text: JS writes UTF-8 and passes the length to `uison_key`.
#[unsafe(no_mangle)]
pub extern "C" fn uison_input_buffer() -> *mut u8 {
    INPUT.with(|b| b.borrow_mut().as_mut_ptr())
}

/// code — `KeyCode` number, mods — bits shift 1, ctrl 2, alt 4, cmd 8.
/// Returns 1 if the app handled the key (then JS cancels the browser's action).
#[unsafe(no_mangle)]
pub extern "C" fn uison_key(code: u32, mods: u32, len: usize) -> u32 {
    let code = match code {
        1 => KeyCode::Backspace,
        2 => KeyCode::Delete,
        3 => KeyCode::Enter,
        4 => KeyCode::Tab,
        5 => KeyCode::Escape,
        6 => KeyCode::Left,
        7 => KeyCode::Right,
        8 => KeyCode::Up,
        9 => KeyCode::Down,
        10 => KeyCode::Home,
        11 => KeyCode::End,
        12 => KeyCode::PageUp,
        13 => KeyCode::PageDown,
        14 => KeyCode::Menu,
        15 => KeyCode::F10,
        _ => KeyCode::Char,
    };
    let text = INPUT.with(|b| {
        let b = b.borrow();
        std::str::from_utf8(&b[..len.min(b.len())]).ok().filter(|s| !s.is_empty()).map(Box::from)
    });
    with(|st| st.dispatch(Event::Key(Key { code, text, mods: self::mods(mods) })) as u32).unwrap_or(0)
}

/// Modifier bits from JS: shift 1, ctrl 2, alt 4, cmd 8.
fn mods(m: u32) -> Mods {
    Mods { shift: m & 1 != 0, ctrl: m & 2 != 0, alt: m & 4 != 0, cmd: m & 8 != 0 }
}

/// Buffer for a string from JS of length `len` (grows when needed).
#[unsafe(no_mangle)]
pub extern "C" fn uison_input(len: usize) -> *mut u8 {
    INPUT.with(|b| {
        let mut b = b.borrow_mut();
        if b.len() < len {
            b.resize(len, 0);
        }
        b.as_mut_ptr()
    })
}

/// Paste: clipboard text (from the `paste` event or `clipboard.readText`) is in `uison_input`.
#[unsafe(no_mangle)]
pub extern "C" fn uison_paste(len: usize) -> u32 {
    let text = INPUT.with(|b| String::from_utf8_lossy(&b.borrow()[..len]).into_owned());
    with(|st| st.dispatch(Event::Paste(text.into())) as u32).unwrap_or(0)
}

/// `Handle::post`: run deferred work (the alarm is `js::wake`).
#[unsafe(no_mangle)]
pub extern "C" fn uison_wake() {
    with(|st| st.dispatch(Event::Wake));
}

#[unsafe(no_mangle)]
pub extern "C" fn uison_tick() {
    with(|st| {
        st.tick_at = None;
        st.dispatch(Event::Tick);
    });
}

/// Browser frame (requestAnimationFrame): draw the accumulated regions.
/// An image finished loading: redraw everything (diff can't see it — same frame), lay out again.
#[unsafe(no_mangle)]
pub extern "C" fn uison_image_loaded() {
    with(|st| {
        st.invalidate(&Damage::Full);
        st.dispatch(Event::ImageLoaded);
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn uison_frame() {
    with(State::draw);
}

/// Measurement: full frame redraw `n` times; mode 2 — without text.
/// Returns the average frame time in microseconds (only recording commands into the canvas).
#[unsafe(no_mangle)]
pub extern "C" fn uison_bench(n: u32, mode: u32) -> f64 {
    with(|st| {
        let frame = &st.rt.frame;
        let t0 = uison_core::time::now();
        for _ in 0..n {
            let mut c = WebCanvas::new(&st.text, st.dpr);
            c.skip_text = mode & 2 != 0;
            uison_frame::paint(frame, &mut c, frame.rect());
        }
        (uison_core::time::now() - t0).as_secs_f64() * 1e6 / n.max(1) as f64
    })
    .unwrap_or(0.0)
}

/// How many times we drew since start.
#[unsafe(no_mangle)]
pub extern "C" fn uison_draws() -> u32 {
    with(|st| st.draws).unwrap_or(0)
}

/// Hit-area map of the current frame (`[[id, x, y, w, h], ...]`) — for automation.
#[unsafe(no_mangle)]
pub extern "C" fn uison_hitmap() -> *const u8 {
    let s = with(|st| {
        let items: Vec<String> = uison_frame::hit_areas(&st.rt.frame)
            .into_iter()
            .map(|(id, r)| {
                let mut q = String::new();
                uison_json_str(&mut q, id);
                format!("[{q},{},{},{},{}]", r.x.round(), r.y.round(), r.w.round(), r.h.round())
            })
            .collect();
        format!("[{}]", items.join(","))
    })
    .unwrap_or_else(|| "[]".into());
    HITMAP.with(|h| {
        *h.borrow_mut() = s;
        h.borrow().as_ptr()
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn uison_hitmap_len() -> usize {
    HITMAP.with(|h| h.borrow().len())
}

/// A quoted JSON string (without depending on uison-json).
fn uison_json_str(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if (c as u32) < 0x20 => out.push(' '),
            c => out.push(c),
        }
    }
    out.push('"');
}
