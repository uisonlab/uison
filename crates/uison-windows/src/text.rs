//! Text via DirectWrite: a format (`IDWriteTextFormat`) per font, a layout
//! (`IDWriteTextLayout`) per line — in a two-generation cache, like CTLine on Apple:
//! layout is expensive, and frames mostly repeat the same lines.
//! Color is not part of the layout — it belongs to the brush at draw time.

use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::c_void;
use std::ptr::null_mut;

use uison_core::{Font, Host, TextMeasure, TextMetrics};

use crate::com::{Com, GUID, HRESULT, HrError, com, create};
use crate::ffi::wide;

/// Lines per generation per font.
const GENERATION: usize = 2048;

const IID_IDWRITEFACTORY: GUID = GUID(0xb859ee5a, 0xd838, 0x4b5b, [0xa2, 0xe8, 0x1a, 0xdc, 0x7d, 0x93, 0xdb, 0x48]);
const DWRITE_FONT_WEIGHT_NORMAL: u32 = 400;
const DWRITE_FONT_WEIGHT_BOLD: u32 = 700;
const DWRITE_FONT_STRETCH_NORMAL: u32 = 5;
const DWRITE_WORD_WRAPPING_NO_WRAP: u32 = 1;
/// Width and height of the layout box: a single line without wrapping — it just has to fit.
const UNBOUNDED: f32 = 1.0e6;

#[repr(C)]
#[derive(Default)]
struct DwTextMetrics {
    left: f32,
    top: f32,
    width: f32,
    width_including_trailing_whitespace: f32,
    height: f32,
    layout_width: f32,
    layout_height: f32,
    max_bidi_reordering_depth: u32,
    line_count: u32,
}

#[repr(C)]
#[derive(Default, Clone, Copy)]
struct DwLineMetrics {
    length: u32,
    trailing_whitespace_length: u32,
    newline_length: u32,
    height: f32,
    baseline: f32,
    is_trimmed: i32,
}

#[link(name = "dwrite")]
unsafe extern "system" {
    fn DWriteCreateFactory(kind: u32, iid: *const GUID, out: *mut *mut c_void) -> HRESULT;
}

struct Line {
    layout: Com,
    metrics: TextMetrics,
}

struct Slot {
    font: Font,
    format: Com,
    cur: HashMap<Box<str>, Line>,
    prev: HashMap<Box<str>, Line>,
}

pub struct Text {
    factory: Com,
    /// System font: (below 20 pt, from 20 pt) — Segoe UI Variable has optical sizes
    /// as separate families (Text and Display), like San Francisco.
    system: (Vec<u16>, Vec<u16>),
    /// There are usually only a few fonts — linear search without key allocations.
    slots: RefCell<Vec<Slot>>,
}

impl Text {
    /// DirectWrite exists in every supported Windows — without it there is no way to draw text.
    pub fn new() -> Text {
        Text::try_new().expect("DirectWrite")
    }

    pub fn try_new() -> Result<Text, HrError> {
        // Shared factory: fonts and caches are shared with the system.
        let factory = create(|out| unsafe { DWriteCreateFactory(0, &IID_IDWRITEFACTORY, out) })?;
        let mut t = Text { factory, system: (wide("Segoe UI"), wide("Segoe UI")), slots: RefCell::new(Vec::new()) };
        if t.has_family("Segoe UI Variable Text") {
            t.system = (wide("Segoe UI Variable Text"), wide("Segoe UI Variable Display"));
        }
        Ok(t)
    }

    /// Whether the family is in the system font collection.
    fn has_family(&self, name: &str) -> bool {
        let Ok(coll) = create(|out| unsafe { com!(self.factory, 3, fn(*mut *mut c_void, i32) -> HRESULT, out, 0) }) else {
            return false;
        };
        let (mut index, mut exists) = (0u32, 0i32);
        let name = wide(name);
        let hr = unsafe { com!(coll, 5, fn(*const u16, *mut u32, *mut i32) -> HRESULT, name.as_ptr(), &mut index, &mut exists) };
        hr >= 0 && exists != 0
    }

