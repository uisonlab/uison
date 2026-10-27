//! Text: CSS fonts are registered in JS once, metrics are cached in Rust
//! (crossing into JS and `measureText` are expensive — like the CTLine cache on macOS).

use std::cell::RefCell;
use std::collections::HashMap;

use uison_core::{Font, Host, Mods, TextMeasure, TextMetrics};

use crate::js;

/// Strings in one cache generation per font.
const GENERATION: usize = 4096;

struct Slot {
    font: Font,
    id: u32,
    cur: HashMap<Box<str>, TextMetrics>,
    prev: HashMap<Box<str>, TextMetrics>,
}

#[derive(Default)]
pub struct WebText {
    slots: RefCell<Vec<Slot>>,
}

/// System font: San Francisco on macOS, Segoe UI on Windows.
const SYSTEM: &str = r#"-apple-system, BlinkMacSystemFont, system-ui, "Segoe UI", Roboto, sans-serif"#;

fn css(f: &Font) -> String {
    // Generic CSS names go without quotes, otherwise it's the name of a specific font.
    let family = match f.family.as_deref() {
        None | Some("sans-serif" | "system-ui") => SYSTEM.to_string(),
        Some(g @ ("monospace" | "serif")) => g.to_string(),
        Some(fam) => format!("\"{fam}\", {SYSTEM}"),
    };
    let style = if f.italic { "italic " } else { "" };
    format!("{style}{} {}px {family}", if f.bold { 700 } else { 400 }, f.size)
}

impl WebText {
    pub fn new() -> WebText {
        WebText::default()
    }

    /// JS font index and string metrics.
    pub fn get(&self, text: &str, font: &Font) -> (u32, TextMetrics) {
        let mut slots = self.slots.borrow_mut();
        let i = match slots.iter().position(|s| s.font == *font) {
            Some(i) => i,
            None => {
                let c = css(font);
                let id = unsafe { js::font(c.as_ptr(), c.len()) };
                slots.push(Slot { font: font.clone(), id, cur: HashMap::new(), prev: HashMap::new() });
                slots.len() - 1
            }
        };
        let slot = &mut slots[i];
        if let Some(m) = slot.cur.get(text) {
            return (slot.id, *m);
        }
        let m = slot.prev.remove(text).unwrap_or_else(|| {
            let mut out = [0f32; 3];
            unsafe { js::measure(text.as_ptr(), text.len(), slot.id, out.as_mut_ptr()) };
            TextMetrics { width: out[0], ascent: out[1], descent: out[2], leading: 0.0 }
        });
        if slot.cur.len() >= GENERATION {
            slot.prev = std::mem::take(&mut slot.cur);
        }
        slot.cur.insert(text.into(), m);
        (slot.id, m)
    }
}

/// Requests to the browser: answers (paste) come as separate calls from JS.
impl Host for WebText {
    fn mods(&self) -> Mods {
        crate::mods(unsafe { js::mods() })
    }

    fn set_clipboard(&self, text: &str) {
        unsafe { js::clipboard_write(text.as_ptr(), text.len()) }
    }

    fn request_paste(&self) {
        unsafe { js::clipboard_read() }
    }

    fn open_url(&self, url: &str) -> bool {
        unsafe { js::open_url(url.as_ptr(), url.len()) != 0 }
    }

    /// `mem:` — from the header; a file — once the browser has loaded it.
    fn image_size(&self, src: &str) -> Option<uison_core::Size> {
        uison_core::image::natural_size(src).or_else(|| {
            let mut out = [0f32; 2];
            (unsafe { js::image_size(src.as_ptr(), src.len(), out.as_mut_ptr()) } != 0).then(|| uison_core::Size::new(out[0], out[1]))
        })
    }
}

impl TextMeasure for WebText {
    fn measure(&self, text: &str, font: &Font) -> TextMetrics {
        self.get(text, font).1
    }
}
