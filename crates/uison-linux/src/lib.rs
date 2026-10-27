//! Linux backend: Wayland (own protocol code) or X11, drawing with Cairo, text with Pango.
//! System libraries are loaded with `dlopen`: one binary for any desktop.

#![cfg(target_os = "linux")]

/// One line to stderr in a single call (as in the other backends).
macro_rules! log {
    ($($t:tt)*) => {{
        use std::io::Write;
        let _ = std::io::stderr().write_all(format!("{}\n", format_args!($($t)*)).as_bytes());
    }};
}

mod canvas;
mod cursor;
mod dbus;
mod decor;
mod dl;
pub mod gfx;
mod headless;
mod image;
mod kinetic;
mod shell;
mod sys;
mod text;
mod wire;
mod wl;
mod x11;
mod xkb;

pub use headless::Headless;
pub use text::Text;

/// Cairo and Pango could be loaded: text can be measured with real fonts (a server without a desktop has none).
pub fn has_text() -> bool {
    gfx::try_gfx().is_ok()
}

/// Open a window with the app and enter the event loop. Wayland if available
/// (`WAYLAND_DISPLAY` or a `wayland-0` socket), otherwise X11 (`DISPLAY`);
/// `UISON_LINUX=x11|wayland` picks one manually.
pub fn run(app: impl uison_core::App + 'static, title: &str) {
    let forced = std::env::var("UISON_LINUX").unwrap_or_default();
    let wayland = match forced.as_str() {
        "x11" => false,
        "wayland" => true,
        _ => {
            let dir = std::env::var("XDG_RUNTIME_DIR").unwrap_or_default();
            let name = std::env::var("WAYLAND_DISPLAY").unwrap_or_else(|_| "wayland-0".into());
            let path = if name.starts_with('/') { name } else { format!("{dir}/{name}") };
            std::path::Path::new(&path).exists() || std::env::var_os("DISPLAY").is_none()
        }
    };
    if wayland { wl::run(app, title) } else { x11::run(app, title) }
}
