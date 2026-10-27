//! `Canvas` over Direct2D. The render target is set to the monitor's DPI,
//! so coordinates are in points (DIP); Direct2D converts to pixels itself.

use uison_core::{Canvas, Color, Font, Paint, Point, Rect};

use crate::com::Com;
use crate::d2d::*;
use crate::text::Text;

/// Resources that live together with the render target.
pub struct Resources {
    pub brush: Com,
    pub strokes: Strokes,
    /// Images are bitmaps of this target (recreated on device loss).
    images: std::cell::RefCell<Images>,
}

impl Resources {
    pub fn new(f: &Factory, rt: &RenderTarget) -> Result<Resources, crate::com::HrError> {
        Ok(Resources { brush: rt.solid_brush()?, strokes: Strokes::new(f)?, images: Default::default() })
    }
}

/// Images per generation: far fewer fit on screen.
const IMAGE_GENERATION: usize = 64;

/// Source and decoded size in pixels → bitmap; two generations, like text lines.
#[derive(Default)]
struct Images {
    cur: std::collections::HashMap<(Box<str>, u32, u32), Option<Com>>,
    prev: std::collections::HashMap<(Box<str>, u32, u32), Option<Com>>,
}

thread_local! {
    /// WIC factory for decoding (created on the first image).
    static WIC: Option<crate::wic::Wic> = crate::wic::Wic::new().ok();
}

pub struct D2dCanvas<'a> {
    rt: &'a RenderTarget,
    factory: &'a Factory,
    res: &'a Resources,
    text: &'a Text,
    /// Physical pixels per point (1.25 at 125 %).
    scale: f32,
    /// Align axis-parallel strokes to the pixel grid.
    snap: bool,
    /// Current brush color: `SetColor` only on change.
    color: Option<Color>,
}

impl<'a> D2dCanvas<'a> {
    pub fn new(rt: &'a RenderTarget, factory: &'a Factory, res: &'a Resources, text: &'a Text, scale: f32) -> Self {
        // UISON_SNAP=0 — no pixel alignment (experiment 2).
        let snap = std::env::var("UISON_SNAP").as_deref() != Ok("0");
        D2dCanvas { rt, factory, res, text, scale, snap, color: None }
    }

    /// Turn alignment off (for experiments).
    pub fn set_snap(&mut self, snap: bool) {
        self.snap = snap;
    }

    /// Align an axis-parallel stroke: width is a whole number of physical
    /// pixels (at least one), edges on pixel boundaries (as on Apple).
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

    fn use_color(&mut self, c: Color) {
        if self.color != Some(c) {
            self.color = Some(c);
            set_color(&self.res.brush, &color(c));
        }
    }
}

pub fn color(c: Color) -> ColorF {
    ColorF { r: c.r as f32 / 255.0, g: c.g as f32 / 255.0, b: c.b as f32 / 255.0, a: c.a as f32 / 255.0 }
}

pub fn rect_f(r: Rect) -> RectF {
    RectF { left: r.x, top: r.y, right: r.right(), bottom: r.bottom() }
}

fn pt(p: Point) -> PointF {
    PointF { x: p.x, y: p.y }
}

fn rounded(r: Rect, radius: f32) -> RoundedRect {
    let rad = radius.min(r.w / 2.0).min(r.h / 2.0).max(0.0);
    RoundedRect { rect: rect_f(r), rx: rad, ry: rad }
}

