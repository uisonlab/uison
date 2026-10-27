//! `Canvas` over Cairo. The context is scaled to physical pixels (`cairo_scale`),
//! coordinates are in points; thin lines snap to pixels, as on Apple and Windows.

use std::f64::consts::PI;

use uison_core::{Canvas, Color, Fit, Font, Paint, Point, Rect};

use crate::gfx::*;
use crate::text::Text;

pub struct CairoCanvas<'a> {
    cr: *mut Cr,
    text: &'a Text,
    /// Physical pixels per point.
    scale: f32,
    snap: bool,
}

impl<'a> CairoCanvas<'a> {
    /// `cr` has no transforms yet: the canvas sets the scale (and removes it in `Drop`).
    pub fn new(cr: *mut Cr, text: &'a Text, scale: f32) -> Self {
        Self::at(cr, text, scale, Point::default())
    }

    /// Origin at `origin` (points; below the title bar of our own window frame).
    /// The pixel offset is rounded so line snapping stays aligned.
    pub fn at(cr: *mut Cr, text: &'a Text, scale: f32, origin: Point) -> Self {
        let c = &gfx().cairo;
        unsafe {
            c.cairo_save(cr);
            let (ox, oy) = ((origin.x * scale).round() as f64, (origin.y * scale).round() as f64);
            if ox != 0.0 || oy != 0.0 {
                c.cairo_translate(cr, ox, oy);
            }
            c.cairo_scale(cr, scale as f64, scale as f64);
            c.cairo_set_operator(cr, OPERATOR_OVER);
        }
        // UISON_SNAP=0: no pixel snapping (for comparison).
        let snap = std::env::var("UISON_SNAP").as_deref() != Ok("0");
        CairoCanvas { cr, text, scale, snap }
    }

    /// Snap an axis-parallel stroke: width is a whole number of physical
    /// pixels (at least one), edges on pixel boundaries.
    fn snap_stroke(&self, center: f32, width: f32) -> (f32, f32) {
        if !self.snap || self.scale <= 0.0 {
            return (center, width);
        }
        let s = self.scale;
        let wd = (width * s).round().max(1.0);
        let cd = center * s;
        let cd = if wd % 2.0 == 1.0 { cd.floor() + 0.5 } else { cd.round() };
        (cd / s, wd / s)
    }

    fn snap_rect(&self, r: Rect, width: f32) -> (Rect, f32) {
        let (x, w) = self.snap_stroke(r.x, width);
        let (y, _) = self.snap_stroke(r.y, width);
        let (right, _) = self.snap_stroke(r.right(), width);
        let (bottom, _) = self.snap_stroke(r.bottom(), width);
        (Rect::new(x, y, right - x, bottom - y), w)
    }

    fn color(&self, c: Color) {
        let f = |v: u8| v as f64 / 255.0;
        unsafe { gfx().cairo.cairo_set_source_rgba(self.cr, f(c.r), f(c.g), f(c.b), f(c.a)) };
    }

    fn rect_path(&self, r: Rect, radius: f32) {
        let c = &gfx().cairo;
        let (x, y, w, h) = (r.x as f64, r.y as f64, r.w as f64, r.h as f64);
        let rad = (radius as f64).min(w / 2.0).min(h / 2.0).max(0.0);
        unsafe {
            if rad <= 0.0 {
                c.cairo_rectangle(self.cr, x, y, w, h);
                return;
            }
            c.cairo_new_sub_path(self.cr);
            c.cairo_arc(self.cr, x + w - rad, y + rad, rad, -PI / 2.0, 0.0);
            c.cairo_arc(self.cr, x + w - rad, y + h - rad, rad, 0.0, PI / 2.0);
            c.cairo_arc(self.cr, x + rad, y + h - rad, rad, PI / 2.0, PI);
            c.cairo_arc(self.cr, x + rad, y + rad, rad, PI, 1.5 * PI);
            c.cairo_close_path(self.cr);
        }
    }

    /// Fill and/or stroke the path built by `path` (for the stroke the path is rebuilt
    /// if it needs snapping).
    fn fill_stroke(&self, paint: Paint, join: i32, path: impl Fn()) {
        let c = &gfx().cairo;
        unsafe {
            if let Some(fill) = paint.fill {
                self.color(fill);
                path();
                c.cairo_fill(self.cr);
            }
            if let Some(stroke) = paint.stroke {
                self.color(stroke);
                path();
                c.cairo_set_line_width(self.cr, paint.width as f64);
                c.cairo_set_line_join(self.cr, join);
                c.cairo_stroke(self.cr);
            }
        }
    }
}

