//! Shared by Wayland and X11 windows: the app and frame, damage region accumulation,
//! ticks, statistics (`UISON_STATS`), highlighting (`UISON_FLASH`), scenarios (`UISON_SCRIPT`),
//! screenshots (`UISON_CAPTURE`) and the element map (`UISON_HITMAP`).

use std::cell::RefCell;
use std::time::{Duration, Instant};

use uison_core::{App, Canvas, Color, Cursor, Event, Font, Host, Mods, Paint, Point, Rect, TextMeasure, TextMetrics};
use uison_frame::script::{Action, Player};
use uison_frame::{Damage, Runtime};

use crate::canvas::CairoCanvas;
use crate::gfx::{Cr, Image};
use crate::text::Text;

/// Ticks more frequent than this follow screen frames, rarer ones use a timer.
pub const FRAME_TICK: Duration = Duration::from_millis(20);

pub fn env_flag(name: &str) -> bool {
    std::env::var(name).is_ok_and(|v| v != "0" && !v.is_empty())
}

pub fn env_str(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|v| !v.is_empty())
}

/// Share in whole percent: statistics without f32 formatting (it weighs ~16 KB).
fn percent(ratio: f32) -> u32 {
    (ratio * 100.0).round() as u32
}

/// What a scenario asks of the window.
pub enum ScriptStep {
    Capture(String),
    Done,
}

/// An app request to the system, performed by the window (Wayland, X11) after the event.
pub enum Ask {
    Copy(String),
    Paste,
    OpenUrl(String),
}

/// Platform for the app during an event: text metrics, modifiers, request queue.
struct ShellHost<'a> {
    text: &'a Text,
    mods: Mods,
    asks: &'a RefCell<Vec<Ask>>,
}

impl TextMeasure for ShellHost<'_> {
    fn measure(&self, text: &str, font: &Font) -> TextMetrics {
        self.text.measure(text, font)
    }
}

impl Host for ShellHost<'_> {
    fn mods(&self) -> Mods {
        self.mods
    }

    fn set_clipboard(&self, text: &str) {
        self.asks.borrow_mut().push(Ask::Copy(text.to_string()));
    }

    fn request_paste(&self) {
        self.asks.borrow_mut().push(Ask::Paste);
    }

    fn open_url(&self, url: &str) -> bool {
        self.asks.borrow_mut().push(Ask::OpenUrl(url.to_string()));
        true
    }
}

/// Open a link without the portal: `xdg-open` (it picks the desktop's browser itself).
pub fn xdg_open(url: &str) {
    if let Err(e) = crate::sys::spawn(&["xdg-open", url]) {
        log!("uison: xdg-open: {e}");
    }
}

pub struct Shell {
    /// Wake-up for `Handle::post` from other threads (the window's `poll` waits on it).
    pub wake: Option<crate::sys::WakeFd>,
    pub rt: Runtime,
    pub text: Text,
    /// Modifiers held now (the window updates them from keyboard and mouse events).
    pub mods: Mods,
    asks: RefCell<Vec<Ask>>,
    /// Changed since the last redraw (points); `full` means the whole window.
    damage: Vec<Rect>,
    full: bool,
    pub cursor: Cursor,
    /// Timer tick deadline (caret) and whether a screen-frame tick is requested (animation).
    pub tick_at: Option<Instant>,
    pub frame_tick: bool,
    /// Animation ticks by timer rather than screen frames (UISON_TIMER=1, for comparison).
    pub timer_only: bool,
    pub stats: bool,
    stats_buf: String,
    pending_since: Option<Instant>,
    flashing: bool,
    flash_n: u32,
    hitmap: Option<String>,
    capture_dir: Option<String>,
    capture_n: u32,
    /// Screenshot without a scenario: 200 ms after the changes are drawn.
    pub capture_at: Option<Instant>,
    player: Option<Player>,
    pub script_at: Option<Instant>,
    pub started: Instant,
    pub first_drawn: bool,
    last_draw: Option<Instant>,
    gaps: Vec<f32>,
    pub dark: bool,
}

