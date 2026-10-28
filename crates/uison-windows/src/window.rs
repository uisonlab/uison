//! Win32 window: window class, our own `WndProc`, a `GetMessageW` loop.
//! Drawing is Direct2D in `WM_PAINT`, only the update region's areas.

use std::cell::{Cell, RefCell};
use std::ptr::{null, null_mut};
use std::time::{Duration, Instant};

use uison_core::{App, Canvas, Color, Cursor, Event, Key, KeyCode, Mods, MouseButton, Paint, Point, Rect, Size};
use uison_frame::script::{Action, Player};
use uison_frame::{Damage, Runtime};

use crate::canvas::D2dCanvas;
use crate::target::Target;
use crate::clock::FrameClock;
use crate::d2d::*;
use crate::ffi::*;
use crate::host::{WinHost, clipboard};
use crate::text::Text;
use crate::wic::{Pixels, Wic};

/// Window timers.
const TIMER_TICK: usize = 1;
const TIMER_SCRIPT: usize = 2;
const TIMER_CAPTURE: usize = 3;
/// A display frame from `FrameClock` (animation ticks).
pub const WM_FRAME: u32 = WM_APP + 1;
/// Wake-up from another thread (`Handle::post`).
const WM_WAKE: u32 = WM_APP + 2;
/// Ticks more frequent than this follow display frames, rarer ones use a timer.
const FRAME_TICK: Duration = Duration::from_millis(20);

struct State {
    rt: Runtime,
    text: Text,
    /// The app requested a paste — perform it after the event.
    paste: Cell<bool>,
    factory: Factory,
    /// The render target is created on first draw and recreated on device loss.
    target: Option<Target>,
    /// Flip chain instead of HwndRenderTarget (UISON_TARGET=flip, experiment 1).
    flip: bool,
    /// What was drawn last time (pixels) — for the flip chain.
    prev_rects: Vec<RECT>,
    hwnd: HWND,
    /// Dots per inch of the window's monitor; scale = dpi / 96.
    dpi: u32,
    /// Client area size in points.
    size: Size,
    present: u32,
    /// Draw right after the event (otherwise wait for WM_PAINT; UISON_PAINT=deferred).
    paint_sync: bool,
    paint_now: bool,
    flashing: bool,
    flash_n: u32,
    stats: bool,
    stats_buf: String,
    pending_since: Option<Instant>,
    cursor: Cursor,
    /// By `Cursor as usize`.
    cursors: [HCURSOR; 5],
    /// A `WM_MOUSELEAVE` notification has been requested.
    tracking: bool,
    tick_at: Option<Instant>,
    clock: Option<FrameClock>,
    frame_pending: bool,
    /// Animation ticks by timer rather than display frames (UISON_TIMER=1, for comparison).
    timer_only: bool,
    hitmap: Option<String>,
    capture: Option<Capture>,
    player: Option<Player>,
    /// The high half of a surrogate pair from `WM_CHAR`.
    high_surrogate: Option<u16>,
    /// Lines per wheel notch (system setting).
    wheel_lines: f32,
    dark: bool,
    /// The title bar follows the frame's background, not the system: an always dark app gets a dark one.
    title_dark: Option<bool>,
    started: Instant,
    first_drawn: bool,
    last_draw: Option<Instant>,
    gaps: Vec<f32>,
}

struct Capture {
    dir: String,
    n: u32,
    scheduled: bool,
    wic: Wic,
}

thread_local! {
    /// The app is handling an event. A modal loop inside a handler (folder picker dialog,
    /// MessageBox) delivers messages to the window — their events wait here until handling ends
    /// instead of re-entering the app.
    static BUSY: Cell<bool> = const { Cell::new(false) };
    static DEFERRED: RefCell<Vec<Event>> = const { RefCell::new(Vec::new()) };
}

thread_local! {
    /// Window state. A pointer, not a RefCell: `WndProc` is re-entered
    /// from inside system calls (SetWindowPos → WM_SIZE); the reference is held only
    /// while handling and never across such calls.
    static STATE: Cell<*mut State> = const { Cell::new(null_mut()) };
}

fn state<'a>() -> Option<&'a mut State> {
    let p = STATE.with(Cell::get);
    unsafe { p.as_mut() }
}

/// Window icon `size` pixels square: the `.exe` resource with id 1 (an .ico with every size,
/// embedded by build.rs), otherwise the PNG from `uison::set_icon`; null — the default icon.
unsafe fn window_icon(inst: HINSTANCE, size: i32) -> *mut std::ffi::c_void {
    unsafe {
        // MAKEINTRESOURCE(1): a resource number in place of a name. Not LR_SHARED: sizes depend
        // on the monitor's DPI, and the caller owns (destroys) the icon it replaces.
        let icon = LoadImageW(inst, std::ptr::without_provenance(1), IMAGE_ICON, size, size, 0);
        if !icon.is_null() {
            return icon;
        }
        match uison_core::icon::icon() {
            Some(png) => icon_from_png(png, size),
            None => null_mut(),
        }
    }
}

