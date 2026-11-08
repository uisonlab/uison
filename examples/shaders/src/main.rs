//! The `shader` item: rectangles painted by programs in uison's shading language, animated by
//! the item's `time`. A frame written by hand (`App` + `Writer`), ticking with the display.
//! The programs run on the CPU on every backend, and on the GPU with `--vulkan` (Windows).

use std::time::Duration;

use uison::{App, Color, Event, Host, Point, Rect, Size, Writer};

const BACKGROUND: Color = Color::rgb(0xF5, 0xF5, 0xF7);
const TEXT: Color = Color::rgb(0x1D, 0x1D, 0x1F);
const MUTED: Color = Color::rgb(0x86, 0x86, 0x8B);

const TILES: &[(&str, &str)] = &[
    ("Gradient", "mix(#3b5bdb, #9c36b5, uv.x)"),
    ("Rings", "let d = distance(uv, vec2(0.5)); mix(#0b7285, #c5f6fa, 0.5 + 0.5 * sin(d * 40.0 - time * 4.0))"),
    ("Checker", "let g = floor(pos / 12.0); mod(g.x + g.y, 2.0) < 1.0 ? #f1f3f5 : #ced4da"),
    (
        "Hue wheel",
        "let p = (pos - size * 0.5) / size.y; let h = fract(atan(p.y, p.x) / (2.0 * pi) + 0.5 + time * 0.1); \
         vec4(abs(h * 6.0 - 3.0) - 1.0, 2.0 - abs(h * 6.0 - 2.0), 2.0 - abs(h * 6.0 - 4.0), 1.0 - smoothstep(0.47, 0.5, length(p)))",
    ),
    (
        "Plasma",
        "let p = pos / 20.0; let v = (sin(p.x + time) + sin(p.y * 1.3 - time * 0.7) + sin((p.x + p.y) * 0.7 + time * 1.3)) * pi; \
         0.5 + 0.5 * sin(vec3(v, v + 2.0, v + 4.0))",
    ),
    ("Stripes", "fract((pos.x + pos.y) / 16.0 - time * 0.5) < 0.5 ? #ffd43b : #343a40"),
];

const TILE: Size = Size { w: 220.0, h: 150.0 };
const GAP: f32 = 24.0;

struct Shaders {
    size: Size,
    /// `uison::time::now()` at the start: `std::time::Instant` panics in the browser.
    start: Duration,
    /// `--time T`: a still frame at T seconds (screenshots, comparing backends).
    frozen: Option<f32>,
}

impl Shaders {
    fn render(&self) -> String {
        let mut w = Writer::new(self.size, BACKGROUND);
        w.text("Shaders", Point::new(GAP, 18.0)).size(22.0).bold().color(TEXT);
        let note = if uison::graphics() == uison::Graphics::Native {
            "uison shading language, run on the CPU"
        } else {
            "uison shading language → SPIR-V, run on the GPU"
        };
        w.text(note, Point::new(GAP, 48.0)).size(13.0).color(MUTED);
        let time = self.frozen.unwrap_or_else(|| (uison::time::now() - self.start).as_secs_f32());
        let columns = (((self.size.w - GAP) / (TILE.w + GAP)) as usize).max(1);
        for (i, (name, src)) in TILES.iter().enumerate() {
            let (col, row) = ((i % columns) as f32, (i / columns) as f32);
            let x = GAP + col * (TILE.w + GAP);
            let y = 84.0 + row * (TILE.h + 48.0);
            w.text(name, Point::new(x, y)).size(13.0).bold().color(TEXT);
            let item = w.shader(Rect::new(x, y + 22.0, TILE.w, TILE.h), src).radius(12.0);
            // Only programs that read `time` change from tick to tick (and get redrawn).
            if src.contains("time") {
                item.time(time);
            }
        }
        w.finish()
    }
}

impl App for Shaders {
    fn frame(&mut self, _host: &dyn Host) -> String {
        self.render()
    }

    fn on_event(&mut self, e: &Event, _hit: Option<&str>, _host: &dyn Host) -> Option<String> {
        match e {
            Event::Resize(s) => self.size = *s,
            Event::Tick => {}
            _ => return None,
        }
        Some(self.render())
    }

    /// Ticks with the display: the animation.
    fn wants_tick(&self) -> Option<Duration> {
        self.frozen.is_none().then_some(Duration::from_millis(16))
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    // `--vulkan`: the programs run on the GPU (built with `--features vulkan`, Windows).
    #[cfg(all(windows, feature = "vulkan"))]
    if args.iter().any(|a| a == "--vulkan") {
        uison::set_graphics(uison::Graphics::Vulkan);
    }
    let frozen = args.iter().position(|a| a == "--time").and_then(|i| args.get(i + 1)?.parse().ok());
    let size = Size::new(3.0 * TILE.w + 4.0 * GAP, 2.0 * (TILE.h + 48.0) + 84.0);
    uison::run(Shaders { size, start: uison::time::now(), frozen }, "uison shaders");
}
