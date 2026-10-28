//! uison gallery: a window on macOS, a canvas in the browser (`tools/web.sh gallery`).

fn main() {
    // `--agent` / `--mcp`: an LLM agent drives the program without a window (built with `--features agent`).
    #[cfg(feature = "agent")]
    if let Some(run) = uison_agent::mode(&mut std::env::args_os().collect()) {
        run(gallery::app(), "uison gallery");
    }
    #[cfg(any(target_vendor = "apple", windows, target_os = "linux"))]
    if native::debug_modes() {
        return;
    }
    uison::run(gallery::app(), "uison gallery");
}

/// Native debug modes (screenshots, measurements).
#[cfg(any(target_vendor = "apple", windows, target_os = "linux"))]
#[allow(unused_imports)]
mod native {
    use std::time::{Duration, Instant};

    use uison::ui::Ui;
    use gallery::*;

    /// Gallery frame build time by stage (median of 50).
    fn bench() {
        use std::time::{Duration, Instant};
        let text = uison::Text::new();
        let mut ui = app();
        ui.render(&text); // warm up fonts and the line cache
        let mut runs: Vec<(Duration, uison::ui::Stats, Duration)> = (0..50)
            .map(|_| {
                let t = Instant::now();
                let src = ui.render(&text);
                let total = t.elapsed();
                let t = Instant::now();
                let _ = uison::Frame::parse(&src, &text).unwrap();
                (total, ui.stats, t.elapsed())
            })
            .collect();
        runs.sort_by_key(|r| r.0);
        let (total, s, parse) = runs[runs.len() / 2];
        let us = |d: Duration| d.as_secs_f64() * 1e6;
        println!(
            "gallery: {} nodes, {} primitives, {} B\n  expand {:>6.0} us\n  layout {:>6.0} us\n  emit   {:>6.0} us\n  total  {:>6.0} us (+ parse {:.0} us)",
            s.nodes, s.prims, s.bytes, us(s.expand), us(s.layout), us(s.emit), us(total), us(parse)
        );
    }

    pub fn debug_modes() -> bool {
        let args: Vec<String> = std::env::args().collect();
        if args.iter().any(|a| a == "--bench") {
            bench();
            return true;
        }
        // For a profiler: rebuild the frame in a loop for N seconds.
        if let Some(i) = args.iter().position(|a| a == "--spin") {
            let secs: u64 = args.get(i + 1).and_then(|s| s.parse().ok()).unwrap_or(5);
            let text = uison::Text::new();
            let mut ui = app();
            let end = std::time::Instant::now() + std::time::Duration::from_secs(secs);
            while std::time::Instant::now() < end {
                std::hint::black_box(ui.render(&text));
            }
            return true;
        }
        if let Some(i) = args.iter().position(|a| a == "--png") {
            let path = args.get(i + 1).expect("--png <file>");
            let mut ui = app();
            if args.iter().any(|a| a == "dark") {
                ui.set_theme("dark");
                ui.data.set("theme", "dark");
            }
            // `--scale 1.25`: fractional scale (Windows 125%).
            let scale = args.iter().position(|a| a == "--scale").and_then(|i| args.get(i + 1)?.parse().ok()).unwrap_or(2.0);
            let h = uison::Headless::new(ui, scale).unwrap_or_else(|e| panic!("{e}"));
            h.save_png(path, false).unwrap();
            return true;
        }
        false
    }
}