    fn format(&self, f: &Font) -> Result<Com, HrError> {
        // Generic CSS names → Windows fonts.
        let family = match f.family.as_deref() {
            Some("monospace") => wide("Consolas"),
            Some("serif") => wide("Times New Roman"),
            Some(name) if name != "sans-serif" && name != "system-ui" => wide(name),
            _ if f.size >= 20.0 => self.system.1.clone(),
            _ => self.system.0.clone(),
        };
        let weight = if f.bold { DWRITE_FONT_WEIGHT_BOLD } else { DWRITE_FONT_WEIGHT_NORMAL };
        // DWRITE_FONT_STYLE_ITALIC
        let style = if f.italic { 2 } else { 0 };
        let locale = wide("en-us");
        let format = create(|out| unsafe {
            com!(self.factory, 15, fn(*const u16, *mut c_void, u32, u32, u32, f32, *const u16, *mut *mut c_void) -> HRESULT,
                family.as_ptr(), null_mut(), weight, style, DWRITE_FONT_STRETCH_NORMAL, f.size, locale.as_ptr(), out)
        })?;
        unsafe { com!(format, 5, fn(u32) -> HRESULT, DWRITE_WORD_WRAPPING_NO_WRAP) }; // SetWordWrapping
        Ok(format)
    }

    /// Call `f` with the line's layout and its metrics. A line that failed to
    /// lay out cannot be drawn — `f` gets `None`.
    pub fn with_line<R>(&self, text: &str, font: &Font, f: impl FnOnce(Option<&Com>, &TextMetrics) -> R) -> R {
        let mut slots = self.slots.borrow_mut();
        let i = match slots.iter().position(|s| s.font == *font) {
            Some(i) => i,
            None => match self.format(font) {
                Ok(format) => {
                    slots.push(Slot { font: font.clone(), format, cur: HashMap::new(), prev: HashMap::new() });
                    slots.len() - 1
                }
                Err(_) => return f(None, &TextMetrics::default()),
            },
        };
        let slot = &mut slots[i];
        if let Some(line) = slot.cur.get(text) {
            return f(Some(&line.layout), &line.metrics);
        }
        let line = match slot.prev.remove(text) {
            Some(line) => line,
            None => match self.layout(&slot.format, text) {
                Ok(line) => line,
                Err(_) => return f(None, &TextMetrics::default()),
            },
        };
        if slot.cur.len() >= GENERATION {
            slot.prev = std::mem::take(&mut slot.cur);
        }
        let line = slot.cur.entry(text.into()).or_insert(line);
        f(Some(&line.layout), &line.metrics)
    }

    fn layout(&self, format: &Com, text: &str) -> Result<Line, HrError> {
        let s: Vec<u16> = text.encode_utf16().collect();
        let layout = create(|out| unsafe {
            com!(self.factory, 18, fn(*const u16, u32, *mut c_void, f32, f32, *mut *mut c_void) -> HRESULT,
                s.as_ptr(), s.len() as u32, format.ptr(), UNBOUNDED, UNBOUNDED, out)
        })?;
        let mut tm = DwTextMetrics::default();
        let mut lm = DwLineMetrics::default();
        let mut n = 0u32;
        unsafe {
            com!(layout, 60, fn(*mut DwTextMetrics) -> HRESULT, &mut tm); // GetMetrics
            com!(layout, 59, fn(*mut DwLineMetrics, u32, *mut u32) -> HRESULT, &mut lm, 1, &mut n); // GetLineMetrics
        }
        // Line height comes from the font (ascent + descent + lineGap), baseline from the top.
        let metrics = TextMetrics {
            width: tm.width_including_trailing_whitespace,
            ascent: lm.baseline,
            descent: lm.height - lm.baseline,
            leading: 0.0,
        };
        Ok(Line { layout, metrics })
    }
}

impl Default for Text {
    fn default() -> Text {
        Text::new()
    }
}

impl Host for Text {}

impl TextMeasure for Text {
    fn measure(&self, text: &str, font: &Font) -> TextMetrics {
        self.with_line(text, font, |_, m| *m)
    }
}
