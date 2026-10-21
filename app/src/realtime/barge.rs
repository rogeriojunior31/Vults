//! Explicit routed capture and local interruption; no device selection or microphone admission.
use super::{Channels, Command, Event, Input, Lease, Signal, Turn, local};
use std::{
    collections::VecDeque,
    future::Future,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc as sync,
    },
    time::{Duration, Instant},
};
use tokio::sync::mpsc;
use vults_chat::realtime::transport::Failure;
use vults_voice::turn::{Action, Effect, Generation, Probe};
const RATE: usize = vults_audio::playback::RATE as usize;
const WINDOW: usize = RATE * 8;
const FRESH: usize = RATE / 5;
const PROBE: Duration = Duration::from_millis(1400);

type Active = Arc<dyn Fn() -> bool + Send + Sync>;
#[derive(Clone, Copy)]
enum Kind {
    Speech,
    Words(Probe),
}
enum Evidence {
    Speech(Option<usize>),
    Words(bool),
}
struct Work {
    generation: Generation,
    kind: Kind,
    samples: Vec<i16>,
    through: u64,
    stop: Arc<AtomicBool>,
}
struct Analysis {
    work: Work,
    evidence: std::result::Result<Evidence, ()>,
}
struct Job {
    stop: Arc<AtomicBool>,
    until: Instant,
}
struct Detector {
    ring: VecDeque<i16>,
    frames: u64,
    observed: u64,
    generation: Option<Generation>,
    onset: Option<u64>,
    retained: Option<u64>,
    pending: Option<(Probe, Instant)>,
    next: Instant,
    job: Option<Job>,
    requests: Option<sync::SyncSender<Work>>,
    results: mpsc::Receiver<Analysis>,
    stop: Arc<AtomicBool>,
    active: Active,
}
impl Detector {
    fn start(
        lease: Arc<Lease>,
        active: Active,
        analyze: impl FnMut(Kind, &[i16], &AtomicBool) -> std::result::Result<Evidence, ()> + Send + 'static,
    ) -> std::result::Result<Self, Failure> {
        if !active() {
            return Err(Failure::Cancelled);
        }
        let (requests, receive) = sync::sync_channel::<Work>(1);
        let (send, results) = mpsc::channel(1);
        let stop = Arc::new(AtomicBool::new(false));
        let (stopped, authority) = (stop.clone(), active.clone());
        std::thread::Builder::new()
            .name("voice-interruption".into())
            .spawn(move || {
                let mut analyze = analyze;
                while let Ok(work) = receive.recv() {
                    if stopped.load(Ordering::SeqCst) || !authority() {
                        break;
                    }
                    let evidence = if work.stop.load(Ordering::SeqCst) {
                        Err(())
                    } else {
                        analyze(work.kind, &work.samples, &work.stop)
                    };
                    if stopped.load(Ordering::SeqCst) || !authority() {
                        break;
                    }
                    if send.try_send(Analysis { work, evidence }).is_err() {
                        break;
                    }
                }
                // Native model destruction can wait on the shared scheduler; retain the real lease.
                drop(analyze);
                drop(lease);
            })
            .map_err(|_| Failure::Input)?;
        Ok(Self {
            ring: VecDeque::with_capacity(WINDOW),
            frames: 0,
            observed: 0,
            generation: None,
            onset: None,
            retained: None,
            pending: None,
            next: Instant::now(),
            job: None,
            requests: Some(requests),
            results,
            stop,
            active,
        })
    }
    fn reset(&mut self) {
        if let Some(job) = &self.job {
            job.stop.store(true, Ordering::SeqCst);
        }
        self.pending = None;
        self.onset = None;
        self.retained = None;
    }
    fn action(&mut self, action: Action) -> Option<Event> {
        match action.effect {
            Effect::Generate => {
                self.reset();
                self.ring.clear();
                self.generation = None;
            }
            Effect::Listen { replay } => {
                let event = if !replay.is_zero() {
                    self.retained.or(self.onset).map(|onset| Event::Replay {
                        generation: action.generation,
                        input: Input {
                            samples: self
                                .since(onset.saturating_sub((replay.as_secs_f64() * RATE as f64) as u64)),
                        },
                    })
                } else {
                    None
                };
                self.reset();
                self.generation = None;
                self.ring.clear();
                return event;
            }
            Effect::ProbeInterruption { probe, .. } if self.generation == Some(action.generation) => {
                self.pending = Some((probe, Instant::now() + PROBE));
            }
            // ResetVad precedes Listen on confirmed interruption: keep its onset until replay.
            Effect::ResetVad => {
                if let Some(job) = &self.job {
                    job.stop.store(true, Ordering::SeqCst);
                }
                self.pending = None;
                self.retained = self.onset.take().or(self.retained);
            }
            Effect::Finished => {
                self.reset();
                self.generation = None;
                self.ring.clear();
            }
            _ => {}
        }
        None
    }
    fn speaking(&mut self, generation: Generation) {
        self.reset();
        self.ring.clear();
        self.generation = Some(generation);
        self.next = Instant::now();
    }
    fn input(&mut self, samples: &[i16]) -> std::result::Result<(), Failure> {
        if samples.len() > RATE {
            return Err(Failure::Input);
        }
        self.frames = self
            .frames
            .checked_add(samples.len() as u64)
            .ok_or(Failure::Input)?;
        if self.generation.is_some() {
            let discard = (self.ring.len() + samples.len()).saturating_sub(WINDOW);
            self.ring.drain(..discard);
            self.ring.extend(samples);
        }
        Ok(())
    }
    fn since(&self, at: u64) -> Vec<i16> {
        let offset = self.frames.saturating_sub(self.ring.len() as u64);
        self.ring
            .iter()
            .skip(at.saturating_sub(offset).min(self.ring.len() as u64) as usize)
            .copied()
            .collect()
    }
    fn tick(&mut self, send: &mpsc::Sender<Command>) -> std::result::Result<(), Failure> {
        if !(self.active)() {
            return Err(Failure::Cancelled);
        }
        let now = Instant::now();
        if let Some(job) = &self.job
            && now >= job.until
        {
            job.stop.store(true, Ordering::SeqCst);
        }
        while let Ok(result) = self.results.try_recv() {
            self.job = None;
            if result.work.stop.load(Ordering::SeqCst) || self.generation != Some(result.work.generation) {
                continue;
            }
            let signal = match (result.work.kind, result.evidence) {
                (Kind::Speech, Ok(Evidence::Speech(onset)))
                    if self.frames.saturating_sub(result.work.through) <= FRESH as u64 =>
                {
                    match (self.onset, onset) {
                        (None, Some(start)) => {
                            self.retained = None;
                            self.onset = Some(
                                result
                                    .work
                                    .through
                                    .saturating_sub(result.work.samples.len() as u64)
                                    .saturating_add(start as u64),
                            );
                            Some(Signal::SpeechStarted)
                        }
                        (Some(_), None) => {
                            self.retained = self.onset.take();
                            Some(Signal::SpeechEnded)
                        }
                        _ => None,
                    }
                }
                (Kind::Words(probe), evidence)
                    if self
                        .pending
                        .is_some_and(|(current, until)| current == probe && now < until) =>
                {
                    self.pending = None;
                    Some(if matches!(evidence, Ok(Evidence::Words(true))) {
                        Signal::Confirm(probe)
                    } else {
                        Signal::Reject(probe)
                    })
                }
                (Kind::Speech, Err(())) => {
                    self.reset();
                    Some(Signal::SpeechEnded)
                }
                _ => None,
            };
            if let Some(signal) = signal {
                send.try_send(Command::Signal {
                    generation: result.work.generation,
                    signal,
                })
                .map_err(|_| Failure::Output)?;
            }
        }
        if self.results.is_closed() {
            return Err(Failure::Input);
        }
        let Some(generation) = self.generation else {
            return Ok(());
        };
        if self.job.is_some() || now < self.next {
            return Ok(());
        }
        if self.pending.is_some_and(|(_, until)| now >= until) {
            self.pending = None;
        }
        let (kind, samples, until) = if let Some((probe, until)) = self.pending {
            let samples = self.since(
                self.onset
                    .or(self.retained)
                    .unwrap_or(self.frames)
                    .saturating_sub(RATE as u64 * 260 / 1000),
            );
            if samples.len() < RATE / 2 {
                return Ok(());
            }
            (Kind::Words(probe), samples, until)
        } else {
            if self.frames == self.observed {
                return Ok(());
            }
            let samples = self.since(self.frames.saturating_sub((RATE * 3) as u64));
            if samples.len() < RATE / 2 {
                return Ok(());
            }
            self.observed = self.frames;
            (Kind::Speech, samples, now + Duration::from_secs(1))
        };
        let stop = Arc::new(AtomicBool::new(false));
        self.requests
            .as_ref()
            .ok_or(Failure::Cancelled)?
            .try_send(Work {
                generation,
                kind,
                samples,
                through: self.frames,
                stop: stop.clone(),
            })
            .map_err(|_| Failure::Input)?;
        self.job = Some(Job { stop, until });
        self.next = now + Duration::from_millis(100);
        Ok(())
    }
}
impl Drop for Detector {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        self.reset();
        self.requests.take();
    }
}

