//! Windows backend drawing with Vulkan: Win32 (window, events), Vulkan (drawing), DirectWrite
//! (text), WIC (images, PNG). All hand-written FFI and COM, no libraries; `vulkan-1.dll` is loaded
//! at run time, so building needs no Vulkan SDK.
//!
//! The window, events and text measurement are a copy of `uison-windows` (the two crates do not
//! depend on each other); only drawing differs. Chosen with `uison::set_graphics(Graphics::Vulkan)`
//! (the `uison` crate's `vulkan` feature). Without a Vulkan driver `run` exits with an error —
//! there is no fallback to Direct2D.
//!
//! How a frame is drawn: `gpu` (device, swapchain, a persistent target image), `canvas` (primitives
//! as instances of one shader quad; text lines and polygon masks in an atlas), `render` (one frame).
//! Shaders: `shaders/*.vert`, `*.frag`, compiled into the `.spv` files next to them by
//! `tools/vulkan-shaders.ps1`. `UISON_VULKAN_VALIDATE=1` turns on the validation layer (Vulkan SDK).
//!
//! The release-build window subsystem and the console attach come from `uison-windows`, which the
//! `uison` crate always links on Windows.

#![cfg(windows)]
// FFI names as in the Windows and Vulkan headers (HRESULT, GUID).
#![allow(clippy::upper_case_acronyms)]

/// A line to stderr in one call: `eprintln!` to unbuffered stderr writes
/// piece by piece of the format — ~1 ms per stats line when stderr is a file.
macro_rules! log {
    ($($t:tt)*) => {{
        use std::io::Write;
        let _ = std::io::stderr().write_all(format!("{}\n", format_args!($($t)*)).as_bytes());
    }};
}

mod atlas;
mod canvas;
mod clock;
mod com;
mod d2d;
mod ffi;
mod gpu;
mod host;
mod lines;
mod raster;
mod render;
mod text;
mod vk;
mod wic;
mod window;

pub use text::Text;
pub use window::run;
