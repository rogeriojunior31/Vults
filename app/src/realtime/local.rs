//! A native complete turn survives remote admission and transport failure in bounded memory.
use std::{future::Future, sync::Arc, time::Duration};
use tokio::sync::{mpsc, watch};
use vults_chat::realtime::transport::{self, Command, Event, Failure};

const RATE: usize = 24_000;
const MAX_FRAMES: usize = RATE * 30;
const CHUNK: usize = RATE / 10;

#[derive(Clone)]
pub struct Turn(Arc<[i16]>);
impl Turn {
    /// Already captured, mono PCM16 at 24 kHz. This grants no microphone authority.
    pub fn new(pcm: Vec<i16>) -> Result<Self, Failure> {
        if !(CHUNK..=MAX_FRAMES).contains(&pcm.len()) {
            return Err(Failure::Input);
        }
        Ok(Self(pcm.into()))
    }
    pub(super) fn samples(&self) -> Arc<[i16]> {
        self.0.clone()
    }
}
impl std::fmt::Debug for Turn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Turn").field("frames", &self.0.len()).finish()
    }
}

#[derive(Debug)]
pub struct Channels {
    /// Supply complete turns only after explicit native capture; at most one is processed at once.
    pub input: mpsc::Receiver<Turn>,
    pub output: mpsc::Sender<Event>,
    pub cancel: watch::Receiver<bool>,
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

/// The provider future stays borrowed here: dropping this owner also drops its socket.
pub(super) async fn drive<F: Future<Output = Result<(), Failure>>>(
    mut channels: Channels,
    active: &(dyn Fn() -> bool + Send + Sync),
    remote: impl FnOnce(transport::Channels) -> F,
) -> Result<Option<(Turn, Failure)>, Failure> {
    let mut poll = tokio::time::interval(Duration::from_millis(25));
    let first = loop {
        check(active, &channels.cancel)?;
        tokio::select! { biased;
            _ = channels.cancel.changed() => {},
            _ = poll.tick() => {},
            turn = channels.input.recv() => break turn,
        }
    };
    let Some(first) = first else {
        return Ok(None);
    };
    let mut pending = Some(first);
    let (send, input) = mpsc::channel(2);
    let (output, mut receive) = mpsc::channel(4);
    let (stop, cancel) = watch::channel(false);
    let provider = remote(transport::Channels {
        input,
        output,
        cancel,
    });
    tokio::pin!(provider);
    let (mut send, mut ready, mut offset, mut closed, mut stopped, mut receiving) =
        (Some(send), false, Some(0), false, false, true);
    let mut terminal = None;
    let result = loop {
        if let Err(error) = check(active, &channels.cancel) {
            break Err(error);
        }
        // A provider may enqueue its final receipt and return within the same poll.
        if stopped && receive.is_empty() {
            break terminal.take().unwrap_or(Err(Failure::Transport)).and_then(|()| {
                if pending.is_none() {
                    Ok(())
                } else {
                    Err(Failure::Transport)
                }
            });
        }
        tokio::select! { biased;
            _ = channels.cancel.changed() => {},
            _ = poll.tick() => {},
            event = receive.recv(), if receiving => {
                match event {
                    Some(Event::Ready) => {
                        ready = true;
                        if channels.output.try_send(Event::Ready).is_err() { break Err(Failure::Output); }
                    },
                    Some(Event::Finished(receipt)) => {
                        if pending.is_none() || offset.is_some() { break Err(Failure::Protocol); }
                        if !receipt.within_bounds { break Err(Failure::Protocol); }
                        if !receipt.ok { break Err(Failure::Response); }
                        pending = None;
                        offset = None;
                        if channels.output.try_send(Event::Finished(receipt)).is_err() { break Err(Failure::Output); }
                    },
                    Some(event @ Event::Audio { .. }) => {
                        if pending.is_none() || offset.is_some() { break Err(Failure::Protocol); }
                        if channels.output.try_send(event).is_err() { break Err(Failure::Output); }
                    },
                    None => receiving = false,
                }
            },
            result = &mut provider, if !stopped => {
                stopped = true;
                terminal = Some(result);
            },
            turn = channels.input.recv(), if pending.is_none() && !closed && !stopped => {
                match turn {
                    Some(turn) => { pending = Some(turn); offset = Some(0); },
                    None => { closed = true; send.take(); },
                }
            },
            permit = async {
                send.as_ref().ok_or(Failure::Transport)?.clone().reserve_owned().await.map_err(|_| Failure::Transport)
            }, if offset.is_some() && ready && !stopped => {
                let permit = match permit { Ok(permit) => permit, Err(error) => break Err(error) };
                let Some(turn) = &pending else { break Err(Failure::Input); };
                let start = offset.unwrap_or(0);
                if start < turn.0.len() {
                    let end = (start + CHUNK).min(turn.0.len());
                    permit.send(Command::Audio(turn.0[start..end].to_vec()));
                    offset = Some(end);
                } else {
                    permit.send(Command::Commit);
                    offset = None;
                    ready = false;
                }
            },
        }
    };
    drop(send);
    let _ = stop.send(true);
    if !stopped {
        // Give the existing transport its bounded settlement path before local recovery.
        let _ = tokio::time::timeout(Duration::from_secs(11), &mut provider).await;
    }
    check(active, &channels.cancel)?;
    if channels.output.is_closed() {
        return Err(Failure::Output);
    }
    match (result, pending) {
        (Err(Failure::Cancelled), _) => result.map(|()| None),
        (Err(reason), Some(turn)) => Ok(Some((turn, reason))),
        (Err(error), None) => Err(error),
        (Ok(()), _) => Ok(None),
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
