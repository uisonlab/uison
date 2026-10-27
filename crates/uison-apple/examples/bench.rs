//! Frame pipeline benchmark: generation → parsing → text metrics → diff → drawing.
//! `cargo run --release -p uison-macos --example bench [items]`

use std::hint::black_box;
use std::time::{Duration, Instant};

use uison_core::{Color, Point, Rect, Size, TextMeasure, TextMetrics};
use uison_frame::{Frame, Writer, diff};
use uison_apple::{Bitmap, Text};

const SIZE: Size = Size::new(1200.0, 900.0);

/// Zero metrics: measures parsing without CoreText.
struct NoMeasure;

impl TextMeasure for NoMeasure {
    fn measure(&self, _: &str, _: &uison_core::Font) -> TextMetrics {
        TextMetrics::default()
    }
}

/// Median over `n` runs.
fn bench<T>(n: usize, mut f: impl FnMut() -> T) -> Duration {
    let mut times: Vec<Duration> = (0..n)
        .map(|_| {
            let t = Instant::now();
            black_box(f());
            t.elapsed()
        })
        .collect();
    times.sort();
    times[n / 2]
}

/// A typical interface: list rows with background, icon, two labels, separator.
fn make(items: usize, shift: f32) -> String {
    let mut w = Writer::new(SIZE, Color::rgb(250, 250, 250));
    let rows = items / 5;
    for i in 0..rows {
        let y = (i % 40) as f32 * 22.0 + shift;
        let x = (i / 40) as f32 * 300.0;
        w.rect(Rect::new(x, y, 296.0, 21.0)).fill(Color::rgb(255, 255, 255)).radius(4.0).id("row");
        w.circle(Point::new(x + 12.0, y + 10.5), 6.0).fill(Color::rgb(0, 122, 255));
        w.text(&format!("List item {i}"), Point::new(x + 24.0, y + 3.0)).size(13.0).color(Color::BLACK);
        w.text(&format!("{}:{:02}", i % 24, i % 60), Point::new(x + 250.0, y + 3.0)).size(11.0).color(Color::rgb(140, 140, 145));
        w.line(Point::new(x, y + 21.5), Point::new(x + 296.0, y + 21.5)).color(Color::rgb(229, 229, 234));
    }
    w.finish()
}

fn main() {
    let items: usize = std::env::args().nth(1).map_or(1000, |s| s.parse().unwrap());
    let text = Text::new();
    let src = make(items, 0.0);
    let src2 = make(items, 0.0).replacen("List item 7\"", "List item 7!\"", 1);
    let n = 50;
    let mb = src.len() as f64 / 1e6;

    let t_gen = bench(n, || make(items, 0.0));
    let t_json = bench(n, || uison_json::parse(&src).unwrap());
    let t_parse = bench(n, || Frame::parse(&src, &NoMeasure).unwrap());
    Frame::parse(&src, &text).unwrap(); // warm up fonts and the line cache
    let t_full = bench(n, || Frame::parse(&src, &text).unwrap());
    let a = Frame::parse(&src, &text).unwrap();
    let b = Frame::parse(&src2, &text).unwrap();
    let t_diff_same = bench(n, || diff(&a, &a));
    let t_diff_one = bench(n, || diff(&a, &b));
    let bmp = Bitmap::new(SIZE, 2.0);
    let t_paint = bench(10, || bmp.paint(&a, &text, a.rect()));
    let dirty = match diff(&a, &b) {
        uison_frame::Damage::Rects(rs) => rs[0],
        d => panic!("{d:?}"),
    };
    let t_paint_dirty = bench(n, || bmp.paint(&b, &text, dirty));

    let us = |d: Duration| d.as_secs_f64() * 1e6;
    println!("frame: {} items, {} bytes ({:.0} B/item)", a.items.len(), src.len(), src.len() as f64 / a.items.len() as f64);
    println!("generate (Writer)        {:>8.0} us", us(t_gen));
    println!("parse → Value tree       {:>8.0} us  {:>6.0} MB/s", us(t_json), mb / t_json.as_secs_f64());
    println!("parse → Frame (direct)   {:>8.0} us  {:>6.0} MB/s", us(t_parse), mb / t_parse.as_secs_f64());
    println!("  + text metrics         {:>8.0} us", us(t_full - t_parse));
    println!("diff identical           {:>8.0} us", us(t_diff_same));
    println!("diff one text changed    {:>8.0} us  → {dirty:?}", us(t_diff_one));
    println!("paint full @2x           {:>8.0} us", us(t_paint));
    println!("paint dirty rect @2x     {:>8.0} us", us(t_paint_dirty));
    let pipeline = t_gen + t_full + t_diff_one;
    println!(
        "one-change update: gen+parse+diff {:.0} us + paint {:.0} us; parse share {:.0}%",
        us(pipeline),
        us(t_paint_dirty),
        100.0 * us(t_parse) / us(pipeline + t_paint_dirty)
    );
}
