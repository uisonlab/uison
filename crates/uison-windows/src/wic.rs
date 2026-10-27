//! PNG via WIC (Windows Imaging Component): window screenshots and headless frames.

use std::ffi::c_void;
use std::ptr::{null, null_mut};

use crate::com::{Com, GUID, HRESULT, HrError, check, com, create};
use crate::ffi::{CLSCTX_INPROC_SERVER, CoCreateInstance, wide};

const CLSID_WIC_IMAGING_FACTORY: GUID = GUID(0xcacaf262, 0x9370, 0x4615, [0xa1, 0x3b, 0x9f, 0x55, 0x39, 0xda, 0x4c, 0x0a]);
const IID_IWIC_IMAGING_FACTORY: GUID = GUID(0xec5ec8a9, 0xc395, 0x4314, [0x9c, 0x77, 0x54, 0xd7, 0xa9, 0x35, 0xff, 0x70]);
const GUID_CONTAINER_FORMAT_PNG: GUID = GUID(0x1b7cfaf4, 0x713f, 0x473c, [0xbb, 0xcd, 0x61, 0x37, 0x42, 0x5f, 0xae, 0xaf]);
pub const GUID_WIC_PIXEL_FORMAT_32BPP_BGRA: GUID =
    GUID(0x6fddc324, 0x4e03, 0x4bfe, [0xb1, 0x85, 0x3d, 0x77, 0x76, 0x8d, 0xc9, 0x0f]);
pub const GUID_WIC_PIXEL_FORMAT_32BPP_PBGRA: GUID =
    GUID(0x6fddc324, 0x4e03, 0x4bfe, [0xb1, 0x85, 0x3d, 0x77, 0x76, 0x8d, 0xc9, 0x10]);
const GENERIC_WRITE: u32 = 0x4000_0000;
const WIC_BITMAP_ENCODER_NO_CACHE: u32 = 2;
const WIC_BITMAP_CACHE_ON_LOAD: u32 = 1;
const WIC_INTERPOLATION_FANT: u32 = 3;

/// `IWICImagingFactory`.
pub struct Wic(Com);

