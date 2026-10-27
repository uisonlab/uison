//! The program icon while it runs: the Dock on macOS, the title bar and taskbar on Windows,
//! `_NET_WM_ICON` on X11. PNG bytes, set before `run`. The icon of the file or package itself
//! (Finder, Explorer, the Linux desktop menu) belongs to packaging: an `.app` with an `.icns`,
//! an `.exe` resource, a `.desktop` file — see `examples/disk-usage`.

use std::sync::OnceLock;

static ICON: OnceLock<Vec<u8>> = OnceLock::new();

/// Set the icon (PNG) once, before `run`; later calls are ignored. On Windows an icon
/// resource in the `.exe` (id 1) takes precedence: it has every size.
pub fn set_icon(png: &[u8]) {
    let _ = ICON.set(png.to_vec());
}

/// The icon set by `set_icon`, for the backend.
pub fn icon() -> Option<&'static [u8]> {
    ICON.get().map(Vec::as_slice)
}
