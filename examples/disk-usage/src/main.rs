//! Disk Usage on uison: `disk-usage [--lang en|ru] [--console] [folder]`.
//!
//! `--console`: no window — frames to stdout, events from stdin as JSON lines, for `uison-player`.
//!
//! Headless (debug): `disk-usage --png <screenshot folder> <what to scan>` takes screenshots of the view,
//! tooltips, selection, zoom and menus; `disk-usage --bench <what to scan>` times a frame.

use std::ffi::OsString;
use std::path::PathBuf;

use disk_usage::Lang;

/// `--lang ru` anywhere on the command line; the rest stays in order.
fn take_lang(args: &mut Vec<OsString>) -> Lang {
    let Some(i) = args.iter().position(|a| a == "--lang") else { return Lang::default() };
    args.remove(i);
    if i < args.len() { Lang::from_code(&args.remove(i).to_string_lossy()) } else { Lang::default() }
}

fn take_flag(args: &mut Vec<OsString>, flag: &str) -> bool {
    let Some(i) = args.iter().position(|a| a == flag) else { return false };
    args.remove(i);
    true
}

fn main() {
    let mut args: Vec<OsString> = std::env::args_os().skip(1).collect();
    let lang = take_lang(&mut args);
    let console = take_flag(&mut args, "--console");
    #[cfg(any(target_vendor = "apple", windows, target_os = "linux"))]
    if native::debug_modes(&args, lang) {
        return;
    }
    // Old macOS versions pass a `-psn_…` process number to a program launched from Finder.
    let path = args.first().filter(|p| !p.is_empty() && !p.to_string_lossy().starts_with("-psn_")).map(PathBuf::from);
    #[cfg(any(target_os = "macos", target_os = "linux", windows))]
    if console {
        uison::run_console(disk_usage::app_console(path, lang), "Disk Usage");
    }
    let _ = console;
    // The running window's icon (Dock, taskbar); the file's own icon comes from the package (bundle.sh, build.rs).
    uison::set_icon(include_bytes!("../assets/icon-256.png"));
    uison::run(disk_usage::app(path, lang), "Disk Usage");
}

#[cfg(any(target_vendor = "apple", windows, target_os = "linux"))]
mod native {
    use std::ffi::OsString;
    use std::path::PathBuf;
    use std::time::Instant;

    use disk_usage::Lang;
    use uison::{Event, Headless, MouseButton, Point};

    /// A point on the first ring of the chart in a 1280×840 window (the left panel is 380 pt).
    fn ring0(angle: f32) -> Point {
        let (cx, cy, r) = (381.0 + 899.0 / 2.0, 47.0 + 764.0 / 2.0, 100.0);
        Point::new(cx + r * angle.sin(), cy - r * angle.cos())
    }

    pub fn debug_modes(args: &[OsString], lang: Lang) -> bool {
        let arg = |i: usize| args.get(i).map(PathBuf::from);
        match args.first().and_then(|a| a.to_str()) {
            Some("--png") => {
                let (Some(dir), Some(path)) = (arg(1), arg(2)) else {
                    eprintln!("disk-usage --png <dir> <path to scan>");
                    return true;
                };
                std::fs::create_dir_all(&dir).unwrap();
                let shot = |h: &Headless, name: &str| h.save_png(&dir.join(name).to_string_lossy(), false).unwrap();
                let h = Headless::new(disk_usage::app(None, lang), 2.0).unwrap();
                shot(&h, "1_start.png");
                let mut h = Headless::new(disk_usage::app_scanned(path, lang), 2.0).unwrap();
                shot(&h, "2_view.png");
                // Hovering the second list row highlights its sector.
                h.event(Event::MouseMove(Point::new(100.0, 219.0))).unwrap();
                shot(&h, "2b_row_hover.png");
                let p = ring0(0.25);
                h.event(Event::MouseMove(p)).unwrap();
                shot(&h, "3_hover.png");
                h.event(Event::MouseDown(p, MouseButton::Left)).unwrap();
                h.event(Event::MouseUp(p, MouseButton::Left)).unwrap();
                shot(&h, "4_selected.png");
                // A second click nearby makes a double click: zoom into the folder (0.35 s animation).
                h.event(Event::MouseDown(p, MouseButton::Left)).unwrap();
                h.event(Event::MouseUp(p, MouseButton::Left)).unwrap();
                std::thread::sleep(std::time::Duration::from_millis(120));
                h.event(Event::Tick).unwrap();
                shot(&h, "5_zooming.png");
                std::thread::sleep(std::time::Duration::from_millis(400));
                for _ in 0..3 {
                    h.event(Event::Tick).unwrap();
                }
                shot(&h, "6_zoomed.png");
                let q = ring0(1.2);
                h.event(Event::MouseMove(q)).unwrap();
                h.event(Event::MouseDown(q, MouseButton::Right)).unwrap();
                shot(&h, "7_menu.png");
                true
            }
            Some("--bench") => {
                let Some(path) = arg(1) else {
                    eprintln!("disk-usage --bench <path to scan>");
                    return true;
                };
                let t = Instant::now();
                let mut ui = disk_usage::app_scanned(path, lang);
                println!("scan + first data: {} ms", t.elapsed().as_millis());
                let text = uison::Text::new();
                ui.render(&text);
                let mut times: Vec<u128> = (0..20)
                    .map(|_| {
                        let t = Instant::now();
                        ui.render(&text);
                        t.elapsed().as_micros()
                    })
                    .collect();
                times.sort();
                let st = ui.stats;
                println!("frame: {} nodes, {} prims, {} KB, build {} us (median)", st.nodes, st.prims, st.bytes / 1024, times[10]);
                true
            }
            _ => false,
        }
    }
}
