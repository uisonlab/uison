//! Palette: dark background and pastel sectors along the rainbow, from red (large) to violet.

use uison::Color;

pub const BG: Color = Color::rgb(0x24, 0x25, 0x2a);
pub const PANEL: Color = Color::rgb(0x2b, 0x2c, 0x32);
pub const SMALL: Color = Color::rgb(0x55, 0x56, 0x5e);
pub const TEXT: Color = Color::rgb(0xe8, 0xe8, 0xec);
pub const TEXT_WEAK: Color = Color::rgb(0x9a, 0x9b, 0xa3);
pub const ACCENT: Color = Color::WHITE;
pub const BORDER: Color = Color::rgb(0x45, 0x46, 0x4e);

/// Share of the view that is already pure red (larger is red too).
pub const SHARE_RED: f32 = 0.3;
/// Share that is already violet. The chart merges everything below ~0.06% of the view,
/// so the whole rainbow lands on what's visible.
pub const SHARE_VIOLET: f32 = 0.001;
const HUE_VIOLET: f32 = 270.0 / 360.0;

/// Position on the rainbow by share of the view: 0 is red (large), 1 is violet. Logarithmic:
/// sizes on disk differ by orders of magnitude.
pub fn rainbow_position(share: f32) -> f32 {
    let (hi, lo) = (SHARE_RED.log10(), SHARE_VIOLET.log10());
    ((hi - share.max(1e-12).log10()) / (hi - lo)).clamp(0.0, 1.0)
}

/// Color of an item with share `share` of the view on ring `ring` (0 is the innermost; fractional during animation).
/// Outer rings are paler, files more muted than folders.
pub fn sector(share: f32, ring: f32, is_dir: bool) -> Color {
    let hue = rainbow_position(share) * HUE_VIOLET;
    let mut sat = (0.5 - 0.045 * ring).max(0.18);
    let mut val = 0.97;
    if !is_dir {
        sat *= 0.8;
        val = 0.9;
    }
    hsv(hue, sat, val)
}

fn hsv(h: f32, s: f32, v: f32) -> Color {
    let h6 = h.rem_euclid(1.0) * 6.0;
    let c = v * s;
    let x = c * (1.0 - (h6 % 2.0 - 1.0).abs());
    let (r, g, b) = match h6 as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = v - c;
    let byte = |f: f32| ((f + m) * 255.0).round().clamp(0.0, 255.0) as u8;
    Color::rgb(byte(r), byte(g), byte(b))
}

pub fn lerp(a: Color, b: Color, t: f32) -> Color {
    let mix = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color::rgba(mix(a.r, b.r), mix(a.g, b.g), mix(a.b, b.b), mix(a.a, b.a))
}

pub fn alpha(c: Color, a: f32) -> Color {
    Color::rgba(c.r, c.g, c.b, (c.a as f32 * a.clamp(0.0, 1.0)).round() as u8)
}

pub fn hovered(c: Color) -> Color {
    lerp(c, Color::WHITE, 0.35)
}

/// In the Trash queue: gray, slightly darker than its previous color, so sectors stay distinguishable.
pub fn queued(c: Color) -> Color {
    let luma = 0.299 * c.r as f32 + 0.587 * c.g as f32 + 0.114 * c.b as f32;
    let g = (luma * 0.62).round() as u8;
    Color::rgba(g, g, g, c.a)
}

pub fn dimmed(c: Color) -> Color {
    lerp(c, BG, 0.55)
}

/// `#RRGGBB` / `#RRGGBBAA` for screen data.
pub fn hex(c: Color) -> String {
    if c.a == 255 { format!("#{:02X}{:02X}{:02X}", c.r, c.g, c.b) } else { format!("#{:02X}{:02X}{:02X}{:02X}", c.r, c.g, c.b, c.a) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn big_is_red_small_is_violet() {
        assert_eq!(rainbow_position(0.9), 0.0);
        assert_eq!(rainbow_position(SHARE_VIOLET), 1.0);
        assert!(rainbow_position(0.1) < rainbow_position(0.01));
        assert_eq!(hsv(0.0, 1.0, 1.0), Color::rgb(255, 0, 0));
        assert_eq!(hex(Color::rgba(1, 2, 3, 4)), "#01020304");
    }
}
