//! C ABI of the framework: `include/uison.h` (C99) and `include/uison.hpp` (C++17 on top of it).
//!
//! Rules at the boundary:
//! - input strings are NUL-terminated UTF-8 (`NULL` is an empty string);
//! - output strings belong to the library (an arena): in a handler they live until it returns,
//!   for `Ui` data before launch — until `uison_ui_free`/`uison_run`;
//! - errors are `NULL`/`false` plus a message in `uison_last_error()` (per thread);
//! - a panic never crosses the boundary: a message and `abort`.

#![allow(non_camel_case_types, clippy::missing_safety_doc)]

mod app;
mod cx;
mod data;
mod fields;
mod ui;

use std::cell::RefCell;
use std::ffi::{CStr, CString, c_char, c_void};
use std::panic::{AssertUnwindSafe, catch_unwind};

pub use cx::uison_cx;
pub use data::uison_data;
pub use fields::uison_field;
pub use ui::{uison_handle, uison_ui};

/// ABI version: changes only on incompatible changes to `uison.h`.
pub const ABI_VERSION: u32 = 1;

pub type uison_handler = Option<unsafe extern "C" fn(cx: *mut uison_cx, user: *mut c_void)>;
pub type uison_derive = Option<unsafe extern "C" fn(d: *mut uison_data, user: *mut c_void)>;
pub type uison_drop = Option<unsafe extern "C" fn(user: *mut c_void)>;

#[unsafe(no_mangle)]
pub extern "C" fn uison_abi_version() -> u32 {
    ABI_VERSION
}

thread_local! {
    static LAST_ERROR: RefCell<Option<CString>> = const { RefCell::new(None) };
}

pub(crate) fn set_error(msg: impl std::fmt::Display) {
    let c = CString::new(msg.to_string().replace('\0', " ")).unwrap_or_default();
    LAST_ERROR.with(|e| *e.borrow_mut() = Some(c));
}

pub(crate) fn clear_error() {
    LAST_ERROR.with(|e| *e.borrow_mut() = None);
}

/// Message of this thread's last error, or `NULL`. Valid until the next failing call.
#[unsafe(no_mangle)]
pub extern "C" fn uison_last_error() -> *const c_char {
    LAST_ERROR.with(|e| e.borrow().as_ref().map_or(std::ptr::null(), |c| c.as_ptr()))
}

/// Run the body of an exported function: a panic doesn't escape into C.
pub(crate) fn guard<R>(f: impl FnOnce() -> R) -> R {
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(r) => r,
        Err(_) => {
            eprintln!("uison: panic inside the C API — aborting");
            std::process::abort()
        }
    }
}

/// String from C: `NULL` is empty; invalid UTF-8 is replaced character by character.
pub(crate) unsafe fn text<'a>(p: *const c_char) -> std::borrow::Cow<'a, str> {
    if p.is_null() {
        return "".into();
    }
    String::from_utf8_lossy(unsafe { CStr::from_ptr(p) }.to_bytes())
}

/// Strings handed out to C: live as long as the arena.
#[derive(Default)]
pub(crate) struct Arena(Vec<CString>);

impl Arena {
    pub fn keep(&mut self, s: impl Into<Vec<u8>>) -> *const c_char {
        let mut v: Vec<u8> = s.into();
        v.retain(|&b| b != 0);
        self.0.push(CString::new(v).unwrap_or_default());
        self.0.last().unwrap().as_ptr()
    }
}

/// User pointer with a release function: `drop` is called when it's no longer
/// needed (this is how C++ keeps a `std::function` without leaks).
pub(crate) struct User {
    pub ptr: *mut c_void,
    pub drop: uison_drop,
}

impl Drop for User {
    fn drop(&mut self) {
        if let Some(d) = self.drop {
            unsafe { d(self.ptr) }
        }
    }
}
