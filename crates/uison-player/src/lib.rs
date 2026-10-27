//! The player's common part: a program's frames in the App tab, the messages both ways in the JSON tab,
//! mouse and keys to the program. How the program is reached is a [`Link`] (a local process —
//! `uison-player`, a channel over SSH — `uison-ssh-player`); what the start screen offers is the
//! `Launcher` component, which each player replaces with its own.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use uison::time::now;
use uison::ui::{IntoJson, Json, Ui, obj};

pub use uison;
use uison::{App, Event, Frame, Host, Keyboard, Point, Rect, Size, Waker};

/// Ids of the program's hit areas in our frame.
const PREFIX: &str = "r/";
/// The log keeps this many messages; frames — only their beginning.
const LOG_MAX: usize = 5000;
const KEEP_FRAME: usize = 8 * 1024;
const KEEP_OTHER: usize = 64 * 1024;

/// The way to the program: a line to its stdin, and stopping it.
pub trait Link {
    /// `false` — the program is gone.
    fn send(&mut self, line: &str) -> bool;
    /// Close its stdin (a console program exits by itself) and make sure it ends.
    fn stop(&mut self);
}

/// What other threads (pipe readers, an SSH session) hand to the UI thread.
pub enum What {
    /// A line from the program's stdout: a frame, `hello`, a host request.
    Out(String),
    /// A line from its stderr.
    Err(String),
    /// The program ended (the status as text).
    Exit(String),
    /// The program runs: switch to it (its name for the title).
    Started(String),
    /// A line for the start screen: progress or an error.
    Info(String),
    /// A data value for the start screen (`connected`, `server`…).
    Set(String, String),
}

struct Incoming {
    /// The run it belongs to; `None` — not tied to a run (connection, upload).
    generation: Option<u32>,
    what: What,
}

type Inbox = Arc<Mutex<Vec<Incoming>>>;
type SharedWaker = Arc<Mutex<Option<Waker>>>;

/// Hands [`What`] to the UI thread from any thread and wakes it.
#[derive(Clone)]
pub struct Poster {
    generation: Option<u32>,
    inbox: Inbox,
    waker: SharedWaker,
}

impl Poster {
    pub fn post(&self, what: What) {
        self.inbox.lock().unwrap().push(Incoming { generation: self.generation, what });
        if let Some(w) = self.waker.lock().unwrap().as_ref() {
            w();
        }
    }
}

/// The player's state, shared with the start screen's handlers.
pub type Shared = Rc<RefCell<State>>;

struct Entry {
    id: u64,
    /// `true` — from the program.
    from: bool,
    t: Duration,
    kind: String,
    bytes: usize,
    text: String,
}

#[derive(Default)]
struct Stats {
    msgs_in: u64,
    bytes_in: u64,
    msgs_out: u64,
    bytes_out: u64,
    frames: u64,
    /// Frames in the last second: for frames per second.
    recent: VecDeque<Duration>,
    /// Event sent → next frame, ms × 10.
    latency: VecDeque<u32>,
    shown_at: Duration,
}

pub struct State {
    link: Option<Box<dyn Link>>,
    /// Run number: messages of earlier runs are ignored.
    generation: u32,
    inbox: Inbox,
    waker: SharedWaker,
    /// The size last sent to the program.
    sent: Size,
    /// The program's last frame and where it is drawn.
    remote: Option<Frame>,
    rect: Rect,
    /// The program is laid out at least this wide (a phone: a desktop program fits, smaller);
    /// `scale` — our points per its point.
    min_width: f32,
    scale: f32,
    entries: VecDeque<Entry>,
    next_id: u64,
    stats: Stats,
    /// An event went out and no frame came back yet.
    waiting_since: Option<Duration>,
    start: Duration,
}

impl State {
    /// A new run: the previous program is stopped, its picture forgotten; the poster's messages belong to
    /// this run. Give the program to [`State::attach`] once it starts.
    pub fn begin(&mut self, what: &str) -> Poster {
        self.stop();
        self.generation += 1;
        self.remote = None;
        self.sent = Size::new(0.0, 0.0);
        self.waiting_since = None;
        self.log(false, "start".into(), what);
        Poster { generation: Some(self.generation), inbox: self.inbox.clone(), waker: self.waker.clone() }
    }

    pub fn attach(&mut self, link: Box<dyn Link>) {
        self.link = Some(link);
    }

    /// For messages not tied to a run: connecting, uploading.
    pub fn poster(&self) -> Poster {
        Poster { generation: None, inbox: self.inbox.clone(), waker: self.waker.clone() }
    }

