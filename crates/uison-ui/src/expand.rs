//! Expanding complex JSON: templates → built-in nodes, values → computed ones.

use std::rc::Rc;

use uison_core::{Color, Cursor, Font, Point};

use crate::el::*;
use crate::library::Component;
use crate::state::Flags;
use crate::values::*;
use crate::{Error, Json};

/// Properties that can be set on any node and any component.
pub(crate) const COMMON: [&str; 19] = [
    "key", "visible", "disabled", "w", "h", "minW", "maxW", "minH", "maxH", "flex", "margin", "place", "pos",
    "overlay", "on", "cursor", "tooltip", "contextMenu", "alt",
];

/// Additionally for built-in nodes: behaviors and interactivity.
pub(crate) const BEHAVIOR: [&str; 7] = ["toggle", "select", "drag", "interactive", "focusable", "dismiss", "href"];

pub(crate) fn expand_root(root: &Json, env: &Env) -> Result<Vec<El>, Error> {
    let mut out = Vec::new();
    expand(root, &Scope::root(), "root", env, &mut out)?;
    Ok(out)
}

/// A node is an object with one capitalized key: `{ Text: { ... } }`.
pub(crate) fn node_kind(v: &Json) -> Option<(&str, &Json)> {
    let o = v.as_object()?;
    let [(k, props)] = o else { return None };
    k.starts_with(|c: char| c.is_ascii_uppercase()).then_some((&**k, props))
}

fn short(v: &Json) -> String {
    let s = v.to_json();
    if s.chars().count() > 60 { format!("{}…", s.chars().take(60).collect::<String>()) } else { s }
}

fn is_selector(v: &Json) -> bool {
    v.as_object().and_then(|o| o.first()).is_some_and(|(k, _)| k == "state" || k == "switch")
}

fn expand(v: &Json, sc: &Scope, key: &str, env: &Env, out: &mut Vec<El>) -> Result<(), Error> {
    match v {
        Json::Null => return Ok(()),
        Json::Array(a) => {
            for (i, x) in a.iter().enumerate() {
                expand(x, sc, &format!("{key}/{i}"), env, out)?;
            }
            return Ok(());
        }
        // Slot: "$children" → nodes passed by the caller (already substituted in its context).
        Json::String(s) => {
            let r = subst(v, sc);
            if matches!(r, Json::String(_)) {
                return Err(Error::new(format!("expected a node, got \"{s}\"")).at(key));
            }
            return expand(&r, sc, key, env, out);
        }
        _ if is_selector(v) => {
            let r = select_branch(v, sc, env).map_err(|e| e.at(key))?;
            return expand(&r, sc, key, env, out);
        }
        _ => {}
    }
    let Some((kind, props)) = node_kind(v) else {
        return Err(Error::new(format!("expected a node like {{ Text: {{ ... }} }}, got {}", short(v))).at(key));
    };
    let empty = Json::object();
    let props = match props {
        Json::Object(_) => props,
        Json::Null => &empty,
        _ => return Err(Error::new(format!("{kind}: properties must be an object")).at(key)),
    };
    let ctx = |e: Error| e.in_node(kind, key);
    if kind == "For" {
        return expand_for(props, sc, key, env, out).map_err(ctx);
    }
    if let Some(vis) = props.get("visible")
        && !truthy(vis, sc, env).map_err(ctx)?
    {
        return Ok(());
    }
    let key = match props.get("key") {
        Some(k) => to_text(&eval(k, sc, env).map_err(ctx)?),
        None => key.to_string(),
    };
    if let Some(comp) = env.lib.components.get(kind) {
        return expand_component(kind, comp, props, sc, key, env, out);
    }
    let el = build(kind, props, sc, key.clone(), env).map_err(|e| e.in_node(kind, &key))?;
    out.push(el);
    Ok(())
}

