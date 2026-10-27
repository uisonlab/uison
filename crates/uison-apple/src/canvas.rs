//! `Canvas` on top of CoreGraphics. The context must be "flipped"
//! (origin at top left, Y down), as in an NSView with `isFlipped = YES`.

use uison_core::{Canvas, Color, Fit, Font, Paint, Point, Rect};

use crate::ffi::*;
use crate::text::Text;

pub struct CgCanvas<'a> {
    ctx: CGContextRef,
    text: &'a Text,
    /// Physical pixels per point (2.0 on Retina).
    scale: f32,
    /// Snap axis-parallel strokes to the pixel grid.
    snap: bool,
}

impl<'a> CgCanvas<'a> {
    /// # Safety
    /// `ctx` is a valid CGContext for the canvas lifetime.
    pub unsafe fn new(ctx: CGContextRef, text: &'a Text) -> Self {
        unsafe {
            CGContextSetLineJoin(ctx, kCGLineJoinRound);
            // Flipped coordinate system, so flip glyphs too.
            CGContextSetTextMatrix(ctx, CGAffineTransform { a: 1.0, b: 0.0, c: 0.0, d: -1.0, tx: 0.0, ty: 0.0 });
        }
        let scale = unsafe { CGContextGetUserSpaceToDeviceSpaceTransform(ctx) }.a.abs() as f32;
        CgCanvas { ctx, text, scale, snap: true }
    }

    /// Turn snapping off (for experiments).
    pub fn set_snap(&mut self, snap: bool) {
        self.snap = snap;
    }

    /// Snap an axis-parallel stroke: width becomes a whole number of physical
    /// pixels (at least one), edges land on pixel boundaries.
    /// The shift is at most half a pixel, covered by the AA margin in the element bounds.
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

    fn fill(&self, c: Color) {
        let [r, g, b, a] = rgba(c);
        unsafe { CGContextSetRGBFillColor(self.ctx, r, g, b, a) }
    }

    fn stroke(&self, c: Color, width: f32) {
        let [r, g, b, a] = rgba(c);
        unsafe {
            CGContextSetRGBStrokeColor(self.ctx, r, g, b, a);
            CGContextSetLineWidth(self.ctx, width as f64);
        }
    }

    fn rounded_path(&self, r: Rect, radius: f32) {
        let rad = radius.min(r.w / 2.0).min(r.h / 2.0) as f64;
        let (x, y, x2, y2) = (r.x as f64, r.y as f64, r.right() as f64, r.bottom() as f64);
        let c = self.ctx;
        unsafe {
            CGContextBeginPath(c);
            CGContextMoveToPoint(c, x + rad, y);
            CGContextAddArcToPoint(c, x2, y, x2, y2, rad);
            CGContextAddArcToPoint(c, x2, y2, x, y2, rad);
            CGContextAddArcToPoint(c, x, y2, x, y, rad);
            CGContextAddArcToPoint(c, x, y, x2, y, rad);
            CGContextClosePath(c);
        }
    }

    fn poly_path(&self, p: &[Point]) {
        let c = self.ctx;
        unsafe {
            CGContextBeginPath(c);
            CGContextMoveToPoint(c, p[0].x as f64, p[0].y as f64);
            for q in &p[1..] {
                CGContextAddLineToPoint(c, q.x as f64, q.y as f64);
            }
            CGContextClosePath(c);
        }
    }
}

fn rgba(c: Color) -> [f64; 4] {
    [c.r as f64 / 255.0, c.g as f64 / 255.0, c.b as f64 / 255.0, c.a as f64 / 255.0]
}

fn circle_rect(c: Point, r: f32) -> CGRect {
    CGRect::new((c.x - r) as f64, (c.y - r) as f64, (2.0 * r) as f64, (2.0 * r) as f64)
}

impl Canvas for CgCanvas<'_> {
    fn rect(&mut self, r: Rect, radius: f32, paint: Paint) {
        let c = self.ctx;
        if let Some(fill) = paint.fill {
            self.fill(fill);
            if radius > 0.0 {
                self.rounded_path(r, radius);
                unsafe { CGContextFillPath(c) }
            } else {
                unsafe { CGContextFillRect(c, r.into()) }
            }
        }
        if let Some(stroke) = paint.stroke {
            let (r, width) = self.snap_rect(r, paint.width);
            self.stroke(stroke, width);
            if radius > 0.0 {
                self.rounded_path(r, radius);
                unsafe { CGContextStrokePath(c) }
            } else {
                unsafe { CGContextStrokeRectWithWidth(c, r.into(), width as f64) }
            }
        }
    }

    fn circle(&mut self, center: Point, r: f32, paint: Paint) {
        let rect = circle_rect(center, r);
        if let Some(fill) = paint.fill {
            self.fill(fill);
            unsafe { CGContextFillEllipseInRect(self.ctx, rect) }
        }
        if let Some(stroke) = paint.stroke {
            self.stroke(stroke, paint.width);
            unsafe { CGContextStrokeEllipseInRect(self.ctx, rect) }
        }
    }

    fn poly(&mut self, p: &[Point], paint: Paint) {
        if p.len() < 2 {
            return;
        }
        if let Some(fill) = paint.fill {
            self.fill(fill);
            self.poly_path(p);
            unsafe { CGContextFillPath(self.ctx) }
        }
        if let Some(stroke) = paint.stroke {
            self.stroke(stroke, paint.width);
            self.poly_path(p);
            unsafe { CGContextStrokePath(self.ctx) }
        }
    }

    fn line(&mut self, mut a: Point, mut b: Point, mut width: f32, color: Color, round: bool) {
        if a.y == b.y {
            let (y, w) = self.snap_stroke(a.y, width);
            (a.y, b.y, width) = (y, y, w);
        } else if a.x == b.x {
            let (x, w) = self.snap_stroke(a.x, width);
            (a.x, b.x, width) = (x, x, w);
        }
        self.stroke(color, width);
        let c = self.ctx;
        unsafe {
            if round {
                CGContextSetLineCap(c, kCGLineCapRound);
            }
            CGContextBeginPath(c);
            CGContextMoveToPoint(c, a.x as f64, a.y as f64);
            CGContextAddLineToPoint(c, b.x as f64, b.y as f64);
            CGContextStrokePath(c);
            if round {
                CGContextSetLineCap(c, kCGLineCapButt);
            }
        }
    }

    fn text(&mut self, at: Point, text: &str, font: &Font, color: Color) {
        self.fill(color);
        let ctx = self.ctx;
        self.text.with_line(text, font, |line, m| unsafe {
            CGContextSetTextPosition(ctx, at.x as f64, (at.y + m.ascent) as f64);
            CTLineDraw(line, ctx);
        });
    }

    fn image(&mut self, r: Rect, src: &str, radius: f32, fit: Fit) {
        unsafe { crate::image::draw(self.ctx, r, src, fit, self.scale, |d| self.rounded_path(d, radius)) }
    }

    fn push_clip(&mut self, r: Rect) {
        unsafe {
            CGContextSaveGState(self.ctx);
            CGContextClipToRect(self.ctx, r.into());
        }
    }

    fn pop_clip(&mut self) {
        unsafe { CGContextRestoreGState(self.ctx) }
    }
}
