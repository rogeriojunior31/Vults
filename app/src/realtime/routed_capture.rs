//! Bounded capture from the already admitted duplex stream; only explicit Finish commits a turn.
use super::{Command, Event, Lease, Turn};
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};
use vults_chat::realtime::transport::Failure;
use vults_voice::turn::{Effect, Generation};

const RATE: usize = vults_audio::playback::RATE as usize;
const MAX: usize = RATE * 30;
const DECODE: Duration = Duration::from_secs(60);

/// Explicit native controls for run_routed headphones/AEC only; other entries ignore them.
#[derive(Debug, Clone, Copy)]
pub enum Operation {
    Begin,
    Finish,
    Cancel,
}
type Active = Arc<dyn Fn() -> bool + Send + Sync>;
type Decode = Arc<dyn Fn(&[i16], &AtomicBool) -> Result<String, Failure> + Send + Sync>;
struct Job {
    generation: Generation,
    result: mpsc::Receiver<Result<Turn, Failure>>,
    stop: Arc<AtomicBool>,
    until: Instant,
}
pub(super) struct Assembler {
    generation: Option<Generation>,
    replay: Vec<i16>,
    samples: Option<Vec<i16>>,
    job: Option<Job>,
    lease: Arc<Lease>,
    active: Active,
    decode: Decode,
}
impl Assembler {
    pub(super) fn new(
        lease: Arc<Lease>,
        active: Active,
        decode: impl Fn(&[i16], &AtomicBool) -> Result<String, Failure> + Send + Sync + 'static,
    ) -> Self {
        Self {
            generation: None,
            replay: Vec::new(),
            samples: None,
            job: None,
            lease,
            active,
            decode: Arc::new(decode),
        }
    }
    pub(super) fn clear(&mut self) {
        if let Some(job) = &self.job {
            job.stop.store(true, Ordering::SeqCst);
        }
        self.generation = None;
        self.samples = None;
        self.replay.clear();
    }
    pub(super) fn external(&mut self, generation: Generation) {
        if self.generation == Some(generation) {
            self.clear();
        }
    }
    pub(super) fn event(&mut self, event: &Event) {
        match event {
            Event::Action(action) => match action.effect {
                Effect::Listen { .. } => {
                    self.clear();
                    self.generation = Some(action.generation);
                }
                Effect::Generate | Effect::Finished => self.clear(),
                _ => {}
            },
            Event::Replay { generation, input }
                if self.generation == Some(*generation)
                    && self.samples.is_none()
                    && input.samples.len() <= RATE * 8 =>
            {
                self.replay.clone_from(&input.samples);
            }
            _ => {}
        }
    }
    pub(super) fn input(&mut self, pcm: &[i16]) -> Result<(), Failure> {
        if !(self.active)() {
            self.clear();
            return Err(Failure::Cancelled);
        }
        if let Some(samples) = &mut self.samples {
            if pcm.len() > RATE || samples.len().saturating_add(pcm.len()) > MAX {
                self.clear();
                return Err(Failure::Input);
            }
            samples.extend_from_slice(pcm);
        }
        Ok(())
    }
    pub(super) fn command(&mut self, generation: Generation, operation: Operation) -> Result<bool, Failure> {
        if !(self.active)() {
            self.clear();
            return Err(Failure::Cancelled);
        }
        if self.generation != Some(generation) {
            return Ok(false);
        }
        match operation {
            Operation::Begin if self.samples.is_none() && self.job.is_none() => {
                self.samples = Some(std::mem::take(&mut self.replay));
            }
            Operation::Finish if self.samples.is_some() && self.job.is_none() => {
                let turn = Turn::new(self.samples.take().ok_or(Failure::Input)?)?;
                let (send, result) = mpsc::sync_channel(1);
                let stop = Arc::new(AtomicBool::new(false));
                let cancelled = stop.clone();
                let active = self.active.clone();
                let decode = self.decode.clone();
                let lease = self.lease.clone();
                std::thread::Builder::new()
                    .name("voice-routed-turn".into())
                    .spawn(move || {
                        let value = if cancelled.load(Ordering::SeqCst) || !active() {
                            Err(Failure::Cancelled)
                        } else {
                            decode(&turn.samples(), &cancelled).and_then(|text| turn.with_transcript(text))
                        };
                        if !cancelled.load(Ordering::SeqCst) && active() {
                            let _ = send.try_send(value);
                        }
                        // Model destruction may block: actual native ownership outlives cancellation.
                        drop(decode);
                        drop(lease);
                    })
                    .map_err(|_| Failure::Input)?;
                self.job = Some(Job {
                    generation,
                    result,
                    stop,
                    until: Instant::now() + DECODE,
                });
            }
            Operation::Cancel => self.clear(),
            _ => return Ok(false),
        }
        Ok(true)
    }
    pub(super) fn poll(&mut self) -> Result<Option<Command>, Failure> {
        if !(self.active)() {
            self.clear();
            return Err(Failure::Cancelled);
        }
        let Some(job) = &self.job else {
            return Ok(None);
        };
        if Instant::now() >= job.until && !job.stop.load(Ordering::SeqCst) {
            self.clear();
            return Err(Failure::Input);
        }
        match job.result.try_recv() {
            Ok(result) => {
                let job = self.job.take().ok_or(Failure::Input)?;
                if job.stop.load(Ordering::SeqCst) || self.generation != Some(job.generation) {
                    return Ok(None);
                }
                let turn = result?;
                self.generation = None;
                Ok(Some(Command::Turn {
                    generation: job.generation,
                    turn,
                }))
            }
            Err(mpsc::TryRecvError::Empty) => Ok(None),
            Err(mpsc::TryRecvError::Disconnected) => {
                let job = self.job.take().ok_or(Failure::Input)?;
                if job.stop.load(Ordering::SeqCst) {
                    Ok(None)
                } else {
                    Err(Failure::Input)
                }
            }
        }
    }
}
impl Drop for Assembler {
    fn drop(&mut self) {
        self.clear();
    }
}

#[cfg(test)]
mod tests;
