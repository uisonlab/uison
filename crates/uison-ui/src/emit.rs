//! Laid-out tree → primitive frame (via `Writer`) + a map
//! of interactive nodes for event handling.

use uison_core::{Color, Cursor, Font, Point, Rect, Size, TextMeasure};
use uison_frame::Writer;

use crate::el::*;

/// An interactive node in the last frame.
#[derive(Clone, Debug)]
pub(crate) struct NodeInfo {
    pub key: String,
    /// Nearest interactive ancestor (index in the list).
    pub parent: Option<usize>,
    /// Rectangle on screen.
    pub rect: Rect,
    pub act: Act,
    pub input: Option<InputInfo>,
    pub scroll: Option<ScrollInfo>,
}

#[derive(Clone, Debug)]
pub(crate) struct InputInfo {
    pub path: String,
    /// Where the text starts on screen (with the horizontal shift).
    pub text_x: f32,
    pub font: Font,
    pub value: String,
    pub password: bool,
    pub keyboard: uison_core::Keyboard,
    /// Shift of a long text to the left in this frame.
    pub shift: f32,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct ScrollInfo {
    pub content: f32,
    pub viewport: f32,
}

pub(crate) struct Emitted {
    pub src: String,
    pub infos: Vec<NodeInfo>,
    pub prims: usize,
}

pub(crate) struct Style {
    pub background: Color,
    /// Default color of lines and text.
    pub ink: Color,
    pub scrollbar: Color,
    /// Touch screen: hit areas of pressable elements are at least [`MIN_TOUCH`].
    pub touch: bool,
}

/// Minimum touch target (pt), as in Apple's guidelines.
pub(crate) const MIN_TOUCH: f32 = 44.0;

/// Grow the rectangle to `min` on each axis around its center.
fn at_least(r: Rect, min: f32) -> Rect {
    let (dw, dh) = ((min - r.w).max(0.0) / 2.0, (min - r.h).max(0.0) / 2.0);
    Rect::new(r.x - dw, r.y - dh, r.w + 2.0 * dw, r.h + 2.0 * dh)
}

/// Custom drawing of a `Canvas` node: (`draw` function, node key, rectangle, frame).
pub(crate) type DrawFn<'a> = &'a mut dyn FnMut(&str, &str, Rect, &mut Writer);

/// `origin` is where the roots lie (top-left corner of the safe area); the background covers the whole `size`.
pub(crate) fn emit(roots: &[El], size: Size, origin: Point, style: &Style, m: &dyn TextMeasure, draw: DrawFn) -> Emitted {
    let mut em = Em {
        draw,
        w: Writer::new(size, style.background),
        infos: Vec::new(),
        parents: Vec::new(),
        overlays: Vec::new(),
        prims: 0,
        style,
        m,
        view: (f32::NEG_INFINITY, f32::INFINITY),
    };
    for el in roots {
        em.node(el, origin);
    }
    // Overlays go after the whole tree: above everything else and outside other clips.
    let mut i = 0;
    while i < em.overlays.len() {
        let (el, origin, parent) = em.overlays[i];
        let saved = std::mem::replace(&mut em.parents, parent.into_iter().collect());
        em.view = (f32::NEG_INFINITY, f32::INFINITY);
        em.node(el, origin);
        em.parents = saved;
        i += 1;
    }
    Emitted { src: em.w.finish(), infos: em.infos, prims: em.prims }
}

struct Em<'a> {
    w: Writer,
    draw: DrawFn<'a>,
    infos: Vec<NodeInfo>,
    parents: Vec<usize>,
    overlays: Vec<(&'a El, Point, Option<usize>)>,
    prims: usize,
    style: &'a Style,
    m: &'a dyn TextMeasure,
    /// Visible vertical band (nearest scroll view): anything entirely outside it isn't drawn.
    view: (f32, f32),
}

/// Margin of the visible band: what is just past the edge is drawn (shadows, protruding elements).
const VIEW_SLACK: f32 = 64.0;

impl<'a> Em<'a> {
    fn children(&mut self, el: &'a El, abs: Point) {
        for ch in &el.children {
            if ch.lay.overlay {
                self.overlays.push((ch, abs, self.parents.last().copied()));
            } else {
                self.node(ch, abs);
            }
        }
    }

