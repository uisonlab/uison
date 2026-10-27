//! Styled tree → uison screen JSON. Everything visible on the page is ordinary nodes:
//! a block is a `Column` (background and border — `Box`), inline content is a `Flow` of `Text` runs,
//! flex is `Row`/`Column`/`Wrap`, a list is a `Stack` with a marker, a table is a `Row` of cells.

use std::sync::atomic::{AtomicBool, Ordering};

use uison::Color;
use uison::ui::Json;

/// Experiment 1 (Task9): inline content as a `Wrap` of words instead of `Flow`.
pub static WORDS: AtomicBool = AtomicBool::new(false);

use crate::html::{Data, Dom};
use crate::style::{Deco, Display, ListStyle, Size, Style, Styled, Transform, Ws};

/// A page ready for display.
pub struct Page {
    pub root: Json,
    pub title: String,
    /// Link addresses as in the markup: the `key` of a link run is `a<number>.<n>`.
    pub links: Vec<String>,
    /// Image addresses: an `Image` has the source `mem:<prefix><number>`.
    pub images: Vec<String>,
    pub background: Option<Color>,
    /// Form fields: path in the data and initial value.
    pub fields: Vec<(String, Json)>,
    /// How many JSON nodes were produced.
    pub nodes: usize,
}

/// A JSON node: `{ Kind: { props } }`.
fn node(kind: &str, props: Vec<(&str, Json)>) -> Json {
    let mut p = Json::object();
    for (k, v) in props {
        p.set(k, v);
    }
    Json::Object(vec![(kind.to_string().into(), p)])
}

fn props_mut(n: &mut Json) -> &mut Json {
    &mut n.as_object_mut().unwrap()[0].1
}

fn kind_of(n: &Json) -> &str {
    n.as_object().and_then(|o| o.first()).map_or("", |(k, _)| k)
}

pub fn hex(c: Color) -> String {
    if c.a == 255 { format!("#{:02X}{:02X}{:02X}", c.r, c.g, c.b) } else { format!("#{:02X}{:02X}{:02X}{:02X}", c.r, c.g, c.b, c.a) }
}

fn num(v: f32) -> Json {
    Json::from((v as f64 * 100.0).round() / 100.0)
}

fn edges(v: [f32; 4]) -> Json {
    if v.iter().all(|&x| x == v[0]) { num(v[0]) } else { Json::Array(v.iter().map(|&x| num(x)).collect()) }
}

/// A finished block and its margins (vertical ones collapse in the parent).
struct Built {
    node: Json,
    margin: [Option<f32>; 4],
    /// An empty block without borders or background: margins only.
    empty: bool,
    /// `float`: 1 — left, 2 — right.
    float: u8,
    /// A heading or `clear`: a float above doesn't reach past it.
    brk: bool,
}

/// Inline content being gathered into one `Flow`.
struct Run {
    items: Vec<Json>,
    /// The last character is a space (or line start): the next leading space is redundant.
    space: bool,
    content: bool,
}

impl Default for Run {
    fn default() -> Self {
        Run { items: Vec::new(), space: true, content: false }
    }
}

/// Inline context: link, background of an inline ancestor, link style on hover.
#[derive(Clone, Copy, Default)]
struct Inline<'a> {
    link: Option<usize>,
    bg: Option<Color>,
    hover: Option<&'a Style>,
}

struct Conv<'a> {
    dom: &'a Dom,
    st: &'a Styled,
    page: Page,
    /// Keys of link runs: `a<number>.<n>`.
    link_parts: Vec<usize>,
    image_prefix: &'a str,
    /// A table with the `border` attribute: cells get a border.
    table_border: bool,
    /// Inside a shrink-to-content block: nested blocks don't stretch to the full
    /// available width either (in uison `fill` and `stretch` take all of it).
    hug: bool,
    /// Inline context outside the block: a link around an inline block (`<a class=btn>`).
    outer: Inline<'a>,
}

pub fn convert(dom: &Dom, st: &Styled, image_prefix: &str) -> Page {
    let title = dom.find("title").map(|t| collapse_ws(&dom.text(t)).trim().to_string()).unwrap_or_default();
    let page = Page { root: Json::Null, title, links: Vec::new(), images: Vec::new(), background: st.page_bg, fields: Vec::new(), nodes: 0 };
    let mut cv = Conv { dom, st, page, link_parts: Vec::new(), image_prefix, table_border: false, hug: false, outer: Inline::default() };
    let top = dom.find("html").unwrap_or(0);
    let root_style = Style::root();
    let mut items = match cv.style(top) {
        Some(_) => cv.block(top).into_iter().collect(),
        None => cv.children_blocks(top, &root_style),
    };
    // The root's margins don't collapse with the window: top and bottom become column padding.
    let (children, mt, mb) = collapse(&mut items, true, true);
    let mut props = vec![("w", Json::from("fill")), ("cross", "stretch".into()), ("children", Json::Array(children))];
    if mt > 0.0 || mb > 0.0 {
        props.push(("padding", Json::Array(vec![num(mt), num(0.0), num(mb), num(0.0)])));
    }
    let mut root = node("Column", props);
    escape_text(&mut root);
    cv.page.root = root;
    cv.page.nodes = count(&cv.page.root);
    cv.page
}

/// Page text is data, not screen expressions: `$initial` in an article is not a parameter, `@user`
/// is not a binding, `{` and `}` are just braces. Done once over the finished tree, after runs are
/// merged and split into words.
fn escape_text(n: &mut Json) {
    match n {
        Json::Object(o) => {
            for (k, v) in o.iter_mut() {
                match v {
                    Json::String(s) if &**k == "text" || &**k == "placeholder" => *s = uison::ui::literal(s).into(),
                    _ => escape_text(v),
                }
            }
        }
        Json::Array(a) => a.iter_mut().for_each(escape_text),
        _ => {}
    }
}

fn count(n: &Json) -> usize {
    match n {
        Json::Object(o) => o.iter().map(|(_, v)| count(v)).sum::<usize>() + (o.len() == 1 && o[0].0.starts_with(|c: char| c.is_ascii_uppercase())) as usize,
        Json::Array(a) => a.iter().map(count).sum(),
        _ => 0,
    }
}