    /// Stop the program; its last lines and exit status no longer matter.
    pub fn stop(&mut self) {
        if let Some(mut link) = self.link.take() {
            link.stop();
            self.generation += 1;
        }
    }

    /// Lay the program out at least `w` points wide and draw it smaller if the window is narrower
    /// (desktop programs on a phone); 0 — always at the window's size.
    pub fn set_min_width(&mut self, w: f32) {
        self.min_width = w;
    }

    pub fn running(&self) -> bool {
        self.link.is_some()
    }

    /// A line to the program (an event, a message typed in the JSON tab).
    pub fn send(&mut self, line: &str) {
        let Some(link) = &mut self.link else { return };
        if !link.send(line) {
            self.link = None;
            return;
        }
        self.stats.msgs_out += 1;
        self.stats.bytes_out += line.len() as u64 + 1;
        let kind = kind_of(line, false);
        self.log(false, kind, line);
    }

    fn event(&mut self, e: &Event) {
        // Latency is the person's: from the mouse and keys (a resize may wait for the program to start).
        if matches!(e, Event::MouseMove(_) | Event::MouseDown(..) | Event::MouseUp(..) | Event::Scroll(..) | Event::Key(_)) {
            self.waiting_since.get_or_insert(now());
        }
        self.send(&uison::event::to_json(e));
    }

    fn log(&mut self, from: bool, kind: String, line: &str) {
        let keep = if kind == "frame" { KEEP_FRAME } else { KEEP_OTHER };
        let mut text = line.to_string();
        if text.len() > keep {
            let mut cut = keep;
            while !text.is_char_boundary(cut) {
                cut -= 1;
            }
            text.truncate(cut);
            text.push_str(" …");
        }
        self.next_id += 1;
        self.entries.push_back(Entry { id: self.next_id, from, t: now().saturating_sub(self.start), kind, bytes: line.len(), text });
        if self.entries.len() > LOG_MAX {
            self.entries.pop_front();
        }
    }
}

/// The kind of a message for the log: `frame`, `hello`, `host copy`, `move`, `action`…
fn kind_of(line: &str, from: bool) -> String {
    let Ok(v) = uison::json::parse(line) else { return "text".into() };
    if from {
        if let Some(h) = v.get("host").and_then(|h| h.as_str()) {
            return format!("host {h}");
        }
        let first = v.as_object().and_then(|o| o.first()).map(|(k, _)| k.to_string()).unwrap_or_default();
        return if first == "uison" { "hello".into() } else { first };
    }
    match v.get("type").and_then(|t| t.as_str()) {
        Some(t) => t.to_string(),
        None if v.get("action").is_some() => "action".into(),
        None if v.get("data").is_some() => "data".into(),
        None => "message".into(),
    }
}

struct Player {
    ui: Ui,
    st: Rc<RefCell<State>>,
    /// The pointer is over the program (or a press started there and the button is still down).
    inside: bool,
    pressed: bool,
    paste_for_program: bool,
    /// Log rows already in the data (when the JSON tab is open).
    shown: u64,
}

impl Player {
    /// The App tab with a running program: mouse and keys go to it.
    fn tab_app(&self) -> bool {
        self.ui.data.str("tab") != "json" && self.ui.data.bool("running")
    }

    /// Messages from the reader threads: frames, host requests, stderr, the program's exit.
    fn drain(&mut self, host: &dyn Host) {
        let mut st = self.st.borrow_mut();
        let st = &mut *st;
        let incoming = std::mem::take(&mut *st.inbox.lock().unwrap());
        for m in incoming {
            if m.generation.is_some_and(|g| g != st.generation) {
                continue;
            }
            match m.what {
                What::Out(line) => {
                    st.stats.msgs_in += 1;
                    st.stats.bytes_in += line.len() as u64 + 1;
                    let kind = kind_of(&line, true);
                    if kind == "frame" {
                        take_frame(&mut self.ui, st, &line, host);
                    } else if let Ok(v) = uison::json::parse(&line) {
                        if let Some(title) = v.get("title").and_then(|t| t.as_str()) {
                            self.ui.data.set("title", title);
                        }
                        match v.get("host").and_then(|h| h.as_str()) {
                            Some("copy") => host.set_clipboard(v.get("text").and_then(|t| t.as_str()).unwrap_or("")),
                            Some("open") => {
                                host.open_url(v.get("url").and_then(|t| t.as_str()).unwrap_or(""));
                            }
                            Some("paste") => {
                                self.paste_for_program = true;
                                host.request_paste();
                            }
                            _ => {}
                        }
                    }
                    st.log(true, kind, &line);
                }
                What::Err(line) => st.log(true, "stderr".into(), &line),
                What::Exit(status) => {
                    st.link = None;
                    st.log(true, "exit".into(), &status);
                    // Back to the start screen, with what happened.
                    self.ui.data.set("status", format!("{} exited: {status}", self.ui.data.str("title")));
                    self.ui.data.set("running", false);
                    self.ui.data.set("title", "");
                }
                What::Started(name) => {
                    self.ui.data.set("title", name);
                    self.ui.data.set("running", true);
                    self.ui.data.set("status", "");
                    self.ui.data.set("tab", "app");
                }
                What::Info(text) => self.ui.data.set("status", text),
                What::Set(path, value) => self.ui.data.set(&path, value),
            }
        }
    }

}

