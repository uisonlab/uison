//! The window's render target (Task6.md, experiment 1):
//! - `ID2D1HwndRenderTarget` (default) — Direct2D presents the frame to the window itself;
//! - `UISON_TARGET=flip` — `ID2D1DeviceContext` over a DXGI flip chain, presenting
//!   only the changed rectangles with `Present1`.

use std::ffi::c_void;
use std::ptr::null_mut;

use crate::canvas::Resources;
use crate::com::{Com, GUID, HRESULT, HrError, check, com, create};
use crate::d2d::*;
use crate::ffi::{HWND, RECT};

const IID_ID2D1FACTORY1: GUID = GUID(0xbb12d362, 0xdaee, 0x4b9a, [0xaa, 0x1d, 0x14, 0xba, 0x40, 0x1c, 0xfa, 0x1f]);
const IID_IDXGIDEVICE: GUID = GUID(0x54ec77fa, 0x1377, 0x44e6, [0x8c, 0x32, 0x88, 0xfd, 0x5f, 0x44, 0xc8, 0x4c]);
const IID_IDXGIFACTORY2: GUID = GUID(0x50c83a1c, 0xe072, 0x4c48, [0x87, 0xb0, 0x36, 0x30, 0xfa, 0x36, 0xa6, 0xd0]);
const IID_IDXGISURFACE: GUID = GUID(0xcafcb56c, 0x6ac3, 0x4889, [0xbf, 0x47, 0x9e, 0x23, 0xbb, 0xd2, 0x60, 0xec]);

#[repr(C)]
struct SwapChainDesc1 {
    width: u32,
    height: u32,
    format: u32,
    stereo: i32,
    sample_count: u32,
    sample_quality: u32,
    buffer_usage: u32,
    buffer_count: u32,
    scaling: u32,
    swap_effect: u32,
    alpha_mode: u32,
    flags: u32,
}

#[repr(C)]
struct PresentParameters {
    dirty_count: u32,
    dirty: *const RECT,
    scroll: *const RECT,
    offset: *const c_void,
}

#[repr(C)]
struct BitmapProperties1 {
    pixel_format: PixelFormat,
    dpi_x: f32,
    dpi_y: f32,
    options: u32,
    color_context: *mut c_void,
}

/// A two-buffer flip chain and a Direct2D context drawing into its back buffer.
pub struct Flip {
    dc: RenderTarget,
    swap: Com,
    bitmap: Option<Com>,
    dpi: f32,
}

impl Flip {
    pub fn new(factory: &Factory, hwnd: HWND, size: SizeU, dpi: f32) -> Result<Flip, HrError> {
        let d3d = d3d_device()?;
        let dxgi = d3d.cast(&IID_IDXGIDEVICE).ok_or(HrError(-1))?;
        let f1 = factory.0.cast(&IID_ID2D1FACTORY1).ok_or(HrError(-1))?;
        let device = create(|out| unsafe { com!(f1, 17, fn(*mut c_void, *mut *mut c_void) -> HRESULT, dxgi.ptr(), out) })?;
        let dc = create(|out| unsafe { com!(device, 4, fn(u32, *mut *mut c_void) -> HRESULT, 0, out) })?;
        let adapter = create(|out| unsafe { com!(dxgi, 7, fn(*mut *mut c_void) -> HRESULT, out) })?;
        let dxgi_factory = create(|out| unsafe { com!(adapter, 6, fn(*const GUID, *mut *mut c_void) -> HRESULT, &IID_IDXGIFACTORY2, out) })?;
        let desc = SwapChainDesc1 {
            width: size.width,
            height: size.height,
            format: DXGI_FORMAT_B8G8R8A8_UNORM,
            stereo: 0,
            sample_count: 1,
            sample_quality: 0,
            buffer_usage: 0x20, // DXGI_USAGE_RENDER_TARGET_OUTPUT
            buffer_count: 2,
            scaling: 1,     // DXGI_SCALING_NONE
            swap_effect: 3, // DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL
            alpha_mode: 0,
            flags: 0,
        };
        let swap = create(|out| unsafe {
            com!(dxgi_factory, 15, fn(*mut c_void, HWND, *const SwapChainDesc1, *const c_void, *mut c_void, *mut *mut c_void) -> HRESULT,
                d3d.ptr(), hwnd, &desc, std::ptr::null(), null_mut(), out)
        })?;
        let mut f = Flip { dc: RenderTarget(dc), swap, bitmap: None, dpi };
        f.bind()?;
        Ok(f)
    }

