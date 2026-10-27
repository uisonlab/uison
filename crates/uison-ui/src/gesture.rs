//! Touch: a press or a finger scroll, inertia after release.
//!
//! A finger over a scroll view first presses the element under it (the reaction is visible),
//! and moving further than [`SLOP`] turns the touch into a scroll: the press is cancelled,
//! no click happens. Releasing with velocity starts inertia with decay, like UIScrollView.
//! Past the edge the content stretches with resistance and springs back (bounce).

use std::time::Duration;

use uison_core::Point;

/// Finger movement (pt) after which a touch is a scroll, not a press.
pub const SLOP: f32 = 8.0;
/// A finger held longer is a long press (select a word, context menu).
pub const HOLD: Duration = Duration::from_millis(500);
/// Inertia velocity decay per millisecond (UIScrollView.DecelerationRate.normal).
const DECAY: f32 = 0.998;
/// Velocity (pt/s) below which inertia stops.
const MIN_SPEED: f32 = 20.0;
/// Window for estimating finger velocity.
const WINDOW: Duration = Duration::from_millis(80);
/// If the finger rested longer before release, there is no inertia.
const STILL: Duration = Duration::from_millis(60);
/// Inertia timer step (~60 frames/s).
pub const FLING_TICK: Duration = Duration::from_millis(16);

pub struct Touch {
    pub start: Point,
    pub last: Point,
    /// Scroll offset without limits — where the finger "took" it (past the edge — overscroll).
    pub raw: f32,
    /// The scroll under the finger (key), if its content is larger than the area.
    pub scroll: Option<String>,
    pub scrolling: bool,
    /// The touch stopped inertia or closed a popup — it isn't a press.
    pub swallowed: bool,
    /// The node that has a long press (key), and whether it fired.
    pub hold: Option<String>,
    pub held: bool,
    t0: Duration,
    /// Finger positions over time (for velocity).
    samples: Vec<(Duration, f32)>,
}

impl Touch {
    pub fn new(p: Point, t: Duration, scroll: Option<String>, swallowed: bool, hold: Option<String>) -> Touch {
        Touch { start: p, last: p, raw: 0.0, scroll, scrolling: false, swallowed, hold, held: false, t0: t, samples: vec![(t, p.y)] }
    }

    /// When the long press fires; `None` — it won't (nothing to fire, already fired, the finger moved).
    pub fn hold_at(&self) -> Option<Duration> {
        let still = (self.last.x - self.start.x).abs() <= SLOP && (self.last.y - self.start.y).abs() <= SLOP;
        (self.hold.is_some() && !self.held && !self.scrolling && still).then_some(self.t0 + HOLD)
    }

    /// New position; returns the vertical movement since the previous one.
    pub fn moved(&mut self, p: Point, t: Duration) -> f32 {
        let dy = p.y - self.last.y;
        self.last = p;
        self.samples.push((t, p.y));
        // Keep one point older than the window as the anchor for the velocity estimate.
        while self.samples.len() > 2 && t.saturating_sub(self.samples[1].0) > WINDOW {
            self.samples.remove(0);
        }
        dy
    }

    /// Vertical finger velocity (pt/s) at time `t`.
    pub fn velocity(&self, t: Duration) -> f32 {
        let (Some(&(t0, y0)), Some(&(t1, y1))) = (self.samples.first(), self.samples.last()) else { return 0.0 };
        if t.saturating_sub(t1) > STILL || t1 <= t0 {
            return 0.0;
        }
        (y1 - y0) / (t1 - t0).as_secs_f32()
    }
}

/// Resistance past the edge (as in UIScrollView): the finger moves by `x`, the content
/// moves by less, at most the area size `dim`.
pub fn rubber(x: f32, dim: f32) -> f32 {
    const C: f32 = 0.55;
    let dim = dim.max(1.0);
    (1.0 - 1.0 / (x.abs() * C / dim + 1.0)) * dim * x.signum()
}

/// Inverse of [`rubber`]: how far the finger must move for the overscroll to be `r`.
pub fn unrubber(r: f32, dim: f32) -> f32 {
    const C: f32 = 0.55;
    let dim = dim.max(1.0);
    let k = (r.abs() / dim).min(0.99);
    dim / C * (1.0 / (1.0 - k) - 1.0) * r.signum()
}

/// Overscroll returning to the edge: a critically damped spring
/// x(t) = (x0 + (v0 + ω·x0)·t)·e^(−ωt) — no oscillation, ~0.5 s.
pub struct Spring {
    pub key: String,
    x0: f32,
    v0: f32,
    t0: Duration,
}

impl Spring {
    const OMEGA: f32 = 11.0;

    /// `x0` is the current overscroll, `v0` its velocity (pt/s).
    pub fn new(key: String, x0: f32, v0: f32, t0: Duration) -> Spring {
        Spring { key, x0, v0, t0 }
    }

    /// Overscroll at time `now` and whether the motion continues.
    pub fn at(&self, now: Duration) -> (f32, bool) {
        let t = now.saturating_sub(self.t0).as_secs_f32();
        let x = (self.x0 + (self.v0 + Self::OMEGA * self.x0) * t) * (-Self::OMEGA * t).exp();
        if t > 0.05 && x.abs() < 0.3 { (0.0, false) } else { (x, true) }
    }
}

/// Scroll inertia after the finger is released.
pub struct Fling {
    pub key: String,
    /// Velocity (pt/s), sign as for the finger movement.
    pub v: f32,
    pub t: Duration,
}

impl Fling {
    pub fn new(key: String, v: f32, t: Duration) -> Option<Fling> {
        (v.abs() >= MIN_SPEED).then_some(Fling { key, v, t })
    }

    /// Movement at time `now` (like finger movement) and whether inertia continues.
    /// Velocity decays as DECAY^ms; the movement is the exact integral, so
    /// the path doesn't depend on the tick rate.
    pub fn step(&mut self, now: Duration) -> (f32, bool) {
        let ms = now.saturating_sub(self.t).as_secs_f32() * 1000.0;
        self.t = now;
        let k = DECAY.powf(ms);
        let d = self.v / 1000.0 * (k - 1.0) / DECAY.ln();
        self.v *= k;
        (d, self.v.abs() >= MIN_SPEED)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fling_distance_does_not_depend_on_tick_rate() {
        let total = |step_ms: u64| {
            let mut f = Fling::new("s".into(), 2000.0, Duration::ZERO).unwrap();
            let (mut sum, mut t) = (0.0, Duration::ZERO);
            loop {
                t += Duration::from_millis(step_ms);
                let (d, alive) = f.step(t);
                sum += d;
                if !alive {
                    return sum;
                }
            }
        };
        let (a, b) = (total(8), total(33));
        // v / -ln(k) ≈ 2000 pt/s · 0.4995 s ≈ 1000 pt
        assert!((a - 990.0).abs() < 15.0, "{a}");
        assert!((a - b).abs() < 15.0, "{a} vs {b}");
    }

    #[test]
    fn velocity_from_recent_samples() {
        let ms = Duration::from_millis;
        let mut t = Touch::new(Point::new(0.0, 0.0), ms(0), None, false, None);
        for i in 1..=10 {
            t.moved(Point::new(0.0, i as f32 * 10.0), ms(i * 10));
        }
        assert!((t.velocity(ms(100)) - 1000.0).abs() < 1.0);
        // The finger stopped before release — no inertia.
        assert_eq!(t.velocity(ms(200)), 0.0);
    }
}
