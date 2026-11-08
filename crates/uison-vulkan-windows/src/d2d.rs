//! Direct2D, only to turn text into pixels: a render target over a WIC bitmap draws a line's
//! `IDWriteTextLayout`, and the pixels go into the atlas. Method indices come from `d2d1.h`
//! (the same as in `uison-windows`, checked there by `tools/vtable.cpp`).

use std::ffi::c_void;
use std::ptr::null;

use crate::com::{Com, GUID, HRESULT, HrError, com, create};

#[repr(C)]
#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct PointF {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct ColorF {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct PixelFormat {
    format: u32,
    alpha: u32,
}

#[repr(C)]
struct RenderTargetProperties {
    kind: u32,
    pixel_format: PixelFormat,
    dpi_x: f32,
    dpi_y: f32,
    usage: u32,
    min_level: u32,
}

const IID_ID2D1FACTORY: GUID = GUID(0x06152247, 0x6f50, 0x465a, [0x92, 0x45, 0x11, 0x8b, 0xfd, 0x3b, 0x60, 0x07]);
const DXGI_FORMAT_B8G8R8A8_UNORM: u32 = 87;
const D2D1_ALPHA_MODE_PREMULTIPLIED: u32 = 1;
/// A bitmap with transparency allows only grayscale text antialiasing.
pub const D2D1_TEXT_ANTIALIAS_MODE_GRAYSCALE: u32 = 2;
pub const D2D1_DRAW_TEXT_OPTIONS_ENABLE_COLOR_FONT: u32 = 4;

#[link(name = "d2d1")]
unsafe extern "system" {
    fn D2D1CreateFactory(kind: u32, iid: *const GUID, options: *const c_void, out: *mut *mut c_void) -> HRESULT;
}

pub struct Factory(pub Com);

impl Factory {
    pub fn new() -> Result<Factory, HrError> {
        // Single-threaded factory: all drawing happens on the window thread.
        create(|out| unsafe { D2D1CreateFactory(0, &IID_ID2D1FACTORY, null(), out) }).map(Factory)
    }

    /// A render target into a WIC bitmap (premultiplied BGRA).
    pub fn wic_target(&self, bitmap: &Com, dpi: f32) -> Result<RenderTarget, HrError> {
        let props = RenderTargetProperties {
            kind: 0,
            pixel_format: PixelFormat { format: DXGI_FORMAT_B8G8R8A8_UNORM, alpha: D2D1_ALPHA_MODE_PREMULTIPLIED },
            dpi_x: dpi,
            dpi_y: dpi,
            usage: 0,
            min_level: 0,
        };
        create(|out| unsafe {
            com!(self.0, 13, fn(*mut c_void, *const RenderTargetProperties, *mut *mut c_void) -> HRESULT, bitmap.ptr(), &props, out)
        })
        .map(RenderTarget)
    }
}

/// `ID2D1RenderTarget`.
pub struct RenderTarget(pub Com);

impl RenderTarget {
    pub fn solid_brush(&self) -> Result<Com, HrError> {
        let c = ColorF { r: 0.0, g: 0.0, b: 0.0, a: 1.0 };
        create(|out| unsafe { com!(self.0, 8, fn(*const ColorF, *const c_void, *mut *mut c_void) -> HRESULT, &c, null(), out) })
    }

    pub fn begin_draw(&self) {
        unsafe { com!(self.0, 48, fn()) }
    }

    pub fn end_draw(&self) -> HRESULT {
        unsafe { com!(self.0, 49, fn(*mut u64, *mut u64) -> HRESULT, std::ptr::null_mut(), std::ptr::null_mut()) }
    }

    pub fn set_dpi(&self, dpi: f32) {
        unsafe { com!(self.0, 51, fn(f32, f32), dpi, dpi) }
    }

    pub fn set_text_antialias(&self, mode: u32) {
        unsafe { com!(self.0, 34, fn(u32), mode) }
    }

    pub fn clear(&self, c: ColorF) {
        unsafe { com!(self.0, 47, fn(*const ColorF), &c) }
    }

    /// `layout` is an `IDWriteTextLayout`, `at` is the top-left corner.
    pub fn draw_text_layout(&self, at: PointF, layout: &Com, brush: &Com, options: u32) {
        unsafe { com!(self.0, 28, fn(PointF, *mut c_void, *mut c_void, u32), at, layout.ptr(), brush.ptr(), options) }
    }
}

/// `ID2D1SolidColorBrush::SetColor`.
pub fn set_color(brush: &Com, c: &ColorF) {
    unsafe { com!(brush, 8, fn(*const ColorF), c) }
}