impl<'a> Conv<'a> {
    fn style(&self, id: usize) -> Option<&'a Style> {
        self.st.styles[id].as_ref()
    }

    fn tag(&self, id: usize) -> &'a str {
        self.dom.nodes[id].tag().unwrap_or("")
    }

    /// Whether it's block-level: such children break the inline flow.
    fn is_block(&self, id: usize) -> bool {
        let Some(s) = self.style(id) else { return false };
        if s.float != 0 && s.display != Display::None {
            return true;
        }
        match s.display {
            Display::Block | Display::Flex | Display::ListItem | Display::Table | Display::RowGroup | Display::Row | Display::Cell => true,
            Display::Inline => self.dom.nodes[id].children.iter().any(|&c| self.is_block(c)),
            Display::InlineBlock | Display::None => false,
        }
    }

    // ---------------------------------------------------------------- blocks

    fn block(&mut self, id: usize) -> Option<Built> {
        self.block_in(id, true)
    }

    /// `stretch` — the block takes the parent's full width (normal flow); otherwise it fits its content
    /// (a flex row item, an inline block).
    fn block_in(&mut self, id: usize, stretch: bool) -> Option<Built> {
        let s = self.style(id)?;
        let saved = self.hug;
        self.hug = s.width.is_none() && (!stretch || self.hug);
        // A block link (`<a style="display: flex">`): all of its content is the link.
        let outer = self.outer;
        if let Some(l) = self.link_of(id) {
            self.outer = Inline { link: Some(l), hover: self.hover_of(id).or(outer.hover), ..outer };
        }
        let b = self.block_body(id, s, !self.hug);
        (self.hug, self.outer) = (saved, outer);
        b
    }

    fn block_body(&mut self, id: usize, s: &'a Style, stretch: bool) -> Option<Built> {
        if s.display == Display::None || s.hidden {
            return None;
        }
        let tag = self.tag(id);
        let content = match tag {
            "img" => {
                let img = self.image(id, s);
                return Some(self.finish(img, s, id, false, stretch));
            }
            "hr" => {
                let c = s.border_color[0];
                let h = (s.border[0] + s.border[2]).max(1.0);
                let line = node("Box", vec![("w", "fill".into()), ("h", num(h)), ("fill", hex(c).into())]);
                return Some(Built { node: line, margin: s.margin, empty: false, float: 0, brk: false });
            }
            "input" | "button" | "select" | "textarea" => return self.control(id, s).map(|n| Built { node: n, margin: s.margin, empty: false, float: 0, brk: false }),
            _ => match s.display {
                Display::Flex => self.flex(id, s),
                // `display: table` without rows (used to center an image) — like a normal block.
                Display::Table if self.has_rows(id) => self.table(id, s),
                _ => {
                    let mut items = self.children_blocks(id, s);
                    let no_top = s.padding[0] == 0.0 && s.border[0] == 0.0;
                    let no_bottom = s.padding[2] == 0.0 && s.border[2] == 0.0 && s.height.is_none();
                    let (children, mt, mb) = collapse(&mut items, no_top, no_bottom);
                    let empty = children.is_empty();
                    let col = node("Column", vec![("cross", "stretch".into()), ("children", Json::Array(children))]);
                    let mut b = self.finish(col, s, id, empty, stretch);
                    // Margins of the first and last child pass through a block without padding.
                    b.margin[0] = Some(b.margin[0].unwrap_or(0.0).max(mt));
                    b.margin[2] = Some(b.margin[2].unwrap_or(0.0).max(mb));
                    if s.display == Display::ListItem && s.list != ListStyle::None {
                        b.node = self.marker(id, s, b.node);
                    }
                    return Some(b);
                }
            },
        };
        Some(self.finish(content, s, id, false, stretch))
    }

    /// A block's children: blocks as they are, inline content between them goes into a `Flow`.
    fn children_blocks(&mut self, id: usize, s: &Style) -> Vec<Built> {
        let mut out = Vec::new();
        let mut run = Run::default();
        let kids = self.dom.nodes[id].children.clone();
        let first_text = kids.first().copied();
        for c in kids {
            if self.is_block(c) {
                self.flush(&mut run, s, &mut out);
                let float = self.style(c).map_or(0, |cs| cs.float);
                if let Some(mut b) = self.block_in(c, float == 0) {
                    b.float = float;
                    b.brk = self.style(c).is_some_and(|cs| cs.clear) || matches!(self.tag(c), "h1" | "h2" | "h3" | "h4" | "h5" | "h6");
                    out.push(b);
                }
            } else {
                // HTML: a newline right after `<pre>` isn't shown.
                let skip_nl = self.tag(id) == "pre" && Some(c) == first_text;
                self.inline(c, s, &mut run, self.outer, skip_nl);
            }
        }
        self.flush(&mut run, s, &mut out);
        wrap_floats(out)
    }

    /// Turn the accumulated inline content into a `Flow` (a single run is just a `Text`).
    fn flush(&mut self, run: &mut Run, s: &Style, out: &mut Vec<Built>) {
        let r = std::mem::take(run);
        if !r.content {
            return;
        }
        let mut items = r.items;
        // A trailing space isn't needed.
        if let Some(last) = items.last_mut()
            && kind_of(last) == "Text"
            && let Some(Json::String(t)) = props_mut(last).get_mut("text")
            && t.ends_with(' ')
        {
            *t = t.trim_end_matches(' ').to_string().into();
        }
        let align = match s.align {
            a if a >= 0.75 => "right",
            a if a >= 0.25 => "center",
            _ => "left",
        };
        let lh = num(s.line_height);
        if WORDS.load(Ordering::Relaxed) {
            out.push(Built { node: words(items, s), margin: [Some(0.0); 4], empty: false, float: 0, brk: false });
            return;
        }
        let flow = if items.len() == 1 && kind_of(&items[0]) == "Text" && props_mut(&mut items[0]).get("on").is_none() {
            let mut t = items.pop().unwrap();
            let p = props_mut(&mut t);
            p.set("lineHeight", lh);
            if align != "left" {
                p.set("align", align);
            }
            t
        } else {
            let mut props = vec![("lineHeight", lh), ("children", Json::Array(items))];
            if align != "left" {
                props.push(("align", align.into()));
            }
            node("Flow", props)
        };
        out.push(Built { node: flow, margin: [Some(0.0); 4], empty: false, float: 0, brk: false });
    }

    /// Sizes, padding, background, border, hover.
    fn finish(&mut self, mut content: Json, s: &Style, id: usize, empty: bool, stretch: bool) -> Built {
        // Fit to content: `stretch` on the column would take all the available width.
        let hug = !stretch && s.width.is_none();
        if hug && kind_of(&content) == "Column" {
            props_mut(&mut content).set("cross", "start");
        }
        // An uneven border is drawn as full-width strips — on a fit-to-content block it is uniform.
        let uniform = (s.border.iter().all(|&w| w == s.border[0]) && s.border_color.iter().all(|&c| c == s.border_color[0]))
            || (hug && s.border.iter().all(|&w| w > 0.0));
        let has_border = s.border.iter().any(|&w| w > 0.0);
        let hover = self.st.hover[id].as_deref();
        let s = &if s.masked { Style { background: None, ..s.clone() } } else { s.clone() };
        let bg_changes = hover.is_some_and(|h| h.background != s.background);
        // An uneven border is drawn by strips (side_borders); it doesn't need a Box.
        let decorated = s.background.is_some() || (has_border && uniform) || s.radius > 0.0 || bg_changes || (s.clip && s.max_height.is_some());
        // A block without padding, borders or sizes and with one child is the child itself (fewer nodes).
        let plain = !decorated && !has_border && s.padding.iter().all(|&p| p == 0.0) && s.width.is_none() && s.height.is_none()
            && s.max_width.is_none() && s.min_width.is_none() && s.min_height.is_none();
        if plain && !hug && kind_of(&content) == "Column" {
            let p = props_mut(&mut content);
            if let Some(Json::Array(kids)) = p.get_mut("children")
                && kids.len() == 1
                && kids[0].as_object().is_some_and(|o| o.len() == 1)
                && props_mut(&mut kids[0]).get("margin").is_none()
            {
                content = kids.pop().unwrap();
            }
        }
        // Padding is inside the border; a uniform border is the Box stroke on the edge, its width goes into padding.
        let mut pad = s.padding;
        // A block with an aspect ratio: the "fill the block" content gives the height, no spacer needed.
        if s.pct_pad && self.dom.nodes[id].children.iter().any(|&c| self.style(c).is_some_and(|cs| cs.abs_fill)) {
            (pad[0], pad[2]) = (0.0, 0.0);
        }
        if uniform && has_border {
            for p in &mut pad {
                *p += s.border[0];
            }
        }
        if pad.iter().any(|&p| p > 0.0) {
            // Only containers have padding: an image or text goes into a Box.
            if !matches!(kind_of(&content), "Column" | "Row" | "Wrap" | "Flow" | "Box" | "Stack") {
                content = node("Box", vec![("child", content)]);
            }
            props_mut(&mut content).set("padding", edges(pad));
        }
        let mut out = content;
        if decorated {
            let mut props: Vec<(&str, Json)> = Vec::new();
            if let Some(bg) = s.background.or(bg_changes.then_some(Color::rgba(0, 0, 0, 0))) {
                let fill = match hover.and_then(|h| h.background).filter(|_| bg_changes) {
                    Some(h) => state(hex(bg).into(), hex(h).into()),
                    None => hex(bg).into(),
                };
                props.push(("fill", fill));
            }
            if has_border && uniform {
                props.push(("stroke", hex(s.border_color[0]).into()));
                props.push(("width", num(s.border[0])));
            }
            if s.radius > 0.0 {
                props.push(("radius", num(s.radius.min(999.0))));
            }
            if s.radius > 0.0 || s.clip {
                props.push(("clip", true.into()));
            }
            if bg_changes {
                props.push(("interactive", true.into()));
            }
            if stretch || s.width.is_some() {
                props_mut(&mut out).set("w", "fill");
            }
            props.push(("child", out));
            out = node("Box", props);
        }
        if has_border && !uniform {
            out = side_borders(out, s);
        }
        let p = props_mut(&mut out);
        // `width` in CSS excludes padding and border (unless `box-sizing: border-box`).
        let extra = if s.border_box { 0.0 } else { s.padding[1] + s.padding[3] + s.border[1] + s.border[3] };
        match s.width {
            Some(Size::Px(w)) => p.set("w", num(w + extra)),
            Some(Size::Pct(f)) => p.set("w", format!("{}%", (f * 100.0).round())),
            None => {}
        }
        if let Some(Size::Px(h)) = s.height {
            let extra = if s.border_box { 0.0 } else { s.padding[0] + s.padding[2] + s.border[0] + s.border[2] };
            p.set("h", num(h + extra));
        }
        if let Some(Size::Px(w)) = s.max_width {
            p.set("maxW", num(w + extra));
        }
        if let Some(Size::Px(w)) = s.min_width {
            p.set("minW", num(w + extra));
        }
        if let Some(Size::Px(h)) = s.min_height {
            p.set("minH", num(h));
        }
        if let (true, Some(Size::Px(h))) = (s.clip, s.max_height) {
            p.set("maxH", num(h));
        }
        let empty = empty && !decorated && s.height.is_none() && s.min_height.is_none() && pad.iter().all(|&x| x == 0.0);
        Built { node: out, margin: s.margin, empty, float: 0, brk: false }
    }

    /// The list item marker goes on the left, in the list's margin, as in a browser.
    fn marker(&self, id: usize, s: &Style, content: Json) -> Json {
        let parent = self.dom.nodes[id].parent.unwrap_or(0);
        let start: i64 = self.dom.nodes[parent].attr("start").and_then(|v| v.parse().ok()).unwrap_or(1);
        let n = start
            + self.dom.nodes[parent].children.iter().take_while(|&&c| c != id).filter(|&&c| self.style(c).is_some_and(|cs| cs.display == Display::ListItem)).count() as i64;
        let text = match s.list {
            ListStyle::Disc => "•".to_string(),
            ListStyle::Circle => "◦".to_string(),
            ListStyle::Square => "▪".to_string(),
            ListStyle::Decimal => format!("{n}."),
            ListStyle::LowerAlpha => format!("{}.", alpha(n, b'a')),
            ListStyle::UpperAlpha => format!("{}.", alpha(n, b'A')),
            ListStyle::LowerRoman => format!("{}.", roman(n).to_lowercase()),
            ListStyle::UpperRoman => format!("{}.", roman(n)),
            ListStyle::None => String::new(),
        };
        let w = s.font_size * 2.5;
        let mut m = text_props(&text, s, None);
        m.extend([("w", num(w)), ("align", "right".into()), ("lineHeight", num(s.line_height))]);
        m.push(("pos", Json::Array(vec![num(-w - s.font_size * 0.5), num(0.0)])));
        let mut content = content;
        if !self.hug {
            props_mut(&mut content).set("w", "fill");
        }
        node("Stack", vec![("children", Json::Array(vec![content, node("Text", m)]))])
    }

    fn flex(&mut self, id: usize, s: &Style) -> Json {
        // Row items fit their content; columns with `align-items: stretch` take the full width.
        let stretch = !s.row && !s.wrap && s.align_items == "stretch";
        let mut items = Vec::new();
        let mut run = Run::default();
        for c in self.dom.nodes[id].children.clone() {
            let el = self.style(c);
            if el.is_some() && !matches!(self.tag(c), "br") {
                self.flush(&mut run, s, &mut items);
                // A child of a flex container is always a block.
                if let Some(b) = self.block_any(c, stretch) {
                    items.push(b);
                }
            } else {
                self.inline(c, s, &mut run, self.outer, false);
            }
        }
        self.flush(&mut run, s, &mut items);
        let children: Vec<Json> = items
            .into_iter()
            .map(|mut b| {
                apply_margin(&mut b.node, b.margin.map(|m| m.unwrap_or(0.0)));
                b.node
            })
            .collect();
        if s.wrap {
            return node("Wrap", vec![("gap", num(s.gap)), ("runGap", num(s.row_gap)), ("children", Json::Array(children))]);
        }
        let (kind, gap) = if s.row { ("Row", s.gap) } else { ("Column", s.row_gap.max(s.gap)) };
        node(kind, vec![("gap", num(gap)), ("main", s.justify.into()), ("cross", s.align_items.into()), ("children", Json::Array(children))])
    }

    /// An element as a block even if it's inline (a flex container child, a cell).
    fn block_any(&mut self, id: usize, stretch: bool) -> Option<Built> {
        let s = self.style(id)?;
        // A growing item gets its width from the row — its content fills it.
        let stretch = stretch || s.grow > 0.0;
        if s.display == Display::None {
            return None;
        }
        if self.is_block(id) || matches!(self.tag(id), "img" | "input" | "button" | "select" | "textarea") {
            let mut b = self.block_in(id, stretch)?;
            if s.grow > 0.0 {
                props_mut(&mut b.node).set("flex", num(s.grow));
            }
            return Some(b);
        }
        let mut items = Vec::new();
        let mut run = Run::default();
        self.inline_children(id, s, &mut run, self.outer);
        self.flush(&mut run, s, &mut items);
        let (children, _, _) = collapse(&mut items, false, false);
        let col = node("Column", vec![("cross", "stretch".into()), ("children", Json::Array(children))]);
        let mut b = self.finish(col, s, id, false, stretch);
        if s.grow > 0.0 {
            props_mut(&mut b.node).set("flex", num(s.grow));
        }
        Some(b)
    }

    /// Visible text of the subtree (without hidden elements: `<svg><title>`, screen-reader labels).
    fn visible_text(&self, id: usize) -> String {
        match &self.dom.nodes[id].data {
            Data::Text(t) => t.clone(),
            _ if self.style(id).is_some_and(|s| s.display == Display::None) => String::new(),
            _ => self.dom.nodes[id].children.iter().map(|&c| self.visible_text(c)).collect(),
        }
    }

    /// Whether the table has rows (directly or in thead/tbody/tfoot).
    fn has_rows(&self, id: usize) -> bool {
        self.dom.nodes[id].children.iter().any(|&c| match self.style(c).map(|cs| cs.display) {
            Some(Display::Row) => true,
            Some(Display::RowGroup) => self.has_rows(c),
            _ => false,
        })
    }

    fn table(&mut self, id: usize, s: &Style) -> Json {
        let saved = self.table_border;
        self.table_border = self.dom.nodes[id].attr("border").is_some_and(|b| b != "0");
        let pad: Option<f32> = self.dom.nodes[id].attr("cellpadding").and_then(|v| v.parse().ok());
        let gap: f32 = self.dom.nodes[id].attr("cellspacing").and_then(|v| v.parse().ok()).unwrap_or(2.0);
        let mut rows = Vec::new();
        let mut caption = None;
        let mut stack = self.dom.nodes[id].children.clone();
        stack.reverse();
        while let Some(c) = stack.pop() {
            match self.style(c).map(|cs| cs.display) {
                Some(Display::RowGroup) => stack.extend(self.dom.nodes[c].children.iter().rev()),
                Some(Display::Row) => rows.push(c),
                Some(Display::None) | None => {}
                _ if self.tag(c) == "caption" => caption = self.block(c),
                _ => {}
            }
        }
        let mut out = Vec::new();
        if let Some(c) = caption {
            out.push(c.node);
        }
        for r in rows {
            let rs = self.style(r).unwrap();
            let mut cells = Vec::new();
            for c in self.dom.nodes[r].children.clone() {
                let Some(cs) = self.style(c) else { continue };
                if cs.display != Display::Cell {
                    continue;
                }
                let mut cs = cs.clone();
                if let Some(p) = pad {
                    cs.padding = [p; 4];
                }
                if self.table_border && cs.border.iter().all(|&w| w == 0.0) {
                    cs.border = [1.0; 4];
                    cs.border_color = [Color::rgb(0x80, 0x80, 0x80); 4];
                }
                if cs.background.is_none() {
                    cs.background = rs.background;
                }
                let mut items = self.children_blocks(c, &cs);
                let (children, _, _) = collapse(&mut items, false, false);
                let col = node("Column", vec![("cross", "stretch".into()), ("children", Json::Array(children))]);
                let mut b = self.finish(col, &cs, c, false, true);
                let span: f32 = self.dom.nodes[c].attr("colspan").and_then(|v| v.parse().ok()).unwrap_or(1.0);
                // A fit-to-content table (floating, inline): cells take their own width,
                // otherwise `flex` in a row without a width divides zero.
                if cs.width.is_none() && !self.hug {
                    props_mut(&mut b.node).set("flex", num(span));
                }
                cells.push(b.node);
            }
            out.push(node("Row", vec![("gap", num(gap)), ("cross", "stretch".into()), ("children", Json::Array(cells))]));
        }
        self.table_border = saved;
        let mut t = node("Column", vec![("gap", num(gap)), ("cross", "stretch".into()), ("children", Json::Array(out))]);
        if self.dom.nodes[id].attr("border").is_some_and(|b| b != "0") && s.border.iter().all(|&w| w == 0.0) {
            t = node("Box", vec![("stroke", "#808080".into()), ("width", num(1.0)), ("padding", num(gap)), ("child", t)]);
        }
        t
    }

    // ---------------------------------------------------------------- inline

    fn inline_children(&mut self, id: usize, s: &Style, run: &mut Run, cx: Inline<'a>) {
        for c in self.dom.nodes[id].children.clone() {
            self.inline(c, s, run, cx, false);
        }
    }

    /// Inline node `id` into the run `run`; `parent` is the parent's style (for text).
    fn inline(&mut self, id: usize, parent: &Style, run: &mut Run, cx: Inline<'a>, skip_nl: bool) {
        let n = &self.dom.nodes[id];
        match &n.data {
            Data::Text(t) => {
                let t = if skip_nl { t.strip_prefix('\n').unwrap_or(t) } else { t };
                self.text(t, parent, run, cx);
            }
            Data::Element { tag, .. } => {
                let Some(s) = self.style(id) else { return };
                if s.display == Display::None || s.hidden {
                    return;
                }
                match tag.as_str() {
                    "br" => {
                        run.items.push(node("Text", text_props("\n", s, cx.bg)));
                        (run.space, run.content) = (true, true);
                    }
                    "img" => {
                        let mut img = self.image(id, s);
                        self.link_props(&mut img, cx);
                        self.atom(run, img, s);
                    }
                    "input" | "button" | "select" | "textarea" => {
                        if let Some(c) = self.control(id, s) {
                            self.atom(run, c, s);
                        }
                    }
                    _ if s.display == Display::InlineBlock || (boxy(s) && self.dom.text(id).chars().count() <= 40) => {
                        let mut items = Vec::new();
                        let mut inner = Run::default();
                        let cx2 = Inline { link: self.link_of(id).or(cx.link), bg: None, hover: self.hover_of(id).or(cx.hover) };
                        if s.display == Display::InlineBlock {
                            let saved = std::mem::replace(&mut self.outer, cx2);
                            items = self.children_blocks(id, s);
                            self.outer = saved;
                        } else {
                            self.inline_children(id, s, &mut inner, cx2);
                            self.flush(&mut inner, s, &mut items);
                        }
                        let (children, _, _) = collapse(&mut items, false, false);
                        let col = node("Column", vec![("cross", "stretch".into()), ("children", Json::Array(children))]);
                        let b = self.finish(col, s, id, false, false);
                        let mut atom = b.node;
                        let m = b.margin.map(|m| m.unwrap_or(0.0));
                        // An inline block's baseline comes from its text: its bottom goes below the line's baseline.
                        let below = s.padding[2] + s.border[2] + s.font_size * (0.25 + (s.line_height - 1.0) / 2.0);
                        apply_margin(&mut atom, [0.0, m[1], -below, m[3]]);
                        self.atom(run, atom, s);
                    }
                    _ => {
                        let cx2 = Inline { link: self.link_of(id).or(cx.link), bg: s.background.or(cx.bg), hover: self.hover_of(id).or(cx.hover) };
                        self.inline_children(id, s, run, cx2);
                    }
                }
            }
            Data::Root => {}
        }
    }

    fn atom(&mut self, run: &mut Run, n: Json, _s: &Style) {
        run.items.push(n);
        (run.space, run.content) = (false, true);
    }

    fn link_of(&mut self, id: usize) -> Option<usize> {
        let n = &self.dom.nodes[id];
        if n.tag() != Some("a") {
            return None;
        }
        let href = n.attr("href")?;
        self.page.links.push(href.to_string());
        self.link_parts.push(0);
        Some(self.page.links.len() - 1)
    }

    fn hover_of(&self, id: usize) -> Option<&'a Style> {
        self.st.hover[id].as_deref()
    }

    /// A link run: a press → the `nav` action, `key` → the link number.
    fn link_props(&mut self, n: &mut Json, cx: Inline) {
        if let Some(l) = cx.link {
            let part = self.link_parts[l];
            self.link_parts[l] += 1;
            let p = props_mut(n);
            p.set("key", format!("a{l}.{part}"));
            p.set("on", node_on("click", "nav"));
            p.set("cursor", "pointer");
        }
    }

    fn text(&mut self, t: &str, s: &Style, run: &mut Run, cx: Inline) {
        let mut text = match s.ws {
            Ws::Pre => t.replace('\t', "    ").replace(' ', "\u{a0}"),
            Ws::PreLine => t.split('\n').map(collapse_ws).collect::<Vec<_>>().join("\n"),
            Ws::Normal | Ws::NoWrap => collapse_ws(t),
        };
        if s.ws != Ws::Pre && run.space {
            text = text.trim_start_matches(' ').to_string();
        }
        if s.ws == Ws::NoWrap {
            text = text.replace(' ', "\u{a0}");
        }
        if text.is_empty() {
            return;
        }
        text = transform(&text, s.transform);
        run.space = text.ends_with(' ') || text.ends_with('\n');
        run.content |= !text.trim().is_empty();
        let mut props = text_props(&text, s, cx.bg);
        if let Some(h) = cx.hover.filter(|_| cx.link.is_some()) {
            if h.color != s.color {
                set_prop(&mut props, "color", state(hex(s.color).into(), hex(h.color).into()));
            }
            if h.deco != s.deco {
                set_prop(&mut props, "decoration", state(deco(s.deco).into(), deco(h.deco).into()));
            }
        }
        let mut n = node("Text", props);
        self.link_props(&mut n, cx);
        run.items.push(n);
    }

    fn image(&mut self, id: usize, s: &Style) -> Json {
        let n = &self.dom.nodes[id];
        let src = n.attr("src").unwrap_or("").to_string();
        let i = match self.page.images.iter().position(|x| *x == src) {
            Some(i) => i,
            None => {
                self.page.images.push(src);
                self.page.images.len() - 1
            }
        };
        let mut props = vec![("src", Json::from(format!("mem:{}{i}", self.image_prefix)))];
        let attr = |k: &str| n.attr(k).and_then(|v| v.trim_end_matches("px").parse::<f32>().ok());
        // A fluid image (`max-width: 100%`): the size from attributes is only a limit,
        // otherwise it won't shrink to the width; the aspect ratio comes from the file itself.
        if matches!(s.max_width, Some(Size::Pct(_))) && s.width.is_none() {
            if let Some(w) = attr("width") {
                props.push(("maxW", num(w)));
            }
            if s.radius > 0.0 {
                props.push(("radius", num(s.radius.min(999.0))));
            }
            return node("Image", props);
        }
        match s.width {
            Some(Size::Px(w)) => props.push(("w", num(w))),
            Some(Size::Pct(f)) => props.push(("w", format!("{}%", (f * 100.0).round()).into())),
            None => {}
        }
        if let (None, Some(w)) = (s.width, attr("width")) {
            props.push(("w", num(w)));
        }
        match (s.height, attr("height")) {
            (Some(Size::Px(h)), _) | (None, Some(h)) => props.push(("h", num(h))),
            _ => {}
        }
        if let Some(Size::Px(w)) = s.max_width {
            props.push(("maxW", num(w)));
        }
        if s.radius > 0.0 {
            props.push(("radius", num(s.radius.min(999.0))));
        }
        node("Image", props)
    }

    /// Form elements are uison library elements.
    fn control(&mut self, id: usize, s: &Style) -> Option<Json> {
        let n = &self.dom.nodes[id];
        let tag = n.tag().unwrap_or("");
        let ty = n.attr("type").unwrap_or("text").to_ascii_lowercase();
        let path = format!("form.f{}", self.page.fields.len());
        let width = |dflt: f32| match s.width {
            Some(Size::Px(w)) => num(w),
            Some(Size::Pct(f)) => format!("{}%", (f * 100.0).round()).into(),
            None => num(n.attr("size").and_then(|v| v.parse::<f32>().ok()).map_or(dflt, |c| c * 8.0 + 16.0)),
        };
        Some(match (tag, ty.as_str()) {
            ("input", "hidden") => return None,
            ("input", "checkbox") | ("input", "radio") => {
                self.page.fields.push((path.clone(), n.attr("checked").is_some().into()));
                node("Checkbox", vec![("checked", format!("@{path}").into())])
            }
            ("input", "submit" | "button" | "reset") | ("button", _) => {
                let text = if tag == "button" { collapse_ws(&self.visible_text(id)).trim().to_string() } else { n.attr("value").unwrap_or("Submit").to_string() };
                // An icon button (we don't draw SVG) — an empty plate is pointless.
                if text.is_empty() {
                    return None;
                }
                node("Button", vec![("text", text.into()), ("variant", "secondary".into()), ("size", "sm".into())])
            }
            ("select", _) => {
                let first = self.dom.nodes[id].children.iter().find(|&&c| self.tag(c) == "option").map(|&c| self.dom.text(c)).unwrap_or_default();
                node("Button", vec![("text", format!("{} ▾", first.trim()).into()), ("variant", "outline".into()), ("size", "sm".into())])
            }
            _ => {
                let value = if tag == "textarea" { self.dom.text(id) } else { n.attr("value").unwrap_or("").to_string() };
                self.page.fields.push((path.clone(), value.into()));
                let mut p = vec![("value", Json::from(format!("@{path}"))), ("w", width(200.0))];
                if let Some(ph) = n.attr("placeholder") {
                    p.push(("placeholder", ph.to_string().into()));
                }
                if ty == "password" {
                    p.push(("password", true.into()));
                }
                node("TextField", p)
            }
        })
    }
}

