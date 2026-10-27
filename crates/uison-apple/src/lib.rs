//! Shared code for Apple platforms (macOS, iOS): CoreGraphics (drawing), CoreText (text),
//! ImageIO (PNG) and hand-written FFI to Objective-C. Windows and events live in uison-macos / uison-ios.

#![cfg(target_vendor = "apple")]

pub mod ffi;

mod bitmap;
mod canvas;
mod headless;
mod image;
mod text;

pub use bitmap::{Bitmap, save_image_png};
pub use canvas::CgCanvas;
pub use headless::Headless;
pub use text::Text;
