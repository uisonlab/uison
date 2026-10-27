//! Text via `android.graphics.Paint`: a paint per font, Java strings and widths
//! in a two-generation cache (like CTLine on Apple and metrics in the browser): crossing
//! into Java and `measureText` are expensive, and frames repeat the same strings.

use std::cell::RefCell;
use std::collections::HashMap;

use uison_core::{Font, Host, TextMeasure, TextMetrics};

use crate::env;
use crate::java::j;
use crate::jni::*;

/// Strings per generation per font (JNI global references — no more than 51200).
const GENERATION: usize = 2048;

/// Text paint: antialiasing, subpixel positioning, linear metrics —
/// widths are proportional to size (the canvas is scaled by screen density).
const TEXT_FLAGS: i32 = 0x01 | 0x40 | 0x80;

struct Line {
    s: jstring,
    width: f32,
}

impl Drop for Line {
    fn drop(&mut self) {
        unsafe { env().delete_global(self.s) }
    }
}

struct Slot {
    font: Font,
    paint: jobject,
    ascent: f32,
    descent: f32,
    leading: f32,
    cur: HashMap<Box<str>, Line>,
    prev: HashMap<Box<str>, Line>,
}

#[derive(Default)]
pub struct AndroidText {
    slots: RefCell<Vec<Slot>>,
}

impl AndroidText {
    pub fn new() -> AndroidText {
        AndroidText::default()
    }

    /// Font paint, Java string and string metrics.
    pub fn get(&self, text: &str, font: &Font) -> (jobject, jstring, TextMetrics) {
        let e = env();
        let j = j();
        let mut slots = self.slots.borrow_mut();
        let i = match slots.iter().position(|s| s.font == *font) {
            Some(i) => i,
            None => {
                slots.push(unsafe { new_slot(e, font) });
                slots.len() - 1
            }
        };
        let slot = &mut slots[i];
        let m = |w: f32, s: &Slot| TextMetrics { width: w, ascent: s.ascent, descent: s.descent, leading: s.leading };
        if let Some(l) = slot.cur.get(text) {
            return (slot.paint, l.s, m(l.width, slot));
        }
        let line = slot.prev.remove(text).unwrap_or_else(|| unsafe {
            let local = e.new_string(text);
            let width = e.call_float(slot.paint, j.measure_text, &[l(local)]);
            let s = e.global(local);
            e.delete_local(local);
            Line { s, width }
        });
        if slot.cur.len() >= GENERATION {
            slot.prev = std::mem::take(&mut slot.cur);
        }
        let (s, w) = (line.s, line.width);
        slot.cur.insert(text.into(), line);
        (slot.paint, s, m(w, slot))
    }
}

unsafe fn new_slot(e: Env, font: &Font) -> Slot {
    let j = j();
    unsafe {
        let paint = e.new_object(j.paint_cls, j.paint_new, &[i(TEXT_FLAGS)]);
        // Typeface: BOLD = 1, ITALIC = 2; Android understands `monospace`, `serif`, `sans-serif` itself.
        let style = font.bold as i32 | (font.italic as i32) << 1;
        let tf = match font.family.as_deref().filter(|f| *f != "system-ui") {
            None if style == 1 => j.typeface_bold,
            None if style == 0 => j.typeface_default,
            name => {
                let n = name.map_or(std::ptr::null_mut(), |n| e.new_string(n));
                let t = e.call_static_object(j.typeface_cls, j.typeface_create, &[l(n), i(style)]);
                if !n.is_null() {
                    e.delete_local(n);
                }
                t
            }
        };
        let old = e.call_object(paint, j.set_typeface, &[l(tf)]);
        if !old.is_null() {
            e.delete_local(old);
        }
        e.call_void(paint, j.set_text_size, &[f(font.size)]);
        let fm = e.new_object(j.fm_cls, j.fm_new, &[]);
        e.call_float(paint, j.get_font_metrics, &[l(fm)]);
        // In Android ascent is negative (up from the baseline).
        let slot = Slot {
            font: font.clone(),
            paint,
            ascent: -e.float_field(fm, j.fm_ascent),
            descent: e.float_field(fm, j.fm_descent),
            leading: e.float_field(fm, j.fm_leading),
            cur: HashMap::new(),
            prev: HashMap::new(),
        };
        e.delete_global(fm);
        slot
    }
}

impl Host for AndroidText {}

impl TextMeasure for AndroidText {
    fn measure(&self, text: &str, font: &Font) -> TextMetrics {
        self.get(text, font).2
    }
}