pub(super) fn available(model: Option<&str>) -> bool {
    model.is_some_and(|model| vults_voice::installed(&crate::voice::models_dir()).contains(&model))
        && vults_voice::vad_path(&crate::voice::models_dir()).is_some()
}

pub(super) async fn drive<F: Future<Output = std::result::Result<Option<(Turn, Failure)>, Failure>>>(
    channels: Channels,
    lease: Arc<Lease>,
    active: Active,
    devices: Arc<super::devices::Devices>,
    app: tauri::AppHandle,
    model: String,
    play: impl FnOnce(Channels) -> F,
) -> std::result::Result<Option<(Turn, Failure)>, Failure> {
    let path = vults_voice::vad_path(&crate::voice::models_dir()).ok_or(Failure::Input)?;
    if !available(Some(&model)) {
        return Err(Failure::Input);
    }
    let language = crate::voice::language(&app);
    let weak = Arc::downgrade(&devices);
    let check = active.clone();
    let authorized: Active = Arc::new(move || {
        check()
            && weak
                .upgrade()
                .is_some_and(|devices| devices.ready().is_ok_and(|ready| ready))
    });
    let decode_app = app.clone();
    let decode_model = model.clone();
    let decode_language = language.clone();
    let decode_path = path.clone();
    let assembler =
        super::routed_capture::Assembler::new(lease.clone(), authorized.clone(), move |pcm, stop| {
            let input: Vec<f32> = pcm.iter().map(|s| f32::from(*s) / 32768.0).collect();
            let pcm = vults_audio::resample(&input, 1, RATE as u32).map_err(|_| Failure::Input)?;
            let native = crate::voice::transcriber(&decode_app, &decode_model).map_err(|_| Failure::Input)?;
            native
                .transcribe_cancellable(&pcm, decode_language.as_deref(), Some(&decode_path), stop, true)
                .map_err(|_| Failure::Input)?
                .ok_or(Failure::Input)
        });
    let (mut vad, mut transcriber) = (None, None);
    let detector = Detector::start(lease, authorized, move |kind, pcm, stop| {
        if stop.load(Ordering::SeqCst) {
            return Err(());
        }
        let input: Vec<f32> = pcm.iter().map(|s| f32::from(*s) / 32768.0).collect();
        let pcm = vults_audio::resample(&input, 1, RATE as u32).map_err(|_| ())?;
        match kind {
            Kind::Speech => {
                if vad.is_none() {
                    vad = Some(vults_voice::Vad::load(&path).map_err(|_| ())?);
                }
                let speech = vad.as_mut().ok_or(())?.speech(&pcm).map_err(|_| ())?;
                let onset = speech
                    .last()
                    .filter(|range| range.end.saturating_add(3200) >= pcm.len())
                    .map(|range| range.start * RATE / vults_audio::SAMPLE_RATE as usize);
                Ok(Evidence::Speech(onset))
            }
            Kind::Words(_) => {
                if transcriber.is_none() {
                    transcriber = Some(crate::voice::transcriber(&app, &model).map_err(|_| ())?);
                }
                let text = transcriber
                    .as_ref()
                    .ok_or(())?
                    .transcribe_cancellable(&pcm, language.as_deref(), Some(&path), stop, true)
                    .map_err(|_| ())?;
                Ok(Evidence::Words(
                    text.is_some_and(|text| text.chars().any(char::is_alphanumeric)),
                ))
            }
        }
    })?;
    pump(channels, detector, Some(assembler), move || devices.input(), play).await
}

