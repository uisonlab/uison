//! The umbrella crate: common types, the frame and the backend of the current platform.

pub use uison_core::*;
pub use uison_frame::event;
pub use uison_frame::{Damage, Frame, Item, ItemWriter, Runtime, Shape, Update, Writer, cursor_at, diff, hit_areas, hit_test, paint};
pub use uison_json as json;
/// The element library: complex JSON, templates, bindings, the Rust DSL.
pub use uison_ui as ui;
/// `#[derive(Data)]`: struct ↔ data (`IntoJson` + `FromJson`); `#[data(rename = "…")]`, `#[data(skip)]`.
pub use uison_derive::Data;

/// Everything a program needs in one `use uison::prelude::*;`: the DSL (`column`, `text`, `button`…), `Ui`, `Cx`,
/// data (`Data`, `IntoJson`, `FromJson`, `obj`, `#[derive(Data)]`), `source!`, `run`.
pub mod prelude {
    pub use crate::run;
    pub use uison_derive::Data;
    pub use uison_ui::dsl::*;
    pub use uison_ui::{Cx, Data, FromJson, Handle, IntoJson, Json, Ui, obj, source};
}

#[cfg(target_os = "macos")]
pub use uison_macos::{Bitmap, Headless, Text, run};

#[cfg(target_os = "ios")]
pub use uison_ios::{Bitmap, Headless, Text, run};

#[cfg(target_os = "android")]
pub use uison_android::run;

#[cfg(target_arch = "wasm32")]
pub use uison_web::{log, run};

#[cfg(windows)]
pub use uison_windows::{Headless, Text};

/// How windows draw. Chosen once with [`set_graphics`], before `run`; the default is the
/// platform's own 2D API.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum Graphics {
    /// Direct2D on Windows, CoreGraphics on Apple, Cairo on Linux, `android.graphics.Canvas`, canvas 2D.
    #[default]
    Native,
    /// Vulkan (Windows, feature `vulkan`). Strict: without a Vulkan driver `run` exits with an
    /// error, it does not fall back to Direct2D.
    #[cfg(all(windows, feature = "vulkan"))]
    Vulkan,
}

static GRAPHICS: std::sync::OnceLock<Graphics> = std::sync::OnceLock::new();

/// Choose how windows draw: once, before `run`; later calls are ignored.
///
/// ```no_run
/// # #[cfg(all(windows, feature = "vulkan"))]
/// uison::set_graphics(uison::Graphics::Vulkan);
/// ```
pub fn set_graphics(g: Graphics) {
    let _ = GRAPHICS.set(g);
}

/// The choice made with [`set_graphics`].
pub fn graphics() -> Graphics {
    GRAPHICS.get().copied().unwrap_or_default()
}

/// Open a window with the app and enter the message loop, drawing as [`set_graphics`] chose.
#[cfg(windows)]
pub fn run(app: impl App + 'static, title: &str) {
    match graphics() {
        #[cfg(feature = "vulkan")]
        Graphics::Vulkan => uison_vulkan_windows::run(app, title),
        Graphics::Native => uison_windows::run(app, title),
    }
}

#[cfg(target_os = "linux")]
pub use uison_linux::{Headless, Text, run};

/// Console mode instead of a window: frames go out to stdout and events come in from stdin, one JSON
/// message per line (`uison_frame::console`); a player (`uison-player`) shows the program. Text is
/// measured with this platform's fonts. Returns when the other side closes the pipe.
#[cfg(any(target_os = "macos", target_os = "linux", windows))]
pub fn run_console(app: impl App + 'static, title: &str) -> ! {
    // `UISON_TEXT=estimate`, or a Linux server without Cairo and Pango: metrics by character widths.
    #[cfg(target_os = "linux")]
    let fonts = uison_linux::has_text();
    #[cfg(not(target_os = "linux"))]
    let fonts = true;
    if !fonts || std::env::var("UISON_TEXT").is_ok_and(|v| v == "estimate") {
        if !fonts {
            eprintln!("uison: no Cairo and Pango here — text is measured approximately (the player draws it with its fonts)");
        }
        std::process::exit(uison_frame::console::run(Box::new(app), &uison_frame::console::Estimate, title))
    }
    let text = Text::new();
    std::process::exit(uison_frame::console::run(Box::new(app), &text, title))
}
