//! Images, shared by all backends: bytes by source name, natural size
//! of PNG/JPEG from the header (without decoding), fitting into a rectangle (`Fit`).
//!
//! Source: `mem:name` — bytes from the app ([`put`]); otherwise a file path — absolute
//! or relative to the program's folder (`.app` on iOS), then the current one. The backend decodes and draws.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use crate::{Rect, Size};

/// How an image fills a rectangle.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Hash)]
pub enum Fit {
    /// Fill entirely, crop the excess (avatars, thumbnails).
    #[default]
    Cover,
    /// Entirely inside, empty at the edges.
    Contain,
    /// Stretch without keeping proportions.
    Fill,
}

impl Fit {
    pub fn parse(s: &str) -> Option<Fit> {
        Some(match s {
            "cover" => Fit::Cover,
            "contain" => Fit::Contain,
            "fill" => Fit::Fill,
            _ => return None,
        })
    }

    pub fn name(self) -> &'static str {
        match self {
            Fit::Cover => "cover",
            Fit::Contain => "contain",
            Fit::Fill => "fill",
        }
    }
}

/// Source → bytes (`None` — not found). Files are read once.
type Store = Mutex<HashMap<String, Option<Arc<[u8]>>>>;

fn store() -> &'static Store {
    static S: OnceLock<Store> = OnceLock::new();
    S.get_or_init(Default::default)
}

/// Image bytes from the app: source `mem:name`. New bytes are better under a new name:
/// backends cache decoded images by source.
pub fn put(name: &str, bytes: impl Into<Arc<[u8]>>) {
    store().lock().unwrap().insert(format!("mem:{name}"), Some(bytes.into()));
}

/// Bytes of a source.
pub fn bytes(src: &str) -> Option<Arc<[u8]>> {
    if let Some(b) = store().lock().unwrap().get(src) {
        return b.clone();
    }
    let b = if src.starts_with("mem:") { None } else { read_file(src) };
    store().lock().unwrap().insert(src.to_string(), b.clone());
    b
}

fn read_file(src: &str) -> Option<Arc<[u8]>> {
    let p = std::path::Path::new(src);
    let mut tries = Vec::new();
    if p.is_absolute() {
        tries.push(p.to_path_buf());
    } else {
        if let Some(dir) = std::env::current_exe().ok().and_then(|e| e.parent().map(|d| d.to_path_buf())) {
            tries.push(dir.join(p));
        }
        tries.push(p.to_path_buf());
    }
    tries.iter().find_map(|t| std::fs::read(t).ok()).map(Into::into)
}

/// Natural size in pixels (= points) from the PNG or JPEG header.
pub fn size_of(b: &[u8]) -> Option<(u32, u32)> {
    let be32 = |i: usize| b.get(i..i + 4).map(|s| u32::from_be_bytes(s.try_into().unwrap()));
    let be16 = |i: usize| b.get(i..i + 2).map(|s| u16::from_be_bytes(s.try_into().unwrap()) as u32);
    if b.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Some((be32(16)?, be32(20)?));
    }
    if !b.starts_with(&[0xFF, 0xD8]) {
        return None;
    }
    // JPEG: segments up to SOFn (frame: height, width).
    let mut i = 2;
    while i + 4 <= b.len() {
        if b[i] != 0xFF {
            i += 1;
            continue;
        }
        let marker = b[i + 1];
        if marker == 0xFF || marker == 0x01 || (0xD0..=0xD7).contains(&marker) {
            i += if marker == 0xFF { 1 } else { 2 };
            continue;
        }
        let len = be16(i + 2)? as usize;
        if matches!(marker, 0xC0..=0xCF) && !matches!(marker, 0xC4 | 0xC8 | 0xCC) {
            return Some((be16(i + 7)?, be16(i + 5)?));
        }
        i += 2 + len;
    }
    None
}

/// Natural size of a source in points.
pub fn natural_size(src: &str) -> Option<Size> {
    let (w, h) = size_of(&bytes(src)?)?;
    Some(Size::new(w as f32, h as f32))
}

/// Fit image `img` (pixels) into `dst`: (part of the image in pixels, where to draw it).
pub fn fit_rects(img: Size, dst: Rect, fit: Fit) -> (Rect, Rect) {
    let full = Rect::new(0.0, 0.0, img.w, img.h);
    if img.w <= 0.0 || img.h <= 0.0 || dst.w <= 0.0 || dst.h <= 0.0 {
        return (full, dst);
    }
    match fit {
        Fit::Fill => (full, dst),
        Fit::Cover => {
            let k = (dst.w / img.w).max(dst.h / img.h);
            let (sw, sh) = (dst.w / k, dst.h / k);
            (Rect::new((img.w - sw) / 2.0, (img.h - sh) / 2.0, sw, sh), dst)
        }
        Fit::Contain => {
            let k = (dst.w / img.w).min(dst.h / img.h);
            let (w, h) = (img.w * k, img.h * k);
            (full, Rect::new(dst.x + (dst.w - w) / 2.0, dst.y + (dst.h - h) / 2.0, w, h))
        }
    }
}

/// How many times to downscale an image when decoding so that its pixels are enough
/// to show it in `dst` at screen scale `scale` (at most 1 — we never upscale).
pub fn decode_scale(img: Size, dst: Rect, fit: Fit, scale: f32) -> f32 {
    let (src, d) = fit_rects(img, dst, fit);
    if src.w <= 0.0 || src.h <= 0.0 {
        return 1.0;
    }
    ((d.w / src.w).max(d.h / src.h) * scale).min(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn png_and_jpeg_size() {
        let mut png = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR".to_vec();
        png.extend_from_slice(&640u32.to_be_bytes());
        png.extend_from_slice(&480u32.to_be_bytes());
        assert_eq!(size_of(&png), Some((640, 480)));
        // SOI, APP0 (length 4), SOF0: length, precision, height 200, width 300.
        let jpg = [0xFF, 0xD8, 0xFF, 0xE0, 0, 4, 0, 0, 0xFF, 0xC0, 0, 17, 8, 0, 200, 1, 44, 3];
        assert_eq!(size_of(&jpg), Some((300, 200)));
        assert_eq!(size_of(b"GIF89a"), None);
    }

    #[test]
    fn fit_cover_contain() {
        let img = Size::new(200.0, 100.0);
        let dst = Rect::new(10.0, 10.0, 50.0, 50.0);
        let (s, d) = fit_rects(img, dst, Fit::Cover);
        assert_eq!((s, d), (Rect::new(50.0, 0.0, 100.0, 100.0), dst));
        let (s, d) = fit_rects(img, dst, Fit::Contain);
        assert_eq!((s, d), (Rect::new(0.0, 0.0, 200.0, 100.0), Rect::new(10.0, 22.5, 50.0, 25.0)));
        // A 50 pt avatar on a 2× screen from a 200×100 photo: needs 100 px of height — half.
        assert_eq!(decode_scale(img, dst, Fit::Cover, 2.0), 1.0);
        assert_eq!(decode_scale(Size::new(800.0, 400.0), dst, Fit::Cover, 2.0), 0.25);
    }
}