async fn pump<F: Future<Output = std::result::Result<Option<(Turn, Failure)>, Failure>>>(
    mut channels: Channels,
    mut detector: Detector,
    mut assembler: Option<super::routed_capture::Assembler>,
    mut input_audio: impl FnMut() -> std::result::Result<Option<Vec<i16>>, Failure>,
    play: impl FnOnce(Channels) -> F,
) -> std::result::Result<Option<(Turn, Failure)>, Failure> {
    let active = detector.active.clone();
    let (send, input) = mpsc::channel(4);
    let mut send = Some(send);
    let (output, mut events) = mpsc::channel(32);
    let driver = play(Channels {
        input,
        output,
        cancel: channels.cancel.clone(),
    });
    tokio::pin!(driver);
    let mut poll = tokio::time::interval(Duration::from_millis(5));
    let mut receiving = true;
    let mut receiving_events = true;
    loop {
        local::check(&*active, &channels.cancel)?;
        tokio::select! { biased;
            event = events.recv(), if receiving_events => if let Some(event) = event {
                let replay = match &event {
                    Event::Action(action) => detector.action(*action),
                    Event::Speaking(generation) => { detector.speaking(*generation); None },
                    _ => None,
                };
                if let Some(assembler) = &mut assembler {
                    assembler.event(&event);
                    if let Some(replay) = &replay { assembler.event(replay); }
                }
                channels.output.try_send(event).map_err(|_| Failure::Output)?;
                if let Some(replay) = replay { channels.output.try_send(replay).map_err(|_| Failure::Output)?; }
            } else { receiving_events = false; },
            command = channels.input.recv(), if receiving => match command {
                Some(Command::Capture { generation, operation }) => {
                    if let Some(assembler) = &mut assembler && assembler.command(generation, operation)? {
                        let phase = match operation {
                            super::RoutedCapture::Begin => Some(super::CapturePhase::Recording),
                            super::RoutedCapture::Finish => Some(super::CapturePhase::Transcribing),
                            super::RoutedCapture::Cancel => None,
                        };
                        if let Some(phase) = phase { channels.output.try_send(Event::Capture(phase)).map_err(|_| Failure::Output)?; }
                    }
                },
                Some(command) => {
                    if let Some(assembler) = &mut assembler && let Command::Turn { generation, .. } = &command { assembler.external(*generation); }
                    send.as_ref().ok_or(Failure::Cancelled)?.try_send(command).map_err(|_| Failure::Output)?; },
                None => { receiving = false; if let Some(assembler) = &mut assembler { assembler.clear(); } detector.reset(); detector.generation = None; send.take(); },
            },
            _ = poll.tick() => {
                while let Some(samples) = input_audio()? {
                    detector.input(&samples)?;
                    if let Some(assembler) = &mut assembler { assembler.input(&samples)?; }
                    channels.output.try_send(Event::Input(Input { samples })).map_err(|_| Failure::Output)?;
                }
                if let Some(send) = &send {
                    detector.tick(send)?;
                    if let Some(assembler) = &mut assembler && let Some(command) = assembler.poll()? {
                        send.try_send(command).map_err(|_| Failure::Output)?;
                    }
                }
            },
            result = &mut driver => {
                while let Ok(event) = events.try_recv() { channels.output.try_send(event).map_err(|_| Failure::Output)?; }
                return result;
            },
        }
    }
}

#[cfg(test)]
mod tests;
