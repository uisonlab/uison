//! Flutter-like layout: constraints go down, sizes go up,
//! the parent sets the child's position.

use uison_core::{Host, Point, Size};

use crate::el::*;
use crate::text::{WrapMode, wrap};

const INF: f32 = f32::INFINITY;

/// Allowed sizes. `max` may be infinite (scroll view, a row along its main axis).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cons {
    pub min_w: f32,
    pub max_w: f32,
    pub min_h: f32,
    pub max_h: f32,
}

impl Cons {
    pub fn tight(s: Size) -> Cons {
        Cons { min_w: s.w, max_w: s.w, min_h: s.h, max_h: s.h }
    }

    pub fn loose(max_w: f32, max_h: f32) -> Cons {
        Cons { min_w: 0.0, max_w, min_h: 0.0, max_h }
    }

    fn clamp(&self, s: Size) -> Size {
        Size::new(s.w.clamp(self.min_w, self.max_w.max(self.min_w)), s.h.clamp(self.min_h, self.max_h.max(self.min_h)))
    }

    fn deflate(&self, i: Insets) -> Cons {
        Cons {
            min_w: (self.min_w - i.h()).max(0.0),
            max_w: (self.max_w - i.h()).max(0.0),
            min_h: (self.min_h - i.v()).max(0.0),
            max_h: (self.max_h - i.v()).max(0.0),
        }
    }
}

pub struct Ctx<'a> {
    pub m: &'a dyn Host,
    pub wrap: WrapMode,
}

/// Lay out a node; returns the outer size (with margin). The frame size goes to `el.size`.
pub fn layout(el: &mut El, c: Cons, cx: &Ctx) -> Size {
    let (mg, pad) = (el.lay.margin, el.lay.padding);
    let c = own(&el.lay, c.deflate(mg));
    // Content is laid out inside the padding, children are shifted to its start.
    let content = layout_kind(el, c.deflate(pad), cx);
    if pad != Insets::default() {
        for ch in &mut el.children {
            ch.pos = Point::new(ch.pos.x + pad.l, ch.pos.y + pad.t);
        }
    }
    let s = c.clamp(Size::new(content.w + pad.h(), content.h + pad.v()));
    el.size = s;
    Size::new(s.w + mg.h(), s.h + mg.v())
}

/// Apply the node's own w/h/min/max.
fn own(lay: &Lay, mut c: Cons) -> Cons {
    // "fill" and flex share are no larger than its own maxW/maxH (like `width: 100%; max-width` in CSS).
    let axis = |d: Dim, min: &mut f32, max: &mut f32, own_max: f32| match d {
        Dim::Px(v) => {
            let v = v.clamp(*min, max.max(*min));
            (*min, *max) = (v, v);
        }
        Dim::Fill if max.is_finite() => *min = max.min(own_max).max(*min),
        Dim::Pct(p) if max.is_finite() => {
            let v = (*max * p).min(own_max).max(*min);
            (*min, *max) = (v, v);
        }
        _ => {}
    };
    axis(lay.w, &mut c.min_w, &mut c.max_w, lay.max_w);
    axis(lay.h, &mut c.min_h, &mut c.max_h, lay.max_h);
    c.min_w = c.min_w.max(lay.min_w).min(c.max_w);
    c.max_w = c.max_w.min(lay.max_w).max(c.min_w);
    c.min_h = c.min_h.max(lay.min_h).min(c.max_h);
    c.max_h = c.max_h.min(lay.max_h).max(c.min_h);
    c
}

fn place(child_outer: Size, area: Size, a: Align) -> Point {
    Point::new((area.w - child_outer.w) * a.x, (area.h - child_outer.h) * a.y)
}

