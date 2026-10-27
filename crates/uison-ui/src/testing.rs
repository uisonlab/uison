//! Testing a screen in `cargo test`, without a window or the system: tap a node by key, type text,
//! read the frame's texts. Text metrics are approximate (0.55 of the font size per character) — the layout
//! is the same as on screen, but not pixel-exact; for a pixel-exact screenshot use `uison::Headless` (macOS, Windows, Linux).
//!
//! ```ignore
//! let mut ui = app();
//! assert!(ui.tap("inc"));
//! assert_eq!(ui.data.num("n"), 1.0);
//! assert!(ui.texts().contains(&"Pressed 1 time".to_string()));
//! ```

use uison_core::{App, Event, Font, Host, Key, KeyCode, Mods, MouseButton, Point, Rect, TextMeasure, TextMetrics};
use uison_frame::{Frame, Item, Shape};

use crate::Ui;

/// Text metrics without the system.
struct Approx;

impl TextMeasure for Approx {
    fn measure(&self, t: &str, f: &Font) -> TextMetrics {
        TextMetrics { width: t.chars().count() as f32 * f.size * 0.55, ascent: f.size * 0.9, descent: f.size * 0.3, leading: 0.0 }
    }
}

impl Host for Approx {}

fn walk<'a>(items: &'a [Item], out: &mut Vec<&'a Item>) {
    for it in items {
        out.push(it);
        if let Shape::Clip { items, .. } = &it.shape {
            walk(items, out);
        }
    }
}

impl Ui {
    fn test_frame(&mut self) -> Frame {
        let src = self.render(&Approx);
        Frame::parse(&src, &Approx).unwrap_or_else(|e| panic!("uison: bad frame: {e}"))
    }

    /// For tests: press the node with this `key` — with the mouse at the center of its area (move, down, up).
    /// `false` — there is no such interactive node on screen.
    pub fn tap(&mut self, key: &str) -> bool {
        let f = self.test_frame();
        let mut all = Vec::new();
        walk(&f.items, &mut all);
        let Some(r): Option<Rect> = all.iter().rev().find_map(|it| match &it.shape {
            Shape::Hit { rect, .. } if it.id.as_deref() == Some(key) => Some(*rect),
            _ => None,
        }) else {
            return false;
        };
        let p = Point::new(r.x + r.w / 2.0, r.y + r.h / 2.0);
        let hit = uison_frame::hit_test(&f, p).map(str::to_string);
        for e in [Event::MouseMove(p), Event::MouseDown(p, MouseButton::Left), Event::MouseUp(p, MouseButton::Left)] {
            self.on_event(&e, hit.as_deref(), &Approx);
        }
        true
    }

    /// For tests: type text into the focused field (after a `tap` on the field).
    pub fn type_text(&mut self, text: &str) {
        for c in text.chars() {
            let k = Key { code: KeyCode::Char, text: Some(c.to_string().into()), mods: Mods::default() };
            self.on_event(&Event::Key(k), None, &Approx);
        }
    }

    /// For tests: press a key (`KeyCode::Enter`, `Tab`, `Escape`…).
    pub fn press_key(&mut self, code: KeyCode) {
        self.on_event(&Event::Key(Key { code, text: None, mods: Mods::default() }), None, &Approx);
    }

    /// For tests: text lines in the frame — what is visible on screen now.
    pub fn texts(&mut self) -> Vec<String> {
        let f = self.test_frame();
        let mut all = Vec::new();
        walk(&f.items, &mut all);
        all.iter()
            .filter_map(|it| match &it.shape {
                Shape::Text { text, .. } => Some(text.to_string()),
                _ => None,
            })
            .collect()
    }
}
