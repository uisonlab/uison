//! Agent mode: an LLM agent works with a uison program without seeing it. It reads the meaning of the
//! screen (`Ui::semantics`: roles, names, states, keys) and acts on it — the actions go through the
//! same path as a person's input (press and release at the node, keys), so a disabled button or one
//! under a modal can't be pressed.
//!
//! Two ways in, both on stdin/stdout, one message per line:
//!
//! ```text
//! uison_agent::run(app(), "Todo")   JSON lines:
//!   ← {"uison":0,"title":"Todo","agent":1}
//!   → {"id":1,"query":"tree"}                         ← {"id":1,"seq":1,"tree":[…]}
//!   → {"id":2,"do":"click","key":"add"}               ← {"id":2,"ok":true,"seq":2,"changes":{…}}
//!   → {"id":3,"do":"type","name":"Email","text":"a@b.c"}
//!   → {"id":4,"query":"data","path":"todos"}
//! uison_agent::mcp(app(), "Todo")   MCP (JSON-RPC) — tools for Claude Code and other agents
//! ```
//!
//! Time is virtual: it moves only while the program settles after an action (animations and timers
//! play out at once) and on `wait`. Between the agent's steps the program stands still; work in other
//! threads still arrives (`Handle::post`) and is seen on the next step.

mod act;
mod mcp;
mod tree;

use std::cell::{Cell, RefCell};
use std::io::{BufRead, Write};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use uison_core::{App, Event, Font, Host, Mods, Size, TextMeasure, TextMetrics};
use uison_frame::Frame;
use uison_ui::{Json, Ui};

pub use tree::{diff, to_text};

/// How long the program may settle after an action (virtual time: animations, tooltip delay).
const SETTLE: Duration = Duration::from_millis(1000);
/// Window size until the agent asks for another.
const SIZE: Size = Size { w: 1280.0, h: 800.0 };

// ---------------------------------------------------------------- virtual clock

static CLOCK: AtomicU64 = AtomicU64::new(0);

fn clock() -> Duration {
    Duration::from_micros(CLOCK.load(Ordering::Relaxed))
}

/// Switch the program to virtual time, continuing from the real time so far.
fn start_clock() {
    if CLOCK.load(Ordering::Relaxed) == 0 {
        CLOCK.store(uison_core::time::now().as_micros() as u64 + 1, Ordering::Relaxed);
        uison_core::time::set_clock(clock);
    }
}

fn advance(d: Duration) {
    CLOCK.fetch_add(d.as_micros() as u64, Ordering::Relaxed);
}

// ---------------------------------------------------------------- host

/// Text is measured by the given metrics; the system's clipboard, links and dialogs become notices
/// for the agent (it has no screen and no system to show them).
struct AgentHost<'h> {
    m: &'h dyn Host,
    notices: RefCell<Vec<Json>>,
    clipboard: RefCell<String>,
    paste: Cell<bool>,
}

impl AgentHost<'_> {
    fn notice(&self, host: &str, key: &str, value: &str) {
        let mut n = Json::object();
        n.set("host", host.to_string());
        if !key.is_empty() {
            n.set(key, value.to_string());
        }
        self.notices.borrow_mut().push(n);
    }
}

impl TextMeasure for AgentHost<'_> {
    fn measure(&self, text: &str, font: &Font) -> TextMetrics {
        self.m.measure(text, font)
    }
}

impl Host for AgentHost<'_> {
    fn mods(&self) -> Mods {
        Mods::default()
    }

    fn set_clipboard(&self, text: &str) {
        *self.clipboard.borrow_mut() = text.to_string();
        self.notice("copy", "text", text);
    }

    fn request_paste(&self) {
        self.paste.set(true);
    }

    fn open_url(&self, url: &str) -> bool {
        self.notice("open", "url", url);
        true
    }

    fn pick_file(&self, action: &str) -> bool {
        // The agent answers with a message: {"action": <action>, "key": <path>}.
        self.notice("pick_file", "action", action);
        true
    }

    fn image_size(&self, src: &str) -> Option<Size> {
        self.m.image_size(src)
    }
}

