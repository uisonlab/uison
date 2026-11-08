//! `Canvas` over Vulkan: every primitive becomes one instance of the shader's quad, in window
//! pixels. Boxes, circles and lines are measured by the shader; text lines and polygons are
//! rasterized once on the CPU into the atlas, images into textures of their own.

use std::collections::HashMap;

use uison_core::{Canvas, Color, Fit, Font, Paint, Point, Rect, Size};

use crate::atlas::Shelves;
use crate::gpu::*;
use crate::lines::Lines;
use crate::raster;
use crate::text::Text;
use crate::wic::Wic;

/// Images per generation: far fewer fit on screen.
const IMAGE_GENERATION: usize = 64;
/// Shader programs per generation: a pipeline each.
const SHADER_GENERATION: usize = 32;

/// What an atlas entry holds.
#[derive(Clone, PartialEq, Eq, Hash)]
enum Key {
    /// A text line in its color at a scale (`f32` bits).
    Text { text: Box<str>, family: Option<Box<str>>, size: u32, bold: bool, italic: bool, color: Color, scale: u32 },
    /// A polygon's fill or stroke mask: points in pixels and the stroke width (`f32` bits; 0 — fill),
    /// cut to the window of that size.
    Poly { points: Vec<u32>, width: u32, bounds: [i32; 4] },
}

/// An atlas entry: pixels at (`x`, `y`) in the atlas, drawn at (`dx`, `dy`): for text, from the
/// line's corner; for masks, in the window.
#[derive(Clone, Copy)]
struct Entry {
    x: u32,
    y: u32,
    w: u32,
    h: u32,
    dx: i32,
    dy: i32,
}

/// Resources that outlive a frame: the atlas and its entries, images, the line rasterizer.
pub struct Res {
    shelves: Shelves,
    entries: HashMap<Key, Option<Entry>>,
    /// The atlas filled up during this frame: it is reset and the frame drawn again.
    pub overflow: bool,
    lines: Lines,
    wic: Wic,
    /// Source and decoded size → texture; two generations, like text lines.
    images: HashMap<(Box<str>, u32, u32), Option<usize>>,
    images_prev: HashMap<(Box<str>, u32, u32), Option<usize>>,
    /// Textures to free at the start of the next frame: this one may still draw them.
    pub retired: Vec<usize>,
    /// Shader source → pipeline (`None`: the program has a mistake, already reported); two generations.
    shaders: HashMap<Box<str>, Option<usize>>,
    shaders_prev: HashMap<Box<str>, Option<usize>>,
    /// Pipelines to free at the start of the next frame.
    pub retired_pipelines: Vec<usize>,
    /// The window in pixels: masks are cut to it.
    pub bounds: [i32; 4],
}

impl Res {
    pub fn new() -> Result<Res, String> {
        Ok(Res {
            shelves: Shelves::new(ATLAS_SIZE),
            entries: HashMap::new(),
            overflow: false,
            lines: Lines::new().map_err(|e| format!("Direct2D: {e}"))?,
            wic: Wic::new().map_err(|e| format!("WIC: {e}"))?,
            images: HashMap::new(),
            images_prev: HashMap::new(),
            retired: Vec::new(),
            shaders: HashMap::new(),
            shaders_prev: HashMap::new(),
            retired_pipelines: Vec::new(),
            bounds: [0; 4],
        })
    }

    pub fn reset_atlas(&mut self) {
        self.shelves.reset();
        self.entries.clear();
    }

