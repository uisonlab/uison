//! Touchpad scroll inertia. On Wayland the app does it: the compositor sends
//! finger offsets (`axis_source = finger`) and `axis_stop` when the fingers lift.
//! Decay matches touch in `uison-ui` (UIScrollView): velocity x 0.998 per millisecond.

use std::time::{Duration, Instant};

const DECAY: f32 = 0.998;
/// Velocity (pt/s) below which inertia stops or does not start.
const MIN_SPEED: f32 = 60.0;
/// Window for estimating finger velocity.
const WINDOW: Duration = Duration::from_millis(80);
/// If the fingers rested longer than this before lifting, no inertia.
const STILL: Duration = Duration::from_millis(60);

#[derive(Default)]
pub struct Kinetic {
    samples: Vec<(Instant, f32, f32)>,
    /// Velocity (pt/s) and time of the previous step.
    fling: Option<(f32, f32, Instant)>,
}

impl Kinetic {
    /// Finger scroll (points, sign as in `Event::Scroll`). Stops inertia.
    pub fn sample(&mut self, dx: f32, dy: f32) {
        self.fling = None;
        let now = Instant::now();
        self.samples.retain(|(t, ..)| now - *t <= WINDOW);
        self.samples.push((now, dx, dy));
    }

    /// Fingers lifted: start inertia if they were moving. `true` if started.
    pub fn release(&mut self) -> bool {
        let now = Instant::now();
        let samples = std::mem::take(&mut self.samples);
        let Some(&(last, ..)) = samples.last() else { return false };
        if now - last > STILL || samples.len() < 2 {
            return false;
        }
        let first = samples[0].0;
        // Velocity is the sum of offsets over the window (the first offset accrued before `first`).
        let span = (last - first).as_secs_f32().max(0.008) + 0.008;
        let (sx, sy) = samples.iter().fold((0.0, 0.0), |(x, y), s| (x + s.1, y + s.2));
        let (vx, vy) = (sx / span, sy / span);
        if vx.hypot(vy) < MIN_SPEED {
            return false;
        }
        self.fling = Some((vx, vy, now));
        true
    }

    pub fn stop(&mut self) {
        self.fling = None;
        self.samples.clear();
    }

    pub fn active(&self) -> bool {
        self.fling.is_some()
    }

    /// Offset up to now (exact integral: the path does not depend on the frame rate).
    pub fn step(&mut self) -> Option<(f32, f32)> {
        let (vx, vy, t) = self.fling?;
        let now = Instant::now();
        let ms = (now - t).as_secs_f32() * 1000.0;
        let k = DECAY.powf(ms);
        let f = (k - 1.0) / DECAY.ln() / 1000.0;
        let (nx, ny) = (vx * k, vy * k);
        self.fling = (nx.hypot(ny) >= MIN_SPEED).then_some((nx, ny, now));
        Some((vx * f, vy * f))
    }
}
