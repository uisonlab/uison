//! Inline flow (`Flow`), like the inline formatting context in CSS: `Text` children wrap as one
//! text (no line break between `foo` and `<b>bar</b>` without a space), other children are
//! unbreakable blocks sitting with their bottom on the baseline. Lines are aligned by the baseline.

use uison_core::{Point, Size};

use crate::el::*;
use crate::layout::{Cons, Ctx, layout};

/// A piece of the flow: a word or a space of segment `child` (bytes `a..b` of its text), a line break, a block.
#[derive(Clone, Copy)]
enum Piece {
    Word { child: usize, a: usize, b: usize, w: f32 },
    Space { child: usize, a: usize, w: f32 },
    Break { child: usize },
    Block { child: usize, w: f32 },
}

/// What was placed in a line: segment `child` (bytes `a..b`) or a block (`a == b == 0`, `block`).
#[derive(Clone, Copy)]
struct Placed {
    child: usize,
    a: usize,
    b: usize,
    x: f32,
    w: f32,
    block: bool,
}

/// Height above and below the baseline (with half the leading, like `line-height` in CSS).
#[derive(Clone, Copy, Default)]
struct VMetrics {
    above: f32,
    below: f32,
}

/// Lay out the flow's children in width `max_w`; returns the content size.
pub(crate) fn layout_flow(children: &mut [El], max_w: f32, align: f32, line_height: f32, cx: &Ctx) -> Size {
    let mut vm = Vec::with_capacity(children.len());
    let mut pieces = Vec::new();
    for (i, ch) in children.iter_mut().enumerate() {
        if let Kind::Text(t) = &mut ch.kind {
            let tm = cx.m.measure("Ag", &t.font);
            let h = tm.height();
            let half = (h * line_height - h) / 2.0;
            (t.glyph_h, t.ascent, t.line_h, t.in_flow) = (h, tm.ascent, h * line_height, true);
            t.frags.clear();
            vm.push(VMetrics { above: tm.ascent + half, below: h - tm.ascent + half });
            let space = cx.m.measure(" ", &t.font).width;
            let mut off = 0;
            for (k, para) in t.text.split('\n').enumerate() {
                if k > 0 {
                    pieces.push(Piece::Break { child: i });
                    off += 1;
                }
                for (j, word) in para.split(' ').enumerate() {
                    if j > 0 {
                        pieces.push(Piece::Space { child: i, a: off - 1, w: space });
                    }
                    if !word.is_empty() {
                        pieces.push(Piece::Word { child: i, a: off, b: off + word.len(), w: cx.m.measure(word, &t.font).width });
                    }
                    off += word.len() + 1;
                }
                off -= 1;
            }
        } else {
            let s = layout(ch, Cons::loose(max_w, f32::INFINITY), cx);
            vm.push(VMetrics { above: s.h, below: 0.0 });
            pieces.push(Piece::Block { child: i, w: s.w });
        }
    }

    // Lines: what was placed in them and whose height "strut" an empty line uses (`<br><br>`).
    let mut lines: Vec<(Vec<Placed>, Option<usize>)> = Vec::new();
    let mut cur: Vec<Placed> = Vec::new();
    let mut x = 0.0f32;
    let mut spaces: Vec<Placed> = Vec::new();
    let mut p = 0;
    while p < pieces.len() {
        match pieces[p] {
            Piece::Break { child } => {
                lines.push((std::mem::take(&mut cur), Some(child)));
                (x, p) = (0.0, p + 1);
                spaces.clear();
            }
            Piece::Space { child, a, w } => {
                // Spaces at the start of a line aren't needed (as in CSS).
                if !cur.is_empty() {
                    spaces.push(Placed { child, a, b: a + 1, x: 0.0, w, block: false });
                }
                p += 1;
            }
            Piece::Word { .. } | Piece::Block { .. } => {
                // An unbreakable group: words in a row without a space (across segment boundaries) or one block.
                let end = match pieces[p] {
                    Piece::Block { .. } => p + 1,
                    _ => {
                        let mut e = p + 1;
                        while e < pieces.len() && matches!(pieces[e], Piece::Word { .. }) {
                            e += 1;
                        }
                        e
                    }
                };
                let gw: f32 = pieces[p..end].iter().map(width).sum();
                let single = end - p == 1;
                let sw: f32 = spaces.iter().map(|s| s.w).sum();
                if !cur.is_empty() && x + sw + gw > max_w {
                    lines.push((std::mem::take(&mut cur), None));
                    x = 0.0;
                } else {
                    for mut s in spaces.drain(..) {
                        s.x = x;
                        x += s.w;
                        cur.push(s);
                    }
                }
                spaces.clear();
                for &piece in &pieces[p..end] {
                    match piece {
                        Piece::Word { child, a, b, w } if w > max_w && single => {
                            // A single word wider than the line — cut it by characters.
                            let Kind::Text(t) = &children[child].kind else { unreachable!() };
                            let measure = |s: &str| cx.m.measure(s, &t.font).width;
                            let mut a = a;
                            while measure(&t.text[a..b]) > max_w - x {
                                let cut = a + crate::text::fit_prefix(&t.text[a..b], (max_w - x).max(1.0), &measure);
                                let w = measure(&t.text[a..cut]);
                                cur.push(Placed { child, a, b: cut, x, w, block: false });
                                lines.push((std::mem::take(&mut cur), None));
                                (x, a) = (0.0, cut);
                            }
                            let w = measure(&t.text[a..b]);
                            cur.push(Placed { child, a, b, x, w, block: false });
                            x += w;
                        }
                        Piece::Word { child, a, b, w } => {
                            cur.push(Placed { child, a, b, x, w, block: false });
                            x += w;
                        }
                        Piece::Block { child, w } => {
                            cur.push(Placed { child, a: 0, b: 0, x, w, block: true });
                            x += w;
                        }
                        _ => {}
                    }
                }
                p = end;
            }
        }
    }
    if !cur.is_empty() || lines.is_empty() {
        lines.push((cur, None));
    }

    // Lines vertically: the baseline follows the tallest segment.
    let mut frags: Vec<Vec<Frag>> = vec![Vec::new(); children.len()];

    let (mut y, mut widest) = (0.0f32, 0.0f32);
    for (items, strut) in &lines {
        let mut m = strut.map(|c| vm[c]).unwrap_or_default();
        for it in items {
            m.above = m.above.max(vm[it.child].above);
            m.below = m.below.max(vm[it.child].below);
        }
        // Line pieces → fragments: adjacent pieces of the same segment are merged; the fragment's
        // width is measured whole (with kerning), not as the sum of words from wrapping.
        let text_of = |c: usize| match &children[c].kind {
            Kind::Text(t) => Some(t),
            _ => None,
        };
        let mut run: Vec<(usize, String, f32, bool)> = Vec::new();
        for it in items {
            let piece = text_of(it.child).map_or("", |t| &t.text[it.a..it.b]);
            match run.last_mut() {
                Some((c, text, _, false)) if *c == it.child && !it.block => text.push_str(piece),
                _ => run.push((it.child, piece.to_string(), it.w, it.block)),
            }
        }
        for (c, text, w, block) in &mut run {
            if let Some(t) = text_of(*c).filter(|_| !*block) {
                *w = cx.m.measure(text, &t.font).width;
            }
        }
        let line_w: f32 = run.iter().map(|r| r.2).sum();
        widest = widest.max(line_w);
        let dx = if max_w.is_finite() { ((max_w - line_w) * align).max(0.0) } else { 0.0 };
        let base = y + m.above;
        let mut x = dx;
        for (c, text, w, block) in run {
            let ch = &mut children[c];
            if block {
                ch.pos = Point::new(x, base - vm[c].above);
            } else if let Kind::Text(t) = &ch.kind {
                frags[c].push(Frag { text, x, y: base - t.ascent, w });
            }
            x += w;
        }
        y += (m.above + m.below).ceil();
    }

    // A segment is a frame around its fragments; fragments are relative to its corner.
    for (ch, mut list) in children.iter_mut().zip(frags) {
        let Kind::Text(t) = &mut ch.kind else { continue };
        if list.is_empty() {
            (ch.pos, ch.size) = (Point::default(), Size::default());
            continue;
        }
        let x0 = list.iter().map(|f| f.x).fold(f32::INFINITY, f32::min);
        let y0 = list.iter().map(|f| f.y).fold(f32::INFINITY, f32::min);
        let x1 = list.iter().map(|f| f.x + f.w).fold(0.0, f32::max);
        let y1 = list.iter().map(|f| f.y).fold(0.0, f32::max) + t.glyph_h;
        for f in &mut list {
            (f.x, f.y) = (f.x - x0, f.y - y0);
        }
        (ch.pos, ch.size) = (Point::new(x0, y0), Size::new(x1 - x0, y1 - y0));
        t.frags = list;
    }
    let w = if align > 0.0 && max_w.is_finite() { max_w } else { widest.ceil() };
    Size::new(w, y)
}

fn width(p: &Piece) -> f32 {
    match *p {
        Piece::Word { w, .. } | Piece::Space { w, .. } | Piece::Block { w, .. } => w,
        Piece::Break { .. } => 0.0,
    }
}
