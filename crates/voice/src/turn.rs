//! Pure conversation timing. Adapters own consent, microphone/ring buffers, providers and playback.
//! Events acknowledge actual playback, not queued audio. No effect grants microphone access.

use std::time::Duration;

pub mod end;
pub mod heard;
#[cfg(feature = "semantic-turn")]
pub mod semantic;

pub const REPLAY: Duration = Duration::from_millis(260);
pub const TAIL_GRACE: Duration = Duration::from_millis(450);
const CONFIRM: Duration = Duration::from_millis(1500);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BargeIn {
    #[default]
    Disabled,
    Headphones,
    EchoCancelled,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Generation(u64);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Probe(u64);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Idle,
    Listening,
    Thinking,
    SpeakingLocked,
    Armed,
    InterruptPending,
    Draining,
    TailGrace,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    InputCommitted,
    /// A local command consumed this input without creating a model response.
    LocalCompleted,
    FirstAudioPlayed {
        at: Duration,
    },
    SpeechStarted,
    SpeechEnded,
    /// Discard speech/probes whose native route can no longer be verified.
    InputUnavailable,
    InterruptConfirmed(Probe),
    InterruptRejected(Probe),
    ResponseFinished,
    /// Only acknowledge the drain requested after ResponseFinished, never a transient empty queue.
    PlaybackDrained,
    Tick,
    Cancel,
    Failed,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Effect {
    /// Reuse an already authorized capture; the adapter must retain/replay this much mic audio.
    Listen {
        replay: Duration,
    },
    Generate,
    /// Decode in a separate scratch session; its result must carry this exact probe token.
    ProbeInterruption {
        probe: Probe,
        replay: Duration,
    },
    CancelResponse,
    ClearPlayback,
    DrainPlayback,
    ResetVad,
    Finished,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Action {
    pub generation: Generation,
    pub effect: Effect,
}

#[derive(Debug)]
pub struct Turn {
    generation: Generation,
    phase: Phase,
    mode: BargeIn,
    probe: Probe,
    now: Duration,
    unlock: Option<Duration>,
    response_done: bool,
    speech: bool,
    deadline: Option<Duration>,
}
impl Default for Turn {
    fn default() -> Self {
        Self::new(BargeIn::Disabled)
    }
}
impl Turn {
    pub fn new(mode: BargeIn) -> Self {
        Self {
            generation: Generation(0),
            phase: Phase::Idle,
            mode,
            probe: Probe(0),
            now: Duration::ZERO,
            unlock: None,
            response_done: false,
            speech: false,
            deadline: None,
        }
    }
    pub fn phase(&self) -> Phase {
        self.phase
    }
    pub fn generation(&self) -> Generation {
        self.generation
    }

    /// The caller must first authorize an explicit user-started voice session.
    pub fn start(&mut self, now: Duration) -> Option<Action> {
        if self.phase != Phase::Idle || now < self.now {
            return None;
        }
        self.now = now;
        self.listen(Duration::ZERO)
    }
    fn action(&self, effect: Effect) -> Action {
        Action {
            generation: self.generation,
            effect,
        }
    }
    fn listen(&mut self, replay: Duration) -> Option<Action> {
        let Some(next) = self.generation.0.checked_add(1) else {
            self.phase = Phase::Idle;
            return None;
        };
        self.generation = Generation(next);
        self.phase = Phase::Listening;
        self.unlock = None;
        self.response_done = false;
        self.speech = false;
        self.deadline = None;
        Some(self.action(Effect::Listen { replay }))
    }
    fn ready(&self) -> bool {
        self.mode != BargeIn::Disabled && self.unlock.is_some_and(|at| self.now >= at)
    }
    fn probe(&mut self, out: &mut Vec<Action>) {
        let Some(next) = self.probe.0.checked_add(1) else {
            return;
        };
        self.probe = Probe(next);
        self.phase = Phase::InterruptPending;
        self.deadline = Some(self.now.saturating_add(CONFIRM));
        out.push(self.action(Effect::ProbeInterruption {
            probe: self.probe,
            replay: REPLAY,
        }));
    }
    fn reject(&mut self, out: &mut Vec<Action>) {
        self.phase = if self.response_done {
            Phase::Draining
        } else {
            Phase::Armed
        };
        self.deadline = None;
        self.speech = false;
        out.push(self.action(Effect::ResetVad));
    }
    fn finish(&mut self, out: &mut Vec<Action>) {
        self.phase = Phase::Idle;
        self.speech = false;
        self.deadline = None;
        out.push(self.action(Effect::ResetVad));
        out.push(self.action(Effect::Finished));
    }
    pub fn handle(&mut self, generation: Generation, event: Event, now: Duration) -> Vec<Action> {
        let mut out = Vec::new();
        if generation != self.generation || self.phase == Phase::Idle || now < self.now {
            return out;
        }
        self.now = now;
        if matches!(event, Event::Cancel | Event::Failed) {
            out.push(self.action(Effect::CancelResponse));
            out.push(self.action(Effect::ClearPlayback));
            self.finish(&mut out);
            return out;
        }
        if self.phase == Phase::TailGrace && self.deadline.is_some_and(|at| now >= at) {
            self.finish(&mut out);
            return out;
        }
        if self.phase == Phase::InterruptPending && self.deadline.is_some_and(|at| now >= at) {
            self.reject(&mut out);
        }
        if event == Event::InputUnavailable {
            let speech = self.speech;
            self.speech = false;
            if self.phase == Phase::InterruptPending {
                self.reject(&mut out);
            } else if speech {
                out.push(self.action(Effect::ResetVad));
            }
            return out;
        }
        if event == Event::SpeechEnded {
            self.speech = false;
        }
        // Suppressed onset survives the first-playback lock only while speech remains active.
        if matches!(self.phase, Phase::SpeakingLocked | Phase::Draining) && self.ready() {
            if self.phase == Phase::SpeakingLocked {
                self.phase = Phase::Armed;
            }
            if self.speech {
                self.probe(&mut out);
            }
        }
        match event {
            Event::LocalCompleted if self.phase == Phase::Listening => self.finish(&mut out),
            Event::InputCommitted if self.phase == Phase::Listening => {
                self.phase = Phase::Thinking;
                out.push(self.action(Effect::Generate));
            }
            Event::FirstAudioPlayed { at }
                if at <= now
                    && (self.phase == Phase::Thinking
                        || (self.phase == Phase::Draining && self.unlock.is_none())) =>
            {
                if !self.response_done {
                    self.phase = Phase::SpeakingLocked;
                }
                self.unlock = Some(at.saturating_add(match self.mode {
                    BargeIn::EchoCancelled => Duration::from_secs(1),
                    _ => Duration::from_millis(500),
                }));
                if self.ready() {
                    if self.phase == Phase::SpeakingLocked {
                        self.phase = Phase::Armed;
                    }
                    if self.speech {
                        self.probe(&mut out);
                    }
                }
            }
            Event::SpeechStarted if self.phase == Phase::TailGrace => {
                out.push(self.action(Effect::ResetVad));
                if let Some(listen) = self.listen(Duration::ZERO) {
                    out.push(listen);
                }
            }
            Event::SpeechStarted
                if matches!(
                    self.phase,
                    Phase::Thinking | Phase::SpeakingLocked | Phase::Armed | Phase::Draining
                ) =>
            {
                self.speech = true;
                if self.ready() {
                    self.probe(&mut out);
                }
            }
            Event::SpeechEnded => {
                self.speech = false;
            }
            Event::InterruptRejected(probe)
                if self.phase == Phase::InterruptPending && probe == self.probe =>
            {
                self.reject(&mut out)
            }
            Event::InterruptConfirmed(probe)
                if self.phase == Phase::InterruptPending && probe == self.probe =>
            {
                out.push(self.action(Effect::CancelResponse));
                out.push(self.action(Effect::ClearPlayback));
                out.push(self.action(Effect::ResetVad));
                if let Some(listen) = self.listen(REPLAY) {
                    out.push(listen);
                }
            }
            Event::ResponseFinished
                if matches!(
                    self.phase,
                    Phase::Thinking
                        | Phase::SpeakingLocked
                        | Phase::Armed
                        | Phase::InterruptPending
                        | Phase::Draining
                ) && !self.response_done =>
            {
                self.response_done = true;
                if self.phase != Phase::InterruptPending {
                    self.phase = Phase::Draining;
                }
                out.push(self.action(Effect::DrainPlayback));
            }
            Event::PlaybackDrained
                if self.response_done && matches!(self.phase, Phase::Draining | Phase::InterruptPending) =>
            {
                if self.unlock.is_none() {
                    self.finish(&mut out);
                    return out;
                }
                let pending = self.phase == Phase::InterruptPending;
                self.phase = Phase::TailGrace;
                self.deadline = Some(now.saturating_add(TAIL_GRACE));
                out.push(self.action(Effect::ResetVad));
                // A pending user onset becomes a new turn once output is silent, never a cancel.
                if pending && let Some(listen) = self.listen(REPLAY) {
                    out.push(listen);
                }
            }
            _ => {}
        }
        out
    }
}

#[cfg(test)]
mod tests;
