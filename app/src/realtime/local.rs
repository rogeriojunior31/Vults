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

const RATE: usize = 24_000;
const MAX_FRAMES: usize = RATE * 30;
const CHUNK: usize = RATE / 10;

#[derive(Clone)]
pub struct Turn {
    samples: Arc<[i16]>,
    transcript: Option<Arc<str>>,
    handled: Arc<AtomicBool>,
}
impl Turn {
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
        if text.trim().is_empty()
            || text.len() > vults_dictation::command::MAX_INPUT_BYTES
            || text.chars().any(|c| c.is_control() && !c.is_whitespace())
        {
            return Err(Failure::Input);
        }
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