impl Shell {
    pub fn new(app: impl App + 'static, started: Instant) -> Shell {
        let text = Text::new();
        let asks = RefCell::new(Vec::new());
        let host = ShellHost { text: &text, mods: Mods::default(), asks: &asks };
        let rt = Runtime::new(Box::new(app), &host).unwrap_or_else(|e| {
            log!("uison: bad frame: {e}");
            std::process::exit(1)
        });
        let player = env_str("UISON_SCRIPT").and_then(|path| match std::fs::read_to_string(&path) {
            // UISON_TOUCH=1: scenario via touches (scroll inertia, finger taps).
            Ok(src) => Player::new(&src, env_flag("UISON_TOUCH")).map_err(|e| log!("uison: {e}")).ok(),
            Err(e) => {
                log!("uison: {path}: {e}");
                None
            }
        });
        let capture_dir = env_str("UISON_CAPTURE");
        if let Some(d) = &capture_dir {
            let _ = std::fs::create_dir_all(d);
        }
        let script_at = player.as_ref().map(|_| started + Duration::from_millis(1000));
        let mut rt = rt;
        let wake = crate::sys::WakeFd::new().map_err(|e| log!("uison: eventfd: {e}")).ok();
        if let Some(w) = &wake {
            let fd = w.fd();
            rt.app.set_waker(std::sync::Arc::new(move || crate::sys::WakeFd::wake(fd)));
        }
        let s = Shell {
            wake,
            rt,
            text,
            mods: Mods::default(),
            asks,
            damage: Vec::new(),
            full: true,
            cursor: Cursor::Arrow,
            tick_at: None,
            frame_tick: false,
            timer_only: env_flag("UISON_TIMER"),
            stats: env_flag("UISON_STATS"),
            stats_buf: String::new(),
            pending_since: None,
            flashing: env_flag("UISON_FLASH"),
            flash_n: 0,
            hitmap: env_str("UISON_HITMAP"),
            capture_dir,
            capture_n: 0,
            capture_at: None,
            player,
            script_at,
            started,
            first_drawn: false,
            last_draw: None,
            gaps: Vec::new(),
            dark: false,
        };
        s.write_hitmap();
        s
    }

    pub fn dispatch(&mut self, ev: Event) -> bool {
        let pos = ev.pos();
        let leave = ev == Event::MouseLeave;
        let changed = self.apply(&ev);
        if let Some(p) = pos {
            self.cursor = uison_frame::cursor_at(&self.rt.frame, p);
        } else if leave {
            self.cursor = Cursor::Arrow;
        }
        self.schedule_tick();
        changed
    }

