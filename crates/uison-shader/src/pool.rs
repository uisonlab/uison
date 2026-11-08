//! Worker threads started once and reused: starting threads for every shader item would cost
//! more than painting it (on Windows a frame of four tiles spent more time creating threads
//! than computing pixels).

use std::sync::{Arc, Condvar, Mutex, OnceLock, mpsc};

type Job = Box<dyn FnOnce() + Send>;

struct Pool {
    tx: Mutex<mpsc::Sender<Job>>,
    workers: usize,
}

fn pool() -> &'static Pool {
    static POOL: OnceLock<Pool> = OnceLock::new();
    POOL.get_or_init(|| {
        let cores = std::thread::available_parallelism().map_or(1, |n| n.get());
        let (tx, rx) = mpsc::channel::<Job>();
        let rx = Arc::new(Mutex::new(rx));
        let mut workers = 0;
        for _ in 1..cores {
            let rx = rx.clone();
            let started = std::thread::Builder::new().name("uison-shader".into()).spawn(move || {
                loop {
                    // The lock is held only while taking a job, not while running it.
                    let job = rx.lock().unwrap_or_else(|e| e.into_inner()).recv();
                    match job {
                        Ok(job) => job(),
                        Err(_) => return,
                    }
                }
            });
            workers += started.is_ok() as usize;
        }
        Pool { tx: Mutex::new(tx), workers }
    })
}

/// Threads that can run parts at once: the workers and the caller.
pub fn threads() -> usize {
    pool().workers + 1
}

/// Counts a finished part down, also when the part panics.
struct Done<'a>(&'a (Mutex<usize>, Condvar));

impl Drop for Done<'_> {
    fn drop(&mut self) {
        let mut left = self.0.0.lock().unwrap_or_else(|e| e.into_inner());
        *left -= 1;
        if *left == 0 {
            self.0.1.notify_all();
        }
    }
}

/// Run `f(0)` … `f(parts - 1)` on the workers and this thread; returns when every part is done.
/// `parts` is at most `threads()`.
pub fn run(parts: usize, f: &(dyn Fn(usize) + Sync)) {
    let parts = parts.min(threads());
    if parts <= 1 {
        if parts == 1 {
            f(0);
        }
        return;
    }
    let latch = Arc::new((Mutex::new(parts - 1), Condvar::new()));
    // SAFETY: a job uses `f` only before its `Done` counts down, and this function returns only
    // after the count reaches zero, so `f` outlives every use.
    let f: &'static (dyn Fn(usize) + Sync) = unsafe { std::mem::transmute::<&(dyn Fn(usize) + Sync), &'static (dyn Fn(usize) + Sync)>(f) };
    let tx = pool().tx.lock().unwrap_or_else(|e| e.into_inner()).clone();
    for i in 1..parts {
        let latch = latch.clone();
        let job: Job = Box::new(move || {
            let _done = Done(&latch);
            f(i);
        });
        if let Err(mpsc::SendError(job)) = tx.send(job) {
            job(); // no workers left: run it here
        }
    }
    f(0);
    let (left, done) = &*latch;
    let mut left = left.lock().unwrap_or_else(|e| e.into_inner());
    while *left > 0 {
        left = done.wait(left).unwrap_or_else(|e| e.into_inner());
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn every_part_runs_once() {
        let hits: Vec<AtomicUsize> = (0..super::threads()).map(|_| AtomicUsize::new(0)).collect();
        for _ in 0..100 {
            super::run(hits.len(), &|i| {
                hits[i].fetch_add(1, Ordering::Relaxed);
            });
        }
        assert!(hits.iter().all(|h| h.load(Ordering::Relaxed) == 100));
    }
}
