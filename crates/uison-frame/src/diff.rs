//! Frame comparison → regions that need a redraw.
//!
//! Correctness: a pixel's color is determined by the sequence of items
//! covering it. If that sequence changed, some changed item
//! (old or new) covers the pixel — and its bounds end up in the damage.

use uison_core::Rect;

use crate::{Frame, Item, Shape};

#[derive(Clone, Debug, PartialEq)]
pub enum Damage {
    None,
    /// Redraw the whole window.
    Full,
    /// Non-overlapping rectangles in whole points.
    Rects(Vec<Rect>),
}

impl Damage {
    /// Rectangles to redraw; `full` is the rectangle of the whole frame.
    pub fn rects(&self, full: Rect) -> impl Iterator<Item = Rect> + '_ {
        let (one, many): (Option<Rect>, &[Rect]) = match self {
            Damage::None => (None, &[]),
            Damage::Full => (Some(full), &[]),
            Damage::Rects(rs) => (None, rs),
        };
        one.into_iter().chain(many.iter().copied())
    }

    /// Fraction of the frame area being redrawn, 0..=1.
    pub fn ratio(&self, full: Rect) -> f32 {
        self.rects(full).fold(0.0, |acc, r| acc + r.area()) / full.area().max(1.0)
    }
}

/// More rectangles than this get merged into one: painting each costs
/// a pass over the item list, and AppKit merges regions anyway.
const MAX_RECTS: usize = 8;
/// If the dirty region covers more than this fraction of the frame, redraw everything.
const FULL_RATIO: f32 = 0.6;

/// The LCS table is at most n·m cells; beyond that the middle is compared pairwise.
const LCS_LIMIT: usize = 128 * 128;

pub fn diff(old: &Frame, new: &Frame) -> Damage {
    if old.size != new.size || old.background != new.background {
        return Damage::Full;
    }
    let mut rects = Vec::new();
    diff_items(&old.items, &new.items, &mut rects);
    merge(rects, new.rect())
}

/// 1. Skip the common prefix and suffix (the common case is a single change).
/// 2. Align the middle by the longest common subsequence (LCS):
///    matched items keep their relative order, so pixels covered
///    only by them don't change. No keys (`id`) are needed for this.
///    A clip with the same rectangle counts as "the same item" even if
///    something inside changed: its contents are compared recursively.
/// 3. Gaps between matches: of equal length — compared pairwise
///    (descending into clips), otherwise dirty as a whole.
fn diff_items(old: &[Item], new: &[Item], out: &mut Vec<Rect>) {
    let prefix = old.iter().zip(new).take_while(|(a, b)| a.shape == b.shape).count();
    let (old, new) = (&old[prefix..], &new[prefix..]);
    let suffix = old.iter().rev().zip(new.iter().rev()).take_while(|(a, b)| a.shape == b.shape).count();
    let (old, new) = (&old[..old.len() - suffix], &new[..new.len() - suffix]);

    if old.is_empty() || new.is_empty() || old.len() * new.len() > LCS_LIMIT {
        gap(old, new, out);
        return;
    }
    let (mut pi, mut pj) = (0, 0);
    for (i, j) in lcs_pairs(old, new).into_iter().chain([(old.len(), new.len())]) {
        gap(&old[pi..i], &new[pj..j], out);
        if i < old.len() && old[i].shape != new[j].shape {
            diff_pair(&old[i], &new[j], out);
        }
        (pi, pj) = (i + 1, j + 1);
    }
}

fn gap(old: &[Item], new: &[Item], out: &mut Vec<Rect>) {
    if old.len() == new.len() {
        for (a, b) in old.iter().zip(new) {
            if a.shape != b.shape {
                diff_pair(a, b, out);
            }
        }
    } else {
        out.extend(old.iter().chain(new).filter_map(visible_bounds));
    }
}

/// Bounds on screen; invisible `hit` areas don't affect pixels.
fn visible_bounds(it: &Item) -> Option<Rect> {
    if matches!(it.shape, Shape::Hit { .. }) { None } else { Some(it.bounds) }
}

/// Equal, or the same clip (same rectangle).
fn same(a: &Item, b: &Item) -> bool {
    match (&a.shape, &b.shape) {
        (Shape::Clip { rect: ra, .. }, Shape::Clip { rect: rb, .. }) => ra == rb,
        _ => a.shape == b.shape,
    }
}

/// Index pairs of matching items (LCS), ascending.
fn lcs_pairs(old: &[Item], new: &[Item]) -> Vec<(usize, usize)> {
    let (n, m) = (old.len(), new.len());
    let w = m + 1;
    let eq: Vec<bool> = old.iter().flat_map(|a| new.iter().map(move |b| same(a, b))).collect();
    // t[i][j] is the LCS length of suffixes old[i..], new[j..].
    let mut t = vec![0u16; (n + 1) * w];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            t[i * w + j] = if eq[i * m + j] { t[(i + 1) * w + j + 1] + 1 } else { t[(i + 1) * w + j].max(t[i * w + j + 1]) };
        }
    }
    let mut pairs = Vec::with_capacity(t[0] as usize);
    let (mut i, mut j) = (0, 0);
    while i < n && j < m {
        if eq[i * m + j] {
            pairs.push((i, j));
            i += 1;
            j += 1;
        } else if t[(i + 1) * w + j] >= t[i * w + j + 1] {
            i += 1;
        } else {
            j += 1;
        }
    }
    pairs
}

fn diff_pair(a: &Item, b: &Item, out: &mut Vec<Rect>) {
    match (&a.shape, &b.shape) {
        // Same clip: look for changes inside and clip them to it.
        (Shape::Clip { rect: ra, items: ia }, Shape::Clip { rect: rb, items: ib }) if ra == rb => {
            let from = out.len();
            diff_items(ia, ib, out);
            for r in &mut out[from..] {
                *r = r.intersect(ra);
            }
        }
        _ => out.extend(visible_bounds(a).into_iter().chain(visible_bounds(b))),
    }
}

fn merge(rects: Vec<Rect>, frame: Rect) -> Damage {
    let mut rs: Vec<Rect> = rects
        .into_iter()
        .map(|r| r.round_out().intersect(&frame))
        .filter(|r| !r.is_empty())
        .collect();
    if rs.is_empty() {
        return Damage::None;
    }
    // Many changes (list scrolling): merging costs more than one common rectangle.
    if rs.len() > MAX_RECTS * 8 {
        rs = vec![rs.iter().fold(Rect::default(), |acc, r| acc.union(r))];
    }
    // Merge overlapping ones. After a merge the rectangle has grown and may
    // touch already checked ones — start over (n is small).
    let mut i = 0;
    while i < rs.len() {
        match (i + 1..rs.len()).find(|&j| rs[i].intersects(&rs[j])) {
            Some(j) => {
                let r = rs.swap_remove(j);
                rs[i] = rs[i].union(&r);
                i = 0;
            }
            None => i += 1,
        }
    }
    if rs.len() > MAX_RECTS {
        rs = vec![rs.iter().fold(Rect::default(), |acc, r| acc.union(r))];
    }
    let covered: f32 = rs.iter().map(Rect::area).sum();
    if covered > frame.area() * FULL_RATIO {
        return Damage::Full;
    }
    Damage::Rects(rs)
}
