//! Cairo (drawing into an in-memory buffer) and Pango (text) are system libraries of any
//! desktop: both GTK and Qt apps pull them in (via Firefox, LibreOffice...).

use std::ffi::{c_char, c_int, c_void};
use std::sync::OnceLock;

use crate::dl::dylib;

pub type Surface = c_void;
pub type Cr = c_void;
pub type FontOptions = c_void;
pub type PangoContext = c_void;
pub type PangoLayout = c_void;
pub type FontDesc = c_void;

pub const FORMAT_ARGB32: c_int = 0;
pub const LINE_CAP_BUTT: c_int = 0;
pub const LINE_CAP_ROUND: c_int = 1;
pub const LINE_JOIN_MITER: c_int = 0;
pub const LINE_JOIN_ROUND: c_int = 1;
pub const HINT_STYLE_SLIGHT: c_int = 2;
pub const HINT_METRICS_OFF: c_int = 1;
pub const ANTIALIAS_GRAY: c_int = 2;
pub const OPERATOR_SOURCE: c_int = 1;
pub const OPERATOR_OVER: c_int = 2;
pub const PANGO_SCALE: f64 = 1024.0;

#[repr(C)]
#[derive(Default, Clone, Copy)]
pub struct PangoRect {
    pub x: c_int,
    pub y: c_int,
    pub width: c_int,
    pub height: c_int,
}

dylib! {
    pub struct Cairo: ["libcairo.so.2"] {
        fn cairo_image_surface_create(format: c_int, w: c_int, h: c_int) -> *mut Surface;
        fn cairo_image_surface_create_for_data(data: *mut u8, format: c_int, w: c_int, h: c_int, stride: c_int) -> *mut Surface;
        fn cairo_surface_destroy(s: *mut Surface);
        fn cairo_surface_flush(s: *mut Surface);
        fn cairo_surface_write_to_png(s: *mut Surface, path: *const c_char) -> c_int;
        fn cairo_create(s: *mut Surface) -> *mut Cr;
        fn cairo_destroy(cr: *mut Cr);
        fn cairo_save(cr: *mut Cr);
        fn cairo_restore(cr: *mut Cr);
        fn cairo_scale(cr: *mut Cr, sx: f64, sy: f64);
        fn cairo_translate(cr: *mut Cr, tx: f64, ty: f64);
        fn cairo_set_source_rgba(cr: *mut Cr, r: f64, g: f64, b: f64, a: f64);
        fn cairo_set_operator(cr: *mut Cr, op: c_int);
        fn cairo_rectangle(cr: *mut Cr, x: f64, y: f64, w: f64, h: f64);
        fn cairo_arc(cr: *mut Cr, xc: f64, yc: f64, r: f64, a1: f64, a2: f64);
        fn cairo_move_to(cr: *mut Cr, x: f64, y: f64);
        fn cairo_line_to(cr: *mut Cr, x: f64, y: f64);
        fn cairo_close_path(cr: *mut Cr);
        fn cairo_new_path(cr: *mut Cr);
        fn cairo_new_sub_path(cr: *mut Cr);
        fn cairo_fill(cr: *mut Cr);
        fn cairo_fill_preserve(cr: *mut Cr);
        fn cairo_stroke(cr: *mut Cr);
        fn cairo_clip(cr: *mut Cr);
        fn cairo_paint(cr: *mut Cr);
        fn cairo_set_line_width(cr: *mut Cr, w: f64);
        fn cairo_set_line_cap(cr: *mut Cr, cap: c_int);
        fn cairo_set_line_join(cr: *mut Cr, join: c_int);
        fn cairo_font_options_create() -> *mut FontOptions;
        fn cairo_font_options_set_hint_style(o: *mut FontOptions, v: c_int);
        fn cairo_font_options_set_hint_metrics(o: *mut FontOptions, v: c_int);
        fn cairo_font_options_set_antialias(o: *mut FontOptions, v: c_int);
        fn cairo_image_surface_get_data(s: *mut Surface) -> *mut u8;
        fn cairo_image_surface_get_stride(s: *mut Surface) -> c_int;
        fn cairo_image_surface_get_width(s: *mut Surface) -> c_int;
        fn cairo_image_surface_get_height(s: *mut Surface) -> c_int;
        fn cairo_image_surface_create_from_png_stream(
            read: unsafe extern "C" fn(*mut c_void, *mut u8, u32) -> c_int,
            closure: *mut c_void,
        ) -> *mut Surface;
        fn cairo_surface_status(s: *mut Surface) -> c_int;
        fn cairo_surface_mark_dirty(s: *mut Surface);
        fn cairo_set_source_surface(cr: *mut Cr, s: *mut Surface, x: f64, y: f64);
        fn cairo_get_source(cr: *mut Cr) -> *mut c_void;
        fn cairo_pattern_set_filter(p: *mut c_void, filter: c_int);
    }
}

