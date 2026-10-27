//! Fast generation of frame text from Rust (the app and the future compiler).
//!
//! ```ignore
//! let mut w = Writer::new(Size::new(400.0, 300.0), Color::WHITE);
//! w.rect(Rect::new(10.0, 10.0, 100.0, 30.0)).fill(blue).radius(6.0).id("ok");
//! w.text("OK", Point::new(20.0, 16.0)).color(Color::WHITE);
//! let src = w.finish();
//! ```
//! An item is closed when its `ItemWriter` is dropped (at the end of the expression).

use std::fmt::Write as _;

use uison_core::{Color, Cursor, Point, Rect, Size};

pub struct Writer {
    out: String,
    /// Whether a comma is needed before the next item (per clip nesting level).
    comma: Vec<bool>,
}

impl Writer {
    pub fn new(size: Size, background: Color) -> Writer {
        let mut w = Writer { out: String::with_capacity(4096), comma: vec![false] };
        w.out.push_str("{size:[");
        w.num(size.w);
        w.out.push(',');
        w.num(size.h);
        w.out.push_str("],background:");
        w.color(background);
        w.out.push_str(",items:[\n");
        w
    }

    pub fn finish(mut self) -> String {
        while self.comma.len() > 1 {
            self.end_clip();
        }
        self.out.push_str("]}\n");
        self.out
    }

    fn begin(&mut self, key: &str) {
        let c = self.comma.last_mut().unwrap();
        if *c {
            self.out.push_str(",\n");
        }
        *c = true;
        self.out.push('{');
        self.out.push_str(key);
        self.out.push(':');
    }

    fn nums(&mut self, v: &[f32]) {
        self.out.push('[');
        for (i, &x) in v.iter().enumerate() {
            if i > 0 {
                self.out.push(',');
            }
            self.num(x);
        }
        self.out.push(']');
    }

    /// Up to 3 decimal places (0.001 pt is plenty for UI). Our own writer
    /// rather than float formatting from std: that weighs ~16 KB.
    fn num(&mut self, v: f32) {
        num(&mut self.out, v);
    }

    fn color(&mut self, c: Color) {
        const HEX: &[u8; 16] = b"0123456789ABCDEF";
        self.out.push_str("\"#");
        let bytes = [c.r, c.g, c.b, c.a];
        let n = if c.a == 255 { 3 } else { 4 };
        for &b in &bytes[..n] {
            self.out.push(HEX[(b >> 4) as usize] as char);
            self.out.push(HEX[(b & 15) as usize] as char);
        }
        self.out.push('"');
    }