    fn paint_rect(&mut self, r: Rect, p: &Paint, radius: f32) {
        if p.fill.is_none() && p.stroke.is_none() {
            return;
        }
        self.prims += 1;
        let mut it = self.w.rect(r);
        if let Some(f) = p.fill {
            it = it.fill(f);
        }
        if let Some(s) = p.stroke {
            it = it.stroke(s, p.width);
        }
        if radius > 0.0 {
            let _ = it.radius(radius);
        }
    }

    fn node(&mut self, el: &'a El, origin: Point) {
        let abs = Point::new(origin.x + el.pos.x + el.lay.margin.l, origin.y + el.pos.y + el.lay.margin.t);
        let rect = Rect::new(abs.x, abs.y, el.size.w, el.size.h);
        // Entirely outside the visible part of the scroll view: don't draw, but keep the node map
        // (focus, press, scroll-to-field look nodes up in it).
        if rect.bottom() < self.view.0 - VIEW_SLACK || rect.y > self.view.1 + VIEW_SLACK {
            self.ghost(el, abs);
            return;
        }
        let idx = el.act.as_ref().map(|act| {
            // A small element (checkbox, small button) is hard to hit with a finger — the area is wider than the drawing.
            let cursor = if act.disabled { Cursor::Arrow } else { act.cursor };
            match &el.kind {
                // A link inside Flow — an area per fragment (line), with a shared id.
                Kind::Text(t) if t.in_flow => {
                    for f in &t.frags {
                        self.w.hit(Rect::new(abs.x + f.x, abs.y + f.y, f.w, t.glyph_h), &el.key, cursor);
                    }
                }
                _ => {
                    let hit = if self.style.touch && act.pressable() { at_least(rect, MIN_TOUCH) } else { rect };
                    self.w.hit(hit, &el.key, cursor);
                }
            }
            self.infos.push(NodeInfo {
                key: el.key.clone(),
                parent: self.parents.last().copied(),
                rect,
                act: (**act).clone(),
                input: None,
                scroll: None,
            });
            let i = self.infos.len() - 1;
            self.parents.push(i);
            i
        });
        match &el.kind {
            Kind::Box { paint, radius, clip, .. } => {
                self.paint_rect(rect, paint, *radius);
                if *clip {
                    self.w.begin_clip(rect);
                }
                self.children(el, abs);
                if *clip {
                    self.w.end_clip();
                }
            }
            Kind::Row { .. } | Kind::Column { .. } | Kind::Stack | Kind::Wrap { .. } | Kind::Flow { .. } | Kind::Spacer => {
                self.children(el, abs)
            }
            Kind::Scroll { offset, content, .. } => self.scroll(el, abs, rect, *offset, *content, idx),
            Kind::List { offset, item_h, count, .. } => {
                self.scroll(el, abs, rect, *offset, *item_h * *count as f32, idx)
            }
            Kind::Text(t) if t.in_flow => {
                for f in &t.frags {
                    self.text_run(t, &f.text, Point::new((abs.x + f.x).round(), abs.y + f.y), f.w);
                }
            }
            Kind::Text(t) => {
                for (i, line) in t.lines.iter().enumerate() {
                    if line.text.is_empty() {
                        continue;
                    }
                    let x = (abs.x + (el.size.w - line.width) * t.align).round();
                    let y = abs.y + i as f32 * t.line_h + (t.line_h - t.glyph_h) / 2.0;
                    self.text_run(t, &line.text, Point::new(x, y), line.width);
                }
            }
            Kind::Input(i) => self.input(i, rect, idx),
            Kind::Image { src, fit, radius } => {
                if !src.is_empty() {
                    self.prims += 1;
                    let _ = self.w.image(rect, src).radius(*radius).fit(*fit);
                }
            }
            Kind::Prim(p) => self.prim(p, abs, el.size, 1.0, None),
            Kind::Canvas { draw } => {
                self.w.begin_clip(rect);
                (self.draw)(draw, &el.key, rect, &mut self.w);
                self.w.end_clip();
            }
            Kind::Vector { view, color, weight } => {
                let s = (el.size.w / view).min(el.size.h / view);
                for ch in &el.children {
                    if let Kind::Prim(p) = &ch.kind {
                        let pos = ch.lay.pos.unwrap_or_default();
                        let size = Size::new(px(ch.lay.w) * s, px(ch.lay.h) * s);
                        let at = Point::new(abs.x + pos.x * s, abs.y + pos.y * s);
                        self.prim(p, at, size, s, Some((*color, *weight)));
                    }
                }
            }
        }
        if idx.is_some() {
            self.parents.pop();
        }
    }