    /// Put `w`×`h` pixels into the atlas with a transparent pixel around them: linear filtering
    /// never picks up a neighbor. `None` — larger than the atlas (not drawn), or the atlas is full
    /// (`overflow` is set: the frame is drawn again with an empty atlas).
    fn insert(&mut self, scene: &mut Scene, px: &[u8], w: u32, h: u32, dx: i32, dy: i32) -> Option<Entry> {
        if w + 2 > ATLAS_SIZE || h + 2 > ATLAS_SIZE {
            return None;
        }
        let Some((x, y)) = self.shelves.place(w + 2, h + 2) else {
            self.overflow = true;
            return None;
        };
        let row = ((w + 2) * 4) as usize;
        let mut padded = vec![0u8; row * (h + 2) as usize];
        for (i, src) in px.chunks_exact((w * 4) as usize).enumerate() {
            padded[row * (i + 1) + 4..][..src.len()].copy_from_slice(src);
        }
        scene.upload(ATLAS, x, y, w + 2, h + 2, &padded);
        Some(Entry { x: x + 1, y: y + 1, w, h, dx, dy })
    }
}

pub struct VkCanvas<'a> {
    gpu: &'a mut Gpu,
    scene: &'a mut Scene,
    res: &'a mut Res,
    text: &'a Text,
    /// Physical pixels per point (1.25 at 125 %).
    scale: f32,
    /// Align axis-parallel strokes to the pixel grid.
    snap: bool,
    /// Clip rectangles in pixels (x0, y0, x1, y1); the last one is in effect.
    clips: Vec<[f32; 4]>,
}

/// Premultiplied, 0..1.
fn premul(c: Color) -> [f32; 4] {
    let a = c.a as f32 / 255.0;
    [c.r as f32 / 255.0 * a, c.g as f32 / 255.0 * a, c.b as f32 / 255.0 * a, a]
}

impl<'a> VkCanvas<'a> {
    pub fn new(gpu: &'a mut Gpu, scene: &'a mut Scene, res: &'a mut Res, text: &'a Text, scale: f32) -> Self {
        // UISON_SNAP=0 — no pixel alignment (as in the Direct2D backend).
        let snap = std::env::var("UISON_SNAP").as_deref() != Ok("0");
        let b = res.bounds.map(|v| v as f32);
        VkCanvas { gpu, scene, res, text, scale, snap, clips: vec![b] }
    }

    fn clip(&self) -> [f32; 4] {
        *self.clips.last().unwrap()
    }

    fn px(&self, r: Rect) -> [f32; 4] {
        let s = self.scale;
        [r.x * s, r.y * s, r.right() * s, r.bottom() * s]
    }

    /// Align an axis-parallel stroke (in points): width is a whole number of physical pixels
    /// (at least one), edges on pixel boundaries — the same rule as Direct2D's canvas.
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

    fn shape(&mut self, kind: f32, rect: [f32; 4], color: Color, extra: [f32; 4], radius: f32, width: f32) {
        if color.a == 0 {
            return;
        }
        let inst = Inst { rect, color: premul(color), extra, clip: self.clip(), param: [radius, width, kind, 0.0] };
        self.scene.push(ATLAS, inst);
    }

    /// Pixels from the atlas at (`x`, `y`), multiplied by `tint`.
    fn atlas(&mut self, e: Entry, x: f32, y: f32, tint: [f32; 4]) {
        let n = ATLAS_SIZE as f32;
        let inst = Inst {
            rect: [x, y, x + e.w as f32, y + e.h as f32],
            color: tint,
            extra: [e.x as f32 / n, e.y as f32 / n, (e.x + e.w) as f32 / n, (e.y + e.h) as f32 / n],
            clip: self.clip(),
            param: [0.0, 0.0, KIND_TEXTURE, 0.0],
        };
        self.scene.push(ATLAS, inst);
    }

    fn mask(&mut self, points: &[(f32, f32)], width: f32, color: Color) {
        if color.a == 0 {
            return;
        }
        let key = Key::Poly {
            points: points.iter().flat_map(|p| [p.0.to_bits(), p.1.to_bits()]).collect(),
            width: width.to_bits(),
            bounds: self.res.bounds,
        };
        let entry = match self.res.entries.get(&key) {
            Some(e) => *e,
            None => {
                let m = if width > 0.0 { raster::stroke(points, width, self.res.bounds) } else { raster::fill(points, self.res.bounds) };
                let e = match m {
                    // White coverage: the tint gives it the color.
                    Some(m) => {
                        let px: Vec<u8> = m.cov.iter().flat_map(|&c| [c; 4]).collect();
                        match self.res.insert(self.scene, &px, m.w, m.h, m.x, m.y) {
                            // The atlas is full: this frame is drawn again, not remembered.
                            None if self.res.overflow => return,
                            e => e,
                        }
                    }
                    None => None,
                };
                self.res.entries.insert(key, e);
                e
            }
        };
        if let Some(e) = entry {
            self.atlas(e, e.dx as f32, e.dy as f32, premul(color));
        }
    }