fn layout_kind(el: &mut El, c: Cons, cx: &Ctx) -> Size {
    match &mut el.kind {
        Kind::Box { align, .. } => {
            let align = *align;
            let inner = Cons::loose(c.max_w, c.max_h);
            let mut content = Size::default();
            let mut sizes = Vec::with_capacity(el.children.len());
            for ch in &mut el.children {
                let s = layout(ch, inner, cx);
                content = Size::new(content.w.max(s.w), content.h.max(s.h));
                sizes.push(s);
            }
            let size = c.clamp(content);
            for (ch, s) in el.children.iter_mut().zip(sizes) {
                ch.pos = place(s, size, align);
            }
            size
        }
        // Narrow screen: a row becomes a column, children take full width, flex has no effect (height follows content).
        Kind::Row { gap, stack_below, .. } if c.max_w < *stack_below => {
            let gap = *gap;
            flex(&mut el.children, &el.lay, c, false, gap, Main::Start, Cross::Stretch, false, cx)
        }
        Kind::Row { gap, main, cross, .. } => {
            let (gap, main, cross) = (*gap, *main, *cross);
            flex(&mut el.children, &el.lay, c, true, gap, main, cross, true, cx)
        }
        Kind::Column { gap, main, cross } => {
            let (gap, main, cross) = (*gap, *main, *cross);
            flex(&mut el.children, &el.lay, c, false, gap, main, cross, true, cx)
        }
        Kind::Stack => {
            let loose = Cons::loose(c.max_w, c.max_h);
            let mut size = Size::new(c.min_w, c.min_h);
            let mut sizes = Vec::with_capacity(el.children.len());
            for ch in &mut el.children {
                let s = layout(ch, loose, cx);
                if !ch.lay.overlay {
                    let p = ch.lay.pos.unwrap_or_default();
                    size = Size::new(size.w.max(p.x + s.w), size.h.max(p.y + s.h));
                }
                sizes.push(s);
            }
            let size = c.clamp(size);
            for (ch, s) in el.children.iter_mut().zip(sizes) {
                // A child spanning "the whole Stack" on an axis is laid out again at the Stack size
                // on that axis only (an overlay panel is wider but taller than its anchor field).
                let (fw, fh) = (ch.lay.w == Dim::Fill, ch.lay.h == Dim::Fill);
                let s = if fw || fh {
                    let max_w = if fw { size.w } else { c.max_w };
                    let max_h = if fh { size.h } else { c.max_h };
                    layout(ch, Cons::loose(max_w, max_h), cx)
                } else {
                    s
                };
                ch.pos = match (ch.lay.pos, ch.lay.place) {
                    (Some(p), _) => p,
                    (None, Some(a)) => place(s, size, a),
                    (None, None) => Point::default(),
                };
            }
            size
        }
        Kind::Wrap { gap, run_gap, center } => {
            let (gap, run_gap, center) = (*gap, *run_gap, *center);
            let max_w = c.max_w;
            let (mut x, mut y, mut run_h, mut w) = (0.0f32, 0.0f32, 0.0f32, 0.0f32);
            // Start of the current run (index) — for alignment by its height.
            let mut run_start = 0;
            let mut heights = Vec::with_capacity(el.children.len());
            let n = el.children.len();
            for i in 0..n {
                let ch = &mut el.children[i];
                let s = layout(ch, Cons::loose(max_w, INF), cx);
                if x > 0.0 && x + s.w > max_w {
                    if center {
                        center_run(&mut el.children[run_start..i], &heights[run_start..i], run_h);
                    }
                    y += run_h + run_gap;
                    (x, run_h, run_start) = (0.0, 0.0, i);
                }
                let ch = &mut el.children[i];
                ch.pos = Point::new(x, y);
                heights.push(s.h);
                x += s.w + gap;
                run_h = run_h.max(s.h);
                w = w.max(x - gap);
            }
            if center {
                center_run(&mut el.children[run_start..], &heights[run_start..], run_h);
            }
            Size::new(w, y + run_h)
        }
        Kind::Flow { align, line_height } => {
            let (align, lh) = (*align, *line_height);
            crate::flow::layout_flow(&mut el.children, c.max_w, align, lh, cx)
        }
        Kind::Spacer => Size::default(),
        Kind::Scroll { offset, content, over } => {
            let vw = if c.max_w.is_finite() { c.max_w } else { 0.0 };
            let mut ch_h = 0.0f32;
            let mut ch_w = 0.0f32;
            // The child is no shorter than the area: short content can be centered.
            let min_h = if c.max_h.is_finite() { c.max_h } else { 0.0 };
            for ch in &mut el.children {
                let w = if vw > 0.0 { vw } else { INF };
                let s = layout(ch, Cons { min_w: if vw > 0.0 { vw } else { 0.0 }, max_w: w, min_h, max_h: INF }, cx);
                ch_h = ch_h.max(s.h);
                ch_w = ch_w.max(s.w);
            }
            let size = c.clamp(Size::new(if vw > 0.0 { vw } else { ch_w }, if c.max_h.is_finite() { c.max_h } else { ch_h }));
            *content = ch_h;
            *offset = offset.clamp(0.0, (ch_h - size.h).max(0.0));
            for ch in &mut el.children {
                ch.pos = Point::new(0.0, -*offset - *over);
            }
            size
        }
        Kind::List { item_h, count, first, offset, over } => {
            let size = c.clamp(Size::new(
                if c.max_w.is_finite() { c.max_w } else { 0.0 },
                if c.max_h.is_finite() { c.max_h } else { *item_h * *count as f32 },
            ));
            let content = *item_h * *count as f32;
            *offset = offset.clamp(0.0, (content - size.h).max(0.0));
            for (i, ch) in el.children.iter_mut().enumerate() {
                layout(ch, Cons::tight(Size::new(size.w, *item_h)), cx);
                ch.pos = Point::new(0.0, (*first + i) as f32 * *item_h - *offset - *over);
            }
            size
        }
        Kind::Text(t) => {
            let tm = cx.m.measure("Ag", &t.font);
            t.glyph_h = tm.height();
            t.line_h = (tm.height() * t.line_height).ceil();
            t.ascent = tm.ascent;
            t.in_flow = false;
            let max_w = if t.wrap { c.max_w } else { INF };
            t.lines = wrap(&t.text, &t.font, max_w, t.max_lines, cx.wrap, cx.m);
            let widest = t.lines.iter().map(|l| l.width).fold(0.0f32, f32::max).ceil();
            let w = if t.align > 0.0 && c.max_w.is_finite() { c.max_w } else { widest };
            Size::new(w, t.line_h * t.lines.len() as f32)
        }
        Kind::Input(i) => {
            i.line_h = cx.m.measure("Ag", &i.font).height().ceil();
            Size::new(if c.max_w.is_finite() { c.max_w } else { 160.0 }, i.line_h)
        }
        Kind::Prim(p) => match p {
            Prim::Rect { .. } => Size::new(c.min_w, c.min_h),
            Prim::Circle { r, .. } => Size::new(2.0 * *r, 2.0 * *r),
            Prim::Line { from, to, .. } => Size::new(from.x.max(to.x), from.y.max(to.y)),
            Prim::Tri { points, .. } => Size::new(
                points.iter().map(|p| p.x).fold(0.0, f32::max),
                points.iter().map(|p| p.y).fold(0.0, f32::max),
            ),
        },
        // Custom drawing takes everything it is given (in Row/Column — with flex or w/h).
        Kind::Canvas { .. } => Size::new(
            if c.max_w.is_finite() { c.max_w } else { c.min_w },
            if c.max_h.is_finite() { c.max_h } else { c.min_h },
        ),
        // Vector children aren't laid out: their coordinates are in the drawing's system.
        Kind::Vector { view, .. } => Size::new(*view, *view),
        Kind::Image { src, .. } => {
            // Natural size; one side given — the other by aspect ratio; doesn't fit — scale down.
            let n = cx.m.image_size(src).filter(|n| n.w > 0.0 && n.h > 0.0).unwrap_or_default();
            if n.w == 0.0 {
                return Size::new(c.min_w, c.min_h);
            }
            let (fixed_w, fixed_h) = (c.min_w == c.max_w, c.min_h == c.max_h);
            match (fixed_w, fixed_h) {
                (true, false) => Size::new(c.max_w, c.max_w * n.h / n.w),
                (false, true) => Size::new(c.max_h * n.w / n.h, c.max_h),
                _ => {
                    let k = (c.max_w / n.w).min(c.max_h / n.h).min(1.0);
                    Size::new(n.w * k, n.h * k)
                }
            }
        }
    }
}

