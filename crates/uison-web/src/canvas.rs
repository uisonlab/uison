//! `Canvas` on top of canvas 2D through imports. Axis-parallel strokes
//! are snapped to physical pixels (devicePixelRatio) — as in the macOS backend.

use uison_core::{Canvas, Color, Fit, Font, Paint, Point, Rect};

use crate::js;
use crate::text::WebText;

pub struct WebCanvas<'a> {
    text: &'a WebText,
    dpr: f32,
    /// Don't draw text (measuring the text share of a frame).
    pub skip_text: bool,
}

fn rgba(c: Color) -> u32 {
    (c.r as u32) << 24 | (c.g as u32) << 16 | (c.b as u32) << 8 | c.a as u32
}

impl<'a> WebCanvas<'a> {
    pub fn new(text: &'a WebText, dpr: f32) -> WebCanvas<'a> {
        WebCanvas { text, dpr, skip_text: false }
    }

    /// Highlight of the redrawn region (`?flash=1`).
    pub fn flash(&mut self, r: Rect, color: u32) {
        unsafe { js::fill_rect(r.x, r.y, r.w, r.h, 0.0, color) }
    }

    fn snap(&self, center: f32, width: f32) -> (f32, f32) {
        let s = self.dpr.max(1.0);
        let wd = (width * s).round().max(1.0);
        let cd = center * s;
        let cd = if wd % 2.0 == 1.0 { cd.floor() + 0.5 } else { cd.round() };
        (cd / s, wd / s)
    }

    fn snap_rect(&self, r: Rect, width: f32) -> (Rect, f32) {
        let (x, w) = self.snap(r.x, width);
        let (y, _) = self.snap(r.y, width);
        let (right, _) = self.snap(r.right(), width);
        let (bottom, _) = self.snap(r.bottom(), width);
        (Rect::new(x, y, right - x, bottom - y), w)
    }
}

impl Canvas for WebCanvas<'_> {
    fn rect(&mut self, r: Rect, radius: f32, paint: Paint) {
        unsafe {
            if let Some(f) = paint.fill {
                js::fill_rect(r.x, r.y, r.w, r.h, radius, rgba(f));
            }
            if let Some(s) = paint.stroke {
                let (r, w) = self.snap_rect(r, paint.width);
                js::stroke_rect(r.x, r.y, r.w, r.h, radius, w, rgba(s));
            }
        }
    }

    fn circle(&mut self, c: Point, r: f32, paint: Paint) {
        unsafe {
            if let Some(f) = paint.fill {
                js::fill_circle(c.x, c.y, r, rgba(f));
            }
            if let Some(s) = paint.stroke {
                js::stroke_circle(c.x, c.y, r, paint.width, rgba(s));
            }
        }
    }

    fn poly(&mut self, p: &[Point], paint: Paint) {
        if p.len() < 2 {
            return;
        }
        let xy: Vec<f32> = p.iter().flat_map(|q| [q.x, q.y]).collect();
        let (fill, stroke) = (paint.fill.map_or(0, rgba), paint.stroke.map_or(0, rgba));
        unsafe { js::poly(xy.as_ptr(), p.len(), fill, stroke, paint.width) }
    }

    fn line(&mut self, mut a: Point, mut b: Point, mut width: f32, color: Color, round: bool) {
        if a.y == b.y {
            let (y, w) = self.snap(a.y, width);
            (a.y, b.y, width) = (y, y, w);
        } else if a.x == b.x {
            let (x, w) = self.snap(a.x, width);
            (a.x, b.x, width) = (x, x, w);
        }
        unsafe { js::line(a.x, a.y, b.x, b.y, width, rgba(color), round as u32) }
    }

    fn text(&mut self, at: Point, text: &str, font: &Font, color: Color) {
        if self.skip_text {
            return;
        }
        let (id, m) = self.text.get(text, font);
        unsafe { js::text(text.as_ptr(), text.len(), at.x, at.y + m.ascent, id, rgba(color)) }
    }

    fn image(&mut self, r: Rect, src: &str, radius: f32, fit: Fit) {
        let f = match fit {
            Fit::Cover => 0,
            Fit::Contain => 1,
            Fit::Fill => 2,
        };
        unsafe {
            // `mem:` bytes go to JS once; the browser decodes them (createImageBitmap), asynchronously.
            if js::image(src.as_ptr(), src.len(), r.x, r.y, r.w, r.h, radius, f) == 0
                && let Some(b) = uison_core::image::bytes(src)
            {
                js::image_data(src.as_ptr(), src.len(), b.as_ptr(), b.len());
            }
        }
    }

    fn push_clip(&mut self, r: Rect) {
        unsafe { js::clip_push(r.x, r.y, r.w, r.h) }
    }

    fn pop_clip(&mut self) {
        unsafe { js::clip_pop() }
    }
}
