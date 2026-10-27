//! Element library measurements on real screens (Task2 experiments).
//! `cargo run --release -p uibench`

use std::time::{Duration, Instant};

use uison::ui::{Ui, WrapMode};
use uison::{App, Event, Frame, MouseButton, Shape, Text, diff, hit_areas};

const SCREENS: [(&str, &str); 3] = [
    ("form2", include_str!("../../form2/form.json")),
    ("settings", include_str!("../../settings/settings.json")),
    ("gallery", include_str!("../../gallery/gallery.json")),
];

fn median(mut v: Vec<Duration>) -> f64 {
    v.sort();
    v[v.len() / 2].as_secs_f64() * 1e6
}

fn texts(f: &Frame) -> Vec<String> {
    fn walk(items: &[uison::Item], out: &mut Vec<String>) {
        for it in items {
            match &it.shape {
                Shape::Text { text, .. } => out.push(text.to_string()),
                Shape::Clip { items, .. } => walk(items, out),
                _ => {}
            }
        }
    }
    let mut out = Vec::new();
    walk(&f.items, &mut out);
    out
}

fn main() {
    let text = Text::new();
    println!("screen     nodes prims   bytes | expand layout   emit  parse |  full  hover→frame | text share");
    for (name, src) in SCREENS {
        let mut ui = Ui::new(src).unwrap();
        let first = ui.render(&text);
        let frame = Frame::parse(&first, &text).unwrap();
        let (mut ex, mut la, mut em, mut pa, mut full) = (vec![], vec![], vec![], vec![], vec![]);
        for _ in 0..40 {
            let t = Instant::now();
            let s = ui.render(&text);
            full.push(t.elapsed());
            let t = Instant::now();
            let _ = Frame::parse(&s, &text).unwrap();
            pa.push(t.elapsed());
            ex.push(ui.stats.expand);
            la.push(ui.stats.layout);
            em.push(ui.stats.emit);
        }
        // Hover: each hit area in turn; the full path event → frame → parse → diff.
        let areas: Vec<(String, uison::Rect)> = hit_areas(&frame).into_iter().map(|(id, r)| (id.to_string(), r)).collect();
        let mut hover = vec![];
        let mut cur = frame;
        for (id, r) in areas.iter().cycle().take(60) {
            let p = uison::Point::new(r.x + r.w / 2.0, r.y + r.h / 2.0);
            let t = Instant::now();
            if let Some(s) = ui.on_event(&Event::MouseMove(p), Some(id), &text) {
                let f = Frame::parse(&s, &text).unwrap();
                let _ = diff(&cur, &f);
                cur = f;
                hover.push(t.elapsed());
            }
        }
        let s = ui.stats;
        let (e, l, m, p, f) = (median(ex), median(la), median(em), median(pa), median(full));
        println!(
            "{name:<10} {:>5} {:>5} {:>7} | {e:>6.0} {l:>6.0} {m:>6.0} {p:>6.0} | {f:>5.0} {:>11.0} | {:>5.0}%",
            s.nodes,
            s.prims,
            s.bytes,
            median(hover),
            100.0 * (m + p) / (f + p)
        );
    }

    // Experiment 3: wrapping by summing word widths vs measuring the whole line.
    // The paragraph mixes Cyrillic and Latin on purpose: breaks are compared on both scripts.
    println!("\nwrap mode: line breaks and time on long texts (CoreText)");
    let para = "Библиотека элементов собирает интерфейс из прямоугольников, линий, кругов, треугольников и текста. \
                Каждый кадр — это примитивный JSON, который бэкенд рисует нативно и быстро, перерисовывая только то, что изменилось. \
                Kerning, ligatures and other typography details are handled by CoreText.";
    let mut diffs = 0;
    let mut total = 0;
    let mut times = [Duration::ZERO; 2];
    for w in (120..=520).step_by(10) {
        let mut lines = Vec::new();
        for (i, mode) in [WrapMode::WordSum, WrapMode::Exact].into_iter().enumerate() {
            let src = format!(r#"{{ size: [{w}, 2000], root: {{ Box: {{ w: "fill", child: {{ Text: {{ text: "{para}" }} }} }} }} }}"#);
            let mut ui = Ui::new(&src).unwrap();
            ui.wrap = mode;
            ui.render(&text); // warm up the cache
            let t = Instant::now();
            let s = ui.render(&text);
            times[i] += t.elapsed();
            lines.push(texts(&Frame::parse(&s, &text).unwrap()));
        }
        total += 1;
        if lines[0] != lines[1] {
            diffs += 1;
        }
    }
    println!(
        "  widths 120..520: different breaks in {diffs} of {total}; time WordSum {:.0} us, Exact {:.0} us (sum over widths)",
        times[0].as_secs_f64() * 1e6,
        times[1].as_secs_f64() * 1e6
    );
    let _ = MouseButton::Left;
}
