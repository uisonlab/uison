//! Console mode: no window — frames go out to stdout and events come in from stdin, one JSON message
//! per line (a player shows the frames and sends the events, see `crates/uison-player`).
//!
//! ```text
//! out: {"uison":0,"title":"Disk Usage"}            first
//!      {"frame":{"size":[1280,840],…}}             after every change, strict JSON on one line
//!      {"host":"copy","text":"…"}  {"host":"open","url":"…"}  {"host":"paste"}
//! in:  events in the form of crate::event: {"type":"resize","w":1280,"h":796}, {"type":"move",…},
//!      {"type":"paste","text":"…"}, messages {"action":…} / {"data":…}
//! ```
//!
//! The loop is the backend's: an event → `on_event` (hit test against the last frame) → a new frame.
//! Timers (`wants_tick`) and wakes from other threads run here; the end of stdin ends the program.

use std::cell::RefCell;
use std::io::{BufRead, Write};
use std::sync::mpsc::{self, RecvTimeoutError};

use uison_core::{App, Event, Font, Host, Mods, Size, TextMeasure, TextMetrics};

use crate::{Frame, hit_test};

enum Msg {
    Line(String),
    Wake,
    End,
}

/// The protocol on stdin/stdout. Returns the exit code: 0 — the other side closed the pipe.
pub fn run(app: Box<dyn App>, host: &dyn Host, title: &str) -> i32 {
    serve(app, host, title, std::io::BufReader::new(std::io::stdin()), protocol_stdout())
}

/// The loop over any input and output (tests feed it from memory).
pub fn serve(mut app: Box<dyn App>, host: &dyn Host, title: &str, input: impl BufRead + Send + 'static, out: impl Write + 'static) -> i32 {
    let (tx, rx) = mpsc::channel();
    let wake = tx.clone();
    app.set_waker(std::sync::Arc::new(move || {
        let _ = wake.send(Msg::Wake);
    }));
    // Reading has its own thread: the other side may write while we write (a pipe holds 64 KB).
    std::thread::spawn(move || {
        for line in input.lines().map_while(Result::ok) {
            if tx.send(Msg::Line(line)).is_err() {
                return;
            }
        }
        let _ = tx.send(Msg::End);
    });

    let out = RefCell::new(Out { w: Box::new(out), closed: false });
    let host = ConsoleHost { m: host, out: &out };
    let mut hello = String::from("{\"uison\":0,\"title\":");
    uison_json::write_str(&mut hello, title);
    hello.push('}');
    out.borrow_mut().send(&hello);
    let src = app.frame(&host);
    let mut frame = match Frame::parse(&src, &host) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("uison: console: bad frame: {e}");
            return 1;
        }
    };
    out.borrow_mut().frame(&src);

    while !out.borrow().closed {
        let msg = match app.wants_tick() {
            Some(d) => rx.recv_timeout(d),
            None => rx.recv().map_err(|_| RecvTimeoutError::Disconnected),
        };
        let ev = match msg {
            Ok(Msg::Line(line)) if line.trim().is_empty() => continue,
            Ok(Msg::Line(line)) => match crate::event::parse(&line) {
                Ok(e) => e,
                Err(e) => {
                    eprintln!("uison: console: {e}: {line}");
                    continue;
                }
            },
            Ok(Msg::Wake) => Event::Wake,
            Err(RecvTimeoutError::Timeout) => Event::Tick,
            Ok(Msg::End) | Err(RecvTimeoutError::Disconnected) => break,
        };
        let hit = ev.pos().and_then(|p| hit_test(&frame, p));
        let Some(src) = app.on_event(&ev, hit, &host) else { continue };
        match Frame::parse(&src, &host) {
            Ok(f) => {
                frame = f;
                out.borrow_mut().frame(&src);
            }
            Err(e) => eprintln!("uison: console: bad frame: {e}"),
        }
    }
    0
}

struct Out {
    w: Box<dyn Write>,
    /// The other side is gone (a write failed).
    closed: bool,
}

impl Out {
    fn send(&mut self, msg: &str) {
        let mut line = String::with_capacity(msg.len() + 1);
        line.push_str(msg);
        line.push('\n');
        // One write per message: a reader never sees half of it mixed with another.
        if self.w.write_all(line.as_bytes()).and_then(|_| self.w.flush()).is_err() {
            self.closed = true;
        }
    }

    fn frame(&mut self, src: &str) {
        let body = uison_json::compact(src);
        let mut msg = String::with_capacity(body.len() + 10);
        msg.push_str("{\"frame\":");
        msg.push_str(&body);
        msg.push('}');
        self.send(&msg);
    }
}

/// Text metrics without fonts: a server without a desktop has no Pango. Widths of a typical sans-serif
/// face per character class (a little wide rather than narrow: a label cut early is better than one
/// running over); the player draws with its own fonts.
pub struct Estimate;