    /// Invisible subtree: only records of interactive nodes (no primitives and no `hit`).
    /// Text fields are emitted as is — there are few of them, and typing goes through their record.
    fn ghost(&mut self, el: &'a El, abs: Point) {
        let rect = Rect::new(abs.x, abs.y, el.size.w, el.size.h);
        let idx = el.act.as_ref().map(|act| {
            self.infos.push(NodeInfo {
                key: el.key.clone(),
                parent: self.parents.last().copied(),
                rect,
                act: (**act).clone(),
                input: None,
                scroll: None,
            });
            self.parents.push(self.infos.len() - 1);
            self.infos.len() - 1
        });
        match &el.kind {
            Kind::Input(i) => self.input(i, rect, idx),
            Kind::Scroll { content, .. } => {
                if let Some(i) = idx {
                    self.infos[i].scroll = Some(ScrollInfo { content: *content, viewport: rect.h });
                }
            }
            Kind::List { item_h, count, .. } => {
                if let Some(i) = idx {
                    self.infos[i].scroll = Some(ScrollInfo { content: *item_h * *count as f32, viewport: rect.h });
                }
            }
            _ => {}
        }
        for ch in &el.children {
            if ch.lay.overlay {
                self.overlays.push((ch, abs, self.parents.last().copied()));
            } else if !matches!(el.kind, Kind::Vector { .. }) {
                let a = Point::new(abs.x + ch.pos.x + ch.lay.margin.l, abs.y + ch.pos.y + ch.lay.margin.t);
                self.ghost(ch, a);
            }
        }
        if idx.is_some() {
            self.parents.pop();
        }
    }

    /// A line of `Text` with width `w`, with background and line (`fill`, `decoration`).
    fn text_run(&mut self, t: &TextEl, s: &str, at: Point, w: f32) {
        // Width of the whole line (with kerning), not the sum of words from the layout.
        let w = if t.fill.is_some() || t.deco != Deco::None { self.m.measure(s.trim_end(), &t.font).width.min(w) } else { w };
        if let Some(fill) = t.fill {
            self.prims += 1;
            let _ = self.w.rect(Rect::new(at.x, at.y, w.ceil(), t.glyph_h)).fill(fill).radius(2.0);
        }
        self.text(s, at, &t.font, t.color);
        let size = t.font.size;
        let y = match t.deco {
            Deco::None => return,
            Deco::Underline => at.y + t.ascent + (size / 10.0).max(1.0),
            Deco::LineThrough => at.y + t.ascent - size * 0.3,
        };
        self.prims += 1;
        let _ = self.w.line(Point::new(at.x, y), Point::new(at.x + w, y)).color(t.color).width((size / 15.0).max(1.0));
    }

    fn text(&mut self, s: &str, at: Point, font: &Font, color: Color) {
        self.prims += 1;
        let mut it = self.w.text(s, at).size(font.size).color(color);
        if font.bold {
            it = it.bold();
        }
        if font.italic {
            it = it.italic();
        }
        if let Some(f) = &font.family {
            let _ = it.font(f);
        }
    }

    fn scroll(&mut self, el: &'a El, abs: Point, rect: Rect, offset: f32, content: f32, idx: Option<usize>) {
        if let Some(i) = idx {
            self.infos[i].scroll = Some(ScrollInfo { content, viewport: rect.h });
        }
        self.w.begin_clip(rect);
        let saved = self.view;
        self.view = (rect.y.max(saved.0), rect.bottom().min(saved.1));
        self.children(el, abs);
        self.view = saved;
        if content > rect.h + 0.5 {
            let thumb = (rect.h * rect.h / content).max(24.0);
            let y = rect.y + (rect.h - thumb) * offset / (content - rect.h);
            self.prims += 1;
            self.w.rect(Rect::new(rect.right() - 7.0, y + 2.0, 4.0, thumb - 4.0)).fill(self.style.scrollbar).radius(2.0);
        }
        self.w.end_clip();
    }

