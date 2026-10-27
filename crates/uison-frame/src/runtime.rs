//! Platform-independent loop: event → hit-test → app → new frame → diff.
//! The backend only delivers events and redraws the returned regions.

use std::time::Duration;

use uison_core::time::now;

use uison_core::{App, Event, Host, MouseButton, Point};

use crate::{Damage, Error, Frame, diff, hit_test};

pub struct Runtime {
    pub app: Box<dyn App>,
    pub frame: Frame,
    /// Where the mouse rests (not a finger): content moving under it changes what it hovers.
    pointer: Option<Point>,
    touching: bool,
}

/// Result of handling an event.
pub struct Update {
    pub damage: Damage,
    pub bytes: usize,
    pub app: Duration,
    pub load: Duration,
    pub diff: Duration,
}

impl Runtime {
    pub fn new(mut app: Box<dyn App>, m: &dyn Host) -> Result<Runtime, Error> {
        let src = app.frame(m);
        let frame = Frame::parse(&src, m)?;
        Ok(Runtime { app, frame, pointer: None, touching: false })
    }

    /// `Ok(None)` — the app didn't change the frame. On a parse error the old frame stays.
    pub fn dispatch(&mut self, ev: &Event, m: &dyn Host) -> Result<Option<Update>, Error> {
        let t0 = now();
        let hit = ev.pos().and_then(|p| hit_test(&self.frame, p));
        match *ev {
            Event::MouseDown(_, MouseButton::Touch) => (self.pointer, self.touching) = (None, true),
            Event::MouseUp(_, MouseButton::Touch) => (self.pointer, self.touching) = (None, false),
            Event::MouseMove(p) | Event::Scroll(p, ..) | Event::MouseDown(p, _) | Event::MouseUp(p, _) if !self.touching => {
                self.pointer = Some(p)
            }
            Event::MouseLeave => self.pointer = None,
            _ => {}
        }
        let Some(mut src) = self.app.on_event(ev, hit, m) else { return Ok(None) };
        let mut t1 = now();
        let mut new = Frame::parse(&src, m)?;
        let mut t2 = now();
        // A scroll, inertia or animation moved another element under the resting mouse — it is hovered now,
        // as if the mouse moved (otherwise the old row stays highlighted until the next move).
        if let Some(p) = self.pointer.filter(|_| !matches!(ev, Event::MouseMove(_))) {
            let under = hit_test(&new, p);
            if under != hit_test(&self.frame, p)
                && let Some(s) = self.app.on_event(&Event::MouseMove(p), under, m)
            {
                let t = now();
                new = Frame::parse(&s, m)?;
                src = s;
                (t1, t2) = (t1 + (t - t2), now());
            }
        }
        let damage = diff(&self.frame, &new);
        let t3 = now();
        self.frame = new;
        Ok(Some(Update { damage, bytes: src.len(), app: t1 - t0, load: t2 - t1, diff: t3 - t2 }))
    }
}
