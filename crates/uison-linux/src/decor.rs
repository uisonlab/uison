//! Our own window frame (CSD), when the compositor does not draw one (GNOME/Mutter, Weston):
//! a title bar with buttons, a shadow and edges for resizing. Drawn with the same primitives
//! over the window buffer; the app gets the size without the title bar and unshifted coordinates.

use std::time::{Duration, Instant};

use uison_core::{Canvas, Color, Font, Paint, Point, Rect, TextMeasure};

use crate::canvas::CairoCanvas;
use crate::gfx::{Cr, OPERATOR_OVER, OPERATOR_SOURCE, gfx};
use crate::text::Text;

/// Title bar height and shadow width (the resize margin). Multiples of 4 so that at scales
/// 1.25 / 1.5 / 1.75 the content offset is whole pixels (line snapping stays aligned).
pub const TITLE: f32 = 36.0;
pub const SHADOW: f32 = 20.0;
const CORNER: f32 = 10.0;
const BUTTON: f32 = 24.0;
/// Resize corners are wider than the shadow itself: easier to grab.
const GRIP: f32 = 16.0;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Part {
    Title,
    Minimize,
    Maximize,
    Close,
    /// Edge for `xdg_toplevel.resize` (bits: top 1, bottom 2, left 4, right 8).
    Edge(u32),
}

#[derive(Default)]
pub struct Decor {
    /// We draw the frame ourselves (the compositor gave no server-side one).
    pub on: bool,
    pub maximized: bool,
    pub fullscreen: bool,
    pub tiled: bool,
    pub active: bool,
    pub hover: Option<Part>,
    pub pressed: Option<Part>,
    pub title: String,
    last_click: Option<Instant>,
}

impl Decor {
    pub fn new(title: &str) -> Decor {
        Decor { title: title.to_string(), active: true, ..Decor::default() }
    }

    /// Shadow and resize margin only on a normal (not maximized) window.
    pub fn margin(&self) -> f32 {
        if self.on && !self.maximized && !self.fullscreen && !self.tiled { SHADOW } else { 0.0 }
    }

    pub fn title_h(&self) -> f32 {
        if self.on && !self.fullscreen { TITLE } else { 0.0 }
    }

    /// Top-left corner of the app content on the surface.
    pub fn offset(&self) -> Point {
        Point::new(self.margin(), self.margin() + self.title_h())
    }

    /// Surface size (points) from the content size.
    pub fn surface(&self, content: (i32, i32)) -> (i32, i32) {
        let m = self.margin() as i32;
        (content.0 + 2 * m, content.1 + self.title_h() as i32 + 2 * m)
    }

    /// Window geometry for the compositor (without the shadow): x, y, w, h.
    pub fn geometry(&self, content: (i32, i32)) -> (i32, i32, i32, i32) {
        let m = self.margin() as i32;
        (m, m, content.0, content.1 + self.title_h() as i32)
    }

    fn buttons(&self, w: f32) -> [(Part, Point); 3] {
        let m = self.margin();
        let y = m + TITLE / 2.0;
        let x = m + w - 8.0 - BUTTON / 2.0;
        let step = BUTTON + 8.0;
        [(Part::Close, Point::new(x, y)), (Part::Maximize, Point::new(x - step, y)), (Part::Minimize, Point::new(x - 2.0 * step, y))]
    }

    /// What is under a surface point; `None` means app content.
    pub fn part_at(&self, p: Point, content: (i32, i32)) -> Option<Part> {
        if !self.on {
            return None;
        }
        let m = self.margin();
        let (w, h) = (content.0 as f32, content.1 as f32 + self.title_h());
        if m > 0.0 {
            let (x0, y0, x1, y1) = (m, m, m + w, m + h);
            let (left, right, top, bottom) = (p.x < x0, p.x >= x1, p.y < y0, p.y >= y1);
            if left || right || top || bottom {
                let (nl, nr, nt, nb) = (p.x < x0 + GRIP, p.x >= x1 - GRIP, p.y < y0 + GRIP, p.y >= y1 - GRIP);
                let mut e = 0;
                if top || (nt && (left || right)) {
                    e |= 1;
                }
                if bottom || (nb && (left || right)) {
                    e |= 2;
                }
                if left || (nl && (top || bottom)) {
                    e |= 4;
                }
                if right || (nr && (top || bottom)) {
                    e |= 8;
                }
                return Some(Part::Edge(e));
            }
        }
        if self.title_h() > 0.0 && p.y < m + self.title_h() {
            for (part, c) in self.buttons(w) {
                if (p.x - c.x).hypot(p.y - c.y) <= BUTTON / 2.0 + 2.0 {
                    return Some(part);
                }
            }
            return Some(Part::Title);
        }
        None
    }

    /// Press in the title bar: a second one within 400 ms is a double click.
    pub fn double_click(&mut self) -> bool {
        let now = Instant::now();
        let dbl = self.last_click.is_some_and(|t| now - t < Duration::from_millis(400));
        self.last_click = if dbl { None } else { Some(now) };
        dbl
    }

