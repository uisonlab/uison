//! A test scenario run by the app itself (on iOS a touch can't be simulated from outside).
//! Format as in `web/drive.js`: a JSON array of steps, elements addressed by hit area id.
//!
//! ```text
//! ["tap", id]             touch (or mouse click) at the element's center
//! ["click", id, n]        n mouse clicks in a row (2 — double, 3 — triple)
//! ["rclick", id]          right mouse button
//! ["hold", id]            long press with a finger (600 ms)
//! ["select", id, x0, x1]  mouse drag inside the element from x0 to x1 (pt from the left edge)
//! ["hover", id, ms]       hover (mouse only), pause in ms (60 by default)
//! ["swipe", id, dy, ms]   a finger moves from the element's center by dy over ms and lifts
//!                         (dy < 0 — up: the content scrolls down); with a mouse — the wheel
//! ["wheel", id, dy]       wheel scroll
//! ["type", "text"]        typing character by character
//! ["key", "Enter", "shift+alt"]  key: Enter Backspace Delete Tab Escape Left Right Up Down Home End Menu F10;
//!                         optional modifiers: shift, alt, ctrl, cmd
//! ["cmd", "n"]            key with the primary modifier
//! ["paste", "text"]       paste as if from the clipboard (`Event::Paste`)
//! ["event", {…}]         a raw event in the form of [`crate::event`]
//! ["action", {…}]        an action or data change for `uison-ui`: {"action":"save","key":"ok"}, {"data":{"q":"x"}}
//! ["wait", ms]
//! ["hitmap"]             print the frame's hit areas: `id x y w h`
//! ["capture", name, ms]   screenshot (the backend prints `uison: capture name`), pause in ms (1500 by default)
//! ["done"]
//! ```
//!
//! The backend calls [`Player::next`] on a timer and performs the action; events take the same
//! path as real ones (on iOS, as UIKit touches).

use std::collections::VecDeque;
use std::time::Duration;

use uison_core::{Event, Key, KeyCode, Mods, MouseButton, Point};
use uison_json::Value;

use crate::{Error, Frame, hit_areas};

/// What the backend should do.
#[derive(Clone, Debug, PartialEq)]
pub enum Action {
    Event(Event),
    /// Take a screenshot: print `uison: capture <name>` (an external script takes it).
    Capture(String),
    /// A log message (e.g. no element with that id).
    Log(String),
    /// Just a pause.
    Wait,
    /// The scenario is over.
    Done,
}

enum Step {
    Tap(String),
    Click(String, u32),
    RightClick(String),
    Hold(String),
    Select(String, f32, f32),
    Hover(String, u64),
    Swipe(String, f32, u64),
    Wheel(String, f32),
    Type(String),
    Key(KeyCode, Mods),
    Cmd(String),
    Paste(String),
    Wait(u64),
    Capture(String, u64),
    /// `["event", {…}]`, `["action", {…}]` (a message for the app).
    Raw(Event),
    Hitmap,
    Done,
}

/// Pause after a step: let the frame render.
const GAP: Duration = Duration::from_millis(60);
/// Interval between finger moves (a 60 Hz frame).
const MOVE: Duration = Duration::from_millis(16);

pub struct Player {
    steps: VecDeque<Step>,
    queue: VecDeque<(Action, Duration)>,
    /// Finger touches (iOS) or a mouse.
    touch: bool,
}

fn ms(d: u64) -> Duration {
    Duration::from_millis(d)
}

