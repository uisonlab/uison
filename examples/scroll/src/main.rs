//! Experiment: scrolling as a stream of frames. Every scroll event is
//! a new frame in which the visible rows are shifted by offset.
//!
//! `scroll` opens a window; `scroll --bench [scale]` runs 600 headless steps with stats.

use std::time::Duration;

use uison::{App, Color, Event, Host, Point, Rect, Size, Writer};

const ROWS: usize = 200;
const ROW_H: f32 = 56.0;
const HEADER_H: f32 = 52.0;

const NAMES: [&str; 8] = ["Anna", "Ben", "Clara", "David", "Emma", "Frank", "Grace", "Henry"];
const TEXTS: [&str; 5] = [
    "Hi! How is the project going?",
    "Sent over the mockups, take a look when you have time",
    "Is the 15:00 call still on?",
    "Great, thanks 👍",
    "Found a rendering bug, fixing it now",
];
const AVATARS: [Color; 6] = [
    Color::rgb(0xFF, 0x95, 0x00),
    Color::rgb(0x34, 0xC7, 0x59),
    Color::rgb(0x00, 0x7A, 0xFF),
    Color::rgb(0xAF, 0x52, 0xDE),
    Color::rgb(0xFF, 0x3B, 0x30),
    Color::rgb(0x5A, 0xC8, 0xFA),
];

struct List {
    size: Size,
    offset: f32,
    hover: Option<usize>,
}

impl List {
    fn max_offset(&self) -> f32 {
        (ROWS as f32 * ROW_H - (self.size.h - HEADER_H)).max(0.0)
    }

    fn render(&self) -> String {
        let (w, h) = (self.size.w, self.size.h);
        let mut out = Writer::new(self.size, Color::WHITE);
        let view = Rect::new(0.0, HEADER_H, w, h - HEADER_H);
        let first = (self.offset / ROW_H) as usize;
        let last = (((self.offset + view.h) / ROW_H).ceil() as usize).min(ROWS);

        // Only the visible rows, clipped to the scroll area.
        out.begin_clip(view);
        for i in first..last {
            let y = (HEADER_H + i as f32 * ROW_H - self.offset).round();
            let bg = if self.hover == Some(i) { Color::rgb(0xF2, 0xF2, 0xF7) } else { Color::WHITE };
            let id = format!("row{i}");
            out.rect(Rect::new(0.0, y, w, ROW_H)).fill(bg).id(&id);
            let name = NAMES[i % NAMES.len()];
            out.circle(Point::new(32.0, y + 28.0), 18.0).fill(AVATARS[i % AVATARS.len()]);
            out.text(&name[..name.char_indices().nth(1).unwrap().0], Point::new(26.0, y + 19.0)).size(15.0).bold().color(Color::WHITE);
            out.text(name, Point::new(62.0, y + 9.0)).size(14.0).bold().color(Color::rgb(0x1C, 0x1C, 0x1E));
            out.text(TEXTS[i % TEXTS.len()], Point::new(62.0, y + 30.0)).size(13.0).color(Color::rgb(0x8E, 0x8E, 0x93));
            out.text(&format!("{:02}:{:02}", 9 + i % 12, (i * 7) % 60), Point::new(w - 52.0, y + 10.0))
                .size(12.0)
                .color(Color::rgb(0x8E, 0x8E, 0x93));
            out.line(Point::new(62.0, y + ROW_H - 0.5), Point::new(w, y + ROW_H - 0.5)).color(Color::rgb(0xE5, 0xE5, 0xEA));
        }
        // Scroll bar.
        let content = ROWS as f32 * ROW_H;
        let thumb_h = (view.h * view.h / content).max(24.0);
        let thumb_y = view.y + (view.h - thumb_h) * self.offset / self.max_offset().max(1.0);
        out.rect(Rect::new(w - 7.0, thumb_y + 2.0, 4.0, thumb_h - 4.0)).fill(Color::rgba(0, 0, 0, 90)).radius(2.0);
        out.end_clip();

        // Header on top.
        out.rect(Rect::new(0.0, 0.0, w, HEADER_H)).fill(Color::rgb(0xF9, 0xF9, 0xF9));
        out.line(Point::new(0.0, HEADER_H - 0.5), Point::new(w, HEADER_H - 0.5)).color(Color::rgb(0xD1, 0xD1, 0xD6));
        out.text("Messages", Point::new(16.0, 14.0)).size(18.0).bold().color(Color::rgb(0x1C, 0x1C, 0x1E));
        out.text(&format!("{}–{} of {ROWS}", first + 1, last), Point::new(w - 110.0, 18.0))
            .size(12.0)
            .color(Color::rgb(0x8E, 0x8E, 0x93));
        out.finish()
    }
}

