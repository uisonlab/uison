//! The meaning of the screen for programs that can't see it (agents, tests, accessibility):
//! the laid-out tree → nodes with a role, a name, a state and a `key`, as JSON.
//!
//! ```text
//! {"role":"button","key":"compose","name":"Compose","rect":[16,12,96,32]}
//! {"role":"checkbox","key":"done-2","name":"Buy milk","checked":true,"rect":[…]}
//! {"role":"list","key":"inbox","count":2000,"first":0,"rect":[…],"children":[…]}
//! ```
//!
//! Roles come from what the node does (behaviors, fields, links), not from component names:
//! the library expands `Checkbox` into a `Row` with `toggle`. Controls take the text inside them as
//! their name; containers (lists, scroll views, dialogs) keep it as child `text` nodes.

use uison_core::{Cursor, Point, Rect};

use crate::Json;
use crate::data::Data;
use crate::el::*;

pub(crate) struct Sem<'a> {
    data: &'a Data,
    focus: Option<&'a str>,
    overlays: Vec<(&'a El, Point)>,
    /// Visible vertical band (nearest scroll view), as in `emit`.
    view: (f32, f32),
}

/// Nodes of the laid-out roots; `origin` — where the roots lie (as in `emit`).
pub(crate) fn build(roots: &[El], origin: Point, data: &Data, focus: Option<&str>) -> Vec<Json> {
    let mut s = Sem { data, focus, overlays: Vec::new(), view: (f32::NEG_INFINITY, f32::INFINITY) };
    let mut out = Vec::new();
    for el in roots {
        s.node(el, origin, &mut out, &mut None);
    }
    // Overlays (dropdowns, popups) after everything: they are on top.
    let mut i = 0;
    while i < s.overlays.len() {
        let (el, at) = s.overlays[i];
        s.view = (f32::NEG_INFINITY, f32::INFINITY);
        s.node(el, at, &mut out, &mut None);
        i += 1;
    }
    out
}

/// Text of a control gathers into its name; outside controls it becomes a `text` node.
type Name = Option<String>;

impl<'a> Sem<'a> {
    fn children(&mut self, el: &'a El, abs: Point, out: &mut Vec<Json>, name: &mut Name) {
        for mut g in self.groups(el, abs, name) {
            out.append(&mut g);
        }
    }

    /// Nodes of each child separately: labels are found among direct children, not in a flattened list.
    fn groups(&mut self, el: &'a El, abs: Point, name: &mut Name) -> Vec<Vec<Json>> {
        let mut groups = Vec::with_capacity(el.children.len());
        for ch in &el.children {
            if ch.lay.overlay {
                self.overlays.push((ch, abs));
            } else {
                let mut g = Vec::new();
                self.node(ch, abs, &mut g, name);
                groups.push(g);
            }
        }
        groups
    }

    /// Children with labels attached: a text beside a control in a row, above a field in a column.
    fn labelled(&mut self, el: &'a El, abs: Point, name: &mut Name) -> Vec<Json> {
        let mut groups = self.groups(el, abs, name);
        match el.kind {
            Kind::Row { .. } | Kind::Wrap { .. } => label_beside(&mut groups),
            Kind::Column { .. } => label_above(&mut groups),
            _ => {}
        }
        groups.into_iter().flatten().collect()
    }

