//! Text lines into pixels: DirectWrite lays the line out (the same layout `Text` measures with),
//! Direct2D draws it into a WIC bitmap in its color (color fonts stay in color), and the
//! trimmed pixels go into the atlas. Antialiasing is grayscale: a bitmap with transparency
//! allows no ClearType.

use uison_core::{Color, Font};

use crate::com::{Com, HrError};
use crate::d2d::*;
use crate::text::Text;
use crate::wic::{Wic, pixels};

/// A line's pixels: premultiplied BGRA, `w`×`h`, the top-left corner at (`ox`, `oy`) from the
/// line's `at` (whole pixels).
pub struct Raster {
    pub px: Vec<u8>,
    pub w: u32,
    pub h: u32,
    pub ox: i32,
    pub oy: i32,
}

/// The bitmap lines are drawn into; grows to the largest line.
struct Scratch {
    bitmap: Com,
    target: RenderTarget,
    brush: Com,
    w: u32,
    h: u32,
}

pub struct Lines {
    factory: Factory,
    scratch: Option<Scratch>,
}

impl Lines {
    pub fn new() -> Result<Lines, HrError> {
        Ok(Lines { factory: Factory::new()?, scratch: None })
    }

    fn scratch(&mut self, wic: &Wic, w: u32, h: u32) -> Result<&Scratch, HrError> {
        if self.scratch.as_ref().is_none_or(|s| s.w < w || s.h < h) {
            let (w, h) = (w.next_multiple_of(256), h.next_multiple_of(64));
            let (w, h) = self.scratch.as_ref().map_or((w, h), |s| (s.w.max(w), s.h.max(h)));
            let bitmap = wic.bitmap(w, h)?;
            let target = self.factory.wic_target(&bitmap, 96.0)?;
            target.set_text_antialias(D2D1_TEXT_ANTIALIAS_MODE_GRAYSCALE);
            let brush = target.solid_brush()?;
            self.scratch = Some(Scratch { bitmap, target, brush, w, h });
        }
        Ok(self.scratch.as_ref().unwrap())
    }

    /// The line's pixels at `scale` pixels per point, at most `max` pixels on each side;
    /// `None` — nothing visible (spaces) or a failure.
    #[allow(clippy::too_many_arguments)]
    pub fn render(&mut self, wic: &Wic, text: &Text, s: &str, font: &Font, color: Color, scale: f32, max: u32) -> Option<Raster> {
        text.with_line(s, font, |layout, m| {
            let layout = layout?;
            // Room around the layout box: italics, accents and emoji reach outside it.
            let pad_x = (font.size * scale * 0.5).ceil() as u32 + 2;
            let pad_y = (font.size * scale * 0.25).ceil() as u32 + 2;
            let w = ((m.width * scale).ceil() as u32 + 2 * pad_x).min(max);
            let h = (((m.ascent + m.descent) * scale).ceil() as u32 + 2 * pad_y).min(max);
            let sc = self.scratch(wic, w, h).ok()?;
            let rt = &sc.target;
            rt.set_dpi(96.0 * scale);
            rt.begin_draw();
            rt.clear(ColorF::default());
            let c = ColorF { r: color.r as f32 / 255.0, g: color.g as f32 / 255.0, b: color.b as f32 / 255.0, a: color.a as f32 / 255.0 };
            set_color(&sc.brush, &c);
            let at = PointF { x: pad_x as f32 / scale, y: pad_y as f32 / scale };
            rt.draw_text_layout(at, layout, &sc.brush, D2D1_DRAW_TEXT_OPTIONS_ENABLE_COLOR_FONT);
            if rt.end_draw() < 0 {
                return None;
            }
            let px = pixels(&sc.bitmap, 0, 0, w, h).ok()?;
            trim(&px, w, h, pad_x as i32, pad_y as i32)
        })
    }
}

/// Cut the transparent margins off.
fn trim(px: &[u8], w: u32, h: u32, pad_x: i32, pad_y: i32) -> Option<Raster> {
    let alpha = |x: u32, y: u32| px[((y * w + x) * 4 + 3) as usize];
    let rows: Vec<u32> = (0..h).filter(|&y| (0..w).any(|x| alpha(x, y) != 0)).collect();
    let (&y0, &y1) = (rows.first()?, rows.last()?);
    let x0 = (0..w).find(|&x| (y0..=y1).any(|y| alpha(x, y) != 0))?;
    let x1 = (0..w).rev().find(|&x| (y0..=y1).any(|y| alpha(x, y) != 0))?;
    let (tw, th) = (x1 - x0 + 1, y1 - y0 + 1);
    let mut out = Vec::with_capacity((tw * th * 4) as usize);
    for y in y0..=y1 {
        let start = ((y * w + x0) * 4) as usize;
        out.extend_from_slice(&px[start..start + (tw * 4) as usize]);
    }
    Some(Raster { px: out, w: tw, h: th, ox: x0 as i32 - pad_x, oy: y0 as i32 - pad_y })
}