/// Experiment: each word is its own `Text` in a `Wrap` (no baseline and no whole links).
fn words(items: Vec<Json>, s: &Style) -> Json {
    let mut out = Vec::new();
    for mut it in items {
        if kind_of(&it) != "Text" {
            out.push(it);
            continue;
        }
        let p = props_mut(&mut it).clone();
        let text = p.get("text").and_then(Json::as_str).unwrap_or("").to_string();
        let key = p.get("key").and_then(Json::as_str).map(str::to_string);
        for (n, w) in text.split([' ', '\n']).filter(|w| !w.is_empty()).enumerate() {
            let mut q = p.clone();
            q.set("text", w.to_string());
            if let Some(k) = &key {
                q.set("key", format!("{k}.w{n}"));
            }
            out.push(Json::Object(vec![("Text".into(), q)]));
        }
    }
    node("Wrap", vec![("gap", num(s.font_size * 0.28)), ("runGap", num(s.font_size * (s.line_height - 1.0))), ("children", Json::Array(out))])
}

/// `float`: the floating block and all blocks after it form a row: it on the side, the rest in a column next to it
/// (text goes beside the image but doesn't wrap under it).
fn wrap_floats(mut items: Vec<Built>) -> Vec<Built> {
    let Some(i) = items.iter().position(|b| b.float != 0) else { return items };
    let mut rest = items.split_off(i + 1);
    // Beside the float go the blocks up to a heading or `clear`; after that, full width again.
    let after = rest.iter().position(|b| b.brk).map_or(Vec::new(), |j| rest.split_off(j));
    let after = wrap_floats(after);
    let rest = wrap_floats(rest);
    let mut fl = items.pop().unwrap();
    let side = fl.float;
    let m = fl.margin.map(|m| m.unwrap_or(0.0));
    apply_margin(&mut fl.node, m);
    let mut rest = rest;
    let (kids, mt, mb) = collapse(&mut rest, true, true);
    let col = node("Column", vec![("cross", "stretch".into()), ("flex", 1.0.into()), ("padding", Json::Array(vec![num(mt), num(0.0), num(mb), num(0.0)])), ("children", Json::Array(kids))]);
    let children = if side == 1 { vec![fl.node, col] } else { vec![col, fl.node] };
    items.push(Built { node: node("Row", vec![("cross", "start".into()), ("children", Json::Array(children))]), margin: [Some(0.0); 4], empty: false, float: 0, brk: false });
    items.extend(after);
    items
}