    /// The texture for image `src` decoded to `w`×`h`.
    fn image_texture(&mut self, src: &str, w: u32, h: u32) -> Option<usize> {
        let key = (Box::<str>::from(src), w, h);
        if let Some(t) = self.res.images.get(&key) {
            return *t;
        }
        let tex = match self.res.images_prev.remove(&key) {
            Some(t) => t,
            None => uison_core::image::bytes(src).and_then(|b| {
                let px = self.res.wic.decode(&b, w, h).ok()?;
                let t = self.gpu.texture(w, h).map_err(|e| log!("uison: image texture: {e}")).ok()?;
                self.scene.upload(t, 0, 0, w, h, &px);
                Some(t)
            }),
        };
        if self.res.images.len() >= IMAGE_GENERATION {
            let old = std::mem::replace(&mut self.res.images_prev, std::mem::take(&mut self.res.images));
            self.res.retired.extend(old.into_values().flatten());
        }
        self.res.images.insert(key, tex);
        tex
    }

    /// The pipeline for shader program `src`, compiled on first use. A mistake is reported once.
    fn shader_pipeline(&mut self, src: &str) -> Option<usize> {
        if let Some(p) = self.res.shaders.get(src) {
            return *p;
        }
        let pipe = match self.res.shaders_prev.remove(src) {
            Some(p) => p,
            None => match uison_shader::spirv::compile(src) {
                Ok(spirv) => self.gpu.shader_pipeline(&spirv).map_err(|e| log!("uison: shader pipeline: {e}")).ok(),
                Err(e) => {
                    log!("uison: shader: {e}\n  {src}");
                    None
                }
            },
        };
        if self.res.shaders.len() >= SHADER_GENERATION {
            let old = std::mem::replace(&mut self.res.shaders_prev, std::mem::take(&mut self.res.shaders));
            self.res.retired_pipelines.extend(old.into_values().flatten());
        }
        self.res.shaders.insert(src.into(), pipe);
        pipe
    }
}

