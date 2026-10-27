//! uison list10k: a window on macOS, a canvas in the browser (`tools/web.sh list10k`).

fn main() {
    #[cfg(target_vendor = "apple")]
    if native::debug_modes() {
        return;
    }
    uison::run(list10k::app(), "uison list10k");
}

#[cfg(target_vendor = "apple")]
mod native {
    use std::time::{Duration, Instant};

    use list10k::sized;
    use uison::{Event, Point};

    fn bench(w: f32, h: f32) {
        let text = uison::Text::new();
        let mut ui = sized(w, h);
        ui.render(&text);
        let p = Point::new(w / 2.0, h / 2.0);
        let mut steps: Vec<Duration> = Vec::new();
        let mut compile = Duration::ZERO;
        for i in 0..400 {
            let dy = if i < 300 { -40.0 } else { 120.0 };
            let t = Instant::now();
            let src = uison::App::on_event(&mut ui, &Event::Scroll(p, 0.0, dy), Some("list"), &text);
            if let Some(src) = src {
                compile += ui.stats.expand + ui.stats.layout + ui.stats.emit;
                let _ = uison::Frame::parse(&src, &text).unwrap();
                steps.push(t.elapsed());
            }
        }
        steps.sort();
        let n = steps.len();
        let us = |d: Duration| d.as_secs_f64() * 1e6;
        println!(
            "list10k {w}x{h}: {n} scroll steps, {} nodes, {} primitives per frame\n  app (expand+layout+emit) avg {:.0} us\n  app + parse median {:.0} us, p95 {:.0} us",
            ui.stats.nodes,
            ui.stats.prims,
            us(compile) / n as f64,
            us(steps[n / 2]),
            us(steps[n * 95 / 100])
        );
    }

    pub fn debug_modes() -> bool {
        let args: Vec<String> = std::env::args().collect();
        let num = |i: usize, d: f32| args.get(i).and_then(|s| s.parse().ok()).unwrap_or(d);
        if let Some(i) = args.iter().position(|a| a == "--bench") {
            bench(num(i + 1, 420.0), num(i + 2, 720.0));
            return true;
        }
        false
    }
}
