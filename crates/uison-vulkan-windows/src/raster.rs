//! Polygon coverage masks on the CPU: a polygon's fill (nonzero rule) and stroke (round joins)
//! are not boxes or segments the shader can measure. Polygons are small (icons, arrows,
//! chart sectors), and a mask goes into the atlas and is reused while the polygon stays put.

/// Coverage 0..=255 of the pixels `x..x + w`, `y..y + h` (window pixels), row by row.
pub struct Mask {
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
    pub cov: Vec<u8>,
}

/// Sub-scanlines per pixel row; along a scanline coverage is exact.
const SUB: usize = 16;

/// The pixel box around the points plus `pad`, cut to `bounds` (x0, y0, x1, y1).
fn area(p: &[(f32, f32)], pad: f32, bounds: [i32; 4]) -> Option<(i32, i32, u32, u32)> {
    let (mut x0, mut y0, mut x1, mut y1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    for &(x, y) in p {
        (x0, y0, x1, y1) = (x0.min(x), y0.min(y), x1.max(x), y1.max(y));
    }
    let x0 = ((x0 - pad).floor() as i32).max(bounds[0]);
    let y0 = ((y0 - pad).floor() as i32).max(bounds[1]);
    let x1 = ((x1 + pad).ceil() as i32).min(bounds[2]);
    let y1 = ((y1 + pad).ceil() as i32).min(bounds[3]);
    (x1 > x0 && y1 > y0).then(|| (x0, y0, (x1 - x0) as u32, (y1 - y0) as u32))
}

/// The closed polygon filled by the nonzero winding rule.
pub fn fill(p: &[(f32, f32)], bounds: [i32; 4]) -> Option<Mask> {
    let (x0, y0, w, h) = area(p, 0.0, bounds)?;
    let mut acc = vec![0f32; w as usize];
    let mut cov = vec![0u8; (w * h) as usize];
    let mut hits: Vec<(f32, i32)> = Vec::new();
    let weight = 1.0 / SUB as f32;
    for row in 0..h as usize {
        acc.fill(0.0);
        for s in 0..SUB {
            let sy = y0 as f32 + row as f32 + (s as f32 + 0.5) * weight;
            hits.clear();
            for i in 0..p.len() {
                let (a, b) = (p[i], p[(i + 1) % p.len()]);
                if a.1 == b.1 {
                    continue;
                }
                let (lo, hi, dir) = if a.1 < b.1 { (a, b, 1) } else { (b, a, -1) };
                if sy >= lo.1 && sy < hi.1 {
                    let x = lo.0 + (sy - lo.1) * (hi.0 - lo.0) / (hi.1 - lo.1);
                    hits.push((x - x0 as f32, dir));
                }
            }
            hits.sort_by(|a, b| a.0.total_cmp(&b.0));
            let mut winding = 0;
            for pair in hits.windows(2) {
                winding += pair[0].1;
                if winding != 0 {
                    span(&mut acc, pair[0].0, pair[1].0, weight);
                }
            }
        }
        for (c, a) in cov[row * w as usize..][..w as usize].iter_mut().zip(&acc) {
            *c = (a.clamp(0.0, 1.0) * 255.0).round() as u8;
        }
    }
    Some(Mask { x: x0, y: y0, w, h, cov })
}

/// Add `weight` × the covered part of each pixel between `xa` and `xb`.
fn span(acc: &mut [f32], xa: f32, xb: f32, weight: f32) {
    let n = acc.len() as f32;
    let (xa, xb) = (xa.clamp(0.0, n), xb.clamp(0.0, n));
    if xb <= xa {
        return;
    }
    let (ia, ib) = (xa as usize, xb as usize);
    if ia == ib {
        acc[ia] += (xb - xa) * weight;
        return;
    }
    acc[ia] += (ia as f32 + 1.0 - xa) * weight;
    for a in &mut acc[ia + 1..ib] {
        *a += weight;
    }
    if ib < acc.len() {
        acc[ib] += (xb - ib as f32) * weight;
    }
}

/// The closed polygon's outline, `width` wide: the union of round-capped segments, which
/// makes the joins round.
pub fn stroke(p: &[(f32, f32)], width: f32, bounds: [i32; 4]) -> Option<Mask> {
    let half = width / 2.0;
    let (x0, y0, w, h) = area(p, half + 1.0, bounds)?;
    let mut cov = vec![0u8; (w * h) as usize];
    for i in 0..p.len() {
        let (a, b) = (p[i], p[(i + 1) % p.len()]);
        let Some((sx, sy, sw, sh)) = area(&[a, b], half + 1.0, [x0, y0, x0 + w as i32, y0 + h as i32]) else { continue };
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let len2 = dx * dx + dy * dy;
        for py in sy..sy + sh as i32 {
            for px in sx..sx + sw as i32 {
                let (cx, cy) = (px as f32 + 0.5 - a.0, py as f32 + 0.5 - a.1);
                let t = if len2 > 0.0 { ((cx * dx + cy * dy) / len2).clamp(0.0, 1.0) } else { 0.0 };
                let d = ((cx - t * dx).powi(2) + (cy - t * dy).powi(2)).sqrt();
                // The part of the pixel inside [-half, half] around the segment, as in the shader.
                let c = ((d + 0.5).min(half) - (d - 0.5).max(-half)).clamp(0.0, 1.0);
                let i = ((py - y0) as u32 * w + (px - x0) as u32) as usize;
                cov[i] = cov[i].max((c * 255.0).round() as u8);
            }
        }
    }
    Some(Mask { x: x0, y: y0, w, h, cov })
}

#[cfg(test)]
mod tests {
    use super::*;

    const BIG: [i32; 4] = [-1000, -1000, 1000, 1000];

    #[test]
    fn square_fill_is_exact() {
        let m = fill(&[(1.0, 1.0), (3.0, 1.0), (3.0, 3.0), (1.0, 3.0)], BIG).unwrap();
        assert_eq!((m.x, m.y, m.w, m.h), (1, 1, 2, 2));
        assert!(m.cov.iter().all(|&c| c == 255));
    }

    #[test]
    fn half_pixel_edges() {
        let m = fill(&[(0.5, 0.0), (1.5, 0.0), (1.5, 1.0), (0.5, 1.0)], BIG).unwrap();
        assert_eq!((m.x, m.w), (0, 2));
        assert_eq!(m.cov, vec![128, 128]);
    }

    #[test]
    fn nonzero_keeps_overlap_filled() {
        // Two overlapping squares wound the same way: the overlap stays filled (even-odd would empty it).
        let p = [(0.0, 0.0), (4.0, 0.0), (4.0, 4.0), (0.0, 4.0), (0.0, 0.0), (2.0, 2.0), (6.0, 2.0), (6.0, 6.0), (2.0, 6.0), (2.0, 2.0)];
        let m = fill(&p, BIG).unwrap();
        assert_eq!(m.cov[(3 * m.w + 3) as usize], 255);
    }

    #[test]
    fn stroke_covers_the_edge() {
        let m = stroke(&[(2.0, 2.0), (8.0, 2.0), (8.0, 8.0)], 2.0, BIG).unwrap();
        // Both sides of the top edge are covered; inside the triangle, 1.5 px from every edge, nothing.
        let at = |x: i32, y: i32| m.cov[((y - m.y) as u32 * m.w + (x - m.x) as u32) as usize];
        assert_eq!(at(5, 1), 255);
        assert_eq!(at(5, 2), 255);
        assert_eq!(at(6, 3), 0);
    }
}