impl Canvas for VkCanvas<'_> {
    fn rect(&mut self, r: Rect, radius: f32, paint: Paint) {
        let s = self.scale;
        if let Some(fill) = paint.fill {
            self.shape(KIND_FILL, self.px(r), fill, [0.0; 4], radius * s, 0.0);
        }
        if let Some(stroke) = paint.stroke {
            let (x, w) = self.snap_stroke(r.x, paint.width);
            let (y, _) = self.snap_stroke(r.y, paint.width);
            let (right, _) = self.snap_stroke(r.right(), paint.width);
            let (bottom, _) = self.snap_stroke(r.bottom(), paint.width);
            let rect = [x * s, y * s, right * s, bottom * s];
            self.shape(KIND_STROKE, rect, stroke, [0.0; 4], radius * s, w * s);
        }
    }

    fn circle(&mut self, c: Point, r: f32, paint: Paint) {
        let s = self.scale;
        let rect = [(c.x - r) * s, (c.y - r) * s, (c.x + r) * s, (c.y + r) * s];
        if let Some(fill) = paint.fill {
            self.shape(KIND_FILL, rect, fill, [0.0; 4], r * s, 0.0);
        }
        if let Some(stroke) = paint.stroke {
            self.shape(KIND_STROKE, rect, stroke, [0.0; 4], r * s, paint.width * s);
        }
    }

    fn poly(&mut self, p: &[Point], paint: Paint) {
        if p.len() < 2 {
            return;
        }
        let s = self.scale;
        let pts: Vec<(f32, f32)> = p.iter().map(|q| (q.x * s, q.y * s)).collect();
        if let Some(fill) = paint.fill {
            self.mask(&pts, 0.0, fill);
        }
        if let Some(stroke) = paint.stroke {
            self.mask(&pts, paint.width * s, stroke);
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
        let s = self.scale;
        let kind = if round { KIND_LINE_ROUND } else { KIND_LINE };
        self.shape(kind, [0.0; 4], color, [a.x * s, a.y * s, b.x * s, b.y * s], 0.0, width * s);
    }

    fn text(&mut self, at: Point, text: &str, font: &Font, color: Color) {
        if color.a == 0 || text.is_empty() {
            return;
        }
        let key = Key::Text {
            text: text.into(),
            family: font.family.clone(),
            size: font.size.to_bits(),
            bold: font.bold,
            italic: font.italic,
            color,
            scale: self.scale.to_bits(),
        };
        let entry = match self.res.entries.get(&key) {
            Some(e) => *e,
            None => {
                let Res { lines, wic, .. } = &mut *self.res;
                let e = match lines.render(wic, self.text, text, font, color, self.scale, ATLAS_SIZE - 2) {
                    Some(r) => match self.res.insert(self.scene, &r.px, r.w, r.h, r.ox, r.oy) {
                        None if self.res.overflow => return,
                        e => e,
                    },
                    None => None,
                };
                self.res.entries.insert(key, e);
                e
            }
        };
        if let Some(e) = entry {
            // The line starts on a whole pixel, as Direct2D snaps text.
            let x = (at.x * self.scale).round() + e.dx as f32;
            let y = (at.y * self.scale).round() + e.dy as f32;
            self.atlas(e, x, y, [1.0; 4]);
        }
    }

    fn image(&mut self, r: Rect, src: &str, radius: f32, fit: Fit) {
        use uison_core::image::{decode_scale, fit_rects, natural_size};
        let Some(n) = natural_size(src) else { return };
        let k = decode_scale(n, r, fit, self.scale);
        let (w, h) = ((n.w * k).round().max(1.0) as u32, (n.h * k).round().max(1.0) as u32);
        let Some(tex) = self.image_texture(src, w, h) else { return };
        let (part, dst) = fit_rects(Size::new(w as f32, h as f32), r, fit);
        let (fw, fh) = (w as f32, h as f32);
        let rad = radius.min(dst.w / 2.0).min(dst.h / 2.0).max(0.0);
        let inst = Inst {
            rect: self.px(dst),
            color: [1.0; 4],
            extra: [part.x / fw, part.y / fh, part.right() / fw, part.bottom() / fh],
            clip: self.clip(),
            param: [rad * self.scale, 0.0, KIND_TEXTURE, 0.0],
        };
        self.scene.push(tex, inst);
    }

    fn shader(&mut self, r: Rect, src: &str, time: f32, radius: f32) {
        let Some(pipe) = self.shader_pipeline(src) else { return };
        let rad = radius.min(r.w / 2.0).min(r.h / 2.0).max(0.0);
        // The interface of `uison_shader::spirv`: the box, (time, scale), (radius); kind "box" for `ui.vert`.
        let inst = Inst {
            rect: self.px(r),
            color: [0.0; 4],
            extra: [time, self.scale, 0.0, 0.0],
            clip: self.clip(),
            param: [rad * self.scale, 0.0, KIND_FILL, 0.0],
        };
        self.scene.push_with(pipe, ATLAS, inst);
    }

    fn push_clip(&mut self, r: Rect) {
        // Whole pixels, like Direct2D's aliased clip: a pixel is in if its center is.
        let c = self.px(r).map(f32::round);
        let t = self.clip();
        self.clips.push([c[0].max(t[0]), c[1].max(t[1]), c[2].min(t[2]), c[3].min(t[3])]);
    }

    fn pop_clip(&mut self) {
        if self.clips.len() > 1 {
            self.clips.pop();
        }
    }
}
