use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use ort::session::RunOptions;

use crate::scheduler::Scheduler;

pub(super) fn native<T>(
    stop: &AtomicBool,
    work: impl FnMut(&RunOptions) -> Result<T, String>,
) -> Result<Option<T>, String> {
    run(
        &crate::WHISPER,
        stop,
        || RunOptions::new().map_err(|e| e.to_string()),
        |options| options.terminate().map_err(|e| e.to_string()),
        work,
    )
}

// Separate control from native options so CI can exercise races without loading a runtime.
pub(super) fn run<T, O: Sync>(
    scheduler: &Scheduler,
    stop: &AtomicBool,
    mut options: impl FnMut() -> Result<O, String>,
    terminate: impl Fn(&O) -> Result<(), String> + Sync,
    mut work: impl FnMut(&O) -> Result<T, String>,
) -> Result<Option<T>, String> {
    loop {
        let Some(busy) = scheduler.background(stop) else {
            return Ok(None);
        };
        let options = options()?;
        // Preparation holds the slot; requests can arrive before the watcher exists.
        if stop.load(Ordering::SeqCst) {
            return Ok(None);
        }
        if scheduler.foreground_waiting() {
            continue;
        }
        let done = AtomicBool::new(false);
        let (result, interrupted) = std::thread::scope(|scope| {
            let watcher = scope.spawn(|| {
                while !done.load(Ordering::SeqCst) {
                    if stop.load(Ordering::SeqCst) || scheduler.foreground_waiting() {
                        terminate(&options)?;
                        return Ok::<_, String>(true);
                    }
                    std::thread::park_timeout(Duration::from_millis(10));
                }
                Ok(false)
            });
            // A model panic must stop the watcher before the scope waits for its thread.
            let finish = Finish {
                done: &done,
                watcher: watcher.thread().clone(),
            };
            let result = work(&options);
            drop(finish);
            let interrupted = watcher
                .join()
                .map_err(|_| "speaker interruption worker failed".to_string())??;
            Ok::<_, String>((result, interrupted))
        })?;
        drop(busy);
        if stop.load(Ordering::SeqCst) {
            return Ok(None);
        }
        if interrupted {
            // A new options object clears termination, and this exact operation is retried.
            continue;
        }
        return result.map(Some);
    }
}

struct Finish<'a> {
    done: &'a AtomicBool,
    watcher: std::thread::Thread,
}
impl Drop for Finish<'_> {
    fn drop(&mut self) {
        self.done.store(true, Ordering::SeqCst);
        self.watcher.unpark();
    }
}

#[cfg(test)]
mod tests;