impl App for List {
    fn frame(&mut self, _: &dyn Host) -> String {
        self.render()
    }

    fn on_event(&mut self, e: &Event, hit: Option<&str>, _: &dyn Host) -> Option<String> {
        let row = |hit: Option<&str>| hit.and_then(|h| h.strip_prefix("row")).and_then(|n| n.parse().ok());
        match e {
            Event::Scroll(_, _, dy) => {
                let off = (self.offset - dy).clamp(0.0, self.max_offset());
                if off == self.offset {
                    return None;
                }
                self.offset = off;
                // The next mouse move recomputes hover; reset it while scrolling.
                self.hover = None;
            }
            Event::MouseMove(_) => {
                let h = row(hit);
                if h == self.hover {
                    return None;
                }
                self.hover = h;
            }
            Event::MouseLeave => self.hover = None,
            Event::Resize(s) => {
                self.size = *s;
                self.offset = self.offset.min(self.max_offset());
            }
            _ => return None,
        }
        Some(self.render())
    }
}

fn bench(scale: f32, size: Size) {
    use uison::Headless;
    let list = List { size, offset: 0.0, hover: None };
    let mut h = Headless::new(list, scale).unwrap();
    let mut steps: Vec<(Duration, Duration, f32)> = Vec::new();
    // Down and back at different speeds, like inertial trackpad scrolling.
    for i in 0..600 {
        let dy = match i / 150 {
            0 => -3.0,
            1 => -17.0,
            2 => 9.0,
            _ => 23.0,
        };
        if let Some((up, paint)) = h.event(Event::Scroll(Point::new(200.0, 300.0), 0.0, dy)).unwrap() {
            let area = up.damage.ratio(h.rt.frame.rect());
            steps.push((up.app + up.load + up.diff, paint, area));
        }
    }
    h.save_png("scroll_last.png", false).unwrap();
    let n = steps.len();
    let mut total: Vec<Duration> = steps.iter().map(|s| s.0 + s.1).collect();
    total.sort();
    let avg = |f: &dyn Fn(&(Duration, Duration, f32)) -> f64| steps.iter().map(f).sum::<f64>() / n as f64;
    println!("{}x{} @{scale}x: {n} scroll frames, {} items/frame, {} B/frame", size.w, size.h, h.rt.frame.items.len(), h.rt.app.frame(&h.text).len());
    println!("  app+load+diff  avg {:>6.0} us", avg(&|s| s.0.as_secs_f64() * 1e6));
    println!("  paint          avg {:>6.0} us", avg(&|s| s.1.as_secs_f64() * 1e6));
    println!("  damaged area   avg {:>6.1} %", avg(&|s| s.2 as f64 * 100.0));
    println!(
        "  total per frame: median {:.0} us, p95 {:.0} us, max {:.0} us → {:.0} fps possible (p95)",
        total[n / 2].as_secs_f64() * 1e6,
        total[n * 95 / 100].as_secs_f64() * 1e6,
        total[n - 1].as_secs_f64() * 1e6,
        1.0 / total[n * 95 / 100].as_secs_f64()
    );
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if let Some(i) = args.iter().position(|a| a == "--bench") {
        let num = |k: usize, d: f32| args.get(i + k).map_or(d, |s| s.parse().unwrap());
        bench(num(1, 2.0), Size::new(num(2, 420.0), num(3, 720.0)));
    } else {
        uison::run(List { size: Size::new(420.0, 720.0), offset: 0.0, hover: None }, "uison scroll");
    }
}