/// Icon from a PNG downscaled by WIC (Fant). `CreateIconFromResourceEx` understands PNG too,
/// but shrinks 256 px to 16 almost without smoothing — jagged edges (Task11.md, stage 5).
fn icon_from_png(png: &[u8], size: i32) -> *mut std::ffi::c_void {
    let Some(px) = Wic::new().ok().and_then(|wic| wic.decode_bgra(png, size as u32, size as u32).ok()) else {
        return null_mut();
    };
    unsafe {
        // A 32-bit color bitmap with alpha; the mask is still required, but unused.
        let color = CreateBitmap(size, size, 1, 32, px.as_ptr().cast());
        let mask = CreateBitmap(size, size, 1, 1, null());
        let info = ICONINFO { fIcon: 1, xHotspot: 0, yHotspot: 0, hbmMask: mask, hbmColor: color };
        let icon = CreateIconIndirect(&info);
        DeleteObject(color);
        DeleteObject(mask);
        icon
    }
}

/// Title bar and taskbar icons for the window's DPI. The class icons have the system DPI's size:
/// on a monitor with a larger scale the system would stretch them (blurry, Task11.md experiment 1).
unsafe fn set_window_icons(hwnd: HWND, dpi: u32) {
    unsafe {
        let inst = GetModuleHandleW(null());
        for (kind, metric) in [(ICON_SMALL, SM_CXSMICON), (ICON_BIG, SM_CXICON)] {
            let icon = window_icon(inst, GetSystemMetricsForDpi(metric, dpi));
            if icon.is_null() {
                return;
            }
            let old = SendMessageW(hwnd, WM_SETICON, kind, icon as LPARAM);
            if old != 0 {
                DestroyIcon(old as *mut std::ffi::c_void);
            }
        }
    }
}

fn env_flag(name: &str) -> bool {
    std::env::var(name).is_ok_and(|v| v != "0" && !v.is_empty())
}

/// A ratio in whole percent: stats without f32 formatting (it weighs ~16 KB).
fn percent(ratio: f32) -> u32 {
    (ratio * 100.0).round() as u32
}

impl State {
    fn scale(&self) -> f32 {
        self.dpi as f32 / 96.0
    }

    /// A rectangle in points → the enclosing one in pixels.
    fn to_px(&self, r: Rect) -> RECT {
        let s = self.scale();
        RECT {
            left: (r.x * s).floor() as i32,
            top: (r.y * s).floor() as i32,
            right: (r.right() * s).ceil() as i32,
            bottom: (r.bottom() * s).ceil() as i32,
        }
    }

    fn px_to_points(&self, r: &RECT) -> Rect {
        let s = self.scale();
        Rect::new(r.left as f32 / s, r.top as f32 / s, (r.right - r.left) as f32 / s, (r.bottom - r.top) as f32 / s)
    }

    fn point(&self, x: i32, y: i32) -> Point {
        let s = self.scale();
        Point::new(x as f32 / s, y as f32 / s)
    }

    /// An event to the app — or to the queue if it is busy with the previous one (`false`: no changes yet).
    fn dispatch(&mut self, ev: Event) -> bool {
        if BUSY.get() {
            DEFERRED.with_borrow_mut(|d| match (d.last_mut(), &ev) {
                // Of mouse moves only the last one matters.
                (Some(Event::MouseMove(last)), Event::MouseMove(p)) => *last = *p,
                _ => d.push(ev),
            });
            return false;
        }
        BUSY.set(true);
        let mut changed = self.dispatch_now(ev);
        while let Some(ev) = DEFERRED.with_borrow_mut(|d| (!d.is_empty()).then(|| d.remove(0))) {
            changed |= self.dispatch_now(ev);
        }
        BUSY.set(false);
        changed
    }

    fn dispatch_now(&mut self, ev: Event) -> bool {
        let pos = ev.pos();
        let leave = ev == Event::MouseLeave;
        let mut changed = self.apply(&ev);
        if self.paste.take()
            && let Some(t) = clipboard(self.hwnd)
        {
            changed |= self.apply(&Event::Paste(t.into()));
        }
        if let Some(p) = pos {
            self.set_cursor(uison_frame::cursor_at(&self.rt.frame, p));
        } else if leave {
            self.cursor = Cursor::Arrow;
        }
        self.schedule_tick();
        changed
    }

    fn apply(&mut self, ev: &Event) -> bool {
        let t0 = Instant::now();
        let host = WinHost { text: &self.text, hwnd: self.hwnd, paste: &self.paste };
        let up = match self.rt.dispatch(ev, &host) {
            Ok(Some(up)) => up,
            Ok(None) => return false,
            Err(e) => {
                log!("uison: bad frame: {e}");
                return false;
            }
        };
        self.write_hitmap();
        self.sync_title();
        match &up.damage {
            Damage::None => {}
            // Full is the whole client area: it may be larger than the frame.
            Damage::Full => unsafe {
                InvalidateRect(self.hwnd, null(), 0);
            },
            Damage::Rects(rs) => {
                for r in rs {
                    let px = self.to_px(*r);
                    unsafe { InvalidateRect(self.hwnd, &px, 0) };
                }
            }
        }
        if up.damage != Damage::None {
            self.paint_now = self.paint_sync;
        }
        if up.damage != Damage::None && self.pending_since.is_none() {
            self.pending_since = Some(t0);
        }
        if self.stats {
            let frame = self.rt.frame.rect();
            let line = format!(
                "{}: app {}us, parse {}us ({} B), diff {}us → {} rect(s), {}%",
                ev.name(),
                up.app.as_micros(),
                up.load.as_micros(),
                up.bytes,
                up.diff.as_micros(),
                up.damage.rects(frame).count(),
                percent(up.damage.ratio(frame)),
            );
            self.stat(line);
        }
        true
    }

