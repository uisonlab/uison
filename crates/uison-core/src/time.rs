//! Platform time. `wasm32-unknown-unknown` has no `std::time::Instant`
//! (it panics), so the backend may set the clock (`set_clock`), and shared code
//! uses only `now()`.

use std::sync::OnceLock;
use std::time::Duration;

static CLOCK: OnceLock<fn() -> Duration> = OnceLock::new();

/// Set the platform clock (once, before the first frame). Web backend: `performance.now()`.
pub fn set_clock(f: fn() -> Duration) {
    let _ = CLOCK.set(f);
}

/// Monotonic time since the program started.
pub fn now() -> Duration {
    match CLOCK.get() {
        Some(f) => f(),
        None => default_now(),
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn default_now() -> Duration {
    static START: OnceLock<std::time::Instant> = OnceLock::new();
    START.get_or_init(std::time::Instant::now).elapsed()
}

#[cfg(target_arch = "wasm32")]
fn default_now() -> Duration {
    // The backend set no clock: time stands still (timers fire at once, but we don't crash).
    Duration::ZERO
}
