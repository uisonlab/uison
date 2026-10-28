//! Actions by meaning, done as a person would: the pointer goes to the node, presses and releases,
//! keys go to the focused field. What a person can't do (a disabled button, a node under a modal)
//! fails with a reason instead of happening.
//!
//! ```text
//! {"do":"click","key":"add"}                 {"do":"click","name":"Compose","role":"button"}
//! {"do":"type","key":"draft/1/0","text":"Milk","submit":true}     replace (default) or "append":true
//! {"do":"key","code":"cmd+z"}                {"do":"hover","key":"star"}
//! {"do":"scroll","key":"inboxList","to":120} "to": an item number or a key, "by": points down, "edge":"top"|"bottom"
//! {"do":"set","key":"volume","value":50}     a slider, a checkbox (true/false), a field (text)
//! {"do":"pick","key":"country","option":"Germany"}   a dropdown, tabs, radio buttons
//! {"do":"menu","key":"row-17","item":"Delete"}       context menu (without "item" — just open it)
//! {"do":"wait","ms":2000,"until":{"name":"Done"}}    real time for other threads; "until": a node or {"path","value"}
//! {"do":"resize","w":390,"h":844}
//! ```

use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use uison_core::{Event, Key, KeyCode, MouseButton, Point, Rect, Size};
use uison_frame::{hit_areas, hit_test};
use uison_ui::Json;

use crate::tree::{by_name, find, flatten};
use crate::{Agent, SETTLE, advance};

/// More than the double-click interval: separate clicks of the agent stay separate.
const APART: Duration = Duration::from_millis(600);

fn str_of<'a>(n: &'a Json, k: &str) -> Option<&'a str> {
    n.get(k).and_then(Json::as_str)
}

fn rect_of(n: &Json) -> Option<Rect> {
    let r = n.get("rect")?.as_array()?;
    let f = |i: usize| r.get(i).and_then(Json::as_f64).unwrap_or(0.0) as f32;
    Some(Rect::new(f(0), f(1), f(2), f(3)))
}

fn center(r: Rect) -> Point {
    Point::new(r.x + r.w / 2.0, r.y + r.h / 2.0)
}