    fn set_cursor(&mut self, c: Cursor) {
        if c == self.cursor {
            return;
        }
        self.cursor = c;
        unsafe { SetCursor(self.cursors[c as usize]) };
    }

    /// The app wants a tick: frequent (animation) via display frames, rare (caret) via a timer.
    fn schedule_tick(&mut self) {
        let Some(d) = self.rt.app.wants_tick() else { return };
        if d <= FRAME_TICK && !self.timer_only {
            if !self.frame_pending {
                self.frame_pending = true;
                self.clock.get_or_insert_with(|| FrameClock::start(self.hwnd)).request();
            }
            return;
        }
        let at = Instant::now() + d;
        if self.tick_at.is_some_and(|t| t <= at) {
            return;
        }
        self.tick_at = Some(at);
        // The same id replaces the previous timer — the nearest deadline remains.
        unsafe { SetTimer(self.hwnd, TIMER_TICK, d.as_millis().max(1) as u32, null()) };
    }

    /// UISON_HITMAP=file: map of the current frame's hit areas (`id x y w h` per line, points).
    fn write_hitmap(&self) {
        if let Some(path) = &self.hitmap {
            let mut s = String::new();
            let i = |v: f32| v.round() as i64;
            for (id, r) in uison_frame::hit_areas(&self.rt.frame) {
                s.push_str(&format!("{id}\t{}\t{}\t{}\t{}\n", i(r.x), i(r.y), i(r.w), i(r.h)));
            }
            let _ = std::fs::write(path, s);
        }
    }

    fn pixel_size(&self) -> SizeU {
        let mut rc = RECT::default();
        unsafe { GetClientRect(self.hwnd, &mut rc) };
        SizeU { width: (rc.right - rc.left).max(1) as u32, height: (rc.bottom - rc.top).max(1) as u32 }
    }

    fn ensure_target(&mut self) -> bool {
        if self.target.is_some() {
            return true;
        }
        let t0 = Instant::now();
        let size = self.pixel_size();
        match Target::new(&self.factory, self.hwnd, size, self.dpi as f32, self.present, self.flip) {
            Ok(t) => {
                // Text antialiasing as in the system (usually ClearType); UISON_TEXT=gray — grayscale.
                if std::env::var("UISON_TEXT").as_deref() == Ok("gray") {
                    t.rt().set_text_antialias(D2D1_TEXT_ANTIALIAS_MODE_GRAYSCALE);
                }
                self.target = Some(t);
                if self.stats {
                    let line = format!("render target: {}us (at {} ms)", t0.elapsed().as_micros(), self.started.elapsed().as_millis());
                    self.stat(line);
                }
                true
            }
            Err(e) => {
                log!("uison: render target: {e}");
                false
            }
        }
    }

    /// `WM_PAINT`: draw the update region's rectangles (not their bounding box).
    fn paint(&mut self) {
        let t0 = Instant::now();
        let rects = unsafe { update_rects(self.hwnd) };
        let mut ps: PAINTSTRUCT = unsafe { std::mem::zeroed() };
        unsafe { BeginPaint(self.hwnd, &mut ps) };
        if !self.ensure_target() {
            unsafe { EndPaint(self.hwnd, &ps) };
            return;
        }
        if self.stats {
            if let Some(t) = self.last_draw.filter(|t| t.elapsed().as_millis() < 100) {
                self.gaps.push(t.elapsed().as_secs_f32() * 1000.0);
            }
            self.last_draw = Some(t0);
        }
        let target = self.target.as_ref().unwrap();
        // Flip chain: the back buffer holds the frame from two presents ago, so also draw
        // what changed last time (DXGI does not copy it by itself).
        let mut rects = rects;
        if target.is_flip() {
            let now = rects.clone();
            rects.extend(std::mem::replace(&mut self.prev_rects, now));
        }
        let scale = self.scale();
        let mut area = 0.0;
        let pts: Vec<Rect> = rects.iter().map(|r| self.px_to_points(r)).collect();
        target.rt().begin_draw();
        {
            let mut canvas = D2dCanvas::new(target.rt(), &self.factory, &target.res, &self.text, scale);
            for &r in &pts {
                area += r.intersect(&self.rt.frame.rect()).area();
                uison_frame::paint(&self.rt.frame, &mut canvas, r);
                if self.flashing {
                    self.flash_n = self.flash_n.wrapping_add(1);
                    let hue = [(255, 60, 60), (60, 200, 60), (60, 120, 255), (230, 180, 0)][self.flash_n as usize % 4];
                    canvas.rect(r, 0.0, Paint::fill(Color::rgba(hue.0, hue.1, hue.2, 70)));
                }
            }
        }
        let t_end = Instant::now();
        let hr = target.end(&rects);
        let end_us = t_end.elapsed().as_micros();
        unsafe { EndPaint(self.hwnd, &ps) };
        if hr == D2DERR_RECREATE_TARGET {
            // Device lost: a new target and a full redraw.
            self.target = None;
            unsafe { InvalidateRect(self.hwnd, null(), 0) };
        }
        // Only after draws we caused (the frame changed): PrintWindow itself
        // requests WM_PAINT — otherwise each screenshot would trigger the next.
        // With a scenario — only its named screenshots.
        let ours = (!self.first_drawn || self.pending_since.is_some()) && self.player.is_none();
        if let Some(c) = &mut self.capture
            && !c.scheduled
            && ours
        {
            // Screenshot after a delay: DWM must have time to present the frame.
            c.scheduled = true;
            unsafe { SetTimer(self.hwnd, TIMER_CAPTURE, 200, null()) };
        }
        if self.stats {
            if !self.first_drawn {
                let line = format!("launch → first draw: {} ms", self.started.elapsed().as_millis());
                self.stat(line);
            }
            let latency = self.pending_since.take().map(|t| t.elapsed().as_micros());
            let line = format!(
                "draw: {} rect(s), {}% of frame, {}us (EndDraw {end_us}us){}",
                pts.len(),
                percent(area / self.rt.frame.rect().area().max(1.0)),
                t0.elapsed().as_micros(),
                latency.map(|l| format!(", event→drawn {l}us")).unwrap_or_default(),
            );
            self.stat(line);
            self.flush_stats();
        }
        self.first_drawn = true;
    }

