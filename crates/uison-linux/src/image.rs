//! Images on Linux: decoded by gdk-pixbuf (`dlopen`; present on any desktop with GTK):
//! PNG and JPEG straight to display size; without it, PNG only via Cairo. The result is a Cairo
//! surface (premultiplied ARGB32), cached in two generations like text lines.

use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::{c_int, c_void};
use std::sync::OnceLock;

use uison_core::image::{bytes, decode_scale, fit_rects, natural_size};
use uison_core::{Fit, Rect, Size};

use crate::dl::dylib;
use crate::gfx::*;

dylib! {
    pub struct Pixbuf: ["libgdk_pixbuf-2.0.so.0"] {
        fn gdk_pixbuf_loader_new() -> *mut c_void;
        fn gdk_pixbuf_loader_set_size(l: *mut c_void, w: c_int, h: c_int);
        fn gdk_pixbuf_loader_write(l: *mut c_void, buf: *const u8, len: usize, err: *mut *mut c_void) -> c_int;
        fn gdk_pixbuf_loader_close(l: *mut c_void, err: *mut *mut c_void) -> c_int;
        fn gdk_pixbuf_loader_get_pixbuf(l: *mut c_void) -> *mut c_void;
        fn gdk_pixbuf_get_width(p: *const c_void) -> c_int;
        fn gdk_pixbuf_get_height(p: *const c_void) -> c_int;
        fn gdk_pixbuf_get_rowstride(p: *const c_void) -> c_int;
        fn gdk_pixbuf_get_n_channels(p: *const c_void) -> c_int;
        fn gdk_pixbuf_get_pixels(p: *const c_void) -> *mut u8;
    }
}

fn pixbuf() -> Option<&'static Pixbuf> {
    static P: OnceLock<Option<Pixbuf>> = OnceLock::new();
    // UISON_PIXBUF=0: no gdk-pixbuf (tests the fallback path).
    P.get_or_init(|| {
        if std::env::var("UISON_PIXBUF").as_deref() == Ok("0") {
            return None;
        }
        Pixbuf::load().map_err(|e| log!("uison: {e} — images: PNG only (Cairo)")).ok()
    })
    .as_ref()
}

/// Images per generation: noticeably fewer of them fit on screen.
const GENERATION: usize = 64;

struct Surf(*mut Surface);

impl Drop for Surf {
    fn drop(&mut self) {
        unsafe { gfx().cairo.cairo_surface_destroy(self.0) }
    }
}

/// Source and decoded size in pixels.
type Key = (Box<str>, i32, i32);

#[derive(Default)]
struct Cache {
    cur: HashMap<Key, Option<Surf>>,
    prev: HashMap<Key, Option<Surf>>,
}

thread_local! {
    static CACHE: RefCell<Cache> = RefCell::default();
}

/// gdk-pixbuf: decode to `w`x`h` and copy into a Cairo surface.
unsafe fn decode_pixbuf(p: &Pixbuf, b: &[u8], w: i32, h: i32) -> Option<Surf> {
    let c = &gfx().cairo;
    let g = &gfx().gobject;
    unsafe {
        let l = p.gdk_pixbuf_loader_new();
        p.gdk_pixbuf_loader_set_size(l, w, h);
        let ok = p.gdk_pixbuf_loader_write(l, b.as_ptr(), b.len(), std::ptr::null_mut()) != 0
            && p.gdk_pixbuf_loader_close(l, std::ptr::null_mut()) != 0;
        let pb = p.gdk_pixbuf_loader_get_pixbuf(l);
        if !ok || pb.is_null() {
            g.g_object_unref(l);
            return None;
        }
        let (pw, ph) = (p.gdk_pixbuf_get_width(pb), p.gdk_pixbuf_get_height(pb));
        let (rs, n) = (p.gdk_pixbuf_get_rowstride(pb) as usize, p.gdk_pixbuf_get_n_channels(pb) as usize);
        let src = p.gdk_pixbuf_get_pixels(pb);
        let s = c.cairo_image_surface_create(FORMAT_ARGB32, pw, ph);
        let (dst, ds) = (c.cairo_image_surface_get_data(s), c.cairo_image_surface_get_stride(s) as usize);
        for y in 0..ph as usize {
            let row = std::slice::from_raw_parts(src.add(y * rs), pw as usize * n);
            let out = std::slice::from_raw_parts_mut(dst.add(y * ds).cast::<u32>(), pw as usize);
            for (x, px) in row.chunks_exact(n).enumerate() {
                let a = if n == 4 { px[3] as u32 } else { 255 };
                let pm = |v: u8| (v as u32 * a + 127) / 255;
                out[x] = (a << 24) | (pm(px[0]) << 16) | (pm(px[1]) << 8) | pm(px[2]);
            }
        }
        c.cairo_surface_mark_dirty(s);
        // The pixbuf belongs to the loader and is freed with it.
        g.g_object_unref(l);
        Some(Surf(s))
    }
}

