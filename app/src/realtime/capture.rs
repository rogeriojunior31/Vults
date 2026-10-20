//! Explicit manual-turn capture. The Nest adapter owns its native controls; no shortcut starts it.
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

mod automatic;

type Active = Arc<dyn Fn() -> bool + Send + Sync>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum End {
    Recording,
    Finished,
    Cancelled,
}

#[derive(Debug)]
pub struct CaptureControl(watch::Sender<End>);
#[derive(Debug, Clone)]
pub struct CaptureRequest(watch::Receiver<End>, pub(super) bool);
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapturePhase {
    Starting,
    Recording,
    Transcribing,
    Manual,
}

/// Creating controls opens nothing. Dropping the only control cancels its one request.
pub fn capture_control() -> (CaptureControl, CaptureRequest) {
    let (send, receive) = watch::channel(End::Recording);
    (CaptureControl(send), CaptureRequest(receive, false))
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
    // Only a completed Listen generation may admit another explicit user turn.
    pub(super) fn restart(&self) -> bool {
        self.0.send_if_modified(|end| {
            if *end != End::Finished {
                return false;
            }
            *end = End::Recording;
            true
        })
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
    #[cfg(test)]
    fn record<C>(
        &self,
        start: impl FnOnce(Active, Active) -> Result<C, Failure>,
        finish: impl FnOnce(C) -> Result<Vec<f32>, Failure>,
        decode: impl FnOnce(&[f32], &AtomicBool) -> Result<String, Failure>,
    ) -> Result<Turn, Failure> {
        self.record_watched(start, finish, decode, |_, _| false)
    }
    fn record_watched<C>(
        &self,
        start: impl FnOnce(Active, Active) -> Result<C, Failure>,
        finish: impl FnOnce(C) -> Result<Vec<f32>, Failure>,
        decode: impl FnOnce(&[f32], &AtomicBool) -> Result<String, Failure>,
        mut ended: impl FnMut(&mut C, Duration) -> bool,
    ) -> Result<Turn, Failure> {
        self.live()?;
        if *self.end.borrow() != End::Recording {
            return Err(Failure::Cancelled);
        }
        self.notify(CapturePhase::Starting)?;
        let (live, admit) = (self.clone(), self.clone());
        let started = Instant::now();
        let until = started + self.record_for;
        let mut microphone = start(
            Arc::new(move || live.live().is_ok()),
            Arc::new(move || {
                admit.live().is_ok() && *admit.end.borrow() == End::Recording && Instant::now() < until
            }),
        )?;
        self.notify(CapturePhase::Recording)?;
        let mut automatic = false;
        while self.live().is_ok() && *self.end.borrow() == End::Recording && Instant::now() < until {
            if ended(&mut microphone, started.elapsed()) {
                automatic = true;
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        let finished = (automatic || *self.end.borrow() == End::Finished) && Instant::now() < until;
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
    generation: Option<vults_voice::turn::Generation>,
) -> Result<Turn, Failure> {
    if !vults_voice::installed(&crate::voice::models_dir()).contains(&model.as_str()) {
        return Err(Failure::Input);
    }
    let grant = crate::questions::admit_ordinary(&app, |stop| stop).ok_or(Failure::Cancelled)?;
    let active = owner.active.clone();
    owner.active = Arc::new(move || active() && !grant.load(Ordering::Acquire));
    options.max_seconds = 30;
    let automatic = request.1.then_some(generation).flatten();
    let worker = Worker {
        end: request.0,
        owner,
        stop: Arc::default(),
        record_for: Duration::from_secs(30),
    };
    worker
        .execute(move |worker| {
            let (loader, selected, spoken) = (app.clone(), model.clone(), language.clone());
            worker.record_watched(
                move |live, admit| {
                    let microphone =
                        vults_audio::Capture::start_guarded(options, |_| {}, move || live(), move || admit())
                            .map_err(|_| Failure::Input)?;
                    Ok(automatic::Recording::new(
                        microphone,
                        automatic,
                        spoken,
                        worker.clone(),
                        move || crate::voice::transcriber(&loader, &selected),
                    ))
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
                |recording, now| recording.ended(now),
            )
        })
        .await
}

#[cfg(test)]
mod tests;

/// Keep the retained provider driver polled while a following explicitly requested turn records.
pub(super) async fn continue_turns<F, Q>(
    driver: F,
    mut next: mpsc::Receiver<vults_voice::turn::Generation>,
    send: mpsc::Sender<super::Command>,
    mut record: impl FnMut(vults_voice::turn::Generation) -> Q,
) -> F::Output
where
    F: std::future::Future<Output = Result<Option<(Turn, Failure)>, Failure>>,
    Q: std::future::Future<Output = Result<Turn, Failure>>,
{
    tokio::pin!(driver);
    let mut recording = None;
    let mut generation = None;
    loop {
        tokio::select! { biased;
            result = &mut driver => return result,
            turn = async { recording.as_mut().expect("An admitted capture").await }, if recording.is_some() => {
                let turn = turn?;
                send.try_send(super::Command::Turn { generation: generation.take().expect("Capture generation"), turn }).map_err(|_| Failure::Output)?;
                recording = None;
            },
            request = next.recv(), if recording.is_none() => match request {
                Some(value) => { generation = Some(value); recording = Some(Box::pin(record(value))); },
                None => return Err(Failure::Cancelled),
            },
        }
    }
}