impl Player {
    pub fn new(src: &str, touch: bool) -> Result<Player, Error> {
        let doc = uison_json::parse(src)?;
        let bad = |what: &str| Error { line: 0, col: 0, msg: format!("script: {what}").into() };
        let mut steps = VecDeque::new();
        for s in doc.as_array().ok_or_else(|| bad("expected an array of steps"))? {
            let a: &[Value] = s.as_array().filter(|a: &&[Value]| !a.is_empty()).ok_or_else(|| bad("step must be a non-empty array"))?;
            let op = a[0].as_str().ok_or_else(|| bad("step name must be a string"))?;
            let str_at = |i: usize| a.get(i).and_then(Value::as_str).map(str::to_string).ok_or_else(|| bad(&format!("'{op}' needs a string argument")));
            let num_at = |i: usize, def: Option<f64>| {
                a.get(i).and_then(Value::as_f64).or(def).ok_or_else(|| bad(&format!("'{op}' needs a number argument")))
            };
            steps.push_back(match op {
                "tap" => Step::Tap(str_at(1)?),
                "click" => Step::Click(str_at(1)?, num_at(2, Some(1.0))? as u32),
                "rclick" => Step::RightClick(str_at(1)?),
                "hold" => Step::Hold(str_at(1)?),
                "select" => Step::Select(str_at(1)?, num_at(2, None)? as f32, num_at(3, None)? as f32),
                "hover" => Step::Hover(str_at(1)?, num_at(2, Some(GAP.as_millis() as f64))? as u64),
                "swipe" => Step::Swipe(str_at(1)?, num_at(2, None)? as f32, num_at(3, Some(200.0))? as u64),
                "wheel" => Step::Wheel(str_at(1)?, num_at(2, None)? as f32),
                "type" => Step::Type(str_at(1)?),
                "key" => Step::Key(
                    key_code(&str_at(1)?).ok_or_else(|| bad("unknown key"))?,
                    mods(a.get(2).and_then(Value::as_str).unwrap_or("")).ok_or_else(|| bad("unknown modifier"))?,
                ),
                "cmd" => Step::Cmd(str_at(1)?),
                "paste" => Step::Paste(str_at(1)?),
                "wait" => Step::Wait(num_at(1, None)? as u64),
                "capture" => Step::Capture(str_at(1)?, num_at(2, Some(1500.0))? as u64),
                "event" | "action" => Step::Raw(a.get(1).map_or(Err("needs an object"), crate::event::from_value).map_err(bad)?),
                "hitmap" => Step::Hitmap,
                "done" => Step::Done,
                _ => return Err(bad(&format!("unknown step '{op}'"))),
            });
        }
        Ok(Player { steps, queue: VecDeque::new(), touch })
    }

    /// The next action and the pause after it; `None` — no more steps.
    /// Elements are looked up in the current frame when the step starts.
    pub fn next(&mut self, frame: &Frame) -> Option<(Action, Duration)> {
        while self.queue.is_empty() {
            let step = self.steps.pop_front()?;
            self.expand(step, frame);
        }
        self.queue.pop_front()
    }

    fn push(&mut self, a: Action, d: Duration) {
        self.queue.push_back((a, d));
    }

    fn ev(&mut self, e: Event, d: Duration) {
        self.push(Action::Event(e), d);
    }