impl TextMeasure for Estimate {
    fn measure(&self, text: &str, font: &Font) -> TextMetrics {
        let mono = font.family.as_deref() == Some("monospace");
        let em: f32 = text
            .chars()
            .map(|c| match c {
                _ if mono => 0.6,
                ' ' => 0.28,
                'i' | 'j' | 'l' | '!' | '|' | '.' | ',' | ':' | ';' | '\'' | '`' => 0.26,
                'f' | 't' | 'r' | 'I' | '(' | ')' | '[' | ']' | '-' | '/' => 0.36,
                'm' | 'w' | 'M' | 'W' | '@' | '%' => 0.86,
                'A'..='Z' => 0.66,
                '0'..='9' => 0.57,
                'a'..='z' => 0.55,
                // Hiragana, CJK, Hangul, full-width forms: square.
                '\u{1100}'..='\u{115F}' | '\u{2E80}'..='\u{A4CF}' | '\u{AC00}'..='\u{D7A3}' | '\u{FF00}'..='\u{FF60}' => 1.0,
                _ if c.is_ascii() => 0.55,
                _ => 0.6,
            })
            .sum();
        let width = em * font.size * if font.bold { 1.05 } else { 1.0 };
        TextMetrics { width, ascent: font.size * 0.93, descent: font.size * 0.24, leading: 0.0 }
    }
}

impl Host for Estimate {}

/// Text is measured here (the program's fonts); requests to the system go to the player.
struct ConsoleHost<'a> {
    m: &'a dyn Host,
    out: &'a RefCell<Out>,
}

impl ConsoleHost<'_> {
    fn request(&self, cmd: &str, key: &str, value: &str) {
        let mut msg = String::from("{\"host\":\"");
        msg.push_str(cmd);
        msg.push('"');
        if !key.is_empty() {
            msg.push_str(",\"");
            msg.push_str(key);
            msg.push_str("\":");
            uison_json::write_str(&mut msg, value);
        }
        msg.push('}');
        self.out.borrow_mut().send(&msg);
    }
}

impl TextMeasure for ConsoleHost<'_> {
    fn measure(&self, text: &str, font: &Font) -> TextMetrics {
        self.m.measure(text, font)
    }
}

impl Host for ConsoleHost<'_> {
    fn mods(&self) -> Mods {
        Mods::default()
    }

    fn set_clipboard(&self, text: &str) {
        self.request("copy", "text", text);
    }

    fn request_paste(&self) {
        self.request("paste", "", "");
    }

    fn open_url(&self, url: &str) -> bool {
        self.request("open", "url", url);
        true
    }

    fn image_size(&self, src: &str) -> Option<Size> {
        self.m.image_size(src)
    }
}

/// stdout belongs to the protocol: we write to a copy of descriptor 1, and descriptor 1 itself now
/// points to stderr — a stray `println!` (or a C library's printf) lands in the log, not in the stream.
#[cfg(unix)]
pub fn protocol_stdout() -> Box<dyn Write> {
    use std::os::fd::FromRawFd;
    unsafe extern "C" {
        fn dup(fd: i32) -> i32;
        fn dup2(from: i32, to: i32) -> i32;
    }
    let fd = unsafe { dup(1) };
    if fd < 0 {
        return Box::new(std::io::stdout());
    }
    unsafe { dup2(2, 1) };
    Box::new(unsafe { std::fs::File::from_raw_fd(fd) })
}

/// Windows: the same with two owners of stdout — Rust's `print!` asks `GetStdHandle` on every write,
/// the CRT's printf writes to its descriptor 1. Both now point to stderr; we keep a copy of the handle.
#[cfg(windows)]
pub fn protocol_stdout() -> Box<dyn Write> {
    use std::ffi::c_void;
    use std::os::windows::io::FromRawHandle;
    const STD_OUTPUT_HANDLE: u32 = -11i32 as u32;
    const STD_ERROR_HANDLE: u32 = -12i32 as u32;
    const DUPLICATE_SAME_ACCESS: u32 = 2;
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetStdHandle(id: u32) -> *mut c_void;
        fn SetStdHandle(id: u32, h: *mut c_void) -> i32;
        fn GetCurrentProcess() -> *mut c_void;
        fn DuplicateHandle(sp: *mut c_void, h: *mut c_void, tp: *mut c_void, out: *mut *mut c_void, access: u32, inherit: i32, options: u32) -> i32;
    }
    unsafe extern "C" {
        fn _dup2(from: i32, to: i32) -> i32;
    }
    unsafe {
        let h = GetStdHandle(STD_OUTPUT_HANDLE);
        let mut copy = std::ptr::null_mut();
        if h.is_null() || h as isize == -1 || DuplicateHandle(GetCurrentProcess(), h, GetCurrentProcess(), &mut copy, 0, 0, DUPLICATE_SAME_ACCESS) == 0 {
            return Box::new(std::io::stdout());
        }
        // _dup2 closes the CRT's descriptor 1 and with it the original handle (and in a console
        // program resets the standard handle) — so it goes first.
        _dup2(2, 1);
        SetStdHandle(STD_OUTPUT_HANDLE, GetStdHandle(STD_ERROR_HANDLE));
        Box::new(std::fs::File::from_raw_handle(copy))
    }
}

#[cfg(not(any(unix, windows)))]
pub fn protocol_stdout() -> Box<dyn Write> {
    Box::new(std::io::stdout())
}
