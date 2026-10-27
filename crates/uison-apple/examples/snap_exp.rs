//! Experiment: sharpness of thin lines depending on coordinate and scale.
//! Prints the vertical coverage profile across a horizontal line
//! (255 = pixel fully covered). A sharp line has only 0 and 255.

use uison_frame::Frame;
use uison_apple::{Bitmap, Text};

fn main() {
    let text = Text::new();
    for snap in [false, true] {
    println!("--- snapping {}", if snap { "on" } else { "off" });
    for scale in [1.0f32, 2.0] {
        for (y, width) in [(10.0, 1.0), (10.5, 1.0), (10.0, 2.0), (10.5, 2.0), (10.25, 0.5), (10.0, 0.5)] {
            let src = format!(
                r##"{{ size: [20, 20], background: "#FFFFFF", items: [{{ line: [0, {y}, 20, {y}], width: {width}, color: "#000000" }}] }}"##
            );
            let f = Frame::parse(&src, &text).unwrap();
            let bmp = Bitmap::new(f.size, scale);
            let mut c = bmp.canvas(&text);
            c.set_snap(snap);
            uison_frame::paint(&f, &mut c, f.rect());
            let px = bmp.pixels();
            let col = bmp.width / 2;
            let profile: Vec<u8> = (0..bmp.height)
                .map(|row| 255 - px[(row * bmp.width + col) * 4])
                .filter(|&v| v > 0)
                .collect();
            let crisp = profile.iter().all(|&v| v == 255);
            println!(
                "scale {scale}  y {y:<5} width {width:<3}  coverage {profile:?}{}",
                if crisp { "  crisp" } else { "  BLURRY" }
            );
        }
    }
    }
}