/// `For { each: "@todos", as: "item", key: "@item.id", do: {...} }`.
fn expand_for(props: &Json, sc: &Scope, key: &str, env: &Env, out: &mut Vec<El>) -> Result<(), Error> {
    for (k, _) in props.as_object().unwrap() {
        if !FOR.contains(&&**k) {
            return Err(Error::new(format!("unknown property '{k}' (expected each, as, index, key, do)")));
        }
    }
    let each = props.get("each").ok_or_else(|| Error::new("'each' is required"))?;
    let template = props.get("do").ok_or_else(|| Error::new("'do' is required"))?;
    let alias = props.get("as").and_then(Json::as_str).unwrap_or("item");
    let index = props.get("index").and_then(Json::as_str).unwrap_or("index");
    let items = Items::new(each, sc, env)?;
    for i in 0..items.len {
        let isc = sc.with_alias(alias, items.alias(i)).with_alias(index, Alias::Value(Json::from(i)));
        let ik = match props.get("key") {
            Some(k) => to_text(&eval(k, &isc, env)?),
            None => i.to_string(),
        };
        expand(template, &isc, &format!("{key}/{ik}"), env, out)?;
    }
    Ok(())
}

/// Source of For/List items: a path in the model or a literal array.
struct Items {
    path: Option<String>,
    values: Vec<Json>,
    len: usize,
}

impl Items {
    fn new(each: &Json, sc: &Scope, env: &Env) -> Result<Items, Error> {
        match subst(each, sc) {
            Json::String(s) if s.starts_with('@') => {
                let path = s[1..].to_string();
                let len = env.data.get(&path).and_then(Json::as_array).map_or(0, <[Json]>::len);
                Ok(Items { path: Some(path), values: Vec::new(), len })
            }
            other => match eval(&other, sc, env)? {
                Json::Array(values) => Ok(Items { path: None, len: values.len(), values }),
                Json::Null => Ok(Items { path: None, values: Vec::new(), len: 0 }),
                v => Err(Error::new(format!("'each' must be a binding @path or an array, got {}", short(&v)))),
            },
        }
    }

    fn alias(&self, i: usize) -> Alias {
        match &self.path {
            Some(p) => Alias::Path(format!("{p}.{i}")),
            None => Alias::Value(self.values[i].clone()),
        }
    }
}

fn expand_component(
    name: &str,
    comp: &Component,
    props: &Json,
    sc: &Scope,
    key: String,
    env: &Env,
    out: &mut Vec<El>,
) -> Result<(), Error> {
    let ctx = |e: Error| e.in_node(name, &key);
    for (k, _) in props.as_object().unwrap() {
        if !COMMON.contains(&&**k) && !comp.params.iter().any(|(p, _)| p == k) {
            let known: Vec<&str> = comp.params.iter().map(|(p, _)| p.as_str()).collect();
            return Err(ctx(Error::new(format!("unknown parameter '{k}' (parameters: {})", known.join(", ")))));
        }
    }
    let disabled = sc.disabled || props.get("disabled").map(|d| truthy(d, sc, env)).transpose().map_err(ctx)?.unwrap_or(false);
    let mut body = Scope {
        inst: Rc::from(key.as_str()),
        params: Rc::new(Vec::new()),
        aliases: Rc::new(Vec::new()),
        // Only its own state: the caller's state (a button's hover for its icon)
        // comes through parameters — `state` selectors are fixed at substitution.
        flags: env.st.flags(&key, disabled),
        disabled,
    };
    // Passed parameters are substituted in the caller's context: bindings
    // and slots become absolute so the template body doesn't confuse them with its own.
    body.params =
        Rc::new(comp.params.iter().map(|(p, _)| (p.clone(), props.get(p).map_or(Json::Null, |v| subst(v, sc)))).collect());
    // Defaults are in the component's own context: they may refer
    // to its other parameters ($variant) and to its state.
    for (i, (p, default)) in comp.params.iter().enumerate() {
        if props.get(p).is_none() {
            let v = subst(default, &body);
            Rc::get_mut(&mut body.params).expect("params are not shared yet")[i].1 = v;
        }
    }
    let start = out.len();
    expand(&comp.body, &body, &key, env, out).map_err(ctx)?;
    // Common properties of the usage go to the body's root.
    let lay = parse_lay(props, sc, env).map_err(ctx)?;
    let on = parse_on(props, sc, env).map_err(ctx)?;
    let cursor = parse_cursor(props, sc, env).map_err(ctx)?;
    let (tooltip, menu) = parse_info(props, sc, env).map_err(ctx)?;
    let alt = match props.get("alt") {
        Some(a) => alt_text(&eval(a, sc, env).map_err(ctx)?),
        None => None,
    };
    for el in &mut out[start..] {
        merge_lay(&mut el.lay, &lay, props);
        if alt.is_some() {
            el.alt = alt.clone();
        }
        if comp.interactive || !on.is_empty() || el.act.is_some() || tooltip.is_some() || menu.is_some() {
            let act = el.act.get_or_insert_with(Default::default);
            if tooltip.is_some() {
                act.tooltip = tooltip.clone();
            }
            if menu.is_some() {
                act.menu = menu.clone();
            }
            act.on.extend(on.iter().cloned());
            act.disabled |= disabled;
            if let Some(c) = cursor {
                act.cursor = c;
            } else if act.cursor == Cursor::Arrow && !disabled && (comp.interactive || !on.is_empty()) {
                act.cursor = Cursor::Pointer;
            }
            if disabled {
                act.cursor = Cursor::Arrow;
            }
        }
    }
    Ok(())
}

