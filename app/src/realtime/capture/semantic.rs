//! One bounded semantic worker per explicit conversation; native teardown retains the desktop lease.
use super::*;
use std::sync::{Mutex, mpsc as sync};
use vults_voice::turn::end::Ticket;

type Score = dyn FnMut(&[f32], &AtomicBool, Instant) -> Result<f32, ()> + Send;
struct Request {
    pcm: Vec<f32>,
    until: Instant,
    stop: Arc<AtomicBool>,
    result: sync::SyncSender<Result<f32, ()>>,
}
pub(super) struct Job {
    pub generation: vults_voice::turn::Generation,
    pub ticket: Ticket,
    pub until: Instant,
    stop: Arc<AtomicBool>,
    result: sync::Receiver<Result<f32, ()>>,
}
impl Job {
    pub(super) fn poll(&self) -> Option<Result<f32, ()>> {
        match self.result.try_recv() {
            Ok(value) => Some(value),
            Err(sync::TryRecvError::Empty) => None,
            Err(sync::TryRecvError::Disconnected) => Some(Err(())),
        }
    }
}
impl Drop for Job {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
    }
}
pub(in crate::realtime) struct Service {
    send: Mutex<Option<sync::SyncSender<Request>>>,
    busy: Arc<AtomicBool>,
    stop: Arc<AtomicBool>,
    failed: Arc<AtomicBool>,
    ready: Arc<AtomicBool>,
    pub(super) policy: Arc<Mutex<vults_voice::turn::end::EndOfTurn>>,
    pub(super) clock: Instant,
}
impl Service {
    fn start(
        lease: Arc<Lease>,
        active: Active,
        initialize: impl FnOnce(&AtomicBool) -> Result<Box<Score>, ()> + Send + 'static,
    ) -> Result<Arc<Self>, ()> {
        let (send, receive) = sync::sync_channel::<Request>(1);
        let service = Arc::new(Self {
            send: Mutex::new(Some(send)),
            busy: Arc::default(),
            stop: Arc::default(),
            failed: Arc::default(),
            ready: Arc::default(),
            policy: Arc::new(Mutex::new(vults_voice::turn::end::EndOfTurn::new(true))),
            clock: Instant::now(),
        });
        let (busy, stop) = (service.busy.clone(), service.stop.clone());
        let (failed, ready, policy) = (
            service.failed.clone(),
            service.ready.clone(),
            service.policy.clone(),
        );
        std::thread::Builder::new()
            .name("voice-semantic-turn".into())
            .spawn(move || {
                let retained = lease;
                if stop.load(Ordering::SeqCst) || !active() {
                    return;
                }
                let mut score =
                    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| initialize(&stop))) {
                        Ok(Ok(score)) => score,
                        _ => {
                            failed.store(true, Ordering::SeqCst);
                            *policy.lock().unwrap_or_else(|e| e.into_inner()) =
                                vults_voice::turn::end::EndOfTurn::new(false);
                            Box::new(|_: &[f32], _: &AtomicBool, _: Instant| Err(())) as Box<Score>
                        }
                    };
                ready.store(true, Ordering::SeqCst);
                while let Ok(request) = receive.recv() {
                    if stop.load(Ordering::SeqCst) || !active() {
                        break;
                    }
                    let result = if request.stop.load(Ordering::SeqCst) || Instant::now() >= request.until {
                        Err(())
                    } else {
                        score(&request.pcm, &request.stop, request.until)
                    };
                    if !stop.load(Ordering::SeqCst) && !request.stop.load(Ordering::SeqCst) && active() {
                        let _ = request.result.try_send(result);
                    }
                    busy.store(false, Ordering::SeqCst);
                }
                // Session/model destruction can wait on the shared native-work slot.
                drop(score);
                drop(retained);
            })
            .map_err(|_| ())?;
        Ok(service)
    }
    #[cfg(test)]
    pub(super) fn scoring(lease: Arc<Lease>, active: Active, score: Box<Score>) -> Result<Arc<Self>, ()> {
        Self::start(lease, active, move |_| Ok(score))
    }
    async fn prepare(&self, active: &(dyn Fn() -> bool + Sync), limit: Duration) -> Result<(), Failure> {
        let until = self.clock + limit;
        loop {
            if !active() {
                return Err(Failure::Cancelled);
            }
            if Instant::now() >= until {
                return Err(Failure::Timeout);
            }
            if self.ready.load(Ordering::SeqCst) {
                return Ok(());
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    }
    pub(in crate::realtime) fn failed(&self) -> bool {
        self.failed.load(Ordering::SeqCst)
    }
    pub(super) fn fail(&self) {
        self.failed.store(true, Ordering::SeqCst);
    }
    pub(super) fn request(
        &self,
        generation: vults_voice::turn::Generation,
        ticket: Ticket,
        until: Instant,
        pcm: impl FnOnce() -> Vec<f32>,
    ) -> Option<Job> {
        if self.failed() || self.stop.load(Ordering::SeqCst) || self.busy.swap(true, Ordering::SeqCst) {
            return None;
        }
        let (result, receive) = sync::sync_channel(1);
        let stop = Arc::new(AtomicBool::new(false));
        let mut samples = pcm();
        if samples.len() > vults_voice::SAMPLE_RATE as usize * 8 {
            samples.drain(..samples.len() - vults_voice::SAMPLE_RATE as usize * 8);
        }
        let request = Request {
            pcm: samples,
            until,
            stop: stop.clone(),
            result,
        };
        let sent = self
            .send
            .lock()
            .ok()
            .and_then(|send| send.as_ref().map(|send| send.try_send(request).is_ok()))
            .unwrap_or(false);
        if !sent {
            self.busy.store(false, Ordering::SeqCst);
            self.fail();
            return None;
        }
        Some(Job {
            generation,
            ticket,
            until,
            stop,
            result: receive,
        })
    }
}
impl Drop for Service {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        self.send.get_mut().unwrap_or_else(|e| e.into_inner()).take();
    }
}

