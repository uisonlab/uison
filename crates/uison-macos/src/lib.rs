//! macOS backend: AppKit (window, events). Drawing and text are shared across Apple, from uison-apple.

#![cfg(target_os = "macos")]

mod host;
mod window;

pub use uison_apple::{Bitmap, CgCanvas, Headless, Text, ffi};
pub use window::run;