    fn node(&mut self, el: &'a El, origin: Point, out: &mut Vec<Json>, name: &mut Name) {
        let abs = Point::new(origin.x + el.pos.x + el.lay.margin.l, origin.y + el.pos.y + el.lay.margin.t);
        let rect = Rect::new(abs.x, abs.y, el.size.w, el.size.h);
        let visible = rect.bottom() > self.view.0 && rect.y < self.view.1;
        let Some(role) = el.act.as_deref().and_then(|a| self.role(el, a)) else {
            match &el.kind {
                // A control's name includes its text even past the edge of the scroll view.
                Kind::Text(t) if visible || name.is_some() => text(out, name, t),
                Kind::Image { .. } if visible && name.is_none() => {
                    if let Some(alt) = &el.alt {
                        let mut n = Json::object();
                        n.set("role", "image");
                        n.set("name", alt.clone());
                        out.push(n);
                    }
                }
                Kind::Vector { .. } | Kind::Prim(_) | Kind::Image { .. } => {}
                Kind::Row { .. } | Kind::Wrap { .. } | Kind::Column { .. } if name.is_none() => {
                    let mut inner = self.labelled(el, abs, name);
                    // A row of plain text reads as one line: "Anna · Report · 10:42".
                    match join_texts(&inner).filter(|_| matches!(el.kind, Kind::Row { .. })) {
                        Some(line) => out.push(line),
                        None => out.append(&mut inner),
                    }
                }
                _ => self.children(el, abs, out, name),
            }
            return;
        };
        let act = el.act.as_deref().unwrap();
        let mut n = Json::object();
        n.set("role", role);
        n.set("key", el.key.clone());
        let mut kids = Vec::new();
        let mut own: Name = if is_control(role) { Some(String::new()) } else { None };
        let saved = self.view;
        match &el.kind {
            Kind::Scroll { offset, content, .. } => {
                n.set("offset", round(*offset));
                n.set("content", round(*content));
                self.view = (rect.y.max(saved.0), rect.bottom().min(saved.1));
            }
            Kind::List { count, first, item_h, offset, .. } => {
                n.set("offset", round(*offset));
                n.set("count", *count);
                n.set("first", *first);
                n.set("itemH", round(*item_h));
                self.view = (rect.y.max(saved.0), rect.bottom().min(saved.1));
            }
            Kind::Input(i) => {
                n.set("value", if i.password { "•".repeat(i.value.chars().count()) } else { i.value.clone() });
                if !i.placeholder.is_empty() {
                    n.set("placeholder", i.placeholder.clone());
                }
                if i.password {
                    n.set("password", true);
                }
            }
            Kind::Canvas { draw } => n.set("draw", draw.clone()),
            _ => {}
        }
        if own.is_some() {
            self.children(el, abs, &mut kids, &mut own);
        } else if !matches!(el.kind, Kind::Vector { .. }) {
            kids = self.labelled(el, abs, &mut own);
        }
        self.view = saved;
        // The node's own `alt`, its text, its tooltip, the `alt` of an icon inside (an icon button).
        // An icon names a button; inside a checkbox it shows the state ("check"), not the name.
        let icon = || inner_alt(el).filter(|_| matches!(role, "button" | "link" | "menuitem"));
        let label = el.alt.clone().or_else(|| own.filter(|s| !s.is_empty())).or_else(|| act.tooltip.clone()).or_else(icon);
        if let Some(l) = label {
            n.set("name", l);
        }
        self.state(&mut n, el, act);
        if !visible {
            n.set("offscreen", true);
        }
        n.set("rect", vec![round(rect.x), round(rect.y), round(rect.w), round(rect.h)]);
        // A wrapper that only adds an event (`TextField`: a box with `submit` around the field) — the field itself.
        if role == "group" && n.get("name").is_none() && act.menu.is_none() && kids.len() == 1 && kids[0].get("key").is_some() {
            let mut kid = kids.pop().unwrap();
            if let Some(on) = n.remove("on") {
                let mut all = kid.remove("on").and_then(|v| v.as_array().map(<[Json]>::to_vec)).unwrap_or_default();
                all.extend(on.as_array().unwrap_or(&[]).iter().cloned());
                kid.set("on", all);
            }
            out.push(kid);
            return;
        }
        if !kids.is_empty() {
            n.set("children", kids);
        }
        out.push(n);
    }

    /// What the node is; `None` — only catches the mouse (a scrim, a hover area without a role).
    fn role(&self, el: &El, act: &Act) -> Option<&'static str> {
        Some(match (&el.kind, &act.behavior) {
            (Kind::Input(_), _) => "textfield",
            (Kind::List { .. }, _) => "list",
            (Kind::Scroll { .. }, _) => "scroll",
            (Kind::Canvas { .. }, _) => "canvas",
            (_, Some(Behavior::Toggle(p))) if local(p) => "button",
            (_, Some(Behavior::Toggle(_))) => "checkbox",
            (_, Some(Behavior::Select(_))) => "option",
            (_, Some(Behavior::Drag { .. })) => "slider",
            _ if act.href.is_some() => "link",
            _ if act.dismiss.is_some() => "dialog",
            _ if act.on.iter().any(|(_, a)| a == "~menu") => "menuitem",
            _ if act.on.iter().any(|(e, _)| e == "click") => "button",
            // Other events (`submit` of a field's box, `hover`) — a group around what is inside.
            _ if !act.on.is_empty() || act.focusable => "group",
            // An interactive component without a handler yet (or disabled) still looks and acts like a button.
            _ if act.cursor == Cursor::Pointer || act.disabled || act.menu.is_some() => "button",
            _ => return None,
        })
    }

    fn state(&self, n: &mut Json, el: &El, act: &Act) {
        match &act.behavior {
            Some(Behavior::Toggle(p)) if local(p) => n.set("expanded", self.data.bool(p)),
            Some(Behavior::Toggle(p)) => n.set("checked", self.data.bool(p)),
            Some(Behavior::Select(sets)) => {
                // Writes to local state (closing the dropdown) don't tell whether it's selected.
                let on = sets.iter().filter(|(p, _)| !local(p)).all(|(p, v)| self.data.get(p) == Some(v));
                n.set("selected", on);
            }
            Some(Behavior::Drag { path, min, max, step, delta, .. }) => {
                n.set("value", self.data.num(path));
                n.set("min", *min);
                n.set("max", *max);
                if *step > 0.0 {
                    n.set("step", *step);
                }
                // A divider: the value follows the shift, not the point on the node.
                if *delta != 0.0 {
                    n.set("delta", *delta);
                }
            }
            None => {}
        }
        if act.disabled {
            n.set("disabled", true);
        }
        if self.focus == Some(el.key.as_str()) {
            n.set("focused", true);
        }
        if let Some(h) = &act.href {
            n.set("url", h.clone());
        }
        // Has a context menu; its items are seen when it is open (they often depend on the selection).
        if act.menu.is_some() {
            n.set("menu", true);
        }
        // Events with handlers besides the obvious click: hover, change, submit…
        let extra: Vec<Json> =
            act.on.iter().filter(|(e, a)| e != "click" && a != "~menu").map(|(e, _)| Json::from(e.clone())).collect();
        if !extra.is_empty() {
            n.set("on", extra);
        }
    }
}

