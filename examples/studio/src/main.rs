//! uison Studio: a window on macOS, a canvas in the browser (`tools/web.sh studio`).
//!
//! Native: `studio --png <dir> [dark]` takes headless screenshots of every section;
//! `studio --bench` times frame rebuilds and data recomputation.

fn main() {
    #[cfg(any(target_vendor = "apple", windows, target_os = "linux"))]
    if native::debug_modes() {
        return;
    }
    uison::run(studio::app(), "uison Studio");
}

#[cfg(any(target_vendor = "apple", windows, target_os = "linux"))]
mod native {
    use std::time::{Duration, Instant};

    use studio::app;

    const SCREENS: [&str; 6] = ["inbox", "tasks", "contacts", "stats", "form", "settings"];

    /// `true`: a debug mode ran, no window needed.
    pub fn debug_modes() -> bool {
        let args: Vec<String> = std::env::args().collect();
        if let Some(i) = args.iter().position(|a| a == "--png") {
            let dir = args.get(i + 1).map_or("studio_png", String::as_str);
            std::fs::create_dir_all(dir).unwrap();
            for (n, screen) in SCREENS.iter().enumerate() {
                let mut ui = app();
                let dark = args.iter().any(|a| a == "dark");
                ui.update(|d| {
                    d.set("nav", *screen);
                    d.set("inbox.sel", 0);
                    if dark {
                        d.set("settings.theme", "dark");
                    }
                });
                if dark {
                    ui.set_theme("dark");
                }
                let h = uison::Headless::new(ui, 2.0).unwrap();
                h.save_png(&format!("{dir}/{}_{screen}.png", n + 1), false).unwrap();
            }
            return true;
        }
        if args.iter().any(|a| a == "--bench") {
            let text = uison::Text::new();
            // Derived data recomputation: a message change (filter over 2000) and a plain model change.
            let mut ui = app();
            let t = Instant::now();
            for i in 0..20 {
                ui.update(|d| d.set("inbox.rev", 1000 + i));
            }
            let rev = t.elapsed() / 20;
            let t = Instant::now();
            for i in 0..20 {
                ui.update(|d| d.set("form.age", 20 + i));
            }
            println!("derive: inbox changed {:.0} us, other change {:.0} us", rev.as_secs_f64() * 1e6, (t.elapsed() / 20).as_secs_f64() * 1e6);
            println!("screen     nodes prims   bytes  build(us)");
            for screen in SCREENS {
                let mut ui = app();
                ui.update(|d| {
                    d.set("nav", screen);
                    d.set("inbox.sel", 0);
                });
                ui.render(&text);
                let mut times: Vec<Duration> = (0..30)
                    .map(|_| {
                        let t = Instant::now();
                        ui.render(&text);
                        t.elapsed()
                    })
                    .collect();
                times.sort();
                let st = ui.stats;
                println!("{screen:<10} {:>5} {:>5} {:>7}  {:>8.0}", st.nodes, st.prims, st.bytes, times[15].as_secs_f64() * 1e6);
            }
            return true;
        }
        false
    }
}