    /// Stats are buffered and written after drawing: writing to redirected stderr
    /// (a file) sometimes costs milliseconds — keep it out of the measured "event → frame" path.
    fn stat(&mut self, line: String) {
        self.stats_buf.push_str(&line);
        self.stats_buf.push('\n');
    }

    fn flush_stats(&mut self) {
        if !self.stats_buf.is_empty() {
            use std::io::Write;
            let _ = std::io::stderr().write_all(self.stats_buf.as_bytes());
            self.stats_buf.clear();
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
        // Tenths of a millisecond as integers: no f32 formatting.
        let ms = |x: f32| {
            let t = (x * 10.0).round() as u32;
            format!("{}.{}", t / 10, t % 10)
        };
        log!(
            "animation: {} frames, gap p50 {} ms, p95 {} ms, max {} ms, >20 ms: {slow}",
            g.len(),
            ms(at(0.5)),
            ms(at(0.95)),
            ms(g[g.len() - 1])
        );
    }

    /// Screenshot of the window's client area (as DWM shows it) to PNG.
    fn save_capture(&mut self, name: Option<&str>) {
        self.flush_stats();
        let Some(c) = &mut self.capture else { return };
        let path = match name {
            Some(n) => format!("{}/{n}.png", c.dir),
            None => {
                c.n += 1;
                format!("{}/{:04}.png", c.dir, c.n - 1)
            }
        };
        match unsafe { capture_window(self.hwnd, &c.wic, &path) } {
            Ok(()) => log!("capture: {path}"),
            Err(e) => log!("uison: capture failed: {e}"),
        }
    }

    fn step_script(&mut self) {
        self.flush_stats();
        let Some(player) = self.player.as_mut() else { return };
        let Some((action, pause)) = player.next(&self.rt.frame) else { return };
        match action {
            Action::Event(ev) => {
                // The scenario moves the mouse — the cursor too, so hover shows on screenshots.
                self.dispatch(ev);
            }
            Action::Capture(name) => {
                if self.capture.is_some() {
                    // First finish drawing whatever is pending.
                    unsafe { UpdateWindow(self.hwnd) };
                    self.save_capture(Some(&name));
                } else {
                    log!("uison: capture {name}");
                }
            }
            Action::Log(m) => log!("uison: {m}"),
            Action::Wait => {}
            Action::Done => {
                log!("uison: done");
                unsafe { PostQuitMessage(0) };
                return;
            }
        }
        unsafe { SetTimer(self.hwnd, TIMER_SCRIPT, pause.as_millis().max(1) as u32, null()) };
    }

    fn key_down(&mut self, vk: u32) -> bool {
        let down = |k: i32| unsafe { GetKeyState(k) } < 0;
        let (shift, ctrl, alt) = (down(VK_SHIFT), down(VK_CONTROL), down(VK_MENU));
        // The main shortcut modifier on Windows is Ctrl (as in a browser off the Mac).
        let mods = Mods { shift, ctrl: false, alt, cmd: ctrl };
        let code = match vk {
            VK_BACK => KeyCode::Backspace,
            VK_DELETE => KeyCode::Delete,
            VK_RETURN => KeyCode::Enter,
            VK_TAB => KeyCode::Tab,
            VK_ESCAPE => KeyCode::Escape,
            VK_LEFT => KeyCode::Left,
            VK_RIGHT => KeyCode::Right,
            VK_UP => KeyCode::Up,
            VK_DOWN => KeyCode::Down,
            VK_HOME => KeyCode::Home,
            VK_END => KeyCode::End,
            VK_PRIOR => KeyCode::PageUp,
            VK_NEXT => KeyCode::PageDown,
            VK_APPS => KeyCode::Menu,
            VK_F10 => KeyCode::F10,
            // Ctrl+char: WM_CHAR gives a control code, so the char comes from the key,
            // not the layout (Ctrl+N works in Cyrillic layouts too). Ctrl+Alt is AltGr, which types text.
            _ if ctrl && !alt => {
                let ch = match vk {
                    0x30..=0x39 | 0x41..=0x5A => char::from_u32(vk).map(|c| c.to_ascii_lowercase()),
                    _ => char::from_u32(unsafe { MapVirtualKeyW(vk, MAPVK_VK_TO_CHAR) } & 0x7FFF_FFFF)
                        .filter(|c| !c.is_control() && *c != '\0'),
                };
                let Some(ch) = ch else { return false };
                let mut buf = [0u8; 4];
                return self.dispatch(Event::Key(Key { code: KeyCode::Char, text: Some(ch.encode_utf8(&mut buf).into()), mods }));
            }
            _ => return false,
        };
        self.dispatch(Event::Key(Key { code, text: None, mods }))
    }

    /// `WM_CHAR`: printable characters (UTF-16, surrogate pairs are joined).
    fn char_input(&mut self, unit: u16) {
        let ch = match unit {
            0xD800..=0xDBFF => {
                self.high_surrogate = Some(unit);
                return;
            }
            0xDC00..=0xDFFF => match self.high_surrogate.take() {
                Some(hi) => char::decode_utf16([hi, unit]).next().and_then(Result::ok),
                None => None,
            },
            _ => char::from_u32(unit as u32),
        };
        // Control keys (Enter, Tab, Backspace, Ctrl+letter) came via WM_KEYDOWN.
        let Some(ch) = ch.filter(|c| !c.is_control()) else { return };
        let mut buf = [0u8; 4];
        let mut key = Key::char(ch.encode_utf8(&mut buf));
        key.mods.shift = unsafe { GetKeyState(VK_SHIFT) } < 0;
        self.dispatch(Event::Key(key));
    }

    fn set_dark(&mut self, dark: bool, initial: bool) {
        if dark != self.dark || (initial && dark) {
            self.dark = dark;
            self.dispatch(Event::Appearance(dark));
        }
        self.sync_title();
    }

    fn sync_title(&mut self) {
        let c = self.rt.frame.background;
        let dark = 299 * u32::from(c.r) + 587 * u32::from(c.g) + 114 * u32::from(c.b) < 128_000;
        if self.title_dark != Some(dark) {
            self.title_dark = Some(dark);
            let v: BOOL = dark as BOOL;
            unsafe { DwmSetWindowAttribute(self.hwnd, DWMWA_USE_IMMERSIVE_DARK_MODE, (&raw const v).cast(), 4) };
        }
    }

    fn resized(&mut self) {
        let px = self.pixel_size();
        if let Some(t) = &mut self.target {
            t.resize(px);
        }
        let s = self.scale();
        let size = Size::new(px.width as f32 / s, px.height as f32 / s);
        if size != self.size {
            self.size = size;
            self.dispatch(Event::Resize(size));
        }
    }
}

/// Rectangles of the area to redraw (in pixels).
unsafe fn update_rects(hwnd: HWND) -> Vec<RECT> {
    unsafe {
        let rgn = CreateRectRgn(0, 0, 0, 0);
        GetUpdateRgn(hwnd, rgn, 0);
        let size = GetRegionData(rgn, 0, null_mut());
        let mut out = Vec::new();
        if size as usize >= size_of::<RGNDATAHEADER>() {
            // u64 for header alignment.
            let mut buf = vec![0u64; (size as usize).div_ceil(8)];
            if GetRegionData(rgn, size, buf.as_mut_ptr().cast()) != 0 {
                let hdr = &*(buf.as_ptr() as *const RGNDATAHEADER);
                let rects = (buf.as_ptr() as *const u8).add(hdr.dwSize as usize) as *const RECT;
                out.extend_from_slice(std::slice::from_raw_parts(rects, hdr.nCount as usize));
            }
        }
        DeleteObject(rgn);
        if out.is_empty() {
            let mut rc = RECT::default();
            GetClientRect(hwnd, &mut rc);
            out.push(rc);
        }
        out
    }
}

/// Screenshot of the client area: `PrintWindow` with `PW_RENDERFULLCONTENT` also captures
/// what was drawn via Direct2D/DXGI (a plain BitBlt from the screen does not).
unsafe fn capture_window(hwnd: HWND, wic: &Wic, path: &str) -> Result<(), String> {
    unsafe {
        let mut rc = RECT::default();
        GetClientRect(hwnd, &mut rc);
        let (w, h) = (rc.right - rc.left, rc.bottom - rc.top);
        if w <= 0 || h <= 0 {
            return Err("empty window".into());
        }
        let screen = GetDC(null_mut());
        let dc = CreateCompatibleDC(screen);
        let info = BITMAPINFOHEADER {
            biSize: size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: w,
            biHeight: -h, // top-down
            biPlanes: 1,
            biBitCount: 32,
            biCompression: 0,
            biSizeImage: 0,
            biXPelsPerMeter: 0,
            biYPelsPerMeter: 0,
            biClrUsed: 0,
            biClrImportant: 0,
        };
        let mut bits = null_mut();
        let bmp = CreateDIBSection(dc, &info, 0, &mut bits, null_mut(), 0);
        let old = SelectObject(dc, bmp);
        let ok = PrintWindow(hwnd, dc, 1 | PW_RENDERFULLCONTENT) != 0; // PW_CLIENTONLY
        let len = (w * h * 4) as usize;
        let data = std::slice::from_raw_parts_mut(bits as *mut u8, len);
        // GDI does not fill alpha.
        for px in data.chunks_exact_mut(4) {
            px[3] = 255;
        }
        let res = if ok {
            wic.save_png(path, Pixels::Bgra { w: w as u32, h: h as u32, stride: w as u32 * 4, data }).map_err(|e| e.to_string())
        } else {
            Err("PrintWindow failed".into())
        };
        SelectObject(dc, old);
        DeleteObject(bmp);
        DeleteDC(dc);
        ReleaseDC(null_mut(), screen);
        res
    }
}

/// Dark app theme (Settings → Personalization → Colors).
fn system_dark() -> bool {
    let key = wide("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize");
    let value = wide("AppsUseLightTheme");
    let mut data: u32 = 1;
    let mut size = 4u32;
    let r = unsafe {
        RegGetValueW(HKEY_CURRENT_USER, key.as_ptr(), value.as_ptr(), RRF_RT_REG_DWORD, null_mut(), (&raw mut data).cast(), &mut size)
    };
    r == 0 && data == 0
}

// ---------------------------------------------------------------- WndProc

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    let Some(st) = state() else { return unsafe { DefWindowProcW(hwnd, msg, w, l) } };
    let r = unsafe { handle(st, hwnd, msg, w, l) };
    // Draw right after the event without waiting for WM_PAINT: the system issues it on the
    // DWM composition beat — up to a frame of extra latency (Task6.md, "Results").
    if let Some(st) = state()
        && std::mem::take(&mut st.paint_now)
    {
        unsafe { UpdateWindow(hwnd) };
    }
    r
}