/// What to write into a PNG frame.
pub enum Pixels<'a> {
    /// BGRA top to bottom, `stride` bytes per row.
    Bgra { w: u32, h: u32, stride: u32, data: &'a [u8] },
    /// A ready `IWICBitmap` (the headless render target).
    Bitmap(&'a Com),
}

impl Wic {
    pub fn new() -> Result<Wic, HrError> {
        create(|out| unsafe {
            CoCreateInstance(&CLSID_WIC_IMAGING_FACTORY, null_mut(), CLSCTX_INPROC_SERVER, &IID_IWIC_IMAGING_FACTORY, out)
        })
        .map(Wic)
    }

    /// In-memory bitmap for a Direct2D render target (premultiplied BGRA).
    pub fn bitmap(&self, w: u32, h: u32) -> Result<Com, HrError> {
        create(|out| unsafe {
            com!(self.0, 17, fn(u32, u32, *const GUID, u32, *mut *mut c_void) -> HRESULT,
                w, h, &GUID_WIC_PIXEL_FORMAT_32BPP_PBGRA, WIC_BITMAP_CACHE_ON_LOAD, out)
        })
    }

    /// PNG/JPEG from memory, downscaled to `w`×`h` (Fant), in premultiplied BGRA:
    /// a source for `CreateBitmapFromWicBitmap` (pixels are copied when the bitmap is created).
    pub fn decode(&self, data: &[u8], w: u32, h: u32) -> Result<Com, HrError> {
        self.decode_as(data, w, h, &GUID_WIC_PIXEL_FORMAT_32BPP_PBGRA)
    }

    /// PNG/JPEG from memory downscaled to `w`×`h`: straight (not premultiplied) BGRA rows,
    /// as icons want them.
    pub fn decode_bgra(&self, data: &[u8], w: u32, h: u32) -> Result<Vec<u8>, HrError> {
        let src = self.decode_as(data, w, h, &GUID_WIC_PIXEL_FORMAT_32BPP_BGRA)?;
        let mut px = vec![0u8; (w * h * 4) as usize];
        unsafe {
            // IWICBitmapSource::CopyPixels: the whole image (null rect), w*4 bytes per row.
            check(com!(src, 7, fn(*const c_void, u32, u32, *mut u8) -> HRESULT, null(), w * 4, px.len() as u32, px.as_mut_ptr()))?;
        }
        Ok(px)
    }

    fn decode_as(&self, data: &[u8], w: u32, h: u32, format: &GUID) -> Result<Com, HrError> {
        let stream = create(|out| unsafe { com!(self.0, 14, fn(*mut *mut c_void) -> HRESULT, out) })?; // CreateStream
        unsafe {
            check(com!(stream, 16, fn(*const u8, u32) -> HRESULT, data.as_ptr(), data.len() as u32))?; // InitializeFromMemory
        }
        let decoder = create(|out| unsafe {
            // CreateDecoderFromStream, WICDecodeMetadataCacheOnDemand.
            com!(self.0, 4, fn(*mut c_void, *const GUID, u32, *mut *mut c_void) -> HRESULT, stream.ptr(), null(), 0, out)
        })?;
        let frame = create(|out| unsafe { com!(decoder, 13, fn(u32, *mut *mut c_void) -> HRESULT, 0, out) })?; // GetFrame
        let scaler = create(|out| unsafe { com!(self.0, 11, fn(*mut *mut c_void) -> HRESULT, out) })?; // CreateBitmapScaler
        unsafe {
            // Initialize: WICBitmapInterpolationModeFant — good downscaling.
            check(com!(scaler, 8, fn(*mut c_void, u32, u32, u32) -> HRESULT, frame.ptr(), w, h, WIC_INTERPOLATION_FANT))?;
        }
        let conv = create(|out| unsafe { com!(self.0, 10, fn(*mut *mut c_void) -> HRESULT, out) })?; // CreateFormatConverter
        unsafe {
            check(com!(conv, 8, fn(*mut c_void, *const GUID, u32, *mut c_void, f64, u32) -> HRESULT,
                scaler.ptr(), format, 0, null_mut(), 0.0, 0))?;
        }
        Ok(conv)
    }

    pub fn save_png(&self, path: &str, px: Pixels) -> Result<(), HrError> {
        let stream = create(|out| unsafe { com!(self.0, 14, fn(*mut *mut c_void) -> HRESULT, out) })?;
        let wpath = wide(path);
        unsafe {
            check(com!(stream, 15, fn(*const u16, u32) -> HRESULT, wpath.as_ptr(), GENERIC_WRITE))?; // InitializeFromFilename
        }
        let encoder = create(|out| unsafe {
            com!(self.0, 8, fn(*const GUID, *const GUID, *mut *mut c_void) -> HRESULT, &GUID_CONTAINER_FORMAT_PNG, null(), out)
        })?;
        unsafe {
            check(com!(encoder, 3, fn(*mut c_void, u32) -> HRESULT, stream.ptr(), WIC_BITMAP_ENCODER_NO_CACHE))?;
            let mut frame = null_mut();
            let mut props = null_mut();
            check(com!(encoder, 10, fn(*mut *mut c_void, *mut *mut c_void) -> HRESULT, &mut frame, &mut props))?;
            let frame = Com::from_raw(frame).ok_or(HrError(-1))?;
            let props = Com::from_raw(props);
            check(com!(frame, 3, fn(*mut c_void) -> HRESULT, props.as_ref().map_or(null_mut(), Com::ptr)))?;
            match px {
                Pixels::Bgra { w, h, stride, data } => {
                    check(com!(frame, 4, fn(u32, u32) -> HRESULT, w, h))?;
                    let mut fmt = GUID_WIC_PIXEL_FORMAT_32BPP_BGRA;
                    check(com!(frame, 6, fn(*mut GUID) -> HRESULT, &mut fmt))?;
                    check(com!(frame, 10, fn(u32, u32, u32, *const u8) -> HRESULT, h, stride, data.len() as u32, data.as_ptr()))?;
                }
                Pixels::Bitmap(bmp) => {
                    // IWICBitmapSource::GetSize.
                    let (mut w, mut h) = (0u32, 0u32);
                    check(com!(bmp, 3, fn(*mut u32, *mut u32) -> HRESULT, &mut w, &mut h))?;
                    check(com!(frame, 4, fn(u32, u32) -> HRESULT, w, h))?;
                    let mut fmt = GUID_WIC_PIXEL_FORMAT_32BPP_BGRA;
                    check(com!(frame, 6, fn(*mut GUID) -> HRESULT, &mut fmt))?;
                    check(com!(frame, 11, fn(*mut c_void, *const c_void) -> HRESULT, bmp.ptr(), null()))?; // WriteSource
                }
            }
            check(com!(frame, 12, fn() -> HRESULT))?;
            check(com!(encoder, 11, fn() -> HRESULT))?;
        }
        Ok(())
    }
}