    /// The chain's back buffer as the context's target.
    fn bind(&mut self) -> Result<(), HrError> {
        let surface = create(|out| unsafe { com!(self.swap, 9, fn(u32, *const GUID, *mut *mut c_void) -> HRESULT, 0, &IID_IDXGISURFACE, out) })?;
        let props = BitmapProperties1 {
            pixel_format: PixelFormat { format: DXGI_FORMAT_B8G8R8A8_UNORM, alpha: D2D1_ALPHA_MODE_IGNORE },
            dpi_x: self.dpi,
            dpi_y: self.dpi,
            options: 1 | 2, // D2D1_BITMAP_OPTIONS_TARGET | CANNOT_DRAW
            color_context: null_mut(),
        };
        let bitmap = create(|out| unsafe {
            com!(self.dc.0, 62, fn(*mut c_void, *const BitmapProperties1, *mut *mut c_void) -> HRESULT, surface.ptr(), &props, out)
        })?;
        unsafe { com!(self.dc.0, 74, fn(*mut c_void), bitmap.ptr()) }; // SetTarget
        self.dc.set_dpi(self.dpi);
        self.bitmap = Some(bitmap);
        Ok(())
    }

    fn resize(&mut self, size: SizeU) -> Result<(), HrError> {
        // No references to the buffers may remain: detach the target and release the bitmap.
        unsafe { com!(self.dc.0, 74, fn(*mut c_void), null_mut()) };
        self.bitmap = None;
        check(unsafe { com!(self.swap, 13, fn(u32, u32, u32, u32, u32) -> HRESULT, 0, size.width, size.height, 0, 0) })?;
        self.bind()
    }

    /// Present the frame: only `dirty` goes to the screen (window pixels).
    fn present(&self, dirty: &[RECT]) -> HRESULT {
        let params = PresentParameters { dirty_count: dirty.len() as u32, dirty: dirty.as_ptr(), scroll: std::ptr::null(), offset: std::ptr::null() };
        unsafe { com!(self.swap, 22, fn(u32, u32, *const PresentParameters) -> HRESULT, 1, 0, &params) }
    }
}

fn d3d_device() -> Result<Com, HrError> {
    let (mut dev, mut ctx) = (null_mut(), null_mut());
    let mut level = 0;
    check(unsafe { D3D11CreateDevice(null_mut(), 1, null_mut(), 0x20, std::ptr::null(), 0, 7, &mut dev, &mut level, &mut ctx) })?;
    drop(unsafe { Com::from_raw(ctx) });
    unsafe { Com::from_raw(dev) }.ok_or(HrError(-1))
}

pub enum Surface {
    Hwnd(RenderTarget),
    Flip(Flip),
}

/// The window's render target and its resources (brush, stroke styles).
pub struct Target {
    pub surface: Surface,
    pub res: Resources,
}

impl Target {
    pub fn new(factory: &Factory, hwnd: HWND, size: SizeU, dpi: f32, present: u32, flip: bool) -> Result<Target, HrError> {
        let surface = if flip {
            Surface::Flip(Flip::new(factory, hwnd, size, dpi)?)
        } else {
            Surface::Hwnd(factory.hwnd_target(hwnd, size, dpi, present)?)
        };
        let res = Resources::new(factory, rt(&surface))?;
        Ok(Target { surface, res })
    }

    pub fn rt(&self) -> &RenderTarget {
        rt(&self.surface)
    }

    pub fn resize(&mut self, size: SizeU) {
        match &mut self.surface {
            Surface::Hwnd(t) => {
                t.resize(size);
            }
            Surface::Flip(f) => {
                let _ = f.resize(size);
            }
        }
    }

    pub fn set_dpi(&mut self, dpi: f32) {
        match &mut self.surface {
            Surface::Hwnd(t) => t.set_dpi(dpi),
            Surface::Flip(f) => {
                f.dpi = dpi;
                f.dc.set_dpi(dpi);
            }
        }
    }

    /// Finish the frame and present it; `dirty` is what was drawn (pixels).
    pub fn end(&self, dirty: &[RECT]) -> HRESULT {
        let hr = self.rt().end_draw();
        match &self.surface {
            Surface::Hwnd(_) => hr,
            Surface::Flip(f) if hr >= 0 => f.present(dirty),
            Surface::Flip(_) => hr,
        }
    }

    pub fn is_flip(&self) -> bool {
        matches!(self.surface, Surface::Flip(_))
    }
}

fn rt(s: &Surface) -> &RenderTarget {
    match s {
        Surface::Hwnd(t) => t,
        Surface::Flip(f) => &f.dc,
    }
}