    pub fn rect(&mut self, r: Rect) -> ItemWriter<'_> {
        self.begin("rect");
        self.nums(&[r.x, r.y, r.w, r.h]);
        ItemWriter { w: self }
    }

    pub fn line(&mut self, a: Point, b: Point) -> ItemWriter<'_> {
        self.begin("line");
        self.nums(&[a.x, a.y, b.x, b.y]);
        ItemWriter { w: self }
    }

    pub fn circle(&mut self, c: Point, r: f32) -> ItemWriter<'_> {
        self.begin("circle");
        self.nums(&[c.x, c.y, r]);
        ItemWriter { w: self }
    }

    pub fn tri(&mut self, p: [Point; 3]) -> ItemWriter<'_> {
        self.begin("tri");
        self.nums(&[p[0].x, p[0].y, p[1].x, p[1].y, p[2].x, p[2].y]);
        ItemWriter { w: self }
    }

    /// Closed polygon: chart sectors, arrows, custom shapes.
    pub fn poly(&mut self, p: &[Point]) -> ItemWriter<'_> {
        self.begin("poly");
        self.out.push('[');
        for (i, q) in p.iter().enumerate() {
            if i > 0 {
                self.out.push(',');
            }
            self.num(q.x);
            self.out.push(',');
            self.num(q.y);
        }
        self.out.push(']');
        ItemWriter { w: self }
    }

    pub fn text(&mut self, text: &str, at: Point) -> ItemWriter<'_> {
        self.begin("text");
        uison_json::write_str(&mut self.out, text);
        self.out.push_str(",at:");
        self.nums(&[at.x, at.y]);
        ItemWriter { w: self }
    }

    /// Image `src` in a rectangle (`fit` defaults to cover).
    pub fn image(&mut self, r: Rect, src: &str) -> ItemWriter<'_> {
        self.begin("image");
        self.nums(&[r.x, r.y, r.w, r.h]);
        self.out.push_str(",src:");
        uison_json::write_str(&mut self.out, src);
        ItemWriter { w: self }
    }

    /// Invisible mouse area.
    pub fn hit(&mut self, r: Rect, id: &str, cursor: Cursor) {
        self.begin("hit");
        self.nums(&[r.x, r.y, r.w, r.h]);
        self.out.push_str(",id:");
        uison_json::write_str(&mut self.out, id);
        if cursor != Cursor::Arrow {
            self.out.push_str(",cursor:\"");
            self.out.push_str(cursor.name());
            self.out.push('"');
        }
        self.out.push('}');
    }

    pub fn begin_clip(&mut self, r: Rect) {
        self.begin("clip");
        self.nums(&[r.x, r.y, r.w, r.h]);
        self.out.push_str(",items:[\n");
        self.comma.push(false);
    }

    pub fn end_clip(&mut self) {
        assert!(self.comma.len() > 1, "end_clip without begin_clip");
        self.comma.pop();
        self.out.push_str("]}");
    }

    /// Another frame inside this one (a player showing a program's frame): its background and items
    /// scaled by `scale`, moved by `origin` and clipped to its size; `prefix` goes before the ids of its
    /// hit areas.
    pub fn frame(&mut self, f: &crate::Frame, origin: Point, scale: f32, prefix: &str) {
        let r = Rect::new(origin.x, origin.y, f.size.w * scale, f.size.h * scale);
        self.begin_clip(r);
        self.rect(r).fill(f.background);
        self.items(&f.items, origin, scale, prefix);
        self.end_clip();
    }

    fn items(&mut self, items: &[crate::Item], o: Point, s: f32, prefix: &str) {
        use crate::Shape;
        let at = |p: Point| Point::new(o.x + p.x * s, o.y + p.y * s);
        let moved = |r: Rect| Rect::new(o.x + r.x * s, o.y + r.y * s, r.w * s, r.h * s);
        for it in items {
            match &it.shape {
                Shape::Rect { rect, radius, paint: p } => {
                    let iw = paint(self.rect(moved(*rect)), p, s);
                    if *radius > 0.0 {
                        iw.radius(*radius * s);
                    }
                }
                Shape::Line { a, b, width, color, round } => {
                    let iw = self.line(at(*a), at(*b)).color(*color).width(*width * s);
                    if *round {
                        iw.round();
                    }
                }
                Shape::Circle { center, radius, paint: p } => {
                    paint(self.circle(at(*center), *radius * s), p, s);
                }
                Shape::Tri { points, paint: p } => {
                    paint(self.tri(points.map(at)), p, s);
                }
                Shape::Poly { points, paint: p } => {
                    let pts: Vec<Point> = points.iter().map(|&q| at(q)).collect();
                    paint(self.poly(&pts), p, s);
                }
                Shape::Text { at: p, text, font, color } => {
                    let mut iw = self.text(text, at(*p)).size(font.size * s).color(*color);
                    if let Some(f) = &font.family {
                        iw = iw.font(f);
                    }
                    if font.bold {
                        iw = iw.bold();
                    }
                    if font.italic {
                        iw.italic();
                    }
                }
                Shape::Image { rect, src, radius, fit } => {
                    let iw = self.image(moved(*rect), src).fit(*fit);
                    if *radius > 0.0 {
                        iw.radius(*radius * s);
                    }
                }
                Shape::Clip { rect, items } => {
                    self.begin_clip(moved(*rect));
                    self.items(items, o, s, prefix);
                    self.end_clip();
                }
                Shape::Hit { rect, cursor } => {
                    let mut id = String::from(prefix);
                    id.push_str(it.id.as_deref().unwrap_or(""));
                    self.hit(moved(*rect), &id, *cursor);
                }
            }
        }
    }
}