/// Margins collapse: between siblings the larger of the two; the first and the last
/// (if `top`/`bottom`) go to the parent. Returns the nodes and the margins for the parent.
fn collapse(items: &mut [Built], top: bool, bottom: bool) -> (Vec<Json>, f32, f32) {
    let mut out: Vec<(Json, [f32; 4])> = Vec::new();
    let mut pending = 0.0f32;
    let mut out_top = 0.0;
    for b in items.iter_mut() {
        let m = b.margin.map(|m| m.unwrap_or(0.0));
        if b.empty {
            pending = pending.max(m[0]).max(m[2]);
            continue;
        }
        let mut t = pending.max(m[0]);
        if out.is_empty() && top {
            out_top = t;
            t = 0.0;
        }
        // auto on the left and right — centered; only on the left — to the right edge.
        let node = std::mem::replace(&mut b.node, Json::Null);
        // Otherwise an explicit width in a `stretch` column gets stretched: the block goes into a row.
        let node = match (b.margin[1], b.margin[3]) {
            (None, None) => center(node, "top"),
            (Some(_), None) => center(node, "topRight"),
            _ => center(node, "topLeft"),
        };
        out.push((node, [t, m[1], 0.0, m[3]]));
        pending = m[2];
    }
    let out_bottom = if bottom {
        pending
    } else {
        if let Some(last) = out.last_mut() {
            last.1[2] = pending;
        }
        0.0
    };
    let nodes = out
        .into_iter()
        .map(|(mut n, m)| {
            apply_margin(&mut n, m);
            n
        })
        .collect();
    (nodes, out_top, out_bottom)
}

