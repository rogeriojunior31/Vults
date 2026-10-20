//! A native complete turn survives remote admission and transport failure in bounded memory.
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use tokio::sync::watch;
use vults_chat::realtime::transport::Failure;

const RATE: usize = vults_audio::playback::RATE as usize;
const MAX_FRAMES: usize = RATE * 30;
const CHUNK: usize = RATE / 10;

#[derive(Clone)]
pub struct Turn {
    samples: Arc<[i16]>,
    transcript: Option<Arc<str>>,
    handled: Arc<AtomicBool>,
}
impl Turn {
    /// Completed native 16 kHz capture and its finalized local transcript; no capture or upload.
    pub fn from_capture(pcm: &[f32], text: String) -> Result<Self, Failure> {
        let source_rate = vults_audio::SAMPLE_RATE as usize;
        if !(source_rate / 10..=source_rate * 30).contains(&pcm.len())
            || pcm.iter().any(|sample| !sample.is_finite())
        {
            return Err(Failure::Input);
        }
        check_transcript(&text)?;
        let mut converter =
            vults_audio::MonoResampler::at_rate(vults_audio::SAMPLE_RATE, vults_audio::playback::RATE)
                .map_err(|_| Failure::Input)?;
        let mut converted = converter.push(pcm).map_err(|_| Failure::Input)?;
        converted.extend(converter.finish().map_err(|_| Failure::Input)?);
        if converted.iter().any(|sample| !sample.is_finite()) {
            return Err(Failure::Input);
        }
        // The filter may overshoot full scale. Float-to-integer casts saturate at PCM16 limits.
        let mut turn = Self::new(
            converted
                .into_iter()
                .map(|sample| (sample * 32768.0).round() as i16)
                .collect(),
        )?;
        turn.transcript = Some(text.into());
        Ok(turn)
    }

    /// Already captured, mono PCM16 at 24 kHz. This grants no microphone authority.
    pub fn new(pcm: Vec<i16>) -> Result<Self, Failure> {
        if !(CHUNK..=MAX_FRAMES).contains(&pcm.len()) {
            return Err(Failure::Input);
        }
        Ok(Self {
            samples: pcm.into(),
            transcript: None,
            handled: Arc::default(),
        })
    }
    /// A finalized local transcript of this same capture, supplied by its native capture owner.
    pub fn with_transcript(mut self, text: String) -> Result<Self, Failure> {
        check_transcript(&text)?;
        self.transcript = Some(text.into());
        Ok(self)
    }
    #[cfg(target_os = "linux")]
    pub(super) fn transcript(&self) -> Option<&str> {
        self.transcript.as_deref()
    }
    pub(super) fn samples(&self) -> Arc<[i16]> {
        self.samples.clone()
    }
    pub(super) fn handled(&self) -> bool {
        self.handled.load(Ordering::SeqCst)
    }
    pub(super) fn consume(&self) {
        self.handled.store(true, Ordering::SeqCst);
    }
}
fn check_transcript(text: &str) -> Result<(), Failure> {
    if text.trim().is_empty()
        || text.len() > vults_dictation::command::MAX_INPUT_BYTES
        || text.chars().any(|c| c.is_control() && !c.is_whitespace())
    {
        Err(Failure::Input)
    } else {
        Ok(())
    }
}
impl std::fmt::Debug for Turn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Turn")
            .field("frames", &self.samples.len())
            .field("transcript_bytes", &self.transcript.as_ref().map(|t| t.len()))
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalFailure {
    Unavailable,
    Decode,
    Empty,
    Timeout,
}

pub enum Recovery {
    Text {
        reason: Failure,
        text: String,
    },
    Audio {
        reason: Failure,
        turn: Turn,
        failure: LocalFailure,
    },
}
impl std::fmt::Debug for Recovery {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Text { reason, text } => f
                .debug_struct("TextRecovery")
                .field("reason", reason)
                .field("bytes", &text.len())
                .finish(),
            Self::Audio {
                reason,
                turn,
                failure,
            } => f
                .debug_struct("AudioRecovery")
                .field("reason", reason)
                .field("turn", turn)
                .field("failure", failure)
                .finish(),
        }
    }
}

pub(super) fn check(
    active: &(dyn Fn() -> bool + Send + Sync),
    cancel: &watch::Receiver<bool>,
) -> Result<(), Failure> {
    if *cancel.borrow() || cancel.has_changed().is_err() {
        Err(Failure::Cancelled)
    } else if !active() {
        Err(Failure::Off)
    } else {
        Ok(())
    }
}

struct Stop(Arc<std::sync::atomic::AtomicBool>);
impl Drop for Stop {
    fn drop(&mut self) {
        self.0.store(true, std::sync::atomic::Ordering::SeqCst);
    }
}

pub(super) async fn recover(
    turn: Turn,
    reason: Failure,
    active: Arc<dyn Fn() -> bool + Send + Sync>,
    mut cancel: watch::Receiver<bool>,
    lease: Arc<super::Lease>,
    decode: impl FnOnce(Arc<[i16]>, Arc<std::sync::atomic::AtomicBool>) -> Result<String, LocalFailure>
    + Send
    + 'static,
) -> Result<Recovery, Failure> {
    check(&*active, &cancel)?;
    if turn.handled() {
        return Err(Failure::Input);
    }
    if let Some(text) = &turn.transcript {
        let text = text.to_string();
        check(&*active, &cancel)?;
        return Ok(Recovery::Text { reason, text });
    }
    let stop = Stop(Arc::default());
    let (pcm, flag) = (turn.samples(), stop.0.clone());
    let worker_lease = lease.clone();
    let mut worker = tokio::task::spawn_blocking(move || {
        let _lease = worker_lease;
        decode(pcm, flag)
    });
    let deadline = tokio::time::Instant::now() + Duration::from_secs(30);
    let mut poll = tokio::time::interval(Duration::from_millis(25));
    let result = loop {
        check(&*active, &cancel)?;
        tokio::select! { biased;
            _ = cancel.changed() => {},
            _ = tokio::time::sleep_until(deadline) => break Err(LocalFailure::Timeout),
            _ = poll.tick() => {},
            result = &mut worker => break result.unwrap_or(Err(LocalFailure::Decode)),
        }
    };
    check(&*active, &cancel)?;
    match result {
        Ok(text) if text.trim().is_empty() => Ok(Recovery::Audio {
            reason,
            turn,
            failure: LocalFailure::Empty,
        }),
        Ok(text) if text.len() <= 64 * 1024 && !text.contains('\0') => Ok(Recovery::Text { reason, text }),
        result => Ok(Recovery::Audio {
            reason,
            turn,
            failure: result.err().unwrap_or(LocalFailure::Decode),
        }),
    }
}

#[cfg(test)]
mod tests;
