//! uison-player: runs a uison program in console mode on this machine and shows it.
//!
//! `uison-player [<program> [arguments…]]`, e.g. `uison-player target/release/disk-usage --console`; without a
//! program — a start screen to type one (programs found next to the player are suggested). The program writes
//! frames to stdout and reads events from stdin, one JSON message per line (`uison_frame::console`).

use std::cell::RefCell;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use uison_player::uison::ui::{IntoJson, Json, obj};
use uison_player::{Link, Poster, Shared, What};

/// A local process: its stdin is the link; stopping closes it, then kills a program that doesn't exit.
struct Process {
    stdin: Option<ChildStdin>,
    child: Arc<Mutex<Child>>,
}

impl Link for Process {
    fn send(&mut self, line: &str) -> bool {
        let Some(stdin) = &mut self.stdin else { return false };
        let mut buf = String::with_capacity(line.len() + 1);
        buf.push_str(line);
        buf.push('\n');
        stdin.write_all(buf.as_bytes()).and_then(|_| stdin.flush()).is_ok()
    }

    fn stop(&mut self) {
        self.stdin = None;
        let child = self.child.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_secs(1));
            if let Ok(mut c) = child.lock()
                && matches!(c.try_wait(), Ok(None))
            {
                let _ = c.kill();
            }
        });
    }
}

/// A reader thread: lines of a pipe to the player; at the end of stdout — the exit status.
fn reader(pipe: impl std::io::Read + Send + 'static, post: Poster, wrap: fn(String) -> What, child: Option<Arc<Mutex<Child>>>) {
    std::thread::spawn(move || {
        for line in BufReader::new(pipe).lines().map_while(Result::ok) {
            post.post(wrap(line));
        }
        let Some(child) = child else { return };
        // `try_wait` with short locks: `stop` may need the child to kill it.
        let status = loop {
            match child.lock().unwrap().try_wait() {
                Ok(Some(s)) => break s.to_string(),
                Ok(None) => {}
                Err(e) => break e.to_string(),
            }
            std::thread::sleep(Duration::from_millis(20));
        };
        post.post(What::Exit(status));
    });
}

/// Start a program in console mode (the previous one is stopped). Returns its name.
fn launch(st: &Shared, line: &str) -> Result<String, String> {
    let args = split_command(line);
    let Some(program) = args.first() else { return Err("Type a program to run.".into()) };
    let mut child = Command::new(program)
        .args(&args[1..])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("{program}: {e}"))?;
    let post = st.borrow_mut().begin(line);
    let (stdout, stderr) = (child.stdout.take().unwrap(), child.stderr.take().unwrap());
    let stdin = child.stdin.take();
    let child = Arc::new(Mutex::new(child));
    reader(stdout, post.clone(), What::Out, Some(child.clone()));
    reader(stderr, post, What::Err, None);
    st.borrow_mut().attach(Box::new(Process { stdin, child }));
    Ok(program.rsplit(['/', '\\']).next().unwrap_or(program).to_string())
}

/// A command line into words: spaces separate, quotes ("…", '…') group, a leading ~ is the home folder.
fn split_command(line: &str) -> Vec<String> {
    let home = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")).unwrap_or_default();
    let mut words = Vec::new();
    let mut word: Option<String> = None;
    let mut quote = None;
    for c in line.chars() {
        match (quote, c) {
            (Some(q), c) if c == q => quote = None,
            (Some(_), c) => word.get_or_insert_with(String::new).push(c),
            (None, '"' | '\'') => {
                quote = Some(c);
                word.get_or_insert_with(String::new);
            }
            (None, c) if c.is_whitespace() => words.extend(word.take()),
            (None, c) => word.get_or_insert_with(String::new).push(c),
        }
    }
    words.extend(word);
    for w in &mut words {
        if w == "~" || w.starts_with("~/") {
            w.replace_range(..1, &home);
        }
    }
    words
}

/// A word for a command line: quoted when it has spaces.
fn quote(word: &str) -> String {
    if word.contains(char::is_whitespace) { format!("\"{word}\"") } else { word.to_string() }
}

/// Programs next to the player that know the console mode, with sensible arguments.
fn found_programs() -> Vec<String> {
    let Some(dir) = std::env::current_exe().ok().and_then(|p| p.parent().map(|d| d.to_path_buf())) else { return Vec::new() };
    let mut out = Vec::new();
    let du = dir.join(if cfg!(windows) { "disk-usage.exe" } else { "disk-usage" });
    if du.exists() {
        let du = quote(&du.to_string_lossy());
        out.push(format!("{du} --console ~"));
        for folder in ["/System/Library/Frameworks", "/usr/lib", "C:\\Windows\\System32"] {
            if std::path::Path::new(folder).is_dir() {
                out.push(format!("{du} --console {}", quote(folder)));
            }
        }
    }
    out
}

fn rows(list: &[String]) -> Json {
    Json::Array(list.iter().enumerate().map(|(i, c)| obj([("i", (i as f64).into_json()), ("cmd", c.as_str().into_json())])).collect())
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    // Commands for the start screen: this session's first, then the found ones.
    let suggestions = Rc::new(RefCell::new(found_programs()));
    let app = uison_player::build(None, |ui, st| {
        let start = |st: &Shared, ui_data: &mut uison_player::uison::ui::Data, sugg: &RefCell<Vec<String>>, line: &str| {
            match launch(st, line) {
                Ok(name) => {
                    ui_data.set("title", name);
                    ui_data.set("running", true);
                    ui_data.set("status", "");
                    ui_data.set("tab", "app");
                    let mut s = sugg.borrow_mut();
                    let line = line.trim().to_string();
                    s.retain(|c| *c != line);
                    s.insert(0, line);
                }
                Err(e) => ui_data.set("status", e),
            }
            ui_data.set("suggest", rows(&sugg.borrow()));
        };
        // A program on the command line starts right away; otherwise the start screen.
        if !args.is_empty() {
            let line = args.iter().map(|a| quote(a)).collect::<Vec<_>>().join(" ");
            ui.data.set("cmd", line.as_str());
            start(st, &mut ui.data, &suggestions, &line);
        } else if let Some(first) = suggestions.borrow().first() {
            ui.data.set("cmd", first.as_str());
        }
        ui.data.set("suggest", rows(&suggestions.borrow()));
        let (s, sg) = (st.clone(), suggestions.clone());
        ui.on("run", move |cx| {
            let line = cx.data.str("cmd").to_string();
            start(&s, cx.data, &sg, &line);
        });
        let (s, sg) = (st.clone(), suggestions.clone());
        ui.on("suggest", move |cx| {
            let i: usize = cx.key.trim_start_matches("sg-").parse().unwrap_or(0);
            let Some(line) = sg.borrow().get(i).cloned() else { return };
            cx.data.set("cmd", line.as_str());
            start(&s, cx.data, &sg, &line);
        });
    });
    uison_player::uison::run(app, "uison player");
}