/// A block with an explicit width goes into an `Align` spanning the parent: the column's `stretch` won't
/// stretch it, and percentages are computed from the parent's width (a `Row` wouldn't have one).
fn center(mut n: Json, align: &str) -> Json {
    let p = props_mut(&mut n);
    if p.get("w").is_none() && p.get("maxW").is_none() {
        return n;
    }
    if p.get("w").is_none() {
        p.set("w", "fill");
    }
    node("Align", vec![("align", align.to_string().into()), ("h", "hug".into()), ("child", n)])
}

fn apply_margin(n: &mut Json, m: [f32; 4]) {
    if m.iter().any(|&x| x != 0.0) {
        props_mut(n).set("margin", edges(m));
    }
}

/// Uneven borders (`border-left` of a quote, `border-bottom` of a heading) are drawn as strips.
fn side_borders(content: Json, s: &Style) -> Json {
    let bar = |w: f32, c: Color, vertical: bool| {
        let (k, other) = if vertical { ("w", "h") } else { ("h", "w") };
        node("Box", vec![(k, num(w)), (other, "fill".into()), ("fill", hex(c).into())])
    };
    let mut out = content;
    if s.border[1] > 0.0 || s.border[3] > 0.0 {
        props_mut(&mut out).set("flex", 1.0);
        let mut kids = Vec::new();
        if s.border[3] > 0.0 {
            kids.push(bar(s.border[3], s.border_color[3], true));
        }
        kids.push(out);
        if s.border[1] > 0.0 {
            kids.push(bar(s.border[1], s.border_color[1], true));
        }
        out = node("Row", vec![("cross", "stretch".into()), ("children", Json::Array(kids))]);
    }
    if s.border[0] > 0.0 || s.border[2] > 0.0 {
        let mut kids = Vec::new();
        if s.border[0] > 0.0 {
            kids.push(bar(s.border[0], s.border_color[0], false));
        }
        kids.push(out);
        if s.border[2] > 0.0 {
            kids.push(bar(s.border[2], s.border_color[2], false));
        }
        out = node("Column", vec![("cross", "stretch".into()), ("children", Json::Array(kids))]);
    }
    out
}