    fn apply(&mut self, ev: &Event) -> bool {
        let t0 = Instant::now();
        let host = ShellHost { text: &self.text, mods: self.mods, asks: &self.asks };
        let up = match self.rt.dispatch(ev, &host) {
            Ok(Some(up)) => up,
            Ok(None) => return false,
            Err(e) => {
                log!("uison: bad frame: {e}");
                return false;
            }
        };
        self.write_hitmap();
        match &up.damage {
            Damage::None => {}
            Damage::Full => self.full = true,
            Damage::Rects(rs) => self.damage.extend_from_slice(rs),
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

    /// Wake-up fired: reset it and run what other threads deferred.
    pub fn woke(&mut self) {
        if let Some(w) = &self.wake {
            w.drain();
        }
        self.dispatch(Event::Wake);
    }

    /// App requests accumulated since last time.
    pub fn take_asks(&mut self) -> Vec<Ask> {
        std::mem::take(self.asks.get_mut())
    }

    /// The app asks for a tick: frequent (animation) follows screen frames, rare (caret) uses a timer.
    fn schedule_tick(&mut self) {
        let Some(d) = self.rt.app.wants_tick() else { return };
        if d <= FRAME_TICK && !self.timer_only {
            self.frame_tick = true;
            return;
        }
        let at = Instant::now() + d;
        if self.tick_at.is_none_or(|t| t > at) {
            self.tick_at = Some(at);
        }
    }

    /// Tick: from the timer or a screen frame.
    pub fn tick(&mut self) {
        self.tick_at = None;
        self.frame_tick = false;
        self.dispatch(Event::Tick);
        if !self.frame_tick {
            self.report_gaps();
        }
    }

    /// Desktop theme (settings portal); `true` if it changed.
    pub fn set_dark(&mut self, dark: bool) -> bool {
        if dark == self.dark {
            return false;
        }
        self.dark = dark;
        self.dispatch(Event::Appearance(dark));
        if self.stats {
            self.stat(format!("appearance: {}", if dark { "dark" } else { "light" }));
        }
        true
    }

    /// Desktop font; `true` if it changed and the frame was laid out again.
    pub fn set_font(&mut self, desc: &str) -> bool {
        if !self.text.set_system(desc) {
            return false;
        }
        // Same size: the app lays out the frame again with the new metrics.
        self.dispatch(Event::Resize(self.rt.frame.size));
        self.full = true;
        true
    }

    /// Redraw the whole window (new size, scale, buffer).
    pub fn invalidate(&mut self) {
        self.full = true;
    }

    pub fn has_damage(&self) -> bool {
        self.full || !self.damage.is_empty()
    }

    /// Take the accumulated areas: `None` means the whole window.
    pub fn take_damage(&mut self) -> Option<Vec<Rect>> {
        let full = std::mem::take(&mut self.full);
        let d = std::mem::take(&mut self.damage);
        if full { None } else { Some(d) }
    }

    /// Draw the areas (in points, already pixel-snapped) into the buffer image;
    /// `origin` is where the frame starts on the buffer (below the title bar of our own frame).
    pub fn paint(&mut self, cr: *mut Cr, scale: f32, origin: Point, rects: &[Rect]) {
        let t0 = Instant::now();
        if self.stats {
            if let Some(t) = self.last_draw.filter(|t| t.elapsed().as_millis() < 100) {
                self.gaps.push(t.elapsed().as_secs_f32() * 1000.0);
            }
            self.last_draw = Some(t0);
        }
        let mut area = 0.0;
        {
            let mut canvas = CairoCanvas::at(cr, &self.text, scale, origin);
            for &r in rects {
                area += r.intersect(&self.rt.frame.rect()).area();
                uison_frame::paint(&self.rt.frame, &mut canvas, r);
                if self.flashing {
                    self.flash_n = self.flash_n.wrapping_add(1);
                    let hue = [(255, 60, 60), (60, 200, 60), (60, 120, 255), (230, 180, 0)][self.flash_n as usize % 4];
                    canvas.rect(r, 0.0, Paint::fill(Color::rgba(hue.0, hue.1, hue.2, 70)));
                }
            }
        }
        // Screenshot only after redraws caused by a frame change; with a scenario,
        // only its named screenshots.
        let ours = !self.first_drawn || self.pending_since.is_some();
        if self.capture_dir.is_some() && self.player.is_none() && ours && self.capture_at.is_none() {
            self.capture_at = Some(Instant::now() + Duration::from_millis(200));
        }
        if self.stats {
            if !self.first_drawn {
                let line = format!("launch → first draw: {} ms", self.started.elapsed().as_millis());
                self.stat(line);
            }
            let latency = self.pending_since.map(|t| t.elapsed().as_micros());
            let line = format!(
                "draw: {} rect(s), {}% of frame, {}us{}",
                rects.len(),
                percent(area / self.rt.frame.rect().area().max(1.0)),
                t0.elapsed().as_micros(),
                latency.map(|l| format!(", event→drawn {l}us")).unwrap_or_default(),
            );
            self.stat(line);
        }
        self.pending_since = None;
        self.first_drawn = true;
    }

    /// After the frame is presented (commit): record statistics.
    pub fn presented(&mut self) {
        self.flush_stats();
    }

    pub fn stat(&mut self, line: String) {
        self.stats_buf.push_str(&line);
        self.stats_buf.push('\n');
    }

    pub fn flush_stats(&mut self) {
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

    /// UISON_HITMAP=file: hit area map of the current frame (`id x y w h` per line, points).
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

    /// Save the image of the last presented buffer (`name: None`: by number).
    pub fn save_capture(&mut self, image: &Image, name: Option<&str>) {
        self.flush_stats();
        let Some(dir) = &self.capture_dir else { return };
        let path = match name {
            Some(n) => format!("{dir}/{n}.png"),
            None => {
                self.capture_n += 1;
                format!("{dir}/{:04}.png", self.capture_n - 1)
            }
        };
        match image.save_png(&path) {
            Ok(()) => log!("capture: {path}"),
            Err(e) => log!("uison: capture failed: {e}"),
        }
    }

    /// Scenario step; the scenario's events have already been sent to the app.
    pub fn step_script(&mut self) -> Option<ScriptStep> {
        self.flush_stats();
        self.script_at = None;
        let player = self.player.as_mut()?;
        let (action, pause) = player.next(&self.rt.frame)?;
        self.script_at = Some(Instant::now() + pause.max(Duration::from_millis(1)));
        match action {
            Action::Event(ev) => {
                self.dispatch(ev);
                None
            }
            Action::Capture(name) => Some(ScriptStep::Capture(name)),
            Action::Log(m) => {
                log!("uison: {m}");
                None
            }
            Action::Wait => None,
            Action::Done => {
                log!("uison: done");
                self.script_at = None;
                Some(ScriptStep::Done)
            }
        }
    }

    /// Nearest deadline among the shell's timers.
    pub fn next_deadline(&self) -> Option<Instant> {
        [self.tick_at, self.script_at, self.capture_at].into_iter().flatten().min()
    }
}
