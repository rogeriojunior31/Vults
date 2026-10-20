//! Explicit first-turn capture. The Nest adapter owns its native controls; no shortcut starts it.
use super::{Event, Lease, Turn, local};
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
use tauri::AppHandle;
use tokio::sync::{mpsc, watch};
use vults_chat::realtime::transport::Failure;

type Active = Arc<dyn Fn() -> bool + Send + Sync>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum End {
    Recording,
    Finished,
    Cancelled,
}

#[derive(Debug)]
pub struct CaptureControl(watch::Sender<End>);
#[derive(Debug)]
pub struct CaptureRequest(watch::Receiver<End>);
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapturePhase {
    Starting,
    Recording,
    Transcribing,
}

/// Creating controls opens nothing. Dropping the only control cancels its one request.
pub fn capture_control() -> (CaptureControl, CaptureRequest) {
    let (send, receive) = watch::channel(End::Recording);
    (CaptureControl(send), CaptureRequest(receive))
}
impl CaptureControl {
    pub fn finish(&self) {
        self.0.send_if_modified(|end| {
            if *end != End::Recording {
                return false;
            }
            *end = End::Finished;
            true
        });
    }
    pub fn cancel(&self) {
        self.0.send_replace(End::Cancelled);
    }
}
impl CaptureRequest {
    pub(super) fn active(&self) -> Active {
        let end = self.0.clone();
        Arc::new(move || end.has_changed().is_ok() && *end.borrow() != End::Cancelled)
    }
}

#[derive(Clone)]
pub(super) struct Owner {
    pub lease: Arc<Lease>,
    pub active: Active,
    pub cancel: watch::Receiver<bool>,
    pub output: mpsc::Sender<Event>,
}
#[derive(Clone)]
struct Worker {
    end: watch::Receiver<End>,
    owner: Owner,
    stop: Arc<AtomicBool>,
    record_for: Duration,
}
struct Stop(Arc<AtomicBool>);
impl Drop for Stop {
    fn drop(&mut self) {
        self.0.store(true, Ordering::SeqCst);
    }
}
impl Worker {
    fn live(&self) -> Result<(), Failure> {
        if self.stop.load(Ordering::SeqCst)
            || self.end.has_changed().is_err()
            || *self.end.borrow() == End::Cancelled
        {
            return Err(Failure::Cancelled);
        }
        local::check(&*self.owner.active, &self.owner.cancel)?;
        if self.owner.output.is_closed() {
            return Err(Failure::Output);
        }
        Ok(())
    }
    fn notify(&self, phase: CapturePhase) -> Result<(), Failure> {
        self.live()?;
        self.owner
            .output
            .try_send(Event::Capture(phase))
            .map_err(|_| Failure::Output)
    }
    fn record<C>(
        &self,
        start: impl FnOnce(Active, Active) -> Result<C, Failure>,
        finish: impl FnOnce(C) -> Result<Vec<f32>, Failure>,
        decode: impl FnOnce(&[f32], &AtomicBool) -> Result<String, Failure>,
    ) -> Result<Turn, Failure> {
        self.live()?;
        if *self.end.borrow() != End::Recording {
            return Err(Failure::Cancelled);
        }
        self.notify(CapturePhase::Starting)?;
        let (live, admit) = (self.clone(), self.clone());
        let until = Instant::now() + self.record_for;
        let microphone = start(
            Arc::new(move || live.live().is_ok()),
            Arc::new(move || {
                admit.live().is_ok() && *admit.end.borrow() == End::Recording && Instant::now() < until
            }),
        )?;
        self.notify(CapturePhase::Recording)?;
        while self.live().is_ok() && *self.end.borrow() == End::Recording && Instant::now() < until {
            std::thread::sleep(Duration::from_millis(10));
        }
        let finished = *self.end.borrow() == End::Finished && Instant::now() < until;
        let pcm = finish(microphone)?;
        self.live()?;
        if !finished {
            return Err(Failure::Timeout);
        }
        self.notify(CapturePhase::Transcribing)?;
        let text = decode(&pcm, &self.stop)?;
        self.live()?;
        Turn::from_capture(&pcm, text)
    }
    async fn execute(
        self,
        work: impl FnOnce(&Self) -> Result<Turn, Failure> + Send + 'static,
    ) -> Result<Turn, Failure> {
        self.live()?;
        let _stop = Stop(self.stop.clone());
        let worker = self.clone();
        let mut task = tokio::task::spawn_blocking(move || {
            // The actual device/model worker retains the desktop lease even after a timeout.
            let _lease = worker.owner.lease.clone();
            work(&worker)
        });
        let deadline = tokio::time::sleep(Duration::from_secs(60));
        tokio::pin!(deadline);
        let mut poll = tokio::time::interval(Duration::from_millis(10));
        loop {
            self.live()?;
            tokio::select! { biased;
                _ = &mut deadline => return Err(Failure::Timeout),
                _ = poll.tick() => {},
                result = &mut task => {
                    self.live()?;
                    return result.map_err(|_| Failure::Input)?;
                },
            }
        }
    }
}

pub(super) async fn record(
    app: AppHandle,
    request: CaptureRequest,
    mut owner: Owner,
    mut options: vults_audio::CaptureOptions,
    model: String,
    language: Option<String>,
) -> Result<Turn, Failure> {
    if !vults_voice::installed(&crate::voice::models_dir()).contains(&model.as_str()) {
        return Err(Failure::Input);
    }
    let grant = crate::questions::admit_ordinary(&app, |stop| stop).ok_or(Failure::Cancelled)?;
    let active = owner.active.clone();
    owner.active = Arc::new(move || active() && !grant.load(Ordering::Acquire));
    options.max_seconds = 30;
    let worker = Worker {
        end: request.0,
        owner,
        stop: Arc::default(),
        record_for: Duration::from_secs(30),
    };
    worker
        .execute(move |worker| {
            worker.record(
                move |live, admit| {
                    vults_audio::Capture::start_guarded(options, |_| {}, move || live(), move || admit())
                        .map_err(|_| Failure::Input)
                },
                |microphone| microphone.finish().map_err(|_| Failure::Input),
                move |pcm, stop| {
                    let native = crate::voice::transcriber(&app, &model).map_err(|_| Failure::Input)?;
                    worker.live()?;
                    native
                        .transcribe_cancellable(
                            pcm,
                            language.as_deref(),
                            vults_voice::vad_path(&crate::voice::models_dir()).as_deref(),
                            stop,
                            true,
                        )
                        .map_err(|_| Failure::Input)?
                        .ok_or(Failure::Input)
                },
            )
        })
        .await
}

#[cfg(test)]
mod tests;
