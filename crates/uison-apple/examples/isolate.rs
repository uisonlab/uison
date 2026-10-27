//! Debugging: which frame element draws differently with and without a clip.
//! `isolate <frame.json> x y w h`

use uison_core::Rect;
use uison_frame::{Frame, Item};
use uison_apple::{Bitmap, Text};

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let src = std::fs::read_to_string(&a[1]).unwrap();
    let n = |i: usize| a[i].parse::<f32>().unwrap();
    let clip = Rect::new(n(2), n(3), n(4), n(5));
    let text = Text::new();
    let f = Frame::parse(&src, &text).unwrap();
    for (i, it) in f.items.iter().enumerate() {
        if !it.bounds.intersects(&clip) {
            continue;
        }
        let one = Frame { items: vec![Item::clone(it)], ..f.clone() };
        let full = Bitmap::new(f.size, 2.0);
        full.paint(&one, &text, one.rect());
        let part = Bitmap::new(f.size, 2.0);
        part.paint(&one, &text, one.rect());
        part.paint(&one, &text, clip);
        let maxd = full.pixels().iter().zip(part.pixels()).map(|(x, y)| x.abs_diff(y)).max().unwrap();
        if maxd > 0 {
            println!("item {i}: max diff {maxd}: {:?}", it.shape);
        }
    }
}
