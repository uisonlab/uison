//! uison todo: a window on macOS, a canvas in the browser (`tools/web.sh todo`).

fn main() {
    // `--agent` / `--mcp`: an LLM agent drives the program without a window (built with `--features agent`).
    #[cfg(feature = "agent")]
    if let Some(run) = uison_agent::mode(&mut std::env::args_os().collect()) {
        run(todo::app(), "uison todo");
    }
    #[cfg(target_vendor = "apple")]
    if native::debug_modes() {
        return;
    }
    uison::run(todo::app(), "uison todo");
}

/// Native debug modes (screenshots, measurements).
#[cfg(target_vendor = "apple")]
#[allow(unused_imports)]
mod native {
    use std::time::{Duration, Instant};

    use uison::ui::Ui;
    use todo::*;

    pub fn debug_modes() -> bool {
        let args: Vec<String> = std::env::args().collect();
        if let Some(i) = args.iter().position(|a| a == "--png") {
            let h = uison::Headless::new(app(), 2.0).unwrap();
            h.save_png(&args[i + 1], false).unwrap();
            return true;
        }
        if args.iter().any(|a| a == "--json") {
            println!("{}", screen().to_json());
            return true;
        }
        false
    }
}