fn center_run(children: &mut [El], heights: &[f32], run_h: f32) {
    for (ch, h) in children.iter_mut().zip(heights) {
        ch.pos.y += (run_h - h) / 2.0;
    }
}

/// Row (`horizontal`) or Column; `use_flex = false` — children's flex and fill along the main axis have no effect.
#[allow(clippy::too_many_arguments)]
fn flex(children: &mut [El], lay: &Lay, c: Cons, horizontal: bool, gap: f32, main: Main, cross: Cross, use_flex: bool, cx: &Ctx) -> Size {
    // Axes: (main, cross).
    let (main_max, cross_max) = if horizontal { (c.max_w, c.max_h) } else { (c.max_h, c.max_w) };
    let cons = |main_min: f32, main_max: f32, cross_min: f32, cross_max: f32| {
        if horizontal {
            Cons { min_w: main_min, max_w: main_max, min_h: cross_min, max_h: cross_max }
        } else {
            Cons { min_w: cross_min, max_w: cross_max, min_h: main_min, max_h: main_max }
        }
    };
    let split = |s: Size| if horizontal { (s.w, s.h) } else { (s.h, s.w) };
    let stretch = cross == Cross::Stretch && cross_max.is_finite();
    let cross_min = if stretch { cross_max } else { 0.0 };
    // "fill" along the main axis is the same as flex: 1. In a row unbounded along the main axis there's nothing
    // to divide (zero free space) — such children are laid out by content.
    let flex_of = |ch: &El| {
        if !use_flex || !main_max.is_finite() {
            return None;
        }
        ch.lay.flex.or_else(|| {
            let d = if horizontal { ch.lay.w } else { ch.lay.h };
            (d == Dim::Fill).then_some(1.0)
        })
    };

    let mut sizes = vec![Size::default(); children.len()];
    let mut used = 0.0;
    let mut flex_total = 0.0;
    for (ch, s) in children.iter_mut().zip(&mut sizes) {
        match flex_of(ch) {
            Some(f) => flex_total += f,
            None => {
                *s = layout(ch, cons(0.0, INF, cross_min, cross_max), cx);
                used += split(*s).0;
            }
        }
    }
    let gaps = gap * children.len().saturating_sub(1) as f32;
    let free = if main_max.is_finite() { (main_max - used - gaps).max(0.0) } else { 0.0 };
    for (ch, s) in children.iter_mut().zip(&mut sizes) {
        if let Some(f) = flex_of(ch) {
            let share = if flex_total > 0.0 { free * f / flex_total } else { 0.0 };
            *s = layout(ch, cons(share, share, cross_min, cross_max), cx);
            used += split(*s).0;
        }
    }
    let own_main = if horizontal { lay.w } else { lay.h };
    let content = used + gaps;
    let main_size = if main_max.is_finite() && (flex_total > 0.0 || main != Main::Start || own_main == Dim::Fill) {
        main_max
    } else {
        content
    };
    let mut cross_size = if stretch { cross_max } else { sizes.iter().map(|s| split(*s).1).fold(0.0, f32::max) };
    // `stretch` without a set size along the cross axis — up to the largest child
    // (a row in a scroll view: equal-height columns, as in CSS).
    if cross == Cross::Stretch && !stretch {
        cross_size = cross_size.max(if horizontal { c.min_h } else { c.min_w });
        for (ch, s) in children.iter_mut().zip(&mut sizes) {
            let (m, x) = split(*s);
            if x < cross_size {
                *s = layout(ch, cons(m, m, cross_size, cross_size), cx);
            }
        }
    }
    let size = c.clamp(if horizontal { Size::new(main_size, cross_size) } else { Size::new(cross_size, main_size) });
    let (main_size, cross_size) = split(size);

    // Distributing free space along the main axis.
    let n = children.len().max(1) as f32;
    let rest = (main_size - content).max(0.0);
    let (mut at, extra) = match main {
        Main::Start => (0.0, 0.0),
        Main::Center => (rest / 2.0, 0.0),
        Main::End => (rest, 0.0),
        Main::Between if children.len() > 1 => (0.0, rest / (n - 1.0)),
        Main::Between => (0.0, 0.0),
        Main::Around => (rest / n / 2.0, rest / n),
        Main::Evenly => (rest / (n + 1.0), rest / (n + 1.0)),
    };
    for (ch, s) in children.iter_mut().zip(&sizes) {
        let (m, x) = split(*s);
        let off = match cross {
            Cross::Start | Cross::Stretch => 0.0,
            Cross::Center => (cross_size - x) / 2.0,
            Cross::End => cross_size - x,
        };
        ch.pos = if horizontal { Point::new(at, off) } else { Point::new(off, at) };
        at += m + gap + extra;
    }
    size
}
