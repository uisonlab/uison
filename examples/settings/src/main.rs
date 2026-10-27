//! uison settings: a window on macOS, a canvas in the browser (`tools/web.sh settings`).

fn main() {
    #[cfg(target_vendor = "apple")]
    if native::debug_modes() {
        return;
    }
    uison::run(settings::app(), "uison settings");
}

/// Native debug modes (screenshots, measurements).
#[cfg(target_vendor = "apple")]
#[allow(unused_imports)]
mod native {
    use std::time::{Duration, Instant};

    use uison::ui::Ui;
    use settings::*;

    pub fn debug_modes() -> bool {
        let args: Vec<String> = std::env::args().collect();
        if let Some(i) = args.iter().position(|a| a == "--png") {
            let h = uison::Headless::new(app(), 2.0).unwrap();
            h.save_png(&args[i + 1], false).unwrap();
            return true;
        }
        false
    }
}