// ---------------------------------------------------------------- agent

/// Who to tell that another thread posted work (the loop waiting on stdin).
type Notify = Arc<Mutex<Option<Box<dyn Fn() + Send>>>>;

/// A program driven by an agent: commands in, answers out (`handle`), no window.
pub struct Agent<'h> {
    ui: Ui,
    host: AgentHost<'h>,
    frame: Frame,
    /// The tree as the agent last saw it: changes are counted from it.
    seen: Json,
    seq: usize,
    woke: Arc<AtomicBool>,
    notify: Notify,
}

impl<'h> Agent<'h> {
    /// Text is measured with `m` (`uison_frame::console::Estimate` without fonts — the agent sees no pixels).
    pub fn new(mut ui: Ui, m: &'h dyn Host) -> Agent<'h> {
        start_clock();
        let woke = Arc::new(AtomicBool::new(false));
        let notify: Notify = Arc::new(Mutex::new(None));
        let (w, n) = (woke.clone(), notify.clone());
        ui.set_waker(Arc::new(move || {
            w.store(true, Ordering::SeqCst);
            if let Some(f) = n.lock().unwrap().as_ref() {
                f();
            }
        }));
        let host = AgentHost { m, notices: RefCell::new(Vec::new()), clipboard: RefCell::new(String::new()), paste: Cell::new(false) };
        let _ = ui.on_event(&Event::Resize(SIZE), None, &host);
        let src = ui.frame(&host);
        let frame = Frame::parse(&src, &host).unwrap_or_else(|_| Frame::parse("{\"size\":[1,1]}", &host).unwrap());
        let mut a = Agent { ui, host, frame, seen: Json::Array(Vec::new()), seq: 0, woke, notify };
        a.settle(SETTLE);
        a
    }

    /// Called from another thread when the program has work for its UI thread (`Handle::post`).
    pub fn on_wake(&self, f: impl Fn() + Send + 'static) {
        *self.notify.lock().unwrap() = Some(Box::new(f));
    }

    pub fn ui(&mut self) -> &mut Ui {
        &mut self.ui
    }

    /// Run what other threads posted (`Handle::post`) and let the program settle.
    pub fn wake(&mut self) {
        self.settle(SETTLE);
    }

    /// One event the way a backend sends it: hit test against the last frame, a new frame.
    fn send(&mut self, ev: Event) {
        let hit = ev.pos().and_then(|p| uison_frame::hit_test(&self.frame, p)).map(str::to_string);
        if let Some(src) = self.ui.on_event(&ev, hit.as_deref(), &self.host) {
            self.set_frame(&src);
        }
        if self.host.paste.take() {
            let text = self.host.clipboard.borrow().clone();
            self.send(Event::Paste(text.into()));
        }
    }

    fn set_frame(&mut self, src: &str) {
        match Frame::parse(src, &self.host) {
            Ok(f) => self.frame = f,
            Err(e) => eprintln!("uison-agent: bad frame: {e}"),
        }
    }

    /// Let the program finish what an action started: work from other threads, animations, timers
    /// and delays (a tooltip) — in virtual time, at once, up to `budget`.
    fn settle(&mut self, budget: Duration) {
        let mut left = budget;
        for _ in 0..10_000 {
            if self.woke.swap(false, Ordering::SeqCst) {
                self.send(Event::Wake);
                continue;
            }
            match self.ui.wants_tick() {
                Some(d) if d <= left => {
                    let d = d.max(Duration::from_millis(1));
                    advance(d);
                    left -= d;
                    self.send(Event::Tick);
                }
                _ => break,
            }
        }
        advance(left);
    }

    /// The current tree (with rectangles), as `Ui::semantics` gives it.
    pub fn tree(&mut self) -> Json {
        self.ui.semantics(&self.host)
    }

    /// One command (a JSON line) → the answer (a JSON line), or `None` for a message without `id`
    /// sent the console way (`{"type":…}`, `{"action":…}`, `{"data":…}`).
    pub fn handle(&mut self, line: &str) -> Option<String> {
        let msg = match uison_json::parse(line) {
            Ok(v) => v.into_owned(),
            Err(e) => return Some(error_answer(None, &format!("bad JSON: {e}"))),
        };
        let id = msg.get("id").cloned();
        let answer = self.command(&msg);
        if id.is_none() && msg.get("query").is_none() && msg.get("do").is_none() {
            return None;
        }
        let mut out = match answer {
            Ok(a) => a,
            Err(e) => return Some(error_answer(id, &e)),
        };
        if let Some(id) = id {
            out.set("id", id);
        }
        Some(out.to_json())
    }

    /// A command as JSON → the answer object (without `id`).
    pub fn command(&mut self, msg: &Json) -> Result<Json, String> {
        if let Some(q) = msg.get("query").and_then(Json::as_str) {
            return self.query(q, msg);
        }
        if msg.get("do").is_some() {
            self.act(msg)?;
            return Ok(self.answer());
        }
        // The console protocol: an event, an action by name, a data change.
        let ev = uison_frame::event::from_value(msg).map_err(|e| e.to_string())?;
        self.send(ev);
        self.settle(SETTLE);
        Ok(self.answer())
    }

    fn query(&mut self, q: &str, msg: &Json) -> Result<Json, String> {
        let mut out = Json::object();
        match q {
            "tree" | "text" => {
                let tree = self.tree();
                let rects = msg.get("rects").and_then(Json::as_bool).unwrap_or(false);
                let under = msg.get("under").and_then(Json::as_str);
                let view = tree::view(&tree, rects, under);
                self.seen = tree;
                self.seq += 1;
                out.set("seq", self.seq);
                if q == "text" {
                    out.set("text", to_text(&view));
                } else {
                    out.set("tree", view);
                }
            }
            "data" => {
                let path = msg.get("path").and_then(Json::as_str).unwrap_or("");
                let v = if path.is_empty() { Some(self.ui.data.root()) } else { self.ui.data.get(path) };
                let v = v.ok_or_else(|| format!("no data at '{path}'"))?;
                out.set("data", fit(v, DATA_MAX));
            }
            "actions" => {
                let names: Vec<Json> = self.ui.actions().into_iter().map(|a| Json::from(a.to_string())).collect();
                out.set("actions", names);
            }
            "frame" => out.set("frame", uison_json::parse(&self.ui.frame(&self.host)).map(|v| v.into_owned()).unwrap_or(Json::Null)),
            _ => return Err(format!("unknown query '{q}' (tree, text, data, actions, frame)")),
        }
        Ok(out)
    }

    /// After an action: what changed since the agent last looked, and what the program asked of the system.
    fn answer(&mut self) -> Json {
        let tree = self.tree();
        let mut out = Json::object();
        out.set("ok", true);
        if let Some(d) = tree::diff(&self.seen, &tree) {
            out.set("changes", d);
        }
        self.seen = tree;
        self.seq += 1;
        out.set("seq", self.seq);
        let notices = std::mem::take(&mut *self.host.notices.borrow_mut());
        if !notices.is_empty() {
            out.set("host", notices);
        }
        out
    }
}

/// Longest data answer (bytes of JSON): a mail app keeps thousands of messages in its data.
const DATA_MAX: usize = 16 * 1024;

/// Data that fits in `max`: as is, or an outline — objects keep their keys with shortened values,
/// long arrays their length and first items (`"…": "1997 more"`). The agent asks for a path to see more.
fn fit(v: &Json, max: usize) -> Json {
    if v.to_json().len() <= max {
        return v.clone();
    }
    match v {
        Json::Object(fields) => {
            let each = (max / fields.len().max(1)).max(64);
            let mut o = Json::object();
            for (k, x) in fields {
                o.set(k, fit(x, each));
            }
            o
        }
        Json::Array(items) => {
            let mut a = Vec::new();
            let mut used = 0;
            for x in items {
                let x = fit(x, max / 4);
                used += x.to_json().len();
                if used > max && !a.is_empty() {
                    break;
                }
                a.push(x);
            }
            if a.len() < items.len() {
                let mut more = Json::object();
                more.set("…", format!("{} more of {} (read_data with a path: \"list.5\")", items.len() - a.len(), items.len()));
                a.push(more);
            }
            Json::Array(a)
        }
        Json::String(s) => Json::from(s.chars().take(max / 2).collect::<String>() + "…"),
        other => other.clone(),
    }
}

fn error_answer(id: Option<Json>, e: &str) -> String {
    let mut out = Json::object();
    if let Some(id) = id {
        out.set("id", id);
    }
    out.set("error", e.to_string());
    out.to_json()
}

// ---------------------------------------------------------------- loops

enum Msg {
    Line(String),
    Wake,
    End,
}

/// Lines of stdin and wakes from other threads in one channel.
fn input(agent: &Agent) -> mpsc::Receiver<Msg> {
    let (tx, rx) = mpsc::channel();
    let wake = Mutex::new(tx.clone());
    agent.on_wake(move || {
        let _ = wake.lock().unwrap().send(Msg::Wake);
    });
    std::thread::spawn(move || {
        for line in std::io::stdin().lock().lines().map_while(Result::ok) {
            if tx.send(Msg::Line(line)).is_err() {
                return;
            }
        }
        let _ = tx.send(Msg::End);
    });
    rx
}

fn write_line(out: &mut dyn Write, s: &str) -> bool {
    let mut line = String::with_capacity(s.len() + 1);
    line.push_str(s);
    line.push('\n');
    out.write_all(line.as_bytes()).and_then(|_| out.flush()).is_ok()
}

/// Agent mode with JSON lines on stdin/stdout (the module docs). Returns when stdin closes.
pub fn run(ui: Ui, title: &str) -> ! {
    let mut out = uison_frame::console::protocol_stdout();
    let m = uison_frame::console::Estimate;
    let mut agent = Agent::new(ui, &m);
    let rx = input(&agent);
    let mut hello = String::from("{\"uison\":0,\"agent\":1,\"title\":");
    uison_json::write_str(&mut hello, title);
    hello.push('}');
    write_line(&mut out, &hello);
    while let Ok(msg) = rx.recv() {
        match msg {
            Msg::Line(l) if l.trim().is_empty() => {}
            Msg::Line(l) => {
                if let Some(a) = agent.handle(&l)
                    && !write_line(&mut out, &a)
                {
                    break;
                }
            }
            Msg::Wake => agent.wake(),
            Msg::End => break,
        }
    }
    std::process::exit(0)
}

/// Agent mode as an MCP server (JSON-RPC on stdin/stdout): `claude mcp add todo -- todo --mcp`.
pub fn mcp(ui: Ui, title: &str) -> ! {
    let mut out = uison_frame::console::protocol_stdout();
    let m = uison_frame::console::Estimate;
    let mut agent = Agent::new(ui, &m);
    let rx = input(&agent);
    let mut server = mcp::Server::new(title);
    while let Ok(msg) = rx.recv() {
        match msg {
            Msg::Line(l) if l.trim().is_empty() => {}
            Msg::Line(l) => {
                if let Some(a) = server.handle(&mut agent, &l)
                    && !write_line(&mut out, &a)
                {
                    break;
                }
            }
            Msg::Wake => agent.wake(),
            Msg::End => break,
        }
    }
    std::process::exit(0)
}

/// Read the program's arguments: `--agent` — JSON lines, `--mcp` — MCP server. Removes the flag;
/// call it before handling the rest of the arguments.
pub fn mode(args: &mut Vec<std::ffi::OsString>) -> Option<fn(Ui, &str) -> !> {
    for (flag, f) in [("--agent", run as fn(Ui, &str) -> !), ("--mcp", mcp as fn(Ui, &str) -> !)] {
        if let Some(i) = args.iter().position(|a| a == flag) {
            args.remove(i);
            return Some(f);
        }
    }
    None
}

#[cfg(test)]
mod tests;
