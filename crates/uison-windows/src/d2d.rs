//! Direct2D: factory, render target, brush, stroke styles, triangle geometry.
//! Method indices come from `d2d1.h` (checked by `tools/vtable.cpp`).

#![allow(non_snake_case)]

use std::ffi::c_void;
use std::ptr::{null, null_mut};

use crate::com::{Com, GUID, HRESULT, HrError, check, com, create};
use crate::ffi::HWND;

#[repr(C)]
#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct RectF {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct PointF {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct SizeU {
    pub width: u32,
    pub height: u32,
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
pub struct RoundedRect {
    pub rect: RectF,
    pub rx: f32,
    pub ry: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Ellipse {
    pub center: PointF,
    pub rx: f32,
    pub ry: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct PixelFormat {
    pub format: u32,
    pub alpha: u32,
}

/// D2D1_BITMAP_PROPERTIES.
#[repr(C)]
pub struct BitmapProperties {
    pub pixel_format: PixelFormat,
    pub dpi_x: f32,
    pub dpi_y: f32,
}

/// `ID2D1Brush::SetTransform`: a 3×2 matrix (m11 m12 m21 m22 dx dy).
pub fn set_brush_transform(brush: &Com, m: [f32; 6]) {
    unsafe { com!(brush, 5, fn(*const [f32; 6]), &m) }
}

#[repr(C)]
pub struct RenderTargetProperties {
    pub kind: u32,
    pub pixel_format: PixelFormat,
    pub dpi_x: f32,
    pub dpi_y: f32,
    pub usage: u32,
    pub min_level: u32,
}

#[repr(C)]
pub struct HwndRenderTargetProperties {
    pub hwnd: HWND,
    pub pixel_size: SizeU,
    pub present_options: u32,
}

#[repr(C)]
struct StrokeStyleProperties {
    start_cap: u32,
    end_cap: u32,
    dash_cap: u32,
    line_join: u32,
    miter_limit: f32,
    dash_style: u32,
    dash_offset: f32,
}

pub const IID_ID2D1FACTORY: GUID =
    GUID(0x06152247, 0x6f50, 0x465a, [0x92, 0x45, 0x11, 0x8b, 0xfd, 0x3b, 0x60, 0x07]);

pub const DXGI_FORMAT_B8G8R8A8_UNORM: u32 = 87;
pub const D2D1_ALPHA_MODE_PREMULTIPLIED: u32 = 1;
pub const D2D1_ALPHA_MODE_IGNORE: u32 = 3;
pub const D2D1_PRESENT_OPTIONS_NONE: u32 = 0;
pub const D2D1_PRESENT_OPTIONS_RETAIN_CONTENTS: u32 = 1;
pub const D2D1_PRESENT_OPTIONS_IMMEDIATELY: u32 = 2;
pub const D2D1_ANTIALIAS_MODE_ALIASED: u32 = 1;
pub const D2D1_TEXT_ANTIALIAS_MODE_DEFAULT: u32 = 0;
pub const D2D1_TEXT_ANTIALIAS_MODE_CLEARTYPE: u32 = 1;
pub const D2D1_TEXT_ANTIALIAS_MODE_GRAYSCALE: u32 = 2;
pub const D2D1_DRAW_TEXT_OPTIONS_ENABLE_COLOR_FONT: u32 = 4;
/// `D2DERR_RECREATE_TARGET`: device lost (GPU or driver changed) — recreate the target.
pub const D2DERR_RECREATE_TARGET: HRESULT = 0x8899000C_u32 as i32;
const D2D1_CAP_STYLE_FLAT: u32 = 0;
const D2D1_CAP_STYLE_ROUND: u32 = 2;
const D2D1_LINE_JOIN_ROUND: u32 = 2;
const D2D1_FILL_MODE_WINDING: u32 = 1;
const D2D1_FIGURE_BEGIN_FILLED: u32 = 0;
const D2D1_FIGURE_END_CLOSED: u32 = 1;

#[link(name = "d2d1")]
unsafe extern "system" {
    fn D2D1CreateFactory(kind: u32, iid: *const GUID, options: *const c_void, out: *mut *mut c_void) -> HRESULT;
}

/// `ID2D1Factory`.
pub struct Factory(pub Com);

impl Factory {
    pub fn new() -> Result<Factory, HrError> {
        // Single-threaded factory: all drawing happens on the window thread.
        create(|out| unsafe { D2D1CreateFactory(0, &IID_ID2D1FACTORY, null(), out) }).map(Factory)
    }

    /// The window's render target; `dpi` is the monitor's dots per inch: we draw in points (DIP).
    pub fn hwnd_target(&self, hwnd: HWND, size: SizeU, dpi: f32, present: u32) -> Result<RenderTarget, HrError> {
        let props = RenderTargetProperties {
            kind: 0,
            pixel_format: PixelFormat { format: DXGI_FORMAT_B8G8R8A8_UNORM, alpha: D2D1_ALPHA_MODE_IGNORE },
            dpi_x: dpi,
            dpi_y: dpi,
            usage: 0,
            min_level: 0,
        };
        let hwnd_props = HwndRenderTargetProperties { hwnd, pixel_size: size, present_options: present };
        create(|out| unsafe {
            com!(self.0, 14, fn(*const RenderTargetProperties, *const HwndRenderTargetProperties, *mut *mut c_void) -> HRESULT,
                &props, &hwnd_props, out)
        })
        .map(RenderTarget)
    }

    /// A render target into a WIC bitmap (headless screenshots).
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

    fn stroke_style(&self, cap: u32) -> Result<Com, HrError> {
        let p = StrokeStyleProperties {
            start_cap: cap,
            end_cap: cap,
            dash_cap: cap,
            line_join: D2D1_LINE_JOIN_ROUND,
            miter_limit: 10.0,
            dash_style: 0,
            dash_offset: 0.0,
        };
        create(|out| unsafe {
            com!(self.0, 11, fn(*const StrokeStyleProperties, *const f32, u32, *mut *mut c_void) -> HRESULT, &p, null(), 0, out)
        })
    }

    /// A closed filled polygon (at least two points) as a path geometry (`ID2D1PathGeometry`).
    pub fn poly(&self, p: &[PointF]) -> Result<Com, HrError> {
        let geom = create(|out| unsafe { com!(self.0, 10, fn(*mut *mut c_void) -> HRESULT, out) })?;
        // ID2D1PathGeometry::Open → ID2D1GeometrySink.
        let sink = create(|out| unsafe { com!(geom, 17, fn(*mut *mut c_void) -> HRESULT, out) })?;
        unsafe {
            // Nonzero winding, as in CoreGraphics, Cairo and canvas.
            com!(sink, 3, fn(u32), D2D1_FILL_MODE_WINDING); // SetFillMode
            com!(sink, 5, fn(PointF, u32), p[0], D2D1_FIGURE_BEGIN_FILLED); // BeginFigure
            com!(sink, 6, fn(*const PointF, u32), p[1..].as_ptr(), (p.len() - 1) as u32); // AddLines
            com!(sink, 8, fn(u32), D2D1_FIGURE_END_CLOSED); // EndFigure
            check(com!(sink, 9, fn() -> HRESULT))?; // Close
        }
        Ok(geom)
    }
}

/// Stroke styles: both with round joins (like `kCGLineJoinRound` on Apple).
pub struct Strokes {
    pub flat: Com,
    pub round: Com,
}

impl Strokes {
    pub fn new(f: &Factory) -> Result<Strokes, HrError> {
        Ok(Strokes { flat: f.stroke_style(D2D1_CAP_STYLE_FLAT)?, round: f.stroke_style(D2D1_CAP_STYLE_ROUND)? })
    }
}

/// `ID2D1RenderTarget` (or `ID2D1HwndRenderTarget`, which derives from it).
pub struct RenderTarget(pub Com);

impl RenderTarget {
    pub fn solid_brush(&self) -> Result<Com, HrError> {
        let c = ColorF { r: 0.0, g: 0.0, b: 0.0, a: 1.0 };
        create(|out| unsafe {
            com!(self.0, 8, fn(*const ColorF, *const c_void, *mut *mut c_void) -> HRESULT, &c, null(), out)
        })
    }

    pub fn begin_draw(&self) {
        unsafe { com!(self.0, 48, fn()) }
    }

    pub fn end_draw(&self) -> HRESULT {
        unsafe { com!(self.0, 49, fn(*mut u64, *mut u64) -> HRESULT, null_mut(), null_mut()) }
    }

    pub fn set_dpi(&self, dpi: f32) {
        unsafe { com!(self.0, 51, fn(f32, f32), dpi, dpi) }
    }

    pub fn get_dpi(&self) -> f32 {
        let (mut x, mut y) = (0.0f32, 0.0f32);
        unsafe { com!(self.0, 52, fn(*mut f32, *mut f32), &mut x, &mut y) };
        x
    }

    /// Size in DIP: the struct is returned via a hidden pointer (MSVC ABI for C++ methods).
    pub fn get_size(&self) -> (f32, f32) {
        let mut s = [0.0f32; 2];
        unsafe { com!(self.0, 53, fn(*mut [f32; 2]) -> *mut [f32; 2], &mut s) };
        (s[0], s[1])
    }

    pub fn get_pixel_size(&self) -> SizeU {
        let mut s = SizeU::default();
        unsafe { com!(self.0, 54, fn(*mut SizeU) -> *mut SizeU, &mut s) };
        s
    }

    pub fn set_text_antialias(&self, mode: u32) {
        unsafe { com!(self.0, 34, fn(u32), mode) }
    }

    /// `ID2D1HwndRenderTarget::Resize` (window target only).
    pub fn resize(&self, size: SizeU) -> HRESULT {
        unsafe { com!(self.0, 58, fn(*const SizeU) -> HRESULT, &size) }
    }

    pub fn clear(&self, c: ColorF) {
        unsafe { com!(self.0, 47, fn(*const ColorF), &c) }
    }

    pub fn fill_rect(&self, r: &RectF, brush: &Com) {
        unsafe { com!(self.0, 17, fn(*const RectF, *mut c_void), r, brush.ptr()) }
    }

    pub fn draw_rect(&self, r: &RectF, brush: &Com, width: f32, style: &Com) {
        unsafe { com!(self.0, 16, fn(*const RectF, *mut c_void, f32, *mut c_void), r, brush.ptr(), width, style.ptr()) }
    }

    pub fn fill_rounded(&self, r: &RoundedRect, brush: &Com) {
        unsafe { com!(self.0, 19, fn(*const RoundedRect, *mut c_void), r, brush.ptr()) }
    }

    pub fn draw_rounded(&self, r: &RoundedRect, brush: &Com, width: f32, style: &Com) {
        unsafe { com!(self.0, 18, fn(*const RoundedRect, *mut c_void, f32, *mut c_void), r, brush.ptr(), width, style.ptr()) }
    }

    pub fn fill_ellipse(&self, e: &Ellipse, brush: &Com) {
        unsafe { com!(self.0, 21, fn(*const Ellipse, *mut c_void), e, brush.ptr()) }
    }

    pub fn draw_ellipse(&self, e: &Ellipse, brush: &Com, width: f32, style: &Com) {
        unsafe { com!(self.0, 20, fn(*const Ellipse, *mut c_void, f32, *mut c_void), e, brush.ptr(), width, style.ptr()) }
    }

    pub fn draw_line(&self, a: PointF, b: PointF, brush: &Com, width: f32, style: &Com) {
        unsafe { com!(self.0, 15, fn(PointF, PointF, *mut c_void, f32, *mut c_void), a, b, brush.ptr(), width, style.ptr()) }
    }

    pub fn fill_geometry(&self, g: &Com, brush: &Com) {
        unsafe { com!(self.0, 23, fn(*mut c_void, *mut c_void, *mut c_void), g.ptr(), brush.ptr(), null_mut()) }
    }

    pub fn draw_geometry(&self, g: &Com, brush: &Com, width: f32, style: &Com) {
        unsafe { com!(self.0, 22, fn(*mut c_void, *mut c_void, f32, *mut c_void), g.ptr(), brush.ptr(), width, style.ptr()) }
    }

    /// `layout` is an `IDWriteTextLayout`, `at` is the top-left corner.
    pub fn draw_text_layout(&self, at: PointF, layout: &Com, brush: &Com, options: u32) {
        unsafe { com!(self.0, 28, fn(PointF, *mut c_void, *mut c_void, u32), at, layout.ptr(), brush.ptr(), options) }
    }

    /// `ID2D1Bitmap` from a WIC source (premultiplied BGRA, 96 dpi: pixels = DIP).
    pub fn bitmap_from_wic(&self, src: &Com) -> Result<Com, HrError> {
        let props = BitmapProperties {
            pixel_format: PixelFormat { format: DXGI_FORMAT_B8G8R8A8_UNORM, alpha: D2D1_ALPHA_MODE_PREMULTIPLIED },
            dpi_x: 96.0,
            dpi_y: 96.0,
        };
        create(|out| unsafe {
            com!(self.0, 5, fn(*mut c_void, *const BitmapProperties, *mut *mut c_void) -> HRESULT, src.ptr(), &props, out)
        })
    }

    /// Part `src` (pixels) of the bitmap into `dst`, linear interpolation.
    pub fn draw_bitmap(&self, bmp: &Com, dst: &RectF, src: &RectF) {
        unsafe { com!(self.0, 26, fn(*mut c_void, *const RectF, f32, u32, *const RectF), bmp.ptr(), dst, 1.0, 1, src) }
    }

    /// A bitmap brush (for a rounded image: `FillRoundedRectangle` with this brush).
    pub fn bitmap_brush(&self, bmp: &Com) -> Result<Com, HrError> {
        create(|out| unsafe {
            com!(self.0, 7, fn(*mut c_void, *const c_void, *const c_void, *mut *mut c_void) -> HRESULT, bmp.ptr(), null(), null(), out)
        })
    }

    pub fn push_clip(&self, r: &RectF, aa: u32) {
        unsafe { com!(self.0, 45, fn(*const RectF, u32), r, aa) }
    }

    pub fn pop_clip(&self) {
        unsafe { com!(self.0, 46, fn()) }
    }
}

/// `ID2D1SolidColorBrush::SetColor`.
pub fn set_color(brush: &Com, c: &ColorF) {
    unsafe { com!(brush, 8, fn(*const ColorF), c) }
}

#[link(name = "d3d11")]
unsafe extern "system" {
    pub(crate) fn D3D11CreateDevice(
        adapter: *mut c_void,
        driver: u32,
        software: *mut c_void,
        flags: u32,
        levels: *const u32,
        n: u32,
        sdk: u32,
        device: *mut *mut c_void,
        level: *mut u32,
        context: *mut *mut c_void,
    ) -> HRESULT;
}

/// Create and release a Direct3D device: loading the driver is most of
/// creating a render target (~40 ms), worth doing ahead of time on another thread.
pub fn warm_device() {
    let (mut dev, mut ctx) = (null_mut(), null_mut());
    let mut level = 0;
    // D3D_DRIVER_TYPE_HARDWARE, D3D11_CREATE_DEVICE_BGRA_SUPPORT, D3D11_SDK_VERSION.
    let hr = unsafe { D3D11CreateDevice(null_mut(), 1, null_mut(), 0x20, null(), 0, 7, &mut dev, &mut level, &mut ctx) };
    if hr >= 0 {
        drop(unsafe { (Com::from_raw(ctx), Com::from_raw(dev)) });
    }
}
