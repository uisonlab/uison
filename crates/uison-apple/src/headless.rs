//! Headless application: events are fed programmatically, drawing is partial,
//! into a bitmap. For automated tests, screenshots and measurements.

use std::time::{Duration, Instant};

use uison_core::{App, Canvas, Color, Event, Paint};
use uison_frame::{Damage, Error, Runtime, Update};

use crate::bitmap::Bitmap;
use crate::text::Text;

pub struct Headless {
    pub rt: Runtime,
    pub text: Text,
    pub bmp: Bitmap,
    /// Regions redrawn by the last event.
    pub last_damage: Damage,
}

impl Headless {
    pub fn new(app: impl App + 'static, scale: f32) -> Result<Headless, Error> {
        let text = Text::new();
        let rt = Runtime::new(Box::new(app), &text)?;
        let bmp = Bitmap::new(rt.frame.size, scale);
        bmp.paint(&rt.frame, &text, rt.frame.rect());
        Ok(Headless { rt, text, bmp, last_damage: Damage::Full })
    }

    /// Handle an event and redraw the changed regions.
    /// Returns pipeline statistics and drawing time.
    pub fn event(&mut self, ev: Event) -> Result<Option<(Update, Duration)>, Error> {
        let up = self.rt.dispatch(&ev, &self.text)?;
        let t0 = Instant::now();
        let frame = &self.rt.frame;
        self.last_damage = up.as_ref().map_or(Damage::None, |u| u.damage.clone());
        if frame.size != self.bmp.size {
            self.bmp = Bitmap::new(frame.size, self.bmp.scale);
            self.last_damage = Damage::Full;
        }
        for r in self.last_damage.rects(frame.rect()) {
            self.bmp.paint(frame, &self.text, r);
        }
        Ok(up.map(|u| (u, t0.elapsed())))
    }

    /// Save the current state; with `flash`, highlight the last redraws.
    pub fn save_png(&self, path: &str, flash: bool) -> Result<(), String> {
        let rects: Vec<_> = if flash { self.last_damage.rects(self.rt.frame.rect()).collect() } else { vec![] };
        let mut c = self.bmp.canvas(&self.text);
        let mark = Paint { fill: Some(Color::rgba(255, 40, 40, 70)), stroke: Some(Color::rgb(255, 0, 0)), width: 1.0 };
        for r in &rects {
            c.rect(*r, 0.0, mark);
        }
        let res = self.bmp.save_png(path);
        // Restore the clean image.
        for r in &rects {
            self.bmp.paint(&self.rt.frame, &self.text, r.inflate(1.0).round_out());
        }
        res
    }
}
