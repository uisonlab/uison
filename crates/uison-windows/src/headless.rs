//! Headless app: events are fed programmatically, drawing is partial,
//! Direct2D into a WIC bitmap. For screenshots (`studio --png`), measurements and comparison with macOS.

use std::ptr::null_mut;
use std::time::{Duration, Instant};

use uison_core::{App, Canvas, Color, Event, Paint, Rect};
use uison_frame::{Damage, Error, Runtime, Update};

use crate::canvas::{D2dCanvas, Resources};
use crate::com::{Com, HrError};
use crate::d2d::{Factory, RenderTarget};
use crate::ffi::{COINIT_APARTMENTTHREADED, CoInitializeEx};
use crate::text::Text;
use crate::wic::{Pixels, Wic};

pub struct Headless {
    pub rt: Runtime,
    pub text: Text,
    factory: Factory,
    wic: Wic,
    bitmap: Com,
    target: RenderTarget,
    res: Resources,
    scale: f32,
    /// Regions redrawn by the last event.
    pub last_damage: Damage,
}

fn com_err(e: HrError) -> Error {
    Error { line: 0, col: 0, msg: format!("headless: {e}").into() }
}

impl Headless {
    pub fn new(app: impl App + 'static, scale: f32) -> Result<Headless, Error> {
        unsafe { CoInitializeEx(null_mut(), COINIT_APARTMENTTHREADED) };
        let text = Text::new();
        let rt = Runtime::new(Box::new(app), &text)?;
        let factory = Factory::new().map_err(com_err)?;
        let wic = Wic::new().map_err(com_err)?;
        let (bitmap, target, res) = Self::surface(&factory, &wic, &rt, scale).map_err(com_err)?;
        let h = Headless { rt, text, factory, wic, bitmap, target, res, scale, last_damage: Damage::Full };
        let full = h.rt.frame.rect();
        h.paint(&[full]);
        Ok(h)
    }

    fn surface(factory: &Factory, wic: &Wic, rt: &Runtime, scale: f32) -> Result<(Com, RenderTarget, Resources), HrError> {
        let s = rt.frame.size;
        let bitmap = wic.bitmap((s.w * scale).ceil() as u32, (s.h * scale).ceil() as u32)?;
        let target = factory.wic_target(&bitmap, 96.0 * scale)?;
        let res = Resources::new(factory, &target)?;
        Ok((bitmap, target, res))
    }

    fn paint(&self, rects: &[Rect]) {
        self.target.begin_draw();
        {
            let mut c = D2dCanvas::new(&self.target, &self.factory, &self.res, &self.text, self.scale);
            for &r in rects {
                uison_frame::paint(&self.rt.frame, &mut c, r);
            }
        }
        self.target.end_draw();
    }

    /// Handle an event and redraw the changed regions.
    /// Returns pipeline stats and drawing time.
    pub fn event(&mut self, ev: Event) -> Result<Option<(Update, Duration)>, Error> {
        let old = self.rt.frame.size;
        let up = self.rt.dispatch(&ev, &self.text)?;
        let t0 = Instant::now();
        self.last_damage = up.as_ref().map_or(Damage::None, |u| u.damage.clone());
        if self.rt.frame.size != old {
            let (b, t, r) = Self::surface(&self.factory, &self.wic, &self.rt, self.scale).map_err(com_err)?;
            (self.bitmap, self.target, self.res) = (b, t, r);
            self.last_damage = Damage::Full;
        }
        let rects: Vec<Rect> = self.last_damage.rects(self.rt.frame.rect()).collect();
        self.paint(&rects);
        Ok(up.map(|u| (u, t0.elapsed())))
    }

    /// Save the current state; with `flash`, highlight the last redraws.
    pub fn save_png(&self, path: &str, flash: bool) -> Result<(), String> {
        let rects: Vec<Rect> = if flash { self.last_damage.rects(self.rt.frame.rect()).collect() } else { vec![] };
        if !rects.is_empty() {
            self.target.begin_draw();
            let mut c = D2dCanvas::new(&self.target, &self.factory, &self.res, &self.text, self.scale);
            let mark = Paint { fill: Some(Color::rgba(255, 40, 40, 70)), stroke: Some(Color::rgb(255, 0, 0)), width: 1.0 };
            for r in &rects {
                c.rect(*r, 0.0, mark);
            }
            self.target.end_draw();
        }
        let res = self.wic.save_png(path, Pixels::Bitmap(&self.bitmap)).map_err(|e| e.to_string());
        // Restore the clean image.
        let clean: Vec<Rect> = rects.iter().map(|r| r.inflate(1.0).round_out()).collect();
        if !clean.is_empty() {
            self.paint(&clean);
        }
        res
    }
}