pub(in crate::realtime) fn supported() -> bool {
    cfg!(all(
        feature = "voice-semantic",
        target_os = "linux",
        target_arch = "x86_64",
        target_env = "gnu"
    ))
}
#[cfg(feature = "voice-semantic")]
pub(in crate::realtime) async fn open(
    lease: Arc<Lease>,
    active: Active,
    root: std::path::PathBuf,
) -> Result<Arc<Service>, Failure> {
    let service = Service::start(lease, active.clone(), move |stop| {
        let runtime = vults_voice::onnx::Runtime::load(
            &root.join("libonnxruntime.so.1.31.0"),
            vults_voice::turn::semantic::RUNTIME_SHA256,
            stop,
        )
        .map_err(|_| ())?
        .ok_or(())?;
        let mut detector =
            vults_voice::turn::semantic::Detector::load(&runtime, &root.join("smart-turn-v3.0.onnx"), stop)
                .map_err(|_| ())?
                .ok_or(())?;
        Ok(Box::new(move |pcm, stop, until| {
            detector
                .score(pcm, stop, until.saturating_duration_since(Instant::now()))
                .map_err(|_| ())?
                .ok_or(())
        }))
    })
    .map_err(|_| Failure::Input)?;
    service.prepare(&*active, Duration::from_secs(60)).await?;
    Ok(service)
}
#[cfg(not(feature = "voice-semantic"))]
pub(in crate::realtime) async fn open(
    lease: Arc<Lease>,
    active: Active,
    _root: std::path::PathBuf,
) -> Result<Arc<Service>, Failure> {
    let service = Service::start(lease, active.clone(), |_| Err(())).map_err(|_| Failure::Input)?;
    service.prepare(&*active, Duration::from_secs(60)).await?;
    Ok(service)
}

#[cfg(test)]
mod tests;
