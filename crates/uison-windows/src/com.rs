//! COM by hand: an interface is a pointer to a pointer to a function table.
//! A method is a function by index in the table, with `this` as the first argument.
//! Indices follow method order in the SDK headers including inheritance
//! (`IUnknown` is 0..3), checked against the headers (`tools/vtable.cpp`, Task6.md, experiment 6).

use std::ffi::c_void;
use std::ptr::NonNull;

/// Call a method by index: `com!(obj, 17, fn(*const RectF, *mut c_void), &r, brush)`.
macro_rules! com {
    ($obj:expr, $n:literal, fn($($t:ty),*) $(-> $r:ty)? $(, $arg:expr)* $(,)?) => {{
        let o: &$crate::com::Com = &$obj;
        let f: unsafe extern "system" fn(*mut ::std::ffi::c_void $(, $t)*) $(-> $r)? = o.method($n);
        f(o.ptr() $(, $arg)*)
    }};
}

pub(crate) use com;

pub type HRESULT = i32;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct GUID(pub u32, pub u16, pub u16, pub [u8; 8]);

/// An owning reference to a COM object: `Release` in `Drop`.
#[repr(transparent)]
pub struct Com(NonNull<c_void>);

impl Com {
    /// # Safety
    /// `p` is a COM object whose reference passes to `Com`.
    pub unsafe fn from_raw(p: *mut c_void) -> Option<Com> {
        NonNull::new(p).map(Com)
    }

    pub fn ptr(&self) -> *mut c_void {
        self.0.as_ptr()
    }

    /// A function from the method table.
    ///
    /// # Safety
    /// `F` is the exact signature of method number `n` (`extern "system"`, `this` first).
    #[inline]
    pub unsafe fn method<F: Copy>(&self, n: usize) -> F {
        unsafe {
            let vtbl = *(self.0.as_ptr() as *const *const *const c_void);
            std::mem::transmute_copy(&*vtbl.add(n))
        }
    }

    /// `QueryInterface`.
    pub fn cast(&self, iid: &GUID) -> Option<Com> {
        let mut out = std::ptr::null_mut();
        let hr = unsafe { com!(self, 0, fn(*const GUID, *mut *mut c_void) -> HRESULT, iid, &mut out) };
        if hr < 0 { None } else { unsafe { Com::from_raw(out) } }
    }
}

impl Clone for Com {
    fn clone(&self) -> Com {
        unsafe { com!(self, 1, fn() -> u32) };
        Com(self.0)
    }
}

impl Drop for Com {
    fn drop(&mut self) {
        unsafe { com!(self, 2, fn() -> u32) };
    }
}

/// A COM / Win32 call error with an HRESULT code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HrError(pub HRESULT);

impl std::fmt::Display for HrError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "HRESULT 0x{:08X}", self.0 as u32)
    }
}

pub fn check(hr: HRESULT) -> Result<(), HrError> {
    if hr < 0 { Err(HrError(hr)) } else { Ok(()) }
}

/// Create an object via a `void**` out parameter.
pub fn create(f: impl FnOnce(*mut *mut c_void) -> HRESULT) -> Result<Com, HrError> {
    let mut out = std::ptr::null_mut();
    check(f(&mut out))?;
    unsafe { Com::from_raw(out) }.ok_or(HrError(-1))
}