/// `alt`: the text alternative of a node (what an icon means) for those who can't see the screen.
fn alt_text(v: &Json) -> Option<String> {
    Some(to_text(v)).filter(|s| !s.is_empty() && !v.is_null())
}

/// Layout properties set where the component is used replace those of the body's root.
fn merge_lay(dst: &mut Lay, src: &Lay, props: &Json) {
    let has = |k: &str| props.get(k).is_some();
    if has("w") {
        dst.w = src.w;
    }
    if has("h") {
        dst.h = src.h;
    }
    if has("minW") {
        dst.min_w = src.min_w;
    }
    if has("maxW") {
        dst.max_w = src.max_w;
    }
    if has("minH") {
        dst.min_h = src.min_h;
    }
    if has("maxH") {
        dst.max_h = src.max_h;
    }
    if has("flex") {
        dst.flex = src.flex;
    }
    if has("margin") {
        dst.margin = src.margin;
    }
    if has("place") {
        dst.place = src.place;
    }
    if has("pos") {
        dst.pos = src.pos;
    }
    if has("overlay") {
        dst.overlay = src.overlay;
    }
}

/// Layout properties — in one pass over the node's properties (no lookup by name).
fn parse_lay(props: &Json, sc: &Scope, env: &Env) -> Result<Lay, Error> {
    let mut lay = Lay::default();
    for (k, v) in props.as_object().unwrap_or(&[]) {
        let k = &**k;
        if !matches!(k, "w" | "h" | "minW" | "maxW" | "minH" | "maxH" | "flex" | "margin" | "place" | "pos" | "overlay") {
            continue;
        }
        let v = eval(v, sc, env)?;
        match k {
            "w" => lay.w = dim(&v, k)?,
            "h" => lay.h = dim(&v, k)?,
            "minW" => lay.min_w = opt_num(&v, k)?.unwrap_or(0.0),
            "maxW" => lay.max_w = opt_num(&v, k)?.unwrap_or(f32::INFINITY),
            "minH" => lay.min_h = opt_num(&v, k)?.unwrap_or(0.0),
            "maxH" => lay.max_h = opt_num(&v, k)?.unwrap_or(f32::INFINITY),
            "flex" => lay.flex = opt_num(&v, k)?.map(|f| f.max(0.0)),
            "margin" => lay.margin = insets(&v, k)?,
            "place" => lay.place = if v.is_null() { None } else { Some(align(&v, k)?) },
            "pos" => lay.pos = if v.is_null() { None } else { Some(point(&v, k)?) },
            _ => lay.overlay = v.truthy(),
        }
    }
    Ok(lay)
}

