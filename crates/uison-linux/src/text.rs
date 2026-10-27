//! Text via Pango: a font description per `Font`, a layout (`PangoLayout`) per line,
//! in a two-generation cache, like CTLine on Apple and `IDWriteTextLayout` on Windows.
//! Color is not part of the layout: it is set on the Cairo source when drawing.

use std::cell::RefCell;
use std::collections::HashMap;
use std::ptr::null_mut;

use uison_core::{Font, Host, TextMeasure, TextMetrics};

use crate::gfx::*;

/// Lines per generation per font.
const GENERATION: usize = 2048;

struct Line {
    layout: *mut PangoLayout,
    metrics: TextMetrics,
}

struct Slot {
    font: Font,
    desc: *mut FontDesc,
    cur: HashMap<Box<str>, Line>,
    prev: HashMap<Box<str>, Line>,
}

pub struct Text {
    ctx: *mut PangoContext,
    /// System font family: `UISON_FONT`, the desktop font (portal, `set_system`)
    /// or fontconfig's `sans-serif` (Noto Sans, DejaVu Sans...).
    system: String,
    /// `UISON_FONT` is set: desktop settings do not override it.
    fixed: bool,
    slots: RefCell<Vec<Slot>>,
}

impl Text {
    pub fn new() -> Text {
        let g = gfx();
        unsafe {
            let ctx = g.pango.pango_font_map_create_context(g.pc.pango_cairo_font_map_get_default());
            // Layout does not depend on the scale: without metric hinting and glyph position rounding
            // widths in points are the same at 1, 1.25 and 2, so the frame is laid out once.
            let o = g.cairo.cairo_font_options_create();
            g.cairo.cairo_font_options_set_hint_metrics(o, HINT_METRICS_OFF);
            g.cairo.cairo_font_options_set_hint_style(o, HINT_STYLE_SLIGHT);
            g.cairo.cairo_font_options_set_antialias(o, ANTIALIAS_GRAY);
            g.pc.pango_cairo_context_set_font_options(ctx, o);
            if let Some(f) = g.pango.pango_context_set_round_glyph_positions {
                f(ctx, 0);
            }
            let fixed = std::env::var("UISON_FONT").ok().filter(|f| !f.is_empty());
            let system = fixed.clone().unwrap_or_else(|| "sans-serif".into());
            Text { ctx, system, fixed: fixed.is_some(), slots: RefCell::new(Vec::new()) }
        }
    }

    /// Desktop font (`Ubuntu Sans 11`, `Cantarell Bold 11`: a Pango description; we take the family).
    /// `true` if the family changed: line layouts are dropped and the frame must be laid out again.
    pub fn set_system(&mut self, desc: &str) -> bool {
        let p = &gfx().pango;
        let Ok(c) = std::ffi::CString::new(desc) else { return false };
        let family = unsafe {
            let d = p.pango_font_description_from_string(c.as_ptr());
            let f = p.pango_font_description_get_family(d);
            let s = (!f.is_null()).then(|| std::ffi::CStr::from_ptr(f).to_string_lossy().into_owned());
            p.pango_font_description_free(d);
            s
        };
        let Some(family) = family.filter(|f| !f.is_empty()) else { return false };
        if self.fixed || family == self.system {
            return false;
        }
        self.system = family;
        for slot in self.slots.get_mut().drain(..) {
            for (_, line) in slot.cur.into_iter().chain(slot.prev) {
                unsafe { gfx().gobject.g_object_unref(line.layout) };
            }
            unsafe { p.pango_font_description_free(slot.desc) };
        }
        true
    }

    fn desc(&self, f: &Font) -> *mut FontDesc {
        let p = &gfx().pango;
        // Pango understands `monospace`, `serif`, `sans-serif` itself.
        let family = format!("{}\0", f.family.as_deref().filter(|f| *f != "system-ui").unwrap_or(&self.system));
        unsafe {
            let d = p.pango_font_description_new();
            p.pango_font_description_set_family(d, family.as_ptr().cast());
            // Absolute size in device units = points: no DPI conversion (72 / 96).
            p.pango_font_description_set_absolute_size(d, f.size as f64 * PANGO_SCALE);
            p.pango_font_description_set_weight(d, if f.bold { 700 } else { 400 });
            if f.italic {
                p.pango_font_description_set_style(d, 2); // PANGO_STYLE_ITALIC
            }
            d
        }
    }

    /// Call `f` with the line's layout and its metrics.
    pub fn with_line<R>(&self, text: &str, font: &Font, f: impl FnOnce(*mut PangoLayout, &TextMetrics) -> R) -> R {
        let mut slots = self.slots.borrow_mut();
        let i = match slots.iter().position(|s| s.font == *font) {
            Some(i) => i,
            None => {
                slots.push(Slot { font: font.clone(), desc: self.desc(font), cur: HashMap::new(), prev: HashMap::new() });
                slots.len() - 1
            }
        };
        let slot = &mut slots[i];
        if let Some(line) = slot.cur.get(text) {
            return f(line.layout, &line.metrics);
        }
        let line = match slot.prev.remove(text) {
            Some(line) => line,
            None => self.layout(slot.desc, text),
        };
        if slot.cur.len() >= GENERATION {
            for (_, old) in std::mem::replace(&mut slot.prev, std::mem::take(&mut slot.cur)) {
                unsafe { gfx().gobject.g_object_unref(old.layout) };
            }
        }
        let line = slot.cur.entry(text.into()).or_insert(line);
        f(line.layout, &line.metrics)
    }

    fn layout(&self, desc: *mut FontDesc, text: &str) -> Line {
        let p = &gfx().pango;
        unsafe {
            let layout = p.pango_layout_new(self.ctx);
            p.pango_layout_set_font_description(layout, desc);
            p.pango_layout_set_text(layout, text.as_ptr().cast(), text.len() as i32);
            let mut logical = PangoRect::default();
            p.pango_layout_get_extents(layout, null_mut(), &mut logical);
            let baseline = p.pango_layout_get_baseline(layout) as f64 / PANGO_SCALE;
            let height = logical.height as f64 / PANGO_SCALE;
            let metrics = TextMetrics {
                width: (logical.width as f64 / PANGO_SCALE) as f32,
                ascent: baseline as f32,
                descent: (height - baseline) as f32,
                leading: 0.0,
            };
            Line { layout, metrics }
        }
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