/// An inline element with a border or padding and background (`<kbd>`, a "button" link) is an atomic block.
fn boxy(s: &Style) -> bool {
    s.border.iter().any(|&w| w > 0.0) || (s.background.is_some() && s.padding.iter().any(|&p| p > 2.0)) || s.radius > 2.0
}

fn text_props(text: &str, s: &Style, bg: Option<Color>) -> Vec<(&'static str, Json)> {
    let mut p = vec![("text", Json::from(text.to_string())), ("size", num(s.font_size)), ("color", hex(s.color).into())];
    if s.bold {
        p.push(("bold", true.into()));
    }
    if s.italic {
        p.push(("italic", true.into()));
    }
    if let Some(f) = s.family {
        p.push(("font", f.into()));
    }
    if s.deco != Deco::None {
        p.push(("decoration", deco(s.deco).into()));
    }
    if let Some(bg) = bg {
        p.push(("fill", hex(bg).into()));
    }
    p
}

fn set_prop(p: &mut Vec<(&'static str, Json)>, k: &'static str, v: Json) {
    match p.iter_mut().find(|(n, _)| *n == k) {
        Some(x) => x.1 = v,
        None => p.push((k, v)),
    }
}

fn deco(d: Deco) -> &'static str {
    match d {
        Deco::None => "none",
        Deco::Underline => "underline",
        Deco::LineThrough => "lineThrough",
    }
}