fn parse_on(props: &Json, sc: &Scope, env: &Env) -> Result<Vec<(String, String)>, Error> {
    let Some(on) = props.get("on") else { return Ok(Vec::new()) };
    let on = eval(on, sc, env)?;
    let o = on.as_object().ok_or_else(|| Error::new("'on' must be an object { event: \"action\" }"))?;
    o.iter()
        .filter(|(_, v)| !v.is_null())
        .map(|(k, v)| match v.as_str() {
            Some(a) => Ok((k.to_string(), a.to_string())),
            None => Err(Error::new(format!("on.{k} must be an action name"))),
        })
        .collect()
}

/// `tooltip` (empty string — no tooltip) and `contextMenu` (array of items).
fn parse_info(props: &Json, sc: &Scope, env: &Env) -> Result<(Option<String>, Option<Json>), Error> {
    let tooltip = match props.get("tooltip") {
        Some(t) => Some(to_text(&eval(t, sc, env)?)).filter(|t| !t.is_empty()),
        None => None,
    };
    let menu = match props.get("contextMenu") {
        Some(m) => match eval(m, sc, env)? {
            Json::Null => None,
            m @ Json::Array(_) => Some(m),
            _ => return Err(Error::new("contextMenu must be an array of { label, action } and \"-\"")),
        },
        None => None,
    };
    Ok((tooltip, menu))
}

fn parse_cursor(props: &Json, sc: &Scope, env: &Env) -> Result<Option<Cursor>, Error> {
    let Some(c) = props.get("cursor") else { return Ok(None) };
    let c = eval(c, sc, env)?;
    c.as_str().and_then(Cursor::parse).map(Some).ok_or_else(|| Error::new("cursor must be arrow, pointer, text, resize-x or resize-y"))
}

/// Path from an `@path` binding (after subst); otherwise `None`.
fn binding(v: &Json) -> Option<String> {
    v.as_str().and_then(|s| s.strip_prefix('@')).map(str::to_string)
}

/// `For` properties (except `key`): handled before other nodes.
pub(crate) const FOR: [&str; 5] = ["each", "as", "index", "key", "do"];

/// A built-in node's properties (except common ones and behaviors); `None` — not built-in.
pub(crate) fn allowed(kind: &str) -> Option<&'static [&'static str]> {
    Some(match kind {
        "Box" => &["padding", "fill", "stroke", "width", "radius", "clip", "align", "child"],
        "Center" => &["padding", "child"],
        "Align" => &["padding", "align", "child"],
        "Row" => &["padding", "gap", "main", "cross", "stackBelow", "children"],
        "Column" => &["padding", "gap", "main", "cross", "children"],
        "Stack" => &["padding", "children"],
        "Wrap" => &["padding", "gap", "runGap", "cross", "children"],
        "Flow" => &["padding", "align", "lineHeight", "children"],
        "Spacer" => &[],
        "Scroll" => &["child"],
        "List" => &["items", "as", "index", "itemKey", "itemHeight", "item"],
        "Text" => &["text", "size", "bold", "color", "font", "align", "maxLines", "lineHeight", "wrap", "decoration", "fill", "italic"],
        "Input" => &["value", "placeholder", "size", "bold", "color", "placeholderColor", "caretColor", "password", "keyboard", "font"],
        "Image" => &["src", "fit", "radius"],
        "Rect" => &["fill", "stroke", "width", "radius", "at"],
        "Circle" => &["r", "fill", "stroke", "width", "at"],
        "Line" => &["from", "to", "width", "color", "cap"],
        "Tri" => &["points", "fill", "stroke", "width"],
        "Vector" => &["view", "color", "weight", "children"],
        "Canvas" => &["draw"],
        _ => return None,
    })
}

fn text_align(v: &Json) -> Result<f32, Error> {
    Ok(match v.as_str().unwrap_or("left") {
        "left" => 0.0,
        "center" => 0.5,
        "right" => 1.0,
        a => return Err(Error::new(format!("align must be left, center or right, got '{a}'"))),
    })
}

fn theme_color(env: &Env, token: &str, fallback: Color) -> Color {
    env.theme.get(token).and_then(|t| t.as_str()).and_then(Color::from_hex).unwrap_or(fallback)
}

fn theme_num(env: &Env, token: &str, fallback: f32) -> f32 {
    env.theme.get(token).and_then(|t| t.as_f64()).map_or(fallback, |n| n as f32)
}