unsafe fn handle(st: &mut State, hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    match msg {
        WM_PAINT => st.paint(),
        // Don't erase the background: WM_PAINT covers everything (otherwise flicker on resize).
        WM_ERASEBKGND => return 1,
        WM_SIZE => st.resized(),
        // The window moves to a monitor with a different scale: compute the size for the new DPI
        // ourselves — the client area in points stays the same. Otherwise the system scales the
        // window with its frame, and after rounding the size in points drifts (940×760 → 941×762), an extra Resize.
        WM_GETDPISCALEDSIZE => {
            let dpi = loword(w) as u32;
            let s = dpi as f32 / 96.0;
            let mut rc = RECT { left: 0, top: 0, right: (st.size.w * s).round() as i32, bottom: (st.size.h * s).round() as i32 };
            unsafe { AdjustWindowRectExForDpi(&mut rc, WS_OVERLAPPEDWINDOW, 0, 0, dpi) };
            let size = l as *mut POINT;
            unsafe { *size = POINT { x: rc.right - rc.left, y: rc.bottom - rc.top } };
            return 1;
        }
        WM_DPICHANGED => {
            st.dpi = hiword(w) as u32;
            if let Some(t) = &mut st.target {
                t.set_dpi(st.dpi as f32);
            }
            let dpi = st.dpi;
            let r = unsafe { *(l as *const RECT) };
            // SetWindowPos sends WM_SIZE and WM_SETICON is sent to this window — don't hold the state reference.
            unsafe {
                set_window_icons(hwnd, dpi);
                SetWindowPos(hwnd, null_mut(), r.left, r.top, r.right - r.left, r.bottom - r.top, SWP_NOZORDER | SWP_NOACTIVATE);
                InvalidateRect(hwnd, null(), 0);
            }
        }
        WM_SETCURSOR if loword(l as usize) == HTCLIENT => {
            unsafe { SetCursor(st.cursors[st.cursor as usize]) };
            return 1;
        }
        WM_MOUSEMOVE => {
            if !st.tracking {
                st.tracking = true;
                let mut t = TRACKMOUSEEVENT { cbSize: size_of::<TRACKMOUSEEVENT>() as u32, dwFlags: TME_LEAVE, hwndTrack: hwnd, dwHoverTime: 0 };
                unsafe { TrackMouseEvent(&mut t) };
            }
            let (x, y) = lparam_point(l);
            st.dispatch(Event::MouseMove(st.point(x, y)));
        }
        WM_MOUSELEAVE => {
            st.tracking = false;
            if unsafe { GetCapture() } != hwnd {
                st.dispatch(Event::MouseLeave);
            }
        }
        WM_LBUTTONDOWN | WM_RBUTTONDOWN | WM_MBUTTONDOWN => {
            // Capture: dragging (Slider) continues outside the window.
            unsafe { SetCapture(hwnd) };
            let b = match msg {
                WM_LBUTTONDOWN => MouseButton::Left,
                WM_RBUTTONDOWN => MouseButton::Right,
                _ => MouseButton::Other,
            };
            let (x, y) = lparam_point(l);
            st.dispatch(Event::MouseDown(st.point(x, y), b));
        }
        WM_LBUTTONUP | WM_RBUTTONUP | WM_MBUTTONUP => {
            let b = match msg {
                WM_LBUTTONUP => MouseButton::Left,
                WM_RBUTTONUP => MouseButton::Right,
                _ => MouseButton::Other,
            };
            let (x, y) = lparam_point(l);
            st.dispatch(Event::MouseUp(st.point(x, y), b));
            // MK_LBUTTON | MK_RBUTTON | MK_MBUTTON: no other buttons are pressed.
            if w & 0x13 == 0 {
                unsafe { ReleaseCapture() };
            }
        }
        WM_MOUSEWHEEL | WM_MOUSEHWHEEL => {
            let delta = hiword(w) as i16 as f32 / 120.0;
            let (x, y) = lparam_point(l);
            let mut p = POINT { x, y };
            unsafe { ScreenToClient(hwnd, &mut p) };
            // A line is 16 pt, like the wheel on macOS; sign as scrollingDeltaY (up moves content down).
            let step = st.wheel_lines * 16.0;
            let (dx, dy) = if msg == WM_MOUSEWHEEL { (0.0, delta * step) } else { (-delta * step, 0.0) };
            st.dispatch(Event::Scroll(st.point(p.x, p.y), dx, dy));
        }
        WM_POINTERDOWN | WM_POINTERUPDATE | WM_POINTERUP | WM_POINTERCAPTURECHANGED => {
            // Mouse and pen arrive as regular WM_MOUSE* (DefWindowProc makes them), fingers here.
            let id = loword(w) as u32;
            let mut kind = 0u32;
            if unsafe { GetPointerType(id, &mut kind) } == 0 || kind != PT_TOUCH {
                return unsafe { DefWindowProcW(hwnd, msg, w, l) };
            }
            let flags = hiword(w) as u32;
            let (x, y) = lparam_point(l);
            let mut p = POINT { x, y };
            unsafe { ScreenToClient(hwnd, &mut p) };
            let p = st.point(p.x, p.y);
            let ev = match msg {
                WM_POINTERDOWN => Event::MouseDown(p, MouseButton::Touch),
                WM_POINTERUPDATE if flags & POINTER_FLAG_INCONTACT != 0 => Event::MouseMove(p),
                WM_POINTERUP => Event::MouseUp(p, MouseButton::Touch),
                // The system took the touch away — release off-target, no click.
                WM_POINTERCAPTURECHANGED => Event::MouseUp(Point::new(-1.0, -1.0), MouseButton::Touch),
                _ => return 0,
            };
            st.dispatch(ev);
            return 0;
        }
        WM_KEYDOWN => {
            if !st.key_down(w as u32) {
                return unsafe { DefWindowProcW(hwnd, msg, w, l) };
            }
        }
        // F10 arrives as a system key (window menu bar) — Shift+F10 is needed for the context menu.
        WM_SYSKEYDOWN if w as u32 == VK_F10 => {
            if !st.key_down(w as u32) {
                return unsafe { DefWindowProcW(hwnd, msg, w, l) };
            }
        }
        WM_CHAR => st.char_input(w as u16),
        WM_TIMER => match w {
            TIMER_TICK => {
                unsafe { KillTimer(hwnd, TIMER_TICK) };
                st.tick_at = None;
                st.dispatch(Event::Tick);
                if st.rt.app.wants_tick().is_none_or(|d| d > FRAME_TICK) {
                    st.report_gaps();
                }
            }
            TIMER_SCRIPT => {
                unsafe { KillTimer(hwnd, TIMER_SCRIPT) };
                st.step_script();
            }
            TIMER_CAPTURE => {
                unsafe { KillTimer(hwnd, TIMER_CAPTURE) };
                if let Some(c) = &mut st.capture {
                    c.scheduled = false;
                }
                st.save_capture(None);
            }
            _ => {}
        },
        WM_WAKE => {
            st.dispatch(Event::Wake);
        }
        WM_FRAME => {
            st.frame_pending = false;
            st.dispatch(Event::Tick);
            if !st.frame_pending {
                st.report_gaps();
            }
        }
        WM_SETTINGCHANGE => {
            // A theme change arrives as "ImmersiveColorSet".
            let dark = system_dark();
            if dark != st.dark {
                st.set_dark(dark, false);
            }
            return unsafe { DefWindowProcW(hwnd, msg, w, l) };
        }
        WM_DESTROY => unsafe { PostQuitMessage(0) },
        _ => return unsafe { DefWindowProcW(hwnd, msg, w, l) },
    }
    0
}