/// Reading PNG from memory for Cairo: closure is `&mut &[u8]`.
unsafe extern "C" fn read_png(closure: *mut c_void, out: *mut u8, n: u32) -> c_int {
    let rest = unsafe { &mut *closure.cast::<&[u8]>() };
    let n = n as usize;
    if rest.len() < n {
        return 11; // CAIRO_STATUS_READ_ERROR
    }
    unsafe { std::ptr::copy_nonoverlapping(rest.as_ptr(), out, n) };
    *rest = &rest[n..];
    0
}

unsafe fn decode_png(b: &[u8]) -> Option<Surf> {
    let c = &gfx().cairo;
    let mut rest: &[u8] = b;
    unsafe {
        let s = c.cairo_image_surface_create_from_png_stream(read_png, (&mut rest as *mut &[u8]).cast());
        if c.cairo_surface_status(s) != 0 {
            c.cairo_surface_destroy(s);
            return None;
        }
        Some(Surf(s))
    }
}

/// `_NET_WM_ICON` from a PNG (X11): width, height, then pixels as non-premultiplied ARGB.
pub(crate) fn icon_argb(png: &[u8]) -> Option<Vec<u32>> {
    let s = unsafe { decode_png(png)? };
    let c = &gfx().cairo;
    unsafe {
        let (w, h) = (c.cairo_image_surface_get_width(s.0), c.cairo_image_surface_get_height(s.0));
        let (data, stride) = (c.cairo_image_surface_get_data(s.0), c.cairo_image_surface_get_stride(s.0) as usize);
        if data.is_null() || w <= 0 || h <= 0 {
            return None;
        }
        let mut out = Vec::with_capacity(2 + (w * h) as usize);
        out.extend([w as u32, h as u32]);
        for y in 0..h as usize {
            let row = std::slice::from_raw_parts(data.add(y * stride).cast::<u32>(), w as usize);
            // Cairo keeps ARGB32 premultiplied; the window manager expects straight alpha.
            out.extend(row.iter().map(|&p| {
                let a = p >> 24;
                if a == 0 || a == 255 {
                    return p;
                }
                let un = |shift: u32| ((p >> shift & 0xFF) * 255 / a).min(255) << shift;
                a << 24 | un(16) | un(8) | un(0)
            }));
        }
        Some(out)
    }
}

/// Draw `src` into `r` (points; the context is already scaled to the screen `scale`), clipped to the `clip` path.
pub(crate) unsafe fn draw(cr: *mut Cr, r: Rect, src: &str, fit: Fit, scale: f32, clip: impl FnOnce(Rect)) {
    let Some(n) = natural_size(src) else { return };
    let k = decode_scale(n, r, fit, scale);
    let (w, h) = ((n.w * k).round().max(1.0) as i32, (n.h * k).round().max(1.0) as i32);
    let key: Key = (src.into(), w, h);
    let c = &gfx().cairo;
    CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        if !cache.cur.contains_key(&key) {
            let surf = match cache.prev.remove(&key) {
                Some(s) => s,
                None => bytes(src).and_then(|b| unsafe {
                    match pixbuf() {
                        Some(p) => decode_pixbuf(p, &b, w, h),
                        None => decode_png(&b),
                    }
                }),
            };
            if cache.cur.len() >= GENERATION {
                cache.prev = std::mem::take(&mut cache.cur);
            }
            cache.cur.insert(key.clone(), surf);
        }
        let Some(Some(s)) = cache.cur.get(&key) else { return };
        unsafe {
            let (iw, ih) = (c.cairo_image_surface_get_width(s.0) as f32, c.cairo_image_surface_get_height(s.0) as f32);
            let (part, dst) = fit_rects(Size::new(iw, ih), r, fit);
            c.cairo_save(cr);
            clip(dst);
            c.cairo_clip(cr);
            c.cairo_translate(cr, dst.x as f64, dst.y as f64);
            c.cairo_scale(cr, (dst.w / part.w) as f64, (dst.h / part.h) as f64);
            c.cairo_set_source_surface(cr, s.0, -part.x as f64, -part.y as f64);
            c.cairo_pattern_set_filter(c.cairo_get_source(cr), FILTER_GOOD);
            c.cairo_paint(cr);
            c.cairo_restore(cr);
        }
    });
}
