use std::{
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};

// Generic options let CI exercise hung-work cancellation/deadlines without loading a native library.
pub(super) fn run<T, O: Sync>(
    stop: &AtomicBool,
    deadline: Instant,
    options: impl FnOnce() -> Result<O, String>,
    terminate: impl Fn(&O) -> Result<(), String> + Sync,
    work: impl FnOnce(&O) -> Result<T, String>,
) -> Result<Option<T>, String> {
    if stop.load(Ordering::SeqCst) {
        return Ok(None);
    }
    if Instant::now() >= deadline {
        return Err("semantic turn inference budget expired".into());
    }
    let options = options()?;
    if stop.load(Ordering::SeqCst) {
        return Ok(None);
    }
    if Instant::now() >= deadline {
        return Err("semantic turn inference budget expired".into());
    }
    let done = AtomicBool::new(false);
    let (result, stopped) = std::thread::scope(|scope| {
        let watcher = scope.spawn(|| {
            while !done.load(Ordering::SeqCst) {
                if stop.load(Ordering::SeqCst) || Instant::now() >= deadline {
                    terminate(&options)?;
                    return Ok::<_, String>(true);
                }
                std::thread::park_timeout(Duration::from_millis(5));
            }
            Ok(false)
        });
        let finish = Finish {
            done: &done,
            watcher: watcher.thread().clone(),
        };
        let result = work(&options);
        drop(finish);
        let stopped = watcher
            .join()
            .map_err(|_| "semantic turn cancellation worker failed".to_string())??;
        Ok::<_, String>((result, stopped))
    })?;
    if stop.load(Ordering::SeqCst) {
        return Ok(None);
    }
    if stopped || Instant::now() >= deadline {
        return Err("semantic turn inference budget expired".into());
    }
    result.map(Some)
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