impl Agent<'_> {
    pub(crate) fn act(&mut self, msg: &Json) -> Result<(), String> {
        let what = str_of(msg, "do").unwrap_or("");
        let num = |k: &str| msg.get(k).and_then(Json::as_f64);
        match what {
            "click" => {
                let key = self.target(msg)?;
                let button = if str_of(msg, "button") == Some("right") { MouseButton::Right } else { MouseButton::Left };
                self.click(&key, button, num("clicks").unwrap_or(1.0) as u32)?;
            }
            "type" => {
                // Without key and name — into the focused field.
                let key = if msg.get("key").is_some() || msg.get("name").is_some() { Some(self.target(msg)?) } else { None };
                let text = str_of(msg, "text").ok_or("type needs \"text\"")?;
                let append = msg.get("append").and_then(Json::as_bool).unwrap_or(false);
                self.type_text(key.as_deref(), text, !append)?;
                if msg.get("submit").and_then(Json::as_bool).unwrap_or(false) {
                    self.send(Event::Key(Key::new(KeyCode::Enter)));
                }
            }
            "key" => {
                let code = str_of(msg, "code").ok_or("key needs \"code\", e.g. \"enter\", \"cmd+z\"")?;
                self.send(Event::Key(parse_key(code)?));
            }
            "hover" => {
                let key = self.target(msg)?;
                let p = self.point(&key)?;
                self.send(Event::MouseMove(p));
            }
            "scroll" => {
                let key = self.target(msg)?;
                self.scroll(&key, msg)?;
            }
            "set" => {
                let key = self.target(msg)?;
                self.set(&key, msg.get("value").ok_or("set needs \"value\"")?)?;
            }
            "pick" => {
                // Without "option" the name is the option's: {"name":"Team"}.
                let (option, key) = match str_of(msg, "option") {
                    Some(o) => (o, if msg.get("key").is_some() || msg.get("name").is_some() { Some(self.target(msg)?) } else { None }),
                    None => (str_of(msg, "name").ok_or("pick needs \"option\" (its name)")?, str_of(msg, "key").map(|k| k.trim_start_matches('#').to_string())),
                };
                self.pick(key.as_deref(), option)?;
            }
            "menu" => {
                let key = self.target(msg)?;
                self.click(&key, MouseButton::Right, 1)?;
                if let Some(item) = str_of(msg, "item") {
                    self.settle(SETTLE);
                    let tree = self.tree();
                    let k = by_name(&tree, item, Some("menuitem"))?;
                    self.click(&k, MouseButton::Left, 1)?;
                }
            }
            "wait" => self.wait(Duration::from_millis(num("ms").unwrap_or(1000.0) as u64), msg.get("until")),
            "resize" => {
                let (w, h) = (num("w").ok_or("resize needs \"w\"")?, num("h").ok_or("resize needs \"h\"")?);
                self.send(Event::Resize(Size::new(w as f32, h as f32)));
            }
            _ => return Err(format!("unknown action '{what}' (click, type, key, hover, scroll, set, pick, menu, wait, resize)")),
        }
        self.settle(SETTLE);
        Ok(())
    }

    /// The node to act on: `key`, or `name` (and `role`) looked up in the tree.
    fn target(&mut self, msg: &Json) -> Result<String, String> {
        // Keys are written "#key" in the text snapshot; agents pass them either way.
        if let Some(k) = str_of(msg, "key") {
            return Ok(k.strip_prefix('#').unwrap_or(k).to_string());
        }
        let name = str_of(msg, "name").ok_or("needs \"key\" or \"name\"")?;
        let tree = self.tree();
        by_name(&tree, name, str_of(msg, "role"))
    }

    /// A point where a press lands on `key`: the node must be enabled, on screen and not covered.
    fn point(&mut self, key: &str) -> Result<Point, String> {
        let mut tree = self.tree();
        let Some((node, _)) = find(&tree, key) else {
            // Hit areas drawn by a `Canvas` (chart sectors) are not nodes of the tree.
            return hit_areas(&self.frame).into_iter().rev().find(|(id, _)| *id == key).map(|(_, r)| center(r)).ok_or_else(|| missing(&tree, key));
        };
        if node.get("disabled").and_then(Json::as_bool) == Some(true) {
            return Err(format!("'{key}' is disabled"));
        }
        if node.get("offscreen").and_then(Json::as_bool) == Some(true) {
            self.reveal(key)?;
            tree = self.tree();
        }
        let (node, _) = find(&tree, key).ok_or_else(|| format!("'{key}' is gone after scrolling"))?;
        let r = rect_of(node).ok_or("node without a rectangle")?;
        // The center first; a row with a button in the middle is pressed beside it.
        for fy in [0.5, 0.25, 0.75] {
            for fx in [0.5, 0.15, 0.85, 0.3, 0.7, 0.05, 0.95] {
                let p = Point::new(r.x + r.w * fx, r.y + r.h * fy);
                if hit_test(&self.frame, p) == Some(key) {
                    return Ok(p);
                }
            }
        }
        match hit_test(&self.frame, center(r)) {
            Some(other) => Err(format!("'{key}' is covered by '{other}'")),
            None => Err(format!("'{key}' can't be pressed here (no hit area)")),
        }
    }

    fn click(&mut self, key: &str, button: MouseButton, clicks: u32) -> Result<(), String> {
        let p = self.point(key)?;
        advance(APART);
        for _ in 0..clicks.max(1) {
            self.send(Event::MouseDown(p, button));
            self.send(Event::MouseUp(p, button));
        }
        // The agent has no pointer resting anywhere: no hover highlight, no tooltip.
        if button == MouseButton::Left {
            self.send(Event::MouseLeave);
        }
        Ok(())
    }

    fn type_text(&mut self, key: Option<&str>, text: &str, replace: bool) -> Result<(), String> {
        if let Some(key) = key {
            let tree = self.tree();
            let (node, _) = find(&tree, key).ok_or_else(|| missing(&tree, key))?;
            if str_of(node, "role") != Some("textfield") {
                return Err(format!("'{key}' is not a textfield"));
            }
            if node.get("focused").and_then(Json::as_bool) != Some(true) {
                self.click(key, MouseButton::Left, 1)?;
            }
        }
        if replace {
            let mut all = Key::char("a");
            all.mods.cmd = true;
            self.send(Event::Key(all));
            if text.is_empty() {
                self.send(Event::Key(Key::new(KeyCode::Backspace)));
            }
        }
        for ch in text.chars() {
            let mut buf = [0u8; 4];
            let k = if ch == '\n' { Key::new(KeyCode::Enter) } else { Key::char(ch.encode_utf8(&mut buf)) };
            self.send(Event::Key(k));
        }
        Ok(())
    }

    /// Scroll the nearest scroll view (or list) until `key` is inside it.
    fn reveal(&mut self, key: &str) -> Result<(), String> {
        const MARGIN: f32 = 8.0;
        for _ in 0..4 {
            let tree = self.tree();
            let (node, path) = find(&tree, key).ok_or_else(|| missing(&tree, key))?;
            if node.get("offscreen").is_none() {
                return Ok(());
            }
            let r = rect_of(node).unwrap_or_default();
            let view = path.iter().rev().find(|n| matches!(str_of(n, "role"), Some("scroll" | "list"))).ok_or_else(|| format!("'{key}' is off screen"))?;
            let v = rect_of(view).unwrap_or_default();
            let dy = if r.y < v.y { v.y - r.y + MARGIN } else { -(r.bottom() - v.bottom() + MARGIN) };
            self.wheel(v, dy);
        }
        Ok(())
    }

    /// A wheel turn over the view: `dy` < 0 moves the content up (scrolls down).
    fn wheel(&mut self, view: Rect, dy: f32) {
        self.send(Event::Scroll(center(view), 0.0, dy));
    }

    fn scroll(&mut self, key: &str, msg: &Json) -> Result<(), String> {
        let tree = self.tree();
        let (node, _) = find(&tree, key).ok_or_else(|| missing(&tree, key))?;
        let role = str_of(node, "role").unwrap_or("");
        if role != "scroll" && role != "list" {
            // Any other node: scroll it into view.
            return self.reveal(key);
        }
        let v = rect_of(node).unwrap_or_default();
        let offset = node.get("offset").and_then(Json::as_f64).unwrap_or(0.0) as f32;
        let dy = match (msg.get("to"), msg.get("by"), str_of(msg, "edge")) {
            (Some(Json::Number(i)), _, _) => {
                let item_h = node.get("itemH").and_then(Json::as_f64).ok_or("\"to\": a number works for lists; for a scroll view give a key")? as f32;
                -(*i as f32 * item_h - offset)
            }
            (Some(Json::String(k)), _, _) => return self.reveal(k),
            (_, Some(b), _) => -(b.as_f64().unwrap_or(0.0) as f32),
            (_, _, Some("top")) => f32::MAX / 4.0,
            (_, _, Some("bottom")) => -f32::MAX / 4.0,
            _ => return Err("scroll needs \"to\" (item number or key), \"by\" (points down) or \"edge\"".into()),
        };
        self.wheel(v, dy);
        Ok(())
    }

    /// A value for a control: a slider is dragged to it, a checkbox is clicked if it differs,
    /// a field gets the text typed.
    fn set(&mut self, key: &str, value: &Json) -> Result<(), String> {
        let tree = self.tree();
        let (node, _) = find(&tree, key).ok_or_else(|| missing(&tree, key))?;
        match str_of(node, "role") {
            Some("checkbox") => {
                let want = value.as_bool().ok_or("a checkbox takes true or false")?;
                if node.get("checked").and_then(Json::as_bool) != Some(want) {
                    self.click(key, MouseButton::Left, 1)?;
                }
                Ok(())
            }
            Some("textfield") => {
                let text = value.as_str().map(str::to_string).unwrap_or_else(|| value.to_json());
                self.type_text(Some(key), &text, true)
            }
            Some("slider") => {
                let want = value.as_f64().ok_or("a slider takes a number")?;
                let n = |k: &str| node.get(k).and_then(Json::as_f64).unwrap_or(0.0);
                let (min, max) = (n("min"), n("max"));
                if max <= min {
                    return Err("slider without a range".into());
                }
                let r = rect_of(node).unwrap_or_default();
                match n("delta") {
                    0.0 => self.drag_to(key, r, ((want - min) / (max - min)).clamp(0.0, 1.0) as f32, min, max, want),
                    delta => self.drag_by(r, ((want - n("value")) / delta) as f32),
                }
            }
            Some(other) => Err(format!("'{key}' is a {other}: set works for sliders, checkboxes and textfields")),
            None => Err(format!("no node '{key}'")),
        }
    }

    /// Press on the slider's track where the value should be; correct by what it reads after (the
    /// thumb inset is not known from outside).
    fn drag_to(&mut self, key: &str, r: Rect, f: f32, min: f64, max: f64, want: f64) -> Result<(), String> {
        let mut x = r.x + r.w * f;
        let y = r.y + r.h / 2.0;
        for _ in 0..4 {
            advance(APART);
            let p = Point::new(x.clamp(r.x, r.right() - 0.5), y);
            self.send(Event::MouseMove(p));
            self.send(Event::MouseDown(p, MouseButton::Left));
            self.send(Event::MouseUp(p, MouseButton::Left));
            let tree = self.tree();
            let got = find(&tree, key).and_then(|(n, _)| n.get("value")?.as_f64()).unwrap_or(want);
            let step = find(&tree, key).and_then(|(n, _)| n.get("step")?.as_f64()).unwrap_or(0.0);
            if (got - want).abs() <= (step / 2.0).max((max - min) * 0.002) {
                break;
            }
            x += ((want - got) / (max - min)) as f32 * r.w;
        }
        self.send(Event::MouseLeave);
        Ok(())
    }

    /// A divider (`drag` with `delta`): press on it and move by `dx`.
    fn drag_by(&mut self, r: Rect, dx: f32) -> Result<(), String> {
        let p = Point::new(r.x + r.w / 2.0, r.y + r.h / 2.0);
        let to = Point::new(p.x + dx, p.y);
        advance(APART);
        self.send(Event::MouseMove(p));
        self.send(Event::MouseDown(p, MouseButton::Left));
        self.send(Event::MouseMove(to));
        self.send(Event::MouseUp(to, MouseButton::Left));
        self.send(Event::MouseLeave);
        Ok(())
    }

    /// Choose an option by name: open the dropdown `key` if it is one, then press the option.
    fn pick(&mut self, key: Option<&str>, option: &str) -> Result<(), String> {
        if let Some(key) = key {
            let tree = self.tree();
            if let Some((node, _)) = find(&tree, key)
                && node.get("expanded").and_then(Json::as_bool) == Some(false)
            {
                self.click(key, MouseButton::Left, 1)?;
                self.settle(SETTLE);
            }
        }
        let tree = self.tree();
        // Options under the node (tabs, radio buttons), under its key as a prefix (a group that is not
        // a node itself: `tabs/all`, `tabs/done`), or anywhere (an open dropdown is an overlay).
        let scope = key.and_then(|k| find(&tree, k)).map(|(n, _)| Json::Array(vec![n.clone()])).or_else(|| {
            let k = format!("{}/", key?);
            let under: Vec<Json> = flatten(&tree).into_iter().filter(|n| str_of(n, "key").is_some_and(|x| x.starts_with(&k))).cloned().collect();
            (!under.is_empty()).then_some(Json::Array(under))
        });
        let found = scope.as_ref().map(|s| by_name(s, option, Some("option"))).filter(Result::is_ok);
        let k = match found {
            Some(k) => k?,
            None => by_name(&tree, option, Some("option"))?,
        };
        self.click(&k, MouseButton::Left, 1)
    }

    /// Wait in real time for work in other threads (up to `max`) — or until a node or a value appears.
    fn wait(&mut self, max: Duration, until: Option<&Json>) {
        let t0 = Instant::now();
        loop {
            if self.woke.swap(false, Ordering::SeqCst) {
                self.send(Event::Wake);
            }
            self.settle(Duration::from_millis(20));
            if let Some(u) = until
                && self.holds(u)
            {
                return;
            }
            if t0.elapsed() >= max {
                return;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    /// `{"name":…,"role":…}` — such a node is on screen; `{"path":…,"value":…}` — the data has it
    /// (without `value` — the path is set and not empty).
    fn holds(&mut self, u: &Json) -> bool {
        if let Some(path) = str_of(u, "path") {
            let v = self.ui.data.get(path);
            return match u.get("value") {
                Some(want) => v == Some(want),
                None => v.is_some_and(Json::truthy),
            };
        }
        let tree = self.tree();
        let name = str_of(u, "name").map(str::to_lowercase);
        let role = str_of(u, "role");
        flatten(&tree).into_iter().any(|n| {
            role.is_none_or(|r| str_of(n, "role") == Some(r))
                && name.as_ref().is_none_or(|w| str_of(n, "name").is_some_and(|x| x.to_lowercase().contains(w.as_str())))
        })
    }
}

/// "No such node" with a hint: nodes whose key starts with it (a group that isn't a node itself).
fn missing(tree: &Json, key: &str) -> String {
    let prefix = format!("{key}/");
    let under: Vec<&str> = flatten(tree).into_iter().filter_map(|n| str_of(n, "key")).filter(|k| k.starts_with(&prefix)).take(6).collect();
    if under.is_empty() { format!("no node '{key}' (take a snapshot: keys change when the screen does)") } else { format!("no node '{key}'; under it: {}", under.join(", ")) }
}

/// `enter`, `cmd+z`, `shift+tab`, `a`, `ctrl+alt+delete` → a key.
fn parse_key(s: &str) -> Result<Key, String> {
    let mut parts: Vec<&str> = s.split('+').collect();
    let last = parts.pop().filter(|p| !p.is_empty()).ok_or_else(|| format!("bad key '{s}'"))?;
    let mut key = match uison_frame::event::key_code(&last.to_lowercase()) {
        Some(code) if last.chars().count() > 1 => Key::new(code),
        _ if last.chars().count() == 1 => Key::char(last),
        _ => return Err(format!("unknown key '{last}'")),
    };
    for m in parts {
        match m.to_lowercase().as_str() {
            "cmd" | "ctrl" | "control" => key.mods.cmd = true,
            "shift" => key.mods.shift = true,
            "alt" | "option" => key.mods.alt = true,
            other => return Err(format!("unknown modifier '{other}'")),
        }
    }
    Ok(key)
}
