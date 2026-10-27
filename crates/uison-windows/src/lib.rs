//! Windows backend: Win32 (window, events), Direct2D (drawing), DirectWrite (text),
//! WIC (PNG). All hand-written FFI and COM, no libraries.

#![cfg(windows)]

/// A line to stderr in one call: `eprintln!` to unbuffered stderr writes
/// piece by piece of the format — ~1 ms per stats line when stderr is a file.
macro_rules! log {
    ($($t:tt)*) => {{
        use std::io::Write;
        let _ = std::io::stderr().write_all(format!("{}\n", format_args!($($t)*)).as_bytes());
    }};
}

mod canvas;
mod clock;
mod headless;
mod host;
pub mod com;
pub mod d2d;
pub mod ffi;
mod target;
mod text;
mod wic;
mod window;

pub use canvas::D2dCanvas;
pub use headless::Headless;
pub use text::Text;
pub use window::run;

/// Release is a Windows app without a console window; debug has a console (`UISON_STATS` visible).
/// Linker directive from the library (like `#pragma comment(linker, …)` in C): examples stay unchanged.
/// Not for `uison-c` (feature `gui-subsystem` off): `uison.dll` has its own entry point.
#[cfg(all(not(debug_assertions), feature = "gui-subsystem"))]
#[used]
#[unsafe(link_section = ".drectve")]
static SUBSYSTEM: [u8; 43] = *b" /SUBSYSTEM:WINDOWS /ENTRY:mainCRTStartup  ";

/// Without a console (release) there are no standard streams — when launched from a terminal
/// (`uibench`, `studio --bench`, `UISON_STATS=1`) write to the parent's console. A CRT
/// initializer: runs before `main`. Redirected output (`2> log.txt`) is left alone.
#[cfg(not(debug_assertions))]
#[used]
#[unsafe(link_section = ".CRT$XCU")]
static ATTACH_CONSOLE: extern "C" fn() = attach_console;

#[cfg(not(debug_assertions))]
extern "C" fn attach_console() {
    unsafe {
        if ffi::GetStdHandle(ffi::STD_OUTPUT_HANDLE).is_null() && ffi::GetStdHandle(ffi::STD_ERROR_HANDLE).is_null() {
            ffi::AttachConsole(ffi::ATTACH_PARENT_PROCESS);
        }
    }
}
