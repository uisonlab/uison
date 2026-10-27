//! iOS backend: UIKit (scene, view, touches). Drawing and text are shared across Apple, from uison-apple.
//!
//! The app is written as usual: `fn main() { uison::run(app(), "Title") }`;
//! `run` goes into `UIApplicationMain`. The `.app` is built by `tools/ios.sh`.

#![cfg(target_os = "ios")]

mod host;
mod view;

pub use uison_apple::{Bitmap, CgCanvas, Headless, Text, ffi};
pub use view::run;
