//! Sunburst chart: laying the tree out into rings of sectors, drawing into a frame, mouse hit-testing.
//!
//! Angles are in radians, `0` at 12 o'clock, clockwise. A sector is a `poly` polygon
//! (arcs in 1.5° steps); a full ring is a circle with a thick stroke.

use std::collections::HashMap;
use std::f32::consts::TAU;

use uison::{Color, Font, Point, Rect, Size, TextMeasure, Writer};

use crate::colors;
use crate::format;
use crate::i18n::Lang;
use crate::queue::TrashQueue;
use crate::tree::{NodeId, Tree};

/// At most this many rings around the center.
pub const MAX_RINGS: usize = 8;
/// Sectors narrower than this angle merge into one gray "small stuff".
const MIN_ANGLE: f64 = 0.004;
/// Hole in the center, as a fraction of the radius.
const HOLE: f32 = 0.2;
/// Each ring is this many times thinner than the previous one.
const RING_DECAY: f32 = 0.9;
/// Arc step.
const ARC_STEP: f32 = 1.5 * TAU / 360.0;
/// Gap between neighboring sectors (pt).
const GAP: f32 = 0.6;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Item {
    Node(NodeId),
    /// Several items of `parent`, too small to draw one by one.
    Small { parent: NodeId, count: u32, size: u64 },
}