/// `{ state: { normal, hover } }`.
fn state(normal: Json, hover: Json) -> Json {
    let mut t = Json::object();
    t.set("normal", normal);
    t.set("hover", hover);
    let mut s = Json::object();
    s.set("state", t);
    s
}

fn node_on(event: &str, action: &str) -> Json {
    let mut o = Json::object();
    o.set(event, action.to_string());
    o
}

/// Whitespace and newlines become one space.
pub fn collapse_ws(t: &str) -> String {
    let mut out = String::with_capacity(t.len());
    let mut space = false;
    for c in t.chars() {
        if c.is_ascii_whitespace() {
            if !space {
                out.push(' ');
            }
            space = true;
        } else {
            out.push(c);
            space = false;
        }
    }
    out
}

fn transform(t: &str, tr: Transform) -> String {
    match tr {
        Transform::None => t.to_string(),
        Transform::Upper => t.to_uppercase(),
        Transform::Lower => t.to_lowercase(),
        Transform::Capitalize => {
            let mut out = String::with_capacity(t.len());
            let mut start = true;
            for c in t.chars() {
                if start && c.is_alphabetic() {
                    out.extend(c.to_uppercase());
                } else {
                    out.push(c);
                }
                start = c.is_whitespace();
            }
            out
        }
    }
}

