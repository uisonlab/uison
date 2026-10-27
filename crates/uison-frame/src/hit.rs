use uison_core::{Cursor, Point};

use crate::{Frame, Item, Shape};

/// `id` of the topmost item under the point. Items without `id` are transparent to the mouse:
/// a label over a button doesn't block hitting the button.
pub fn hit_test(frame: &Frame, p: Point) -> Option<&str> {
    hit_items(&frame.items, p).and_then(|it| it.id.as_deref())
}

/// Cursor shape over the point: the cursor of the topmost `hit` area, otherwise the arrow.
pub fn cursor_at(frame: &Frame, p: Point) -> Cursor {
    match hit_items(&frame.items, p).map(|it| &it.shape) {
        Some(Shape::Hit { cursor, .. }) => *cursor,
        _ => Cursor::Arrow,
    }
}

/// All `hit` areas of the frame with their visible part (clips applied) — for debugging
/// and automation (click an element by id).
pub fn hit_areas(frame: &Frame) -> Vec<(&str, uison_core::Rect)> {
    fn walk<'f>(items: &'f [Item], clip: uison_core::Rect, out: &mut Vec<(&'f str, uison_core::Rect)>) {
        for it in items {
            match &it.shape {
                Shape::Hit { rect, .. } => {
                    let r = rect.intersect(&clip);
                    if let (Some(id), false) = (&it.id, r.is_empty()) {
                        out.push((id, r));
                    }
                }
                Shape::Clip { rect, items } => walk(items, clip.intersect(rect), out),
                _ => {}
            }
        }
    }
    let mut out = Vec::new();
    walk(&frame.items, frame.rect(), &mut out);
    out
}

/// Topmost item with an `id` under the point.
fn hit_items(items: &[Item], p: Point) -> Option<&Item> {
    for it in items.iter().rev() {
        if !it.bounds.contains(p) {
            continue;
        }
        if let Shape::Clip { items, .. } = &it.shape
            && let Some(hit) = hit_items(items, p)
        {
            return Some(hit);
        }
        if it.id.is_some() && contains(&it.shape, p) {
            return Some(it);
        }
    }
    None
}

fn contains(shape: &Shape, p: Point) -> bool {
    match shape {
        Shape::Rect { rect, .. } | Shape::Clip { rect, .. } | Shape::Hit { rect, .. } | Shape::Image { rect, .. } => rect.contains(p),
        Shape::Circle { center, radius, .. } => {
            let (dx, dy) = (p.x - center.x, p.y - center.y);
            dx * dx + dy * dy <= radius * radius
        }
        Shape::Tri { points: [a, b, c], .. } => {
            let s = |a: &Point, b: &Point| (b.x - a.x) * (p.y - a.y) - (b.y - a.y) * (p.x - a.x);
            let (d1, d2, d3) = (s(a, b), s(b, c), s(c, a));
            !((d1 < 0.0 || d2 < 0.0 || d3 < 0.0) && (d1 > 0.0 || d2 > 0.0 || d3 > 0.0))
        }
        // Parity of crossings of a ray to the right: matches the fill for simple shapes.
        Shape::Poly { points, .. } => {
            let mut inside = false;
            let mut j = points.len() - 1;
            for (i, a) in points.iter().enumerate() {
                let b = points[j];
                if (a.y > p.y) != (b.y > p.y) && p.x < (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x {
                    inside = !inside;
                }
                j = i;
            }
            inside
        }
        Shape::Line { a, b, width, .. } => {
            let (vx, vy) = (b.x - a.x, b.y - a.y);
            let len2 = vx * vx + vy * vy;
            let t = if len2 > 0.0 { (((p.x - a.x) * vx + (p.y - a.y) * vy) / len2).clamp(0.0, 1.0) } else { 0.0 };
            let (dx, dy) = (p.x - a.x - t * vx, p.y - a.y - t * vy);
            let tol = (width / 2.0).max(3.0);
            dx * dx + dy * dy <= tol * tol
        }
        // Text bounds are already computed from metrics — that's enough.
        Shape::Text { .. } => true,
    }
}