    fn input(&mut self, i: &InputEl, rect: Rect, idx: Option<usize>) {
        let shown = i.shown();
        let (text, color) = if shown.is_empty() { (i.placeholder.as_str(), i.placeholder_color) } else { (shown.as_str(), i.color) };
        let x_of = |c: usize| self.m.measure(&shown[..i.shown_index(c)], &i.font).width;
        let caret_x = i.caret.map(x_of);
        // A long text shifts left just enough to keep the caret visible;
        // inside the visible part the caret moves without shifting, deleting from the end reduces the shift.
        let avail = rect.w - 2.0;
        let shift = caret_x.map_or(0.0, |x| {
            let s = if x - i.shift > avail { x - avail } else { i.shift.min(x) };
            s.min(x_of(i.value.len()) - avail).max(0.0)
        });
        let text_x = rect.x - shift;
        if let Some(n) = idx {
            self.infos[n].input =
                Some(InputInfo {
                    path: i.path.clone(),
                    text_x,
                    font: i.font.clone(),
                    value: i.value.clone(),
                    password: i.password,
                    keyboard: i.keyboard,
                    shift,
                });
        }
        self.w.begin_clip(rect.inflate(1.0));
        let y = rect.y + (rect.h - i.line_h) / 2.0;
        if let Some((a, b)) = i.sel {
            let (x0, x1) = ((text_x + x_of(a)).round(), (text_x + x_of(b)).round());
            self.prims += 1;
            self.w.rect(Rect::new(x0, y, x1 - x0, i.line_h)).fill(i.sel_color);
        }
        if !text.is_empty() {
            self.text(text, Point::new(text_x, y), &i.font, color);
        }
        if let Some(x) = caret_x.filter(|_| i.caret_on && i.sel.is_none()) {
            let x = (text_x + x).round() + 0.5;
            self.prims += 1;
            let _ = self.w.line(Point::new(x, rect.y + 1.0), Point::new(x, rect.bottom() - 1.0)).color(i.caret_color).width(1.5);
        }
        self.w.end_clip();
    }

    /// A primitive at point `at`; `s` is the coordinate scale (Vector), `vector` the default color and width.
    fn prim(&mut self, p: &Prim, at: Point, size: Size, s: f32, vector: Option<(Color, f32)>) {
        let ink = vector.map_or(self.style.ink, |v| v.0);
        // In a Vector a shape without fill and stroke is an outline in the icon color.
        let paint = |p: &Paint| match vector {
            Some((c, w)) if p.fill.is_none() && p.stroke.is_none() => Paint { fill: None, stroke: Some(c), width: w },
            _ => *p,
        };
        let tp = |q: Point| Point::new(at.x + q.x * s, at.y + q.y * s);
        match p {
            Prim::Rect { paint: pt, radius } => {
                self.paint_rect(Rect::new(at.x, at.y, size.w, size.h), &paint(pt), radius * s)
            }
            Prim::Circle { r, paint: pt } => {
                let pt = paint(pt);
                if pt.fill.is_none() && pt.stroke.is_none() {
                    return;
                }
                self.prims += 1;
                let mut it = self.w.circle(Point::new(at.x + r * s, at.y + r * s), r * s);
                if let Some(f) = pt.fill {
                    it = it.fill(f);
                }
                if let Some(c) = pt.stroke {
                    let _ = it.stroke(c, pt.width);
                }
            }
            Prim::Line { from, to, width, color, round } => {
                self.prims += 1;
                let w = width.unwrap_or(vector.map_or(1.0, |v| v.1));
                let it = self.w.line(tp(*from), tp(*to)).color(color.unwrap_or(ink)).width(w);
                if *round || vector.is_some() {
                    let _ = it.round();
                }
            }
            Prim::Tri { points, paint: pt } => {
                let pt = paint(pt);
                if pt.fill.is_none() && pt.stroke.is_none() {
                    return;
                }
                self.prims += 1;
                let mut it = self.w.tri(points.map(tp));
                if let Some(f) = pt.fill {
                    it = it.fill(f);
                }
                if let Some(c) = pt.stroke {
                    let _ = it.stroke(c, pt.width);
                }
            }
        }
    }
}

fn px(d: Dim) -> f32 {
    if let Dim::Px(v) = d { v } else { 0.0 }
}