    fn expand(&mut self, step: Step, frame: &Frame) {
        // The last area with this id is the topmost (popups are drawn later).
        let area = |id: &str| hit_areas(frame).into_iter().rev().find(|(i, _)| *i == id).map(|(_, r)| r);
        let center = |id: &str| area(id).map(|r| Point::new(r.x + r.w / 2.0, r.y + r.h / 2.0));
        let missing = |id: &str| Action::Log(format!("script: no hit area '{id}'"));
        match step {
            Step::Tap(id) => match center(&id) {
                Some(p) if self.touch => {
                    self.ev(Event::MouseDown(p, MouseButton::Touch), ms(70));
                    self.ev(Event::MouseUp(p, MouseButton::Touch), GAP);
                }
                Some(p) => {
                    self.ev(Event::MouseMove(p), ms(20));
                    self.ev(Event::MouseDown(p, MouseButton::Left), ms(40));
                    self.ev(Event::MouseUp(p, MouseButton::Left), GAP);
                }
                None => self.push(missing(&id), GAP),
            },
            Step::Click(id, n) => match center(&id) {
                Some(p) => {
                    self.ev(Event::MouseMove(p), ms(20));
                    for _ in 0..n {
                        self.ev(Event::MouseDown(p, MouseButton::Left), ms(30));
                        self.ev(Event::MouseUp(p, MouseButton::Left), ms(60));
                    }
                    self.push(Action::Wait, GAP);
                }
                None => self.push(missing(&id), GAP),
            },
            Step::RightClick(id) => match center(&id) {
                Some(p) => {
                    self.ev(Event::MouseMove(p), ms(20));
                    self.ev(Event::MouseDown(p, MouseButton::Right), ms(40));
                    self.ev(Event::MouseUp(p, MouseButton::Right), GAP);
                }
                None => self.push(missing(&id), GAP),
            },
            Step::Hold(id) => match center(&id) {
                Some(p) => {
                    self.ev(Event::MouseDown(p, MouseButton::Touch), ms(600));
                    self.ev(Event::MouseUp(p, MouseButton::Touch), GAP);
                }
                None => self.push(missing(&id), GAP),
            },
            Step::Select(id, x0, x1) => match area(&id) {
                Some(r) => {
                    let y = r.y + r.h / 2.0;
                    let n = 6;
                    self.ev(Event::MouseMove(Point::new(r.x + x0, y)), ms(20));
                    self.ev(Event::MouseDown(Point::new(r.x + x0, y), MouseButton::Left), MOVE);
                    for k in 1..=n {
                        self.ev(Event::MouseMove(Point::new(r.x + x0 + (x1 - x0) * k as f32 / n as f32, y)), MOVE);
                    }
                    self.ev(Event::MouseUp(Point::new(r.x + x1, y), MouseButton::Left), GAP);
                }
                None => self.push(missing(&id), GAP),
            },
            Step::Raw(e) => self.ev(e, GAP),
            Step::Hover(id, d) => match center(&id) {
                Some(p) => self.ev(Event::MouseMove(p), ms(d)),
                None => self.push(missing(&id), GAP),
            },
            Step::Swipe(id, dy, dur) => match center(&id) {
                Some(p) if self.touch => {
                    let n = (dur / MOVE.as_millis() as u64).max(1);
                    self.ev(Event::MouseDown(p, MouseButton::Touch), MOVE);
                    for k in 1..=n {
                        let y = p.y + dy * k as f32 / n as f32;
                        self.ev(Event::MouseMove(Point::new(p.x, y)), MOVE);
                    }
                    self.ev(Event::MouseUp(Point::new(p.x, p.y + dy), MouseButton::Touch), GAP);
                }
                // With a mouse: finger up = wheel "content up".
                Some(p) => self.ev(Event::Scroll(p, 0.0, dy), GAP),
                None => self.push(missing(&id), GAP),
            },
            Step::Wheel(id, dy) => match center(&id) {
                Some(p) => self.ev(Event::Scroll(p, 0.0, dy), GAP),
                None => self.push(missing(&id), GAP),
            },
            Step::Type(text) => {
                for ch in text.chars() {
                    let mut buf = [0u8; 4];
                    let k = match ch {
                        '\n' => Key::new(KeyCode::Enter),
                        _ => Key::char(ch.encode_utf8(&mut buf)),
                    };
                    self.ev(Event::Key(k), ms(25));
                }
                self.push(Action::Wait, GAP);
            }
            Step::Key(code, mods) => self.ev(Event::Key(Key { code, text: None, mods }), GAP),
            Step::Cmd(ch) => {
                let mut k = Key::char(&ch);
                k.mods.cmd = true;
                self.ev(Event::Key(k), GAP);
            }
            Step::Paste(t) => self.ev(Event::Paste(t.into()), GAP),
            Step::Wait(d) => self.push(Action::Wait, ms(d)),
            Step::Capture(name, d) => self.push(Action::Capture(name), ms(d)),
            Step::Hitmap => {
                let i = |v: f32| v.round() as i64;
                let list: Vec<String> =
                    hit_areas(frame).iter().map(|(id, r)| format!("{id}\t{}\t{}\t{}\t{}", i(r.x), i(r.y), i(r.w), i(r.h))).collect();
                self.push(Action::Log(format!("hitmap:\n{}", list.join("\n"))), GAP);
            }
            Step::Done => self.push(Action::Done, Duration::ZERO),
        }
    }
}

/// `"shift+alt"` → modifiers; an empty string means none.
fn mods(spec: &str) -> Option<Mods> {
    let mut m = Mods::default();
    for part in spec.split('+').filter(|p| !p.is_empty()) {
        match part {
            "shift" => m.shift = true,
            "alt" => m.alt = true,
            "ctrl" => m.ctrl = true,
            "cmd" => m.cmd = true,
            _ => return None,
        }
    }
    Some(m)
}

fn key_code(name: &str) -> Option<KeyCode> {
    Some(match name {
        "Enter" => KeyCode::Enter,
        "Backspace" => KeyCode::Backspace,
        "Delete" => KeyCode::Delete,
        "Tab" => KeyCode::Tab,
        "Escape" => KeyCode::Escape,
        "Left" | "ArrowLeft" => KeyCode::Left,
        "Right" | "ArrowRight" => KeyCode::Right,
        "Up" | "ArrowUp" => KeyCode::Up,
        "Down" | "ArrowDown" => KeyCode::Down,
        "Home" => KeyCode::Home,
        "End" => KeyCode::End,
        "PageUp" => KeyCode::PageUp,
        "PageDown" => KeyCode::PageDown,
        "Menu" | "ContextMenu" => KeyCode::Menu,
        "F10" => KeyCode::F10,
        "Back" => KeyCode::Back,
        _ => return None,
    })
}
