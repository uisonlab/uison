//! Images on Apple: ImageIO decodes straight to the display size
//! (`CGImageSourceCreateThumbnailAtIndex`); the `CGImage` cache has two generations, like text lines.

use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::c_void;

use uison_core::image::{bytes, decode_scale, fit_rects, natural_size};
use uison_core::{Fit, Rect, Size};

use crate::ffi::*;

/// Images per generation: far fewer fit on screen.
const GENERATION: usize = 64;

struct Img(CFTypeRef);

impl Drop for Img {
    fn drop(&mut self) {
        unsafe { CGImageRelease(self.0) }
    }
}

/// Source and the longest side of the decoded image in pixels.
type Key = (Box<str>, u32);

#[derive(Default)]
struct Cache {
    cur: HashMap<Key, Option<Img>>,
    prev: HashMap<Key, Option<Img>>,
}

thread_local! {
    static CACHE: RefCell<Cache> = RefCell::default();
}

/// Decode `src` so that its longest side is `max_px`.
unsafe fn decode(src: &str, max_px: u32) -> Option<Img> {
    let b = bytes(src)?;
    unsafe {
        let data = CFDataCreate(std::ptr::null(), b.as_ptr(), b.len() as isize);
        let source = CGImageSourceCreateWithData(data, std::ptr::null());
        CFRelease(data);
        if source.is_null() {
            return None;
        }
        let px = max_px.max(1) as i32;
        let num = CFNumberCreate(std::ptr::null(), CF_NUMBER_SINT32, (&px as *const i32).cast::<c_void>());
        let keys = [
            kCGImageSourceCreateThumbnailFromImageAlways,
            kCGImageSourceThumbnailMaxPixelSize,
            kCGImageSourceCreateThumbnailWithTransform,
        ];
        let values = [kCFBooleanTrue, num, kCFBooleanTrue];
        let opts = CFDictionaryCreate(
            std::ptr::null(),
            keys.as_ptr(),
            values.as_ptr(),
            3,
            (&raw const kCFTypeDictionaryKeyCallBacks).cast(),
            (&raw const kCFTypeDictionaryValueCallBacks).cast(),
        );
        let img = CGImageSourceCreateThumbnailAtIndex(source, 0, opts);
        CFRelease(opts);
        CFRelease(num);
        CFRelease(source);
        (!img.is_null()).then_some(Img(img))
    }
}

/// Draw the image into `r` (the context is flipped: Y down). `scale` is pixels per point.
/// `clip` builds the rounded path the image is clipped to.
pub(crate) unsafe fn draw(ctx: CGContextRef, r: Rect, src: &str, fit: Fit, scale: f32, clip: impl FnOnce(Rect)) {
    let Some(n) = natural_size(src) else { return };
    let k = decode_scale(n, r, fit, scale);
    let max_px = (n.w.max(n.h) * k).ceil() as u32;
    let key: Key = (src.into(), max_px);
    CACHE.with(|c| {
        let mut c = c.borrow_mut();
        if !c.cur.contains_key(&key) {
            let img = match c.prev.remove(&key) {
                Some(img) => img,
                None => unsafe { decode(src, max_px) },
            };
            if c.cur.len() >= GENERATION {
                c.prev = std::mem::take(&mut c.cur);
            }
            c.cur.insert(key.clone(), img);
        }
        let Some(Some(img)) = c.cur.get(&key) else { return };
        unsafe {
            let (iw, ih) = (CGImageGetWidth(img.0) as f32, CGImageGetHeight(img.0) as f32);
            let (part, dst) = fit_rects(Size::new(iw, ih), r, fit);
            let whole = part.x == 0.0 && part.y == 0.0 && part.w == iw && part.h == ih;
            // For cover, crop the needed part (CGImage coordinates start at the top left corner).
            let cropped = if whole { std::ptr::null() } else { CGImageCreateWithImageInRect(img.0, CGRect::from(part)) };
            let image = if cropped.is_null() { img.0 } else { cropped };
            CGContextSaveGState(ctx);
            clip(dst);
            CGContextClip(ctx);
            CGContextSetInterpolationQuality(ctx, INTERPOLATION_HIGH);
            // The context is flipped, but CoreGraphics draws images bottom up, so flip back.
            CGContextTranslateCTM(ctx, dst.x as f64, (dst.y + dst.h) as f64);
            CGContextScaleCTM(ctx, 1.0, -1.0);
            CGContextDrawImage(ctx, CGRect::new(0.0, 0.0, dst.w as f64, dst.h as f64), image);
            CGContextRestoreGState(ctx);
            if !cropped.is_null() {
                CGImageRelease(cropped);
            }
        }
    });
}