/// A frame from the program: parsed for drawing, counted for the status line.
fn take_frame(ui: &mut Ui, st: &mut State, line: &str, host: &dyn Host) {
    // `{"frame":{…}}` — the frame is the object after the first colon.
    let body = line.strip_prefix("{\"frame\":").and_then(|b| b.strip_suffix('}')).unwrap_or("");
    match Frame::parse(body, host) {
        Ok(f) => st.remote = Some(f),
        Err(e) => st.log(true, "bad frame".into(), &e.to_string()),
    }
    let t = now();
    st.stats.frames += 1;
    st.stats.recent.push_back(t);
    if let Some(since) = st.waiting_since.take() {
        st.stats.latency.push_back((t.saturating_sub(since).as_micros() / 100) as u32);
        if st.stats.latency.len() > 100 {
            st.stats.latency.pop_front();
        }
    }
    // Our frame must be rebuilt with the new picture.
    ui.data.update("remoteVersion", |v| v.as_f64().unwrap_or(0.0) + 1.0);
}

impl Player {
    /// The status line, at most twice a second.
    fn stats(&mut self) {
        let mut st = self.st.borrow_mut();
        let t = now();
        if t.saturating_sub(st.stats.shown_at) < Duration::from_millis(500) {
            return;
        }
        let s = &mut st.stats;
        s.shown_at = t;
        while s.recent.front().is_some_and(|&f| t.saturating_sub(f) > Duration::from_secs(1)) {
            s.recent.pop_front();
        }
        let mut lat: Vec<u32> = s.latency.iter().copied().collect();
        lat.sort_unstable();
        let p50 = lat.get(lat.len() / 2).map_or("–".into(), |v| format!("{}.{} ms", v / 10, v % 10));
        let text = format!(
            "in {} msgs {} KB · out {} msgs {} KB · {} frames · {}/s · latency p50 {p50}",
            s.msgs_in,
            s.bytes_in / 1024,
            s.msgs_out,
            s.bytes_out / 1024,
            s.frames,
            s.recent.len()
        );
        drop(st);
        self.ui.data.set("stats", text);
    }

    /// The JSON tab's list: new rows, or all of them again (`all`: a filter changed, the log was trimmed).
    fn sync_log(&mut self, all: bool) {
        if self.tab_app() {
            return;
        }
        let st = self.st.borrow();
        // The list in the data is trimmed with the log.
        let all = all || self.ui.data.len("log") > LOG_MAX;
        if all {
            self.ui.data.set("log", Json::Array(Vec::new()));
            self.shown = 0;
        }
        let frames = self.ui.data.bool("frames");
        let moves = self.ui.data.bool("moves");
        let rows: Vec<Json> = st
            .entries
            .iter()
            .filter(|e| e.id > self.shown)
            .filter(|e| (frames || e.kind != "frame") && (moves || e.kind != "move"))
            .map(row)
            .collect();
        let last = st.entries.back().map_or(self.shown, |e| e.id);
        drop(st);
        for r in rows {
            self.ui.data.push("log", r);
        }
        self.shown = last;
    }

    fn content_resized(&mut self) {
        let mut st = self.st.borrow_mut();
        let size = Size::new(st.rect.w / st.scale, st.rect.h / st.scale);
        if self.tab_app() && size.w > 0.0 && size != st.sent {
            st.sent = size;
            st.event(&Event::Resize(size));
        }
    }

    fn over_program(&self, p: Point) -> bool {
        let r = self.st.borrow().rect;
        self.tab_app() && p.x >= r.x && p.y >= r.y && p.x < r.x + r.w && p.y < r.y + r.h
    }

