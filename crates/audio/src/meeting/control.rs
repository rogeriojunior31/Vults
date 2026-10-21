use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use super::{Report, TemporaryAudio};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Starting,
    Recording,
    Stopping,
    Processing,
    Cancelling,
    Finished,
    Cancelled,
    Failed,
}

#[derive(Debug, Clone, Copy)]
pub struct Status {
    pub phase: Phase,
    pub elapsed: Duration,
}

#[derive(Debug)]
pub enum Outcome {
    Finished(Report),
    Cancelled,
}

#[derive(Debug)]
struct State {
    phase: Phase,
    started: Option<Instant>,
    elapsed: Duration,
}

#[derive(Debug, Clone)]
pub(super) struct Control {
    state: Arc<Mutex<State>>,
    stop: Arc<AtomicBool>,
}

impl Control {
    pub(super) fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(State {
                phase: Phase::Starting,
                started: None,
                elapsed: Duration::ZERO,
            })),
            stop: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn phase(&self) -> Phase {
        self.status().phase
    }

    pub fn status(&self) -> Status {
        let state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        Status {
            phase: state.phase,
            elapsed: state.started.map_or(state.elapsed, |start| start.elapsed()),
        }
    }

    pub fn continuing(&self) -> bool {
        matches!(self.phase(), Phase::Starting | Phase::Recording)
    }

    pub fn started(&self) -> bool {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if state.phase != Phase::Starting {
            return false;
        }
        state.phase = Phase::Recording;
        state.started = Some(Instant::now());
        true
    }

    pub(super) fn request(&self, cancel: bool) {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        state.phase = match (state.phase, cancel) {
            (Phase::Starting, _) | (Phase::Recording | Phase::Stopping | Phase::Processing, true) => {
                Phase::Cancelling
            }
            (Phase::Recording, false) => Phase::Stopping,
            (phase, _) => phase,
        };
        if state.phase == Phase::Cancelling {
            self.stop.store(true, Ordering::SeqCst);
        }
    }

    fn processing(&self) -> bool {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if !matches!(state.phase, Phase::Recording | Phase::Stopping) {
            return false;
        }
        state.elapsed = state
            .started
            .take()
            .map_or(state.elapsed, |start| start.elapsed());
        state.phase = Phase::Processing;
        true
    }

    fn complete(&self, result: Result<Outcome, String>) -> Result<Outcome, String> {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        // A cancellation that wins this lock cannot publish a completed recording afterward.
        let result = if state.phase == Phase::Cancelling && result.is_ok() {
            Ok(Outcome::Cancelled)
        } else {
            result
        };
        state.phase = match &result {
            Ok(Outcome::Finished(_)) => Phase::Finished,
            Ok(Outcome::Cancelled) => Phase::Cancelled,
            Err(_) => Phase::Failed,
        };
        state.elapsed = state
            .started
            .take()
            .map_or(state.elapsed, |start| start.elapsed());
        result
    }
}

/// Closed capture streams, readable only while their worker retains the cleanup lock.
#[derive(Debug)]
pub struct Tracks {
    pub microphone: BufReader<File>,
    pub system: BufReader<File>,
}

/// One explicit capture. Stop/cancel and status never wait for the audio backend.
/// A pending backend call stays in Starting/Stopping/Cancelling until the worker releases it.
#[derive(Debug)]
pub struct Recording {
    control: Control,
    worker: Option<JoinHandle<Result<Outcome, String>>>,
}

impl Recording {
    /// Acquires the private file lock before spawning. Device enumeration/play run on the worker.
    pub fn start(root: &Path, seconds: u32) -> Result<Self, String> {
        Self::spawn(root, seconds, super::capture)
    }

    /// Refuse changed default sources before opening either audio stream.
    pub fn start_checked(root: &Path, seconds: u32, expected: super::Sources) -> Result<Self, String> {
        Self::spawn(root, seconds, move |temporary, seconds, control| {
            super::capture_checked(temporary, seconds, control, Some(&expected))
        })
    }

