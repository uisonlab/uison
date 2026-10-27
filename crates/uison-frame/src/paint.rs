use uison_core::{Canvas, Paint, Rect};

use crate::{Frame, Item, Shape};

/// Paint the part of the frame inside `dirty`: background, then every item
/// whose bounds intersect the region, in list order.
pub fn paint(frame: &Frame, c: &mut impl Canvas, dirty: Rect) {
    c.push_clip(dirty);
    c.rect(dirty, 0.0, Paint::fill(frame.background));
    paint_items(&frame.items, c, dirty);
    c.pop_clip();
}

fn paint_items(items: &[Item], c: &mut impl Canvas, dirty: Rect) {
    for it in items {
        if !it.bounds.intersects(&dirty) {
            continue;
        }
        match &it.shape {
            Shape::Rect { rect, radius, paint } => c.rect(*rect, *radius, *paint),
            Shape::Circle { center, radius, paint } => c.circle(*center, *radius, *paint),
            Shape::Tri { points, paint } => c.triangle(*points, *paint),
            Shape::Poly { points, paint } => c.poly(points, *paint),
            Shape::Line { a, b, width, color, round } => c.line(*a, *b, *width, *color, *round),
            Shape::Hit { .. } => {}
            Shape::Text { at, text, font, color } => c.text(*at, text, font, *color),
            Shape::Image { rect, src, radius, fit } => c.image(*rect, src, *radius, *fit),
            Shape::Clip { rect, items } => {
                c.push_clip(*rect);
                paint_items(items, c, dirty.intersect(rect));
                c.pop_clip();
            }
        }
    }
}
