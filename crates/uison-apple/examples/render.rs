//! Render a frame to PNG headless: `render <frame.json> <out.png> [scale]`.

use std::time::Instant;

use uison_frame::Frame;
use uison_apple::{Bitmap, Text};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("usage: render <frame.json> <out.png> [scale]");
        std::process::exit(2);
    }
    let src = std::fs::read_to_string(&args[1]).expect("read frame");
    let scale: f32 = args.get(3).map_or(2.0, |s| s.parse().expect("scale"));
    let text = Text::new();
    let t0 = Instant::now();
    let frame = Frame::parse(&src, &text).unwrap_or_else(|e| {
        eprintln!("{}:{e}", args[1]);
        std::process::exit(1)
    });
    let t1 = Instant::now();
    let bmp = Bitmap::new(frame.size, scale);
    bmp.paint(&frame, &text, frame.rect());
    let t2 = Instant::now();
    bmp.save_png(&args[2]).expect("save");
    eprintln!(
        "{} items, load {}us, paint {}us ({}x{} px)",
        frame.items.len(),
        (t1 - t0).as_micros(),
        (t2 - t1).as_micros(),
        bmp.width,
        bmp.height
    );
}