impl Item {
    /// The node whose contents the sector represents.
    pub fn node(self) -> NodeId {
        match self {
            Item::Node(n) => n,
            Item::Small { parent, .. } => parent,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Sector {
    pub item: Item,
    pub a0: f32,
    pub a1: f32,
    pub color: Color,
}

pub struct Layout {
    pub root: NodeId,
    /// Root size: a sector's color is its share of this.
    total: u64,
    /// Sectors by ring, in increasing angle.
    pub rings: Vec<Vec<Sector>>,
    index: HashMap<NodeId, (usize, usize)>,
}

impl Layout {
    pub fn compute(tree: &Tree, root: NodeId) -> Self {
        let total = tree.node(root).size;
        let mut layout = Self { root, total, rings: Vec::new(), index: HashMap::new() };
        layout.place(tree, root, 0, 0.0, TAU as f64);
        layout
    }

    fn place(&mut self, tree: &Tree, node: NodeId, ring: usize, a0: f64, a1: f64) {
        let total = tree.node(node).size;
        if ring >= MAX_RINGS || total == 0 {
            return;
        }
        let scale = (a1 - a0) / total as f64;
        let mut a = a0;
        let (mut small_count, mut small_size) = (0u32, 0u64);
        for child in tree.sorted_children(node) {
            let n = tree.node(child);
            let da = n.size as f64 * scale;
            if da < MIN_ANGLE {
                if n.size > 0 {
                    small_count += 1;
                    small_size += n.size;
                }
                continue;
            }
            self.push(ring, Item::Node(child), a, a + da, n.size, n.is_dir());
            if n.is_dir() {
                self.place(tree, child, ring + 1, a, a + da);
            }
            a += da;
        }
        if small_count > 0 {
            let da = small_size as f64 * scale;
            let item = Item::Small { parent: node, count: small_count, size: small_size };
            self.push(ring, item, a, (a + da.max(MIN_ANGLE / 2.0)).min(a1), small_size, false);
        }
    }

    fn push(&mut self, ring: usize, item: Item, a0: f64, a1: f64, size: u64, is_dir: bool) {
        if self.rings.len() <= ring {
            self.rings.resize_with(ring + 1, Vec::new);
        }
        let color = match item {
            Item::Node(_) => colors::sector(size as f32 / self.total.max(1) as f32, ring as f32, is_dir),
            Item::Small { .. } => colors::SMALL,
        };
        if let Item::Node(n) = item {
            self.index.insert(n, (ring, self.rings[ring].len()));
        }
        self.rings[ring].push(Sector { item, a0: a0 as f32, a1: a1 as f32, color });
    }

    pub fn sector_of(&self, node: NodeId) -> Option<&Sector> {
        self.locate(node).map(|(_, s)| s)
    }

    fn locate(&self, node: NodeId) -> Option<(usize, &Sector)> {
        self.index.get(&node).map(|&(r, i)| (r, &self.rings[r][i]))
    }

    fn find(&self, ring: usize, angle: f32) -> Option<&Sector> {
        let sectors = self.rings.get(ring)?;
        let i = sectors.partition_point(|s| s.a1 <= angle);
        sectors.get(i).filter(|s| s.a0 <= angle)
    }
}

/// Ring geometry in a rectangle.
#[derive(Clone, Copy)]
struct Geometry {
    center: Point,
    hole: f32,
    /// Width of the inner ring; each next one is `RING_DECAY` times thinner.
    base: f32,
    rings: usize,
}

impl Geometry {
    fn new(rect: Rect, rings: usize) -> Self {
        let center = Point::new(rect.x + rect.w / 2.0, rect.y + rect.h / 2.0);
        let radius = (rect.w.min(rect.h) / 2.0 - 12.0).max(40.0);
        let hole = radius * HOLE;
        // Fewer rings means each is wider, but leave room for at least 4.
        let slots = rings.clamp(4, MAX_RINGS);
        let base = (radius - hole) * (1.0 - RING_DECAY) / (1.0 - RING_DECAY.powi(slots as i32));
        Self { center, hole, base, rings }
    }

    /// Inner radius of ring `x`. Fractional rings occur during animation, negative ones go into the hole.
    fn boundary(&self, x: f32) -> f32 {
        if x >= 0.0 {
            self.hole + self.base * (1.0 - RING_DECAY.powf(x)) / (1.0 - RING_DECAY)
        } else {
            (self.hole + self.base * x).max(0.0)
        }
    }

    fn radii(&self, ring: f32) -> (f32, f32) {
        (self.boundary(ring), self.boundary(ring + 1.0))
    }

    fn point(&self, r: f32, a: f32) -> Point {
        let (s, c) = a.sin_cos();
        Point::new(self.center.x + r * s, self.center.y - r * c)
    }

    /// Ring and angle under a point; ring `None` is the hole in the center.
    fn locate(&self, p: Point) -> Option<(Option<usize>, f32)> {
        let (dx, dy) = (p.x - self.center.x, p.y - self.center.y);
        let r = (dx * dx + dy * dy).sqrt();
        let angle = dx.atan2(-dy).rem_euclid(TAU);
        if r < self.hole {
            return Some((None, angle));
        }
        (0..self.rings)
            .find(|&i| {
                let (r0, r1) = self.radii(i as f32);
                r >= r0 && r < r1
            })
            .map(|ring| (Some(ring), angle))
    }
}

pub const ZOOM_SECONDS: f64 = 0.35;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Zoom {
    In,
    Out,
}

/// Changing the chart root: the part shared by both views flows into place, the rest fades.
pub struct Transition {
    old: Layout,
    zoom: Zoom,
    /// Angular span of the smaller view inside the larger one.
    span: (f32, f32),
    /// Rings between the children of the two roots.
    shift: f32,
    old_slots: usize,
    started: f64,
}

impl Transition {
    /// `None`: the views are unrelated or the target isn't visible in the other view.
    pub fn new(tree: &Tree, old: Layout, new: &Layout, now: f64) -> Option<Self> {
        if old.root == new.root {
            return None;
        }
        let (zoom, (ring, sector)) = if tree.is_within(new.root, old.root) {
            (Zoom::In, old.locate(new.root)?)
        } else if tree.is_within(old.root, new.root) {
            (Zoom::Out, new.locate(old.root)?)
        } else {
            return None;
        };
        let span = (sector.a0, sector.a1);
        let old_slots = old.rings.len();
        Some(Self { old, zoom, span, shift: ring as f32 + 1.0, old_slots, started: now })
    }

    /// Progress in `0..=1` with easing.
    fn progress(&self, now: f64) -> f32 {
        let t = ((now - self.started) / ZOOM_SECONDS).clamp(0.0, 1.0) as f32;
        1.0 - (1.0 - t).powi(3)
    }

    pub fn finished(&self, now: f64) -> bool {
        now - self.started >= ZOOM_SECONDS
    }

    /// Where a sector of the new view is drawn at progress `e`: (a0, a1, ring).
    fn morph(&self, s: &Sector, ring: usize, e: f32) -> (f32, f32, f32) {
        let (s0, s1) = self.span;
        let lerp = |a: f32, b: f32| a + (b - a) * e;
        match self.zoom {
            // Start where the new root's contents were in the old view.
            Zoom::In => {
                let from = |a: f32| s0 + a / TAU * (s1 - s0);
                (lerp(from(s.a0), s.a0), lerp(from(s.a1), s.a1), ring as f32 + self.shift * (1.0 - e))
            }
            // Start stretched so the old root's contents fill the whole circle.
            Zoom::Out => {
                let from = |a: f32| (a - s0) / (s1 - s0) * TAU;
                (lerp(from(s.a0), s.a0), lerp(from(s.a1), s.a1), ring as f32 - self.shift * (1.0 - e))
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hit {
    Center,
    Sector(Item),
}

/// What is under point `p` (coordinates of a chart node of size `size`), by the final layout,
/// so clicks work during the animation too.
pub fn hit(layout: &Layout, size: Size, p: Point) -> Option<Hit> {
    let geo = Geometry::new(Rect::from_size(size), layout.rings.len());
    match geo.locate(p)? {
        (None, _) => Some(Hit::Center),
        (Some(ring), angle) => layout.find(ring, angle).map(|s| Hit::Sector(s.item)),
    }
}

/// What the app wants highlighted.
pub struct Highlight<'a> {
    pub selected: Option<NodeId>,
    pub hovered: Option<Item>,
    /// What is under the cursor on the chart itself (the center means the "up" hint).
    pub hit: Option<Hit>,
    /// Cursor in node coordinates, for the tooltip.
    pub pointer: Option<Point>,
    /// Gray: will go to the Trash with the queue.
    pub queue: &'a TrashQueue,
    /// Language of the texts on the chart.
    pub lang: Lang,
}

/// Draw the chart in `rect` (window coordinates). `true`: an animation is running, the next frame is needed.
#[allow(clippy::too_many_arguments)]
pub fn draw(
    w: &mut Writer,
    m: &dyn TextMeasure,
    rect: Rect,
    tree: &Tree,
    layout: &Layout,
    transition: Option<&Transition>,
    hl: &Highlight,
    now: f64,
) -> bool {
    let target = Geometry::new(rect, layout.rings.len());
    let mut geo = target;
    let anim = transition.filter(|t| !t.finished(now)).map(|t| (t, t.progress(now)));
    if let Some((t, e)) = anim {
        let old = Geometry::new(rect, t.old_slots);
        geo.base = old.base + (target.base - old.base) * e;
    }

    // Color is the share of the view; while zooming the reference size moves smoothly from the old root to the new one.
    let reference = match anim {
        Some((t, e)) => {
            let (from, to) = ((t.old.total.max(1) as f32).ln(), (layout.total.max(1) as f32).ln());
            (from + (to - from) * e).exp()
        }
        None => layout.total.max(1) as f32,
    };

    // Zooming in: everything outside the new root stays in place and fades.
    if let Some((t, e)) = anim
        && t.zoom == Zoom::In
    {
        for (ring, sectors) in t.old.rings.iter().enumerate() {
            let (r0, r1) = geo.radii(ring as f32);
            for s in sectors.iter().filter(|s| !tree.is_within(s.item.node(), layout.root)) {
                sector(w, &geo, r0, r1, s.a0, s.a1, colors::alpha(s.color, 1.0 - e));
            }
        }
    }

    let mut selected_outline = None;
    for (ring, sectors) in layout.rings.iter().enumerate() {
        for s in sectors {
            let (a0, a1, ring_pos, mut color, fade) = match anim {
                Some((t, e)) if t.zoom == Zoom::In || tree.is_within(s.item.node(), t.old.root) => {
                    let (a0, a1, r) = t.morph(s, ring, e);
                    let color = match s.item {
                        Item::Node(n) => {
                            let node = tree.node(n);
                            colors::sector(node.size as f32 / reference, r.max(0.0), node.is_dir())
                        }
                        Item::Small { .. } => colors::SMALL,
                    };
                    (a0, a1, r, color, 1.0)
                }
                // Zooming out: the old root's surroundings appear in their place.
                Some((_, e)) => (s.a0, s.a1, ring as f32, s.color, e),
                None => (s.a0, s.a1, ring as f32, s.color, 1.0),
            };
            if hl.queue.covers(tree, s.item.node()) {
                color = colors::queued(color);
            }
            if let Some(sel) = hl.selected {
                if !tree.is_within(s.item.node(), sel) {
                    color = colors::dimmed(color);
                }
                if s.item == Item::Node(sel) {
                    selected_outline = Some((ring, *s));
                }
            }
            if hl.hovered == Some(s.item) {
                color = colors::hovered(color);
            }
            let (r0, r1) = geo.radii(ring_pos);
            sector(w, &geo, r0, r1, a0, a1, colors::alpha(color, fade));
        }
    }

    if let (Some((ring, s)), None) = (selected_outline, anim) {
        let (r0, r1) = geo.radii(ring as f32);
        outline(w, &geo, r0, r1, s.a0, s.a1);
    }

    let hint = match (hl.hit, hl.selected) {
        (Some(Hit::Center), Some(_)) => Some(hl.lang.deselect()),
        (Some(Hit::Center), None) if layout.root != Tree::ROOT => Some(hl.lang.up()),
        _ => None,
    };
    center(w, m, &geo, tree, layout.root, hint, hl.lang);
    legend(w, m, rect, hl.lang);
    if let (Some(item), Some(p)) = (hl.hovered.filter(|_| matches!(hl.hit, Some(Hit::Sector(_)))), hl.pointer) {
        tooltip(w, m, rect, Point::new(rect.x + p.x, rect.y + p.y), tree, item, hl.lang);
    }
    anim.is_some()
}

fn arc_segments(span: f32) -> usize {
    ((span / ARC_STEP).ceil() as usize).max(1)
}

/// A ring sector shrunk by the gap: neighbors are separated by a thin slit of background.
#[allow(clippy::too_many_arguments)]
fn sector(w: &mut Writer, geo: &Geometry, r0: f32, r1: f32, a0: f32, a1: f32, color: Color) {
    let span = a1 - a0;
    if span <= 0.0 || r1 - r0 < 2.0 * GAP + 0.5 || color.a == 0 {
        return;
    }
    let (r0, r1) = (r0 + GAP, r1 - GAP);
    // A full ring has no radial edges: a circle stroked at full thickness.
    if span >= TAU - 1e-4 {
        let _ = w.circle(geo.center, (r0 + r1) / 2.0).stroke(color, r1 - r0);
        return;
    }
    let inset = |r: f32| if span * r > 4.0 * GAP { GAP / r } else { 0.0 };
    let (i0, i1) = (inset(r0), inset(r1));
    let n = arc_segments(span);
    let at = |i: usize, inset: f32| a0 + inset + (span - 2.0 * inset) * i as f32 / n as f32;
    let mut pts = Vec::with_capacity(2 * (n + 1));
    pts.extend((0..=n).map(|i| geo.point(r1, at(i, i1))));
    if r0 > 0.5 {
        pts.extend((0..=n).rev().map(|i| geo.point(r0, at(i, i0))));
    } else {
        pts.push(geo.center);
    }
    let _ = w.poly(&pts).fill(color);
}

/// Outline of the selected sector.
fn outline(w: &mut Writer, geo: &Geometry, r0: f32, r1: f32, a0: f32, a1: f32) {
    if a1 - a0 >= TAU - 1e-4 {
        let _ = w.circle(geo.center, r1).stroke(colors::ACCENT, 2.0);
        let _ = w.circle(geo.center, r0).stroke(colors::ACCENT, 2.0);
        return;
    }
    let n = arc_segments(a1 - a0);
    let at = |i: usize| a0 + (a1 - a0) * i as f32 / n as f32;
    let mut pts: Vec<Point> = (0..=n).map(|i| geo.point(r1, at(i))).collect();
    pts.extend((0..=n).rev().map(|i| geo.point(r0, at(i))));
    let _ = w.poly(&pts).stroke(colors::ACCENT, 2.0);
}

fn font(size: f32) -> Font {
    Font { size, ..Font::default() }
}

/// Text centered at `c`.
fn text_centered(w: &mut Writer, m: &dyn TextMeasure, s: &str, c: Point, size: f32, color: Color) {
    let tm = m.measure(s, &font(size));
    let _ = w.text(s, Point::new(c.x - tm.width / 2.0, c.y - tm.height() / 2.0)).size(size).color(color);
}

/// View size in the center; `hint` is what a click on the center does (it's highlighted then).
fn center(w: &mut Writer, m: &dyn TextMeasure, geo: &Geometry, tree: &Tree, root: NodeId, hint: Option<&str>, lang: Lang) {
    let fill = if hint.is_some() { colors::PANEL } else { colors::BG };
    let _ = w.circle(geo.center, geo.hole - 1.0).fill(fill);
    let size = format::bytes(tree.node(root).size, lang);
    let (number, unit) = size.split_once(' ').unwrap_or((&size, ""));
    let big = (geo.hole * 0.42).clamp(12.0, 44.0);
    let c = geo.center;
    text_centered(w, m, number, Point::new(c.x, c.y - big * 0.45), big, colors::TEXT);
    text_centered(w, m, unit, Point::new(c.x, c.y + big * 0.6), big * 0.8, colors::TEXT);
    if let Some(hint) = hint {
        text_centered(w, m, hint, Point::new(c.x, c.y + big * 1.55), (big * 0.4).max(11.0), colors::TEXT_WEAK);
    }
}

/// Color scale in the bottom left corner.
fn legend(w: &mut Writer, m: &dyn TextMeasure, rect: Rect, lang: Lang) {
    const STEPS: usize = 24;
    let bar = Rect::new(rect.x + 16.0, rect.bottom() - 34.0, 180.0, 9.0);
    let f = font(13.0);
    let h = m.measure("Ag", &f).height();
    let _ = w.text(lang.legend(), Point::new(bar.x, bar.y - 5.0 - h)).size(13.0).color(colors::TEXT_WEAK);
    let (hi, lo) = (colors::SHARE_RED.log10(), colors::SHARE_VIOLET.log10());
    let step = bar.w / STEPS as f32;
    for i in 0..STEPS {
        let t = (i as f32 + 0.5) / STEPS as f32;
        let color = colors::sector(10f32.powf(hi + (lo - hi) * t), 0.0, true);
        // Overlap by a fraction of a point: no slits between the strips.
        let _ = w.rect(Rect::new(bar.x + step * i as f32, bar.y, step + 0.5, bar.h)).fill(color);
    }
    let y = bar.bottom() + 3.0;
    let _ = w.text("≥30%", Point::new(bar.x, y)).size(13.0).color(colors::TEXT_WEAK);
    let small = format!("≤0{}1%", lang.decimal());
    let sw = m.measure(&small, &f).width;
    let _ = w.text(&small, Point::new(bar.right() - sw, y)).size(13.0).color(colors::TEXT_WEAK);
}

/// Tooltip at the cursor: name, size and share, file count. Kept within the chart's edges.
fn tooltip(w: &mut Writer, m: &dyn TextMeasure, rect: Rect, at: Point, tree: &Tree, item: Item, lang: Lang) {
    let lines: Vec<(String, bool)> = match item {
        Item::Node(n) => {
            let node = tree.node(n);
            let parent = tree.parent(n).map_or(node.size, |p| tree.node(p).size);
            let mut v = vec![
                (format::ellipsize(&tree.display_name(n), 80), true),
                (lang.size_of_parent(node.size, &format::percent(node.size, parent, lang)), false),
            ];
            if node.is_dir() {
                v.push((lang.files(node.files.into()), false));
            }
            v
        }
        Item::Small { parent, count, size } => vec![
            (lang.smaller_objects(count.into()), true),
            (lang.size_in(size, &format::ellipsize(&tree.display_name(parent), 60)), false),
        ],
    };
    const PAD: f32 = 8.0;
    let f = font(13.0);
    let lh = m.measure("Ag", &f).height().ceil() + 3.0;
    let tw = lines.iter().map(|(s, b)| m.measure(s, &Font { bold: *b, ..f.clone() }).width).fold(0.0, f32::max);
    let (bw, bh) = (tw.ceil() + 2.0 * PAD, lh * lines.len() as f32 + 2.0 * PAD - 3.0);
    let x = (at.x + 14.0).min(rect.right() - bw - 4.0).max(rect.x + 4.0);
    let y = if at.y + 18.0 + bh > rect.bottom() - 4.0 { at.y - bh - 8.0 } else { at.y + 18.0 };
    let _ = w.rect(Rect::new(x, y, bw, bh)).fill(Color::rgb(0x1b, 0x1c, 0x20)).stroke(colors::BORDER, 1.0).radius(6.0);
    for (i, (s, bold)) in lines.iter().enumerate() {
        let it = w.text(s, Point::new(x + PAD, y + PAD + i as f32 * lh)).size(13.0).color(colors::TEXT);
        if *bold {
            let _ = it.bold();
        }
    }
}

/// Spinning arc of the scanning screen.
pub fn spinner(w: &mut Writer, rect: Rect, now: f64) {
    let geo = Geometry { center: Point::new(rect.x + rect.w / 2.0, rect.y + rect.h / 2.0), hole: 0.0, base: 0.0, rings: 0 };
    let r = rect.w.min(rect.h) / 2.0 - 2.0;
    let a0 = (now.rem_euclid(1.0) * TAU as f64) as f32;
    let _ = w.circle(geo.center, r - 2.5).stroke(colors::PANEL, 5.0);
    sector(w, &geo, r - 5.0 - GAP, r + GAP, a0, a0 + TAU * 0.3, colors::TEXT_WEAK);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::tests::{dir, file};
    use std::path::PathBuf;

    #[test]
    fn layout_covers_the_circle_in_order() {
        let t = crate::tree::tests::sample();
        let l = Layout::compute(&t, Tree::ROOT);
        assert_eq!(l.rings.len(), 2);
        assert_eq!(l.rings[0].len(), 3);
        assert_eq!(l.rings[1].len(), 2);
        assert!(l.rings[0][0].a0.abs() < 1e-6);
        assert!((l.rings[0].last().unwrap().a1 - TAU).abs() < 1e-4);
    }

    #[test]
    fn tiny_items_are_merged() {
        let mut children = vec![file("huge", 1_000_000)];
        children.extend((0..50).map(|i| file(&format!("tiny{i}"), 10)));
        let t = Tree::from_scan(PathBuf::from("/x"), dir("/x", children));
        let l = Layout::compute(&t, Tree::ROOT);
        assert_eq!(l.rings[0][1].item, Item::Small { parent: Tree::ROOT, count: 50, size: 500 });
    }

    #[test]
    fn hit_testing() {
        let t = crate::tree::tests::sample();
        let l = Layout::compute(&t, Tree::ROOT);
        let size = Size::new(400.0, 400.0);
        assert_eq!(hit(&l, size, Point::new(200.0, 200.0)), Some(Hit::Center));
        let geo = Geometry::new(Rect::from_size(size), l.rings.len());
        let (r0, r1) = geo.radii(0.0);
        let big = t.children(Tree::ROOT).next().unwrap();
        assert_eq!(hit(&l, size, geo.point((r0 + r1) / 2.0, 0.01)), Some(Hit::Sector(Item::Node(big))));
        assert_eq!(hit(&l, size, Point::new(0.0, 0.0)), None);
    }
}