/// The first `alt` inside the node, not going into other interactive nodes: `Icon` sets its name there.
fn inner_alt(el: &El) -> Option<String> {
    el.children.iter().filter(|c| c.act.is_none()).find_map(|c| c.alt.clone().or_else(|| inner_alt(c)))
}

/// Controls take the text inside them as their name.
fn is_control(role: &str) -> bool {
    matches!(role, "button" | "checkbox" | "option" | "slider" | "link" | "menuitem" | "textfield")
}

/// Local state of an element (`@~open`): a dropdown, not user data.
fn local(path: &str) -> bool {
    path.contains('~')
}

fn text(out: &mut Vec<Json>, name: &mut Name, t: &TextEl) {
    let s = t.text.trim();
    if s.is_empty() {
        return;
    }
    if let Some(name) = name {
        if !name.is_empty() {
            name.push(' ');
        }
        name.push_str(s);
        return;
    }
    let mut n = Json::object();
    n.set("role", if t.font.bold && t.font.size >= 16.0 { "heading" } else { "text" });
    n.set("name", s.to_string());
    out.push(n);
}

fn role_of(n: &Json) -> &str {
    n.get("role").and_then(Json::as_str).unwrap_or("")
}

/// The only node of a child group, if it has this kind of role.
fn single(g: &mut [Json], ok: impl Fn(&Json) -> bool) -> Option<&mut Json> {
    match g {
        [n] if ok(n) => Some(n),
        _ => None,
    }
}

fn unnamed(n: &Json) -> bool {
    n.get("name").is_none()
}

/// Move the text of group `from` into the name of the control in group `to`.
fn attach(groups: &mut [Vec<Json>], from: usize, to: usize) {
    let label = groups[from].pop().and_then(|mut t| t.remove("name")).unwrap_or(Json::Null);
    let n = &mut groups[to][0];
    // A dropdown shows its value as its text: the value moves aside, the label becomes the name.
    if n.get("expanded").is_some()
        && let Some(v) = n.remove("name")
    {
        n.set("value", v);
    }
    n.set("name", label);
}

/// In a row: a box to tick takes the text after it ("☐ Buy bread"), a field or a slider the text before it.
fn label_beside(groups: &mut [Vec<Json>]) {
    let text = |g: &mut [Json]| single(g, |n| role_of(n) == "text").is_some();
    for i in 0..groups.len() {
        let Some(n) = single(&mut groups[i], |n| is_control(role_of(n)) && unnamed(n)) else { continue };
        let after = matches!(role_of(n), "checkbox" | "option");
        if after && i + 1 < groups.len() && text(&mut groups[i + 1]) {
            attach(groups, i + 1, i);
        } else if !after && i > 0 && text(&mut groups[i - 1]) {
            attach(groups, i - 1, i);
        }
    }
}

/// In a column: a text right above a field, a slider or a dropdown is its label (`TextField`, `Select`).
fn label_above(groups: &mut [Vec<Json>]) {
    let field = |n: &Json| {
        let r = role_of(n);
        (matches!(r, "textfield" | "slider") && unnamed(n)) || (r == "button" && n.get("expanded").is_some() && n.get("value").is_none())
    };
    for i in 1..groups.len() {
        // The field alone, or with its value beside it ("Age: [slider] 30").
        let g = &groups[i];
        let first = g.first().is_some_and(field) && g[1..].iter().all(|n| role_of(n) == "text");
        if first && single(&mut groups[i - 1], |n| role_of(n) == "text").is_some() {
            attach(groups, i - 1, i);
        }
    }
}

/// Only plain texts → one `text` node joined with " · ".
fn join_texts(nodes: &[Json]) -> Option<Json> {
    if nodes.len() < 2 || !nodes.iter().all(|n| n.get("role").and_then(Json::as_str) == Some("text")) {
        return None;
    }
    let parts: Vec<&str> = nodes.iter().filter_map(|n| n.get("name").and_then(Json::as_str)).collect();
    let mut n = Json::object();
    n.set("role", "text");
    n.set("name", parts.join(" · "));
    Some(n)
}

fn round(v: f32) -> f64 {
    (v as f64).round()
}