// ---------------------------------------------------------------- launch

/// Open a window with the app and enter the message loop.
pub fn run(app: impl App + 'static, title: &str) {
    let started = Instant::now();
    if std::env::var("UISON_PREWARM").as_deref() != Ok("0") {
        prewarm();
    }
    unsafe {
        SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        CoInitializeEx(null_mut(), COINIT_APARTMENTTHREADED);
    }
    let fail = |what: &str, e: &dyn std::fmt::Display| -> ! {
        log!("uison: {what}: {e}");
        std::process::exit(1)
    };
    let text = Text::try_new().unwrap_or_else(|e| fail("DirectWrite", &e));
    let factory = Factory::new().unwrap_or_else(|e| fail("Direct2D", &e));
    let rt = Runtime::new(Box::new(app), &text).unwrap_or_else(|e| fail("bad frame", &e));
    let load_us = started.elapsed().as_micros();

    unsafe {
        let inst = GetModuleHandleW(null());
        let class = wide("UisonWindow");
        let arrow = LoadCursorW(null_mut(), IDC_ARROW as *const u16);
        let (big, small) = (window_icon(inst, GetSystemMetrics(SM_CXICON)), window_icon(inst, GetSystemMetrics(SM_CXSMICON)));
        let wc = WNDCLASSEXW {
            cbSize: size_of::<WNDCLASSEXW>() as u32,
            style: CS_OWNDC,
            lpfnWndProc: wndproc,
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: inst,
            hIcon: big,
            hCursor: arrow,
            hbrBackground: null_mut(),
            lpszMenuName: null(),
            lpszClassName: class.as_ptr(),
            hIconSm: small,
        };
        RegisterClassExW(&wc);
        let title = wide(title);
        let hwnd = CreateWindowExW(
            0,
            class.as_ptr(),
            title.as_ptr(),
            WS_OVERLAPPEDWINDOW,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            null_mut(),
            null_mut(),
            inst,
            null_mut(),
        );
        if hwnd.is_null() {
            fail("CreateWindowExW", &"failed");
        }
        let dpi = GetDpiForWindow(hwnd).max(96);
        set_window_icons(hwnd, dpi);
        let mut lines = 3u32;
        SystemParametersInfoW(SPI_GETWHEELSCROLLLINES, 0, (&raw mut lines).cast(), 0);
        let stats = env_flag("UISON_STATS");
        // UISON_TIMER=fine: a timer with 1 ms resolution instead of 15.6 (experiment 4).
        if std::env::var("UISON_TIMER").as_deref() == Ok("fine") {
            timeBeginPeriod(1);
        }
        let present = match std::env::var("UISON_PRESENT").as_deref() {
            Ok("immediate") => D2D1_PRESENT_OPTIONS_RETAIN_CONTENTS | D2D1_PRESENT_OPTIONS_IMMEDIATELY,
            _ => D2D1_PRESENT_OPTIONS_RETAIN_CONTENTS,
        };
        let script = std::env::var("UISON_SCRIPT").ok().filter(|p| !p.is_empty());
        let player = script.and_then(|path| match std::fs::read_to_string(&path) {
            Ok(src) => Player::new(&src, false).map_err(|e| log!("uison: {e}")).ok(),
            Err(e) => {
                log!("uison: {path}: {e}");
                None
            }
        });
        let frame_size = rt.frame.size;
        // Waker for Handle::post: a message to the window's queue (from any thread).
        let mut rt = rt;
        let hwnd_addr = hwnd as usize;
        rt.app.set_waker(std::sync::Arc::new(move || {
            PostMessageW(hwnd_addr as HWND, WM_WAKE, 0, 0);
        }));
        let st = Box::new(State {
            rt,
            text,
            paste: Cell::new(false),
            factory,
            target: None,
            flip: std::env::var("UISON_TARGET").as_deref() == Ok("flip"),
            prev_rects: Vec::new(),
            hwnd,
            dpi,
            size: frame_size,
            present,
            paint_sync: std::env::var("UISON_PAINT").as_deref() != Ok("deferred"),
            paint_now: false,
            flashing: env_flag("UISON_FLASH"),
            flash_n: 0,
            stats,
            stats_buf: String::new(),
            pending_since: None,
            cursor: Cursor::Arrow,
            cursors: [IDC_ARROW, IDC_HAND, IDC_IBEAM, IDC_SIZEWE, IDC_SIZENS].map(|c| LoadCursorW(null_mut(), c as *const u16)),
            tracking: false,
            tick_at: None,
            clock: None,
            frame_pending: false,
            timer_only: env_flag("UISON_TIMER"),
            hitmap: std::env::var("UISON_HITMAP").ok().filter(|p| !p.is_empty()),
            capture: std::env::var("UISON_CAPTURE").ok().filter(|d| !d.is_empty()).and_then(|dir| {
                let _ = std::fs::create_dir_all(&dir);
                Wic::new().map_err(|e| log!("uison: WIC: {e}")).ok().map(|wic| Capture { dir, n: 0, scheduled: false, wic })
            }),
            player,
            high_surrogate: None,
            wheel_lines: lines.max(1) as f32,
            dark: false,
            title_dark: None,
            started,
            first_drawn: false,
            last_draw: None,
            gaps: Vec::new(),
        });
        if stats {
            log!("first frame: {} items, app + load {load_us}us, dpi {dpi}", st.rt.frame.items.len());
        }
        st.write_hitmap();
        let has_player = st.player.is_some();
        STATE.with(|s| s.set(Box::into_raw(st)));

        // Window size comes from the first frame, centered in the monitor's work area.
        let scale = dpi as f32 / 96.0;
        let mut rc = RECT { left: 0, top: 0, right: (frame_size.w * scale).round() as i32, bottom: (frame_size.h * scale).round() as i32 };
        AdjustWindowRectExForDpi(&mut rc, WS_OVERLAPPEDWINDOW, 0, 0, dpi);
        let (w, h) = (rc.right - rc.left, rc.bottom - rc.top);
        let mut mi = MONITORINFO { cbSize: size_of::<MONITORINFO>() as u32, rcMonitor: RECT::default(), rcWork: RECT::default(), dwFlags: 0 };
        GetMonitorInfoW(MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST), &mut mi);
        let work = mi.rcWork;
        // Larger than the work area (high scale) — shrink: the app gets a Resize.
        let (w, h) = (w.min(work.right - work.left), h.min(work.bottom - work.top));
        let x = work.left + ((work.right - work.left - w) / 2).max(0);
        let y = work.top + ((work.bottom - work.top - h) / 2).max(0);
        SetWindowPos(hwnd, null_mut(), x, y, w, h, SWP_NOZORDER | SWP_NOACTIVATE);
        if let Some(st) = state() {
            st.set_dark(system_dark(), true);
            if st.stats {
                let line = format!("window: {} ms", st.started.elapsed().as_millis());
                st.stat(line);
            }
        }
        ShowWindow(hwnd, SW_SHOW);
        if let Some(st) = state()
            && st.stats
        {
            let line = format!("shown: {} ms", st.started.elapsed().as_millis());
            st.stat(line);
        }
        UpdateWindow(hwnd);
        if has_player {
            SetTimer(hwnd, TIMER_SCRIPT, 1000, null());
        }

        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}

/// Warm up the Direct3D driver in the background while the window thread builds the first frame
/// and shows the window (`ShowWindow` ~35 ms): creating the render target takes 20 ms instead of 40–60.
fn prewarm() {
    std::thread::spawn(crate::d2d::warm_device);
}
