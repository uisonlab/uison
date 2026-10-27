//! Canonical JSON form of [`Event`] — the same one the C API hands to `uison_app.on_event`:
//!
//! ```text
//! {"type":"down","x":120,"y":48.5,"button":"left"}     up, move (x, y), leave, scroll (x, y, dx, dy)
//! {"type":"key","code":"char","text":"ж","shift":false,"ctrl":false,"alt":false,"cmd":true}
//! {"type":"resize","w":1280,"h":800}  safe-area (top, right, bottom, left)  appearance (dark)
//! {"type":"paste","text":"…"}  tick  image  wake
//! ```
//!
//! Numbers are written to 1/1000 of a point (as in [`crate::Writer`]), without `f32` formatting.

use uison_core::{Event, Insets, Key, KeyCode, Mods, MouseButton, Point, Size};
use uison_json::Value;

use crate::Error;

const KEYS: [(KeyCode, &str); 17] = [
    (KeyCode::Char, "char"),
    (KeyCode::Backspace, "backspace"),
    (KeyCode::Delete, "delete"),
    (KeyCode::Enter, "enter"),
    (KeyCode::Tab, "tab"),
    (KeyCode::Escape, "escape"),
    (KeyCode::Left, "left"),
    (KeyCode::Right, "right"),
    (KeyCode::Up, "up"),
    (KeyCode::Down, "down"),
    (KeyCode::Home, "home"),
    (KeyCode::End, "end"),
    (KeyCode::PageUp, "pageup"),
    (KeyCode::PageDown, "pagedown"),
    (KeyCode::Back, "back"),
    (KeyCode::Menu, "menu"),
    (KeyCode::F10, "f10"),
];

const BUTTONS: [(MouseButton, &str); 4] =
    [(MouseButton::Left, "left"), (MouseButton::Right, "right"), (MouseButton::Other, "other"), (MouseButton::Touch, "touch")];

pub fn key_name(code: KeyCode) -> &'static str {
    KEYS.iter().find(|k| k.0 == code).map_or("char", |k| k.1)
}

pub fn key_code(name: &str) -> Option<KeyCode> {
    KEYS.iter().find(|k| k.1 == name).map(|k| k.0)
}

pub fn to_json(e: &Event) -> String {
    let mut out = String::with_capacity(64);
    write(e, &mut out);
    out
}

/// Append the event as one JSON object (no trailing newline).
pub fn write(e: &Event, out: &mut String) {
    // A message is a JSON object already (`{"action":…}`, `{"data":…}`).
    if let Event::Message(m) = e {
        out.push_str(m);
        return;
    }
    out.push_str("{\"type\":\"");
    out.push_str(e.name());
    out.push('"');
    let num = |out: &mut String, key: &str, v: f32| {
        out.push_str(",\"");
        out.push_str(key);
        out.push_str("\":");
        crate::writer::num(out, v);
    };
    match e {
        Event::MouseDown(p, b) | Event::MouseUp(p, b) => {
            num(out, "x", p.x);
            num(out, "y", p.y);
            out.push_str(",\"button\":\"");
            out.push_str(BUTTONS.iter().find(|x| x.0 == *b).map_or("left", |x| x.1));
            out.push('"');
        }
        Event::MouseMove(p) => {
            num(out, "x", p.x);
            num(out, "y", p.y);
        }
        Event::Scroll(p, dx, dy) => {
            num(out, "x", p.x);
            num(out, "y", p.y);
            num(out, "dx", *dx);
            num(out, "dy", *dy);
        }
        Event::Resize(s) => {
            num(out, "w", s.w);
            num(out, "h", s.h);
        }
        Event::SafeArea(i) => {
            num(out, "top", i.top);
            num(out, "right", i.right);
            num(out, "bottom", i.bottom);
            num(out, "left", i.left);
        }
        Event::Appearance(dark) => out.push_str(if *dark { ",\"dark\":true" } else { ",\"dark\":false" }),
        Event::Key(k) => {
            out.push_str(",\"code\":\"");
            out.push_str(key_name(k.code));
            out.push_str("\",\"text\":");
            match &k.text {
                Some(t) => uison_json::write_str(out, t),
                None => out.push_str("null"),
            }
            for (name, on) in [("shift", k.mods.shift), ("ctrl", k.mods.ctrl), ("alt", k.mods.alt), ("cmd", k.mods.cmd)] {
                out.push_str(",\"");
                out.push_str(name);
                out.push_str(if on { "\":true" } else { "\":false" });
            }
        }
        Event::Paste(t) => {
            out.push_str(",\"text\":");
            uison_json::write_str(out, t);
        }
        Event::MouseLeave | Event::Tick | Event::ImageLoaded | Event::Wake | Event::Message(_) => {}
    }
    out.push('}');
}

/// Parse one event. Unknown keys are skipped (a message may carry more, e.g. a time).
pub fn parse(src: &str) -> Result<Event, Error> {
    let v = uison_json::parse(src)?;
    from_value(&v).map_err(|msg| Error { line: 1, col: 1, msg: msg.into() })
}

/// An event from an already parsed object (a scenario step, a part of a larger message).
pub fn from_value(v: &Value) -> Result<Event, &'static str> {
    let f = |k: &str| v.get(k).and_then(Value::as_f64).unwrap_or(0.0) as f32;
    let b = |k: &str| v.get(k).and_then(Value::as_bool).unwrap_or(false);
    let at = || Point::new(f("x"), f("y"));
    let button = || {
        let s = v.get("button").and_then(Value::as_str).unwrap_or("left");
        BUTTONS.iter().find(|b| b.1 == s).map(|b| b.0).ok_or("unknown button")
    };
    let text = || v.get("text").and_then(Value::as_str).map(Box::<str>::from);
    let Some(ty) = v.get("type").and_then(Value::as_str) else {
        // A message for the app (an action, a data change).
        return if v.get("action").is_some() || v.get("data").is_some() { Ok(Event::Message(v.to_json().into())) } else { Err("event without type") };
    };
    Ok(match ty {
        "down" => Event::MouseDown(at(), button()?),
        "up" => Event::MouseUp(at(), button()?),
        "move" => Event::MouseMove(at()),
        "leave" => Event::MouseLeave,
        "scroll" => Event::Scroll(at(), f("dx"), f("dy")),
        "resize" => Event::Resize(Size::new(f("w"), f("h"))),
        "safe-area" => Event::SafeArea(Insets { top: f("top"), right: f("right"), bottom: f("bottom"), left: f("left") }),
        "appearance" => Event::Appearance(b("dark")),
        "key" => {
            let code = key_code(v.get("code").and_then(Value::as_str).unwrap_or("char")).ok_or("unknown key code")?;
            Event::Key(Key { code, text: text(), mods: Mods { shift: b("shift"), ctrl: b("ctrl"), alt: b("alt"), cmd: b("cmd") } })
        }
        "paste" => Event::Paste(text().unwrap_or_default()),
        "tick" => Event::Tick,
        "image" => Event::ImageLoaded,
        "wake" => Event::Wake,
        _ => return Err("unknown event type"),
    })
}