impl Canvas for D2dCanvas<'_> {
    fn rect(&mut self, r: Rect, radius: f32, paint: Paint) {
        if let Some(fill) = paint.fill {
            self.use_color(fill);
            let b = &self.res.brush;
            if radius > 0.0 {
                self.rt.fill_rounded(&rounded(r, radius), b);
            } else {
                self.rt.fill_rect(&rect_f(r), b);
            }
        }
        if let Some(stroke) = paint.stroke {
            let (r, width) = self.snap_rect(r, paint.width);
            self.use_color(stroke);
            let b = &self.res.brush;
            if radius > 0.0 {
                self.rt.draw_rounded(&rounded(r, radius), b, width, &self.res.strokes.flat);
            } else {
                self.rt.draw_rect(&rect_f(r), b, width, &self.res.strokes.flat);
            }
        }
    }

    fn circle(&mut self, c: Point, r: f32, paint: Paint) {
        let e = Ellipse { center: pt(c), rx: r, ry: r };
        if let Some(fill) = paint.fill {
            self.use_color(fill);
            let b = &self.res.brush;
            self.rt.fill_ellipse(&e, b);
        }
        if let Some(stroke) = paint.stroke {
            self.use_color(stroke);
            let b = &self.res.brush;
            self.rt.draw_ellipse(&e, b, paint.width, &self.res.strokes.flat);
        }
    }

    fn poly(&mut self, p: &[Point], paint: Paint) {
        if p.len() < 2 {
            return;
        }
        let pts: Vec<PointF> = p.iter().map(|&q| pt(q)).collect();
        let Ok(geom) = self.factory.poly(&pts) else { return };
        if let Some(fill) = paint.fill {
            self.use_color(fill);
            let b = &self.res.brush;
            self.rt.fill_geometry(&geom, b);
        }
        if let Some(stroke) = paint.stroke {
            self.use_color(stroke);
            let b = &self.res.brush;
            self.rt.draw_geometry(&geom, b, paint.width, &self.res.strokes.flat);
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
        self.use_color(color);
        let br = &self.res.brush;
        let style = if round { &self.res.strokes.round } else { &self.res.strokes.flat };
        self.rt.draw_line(pt(a), pt(b), br, width, style);
    }

    fn text(&mut self, at: Point, text: &str, font: &Font, color: Color) {
        self.use_color(color);
        let b = &self.res.brush;
        let rt = self.rt;
        self.text.with_line(text, font, |layout, _| {
            if let Some(layout) = layout {
                rt.draw_text_layout(pt(at), layout, b, D2D1_DRAW_TEXT_OPTIONS_ENABLE_COLOR_FONT);
            }
        });
    }

    fn image(&mut self, r: Rect, src: &str, radius: f32, fit: uison_core::Fit) {
        use uison_core::image::{bytes, decode_scale, fit_rects, natural_size};
        let Some(n) = natural_size(src) else { return };
        let k = decode_scale(n, r, fit, self.scale);
        let (w, h) = ((n.w * k).round().max(1.0) as u32, (n.h * k).round().max(1.0) as u32);
        let key = (Box::<str>::from(src), w, h);
        let mut im = self.res.images.borrow_mut();
        if !im.cur.contains_key(&key) {
            let bmp = match im.prev.remove(&key) {
                Some(b) => b,
                None => bytes(src).and_then(|b| {
                    WIC.with(|wic| wic.as_ref()?.decode(&b, w, h).and_then(|s| self.rt.bitmap_from_wic(&s)).ok())
                }),
            };
            if im.cur.len() >= IMAGE_GENERATION {
                im.prev = std::mem::take(&mut im.cur);
            }
            im.cur.insert(key.clone(), bmp);
        }
        let Some(Some(bmp)) = im.cur.get(&key) else { return };
        let (part, dst) = fit_rects(uison_core::Size::new(w as f32, h as f32), r, fit);
        let rf = |r: Rect| RectF { left: r.x, top: r.y, right: r.right(), bottom: r.bottom() };
        let rad = radius.min(dst.w / 2.0).min(dst.h / 2.0);
        if rad <= 0.0 {
            self.rt.draw_bitmap(bmp, &rf(dst), &rf(part));
            return;
        }
        // Rounded corners: a bitmap brush translated so that `part` lands in `dst`.
        let Ok(brush) = self.rt.bitmap_brush(bmp) else { return };
        let (sx, sy) = (dst.w / part.w, dst.h / part.h);
        set_brush_transform(&brush, [sx, 0.0, 0.0, sy, dst.x - part.x * sx, dst.y - part.y * sy]);
        self.rt.fill_rounded(&RoundedRect { rect: rf(dst), rx: rad, ry: rad }, &brush);
    }

    fn push_clip(&mut self, r: Rect) {
        self.rt.push_clip(&rect_f(r), D2D1_ANTIALIAS_MODE_ALIASED);
    }

    fn pop_clip(&mut self) {
        self.rt.pop_clip();
    }
}
