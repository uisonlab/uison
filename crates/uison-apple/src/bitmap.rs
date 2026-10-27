//! Headless rendering: CGBitmapContext. For screenshots, benchmarks and tests
//! of partial redraw (the result must match a full redraw pixel for pixel).

use std::ptr::{null, null_mut};

use uison_core::{Rect, Size};
use uison_frame::Frame;

use crate::canvas::CgCanvas;
use crate::ffi::*;
use crate::text::Text;

pub struct Bitmap {
    ctx: CGContextRef,
    /// Size in logical points.
    pub size: Size,
    pub width: usize,
    pub height: usize,
    pub scale: f32,
}

impl Bitmap {
    /// A canvas of `size` logical points with scale `scale` (2.0 for Retina).
    pub fn new(size: Size, scale: f32) -> Bitmap {
        let width = (size.w * scale).ceil() as usize;
        let height = (size.h * scale).ceil() as usize;
        unsafe {
            let cs = CGColorSpaceCreateDeviceRGB();
            let ctx = CGBitmapContextCreate(null_mut(), width, height, 8, 0, cs, kCGImageAlphaPremultipliedLast);
            CGColorSpaceRelease(cs);
            assert!(!ctx.is_null(), "CGBitmapContextCreate failed");
            // Logical points, Y down, as in a flipped NSView.
            CGContextTranslateCTM(ctx, 0.0, height as f64);
            CGContextScaleCTM(ctx, scale as f64, -scale as f64);
            Bitmap { ctx, size, width, height, scale }
        }
    }

    pub fn canvas<'a>(&self, text: &'a Text) -> CgCanvas<'a> {
        unsafe { CgCanvas::new(self.ctx, text) }
    }

    /// Draw a region of the frame.
    pub fn paint(&self, frame: &Frame, text: &Text, dirty: Rect) {
        uison_frame::paint(frame, &mut self.canvas(text), dirty);
    }

    /// RGBA pixels (premultiplied), rows top to bottom.
    pub fn pixels(&self) -> Vec<u8> {
        unsafe {
            let data = CGBitmapContextGetData(self.ctx);
            let stride = CGBitmapContextGetBytesPerRow(self.ctx);
            let mut out = Vec::with_capacity(self.width * self.height * 4);
            for y in 0..self.height {
                let row = std::slice::from_raw_parts(data.add(y * stride), self.width * 4);
                out.extend_from_slice(row);
            }
            out
        }
    }

    pub fn save_png(&self, path: &str) -> Result<(), String> {
        unsafe {
            let img = CGBitmapContextCreateImage(self.ctx);
            let res = save_image_png(img, path);
            CGImageRelease(img);
            res
        }
    }
}

/// Write a CGImage to PNG via ImageIO.
///
/// # Safety
/// `img` is a valid CGImageRef.
pub unsafe fn save_image_png(img: CFTypeRef, path: &str) -> Result<(), String> {
    unsafe {
        let url = CFURLCreateFromFileSystemRepresentation(null(), path.as_ptr(), path.len() as isize, false);
        let kind = cfstring("public.png");
        let dest = CGImageDestinationCreateWithURL(url, kind, 1, null());
        let ok = !dest.is_null() && {
            CGImageDestinationAddImage(dest, img, null());
            CGImageDestinationFinalize(dest)
        };
        if !dest.is_null() {
            CFRelease(dest);
        }
        CFRelease(kind);
        CFRelease(url);
        if ok { Ok(()) } else { Err(format!("cannot write {path}")) }
    }
}

impl Drop for Bitmap {
    fn drop(&mut self) {
        unsafe { CGContextRelease(self.ctx) }
    }
}