    /// Draw the frame inside `clip` (surface points).
    pub fn paint(&self, cr: *mut Cr, scale: f32, clip: Rect, content: (i32, i32), dark: bool, text: &Text) {
        let c = &gfx().cairo;
        let s = scale as f64;
        unsafe {
            // The shadow margin and title bar corners are transparent: clear first.
            c.cairo_save(cr);
            c.cairo_rectangle(cr, clip.x as f64 * s, clip.y as f64 * s, clip.w as f64 * s, clip.h as f64 * s);
            c.cairo_clip(cr);
            c.cairo_set_operator(cr, OPERATOR_SOURCE);
            c.cairo_set_source_rgba(cr, 0.0, 0.0, 0.0, 0.0);
            c.cairo_paint(cr);
            c.cairo_set_operator(cr, OPERATOR_OVER);
        }
        {
            let mut cv = CairoCanvas::new(cr, text, scale);
            let m = self.margin();
            let (w, h) = (content.0 as f32, content.1 as f32 + self.title_h());
            let win = Rect::new(m, m, w, h);
            if m > 0.0 {
                // Soft shadow: several translucent layers, a bit larger below the window.
                let n = 8;
                for k in (1..=n).rev() {
                    let d = m * k as f32 / n as f32;
                    let r = Rect::new(win.x - d * 0.8, win.y - d * 0.6, win.w + d * 1.6, win.h + d * 1.4);
                    let a = if self.active { 9 } else { 6 };
                    cv.rect(r, CORNER + d, Paint::fill(Color::rgba(0, 0, 0, a)));
                }
            }
            if self.title_h() > 0.0 {
                let (bg, fg) = match (dark, self.active) {
                    (false, true) => (Color::rgb(0xeb, 0xeb, 0xeb), Color::rgb(0x2e, 0x34, 0x36)),
                    (false, false) => (Color::rgb(0xfa, 0xfa, 0xfa), Color::rgb(0x92, 0x95, 0x95)),
                    (true, true) => (Color::rgb(0x30, 0x30, 0x30), Color::rgb(0xff, 0xff, 0xff)),
                    (true, false) => (Color::rgb(0x24, 0x24, 0x24), Color::rgb(0x91, 0x91, 0x91)),
                };
                let bar = Rect::new(m, m, w, TITLE);
                let radius = if m > 0.0 { CORNER } else { 0.0 };
                cv.rect(bar, radius, Paint::fill(bg));
                // Only the top corners are rounded.
                cv.rect(Rect::new(m, m + TITLE / 2.0, w, TITLE / 2.0), 0.0, Paint::fill(bg));
                let line = if dark { Color::rgba(0, 0, 0, 90) } else { Color::rgba(0, 0, 0, 30) };
                cv.line(Point::new(m, m + TITLE - 0.5), Point::new(m + w, m + TITLE - 0.5), 1.0, line, false);

                let font = Font { family: None, size: 14.0, bold: true, italic: false };
                let buttons = self.buttons(w);
                let room = (buttons[2].1.x - BUTTON / 2.0 - 8.0 - (m + 8.0)).max(0.0);
                let tm = text.measure(&self.title, &font);
                let tx = (m + (w - tm.width) / 2.0).max(m + 8.0);
                cv.push_clip(Rect::new(m + 8.0, m, room, TITLE));
                cv.text(Point::new(tx, m + (TITLE - tm.height()) / 2.0), &self.title, &font, fg);
                cv.pop_clip();

                for (part, at) in buttons {
                    let hover = self.hover == Some(part);
                    let pressed = self.pressed == Some(part) && hover;
                    let tint = if dark { 255 } else { 0 };
                    let a = if pressed { 64 } else if hover { 36 } else { 14 };
                    cv.circle(at, BUTTON / 2.0, Paint::fill(Color::rgba(tint, tint, tint, a)));
                    let (x, y) = (at.x, at.y);
                    match part {
                        Part::Close => {
                            cv.line(Point::new(x - 4.0, y - 4.0), Point::new(x + 4.0, y + 4.0), 1.5, fg, true);
                            cv.line(Point::new(x + 4.0, y - 4.0), Point::new(x - 4.0, y + 4.0), 1.5, fg, true);
                        }
                        Part::Maximize if self.maximized => {
                            cv.rect(Rect::new(x - 2.5, y - 4.5, 7.0, 7.0), 1.0, Paint::stroke(fg, 1.0));
                            cv.rect(Rect::new(x - 4.5, y - 2.5, 7.0, 7.0), 1.0, Paint { fill: Some(bg), stroke: Some(fg), width: 1.0 });
                        }
                        Part::Maximize => cv.rect(Rect::new(x - 4.5, y - 4.5, 9.0, 9.0), 1.0, Paint::stroke(fg, 1.0)),
                        _ => cv.line(Point::new(x - 4.5, y + 3.5), Point::new(x + 4.5, y + 3.5), 1.0, fg, false),
                    }
                }
            }
        }
        unsafe { c.cairo_restore(cr) };
    }
}
