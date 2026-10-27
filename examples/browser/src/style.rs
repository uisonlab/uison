//! Cascade: the browser's default styles (UA) → the page's `<style>` → the `style` attribute;
//! specificity, order, `!important`, inheritance. The result is the computed style
//! of each element (lengths in points, colors resolved).

use uison::Color;

use crate::css::{Decl, Len, Sheet, parse_color, parse_decls, parse_len, split_top};
use crate::html::{Data, Dom};

/// The window the page is shown in: for `@media`, `vw`, `%`.
#[derive(Clone, Copy, Debug)]
pub struct Env {
    pub width: f32,
    pub height: f32,
    pub dark: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Display {
    Block,
    Inline,
    InlineBlock,
    Flex,
    ListItem,
    Table,
    RowGroup,
    Row,
    Cell,
    None,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ws {
    Normal,
    NoWrap,
    /// `pre`, `pre-wrap`: spaces and newlines as is.
    Pre,
    /// `pre-line`: newlines as is, spaces collapse.
    PreLine,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Deco {
    None,
    Underline,
    LineThrough,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ListStyle {
    None,
    Disc,
    Circle,
    Square,
    Decimal,
    LowerAlpha,
    UpperAlpha,
    LowerRoman,
    UpperRoman,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Transform {
    None,
    Upper,
    Lower,
    Capitalize,
}

/// Block size: points or a fraction of the parent's width.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Size {
    Px(f32),
    Pct(f32),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Style {
    pub display: Display,
    pub color: Color,
    pub background: Option<Color>,
    pub font_size: f32,
    pub bold: bool,
    pub italic: bool,
    /// `monospace`, `serif` or `None` — the system font.
    pub family: Option<&'static str>,
    /// 0 — left, 0.5 — center, 1 — right.
    pub align: f32,
    /// Line height multiplier.
    pub line_height: f32,
    pub ws: Ws,
    pub deco: Deco,
    pub transform: Transform,
    pub list: ListStyle,
    pub hidden: bool,
    pub pointer: bool,
    /// Margins top, right, bottom, left; `None` — `auto`.
    pub margin: [Option<f32>; 4],
    pub padding: [f32; 4],
    pub border: [f32; 4],
    pub border_color: [Color; 4],
    pub radius: f32,
    /// `box-sizing: border-box`: the width includes padding and border.
    pub border_box: bool,
    pub width: Option<Size>,
    pub height: Option<Size>,
    pub min_width: Option<Size>,
    pub max_width: Option<Size>,
    pub min_height: Option<Size>,
    pub row: bool,
    pub wrap: bool,
    pub justify: &'static str,
    pub align_items: &'static str,
    pub gap: f32,
    pub row_gap: f32,
    pub grow: f32,
    /// CSS variables (`--x`), inherited.
    pub vars: std::rc::Rc<Vec<(Box<str>, Box<str>)>>,
    /// `float`: 0 — none, 1 — left, 2 — right.
    pub float: u8,
    /// `clear`: the block starts below floats.
    pub clear: bool,
    /// `overflow: hidden/clip`: content is clipped to the border.
    pub clip: bool,
    pub max_height: Option<Size>,
    /// `position: absolute/fixed` "covering the parent" (`inset: 0` or 100% × 100%) — an image
    /// in an aspect-ratio block; other absolute elements aren't shown.
    pub abs_fill: bool,
    /// Vertical padding is given in percent (the "aspect-ratio block" trick).
    pub pct_pad: bool,
    pub(crate) abs: bool,
    pub(crate) inset0: bool,
    /// `mask-image`: the background is paint for the mask (icons) and isn't drawn by itself.
    pub masked: bool,
}

impl Style {
    /// Root style: what is inherited by default.
    pub fn root() -> Style {
        Style {
            display: Display::Block,
            color: Color::BLACK,
            background: None,
            font_size: 16.0,
            bold: false,
            italic: false,
            family: None,
            align: 0.0,
            line_height: 1.2,
            ws: Ws::Normal,
            deco: Deco::None,
            transform: Transform::None,
            list: ListStyle::Disc,
            hidden: false,
            pointer: false,
            margin: [Some(0.0); 4],
            padding: [0.0; 4],
            border: [0.0; 4],
            border_color: [Color::BLACK; 4],
            radius: 0.0,
            border_box: false,
            width: None,
            height: None,
            min_width: None,
            max_width: None,
            min_height: None,
            row: true,
            wrap: false,
            justify: "start",
            align_items: "stretch",
            gap: 0.0,
            row_gap: 0.0,
            grow: 0.0,
            vars: std::rc::Rc::default(),
            float: 0,
            clear: false,
            clip: false,
            max_height: None,
            abs_fill: false,
            pct_pad: false,
            abs: false,
            inset0: false,
            masked: false,
        }
    }

    /// A child: inherited properties from the parent, the rest are initial values.
    fn inherit(&self) -> Style {
        let root = Style::root();
        Style {
            color: self.color,
            font_size: self.font_size,
            bold: self.bold,
            italic: self.italic,
            family: self.family,
            align: self.align,
            line_height: self.line_height,
            ws: self.ws,
            // Text decoration propagates to inline descendants.
            deco: self.deco,
            transform: self.transform,
            list: self.list,
            hidden: self.hidden,
            pointer: self.pointer,
            vars: self.vars.clone(),
            display: Display::Inline,
            border_color: [self.color; 4],
            ..root
        }
    }
}

/// The browser's default styles — as in the HTML Living Standard, simplified.
pub const UA: &str = r#"
html, address, blockquote, body, center, dialog, div, figure, figcaption, footer, form, header, hr, legend, main, nav,
ol, p, pre, section, ul, dl, dt, dd, article, aside, details, summary, fieldset, h1, h2, h3, h4, h5, h6, menu, search,
hgroup, caption, picture { display: block }
head, script, style, title, meta, link, template, base, datalist, [hidden], area, param, noembed, noframes { display: none }
svg, canvas, video, audio, iframe, object, embed, math, dialog:not([open]) { display: none }
li { display: list-item }
table { display: table; border-spacing: 2px }
thead, tbody, tfoot { display: table-row-group }
tr { display: table-row }
td, th { display: table-cell; padding: 1px }
th { font-weight: bold; text-align: center }
caption { text-align: center }
body { margin: 8px }
p, dl, ul, ol, menu, pre { margin: 1em 0 }
blockquote, figure { margin: 1em 40px }
h1 { font-size: 2em; margin: .67em 0 }
h2 { font-size: 1.5em; margin: .83em 0 }
h3 { font-size: 1.17em; margin: 1em 0 }
h4 { margin: 1.33em 0 }
h5 { font-size: .83em; margin: 1.67em 0 }
h6 { font-size: .67em; margin: 2.33em 0 }
h1, h2, h3, h4, h5, h6, b, strong, dt, legend { font-weight: bold }
i, em, cite, var, dfn, address { font-style: italic }
code, kbd, samp, tt, pre, listing, xmp, plaintext { font-family: monospace }
code, kbd, samp, tt { font-size: .875em }
pre { white-space: pre; font-size: .875em }
a:link { color: #0000EE; text-decoration: underline; cursor: pointer }
u, ins { text-decoration: underline }
s, strike, del { text-decoration: line-through }
small, sub, sup { font-size: smaller }
big { font-size: larger }
mark { background-color: yellow; color: black }
hr { border: 1px solid #a0a0a0; margin: .5em 0 }
ul, ol, menu { padding-left: 40px }
ul, menu { list-style-type: disc }
ol { list-style-type: decimal }
ul ul, ol ul { list-style-type: circle }
ul ul ul, ol ol ul, ol ul ul, ul ol ul { list-style-type: square }
dd { margin-left: 40px }
center { text-align: center }
center table, table[align=center] { margin-left: auto; margin-right: auto }
button { cursor: pointer }
"#;

/// Computed styles of the document's elements; text has `None` (the parent's style).
pub struct Styled {
    pub styles: Vec<Option<Style>>,
    /// Style when hovering the element itself — if some `:hover` rule changes it.
    pub hover: Vec<Option<Box<Style>>>,
    /// The page background (`body`/`html`) and its text color.
    pub page_bg: Option<Color>,
}

pub fn compute(dom: &Dom, ua: &Sheet, author: &[Sheet], env: Env) -> Styled {
    let mut out = Styled { styles: vec![None; dom.nodes.len()], hover: vec![None; dom.nodes.len()], page_bg: None };
    let sheets: Vec<&Sheet> = std::iter::once(ua).chain(author).collect();
    let root = Style::root();
    walk(dom, 0, &root, &sheets, env, &mut out, &mut Ancestors::default());
    for tag in ["body", "html"] {
        if out.page_bg.is_none()
            && let Some(id) = dom.find(tag)
        {
            out.page_bg = out.styles[id].as_ref().and_then(|s| s.background);
        }
    }
    out
}

/// Keys of the current element's ancestors (with counters): a filter for descendant selectors.
#[derive(Default)]
struct Ancestors(std::collections::HashMap<u64, u32, crate::css::KeyHash>);

impl Ancestors {
    fn has_all(&self, keys: &[u64]) -> bool {
        keys.iter().all(|k| self.0.contains_key(k))
    }
}

fn walk(dom: &Dom, id: usize, parent: &Style, sheets: &[&Sheet], env: Env, out: &mut Styled, anc: &mut Ancestors) {
    let node = &dom.nodes[id];
    let style = match &node.data {
        Data::Element { .. } => {
            let s = cascade(dom, id, parent, sheets, env, None, anc);
            let has_hover = sheets.iter().any(|sh| {
                sh.hover
                    .iter()
                    .map(|&i| &sh.rules[i as usize])
                    .any(|r| anc.has_all(&r.sel.anc) && media_ok(sh, r.media, env) && r.sel.matches(dom, id, Some(id)))
            });
            if has_hover {
                let h = cascade(dom, id, parent, sheets, env, Some(id), anc);
                if h != s {
                    out.hover[id] = Some(Box::new(h));
                }
            }
            out.styles[id] = Some(s.clone());
            s
        }
        _ => parent.clone(),
    };
    // A hidden subtree isn't shown: its descendants' styles aren't needed.
    if style.display == Display::None {
        return;
    }
    let mut keys = Vec::new();
    crate::css::keys_of(node, &mut keys);
    for &k in &keys {
        *anc.0.entry(k).or_default() += 1;
    }
    for &c in &node.children {
        walk(dom, c, &style, sheets, env, out, anc);
    }
    for k in keys {
        if let Some(n) = anc.0.get_mut(&k) {
            *n -= 1;
            if *n == 0 {
                anc.0.remove(&k);
            }
        }
    }
}

fn media_ok(sheet: &Sheet, media: Option<usize>, env: Env) -> bool {
    media.is_none_or(|m| sheet.media[m].iter().any(|q| q.matches(env.width, env.dark)))
}

/// Cascade order: importance, origin (UA, page, `style=""`), specificity, order.
type Rank = (bool, u8, u32, usize);

/// An element's style: matching declarations in cascade order on top of the inherited style.
fn cascade(dom: &Dom, id: usize, parent: &Style, sheets: &[&Sheet], env: Env, hover: Option<usize>, anc: &Ancestors) -> Style {
    let mut found: Vec<(Rank, &Decl)> = Vec::new();
    let mut order = 0;
    let mut cand = Vec::new();
    for (origin, sheet) in sheets.iter().enumerate() {
        sheet.candidates(dom, id, &mut cand);
        for &i in &cand {
            let r = &sheet.rules[i as usize];
            if anc.has_all(&r.sel.anc) && media_ok(sheet, r.media, env) && r.sel.matches(dom, id, hover) {
                for d in &sheet.blocks[r.block] {
                    found.push(((d.important, origin.min(1) as u8, r.sel.spec, order + i as usize), d));
                }
            }
        }
        order += sheet.rules.len();
    }
    let inline = dom.nodes[id].attr("style").map(parse_decls).unwrap_or_default();
    for d in &inline {
        found.push(((d.important, 2, u32::MAX, 0), d));
    }
    found.sort_by_key(|f| f.0);
    let mut s = parent.inherit();
    // Variables `--x` (inherited), then `var()` in values is substituted before parsing.
    if found.iter().any(|(_, d)| d.name.starts_with("--")) {
        let mut vars = (*s.vars).clone();
        for (_, d) in found.iter().filter(|(_, d)| d.name.starts_with("--")) {
            let v = subst_vars(&d.value, &vars);
            match vars.iter_mut().find(|(k, _)| **k == *d.name) {
                Some(e) => e.1 = v.into(),
                None => vars.push((d.name.as_str().into(), v.into())),
            }
        }
        s.vars = std::rc::Rc::new(vars);
    }
    let resolved: Vec<std::borrow::Cow<Decl>> = found
        .iter()
        .filter(|(_, d)| !d.name.starts_with("--"))
        .map(|&(_, d)| {
            if d.has_var {
                std::borrow::Cow::Owned(Decl { value: subst_vars(&d.value, &s.vars), ..d.clone() })
            } else {
                std::borrow::Cow::Borrowed(d)
            }
        })
        .collect();
    // Font size first: the other properties' `em` are computed from it.
    for d in &resolved {
        if d.name == "font-size" || d.name == "font" {
            apply(&mut s, parent, d, env, true);
        }
    }
    for d in &resolved {
        apply(&mut s, parent, d, env, false);
    }
    // No positioning: absolute elements on sites are menus, tooltips, labels for
    // screen readers; we don't show them. Except "covering the parent" — that's the content of an
    // aspect-ratio block (image, video): it goes into the flow instead of the spacer padding.
    // A collapsed block (spoiler, "accordion"): height 0 and clipping — nothing is visible.
    if s.clip && (s.max_height == Some(Size::Px(0.0)) || s.height == Some(Size::Px(0.0))) {
        s.display = Display::None;
    }
    if s.abs {
        s.abs_fill = s.inset0 || (s.width == Some(Size::Pct(1.0)) && s.height == Some(Size::Pct(1.0)));
        if !s.abs_fill {
            s.display = Display::None;
        }
    }
    // Presentational attributes from old HTML: `bgcolor`, `width`, `align`, `<font color>`.
    let n = &dom.nodes[id];
    if s.background.is_none()
        && let Some(c) = n.attr("bgcolor").and_then(parse_color)
    {
        s.background = Some(c);
    }
    if matches!(n.tag(), Some("font" | "body"))
        && let Some(c) = n.attr(if n.tag() == Some("body") { "text" } else { "color" }).and_then(parse_color)
    {
        s.color = c;
    }
    // <font size=1..7> and relative +1, -2.
    if n.tag() == Some("font")
        && let Some(v) = n.attr("size")
    {
        let v = v.trim();
        let k = match v.strip_prefix(['+', '-']) {
            Some(d) => d.parse::<i32>().ok().map(|d| if v.starts_with('-') { 3 - d } else { 3 + d }),
            None => v.parse().ok(),
        };
        if let Some(k) = k {
            s.font_size = [10.0, 13.0, 16.0, 18.0, 24.0, 32.0, 48.0][(k.clamp(1, 7) - 1) as usize];
        }
    }
    if n.tag() == Some("font")
        && let Some(f) = n.attr("face")
    {
        s.family = family(&f.to_ascii_lowercase());
    }
    if s.width.is_none()
        && let Some(w) = n.attr("width")
    {
        s.width = size(w, &s, env);
    }
    if let Some(a) = n.attr("align") {
        match a.to_ascii_lowercase().as_str() {
            "center" | "middle" => s.align = 0.5,
            "right" => s.align = 1.0,
            _ => {}
        }
    }
    s
}

/// Length in points: `em` from `font`, `%` from `base`.
fn px(l: Len, font: f32, base: f32, env: Env) -> Option<f32> {
    Some(match l {
        Len::Px(v) => v,
        Len::Em(v) => v * font,
        Len::Rem(v) => v * 16.0,
        Len::Pct(v) => v * base / 100.0,
        Len::Vw(v) => v * env.width / 100.0,
        Len::Vh(v) => v * env.height / 100.0,
        Len::Lin { px, em, pct, vw } => px + em * font + pct * base / 100.0 + vw * env.width / 100.0,
        Len::Auto => return None,
    })
}

/// Substitute `var(--x)` / `var(--x, fallback)` (nested ones too).
fn subst_vars(v: &str, vars: &[(Box<str>, Box<str>)]) -> String {
    let mut out = v.to_string();
    for _ in 0..8 {
        let Some(p) = out.find("var(") else { break };
        let Some(end) = matching(&out, p + 3) else { break };
        let inner = &out[p + 4..end];
        let (name, fallback) = match inner.split_once(',') {
            Some((n, f)) => (n.trim(), Some(f.trim())),
            None => (inner.trim(), None),
        };
        let val = vars.iter().rev().find(|(k, _)| **k == *name).map(|(_, v)| v.to_string()).or(fallback.map(str::to_string)).unwrap_or_default();
        out.replace_range(p..=end, &val);
    }
    out
}

/// The closing parenthesis for the opening one at `open`.
fn matching(s: &str, open: usize) -> Option<usize> {
    let mut depth = 0;
    for (i, c) in s[open..].char_indices() {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(open + i);
                }
            }
            _ => {}
        }
    }
    None
}

fn size(v: &str, s: &Style, env: Env) -> Option<Size> {
    match parse_len(v)? {
        Len::Pct(p) => Some(Size::Pct(p / 100.0)),
        // `calc(100% - 2em)`: the fraction without the offset — in uison a size is either a fraction or points.
        Len::Lin { pct, .. } if pct != 0.0 => Some(Size::Pct(pct / 100.0)),
        Len::Auto => None,
        l => px(l, s.font_size, 0.0, env).map(Size::Px),
    }
}

/// Space-separated values (outside parentheses): `1px solid rgb(0, 0, 0)` → 3 parts.
fn parts(v: &str) -> Vec<&str> {
    split_top(v, ' ').into_iter().map(str::trim).filter(|p| !p.is_empty()).collect()
}

/// 1–4 values → top, right, bottom, left.
fn four<T: Copy>(vals: &[T]) -> Option<[T; 4]> {
    Some(match *vals {
        [a] => [a, a, a, a],
        [a, b] => [a, b, a, b],
        [a, b, c] => [a, b, c, b],
        [a, b, c, d] => [a, b, c, d],
        _ => return None,
    })
}

const SIDES: [&str; 4] = ["top", "right", "bottom", "left"];

fn apply(s: &mut Style, parent: &Style, d: &Decl, env: Env, font_pass: bool) {
    let v = d.value.trim();
    let lower = v.to_ascii_lowercase();
    let len = |x: &str| parse_len(x).and_then(|l| px(l, s.font_size, env.width, env));
    if font_pass {
        match d.name.as_str() {
            "font-size" => {
                if lower == "inherit" {
                    s.font_size = parent.font_size;
                } else if let Some(f) = font_size(&lower, parent.font_size) {
                    s.font_size = f;
                }
            }
            "font" => font_shorthand(s, parent, &lower),
            _ => {}
        }
        return;
    }
    // `inherit` is the parent's value (not what earlier rules have already set).
    if lower == "inherit" {
        match d.name.as_str() {
            "color" => s.color = parent.color,
            "font-weight" => s.bold = parent.bold,
            "font-style" => s.italic = parent.italic,
            "font-family" => s.family = parent.family,
            "line-height" => s.line_height = parent.line_height,
            "text-align" => s.align = parent.align,
            "text-decoration" | "text-decoration-line" => s.deco = parent.deco,
            "white-space" => s.ws = parent.ws,
            "text-transform" => s.transform = parent.transform,
            "background" | "background-color" => s.background = parent.background,
            "display" => s.display = parent.display,
            _ => {}
        }
        return;
    }
    match d.name.as_str() {
        "font-size" | "font" => {}
        "color" => {
            if let Some(c) = color(&lower, s) {
                s.color = c;
            }
        }
        "background-color" | "background" => {
            // In `background` — the first color (for a gradient, the first stop).
            if lower == "none" || lower == "transparent" {
                s.background = None;
            } else if let Some(c) = parts(&lower).iter().find_map(|p| color(p, s).or_else(|| gradient_color(p, s))) {
                s.background = (c.a > 0).then_some(c);
            }
        }
        "display" => {
            s.display = match lower.split_whitespace().last().unwrap_or("") {
                "block" | "flow-root" | "grid" => Display::Block,
                "inline" | "contents" => Display::Inline,
                "inline-block" | "inline-grid" | "inline-table" => Display::InlineBlock,
                "flex" | "inline-flex" => Display::Flex,
                "list-item" => Display::ListItem,
                "table" => Display::Table,
                "table-row-group" | "table-header-group" | "table-footer-group" => Display::RowGroup,
                "table-row" => Display::Row,
                "table-cell" => Display::Cell,
                "none" => Display::None,
                _ => s.display,
            }
        }
        "font-weight" => s.bold = matches!(lower.as_str(), "bold" | "bolder") || lower.parse::<u32>().is_ok_and(|w| w >= 600),
        "font-style" => s.italic = lower == "italic" || lower.starts_with("oblique"),
        "font-family" => s.family = family(&lower),
        "line-height" => {
            if let Some(lh) = line_height(&lower, s.font_size) {
                s.line_height = lh;
            }
        }
        "text-align" => {
            s.align = match lower.as_str() {
                "center" | "-webkit-center" => 0.5,
                "right" | "end" => 1.0,
                _ => 0.0,
            }
        }
        "text-decoration" | "text-decoration-line" => {
            s.deco = if lower.contains("underline") {
                Deco::Underline
            } else if lower.contains("line-through") {
                Deco::LineThrough
            } else if lower.contains("none") {
                Deco::None
            } else {
                s.deco
            }
        }
        "text-transform" => {
            s.transform = match lower.as_str() {
                "uppercase" => Transform::Upper,
                "lowercase" => Transform::Lower,
                "capitalize" => Transform::Capitalize,
                _ => Transform::None,
            }
        }
        "white-space" => {
            s.ws = match lower.as_str() {
                "pre" | "pre-wrap" | "break-spaces" => Ws::Pre,
                "pre-line" => Ws::PreLine,
                "nowrap" => Ws::NoWrap,
                _ => Ws::Normal,
            }
        }
        "list-style-type" | "list-style" => {
            for p in parts(&lower) {
                s.list = match p {
                    "none" => ListStyle::None,
                    "disc" => ListStyle::Disc,
                    "circle" => ListStyle::Circle,
                    "square" => ListStyle::Square,
                    "decimal" | "decimal-leading-zero" => ListStyle::Decimal,
                    "lower-alpha" | "lower-latin" => ListStyle::LowerAlpha,
                    "upper-alpha" | "upper-latin" => ListStyle::UpperAlpha,
                    "lower-roman" => ListStyle::LowerRoman,
                    "upper-roman" => ListStyle::UpperRoman,
                    _ => continue,
                };
            }
        }
        "visibility" => s.hidden = lower == "hidden" || lower == "collapse",
        "cursor" => s.pointer = lower == "pointer",
        "margin" => {
            let vals: Vec<Option<f32>> = parts(&lower).iter().map(|p| if *p == "auto" { None } else { Some(len(p).unwrap_or(0.0)) }).collect();
            if let Some(m) = four(&vals) {
                s.margin = m;
            }
        }
        "padding" => {
            let ps = parts(&lower);
            s.pct_pad = ps.first().is_some_and(|p| p.ends_with('%')) || ps.get(2).is_some_and(|p| p.ends_with('%'));
            let vals: Vec<f32> = ps.iter().map(|p| len(p).unwrap_or(0.0).max(0.0)).collect();
            if let Some(p) = four(&vals) {
                s.padding = p;
            }
        }
        "border" => border(s, &lower, &[0, 1, 2, 3]),
        "border-width" => {
            let vals: Vec<f32> = parts(&lower).iter().map(|p| border_width(p, s, env)).collect();
            if let Some(w) = four(&vals) {
                s.border = w;
            }
        }
        "border-color" => {
            let vals: Vec<Color> = parts(&lower).iter().filter_map(|p| color(p, s)).collect();
            if let Some(c) = four(&vals) {
                s.border_color = c;
            }
        }
        "border-style" => {
            let vals: Vec<&str> = parts(&lower);
            if let Some(st) = four(&vals) {
                for (w, st) in s.border.iter_mut().zip(st) {
                    if st == "none" || st == "hidden" {
                        *w = 0.0;
                    } else if *w == 0.0 {
                        *w = 3.0; // `medium` — until a width is given
                    }
                }
            }
        }
        "border-radius" => {
            let first = parts(&lower).first().copied().unwrap_or("0").split('/').next().unwrap_or("0").to_string();
            s.radius = match parse_len(&first) {
                Some(Len::Pct(p)) if p >= 50.0 => 9999.0,
                Some(l) => px(l, s.font_size, 0.0, env).unwrap_or(0.0),
                None => 0.0,
            };
        }
        "box-sizing" => s.border_box = lower == "border-box",
        "float" => {
            s.float = match lower.as_str() {
                "left" | "inline-start" => 1,
                "right" | "inline-end" => 2,
                _ => 0,
            }
        }
        "clear" => s.clear = lower != "none",
        // No translucency; fully transparent things (menu "toggle" checkboxes) aren't shown.
        "opacity" if lower.parse::<f32>().is_ok_and(|o| o <= 0.0) => s.display = Display::None,
        // No positioning: such elements on sites are menus, tooltips, labels for
        // screen readers; showing them in place is worse than not showing them.
        "position" => s.abs = lower == "absolute" || lower == "fixed",
        "inset" => s.inset0 = parts(&lower).iter().all(|p| parse_len(p).is_some_and(|l| l == Len::Px(0.0))),
        "mask-image" | "-webkit-mask-image" | "mask" | "-webkit-mask" => s.masked = lower != "none",
        "width" => s.width = size(&lower, s, env),
        "height" => s.height = size(&lower, s, env),
        "min-width" => s.min_width = size(&lower, s, env),
        "max-width" => s.max_width = size(&lower, s, env),
        "min-height" => s.min_height = size(&lower, s, env),
        "max-height" => s.max_height = size(&lower, s, env),
        "overflow" | "overflow-y" => s.clip = parts(&lower).iter().any(|p| matches!(*p, "hidden" | "clip" | "auto" | "scroll")),
        "flex-direction" => s.row = !lower.starts_with("column"),
        "flex-wrap" => s.wrap = lower == "wrap" || lower == "wrap-reverse",
        "flex-flow" => {
            for p in parts(&lower) {
                match p {
                    "row" | "row-reverse" => s.row = true,
                    "column" | "column-reverse" => s.row = false,
                    "wrap" | "wrap-reverse" => s.wrap = true,
                    "nowrap" => s.wrap = false,
                    _ => {}
                }
            }
        }
        "justify-content" => {
            s.justify = match lower.as_str() {
                "center" => "center",
                "flex-end" | "end" | "right" => "end",
                "space-between" => "between",
                "space-around" => "around",
                "space-evenly" => "evenly",
                _ => "start",
            }
        }
        "align-items" => {
            s.align_items = match lower.as_str() {
                "center" => "center",
                "flex-end" | "end" => "end",
                "flex-start" | "start" | "baseline" | "first baseline" => "start",
                _ => "stretch",
            }
        }
        "gap" | "grid-gap" => {
            let p = parts(&lower);
            s.row_gap = p.first().and_then(|x| len(x)).unwrap_or(0.0);
            s.gap = p.get(1).and_then(|x| len(x)).unwrap_or(s.row_gap);
        }
        "column-gap" => s.gap = len(&lower).unwrap_or(0.0),
        "row-gap" => s.row_gap = len(&lower).unwrap_or(0.0),
        "flex" => {
            s.grow = match lower.as_str() {
                "none" => 0.0,
                "auto" => 1.0,
                _ => parts(&lower).first().and_then(|p| p.parse().ok()).unwrap_or(0.0),
            }
        }
        "flex-grow" => s.grow = lower.parse().unwrap_or(0.0),
        name => {
            // margin-top, padding-left, border-bottom, border-left-color…
            let Some(i) = SIDES.iter().position(|side| name.contains(side)) else { return };
            let side = SIDES[i];
            let prop = name.replacen(side, "*", 1);
            match prop.as_str() {
                "margin-*" => s.margin[i] = if lower == "auto" { None } else { Some(len(&lower).unwrap_or(0.0)) },
                "padding-*" => {
                    s.padding[i] = len(&lower).unwrap_or(0.0).max(0.0);
                    s.pct_pad |= (i == 0 || i == 2) && lower.ends_with('%');
                }
                "border-*" => border(s, &lower, &[i]),
                "border-*-width" => s.border[i] = border_width(&lower, s, env),
                "border-*-color" => {
                    if let Some(c) = color(&lower, s) {
                        s.border_color[i] = c;
                    }
                }
                "border-*-style" if lower == "none" || lower == "hidden" => s.border[i] = 0.0,
                _ => {}
            }
        }
    }
}

fn color(v: &str, s: &Style) -> Option<Color> {
    if v == "currentcolor" || v == "inherit" {
        return Some(s.color);
    }
    parse_color(v)
}

/// The first gradient color approximates the background.
fn gradient_color(p: &str, s: &Style) -> Option<Color> {
    let inner = p.split_once('(')?.1;
    split_top(inner.trim_end_matches(')'), ',').into_iter().find_map(|a| parts(a).iter().find_map(|x| color(x, s)))
}

fn border_width(p: &str, s: &Style, env: Env) -> f32 {
    match p {
        "thin" => 1.0,
        "medium" => 3.0,
        "thick" => 5.0,
        _ => parse_len(p).and_then(|l| px(l, s.font_size, 0.0, env)).unwrap_or(0.0),
    }
}

/// `border: 1px solid #ccc` on the sides `sides`.
fn border(s: &mut Style, v: &str, sides: &[usize]) {
    let (mut w, mut c, mut none) = (None, None, false);
    for p in parts(v) {
        match p {
            "none" | "hidden" | "0" => none = true,
            "thin" => w = Some(1.0),
            "medium" => w = Some(3.0),
            "thick" => w = Some(5.0),
            "solid" | "dashed" | "dotted" | "double" | "groove" | "ridge" | "inset" | "outset" => {}
            _ => {
                if let Some(Len::Px(x)) = parse_len(p) {
                    w = Some(x);
                } else if let Some(Len::Em(x)) = parse_len(p) {
                    w = Some(x * s.font_size);
                } else if let Some(col) = color(p, s) {
                    c = Some(col);
                }
            }
        }
    }
    for &i in sides {
        s.border[i] = if none { 0.0 } else { w.unwrap_or(3.0) };
        s.border_color[i] = c.unwrap_or(s.color);
    }
}

fn font_size(v: &str, parent: f32) -> Option<f32> {
    Some(match v {
        "xx-small" => 9.0,
        "x-small" => 10.0,
        "small" => 13.0,
        "medium" => 16.0,
        "large" => 18.0,
        "x-large" => 24.0,
        "xx-large" => 32.0,
        "xxx-large" => 48.0,
        "smaller" => parent / 1.2,
        "larger" => parent * 1.2,
        _ => match parse_len(v)? {
            Len::Px(x) => x,
            Len::Em(x) => x * parent,
            Len::Rem(x) => x * 16.0,
            Len::Pct(x) => x * parent / 100.0,
            _ => return None,
        },
    })
}

fn line_height(v: &str, font: f32) -> Option<f32> {
    if v == "normal" {
        return Some(1.2);
    }
    if let Ok(n) = v.parse::<f32>() {
        return Some(n);
    }
    Some(match parse_len(v)? {
        Len::Px(x) => x / font,
        Len::Em(x) => x,
        Len::Rem(x) => x * 16.0 / font,
        Len::Pct(x) => x / 100.0,
        _ => return None,
    })
}

/// A family list → a generic kind: monospace, serif or the system font.
fn family(v: &str) -> Option<&'static str> {
    for f in v.split(',') {
        let f = f.trim().trim_matches(['"', '\'']);
        if ["monospace", "courier", "courier new", "consolas", "menlo", "monaco", "sf mono", "ui-monospace", "lucida console", "dejavu sans mono", "source code pro", "fira code", "jetbrains mono"].contains(&f) {
            return Some("monospace");
        }
        if ["serif", "georgia", "times", "times new roman", "ui-serif", "palatino", "book antiqua", "garamond", "cambria", "pt serif", "noto serif"].contains(&f) {
            return Some("serif");
        }
        if ["sans-serif", "system-ui", "-apple-system", "blinkmacsystemfont", "ui-sans-serif", "arial", "helvetica", "helvetica neue", "verdana", "segoe ui", "roboto", "tahoma", "inter"].contains(&f) {
            return None;
        }
    }
    None
}

/// `font: italic bold 12px/1.5 Georgia, serif`.
fn font_shorthand(s: &mut Style, parent: &Style, v: &str) {
    let p = parts(v);
    let Some(i) = p.iter().position(|x| font_size(x.split('/').next().unwrap_or(""), parent.font_size).is_some() && !matches!(*x, "bold" | "normal")) else {
        return;
    };
    let (sz, lh) = p[i].split_once('/').map_or((p[i], None), |(a, b)| (a, Some(b)));
    s.font_size = font_size(sz, parent.font_size).unwrap_or(s.font_size);
    s.bold = false;
    s.italic = false;
    for x in &p[..i] {
        match *x {
            "bold" | "bolder" => s.bold = true,
            "italic" | "oblique" => s.italic = true,
            w if w.parse::<u32>().is_ok_and(|w| w >= 600) => s.bold = true,
            _ => {}
        }
    }
    if let Some(lh) = lh.and_then(|lh| line_height(lh, s.font_size)) {
        s.line_height = lh;
    }
    s.family = family(&p[i + 1..].join(" "));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn styled(html: &str) -> (Dom, Styled) {
        let dom = Dom::parse(html);
        let css: Vec<Sheet> = (0..dom.nodes.len())
            .filter(|&i| dom.nodes[i].tag() == Some("style"))
            .map(|i| Sheet::parse(&dom.text(i)))
            .collect();
        let st = compute(&dom, &Sheet::parse(UA), &css, Env { width: 800.0, height: 600.0, dark: false });
        (dom, st)
    }

    fn of<'a>(dom: &Dom, st: &'a Styled, tag: &str) -> &'a Style {
        st.styles[dom.find(tag).unwrap()].as_ref().unwrap()
    }

    #[test]
    fn ua_defaults_and_inheritance() {
        let (dom, st) = styled("<body><h1>T</h1><p>x <a href=/>link <b>b</b></a></p><ul><li>1</li></ul><pre>c</pre>");
        let h1 = of(&dom, &st, "h1");
        assert_eq!((h1.font_size, h1.bold, h1.display), (32.0, true, Display::Block));
        assert_eq!(h1.margin[0], Some(32.0 * 0.67));
        let b = of(&dom, &st, "b");
        assert_eq!((b.color, b.deco, b.bold), (Color::rgb(0, 0, 0xEE), Deco::Underline, true));
        assert_eq!(of(&dom, &st, "li").display, Display::ListItem);
        assert_eq!(of(&dom, &st, "ul").padding[3], 40.0);
        let pre = of(&dom, &st, "pre");
        assert_eq!((pre.family, pre.ws), (Some("monospace"), Ws::Pre));
        assert_eq!(of(&dom, &st, "body").margin, [Some(8.0); 4]);
    }

    #[test]
    fn specificity_order_important_and_inline() {
        let (dom, st) = styled(
            "<style>p { color: red } .a { color: green } #i { color: blue } p { color: orange !important; }
             div { color: purple } div { margin: 0 auto; padding: 1em 2px; border: 2px solid #333; border-left: none }
             span { font: italic bold 20px/30px Georgia, serif } @media (max-width: 500px) { div { color: lime } }</style>
             <p class=a id=i>x</p><div style='color: teal; background: linear-gradient(#fff, #000)'>y</div><span>z</span>",
        );
        assert_eq!(of(&dom, &st, "p").color, parse_color("orange").unwrap());
        let div = of(&dom, &st, "div");
        assert_eq!(div.color, parse_color("teal").unwrap());
        assert_eq!(div.background, Some(Color::WHITE));
        assert_eq!(div.margin, [Some(0.0), None, Some(0.0), None]);
        assert_eq!(div.padding, [16.0, 2.0, 16.0, 2.0]);
        assert_eq!(div.border, [2.0, 2.0, 2.0, 0.0]);
        let span = of(&dom, &st, "span");
        assert_eq!((span.font_size, span.bold, span.italic, span.line_height, span.family), (20.0, true, true, 1.5, Some("serif")));
    }

    #[test]
    fn css_variables_and_calc() {
        let (dom, st) = styled(
            "<style>:root { --c: #123456; --pad: 10px } div { color: var(--c); padding: calc(var(--pad) * 2) var(--none, 3px) }
             p { --c: red; color: var(--c) }</style><div>x<p>y</p></div>",
        );
        let div = of(&dom, &st, "div");
        assert_eq!(div.color, Color::rgb(0x12, 0x34, 0x56));
        assert_eq!(div.padding, [20.0, 3.0, 20.0, 3.0]);
        assert_eq!(of(&dom, &st, "p").color, Color::rgb(255, 0, 0));
    }

    #[test]
    fn hover_style_only_when_it_differs() {
        let (dom, st) = styled("<style>a:hover { color: red } p:hover { margin: 0 }</style><p><a href=x>l</a></p>");
        let a = dom.find("a").unwrap();
        assert_eq!(st.hover[a].as_ref().unwrap().color, Color::rgb(255, 0, 0));
        // Does hover change anything on p? (Margins already 0? No, 1em.) It does.
        assert!(st.hover[dom.find("p").unwrap()].is_some());
        assert!(st.hover[dom.find("body").unwrap_or(0)].is_none());
    }
}