    /// Mouse, wheel and keys for the program; `true` — the event went there.
    fn forward(&mut self, e: &Event) -> bool {
        let (r, k) = {
            let st = self.st.borrow();
            (st.rect, st.scale)
        };
        let moved = |p: Point| Point::new((p.x - r.x) / k, (p.y - r.y) / k);
        let out = match e {
            Event::MouseDown(p, b) if self.over_program(*p) => {
                self.pressed = true;
                Event::MouseDown(moved(*p), *b)
            }
            Event::MouseUp(p, b) if self.pressed => {
                self.pressed = false;
                Event::MouseUp(moved(*p), *b)
            }
            Event::MouseMove(p) if self.pressed || self.over_program(*p) => Event::MouseMove(moved(*p)),
            Event::Scroll(p, dx, dy) if self.over_program(*p) => Event::Scroll(moved(*p), *dx / k, *dy / k),
            Event::Key(_) if self.tab_app() => e.clone(),
            Event::Paste(_) if self.paste_for_program => {
                self.paste_for_program = false;
                e.clone()
            }
            _ => return false,
        };
        self.st.borrow_mut().event(&out);
        true
    }
}

fn row(e: &Entry) -> Json {
    let ms = e.t.as_millis();
    let size = if e.bytes >= 1024 { format!("{} KB", e.bytes / 1024) } else { format!("{} B", e.bytes) };
    let preview: String = e.text.chars().take(240).collect();
    obj([
        ("id", (e.id as f64).into_json()),
        ("dir", if e.from { "←" } else { "→" }.into_json()),
        ("color", if e.kind == "stderr" || e.kind == "exit" { "#F0A06A" } else if e.from { "#7FB4FF" } else { "#8FD18F" }.into_json()),
        ("bg", if e.id.is_multiple_of(2) { "#FFFFFF06" } else { "#00000000" }.into_json()),
        ("t", format!("{}.{:03}", ms / 1000, ms % 1000).into_json()),
        ("kind", e.kind.as_str().into_json()),
        ("size", size.into_json()),
        ("text", preview.into_json()),
    ])
}

/// A message for the detail pane: small ones pretty-printed, frames as they came (cut).
fn detail(e: &Entry) -> String {
    let head = format!("{} {} · {} · {} bytes\n\n", if e.from { "←" } else { "→" }, e.kind, e.t.as_millis(), e.bytes);
    let body = match uison::json::parse(&e.text) {
        // Indents as no-break spaces: the text node doesn't keep leading spaces.
        Ok(v) if e.text.len() < 16 * 1024 => v
            .to_json_pretty()
            .lines()
            .map(|l| {
                let t = l.trim_start();
                "\u{a0}".repeat(l.len() - t.len()) + t
            })
            .collect::<Vec<_>>()
            .join("\n"),
        _ => e.text.clone(),
    };
    head + &body
}

impl App for Player {
    fn frame(&mut self, host: &dyn Host) -> String {
        let f = self.ui.frame(host);
        self.content_resized();
        f
    }

    fn on_event(&mut self, e: &Event, hit: Option<&str>, host: &dyn Host) -> Option<String> {
        // Data changed here, before `Ui` sees the event, doesn't make it rebuild the frame by itself.
        let version = self.ui.data.version();
        if matches!(e, Event::Wake) {
            self.drain(host);
            self.sync_log(false);
            self.stats();
        }
        if let Event::Appearance(_) = e {
            self.st.borrow_mut().event(e);
        }
        let mut frame = None;
        if self.forward(e) {
            // Leaving our own controls for the program's area: let them lose hover.
            if !self.inside {
                self.inside = true;
                frame = self.ui.on_event(&Event::MouseLeave, None, host);
            }
        } else {
            if self.inside && matches!(e, Event::MouseMove(_) | Event::MouseLeave) {
                self.inside = false;
                self.st.borrow_mut().event(&Event::MouseLeave);
            }
            let hit = hit.filter(|h| !h.starts_with(PREFIX));
            frame = self.ui.on_event(e, hit, host);
        }
        if frame.is_none() && self.ui.data.version() != version {
            frame = Some(self.ui.frame(host));
        }
        if frame.is_some() {
            self.content_resized();
        }
        frame
    }

    fn wants_tick(&self) -> Option<Duration> {
        self.ui.wants_tick()
    }

    fn wants_keyboard(&self) -> Option<Keyboard> {
        self.ui.wants_keyboard()
    }

    fn set_waker(&mut self, wake: Waker) {
        let st = self.st.borrow();
        *st.waker.lock().unwrap() = Some(wake.clone());
        self.ui.set_waker(wake.clone());
        if !st.inbox.lock().unwrap().is_empty() {
            wake();
        }
    }
}