impl Drop for CairoCanvas<'_> {
    fn drop(&mut self) {
        unsafe { gfx().cairo.cairo_restore(self.cr) };
    }
}

impl Canvas for CairoCanvas<'_> {
    fn rect(&mut self, r: Rect, radius: f32, paint: Paint) {
        let c = &gfx().cairo;
        if let Some(fill) = paint.fill {
            self.color(fill);
            self.rect_path(r, radius);
            unsafe { c.cairo_fill(self.cr) };
        }
        if let Some(stroke) = paint.stroke {
            let (r, width) = self.snap_rect(r, paint.width);
            self.color(stroke);
            self.rect_path(r, radius);
            unsafe {
                c.cairo_set_line_width(self.cr, width as f64);
                c.cairo_set_line_join(self.cr, LINE_JOIN_MITER);
                c.cairo_stroke(self.cr);
            }
        }
    }

    fn circle(&mut self, center: Point, r: f32, paint: Paint) {
        let c = &gfx().cairo;
        self.fill_stroke(paint, LINE_JOIN_MITER, || unsafe {
            c.cairo_new_sub_path(self.cr);
            c.cairo_arc(self.cr, center.x as f64, center.y as f64, r as f64, 0.0, 2.0 * PI);
        });
    }

    fn poly(&mut self, p: &[Point], paint: Paint) {
        if p.len() < 2 {
            return;
        }
        let c = &gfx().cairo;
        // Stroke joins are rounded, like Direct2D geometry and CoreGraphics in the other backends.
        self.fill_stroke(paint, LINE_JOIN_ROUND, || unsafe {
            c.cairo_move_to(self.cr, p[0].x as f64, p[0].y as f64);
            for q in &p[1..] {
                c.cairo_line_to(self.cr, q.x as f64, q.y as f64);
            }
            c.cairo_close_path(self.cr);
        });
    }

    fn line(&mut self, mut a: Point, mut b: Point, mut width: f32, color: Color, round: bool) {
        if a.y == b.y {
            let (y, w) = self.snap_stroke(a.y, width);
            (a.y, b.y, width) = (y, y, w);
        } else if a.x == b.x {
            let (x, w) = self.snap_stroke(a.x, width);
            (a.x, b.x, width) = (x, x, w);
        }
        let c = &gfx().cairo;
        self.color(color);
        unsafe {
            c.cairo_move_to(self.cr, a.x as f64, a.y as f64);
            c.cairo_line_to(self.cr, b.x as f64, b.y as f64);
            c.cairo_set_line_width(self.cr, width as f64);
            c.cairo_set_line_cap(self.cr, if round { LINE_CAP_ROUND } else { LINE_CAP_BUTT });
            c.cairo_stroke(self.cr);
            c.cairo_set_line_cap(self.cr, LINE_CAP_BUTT);
        }
    }

    fn text(&mut self, at: Point, text: &str, font: &Font, color: Color) {
        let g = gfx();
        self.color(color);
        self.text.with_line(text, font, |layout, _| unsafe {
            g.cairo.cairo_move_to(self.cr, at.x as f64, at.y as f64);
            g.pc.pango_cairo_show_layout(self.cr, layout);
        });
    }

    fn image(&mut self, r: Rect, src: &str, radius: f32, fit: Fit) {
        let cr = self.cr;
        unsafe {
            crate::image::draw(cr, r, src, fit, self.scale, |d| {
                gfx().cairo.cairo_new_path(cr);
                self.rect_path(d, radius);
            })
        }
    }

    fn push_clip(&mut self, r: Rect) {
        let c = &gfx().cairo;
        unsafe {
            c.cairo_save(self.cr);
            c.cairo_rectangle(self.cr, r.x as f64, r.y as f64, r.w as f64, r.h as f64);
            c.cairo_clip(self.cr);
        }
    }

    fn pop_clip(&mut self) {
        unsafe { gfx().cairo.cairo_restore(self.cr) };
    }
}
