//! Android backend: `android.graphics.Canvas` (Skia) and input via JNI.
//!
//! Java is just two small classes (`java/dev/uison`): a full-screen window and a View
//! that forwards events here (the on-screen keyboard needs an `InputConnection`,
//! and system bar insets need `WindowInsets`; neither is available from the NDK).
//!
//! An app is written as usual: `fn main() { uison::run(app(), "Title") }`.
//! The binary is built as `libmain.so` (`-C link-arg=-shared`, see `tools/android.sh`):
//! `System.loadLibrary` → [`JNI_OnLoad`] → `main()` → `run` remembers the app,
//! then Java calls the native methods of `UisonView` — as in the WASM backend.

#![cfg(target_os = "android")]

mod canvas;
mod host;
mod image;
mod java;
mod jni;
mod text;
mod view;

use std::cell::Cell;
use std::ffi::{c_char, c_int, c_void};

pub use view::run;

use crate::jni::{Env, JNIEnv};

thread_local! {
    /// JNIEnv of the main thread: all of uison runs on it (native methods, drawing).
    static ENV: Cell<*mut JNIEnv> = const { Cell::new(std::ptr::null_mut()) };
}

pub(crate) fn env() -> Env {
    Env(ENV.with(Cell::get))
}

pub(crate) fn set_env(e: *mut JNIEnv) {
    ENV.with(|c| c.set(e));
}

// ---------------------------------------------------------------- log

#[link(name = "log")]
unsafe extern "C" {
    fn __android_log_write(prio: c_int, tag: *const c_char, text: *const c_char) -> c_int;
}

/// Straight to logcat, bypassing the forwarding thread (it won't make it before the process exits).
pub(crate) fn log_now(msg: &str) {
    let line = std::ffi::CString::new(msg.replace('\0', " ")).unwrap_or_default();
    unsafe { __android_log_write(4, c"uison".as_ptr(), line.as_ptr()) };
}

unsafe extern "C" {
    fn pipe(fds: *mut c_int) -> c_int;
    fn dup2(old: c_int, new: c_int) -> c_int;
}

/// stdout and stderr → logcat (tag `uison`): `eprintln!` works as on other platforms.
fn redirect_output() {
    use std::io::{BufRead, BufReader};
    use std::os::fd::FromRawFd;
    let mut fds = [0 as c_int; 2];
    unsafe {
        if pipe(fds.as_mut_ptr()) != 0 {
            return;
        }
        dup2(fds[1], 1);
        dup2(fds[1], 2);
    }
    let read = unsafe { std::fs::File::from_raw_fd(fds[0]) };
    let _ = std::thread::Builder::new().name("uison-log".into()).spawn(move || {
        for line in BufReader::new(read).lines().map_while(Result::ok) {
            let line = std::ffi::CString::new(line.replace('\0', " ")).unwrap_or_default();
            unsafe { __android_log_write(4, c"uison".as_ptr(), line.as_ptr()) };
        }
    });
}

// ---------------------------------------------------------------- entry point

unsafe extern "C" {
    /// `main` of the app binary (it's built as a shared library).
    fn main(argc: c_int, argv: *const *const c_char) -> c_int;
}

const JNI_VERSION_1_6: i32 = 0x0001_0006;

/// `System.loadLibrary("main")`: output to logcat, the app's `main()` (registers it
/// via `uison::run`), Java classes and methods, native methods of `UisonView`.
///
/// # Safety
/// Called by the Java virtual machine with its `JavaVM*`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn JNI_OnLoad(vm: *mut *const *const c_void, _: *mut c_void) -> i32 {
    redirect_output();
    std::panic::set_hook(Box::new(|info| eprintln!("uison: {info}")));
    unsafe {
        // JavaVM → GetEnv (index 6 in JNIInvokeInterface).
        type GetEnv = unsafe extern "C" fn(*mut *const *const c_void, *mut *mut JNIEnv, i32) -> i32;
        let get_env: GetEnv = std::mem::transmute(*(*vm).add(6));
        let mut e: *mut JNIEnv = std::ptr::null_mut();
        if get_env(vm, &mut e, JNI_VERSION_1_6) != 0 {
            return -1;
        }
        set_env(e);
        // Folder with files from assets (UisonActivity): the program's relative paths start there.
        if let Some(dir) = std::env::var_os("UISON_FILES") {
            let _ = std::env::set_current_dir(dir);
        }
        main(0, std::ptr::null());
        let view = java::init(Env(e));
        view::register(Env(e), view);
    }
    JNI_VERSION_1_6
}
