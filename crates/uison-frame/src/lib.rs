//! The primitive frame: format, parsing, bounds, diff, hit-test, painting.

#[cfg(not(target_arch = "wasm32"))]
pub mod console;
mod decode;
mod diff;
pub mod event;
mod hit;
mod paint;
mod runtime;
pub mod script;
mod writer;

pub use diff::{Damage, diff};
pub use hit::{cursor_at, hit_areas, hit_test};
pub use paint::paint;
pub use runtime::{Runtime, Update};
pub use uison_json::Error;
pub use writer::{ItemWriter, Writer};

use uison_core::{Color, Cursor, Fit, Font, Paint, Point, Rect, Size, TextMeasure};

#[derive(Clone, Debug, PartialEq)]
pub struct Frame {
    pub size: Size,
    pub background: Color,
    pub items: Vec<Item>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Item {
    pub shape: Shape,
    pub id: Option<Box<str>>,
    /// The area the item may paint (including stroke and antialiasing).
    pub bounds: Rect,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Shape {
    Rect { rect: Rect, radius: f32, paint: Paint },
    /// `round` — rounded caps.
    Line { a: Point, b: Point, width: f32, color: Color, round: bool },
    Circle { center: Point, radius: f32, paint: Paint },
    Tri { points: [Point; 3], paint: Paint },
    /// Closed polygon (at least three points).
    Poly { points: Box<[Point]>, paint: Paint },
    Text { at: Point, text: Box<str>, font: Font, color: Color },
    Clip { rect: Rect, items: Vec<Item> },
    /// Image from source `src` (`mem:name` or a file), fitted into `rect`.
    Image { rect: Rect, src: Box<str>, radius: f32, fit: Fit },
    /// Invisible mouse area: only hit-test and cursor shape; not painted
    /// and doesn't affect damage. Makes the whole element (with padding) clickable.
    Hit { rect: Rect, cursor: Cursor },
}

/// Margin for edge antialiasing.
const AA: f32 = 1.0;

impl Frame {
    /// Parse a frame. Item bounds are computed right away (text via `m`).
    pub fn parse(src: &str, m: &dyn TextMeasure) -> Result<Frame, Error> {
        decode::parse(src, m)
    }

    pub fn rect(&self) -> Rect {
        Rect::from_size(self.size)
    }
}

impl Shape {
    /// The area a shape may paint, including stroke and antialiasing.
    fn bounds(&self, m: &dyn TextMeasure) -> Rect {
        match self {
            Shape::Rect { rect, paint, .. } => rect.inflate(paint.outset() + AA),
            // Round caps extend past the points by half the width — already covered.
            Shape::Line { a, b, width, .. } => Rect::bounding(&[*a, *b]).inflate(width / 2.0 + AA),
            Shape::Circle { center, radius, paint } => {
                Rect::new(center.x - radius, center.y - radius, 2.0 * radius, 2.0 * radius).inflate(paint.outset() + AA)
            }
            // A triangle's stroke is drawn with round joins,
            // so it extends past the outline by at most half the width.
            Shape::Tri { points, paint } => Rect::bounding(points).inflate(paint.outset() + AA),
            Shape::Poly { points, paint } => Rect::bounding(points).inflate(paint.outset() + AA),
            Shape::Text { at, text, font, .. } => {
                let tm = m.measure(text, font);
                // Glyphs may overhang the typographic bounds (italics, diacritics).
                let pad = (font.size * 0.25).max(AA);
                Rect::new(at.x, at.y, tm.width, tm.height()).inflate(pad)
            }
            Shape::Clip { rect, .. } | Shape::Hit { rect, .. } => *rect,
            Shape::Image { rect, .. } => rect.inflate(AA),
        }
    }
}

#[cfg(test)]
mod tests;