fn build(kind: &str, props: &Json, sc: &Scope, key: String, env: &Env) -> Result<El, Error> {
    let Some(allowed) = allowed(kind) else {
        return Err(Error::new(format!("unknown node type '{kind}'")));
    };
    for (k, _) in props.as_object().unwrap() {
        if !COMMON.contains(&&**k) && !BEHAVIOR.contains(&&**k) && !allowed.contains(&&**k) {
            return Err(Error::new(format!("unknown property '{k}' (allowed: {})", allowed.join(", "))));
        }
    }
    let has = |k: &str| props.get(k).is_some();
    let disabled = sc.disabled || props.get("disabled").map(|d| truthy(d, sc, env)).transpose()?.unwrap_or(false);
    let behavior_prop = has("toggle") || has("select") || has("drag") || has("dismiss") || has("href");
    let interactive = has("on") || behavior_prop || kind == "Input" || props.get("interactive").is_some_and(Json::truthy);
    // Tooltip and menu also make a node in the map (hover, right button), but not a pressable one.
    let info = has("tooltip") || has("contextMenu");
    // An interactive node is its own state scope for the `state` selectors inside.
    let sc = if interactive {
        Scope { flags: env.st.flags(&key, disabled), disabled, ..sc.clone() }
    } else if disabled != sc.disabled {
        Scope { flags: Flags { disabled, ..sc.flags }, disabled, ..sc.clone() }
    } else {
        sc.clone()
    };
    let sc = &sc;
    let ev = |k: &str| props.get(k).map_or(Ok(Json::Null), |v| eval(v, sc, env));
    let paint = || -> Result<Paint, Error> {
        Ok(Paint {
            fill: color(&ev("fill")?, "fill")?,
            stroke: color(&ev("stroke")?, "stroke")?,
            width: opt_num(&ev("width")?, "width")?.unwrap_or(1.0),
        })
    };
    let font = |default_size: f32| -> Result<Font, Error> {
        Ok(Font {
            family: ev("font")?.as_str().map(Into::into),
            size: opt_num(&ev("size")?, "size")?.unwrap_or(default_size),
            bold: ev("bold")?.truthy(),
            italic: ev("italic")?.truthy(),
        })
    };
    let main_cross = || -> Result<(f32, Main, Cross), Error> {
        let main = match ev("main")?.as_str().unwrap_or("start") {
            "start" => Main::Start,
            "center" => Main::Center,
            "end" => Main::End,
            "between" => Main::Between,
            "around" => Main::Around,
            "evenly" => Main::Evenly,
            m => return Err(Error::new(format!("main must be start, center, end, between, around or evenly, got '{m}'"))),
        };
        let cross = match ev("cross")?.as_str().unwrap_or("start") {
            "start" => Cross::Start,
            "center" => Cross::Center,
            "end" => Cross::End,
            "stretch" => Cross::Stretch,
            c => return Err(Error::new(format!("cross must be start, center, end or stretch, got '{c}'"))),
        };
        Ok((opt_num(&ev("gap")?, "gap")?.unwrap_or(0.0), main, cross))
    };
    let base_size = theme_num(env, "fontSize", 13.0);

    let mut lay = parse_lay(props, sc, env)?;
    lay.padding = insets(&ev("padding")?, "padding")?;
    let mut children = Vec::new();
    let kind = match kind {
        "Box" | "Center" | "Align" => {
            let a = match kind {
                "Center" => Align { x: 0.5, y: 0.5 },
                _ => {
                    let a = ev("align")?;
                    if a.is_null() { Align::TOP_LEFT } else { align(&a, "align")? }
                }
            };
            if kind != "Box" {
                // Center/Align take all available space by default.
                if !has("w") {
                    lay.w = Dim::Fill;
                }
                if !has("h") {
                    lay.h = Dim::Fill;
                }
            }
            if let Some(c) = props.get("child") {
                expand(c, sc, &format!("{key}/0"), env, &mut children)?;
            }
            Kind::Box {
                paint: if kind == "Box" { paint()? } else { Paint::default() },
                radius: opt_num(&ev("radius")?, "radius")?.unwrap_or(0.0),
                clip: ev("clip")?.truthy(),
                align: a,
            }
        }
        "Row" | "Column" | "Stack" | "Wrap" => {
            if let Some(c) = props.get("children") {
                expand(c, sc, &key, env, &mut children)?;
            }
            match kind {
                "Row" => {
                    let (gap, main, cross) = main_cross()?;
                    let stack_below = opt_num(&ev("stackBelow")?, "stackBelow")?.unwrap_or(0.0);
                    Kind::Row { gap, main, cross, stack_below }
                }
                "Column" => {
                    let (gap, main, cross) = main_cross()?;
                    Kind::Column { gap, main, cross }
                }
                "Stack" => Kind::Stack,
                _ => {
                    let gap = opt_num(&ev("gap")?, "gap")?.unwrap_or(0.0);
                    let center = match ev("cross")?.as_str().unwrap_or("start") {
                        "start" => false,
                        "center" => true,
                        c => return Err(Error::new(format!("Wrap cross must be start or center, got '{c}'"))),
                    };
                    Kind::Wrap { gap, run_gap: opt_num(&ev("runGap")?, "runGap")?.unwrap_or(gap), center }
                }
            }
        }
        "Spacer" => {
            if lay.flex.is_none() && !has("w") && !has("h") {
                lay.flex = Some(1.0);
            }
            Kind::Spacer
        }
        "Scroll" => {
            if let Some(c) = props.get("child") {
                expand(c, sc, &format!("{key}/0"), env, &mut children)?;
            }
            let over = env.st.over.get(&key).copied().unwrap_or(0.0);
            Kind::Scroll { offset: env.st.scroll.get(&key).copied().unwrap_or(0.0), content: 0.0, over }
        }
        "List" => {
            let item = props.get("item").ok_or_else(|| Error::new("'item' is required"))?;
            let item_h = opt_num(&ev("itemHeight")?, "itemHeight")?.ok_or_else(|| Error::new("'itemHeight' is required"))?;
            let items = Items::new(props.get("items").ok_or_else(|| Error::new("'items' is required"))?, sc, env)?;
            let alias = props.get("as").and_then(Json::as_str).unwrap_or("item");
            let index = props.get("index").and_then(Json::as_str).unwrap_or("index");
            let offset = env.st.scroll.get(&key).copied().unwrap_or(0.0).max(0.0);
            let view = env.st.viewport.get(&key).copied().unwrap_or(1000.0);
            // Expand only the visible items (the area height comes from the previous layout).
            let first = ((offset / item_h) as usize).min(items.len);
            let last = (((offset + view) / item_h).ceil() as usize + 1).min(items.len);
            for i in first..last {
                let isc = sc.with_alias(alias, items.alias(i)).with_alias(index, Alias::Value(Json::from(i)));
                let ik = match props.get("itemKey") {
                    Some(k) => to_text(&eval(k, &isc, env)?),
                    None => i.to_string(),
                };
                let n = children.len();
                expand(item, &isc, &format!("{key}/{ik}"), env, &mut children)?;
                if children.len() != n + 1 {
                    return Err(Error::new("'item' must produce exactly one node"));
                }
            }
            let over = env.st.over.get(&key).copied().unwrap_or(0.0);
            Kind::List { item_h, count: items.len, first, offset, over }
        }
        "Flow" => {
            if let Some(c) = props.get("children") {
                expand(c, sc, &key, env, &mut children)?;
            }
            Kind::Flow { align: text_align(&ev("align")?)?, line_height: opt_num(&ev("lineHeight")?, "lineHeight")?.unwrap_or(1.0) }
        }
        "Text" => Kind::Text(Box::new(TextEl {
            text: to_text(&ev("text")?),
            font: font(base_size)?,
            color: color(&ev("color")?, "color")?.unwrap_or(theme_color(env, "text", Color::BLACK)),
            align: text_align(&ev("align")?)?,
            max_lines: opt_num(&ev("maxLines")?, "maxLines")?.map_or(usize::MAX, |n| n.max(1.0) as usize),
            line_height: opt_num(&ev("lineHeight")?, "lineHeight")?.unwrap_or(1.0),
            wrap: !matches!(ev("wrap")?, Json::Bool(false)),
            deco: match ev("decoration")?.as_str().unwrap_or("none") {
                "none" => Deco::None,
                "underline" => Deco::Underline,
                "lineThrough" => Deco::LineThrough,
                d => return Err(Error::new(format!("decoration must be none, underline or lineThrough, got '{d}'"))),
            },
            fill: color(&ev("fill")?, "fill")?,
            lines: Vec::new(),
            line_h: 0.0,
            glyph_h: 0.0,
            ascent: 0.0,
            in_flow: false,
            frags: Vec::new(),
        })),
        "Input" => {
            let raw = props.get("value").map(|v| subst(v, sc)).unwrap_or(Json::Null);
            let path = binding(&raw).ok_or_else(|| Error::new("Input 'value' must be a binding \"@path\""))?;
            let value = string(&env.data.get(&path).cloned().unwrap_or(Json::Null));
            let focused = env.st.focus.as_deref() == Some(key.as_str());
            if !has("w") {
                lay.w = Dim::Fill;
            }
            let password = ev("password")?.truthy();
            let keyboard = match ev("keyboard")? {
                Json::Null => uison_core::Keyboard::Text,
                v => uison_core::Keyboard::parse(v.as_str().unwrap_or(""))
                    .ok_or_else(|| Error::new("keyboard must be text, email, number, decimal, phone, url or password"))?,
            };
            let field = focused.then(|| crate::edit::Field::new(&value, env.st.caret, env.st.anchor));
            let shift = env.st.input_shift.as_ref().filter(|(k, _)| focused && *k == key).map_or(0.0, |s| s.1);
            Kind::Input(Box::new(InputEl {
                caret: field.as_ref().map(|f| f.caret),
                caret_on: env.st.caret_on,
                sel: field.as_ref().and_then(crate::edit::Field::selection),
                sel_color: theme_color(env, "textSelection", Color::rgb(0xB4, 0xD5, 0xFE)),
                shift,
                path,
                value,
                placeholder: to_text(&ev("placeholder")?),
                font: font(base_size)?,
                color: color(&ev("color")?, "color")?.unwrap_or(theme_color(env, "text", Color::BLACK)),
                placeholder_color: color(&ev("placeholderColor")?, "placeholderColor")?
                    .unwrap_or(theme_color(env, "muted", Color::rgb(142, 142, 147))),
                caret_color: color(&ev("caretColor")?, "caretColor")?
                    .unwrap_or(theme_color(env, "primary", Color::rgb(0, 122, 255))),
                password,
                keyboard: if password { uison_core::Keyboard::Password } else { keyboard },
                line_h: 0.0,
            }))
        }
        "Image" => Kind::Image {
            src: to_text(&ev("src")?),
            fit: match ev("fit")? {
                Json::Null => uison_core::Fit::Cover,
                f => uison_core::Fit::parse(f.as_str().unwrap_or("")).ok_or_else(|| Error::new("fit must be cover, contain or fill"))?,
            },
            radius: opt_num(&ev("radius")?, "radius")?.unwrap_or(0.0),
        },
        "Rect" => {
            if let Some(at) = props.get("at") {
                lay.pos = Some(point(&eval(at, sc, env)?, "at")?);
            }
            Kind::Prim(Prim::Rect { paint: paint()?, radius: opt_num(&ev("radius")?, "radius")?.unwrap_or(0.0) })
        }
        "Circle" => {
            let r = num(&ev("r")?, "r")?;
            if let Some(at) = props.get("at") {
                let c = point(&eval(at, sc, env)?, "at")?;
                lay.pos = Some(Point::new(c.x - r, c.y - r));
            }
            Kind::Prim(Prim::Circle { r, paint: paint()? })
        }
        "Line" => Kind::Prim(Prim::Line {
            from: point(&ev("from")?, "from")?,
            to: point(&ev("to")?, "to")?,
            width: opt_num(&ev("width")?, "width")?,
            color: color(&ev("color")?, "color")?,
            round: ev("cap")?.as_str() == Some("round"),
        }),
        "Tri" => {
            let pts = ev("points")?;
            let p: Vec<f32> = match pts.as_array() {
                Some(a) if a.len() == 6 => a.iter().map(|x| num(x, "points")).collect::<Result<_, _>>()?,
                _ => return Err(Error::new("'points' must be [x1, y1, x2, y2, x3, y3]")),
            };
            Kind::Prim(Prim::Tri {
                points: [Point::new(p[0], p[1]), Point::new(p[2], p[3]), Point::new(p[4], p[5])],
                paint: paint()?,
            })
        }
        "Vector" => {
            if let Some(c) = props.get("children") {
                expand(c, sc, &key, env, &mut children)?;
            }
            if let Some(bad) = children.iter().find(|c| !matches!(c.kind, Kind::Prim(_))) {
                return Err(Error::new(format!("Vector may contain only Rect, Circle, Line, Tri (got {})", bad.key)));
            }
            let view = opt_num(&ev("view")?, "view")?.unwrap_or(24.0);
            if !has("w") {
                lay.w = Dim::Px(view);
            }
            if !has("h") {
                lay.h = Dim::Px(view);
            }
            Kind::Vector {
                view,
                color: color(&ev("color")?, "color")?.unwrap_or(theme_color(env, "text", Color::BLACK)),
                weight: opt_num(&ev("weight")?, "weight")?.unwrap_or(1.5),
            }
        }
        "Canvas" => Kind::Canvas { draw: to_text(&ev("draw")?) },
        _ => unreachable!(),
    };

    let mut el = El::new(key, kind);
    el.alt = alt_text(&ev("alt")?);
    el.lay = lay;
    el.children = children;
    let is_scroll = matches!(el.kind, Kind::Scroll { .. } | Kind::List { .. });
    if interactive || is_scroll || info {
        let behavior = if has("toggle") {
            binding(&subst(props.get("toggle").unwrap(), sc)).map(Behavior::Toggle)
        } else if has("select") {
            // One { path, value } entry or several.
            let s = subst(props.get("select").unwrap(), sc);
            let list = match &s {
                Json::Array(a) => a.clone(),
                _ => vec![s.clone()],
            };
            let mut sets = Vec::new();
            for x in &list {
                let path = x.get("path").and_then(binding).ok_or_else(|| Error::new("select needs { path: \"@...\", value }"))?;
                sets.push((path, eval(x.get("value").unwrap_or(&Json::Null), sc, env)?));
            }
            Some(Behavior::Select(sets))
        } else if has("drag") {
            let s = subst(props.get("drag").unwrap(), sc);
            let path = s.get("path").and_then(binding).ok_or_else(|| Error::new("drag needs { path: \"@...\", min, max }"))?;
            let n = |k: &str, d: f32| -> Result<f32, Error> {
                opt_num(&eval(s.get(k).unwrap_or(&Json::Null), sc, env)?, k).map(|v| v.unwrap_or(d))
            };
            let (min, max, step, inset, delta) = (n("min", 0.0)?, n("max", 1.0)?, n("step", 0.0)?, n("inset", 0.0)?, n("delta", 0.0)?);
            Some(Behavior::Drag { path, min, max, step, inset, delta })
        } else {
            None
        };
        let cursor = match parse_cursor(props, sc, env)? {
            Some(c) => c,
            None if disabled || !interactive || has("dismiss") => Cursor::Arrow,
            None if matches!(el.kind, Kind::Input(_)) => Cursor::Text,
            None => Cursor::Pointer,
        };
        let (tooltip, menu) = parse_info(props, sc, env)?;
        let href = match ev("href")? {
            Json::Null => None,
            h => Some(to_text(&h)).filter(|h| !h.is_empty()),
        };
        el.act = Some(Box::new(Act {
            tooltip,
            menu,
            href,
            on: parse_on(props, sc, env)?,
            behavior,
            cursor,
            focusable: matches!(el.kind, Kind::Input(_)) || ev("focusable")?.truthy(),
            disabled,
            dismiss: props.get("dismiss").map(|d| subst(d, sc)).as_ref().and_then(binding),
        }));
    }
    Ok(el)
}

