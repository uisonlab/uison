//! Partial redraw must give the same result as a full one:
//! draw A fully, then only B's damage regions on top, compare with B drawn fully.

use uison_core::{Color, Point, Rect, Size};
use uison_frame::{Frame, Writer, diff};
use uison_apple::{Bitmap, Text};

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn f(&mut self, max: f32) -> f32 {
        (self.next() % 10_000) as f32 / 10_000.0 * max
    }
    fn n(&mut self, max: u64) -> u64 {
        self.next() % max
    }
    fn color(&mut self) -> Color {
        let v = self.next();
        Color::rgba(v as u8, (v >> 8) as u8, (v >> 16) as u8, if v & (1 << 30) != 0 { 255 } else { 128 })
    }
}

const TOLERANCE: u8 = 32;
const SIZE: Size = Size::new(320.0, 240.0);
// Cyrillic on purpose: non-Latin glyphs in the text mix.
const WORDS: [&str; 6] = ["Привет", "OK", "Отправить", "jg|Ég", "lorem ipsum", "W"];

/// An element is described by a "recipe" so it can be mutated and written out again.
#[derive(Clone)]
enum Spec {
    Rect(Rect, f32, Color, Option<Color>),
    Line(Point, Point, f32, Color),
    Circle(Point, f32, Color, Option<Color>),
    Tri([Point; 3], Color),
    Text(&'static str, Point, f32, bool, Color),
    Clip(Rect, Vec<Spec>),
}

fn random_spec(r: &mut Rng, depth: u32) -> Spec {
    let p = |r: &mut Rng| Point::new(r.f(SIZE.w), r.f(SIZE.h));
    match r.n(if depth == 0 { 6 } else { 5 }) {
        0 => Spec::Rect(
            Rect::new(r.f(SIZE.w), r.f(SIZE.h), r.f(120.0), r.f(80.0)),
            r.f(12.0),
            r.color(),
            if r.n(2) == 0 { Some(r.color()) } else { None },
        ),
        1 => Spec::Line(p(r), p(r), 0.5 + r.f(4.0), r.color()),
        2 => Spec::Circle(p(r), 2.0 + r.f(30.0), r.color(), if r.n(2) == 0 { Some(r.color()) } else { None }),
        3 => Spec::Tri([p(r), p(r), p(r)], r.color()),
        4 => Spec::Text(WORDS[r.n(6) as usize], p(r), 9.0 + r.f(20.0), r.n(2) == 0, r.color()),
        _ => Spec::Clip(
            Rect::new(r.f(SIZE.w / 2.0), r.f(SIZE.h / 2.0), 40.0 + r.f(150.0), 40.0 + r.f(100.0)),
            (0..r.n(6)).map(|_| random_spec(r, depth + 1)).collect(),
        ),
    }
}

fn write(w: &mut Writer, s: &Spec) {
    match s {
        Spec::Rect(rect, rad, fill, stroke) => {
            let it = w.rect(*rect).fill(*fill).radius(*rad);
            if let Some(c) = stroke {
                let _ = it.stroke(*c, 2.0);
            }
        }
        Spec::Line(a, b, width, c) => {
            w.line(*a, *b).color(*c).width(*width);
        }
        Spec::Circle(c, rad, fill, stroke) => {
            let it = w.circle(*c, *rad).fill(*fill);
            if let Some(s) = stroke {
                let _ = it.stroke(*s, 3.0);
            }
        }
        Spec::Tri(p, c) => {
            w.tri(*p).fill(*c);
        }
        Spec::Text(t, at, size, bold, c) => {
            let it = w.text(t, *at).size(*size).color(*c);
            if *bold {
                let _ = it.bold();
            }
        }
        Spec::Clip(r, items) => {
            w.begin_clip(*r);
            for s in items {
                write(w, s);
            }
            w.end_clip();
        }
    }
}

fn build(specs: &[Spec], text: &Text) -> Frame {
    let mut w = Writer::new(SIZE, Color::rgb(250, 250, 250));
    for s in specs {
        write(&mut w, s);
    }
    let src = w.finish();
    LAST_SRC.with(|l| *l.borrow_mut() = src.clone());
    Frame::parse(&src, text).unwrap()
}

thread_local! {
    static LAST_SRC: std::cell::RefCell<String> = Default::default();
}

fn mutate(r: &mut Rng, specs: &mut Vec<Spec>) {
    for _ in 0..=r.n(3) {
        let i = r.n(specs.len() as u64 + 1) as usize;
        match r.n(4) {
            0 if i < specs.len() => {
                specs.remove(i);
            }
            1 => specs.insert(i, random_spec(r, 0)),
            _ if i < specs.len() => {
                if let Spec::Clip(_, items) = &mut specs[i]
                    && !items.is_empty()
                {
                    let j = r.n(items.len() as u64) as usize;
                    items[j] = random_spec(r, 1);
                } else {
                    specs[i] = random_spec(r, 0);
                }
            }
            _ => {}
        }
    }
}

/// Returns (wrong pixels, damage region share).
/// 1. Outside damage, full drawings of A and B must match exactly, otherwise diff missed something.
/// 2. Inside damage, partial drawing must match the full one up to antialiasing
///    noise: CoreGraphics antialiases diagonal edges slightly differently
///    with and without a clip: up to ~7/255 for triangles, up to ~20/255 for long thin
///    lines (see examples/clip_exp.rs, examples/isolate.rs).
fn check(a: &Frame, b: &Frame, text: &Text, scale: f32) -> (usize, f32) {
    let damage = diff(a, b);
    let rects: Vec<Rect> = damage.rects(b.rect()).collect();
    let full_a = Bitmap::new(SIZE, scale);
    full_a.paint(a, text, a.rect());
    let full_b = Bitmap::new(SIZE, scale);
    full_b.paint(b, text, b.rect());
    let partial = Bitmap::new(SIZE, scale);
    partial.paint(a, text, a.rect());
    for r in &rects {
        partial.paint(b, text, *r);
    }
    let (pa, pb, pp) = (full_a.pixels(), full_b.pixels(), partial.pixels());
    let w = full_a.width;
    let in_damage = |i: usize| {
        let p = uison_core::Point::new((i % w) as f32 / scale + 0.01, (i / w) as f32 / scale + 0.01);
        rects.iter().any(|r| r.contains(p))
    };
    let mut bad = 0;
    for i in 0..pa.len() / 4 {
        let px = |v: &[u8]| -> [u8; 4] { v[i * 4..i * 4 + 4].try_into().unwrap() };
        let ok = if in_damage(i) {
            px(&pp).iter().zip(px(&pb)).all(|(x, y)| x.abs_diff(y) <= TOLERANCE)
        } else {
            px(&pa) == px(&pb)
        };
        if !ok {
            if bad == 0 {
                eprintln!("first bad pixel ({}, {}) inside damage: {}", i % w, i / w, in_damage(i));
            }
            bad += 1;
        }
    }
    if bad > 0 {
        let maxd = pp.iter().zip(&pb).map(|(x, y)| x.abs_diff(*y)).max().unwrap();
        eprintln!("max diff inside damage: {maxd}");
        let dir = std::env::temp_dir();
        partial.save_png(dir.join("uison_partial.png").to_str().unwrap()).unwrap();
        full_b.save_png(dir.join("uison_full.png").to_str().unwrap()).unwrap();
        eprintln!("damage: {damage:?}");
        LAST_SRC.with(|l| std::fs::write(dir.join("uison_b.json"), &*l.borrow()).unwrap());
    }
    let area: f32 = rects.iter().map(Rect::area).sum();
    (bad, area / SIZE.w / SIZE.h)
}

#[test]
fn partial_equals_full() {
    let text = Text::new();
    let mut r = Rng(0xDEADBEEF12345);
    let mut total_ratio = 0.0;
    let runs = 300;
    for run in 0..runs {
        let mut specs: Vec<Spec> = (0..30).map(|_| random_spec(&mut r, 0)).collect();
        let a = build(&specs, &text);
        mutate(&mut r, &mut specs);
        let b = build(&specs, &text);
        let scale = if run % 2 == 0 { 2.0 } else { 1.0 };
        let (bad, ratio) = check(&a, &b, &text, scale);
        assert_eq!(bad, 0, "run {run}: {bad} pixels differ (scale {scale}); see $TMPDIR/uison_*.png");
        total_ratio += ratio;
    }
    eprintln!("average damaged area: {:.1}% of frame", 100.0 * total_ratio / runs as f32);
}