/// Optional item properties. Closes the item when dropped.
pub struct ItemWriter<'w> {
    w: &'w mut Writer,
}

impl ItemWriter<'_> {
    fn key(&mut self, k: &str) {
        self.w.out.push(',');
        self.w.out.push_str(k);
        self.w.out.push(':');
    }

    pub fn fill(mut self, c: Color) -> Self {
        self.key("fill");
        self.w.color(c);
        self
    }

    pub fn stroke(mut self, c: Color, width: f32) -> Self {
        self.key("stroke");
        self.w.color(c);
        if width != 1.0 {
            self.key("width");
            self.w.num(width);
        }
        self
    }

    /// Line or text color.
    pub fn color(mut self, c: Color) -> Self {
        self.key("color");
        self.w.color(c);
        self
    }

    /// Line width.
    pub fn width(mut self, v: f32) -> Self {
        self.key("width");
        self.w.num(v);
        self
    }

    /// Round line caps.
    pub fn round(mut self) -> Self {
        self.key("cap");
        self.w.out.push_str("\"round\"");
        self
    }

    pub fn radius(mut self, v: f32) -> Self {
        self.key("radius");
        self.w.num(v);
        self
    }

    /// Font size.
    pub fn size(mut self, v: f32) -> Self {
        self.key("size");
        self.w.num(v);
        self
    }

    pub fn font(mut self, family: &str) -> Self {
        self.key("font");
        uison_json::write_str(&mut self.w.out, family);
        self
    }

    pub fn bold(mut self) -> Self {
        self.key("bold");
        self.w.out.push_str("true");
        self
    }

    pub fn italic(mut self) -> Self {
        self.key("italic");
        self.w.out.push_str("true");
        self
    }

    pub fn fit(mut self, f: uison_core::Fit) -> Self {
        if f != uison_core::Fit::Cover {
            self.key("fit");
            self.w.out.push('"');
            self.w.out.push_str(f.name());
            self.w.out.push('"');
        }
        self
    }

    pub fn id(mut self, id: &str) -> Self {
        self.key("id");
        uison_json::write_str(&mut self.w.out, id);
        self
    }
}

/// Fill and stroke of an embedded item, the stroke scaled.
fn paint<'w>(mut iw: ItemWriter<'w>, p: &uison_core::Paint, scale: f32) -> ItemWriter<'w> {
    if let Some(c) = p.fill {
        iw = iw.fill(c);
    }
    if let Some(c) = p.stroke {
        iw = iw.stroke(c, p.width * scale);
    }
    iw
}

impl Drop for ItemWriter<'_> {
    fn drop(&mut self) {
        self.w.out.push('}');
    }
}

/// A number to 1/1000 without `f32` formatting (it weighs ~16 KB).
pub(crate) fn num(out: &mut String, v: f32) {
    let milli = (v as f64 * 1000.0).round() as i64; // NaN → 0, infinity saturates
    if milli < 0 {
        out.push('-');
    }
    let m = milli.unsigned_abs();
    let _ = write!(out, "{}", m / 1000);
    let frac = m % 1000;
    if frac != 0 {
        let digits = [frac / 100, frac / 10 % 10, frac % 10];
        let len = if digits[2] != 0 { 3 } else if digits[1] != 0 { 2 } else { 1 };
        out.push('.');
        for d in &digits[..len] {
            out.push((b'0' + *d as u8) as char);
        }
    }
}
