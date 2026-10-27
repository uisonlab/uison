//! System libraries are loaded with `dlopen` at run time, not linked: the build needs no
//! `-dev` packages, the binary depends only on libc, and a missing library gives a clear error.

use std::ffi::{CStr, c_char, c_int, c_void};

unsafe extern "C" {
    fn dlopen(file: *const c_char, mode: c_int) -> *mut c_void;
    fn dlsym(handle: *mut c_void, name: *const c_char) -> *mut c_void;
    fn dlerror() -> *const c_char;
}

const RTLD_NOW: c_int = 2;

/// The first library found (versioned `.so.N` names are always present).
pub fn open(names: &[&str]) -> Result<*mut c_void, String> {
    for name in names {
        let c = format!("{name}\0");
        let h = unsafe { dlopen(c.as_ptr().cast(), RTLD_NOW) };
        if !h.is_null() {
            return Ok(h);
        }
    }
    Err(format!("{} not found", names[0]))
}

/// Function address; `name` is NUL-terminated.
pub fn sym(h: *mut c_void, name: &str) -> Result<*mut c_void, String> {
    let p = unsafe { dlsym(h, name.as_ptr().cast()) };
    if p.is_null() {
        let e = unsafe { dlerror() };
        let msg = if e.is_null() { String::new() } else { unsafe { CStr::from_ptr(e) }.to_string_lossy().into_owned() };
        return Err(format!("{}: {msg}", name.trim_end_matches('\0')));
    }
    Ok(p)
}

/// A library's function table: a field and a method of the same name per function.
/// `opt fn` is optional (missing in old versions): an `Option` field, no method generated.
macro_rules! dylib {
    ($vis:vis struct $name:ident: [$($file:literal),+] {
        $(fn $f:ident($($a:ident: $t:ty),* $(,)?) $(-> $r:ty)?;)*
        $(opt fn $o:ident($($oa:ident: $ot:ty),* $(,)?) $(-> $or:ty)?;)*
    }) => {
        #[allow(non_snake_case)]
        $vis struct $name {
            $($f: unsafe extern "C" fn($($t),*) $(-> $r)?,)*
            $(pub $o: Option<unsafe extern "C" fn($($ot),*) $(-> $or)?>,)*
        }

        #[allow(clippy::too_many_arguments, clippy::missing_safety_doc)]
        impl $name {
            pub fn load() -> Result<$name, String> {
                let h = $crate::dl::open(&[$($file),+])?;
                unsafe {
                    Ok($name {
                        $($f: std::mem::transmute::<*mut std::ffi::c_void, unsafe extern "C" fn($($t),*) $(-> $r)?>(
                            $crate::dl::sym(h, concat!(stringify!($f), "\0"))?
                        ),)*
                        $($o: $crate::dl::sym(h, concat!(stringify!($o), "\0")).ok().map(|p| {
                            std::mem::transmute::<*mut std::ffi::c_void, unsafe extern "C" fn($($ot),*) $(-> $or)?>(p)
                        }),)*
                    })
                }
            }

            $(
                #[inline]
                pub unsafe fn $f(&self, $($a: $t),*) $(-> $r)? {
                    unsafe { (self.$f)($($a),*) }
                }
            )*
        }
    };
}

pub(crate) use dylib;
