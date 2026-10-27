//! App without a window: events are fed programmatically, redraw is partial,
//! Cairo into memory. For screenshots (`studio --png`), measurements and comparison with other platforms.

use std::time::{Duration, Instant};

use uison_core::{App, Canvas, Color, Event, Paint, Rect};
use uison_frame::{Damage, Error, Runtime, Update};

use crate::canvas::CairoCanvas;
use crate::gfx::Image;
use crate::text::Text;

pub struct Headless {
    pub rt: Runtime,
    pub text: Text,
    image: Image,
    scale: f32,
    /// Areas redrawn by the last event.
    pub last_damage: Damage,
}

impl Headless {
    pub fn new(app: impl App + 'static, scale: f32) -> Result<Headless, Error> {
        let text = Text::new();
        let rt = Runtime::new(Box::new(app), &text)?;
        let image = Self::image(&rt, scale);
        let h = Headless { rt, text, image, scale, last_damage: Damage::Full };
        let full = h.rt.frame.rect();
        h.paint(&[full]);
        Ok(h)
    }

    fn image(rt: &Runtime, scale: f32) -> Image {
        let s = rt.frame.size;
        Image::new((s.w * scale).ceil() as i32, (s.h * scale).ceil() as i32)
    }

    fn paint(&self, rects: &[Rect]) {
        let mut c = CairoCanvas::new(self.image.cr, &self.text, self.scale);
        for &r in rects {
            uison_frame::paint(&self.rt.frame, &mut c, r);
        }
    }

    /// Handle an event and redraw the changed areas.
    pub fn event(&mut self, ev: Event) -> Result<Option<(Update, Duration)>, Error> {
        let old = self.rt.frame.size;
        let up = self.rt.dispatch(&ev, &self.text)?;
        let t0 = Instant::now();
        self.last_damage = up.as_ref().map_or(Damage::None, |u| u.damage.clone());
        if self.rt.frame.size != old {
            self.image = Self::image(&self.rt, self.scale);
            self.last_damage = Damage::Full;
        }
        let rects: Vec<Rect> = self.last_damage.rects(self.rt.frame.rect()).collect();
        self.paint(&rects);
        self.image.flush();
        Ok(up.map(|u| (u, t0.elapsed())))
    }

    /// Save the current state; with `flash`, highlighting the latest redraws.
    pub fn save_png(&self, path: &str, flash: bool) -> Result<(), String> {
        let rects: Vec<Rect> = if flash { self.last_damage.rects(self.rt.frame.rect()).collect() } else { vec![] };
        if !rects.is_empty() {
            let mut c = CairoCanvas::new(self.image.cr, &self.text, self.scale);
            let mark = Paint { fill: Some(Color::rgba(255, 40, 40, 70)), stroke: Some(Color::rgb(255, 0, 0)), width: 1.0 };
            for r in &rects {
                c.rect(*r, 0.0, mark);
            }
        }
        let res = self.image.save_png(path);
        // Restore the clean image.
        let clean: Vec<Rect> = rects.iter().map(|r| r.inflate(1.0).round_out()).collect();
        if !clean.is_empty() {
            self.paint(&clean);
        }
        res
    }
}
