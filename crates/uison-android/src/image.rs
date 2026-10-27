//! Images on Android: `BitmapFactory` (`UisonView.decodeImage` — straight to display size) and
//! `Canvas.drawBitmap` with a rounded clip (`UisonView.drawImage`). Cache: global references
//! to `Bitmap` in two generations, like text strings.

use std::cell::RefCell;
use std::collections::HashMap;

use uison_core::image::{bytes, decode_scale, fit_rects, natural_size};
use uison_core::{Fit, Rect, Size};

use crate::env;
use crate::java::j;
use crate::jni::*;

/// Images per generation: noticeably fewer of them fit on a screen.
const GENERATION: usize = 64;

/// Global reference to a `Bitmap`.
struct Bmp(jobject);

impl Drop for Bmp {
    fn drop(&mut self) {
        unsafe { env().delete_global(self.0) }
    }
}

type Key = (Box<str>, i32, i32);

#[derive(Default)]
struct Cache {
    cur: HashMap<Key, Option<Bmp>>,
    prev: HashMap<Key, Option<Bmp>>,
}

thread_local! {
    static CACHE: RefCell<Cache> = RefCell::default();
}

unsafe fn decode(src: &str, w: i32, h: i32) -> Option<Bmp> {
    let b = bytes(src)?;
    let e = env();
    unsafe {
        let arr = e.byte_array(&b);
        let bmp = e.call_static_object(j().view_cls, j().view_decode_image, &[l(arr), i(w), i(h)]);
        e.delete_local(arr);
        if e.check() || bmp.is_null() {
            return None;
        }
        let g = e.global(bmp);
        e.delete_local(bmp);
        Some(Bmp(g))
    }
}

/// Draw `src` into `r` (points; the canvas is already scaled by `scale`) with corner `radius`.
pub(crate) unsafe fn draw(canvas: jobject, r: Rect, src: &str, radius: f32, fit: Fit, scale: f32) {
    let Some(n) = natural_size(src) else { return };
    let k = decode_scale(n, r, fit, scale);
    let (w, h) = ((n.w * k).round().max(1.0) as i32, (n.h * k).round().max(1.0) as i32);
    let key: Key = (src.into(), w, h);
    CACHE.with(|c| {
        let mut c = c.borrow_mut();
        if !c.cur.contains_key(&key) {
            let bmp = match c.prev.remove(&key) {
                Some(b) => b,
                None => unsafe { decode(src, w, h) },
            };
            if c.cur.len() >= GENERATION {
                c.prev = std::mem::take(&mut c.cur);
            }
            c.cur.insert(key.clone(), bmp);
        }
        let Some(Some(bmp)) = c.cur.get(&key) else { return };
        let (part, dst) = fit_rects(Size::new(w as f32, h as f32), r, fit);
        let e = env();
        unsafe {
            let px = |v: f32| v.round() as i32;
            e.call_static_void(
                j().view_cls,
                j().view_draw_image,
                &[
                    l(canvas),
                    l(bmp.0),
                    i(px(part.x)),
                    i(px(part.y)),
                    i(px(part.w)),
                    i(px(part.h)),
                    f(dst.x),
                    f(dst.y),
                    f(dst.w),
                    f(dst.h),
                    f(radius),
                ],
            );
            e.check();
        }
    });
}