fn alpha(n: i64, base: u8) -> String {
    let mut n = n.max(1) - 1;
    let mut s = Vec::new();
    loop {
        s.push(base + (n % 26) as u8);
        if n < 26 {
            break;
        }
        n = n / 26 - 1;
    }
    s.reverse();
    String::from_utf8(s).unwrap()
}

fn roman(mut n: i64) -> String {
    const R: [(i64, &str); 13] =
        [(1000, "M"), (900, "CM"), (500, "D"), (400, "CD"), (100, "C"), (90, "XC"), (50, "L"), (40, "XL"), (10, "X"), (9, "IX"), (5, "V"), (4, "IV"), (1, "I")];
    let mut s = String::new();
    for (v, r) in R {
        while n >= v {
            s.push_str(r);
            n -= v;
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::css::Sheet;
    use crate::style::{Env, UA, compute};

    fn page(html: &str) -> Page {
        let dom = Dom::parse(html);
        let css: Vec<Sheet> = (0..dom.nodes.len()).filter(|&i| dom.nodes[i].tag() == Some("style")).map(|i| Sheet::parse(&dom.text(i))).collect();
        let st = compute(&dom, &Sheet::parse(UA), &css, Env { width: 800.0, height: 600.0, dark: false });
        convert(&dom, &st, "p")
    }

    fn json(html: &str) -> String {
        page(html).root.to_json()
    }

    #[test]
    fn paragraph_with_spans_and_links() {
        let p = page("<title> Hi  there </title><p>Hello <b>bold</b> and <a href='/x'>a <i>link</i></a>.</p>");
        assert_eq!(p.title, "Hi there");
        assert_eq!(p.links, ["/x"]);
        let j = p.root.to_json();
        assert!(j.contains(r##"{"Flow":{"lineHeight":1.2,"children":[{"Text":{"text":"Hello ","size":16,"color":"#000000"}}"##), "{j}");
        assert!(j.contains(r##""text":"bold","size":16,"color":"#000000","bold":true"##), "{j}");
        assert!(j.contains(r##""key":"a0.0","on":{"click":"nav"},"cursor":"pointer""##), "{j}");
        assert!(j.contains(r##""key":"a0.1""##), "{j}");
    }

    #[test]
    fn margins_collapse_and_auto_centers() {
        // body 8 + h1 21.44 → 21.44 at the top (h1's margin passes through body), between h1 and p — max(21.44, 16).
        let j = json("<body><h1>T</h1><p>x</p><div style='max-width: 300px; margin: 0 auto'>c</div>");
        assert!(j.contains(r##""padding":[21.44,0,8,0]"##), "{j}");
        assert!(j.contains(r##""margin":[21.44,0,0,0]"##), "{j}");
        assert!(j.contains(r##"{"Align":{"align":"top","h":"hug","child":{"Column":{"##), "{j}");
        assert!(j.contains(r##""maxW":300"##), "{j}");
    }

    #[test]
    fn lists_tables_flex_and_boxes() {
        let j = json("<ol start=3><li>a<li>b</ol>");
        assert!(j.contains(r##""text":"3.""##) && j.contains(r##""text":"4.""##), "{j}");
        let j = json("<table border=1><tr><td>1<td>2</table>");
        assert!(j.contains(r##""stroke":"#808080""##), "{j}");
        let j = json("<div style='display:flex; gap: 10px; justify-content: space-between'><div style='flex: 1'>a</div><span>b</span></div>");
        assert!(j.contains(r##"{"Row":{"gap":10,"main":"between","cross":"stretch""##), "{j}");
        assert!(j.contains(r##""flex":1"##), "{j}");
        let j = json("<div style='background: #eee; border: 1px solid red; padding: 4px; border-radius: 6px'>x</div>");
        assert!(j.contains(r##"{"Box":{"fill":"#EEEEEE","stroke":"#FF0000","width":1,"radius":6,"clip":true,"child":{"Column""##), "{j}");
        assert!(j.contains(r##""padding":5"##), "{j}");
    }

    #[test]
    fn whitespace_pre_and_hover() {
        let j = json("<p>  a \n  b  </p><pre>\n  x\n y</pre>");
        assert!(j.contains(r##""text":"a b""##), "{j}");
        assert!(j.contains("\"text\":\"\u{a0}\u{a0}x\\n\u{a0}y\""), "{j}");
        let j = json("<style>a:hover { color: red }</style><a href=x>l</a>");
        assert!(j.contains(r##""color":{"state":{"normal":"#0000EE","hover":"#FF0000"}}"##), "{j}");
    }
}