/// The player's window. `launcher` replaces the start screen's component (`{ body: … }`), `setup`
/// adds its handlers; the state is shared with them.
pub fn build(launcher: Option<&str>, setup: impl FnOnce(&mut Ui, &Shared)) -> PlayerApp {
    let st = Rc::new(RefCell::new(State {
        link: None,
        generation: 0,
        inbox: Arc::default(),
        waker: Arc::default(),
        sent: Size::new(0.0, 0.0),
        remote: None,
        rect: Rect::new(0.0, 0.0, 0.0, 0.0),
        min_width: 0.0,
        scale: 1.0,
        entries: VecDeque::new(),
        next_id: 0,
        stats: Stats::default(),
        waiting_since: None,
        start: now(),
    }));
    let mut ui = Ui::new(include_str!("../player.json")).unwrap_or_else(|e| panic!("player.json: {e}"));
    if let Some(def) = launcher {
        ui.component("Launcher", def).unwrap_or_else(|e| panic!("Launcher: {e}"));
    }
    let s = st.clone();
    ui.draw("remote", move |d| {
        let mut s = s.borrow_mut();
        s.rect = d.rect;
        s.scale = if s.min_width > d.rect.w { d.rect.w / s.min_width } else { 1.0 };
        match &s.remote {
            Some(f) => d.w.frame(f, Point::new(d.rect.x, d.rect.y), s.scale, PREFIX),
            None => {
                d.w.text("Waiting for the program's first frame…", Point::new(d.rect.x + 16.0, d.rect.y + 16.0))
                    .size(13.0)
                    .color(uison::Color::rgb(150, 150, 160));
            }
        }
    });
    let s = st.clone();
    ui.on("stop", move |cx| {
        s.borrow_mut().stop();
        cx.data.set("running", false);
        cx.data.set("status", format!("{} stopped.", cx.data.str("title")));
        cx.data.set("title", "");
    });
    let s = st.clone();
    ui.on("pick", move |cx| {
        let id: u64 = cx.key.trim_start_matches("m-").parse().unwrap_or(0);
        if let Some(e) = s.borrow().entries.iter().find(|e| e.id == id) {
            cx.data.set("detail", detail(e));
        }
    });
    let s = st.clone();
    ui.on("send", move |cx| {
        let line = cx.data.str("send").trim().to_string();
        if !line.is_empty() {
            s.borrow_mut().send(&line);
            cx.data.set("send", "");
        }
    });
    let s = st.clone();
    ui.on("clear", move |cx| {
        s.borrow_mut().entries.clear();
        cx.data.set("log", Json::Array(Vec::new()));
        cx.data.set("detail", "");
    });
    // The list is rebuilt by the player after these (see `PlayerApp`).
    ui.on("filter", |_| {});
    ui.on("tab", |_| {});
    setup(&mut ui, &st);
    PlayerApp(Player { ui, st, inside: false, pressed: false, paste_for_program: false, shown: 0 })
}

/// The player as an [`App`]; it also rebuilds the JSON tab's list after a filter or tab change and after
/// a clear (handlers can't reach the player).
pub struct PlayerApp(Player);

impl App for PlayerApp {
    fn frame(&mut self, host: &dyn Host) -> String {
        self.0.frame(host)
    }

    fn on_event(&mut self, e: &Event, hit: Option<&str>, host: &dyn Host) -> Option<String> {
        let p = &mut self.0;
        let before = (p.ui.data.str("tab").to_string(), p.ui.data.bool("frames"), p.ui.data.bool("moves"), p.ui.data.len("log"));
        let mut frame = p.on_event(e, hit, host);
        let after = (p.ui.data.str("tab").to_string(), p.ui.data.bool("frames"), p.ui.data.bool("moves"), p.ui.data.len("log"));
        if before != after {
            let cleared = after.3 == 0 && before.3 != 0;
            if cleared {
                p.shown = p.st.borrow().entries.back().map_or(0, |e| e.id);
            } else {
                p.sync_log(true);
            }
            // The list changed after the frame was built: build it again.
            frame = Some(p.ui.frame(host));
            p.content_resized();
        }
        frame
    }

    fn wants_tick(&self) -> Option<Duration> {
        self.0.wants_tick()
    }

    fn wants_keyboard(&self) -> Option<Keyboard> {
        self.0.wants_keyboard()
    }

    fn set_waker(&mut self, wake: Waker) {
        self.0.set_waker(wake);
    }
}
