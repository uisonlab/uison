//! Text via CoreText: fonts and lines (CTLine) with a cache.
//!
//! Creating a CTLine is the most expensive pipeline step (~4 µs per line),
//! and frames mostly repeat the same lines. The cache has two generations: on
//! overflow the current generation becomes the previous one and the old one
//! is freed, so only recently used lines stay alive.

use std::cell::RefCell;
use std::collections::HashMap;
use std::ptr::null;

use uison_core::{Font, Host, TextMeasure, TextMetrics};

use crate::ffi::*;

/// Lines per generation per font.
const GENERATION: usize = 2048;

struct Line {
    ct: CTLineRef,
    metrics: TextMetrics,
}

impl Drop for Line {
    fn drop(&mut self) {
        unsafe { CFRelease(self.ct) }
    }
}

struct Slot {
    font: Font,
    ct: CTFontRef,
    cur: HashMap<Box<str>, Line>,
    prev: HashMap<Box<str>, Line>,
}

impl Drop for Slot {
    fn drop(&mut self) {
        self.cur.clear();
        self.prev.clear();
        unsafe { CFRelease(self.ct) }
    }
}

#[derive(Default)]
pub struct Text {
    /// There are usually only a few fonts, so a linear search without key allocations.
    slots: RefCell<Vec<Slot>>,
}

impl Text {
    pub fn new() -> Text {
        Text::default()
    }

    /// Call `f` with the CoreText line and its metrics.
    pub fn with_line<R>(&self, text: &str, font: &Font, f: impl FnOnce(CTLineRef, &TextMetrics) -> R) -> R {
        let mut slots = self.slots.borrow_mut();
        let i = match slots.iter().position(|s| s.font == *font) {
            Some(i) => i,
            None => {
                let ct = unsafe { create_font(font) };
                slots.push(Slot { font: font.clone(), ct, cur: HashMap::new(), prev: HashMap::new() });
                slots.len() - 1
            }
        };
        let slot = &mut slots[i];
        if let Some(line) = slot.cur.get(text) {
            return f(line.ct, &line.metrics);
        }
        let line = match slot.prev.remove(text) {
            Some(line) => line,
            None => Line::new(slot.ct, text),
        };
        if slot.cur.len() >= GENERATION {
            slot.prev = std::mem::take(&mut slot.cur);
        }
        let line = slot.cur.entry(text.into()).or_insert(line);
        f(line.ct, &line.metrics)
    }
}

impl Line {
    fn new(font: CTFontRef, text: &str) -> Line {
        unsafe {
            let keys = [kCTFontAttributeName, kCTForegroundColorFromContextAttributeName];
            let values = [font, kCFBooleanTrue];
            let attrs = CFDictionaryCreate(
                null(),
                keys.as_ptr(),
                values.as_ptr(),
                2,
                (&raw const kCFTypeDictionaryKeyCallBacks).cast(),
                (&raw const kCFTypeDictionaryValueCallBacks).cast(),
            );
            let s = cfstring(text);
            let attr = CFAttributedStringCreate(null(), s, attrs);
            // Color is not part of the line: it comes from the context fill color,
            // so one CTLine works for any color.
            let ct = CTLineCreateWithAttributedString(attr);
            CFRelease(attr);
            CFRelease(s);
            CFRelease(attrs);
            let (mut ascent, mut descent, mut leading) = (0.0, 0.0, 0.0);
            let width = CTLineGetTypographicBounds(ct, &mut ascent, &mut descent, &mut leading);
            let metrics =
                TextMetrics { width: width as f32, ascent: ascent as f32, descent: descent as f32, leading: leading as f32 };
            Line { ct, metrics }
        }
    }
}

impl Host for Text {}

impl TextMeasure for Text {
    fn measure(&self, text: &str, font: &Font) -> TextMetrics {
        self.with_line(text, font, |_, m| *m)
    }
}

unsafe fn create_font(f: &Font) -> CTFontRef {
    let size = f.size as f64;
    unsafe {
        // Generic CSS names: system, monospace, serif.
        let (font, bold_done) = match f.family.as_deref() {
            None | Some("sans-serif" | "system-ui") => {
                let kind = if f.bold { kCTFontUIFontEmphasizedSystem } else { kCTFontUIFontSystem };
                (CTFontCreateUIFontForLanguage(kind, size, null()), true)
            }
            Some("monospace") => (CTFontCreateUIFontForLanguage(kCTFontUIFontUserFixedPitch, size, null()), false),
            Some(name) => {
                let n = cfstring(if name == "serif" { "Times New Roman" } else { name });
                let font = CTFontCreateWithName(n, size, null());
                CFRelease(n);
                (font, false)
            }
        };
        let traits = (if f.bold && !bold_done { kCTFontBoldTrait } else { 0 }) | (if f.italic { kCTFontItalicTrait } else { 0 });
        if traits == 0 {
            return font;
        }
        let styled = CTFontCreateCopyWithSymbolicTraits(font, size, null(), traits, traits);
        if styled.is_null() {
            font // the family has no such face
        } else {
            CFRelease(font);
            styled
        }
    }
}
