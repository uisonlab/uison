//! Experiment: does drawing a shape whole match drawing it tile by tile with a clip.

use uison_frame::Frame;
use uison_apple::{Bitmap, Text};
use uison_core::Rect;

fn main() {
    let text = Text::new();
    let cases = [
        ("rect opaque", r##"{ rect: [10.3, 10.7, 70, 50], fill: "#3366CC" }"##),
        ("rect alpha", r##"{ rect: [10.3, 10.7, 70, 50], fill: "#3366CC80" }"##),
        ("rrect alpha", r##"{ rect: [10, 10, 70, 50], radius: 8, fill: "#3366CC80" }"##),
        ("circle opaque", r##"{ circle: [50, 40, 27], fill: "#3366CC" }"##),
        ("circle alpha", r##"{ circle: [50, 40, 27], fill: "#3366CC80" }"##),
        ("tri opaque", r##"{ tri: [5, 5, 90, 30, 30, 75], fill: "#3366CC" }"##),
        ("tri alpha", r##"{ tri: [5, 5, 90, 30, 30, 75], fill: "#3366CC80" }"##),
        ("line opaque", r##"{ line: [5, 5, 90, 70], width: 3, color: "#3366CC" }"##),
        ("line alpha", r##"{ line: [5, 5, 90, 70], width: 3, color: "#3366CC80" }"##),
        ("stroke circle", r##"{ circle: [50, 40, 27], stroke: "#3366CC", width: 3 }"##),
        ("text", r##"{ text: "Привет W", at: [5, 30], size: 20, color: "#3366CC" }"##), // Cyrillic on purpose: non-Latin glyphs
    ];
    for scale in [1.0, 2.0] {
        for (name, item) in cases {
            let src = format!("{{ size: [100, 80], background: \"#FAFAFA\", items: [{item}] }}");
            let f = Frame::parse(&src, &text).unwrap();
            let full = Bitmap::new(f.size, scale);
            full.paint(&f, &text, f.rect());
            let tiled = Bitmap::new(f.size, scale);
            for ty in 0..4 {
                for tx in 0..4 {
                    tiled.paint(&f, &text, Rect::new(tx as f32 * 25.0, ty as f32 * 20.0, 25.0, 20.0));
                }
            }
            let (a, b) = (full.pixels(), tiled.pixels());
            let bad = a.chunks(4).zip(b.chunks(4)).filter(|(x, y)| x != y).count();
            let maxd = a.iter().zip(&b).map(|(x, y)| x.abs_diff(*y)).max().unwrap();
            println!("scale {scale}  {name:<14} differing px: {bad:>4}  max diff: {maxd}");
        }
    }
}
