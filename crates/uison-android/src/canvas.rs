//! `Canvas` on top of `android.graphics.Canvas` via JNI. The canvas is scaled by
//! screen density: we draw in logical points (dp); axis-parallel strokes
//! are snapped to physical pixels — as in the other backends.

use std::cell::Cell;

use uison_core::{Canvas, Color, Fit, Font, Paint, Point, Rect};

use crate::java::j;
use crate::jni::*;
use crate::text::AndroidText;

const ANTI_ALIAS: i32 = 1;

/// Paints for shapes and a path for triangles — created once; paint state
/// is remembered so Java isn't called needlessly.
pub struct Paints {
    fill: jobject,
    stroke: jobject,
    path: jobject,
    fill_color: Cell<i32>,
    stroke_color: Cell<i32>,
    stroke_width: Cell<f32>,
    round: Cell<bool>,
}

impl Paints {
    pub unsafe fn new(e: Env) -> Paints {
        let j = j();
        unsafe {
            let fill = e.new_object(j.paint_cls, j.paint_new, &[i(ANTI_ALIAS)]);
            let stroke = e.new_object(j.paint_cls, j.paint_new, &[i(ANTI_ALIAS)]);
            e.call_void(stroke, j.set_style, &[l(j.style_stroke)]);
            e.call_void(stroke, j.set_stroke_join, &[l(j.join_round)]);
            let path = e.new_object(j.path_cls, j.path_new, &[]);
            Paints {
                fill,
                stroke,
                path,
                fill_color: Cell::new(0),
                stroke_color: Cell::new(0),
                stroke_width: Cell::new(-1.0),
                round: Cell::new(false),
            }
        }
    }
}

fn argb(c: Color) -> i32 {
    ((c.a as u32) << 24 | (c.r as u32) << 16 | (c.g as u32) << 8 | c.b as u32) as i32
}

pub struct JCanvas<'a> {
    e: Env,
    c: jobject,
    text: &'a AndroidText,
    p: &'a Paints,
    /// Physical pixels per point.
    scale: f32,
}

impl<'a> JCanvas<'a> {
    pub fn new(e: Env, c: jobject, text: &'a AndroidText, p: &'a Paints, scale: f32) -> JCanvas<'a> {
        JCanvas { e, c, text, p, scale }
    }

    fn fill(&self, c: Color) -> jobject {
        let v = argb(c);
        if self.p.fill_color.replace(v) != v {
            unsafe { self.e.call_void(self.p.fill, j().set_color, &[i(v)]) };
        }
        self.p.fill
    }

    fn stroke(&self, c: Color, width: f32, round: bool) -> jobject {
        let j = j();
        let v = argb(c);
        unsafe {
            if self.p.stroke_color.replace(v) != v {
                self.e.call_void(self.p.stroke, j.set_color, &[i(v)]);
            }
            if self.p.stroke_width.replace(width) != width {
                self.e.call_void(self.p.stroke, j.set_stroke_width, &[f(width)]);
            }
            if self.p.round.replace(round) != round {
                let cap = if round { j.cap_round } else { j.cap_butt };
                self.e.call_void(self.p.stroke, j.set_stroke_cap, &[l(cap)]);
            }
        }
        self.p.stroke
    }

    fn snap(&self, center: f32, width: f32) -> (f32, f32) {
        let s = self.scale.max(1.0);
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

    fn rect_call(&self, r: Rect, radius: f32, paint: jobject) {
        let j = j();
        unsafe {
            if radius > 0.0 {
                let rad = radius.min(r.w / 2.0).min(r.h / 2.0);
                self.e.call_void(self.c, j.draw_round_rect, &[f(r.x), f(r.y), f(r.right()), f(r.bottom()), f(rad), f(rad), l(paint)]);
            } else {
                self.e.call_void(self.c, j.draw_rect, &[f(r.x), f(r.y), f(r.right()), f(r.bottom()), l(paint)]);
            }
        }
    }
}

impl Canvas for JCanvas<'_> {
    fn rect(&mut self, r: Rect, radius: f32, paint: Paint) {
        if let Some(c) = paint.fill {
            let p = self.fill(c);
            self.rect_call(r, radius, p);
        }
        if let Some(c) = paint.stroke {
            let (r, w) = self.snap_rect(r, paint.width);
            let p = self.stroke(c, w, false);
            self.rect_call(r, radius, p);
        }
    }

    fn circle(&mut self, c: Point, r: f32, paint: Paint) {
        let j = j();
        unsafe {
            if let Some(col) = paint.fill {
                let p = self.fill(col);
                self.e.call_void(self.c, j.draw_circle, &[f(c.x), f(c.y), f(r), l(p)]);
            }
            if let Some(col) = paint.stroke {
                let p = self.stroke(col, paint.width, false);
                self.e.call_void(self.c, j.draw_circle, &[f(c.x), f(c.y), f(r), l(p)]);
            }
        }
    }

    fn poly(&mut self, pts: &[Point], paint: Paint) {
        if pts.len() < 2 {
            return;
        }
        let j = j();
        let path = self.p.path;
        unsafe {
            self.e.call_void(path, j.path_reset, &[]);
            self.e.call_void(path, j.path_move_to, &[f(pts[0].x), f(pts[0].y)]);
            for q in &pts[1..] {
                self.e.call_void(path, j.path_line_to, &[f(q.x), f(q.y)]);
            }
            self.e.call_void(path, j.path_close, &[]);
            if let Some(c) = paint.fill {
                let p = self.fill(c);
                self.e.call_void(self.c, j.draw_path, &[l(path), l(p)]);
            }
            if let Some(c) = paint.stroke {
                let p = self.stroke(c, paint.width, false);
                self.e.call_void(self.c, j.draw_path, &[l(path), l(p)]);
            }
        }
    }

    fn line(&mut self, mut a: Point, mut b: Point, mut width: f32, color: Color, round: bool) {
        if a.y == b.y {
            let (y, w) = self.snap(a.y, width);
            (a.y, b.y, width) = (y, y, w);
        } else if a.x == b.x {
            let (x, w) = self.snap(a.x, width);
            (a.x, b.x, width) = (x, x, w);
        }
        let p = self.stroke(color, width, round);
        unsafe { self.e.call_void(self.c, j().draw_line, &[f(a.x), f(a.y), f(b.x), f(b.y), l(p)]) }
    }

    fn text(&mut self, at: Point, text: &str, font: &Font, color: Color) {
        let (paint, s, m) = self.text.get(text, font);
        unsafe {
            self.e.call_void(paint, j().set_color, &[i(argb(color))]);
            self.e.call_void(self.c, j().draw_text, &[l(s), f(at.x), f(at.y + m.ascent), l(paint)]);
        }
    }

    fn image(&mut self, r: Rect, src: &str, radius: f32, fit: Fit) {
        unsafe { crate::image::draw(self.c, r, src, radius, fit, self.scale) }
    }

    fn push_clip(&mut self, r: Rect) {
        let j = j();
        unsafe {
            self.e.call_int(self.c, j.save, &[]);
            self.e.call_bool(self.c, j.clip_rect, &[f(r.x), f(r.y), f(r.right()), f(r.bottom())]);
        }
    }

    fn pop_clip(&mut self) {
        unsafe { self.e.call_void(self.c, j().restore, &[]) }
    }
}
