//! Hand-written FFI: Objective-C runtime, CoreFoundation, CoreGraphics, CoreText, ImageIO.
//! Only what is actually used. Targets are arm64 (macOS, iOS, iOS simulator):
//! structs are returned through plain `objc_msgSend`, without `_stret`.

#![allow(non_upper_case_globals, non_snake_case, clippy::upper_case_acronyms)]

use std::ffi::{c_char, c_void};

pub type Id = *mut c_void;
pub type Sel = *const c_void;
pub type Class = *mut c_void;
pub type Ivar = *const c_void;
pub const NIL: Id = std::ptr::null_mut();

pub type CFTypeRef = *const c_void;
pub type CFStringRef = *const c_void;
pub type CGContextRef = *mut c_void;
pub type CTFontRef = *const c_void;
pub type CTLineRef = *const c_void;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CGPoint {
    pub x: f64,
    pub y: f64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CGSize {
    pub width: f64,
    pub height: f64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CGRect {
    pub origin: CGPoint,
    pub size: CGSize,
}

impl CGRect {
    pub fn new(x: f64, y: f64, w: f64, h: f64) -> CGRect {
        CGRect { origin: CGPoint { x, y }, size: CGSize { width: w, height: h } }
    }
}

impl From<uison_core::Rect> for CGRect {
    fn from(r: uison_core::Rect) -> CGRect {
        CGRect::new(r.x as f64, r.y as f64, r.w as f64, r.h as f64)
    }
}

impl From<CGRect> for uison_core::Rect {
    fn from(r: CGRect) -> uison_core::Rect {
        uison_core::Rect::new(r.origin.x as f32, r.origin.y as f32, r.size.width as f32, r.size.height as f32)
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct CGAffineTransform {
    pub a: f64,
    pub b: f64,
    pub c: f64,
    pub d: f64,
    pub tx: f64,
    pub ty: f64,
}

// ---------------------------------------------------------------- objc runtime

#[link(name = "objc")]
unsafe extern "C" {
    pub fn objc_getClass(name: *const c_char) -> Class;
    pub fn sel_registerName(name: *const c_char) -> Sel;
    pub fn objc_msgSend();
    pub fn objc_msgSendSuper();
    pub fn objc_getProtocol(name: *const c_char) -> *mut c_void;
    pub fn class_addProtocol(cls: Class, protocol: *mut c_void) -> bool;
    pub fn objc_allocateClassPair(superclass: Class, name: *const c_char, extra: usize) -> Class;
    pub fn objc_registerClassPair(cls: Class);
    pub fn class_addMethod(cls: Class, name: Sel, imp: *const c_void, types: *const c_char) -> bool;
    pub fn class_addIvar(cls: Class, name: *const c_char, size: usize, align: u8, types: *const c_char) -> bool;
    pub fn class_getInstanceVariable(cls: Class, name: *const c_char) -> Ivar;
    pub fn ivar_getOffset(ivar: Ivar) -> isize;
    pub fn objc_autoreleasePoolPush() -> *mut c_void;
    pub fn objc_autoreleasePoolPop(pool: *mut c_void);
}

/// Message arguments: a tuple is cast to the matching `objc_msgSend` signature.
pub trait Args {
    /// # Safety
    /// The signature (argument types and `R`) must match the method.
    unsafe fn send<R>(self, obj: Id, sel: Sel) -> R;
    /// # Safety
    /// Same as [`Args::send`].
    unsafe fn send_super<R>(self, sup: *const Super, sel: Sel) -> R;
}

macro_rules! impl_args {
    ($($a:ident: $t:ident),*) => {
        impl<$($t),*> Args for ($($t,)*) {
            unsafe fn send<R>(self, obj: Id, sel: Sel) -> R {
                let ($($a,)*) = self;
                unsafe {
                    let f: unsafe extern "C" fn(Id, Sel $(, $t)*) -> R =
                        std::mem::transmute(objc_msgSend as *const ());
                    f(obj, sel $(, $a)*)
                }
            }
            unsafe fn send_super<R>(self, sup: *const Super, sel: Sel) -> R {
                let ($($a,)*) = self;
                unsafe {
                    let f: unsafe extern "C" fn(*const Super, Sel $(, $t)*) -> R =
                        std::mem::transmute(objc_msgSendSuper as *const ());
                    f(sup, sel $(, $a)*)
                }
            }
        }
    };
}

impl_args!();
impl_args!(a: A);
impl_args!(a: A, b: B);
impl_args!(a: A, b: B, c: C);
impl_args!(a: A, b: B, c: C, d: D);

/// Receiver of a message to the superclass (`struct objc_super`).
#[repr(C)]
pub struct Super {
    pub receiver: Id,
    pub class: Class,
}

/// Call the method implementation in superclass `class` (like `[super …]`).
///
/// # Safety
/// See [`Args::send`]; `class` is the superclass of `obj`'s class.
pub unsafe fn msg_super<R, A: Args>(obj: Id, class: Class, sel: Sel, args: A) -> R {
    let sup = Super { receiver: obj, class };
    unsafe { args.send_super(&sup, sel) }
}

/// Send a message. `msg::<R>(obj, sel!("name:"), (arg,))`.
///
/// # Safety
/// See [`Args::send`].
#[inline]
pub unsafe fn msg<R, A: Args>(obj: Id, sel: Sel, args: A) -> R {
    unsafe { args.send(obj, sel) }
}

/// A selector cached in a static at the call site.
#[macro_export]
macro_rules! sel {
    ($s:literal) => {{
        use std::sync::atomic::{AtomicPtr, Ordering};
        static S: AtomicPtr<std::ffi::c_void> = AtomicPtr::new(std::ptr::null_mut());
        let mut p = S.load(Ordering::Relaxed);
        if p.is_null() {
            p = $crate::ffi::register_sel(concat!($s, "\0")) as *mut _;
            S.store(p, Ordering::Relaxed);
        }
        p as $crate::ffi::Sel
    }};
}

/// An Objective-C class by name.
#[macro_export]
macro_rules! class {
    ($s:literal) => {
        $crate::ffi::get_class(concat!($s, "\0"))
    };
}

pub use crate::{class, sel};

/// `name` must end with `\0`.
pub fn register_sel(name: &str) -> Sel {
    debug_assert!(name.ends_with('\0'));
    unsafe { sel_registerName(name.as_ptr().cast()) }
}

/// `name` must end with `\0`.
pub fn get_class(name: &str) -> Class {
    debug_assert!(name.ends_with('\0'));
    unsafe { objc_getClass(name.as_ptr().cast()) }
}

/// Release an object obtained via alloc/init/new/copy.
///
/// # Safety
/// `obj` is a live Objective-C object owned by the caller.
pub unsafe fn release(obj: Id) {
    unsafe { msg::<(), _>(obj, sel!("release"), ()) }
}

/// A new NSString (+1, owned by the caller).
pub fn nsstring(s: &str) -> Id {
    unsafe {
        let o: Id = msg(class!("NSString"), sel!("alloc"), ());
        msg(o, sel!("initWithBytes:length:encoding:"), (s.as_ptr(), s.len(), 4usize))
    }
}

// ---------------------------------------------------------------- CoreFoundation

pub const kCFStringEncodingUTF8: u32 = 0x0800_0100;

#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    pub fn CFRelease(cf: CFTypeRef);
    pub fn CFStringCreateWithBytes(
        alloc: CFTypeRef,
        bytes: *const u8,
        len: isize,
        encoding: u32,
        external: bool,
    ) -> CFStringRef;
    pub fn CFDictionaryCreate(
        alloc: CFTypeRef,
        keys: *const CFTypeRef,
        values: *const CFTypeRef,
        n: isize,
        key_cb: *const c_void,
        value_cb: *const c_void,
    ) -> CFTypeRef;
    pub fn CFAttributedStringCreate(alloc: CFTypeRef, s: CFStringRef, attrs: CFTypeRef) -> CFTypeRef;
    pub fn CFURLCreateFromFileSystemRepresentation(
        alloc: CFTypeRef,
        buf: *const u8,
        len: isize,
        is_dir: bool,
    ) -> CFTypeRef;
    pub static kCFBooleanTrue: CFTypeRef;
    pub fn CFDataCreate(alloc: CFTypeRef, bytes: *const u8, len: isize) -> CFTypeRef;
    pub fn CFNumberCreate(alloc: CFTypeRef, kind: isize, value: *const c_void) -> CFTypeRef;
    // Only the addresses of the callback structs are needed.
    pub static kCFTypeDictionaryKeyCallBacks: u8;
    pub static kCFTypeDictionaryValueCallBacks: u8;
}

pub fn cfstring(s: &str) -> CFStringRef {
    unsafe {
        CFStringCreateWithBytes(std::ptr::null(), s.as_ptr(), s.len() as isize, kCFStringEncodingUTF8, false)
    }
}

// ---------------------------------------------------------------- CoreGraphics

pub const kCGLineJoinRound: i32 = 1;
pub const kCGLineCapButt: i32 = 0;
pub const kCGLineCapRound: i32 = 1;
pub const kCGImageAlphaPremultipliedLast: u32 = 1;

#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    pub fn CGContextSaveGState(c: CGContextRef);
    pub fn CGContextRestoreGState(c: CGContextRef);
    pub fn CGContextClipToRect(c: CGContextRef, r: CGRect);
    pub fn CGContextSetRGBFillColor(c: CGContextRef, r: f64, g: f64, b: f64, a: f64);
    pub fn CGContextSetRGBStrokeColor(c: CGContextRef, r: f64, g: f64, b: f64, a: f64);
    pub fn CGContextFillRect(c: CGContextRef, r: CGRect);
    pub fn CGContextStrokeRectWithWidth(c: CGContextRef, r: CGRect, w: f64);
    pub fn CGContextFillEllipseInRect(c: CGContextRef, r: CGRect);
    pub fn CGContextStrokeEllipseInRect(c: CGContextRef, r: CGRect);
    pub fn CGContextSetLineWidth(c: CGContextRef, w: f64);
    pub fn CGContextSetLineJoin(c: CGContextRef, join: i32);
    pub fn CGContextSetLineCap(c: CGContextRef, cap: i32);
    pub fn CGContextBeginPath(c: CGContextRef);
    pub fn CGContextMoveToPoint(c: CGContextRef, x: f64, y: f64);
    pub fn CGContextAddLineToPoint(c: CGContextRef, x: f64, y: f64);
    pub fn CGContextAddArcToPoint(c: CGContextRef, x1: f64, y1: f64, x2: f64, y2: f64, r: f64);
    pub fn CGContextClosePath(c: CGContextRef);
    pub fn CGContextFillPath(c: CGContextRef);
    pub fn CGContextStrokePath(c: CGContextRef);
    pub fn CGContextSetTextMatrix(c: CGContextRef, t: CGAffineTransform);
    pub fn CGContextSetTextPosition(c: CGContextRef, x: f64, y: f64);
    pub fn CGContextTranslateCTM(c: CGContextRef, tx: f64, ty: f64);
    pub fn CGContextScaleCTM(c: CGContextRef, sx: f64, sy: f64);
    pub fn CGContextGetUserSpaceToDeviceSpaceTransform(c: CGContextRef) -> CGAffineTransform;
    pub fn CGContextRelease(c: CGContextRef);
    pub fn CGColorSpaceCreateDeviceRGB() -> CFTypeRef;
    pub fn CGColorSpaceRelease(cs: CFTypeRef);
    pub fn CGBitmapContextCreate(
        data: *mut c_void,
        w: usize,
        h: usize,
        bits_per_component: usize,
        bytes_per_row: usize,
        space: CFTypeRef,
        info: u32,
    ) -> CGContextRef;
    pub fn CGBitmapContextCreateImage(c: CGContextRef) -> CFTypeRef;
    pub fn CGBitmapContextGetData(c: CGContextRef) -> *mut u8;
    pub fn CGBitmapContextGetBytesPerRow(c: CGContextRef) -> usize;
    pub fn CGImageRelease(img: CFTypeRef);
}

pub const CGRectNull: CGRect =
    CGRect { origin: CGPoint { x: f64::INFINITY, y: f64::INFINITY }, size: CGSize { width: 0.0, height: 0.0 } };

// ---------------------------------------------------------------- CoreText

pub const kCTFontUIFontUserFixedPitch: u32 = 1;
pub const kCTFontUIFontSystem: u32 = 2;
pub const kCTFontUIFontEmphasizedSystem: u32 = 3;
pub const kCTFontItalicTrait: u32 = 1 << 0;
pub const kCTFontBoldTrait: u32 = 1 << 1;

#[link(name = "CoreText", kind = "framework")]
unsafe extern "C" {
    pub fn CTFontCreateWithName(name: CFStringRef, size: f64, matrix: *const CGAffineTransform) -> CTFontRef;
    pub fn CTFontCreateUIFontForLanguage(kind: u32, size: f64, lang: CFStringRef) -> CTFontRef;
    pub fn CTFontCreateCopyWithSymbolicTraits(
        font: CTFontRef,
        size: f64,
        matrix: *const CGAffineTransform,
        value: u32,
        mask: u32,
    ) -> CTFontRef;
    pub fn CTLineCreateWithAttributedString(s: CFTypeRef) -> CTLineRef;
    pub fn CTLineGetTypographicBounds(line: CTLineRef, ascent: *mut f64, descent: *mut f64, leading: *mut f64) -> f64;
    pub fn CTLineDraw(line: CTLineRef, c: CGContextRef);
    pub static kCTFontAttributeName: CFStringRef;
    pub static kCTForegroundColorFromContextAttributeName: CFStringRef;
}

// ---------------------------------------------------------------- ImageIO

#[link(name = "ImageIO", kind = "framework")]
unsafe extern "C" {
    pub fn CGImageDestinationCreateWithURL(url: CFTypeRef, kind: CFStringRef, count: usize, opts: CFTypeRef) -> CFTypeRef;
    pub fn CGImageDestinationAddImage(dest: CFTypeRef, img: CFTypeRef, props: CFTypeRef);
    pub fn CGImageDestinationFinalize(dest: CFTypeRef) -> bool;
    pub fn CGImageSourceCreateWithData(data: CFTypeRef, opts: CFTypeRef) -> CFTypeRef;
    pub fn CGImageSourceCreateThumbnailAtIndex(src: CFTypeRef, index: usize, opts: CFTypeRef) -> CFTypeRef;
    pub static kCGImageSourceCreateThumbnailFromImageAlways: CFStringRef;
    pub static kCGImageSourceThumbnailMaxPixelSize: CFStringRef;
    pub static kCGImageSourceCreateThumbnailWithTransform: CFStringRef;
}

#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    pub fn CGImageGetWidth(img: CFTypeRef) -> usize;
    pub fn CGImageGetHeight(img: CFTypeRef) -> usize;
    pub fn CGImageCreateWithImageInRect(img: CFTypeRef, r: CGRect) -> CFTypeRef;
    pub fn CGContextDrawImage(c: CGContextRef, r: CGRect, img: CFTypeRef);
    pub fn CGContextClip(c: CGContextRef);
    pub fn CGContextSetInterpolationQuality(c: CGContextRef, q: i32);
}

/// kCFNumberSInt32Type.
pub const CF_NUMBER_SINT32: isize = 3;
/// kCGInterpolationHigh.
pub const INTERPOLATION_HIGH: i32 = 3;