    /// Process both finalized WAVs off-thread before cleanup. Cancellation keeps the lock until
    /// `process` releases its readers; callers must not publish results before `take_result`.
    pub fn start_processed(
        root: &Path,
        seconds: u32,
        expected: super::Sources,
        process: impl FnOnce(Tracks, &AtomicBool) -> Result<(), String> + Send + 'static,
    ) -> Result<Self, String> {
        Self::spawn(root, seconds, move |temporary, seconds, control| {
            let result = super::capture_checked(temporary, seconds, control, Some(&expected))?;
            process_capture(temporary, control, result, process)
        })
    }

    fn spawn(
        root: &Path,
        seconds: u32,
        work: impl FnOnce(&TemporaryAudio, u32, &Control) -> Result<Outcome, String> + Send + 'static,
    ) -> Result<Self, String> {
        if !(1..=super::MAX_SECONDS).contains(&seconds) {
            return Err("meeting capture must be limited to 1-14400 seconds".into());
        }
        let temporary = TemporaryAudio::open(root).map_err(|e| e.to_string())?;
        let control = Control::new();
        let shared = control.clone();
        let worker = std::thread::Builder::new()
            .name("meeting-capture".into())
            .spawn(move || {
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    let result = work(&temporary, seconds, &shared);
                    // Cleanup errors are observable even when capture or cancellation also failed.
                    let cleanup = temporary.finish();
                    match (result, cleanup) {
                        (result, Ok(())) => result,
                        (Ok(_), Err(error)) => Err(format!("meeting audio cleanup failed: {error}")),
                        (Err(error), Err(cleanup)) => {
                            Err(format!("{error}; meeting audio cleanup failed: {cleanup}"))
                        }
                    }
                }))
                .unwrap_or_else(|_| Err("the meeting capture worker panicked".into()));
                shared.complete(result)
            })
            .map_err(|e| e.to_string())?;
        Ok(Self {
            control,
            worker: Some(worker),
        })
    }

    pub fn status(&self) -> Status {
        self.control.status()
    }

    /// During startup, Stop cancels; no empty successful recording is produced.
    pub fn stop(&self) {
        self.control.request(false);
    }

    /// Cancellation overrides a pending Stop and discards its report.
    pub fn cancel(&self) {
        self.control.request(true);
    }

    /// Returns once, after stream teardown, file cleanup and worker exit. Never joins a live worker.
    pub fn take_result(&mut self) -> Option<Result<Outcome, String>> {
        if !self.worker.as_ref()?.is_finished() {
            return None;
        }
        Some(self.worker.take()?.join().unwrap_or_else(|_| {
            self.control
                .complete(Err("the meeting capture worker panicked".into()))
        }))
    }
}

fn process_capture(
    temporary: &TemporaryAudio,
    control: &Control,
    outcome: Outcome,
    process: impl FnOnce(Tracks, &AtomicBool) -> Result<(), String>,
) -> Result<Outcome, String> {
    if !matches!(outcome, Outcome::Finished(_)) || !control.processing() {
        return Ok(Outcome::Cancelled);
    }
    let result = process(
        Tracks {
            microphone: BufReader::new(temporary.reader("mic.wav")?),
            system: BufReader::new(temporary.reader("system.wav")?),
        },
        &control.stop,
    );
    if control.stop.load(Ordering::SeqCst) {
        return Ok(Outcome::Cancelled);
    }
    result?;
    Ok(outcome)
}

impl Drop for Recording {
    fn drop(&mut self) {
        // Keep the worker's lock until teardown, even if its UI owner goes away.
        self.cancel();
    }
}

/// Startup recovery without opening a source or creating a directory on a fresh profile.
/// A busy recorder is left untouched and returns an error for an explicit retry.
pub fn recover(root: &Path) -> Result<(), String> {
    match std::fs::symlink_metadata(root) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e.to_string()),
        Ok(_) => {}
    }
    TemporaryAudio::open(root).map_err(|e| e.to_string())?.finish()
}

#[cfg(test)]
mod tests;