/// CAIRO_FILTER_GOOD.
pub const FILTER_GOOD: c_int = 1;

dylib! {
    pub struct Pango: ["libpango-1.0.so.0"] {
        fn pango_font_map_create_context(map: *mut c_void) -> *mut PangoContext;
        fn pango_layout_new(ctx: *mut PangoContext) -> *mut PangoLayout;
        fn pango_layout_set_text(l: *mut PangoLayout, text: *const c_char, len: c_int);
        fn pango_layout_set_font_description(l: *mut PangoLayout, d: *const FontDesc);
        fn pango_layout_get_extents(l: *mut PangoLayout, ink: *mut PangoRect, logical: *mut PangoRect);
        fn pango_layout_get_baseline(l: *mut PangoLayout) -> c_int;
        fn pango_font_description_new() -> *mut FontDesc;
        fn pango_font_description_set_family(d: *mut FontDesc, family: *const c_char);
        fn pango_font_description_set_absolute_size(d: *mut FontDesc, size: f64);
        fn pango_font_description_set_weight(d: *mut FontDesc, w: c_int);
        fn pango_font_description_set_style(d: *mut FontDesc, s: c_int);
        fn pango_font_description_free(d: *mut FontDesc);
        fn pango_font_description_from_string(s: *const c_char) -> *mut FontDesc;
        fn pango_font_description_get_family(d: *const FontDesc) -> *const c_char;
        opt fn pango_context_set_round_glyph_positions(ctx: *mut PangoContext, round: c_int);
    }
}

dylib! {
    pub struct PangoCairo: ["libpangocairo-1.0.so.0"] {
        fn pango_cairo_font_map_get_default() -> *mut c_void;
        fn pango_cairo_context_set_font_options(ctx: *mut PangoContext, o: *const FontOptions);
        fn pango_cairo_show_layout(cr: *mut Cr, l: *mut PangoLayout);
    }
}

dylib! {
    pub struct GObject: ["libgobject-2.0.so.0"] {
        fn g_object_unref(o: *mut c_void);
    }
}

pub struct Gfx {
    pub cairo: Cairo,
    pub pango: Pango,
    pub pc: PangoCairo,
    pub gobject: GObject,
}

static GFX: OnceLock<Result<Gfx, String>> = OnceLock::new();

/// Libraries are loaded once; without them there is nothing to draw with.
pub fn try_gfx() -> Result<&'static Gfx, String> {
    GFX.get_or_init(|| {
        Ok(Gfx { cairo: Cairo::load()?, pango: Pango::load()?, pc: PangoCairo::load()?, gobject: GObject::load()? })
    })
    .as_ref()
    .map_err(Clone::clone)
}

pub fn gfx() -> &'static Gfx {
    match try_gfx() {
        Ok(g) => g,
        Err(e) => {
            log!("uison: {e} (Cairo and Pango are needed: libcairo2, libpango-1.0-0, libpangocairo-1.0-0)");
            std::process::exit(1)
        }
    }
}

/// Cairo surface with a drawing context.
pub struct Image {
    pub surface: *mut Surface,
    pub cr: *mut Cr,
}

impl Image {
    /// Cairo's own memory (headless screenshots).
    pub fn new(w: i32, h: i32) -> Image {
        let c = &gfx().cairo;
        unsafe {
            let surface = c.cairo_image_surface_create(FORMAT_ARGB32, w.max(1), h.max(1));
            Image { surface, cr: c.cairo_create(surface) }
        }
    }

    /// On top of foreign memory (a `wl_shm` buffer / X11 SHM segment): ARGB32 = ARGB8888.
    ///
    /// # Safety
    /// `data` outlives the image and holds `stride * h` bytes.
    pub unsafe fn over(data: *mut u8, w: i32, h: i32, stride: i32) -> Image {
        let c = &gfx().cairo;
        unsafe {
            let surface = c.cairo_image_surface_create_for_data(data, FORMAT_ARGB32, w, h, stride);
            Image { surface, cr: c.cairo_create(surface) }
        }
    }

    pub fn flush(&self) {
        unsafe { gfx().cairo.cairo_surface_flush(self.surface) };
    }

    pub fn save_png(&self, path: &str) -> Result<(), String> {
        self.flush();
        let c = format!("{path}\0");
        match unsafe { gfx().cairo.cairo_surface_write_to_png(self.surface, c.as_ptr().cast()) } {
            0 => Ok(()),
            s => Err(format!("cairo status {s}")),
        }
    }
}

impl Drop for Image {
    fn drop(&mut self) {
        let c = &gfx().cairo;
        unsafe {
            c.cairo_destroy(self.cr);
            c.cairo_surface_destroy(self.surface);
        }
    }
}
