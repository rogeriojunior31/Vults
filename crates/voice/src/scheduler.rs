//! Serialize native speech work, letting interactive speech preempt meeting analysis.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Condvar, Mutex};
use std::time::{Duration, Instant};

#[derive(Debug, Default)]
struct State {
    running: bool,
    waiting: usize,
}

#[derive(Debug)]
pub(crate) struct Scheduler {
    state: Mutex<State>,
    wake: Condvar,
    foreground: AtomicBool,
}

impl Scheduler {
    pub const fn new() -> Self {
        Self {
            state: Mutex::new(State {
                running: false,
                waiting: 0,
            }),
            wake: Condvar::new(),
            foreground: AtomicBool::new(false),
        }
    }

    pub fn lock(&self) -> Guard<'_> {
        self.foreground(None, None)
            .expect("unbounded foreground admission")
    }
    #[cfg(feature = "semantic-turn")]
    pub fn interactive(&self, stop: &AtomicBool, deadline: Option<Instant>) -> Option<Guard<'_>> {
        self.foreground(Some(stop), deadline)
    }
    fn foreground(&self, stop: Option<&AtomicBool>, deadline: Option<Instant>) -> Option<Guard<'_>> {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        state.waiting += 1;
        self.foreground.store(true, Ordering::SeqCst);
        loop {
            if stop.is_some_and(|s| s.load(Ordering::SeqCst))
                || deadline.is_some_and(|at| Instant::now() >= at)
            {
                state.waiting -= 1;
                self.foreground.store(state.waiting > 0, Ordering::SeqCst);
                self.wake.notify_all();
                return None;
            }
            if !state.running {
                state.waiting -= 1;
                self.foreground.store(state.waiting > 0, Ordering::SeqCst);
                state.running = true;
                return Some(Guard(self));
            }
            state = if stop.is_some() || deadline.is_some() {
                self.wake
                    .wait_timeout(state, Duration::from_millis(10))
                    .unwrap_or_else(|e| e.into_inner())
                    .0
            } else {
                self.wake.wait(state).unwrap_or_else(|e| e.into_inner())
            };
        }
    }

    pub fn background(&self, stop: &AtomicBool) -> Option<Guard<'_>> {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        loop {
            if stop.load(Ordering::SeqCst) {
                return None;
            }
            if !state.running && state.waiting == 0 {
                state.running = true;
                return Some(Guard(self));
            }
            // Cancellation is caller-owned, so it cannot notify this condition variable.
            state = self
                .wake
                .wait_timeout(state, Duration::from_millis(10))
                .unwrap_or_else(|e| e.into_inner())
                .0;
        }
    }

    pub fn foreground_waiting(&self) -> bool {
        self.foreground.load(Ordering::SeqCst)
    }
}

#[derive(Debug)]
pub(crate) struct Guard<'a>(&'a Scheduler);

impl Drop for Guard<'_> {
    fn drop(&mut self) {
        self.0.state.lock().unwrap_or_else(|e| e.into_inner()).running = false;
        self.0.wake.notify_all();
    }
}

#[cfg(test)]
mod tests;
