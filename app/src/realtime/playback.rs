//! Native playback/history owner; no microphone, renderer command or persistent transcript storage.
use super::local::{self, Turn};
use std::{
    future::Future,
    sync::{Arc, atomic::AtomicBool},
    time::Duration,
};
use tokio::sync::{mpsc, watch};
use vults_audio::playback::{Cursor, Phase as OutputPhase, Playback};
use vults_chat::realtime::{
    Identity, Repair, Settlement, Transcript,
    transport::{self, Failure},
};
use vults_voice::turn::{self, Action, Effect, Generation, Probe, heard};

#[derive(Debug)]
pub enum Command {
    Turn { generation: Generation, turn: Turn },
    Signal { generation: Generation, signal: Signal },
}
#[derive(Debug, Clone, Copy)]
pub enum Signal {
    SpeechStarted,
    SpeechEnded,
    Confirm(Probe),
    Reject(Probe),
}
impl Signal {
    fn event(self) -> turn::Event {
        match self {
            Self::SpeechStarted => turn::Event::SpeechStarted,
            Self::SpeechEnded => turn::Event::SpeechEnded,
            Self::Confirm(probe) => turn::Event::InterruptConfirmed(probe),
            Self::Reject(probe) => turn::Event::InterruptRejected(probe),
        }
    }
}
pub struct Record {
    pub identity: Identity,
    pub text: String,
    pub interrupted: bool,
    pub played_frames: u64,
    pub total_frames: u64,
    pub server_synchronized: bool,
}
impl std::fmt::Debug for Record {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VoiceHistory")
            .field("identity", &self.identity)
            .field("bytes", &self.text.len())
            .field("interrupted", &self.interrupted)
            .field("played_frames", &self.played_frames)
            .field("total_frames", &self.total_frames)
            .finish()
    }
}
pub struct LocalReply {
    pub feedback: crate::voice_commands::Feedback,
    pub text: String,
    pub choices: Vec<vults_dictation::command::names::Name>,
}
impl std::fmt::Debug for LocalReply {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LocalReply")
            .field("feedback", &self.feedback)
            .field("bytes", &self.text.len())
            .field("choices", &self.choices.len())
            .finish()
    }
}
#[derive(Debug)]
pub enum Event {
    Action(Action),
    Ready(Generation),
    History(Record),
    Settled(Settlement),
    Input(Input),
    Local {
        generation: Generation,
        reply: LocalReply,
    },
}
pub struct Input {
    pub samples: Vec<i16>,
}
impl std::fmt::Debug for Input {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VoiceInput")
            .field("frames", &self.samples.len())
            .finish()
    }
}
#[derive(Debug)]
pub struct Channels {
    pub input: mpsc::Receiver<Command>,
    pub output: mpsc::Sender<Event>,
    pub cancel: watch::Receiver<bool>,
}

// This boundary also lets deterministic tests drive the real owner without opening speakers.
pub(super) trait Output {
    fn push(&self, pcm: &[i16]) -> Result<(), Failure>;
    fn finish(&self) -> Result<(), Failure>;
    fn cursor(&self) -> Cursor;
    fn stop(&self) -> Cursor;
    fn route_verified(&self) -> bool {
        true
    }
}
impl Output for Playback {
    fn push(&self, pcm: &[i16]) -> Result<(), Failure> {
        self.push(pcm).map_err(|_| Failure::Output)
    }
    fn finish(&self) -> Result<(), Failure> {
        self.finish().map_err(|_| Failure::Output)
    }
    fn cursor(&self) -> Cursor {
        self.cursor()
    }
    fn stop(&self) -> Cursor {
        self.stop()
    }
}
struct Response<P> {
    generation: Generation,
    identity: Option<Identity>,
    output: P,
    transcript: Option<Transcript>,
    settled: bool,
    first_played: bool,
    frozen: Option<Cursor>,
    truncated: bool,
    local_only: bool,
    committed: bool,
    tool_attempts: std::collections::BTreeSet<String>,
}
fn emit(output: &mpsc::Sender<Event>, event: Event) -> Result<(), Failure> {
    output.try_send(event).map_err(|_| Failure::Output)
}
fn actions(output: &mpsc::Sender<Event>, actions: Vec<Action>) -> Result<bool, Failure> {
    let interrupted = actions.iter().any(|a| a.effect == Effect::ClearPlayback);
    for action in actions {
        emit(output, Event::Action(action))?;
    }
    Ok(interrupted)
}

pub(super) const LOCAL_TIMEOUT: Duration = Duration::from_secs(3);
async fn prepare_turn(
    future: impl Future<Output = Result<Option<LocalReply>, Failure>>,
    active: &(dyn Fn() -> bool + Send + Sync),
    channels: &Channels,
) -> Result<Option<LocalReply>, Failure> {
    tokio::pin!(future);
    let deadline = tokio::time::sleep(LOCAL_TIMEOUT);
    tokio::pin!(deadline);
    let mut poll = tokio::time::interval(Duration::from_millis(5));
    loop {
        local::check(active, &channels.cancel)?;
        if channels.output.is_closed() {
            return Err(Failure::Output);
        }
        tokio::select! { biased;
            _ = &mut deadline => return Err(Failure::Timeout),
            _ = poll.tick() => {},
            result = &mut future => return result,
        }
    }
}

/// Dropping this borrowed future drops its transport/socket and stops its native output.
pub(super) async fn drive<
    P: Output,
    F: Future<Output = Result<(), Failure>>,
    Q: Future<Output = Result<Option<LocalReply>, Failure>>,
>(
    mut channels: Channels,
    active: &(dyn Fn() -> bool + Send + Sync),
    mode: turn::BargeIn,
    mut open: impl FnMut(Arc<AtomicBool>) -> Result<P, Failure>,
    remote: impl FnOnce(transport::Channels) -> F,
    mut prepare: impl FnMut(Turn) -> Q,
) -> Result<Option<(Turn, Failure)>, Failure> {
    let start = std::time::Instant::now();
    let mut machine = turn::Turn::new(mode);
    let listen = machine.start(Duration::ZERO).ok_or(Failure::Input)?;
    local::check(active, &channels.cancel)?;
    emit(&channels.output, Event::Action(listen))?;
    let mut poll = tokio::time::interval(Duration::from_millis(5));
    let first = loop {
        local::check(active, &channels.cancel)?;
        tokio::select! { biased;
            _ = channels.cancel.changed() => {},
            _ = poll.tick() => {},
            command = channels.input.recv() => match command {
                Some(Command::Turn { generation, turn }) if generation == machine.generation() => {
                    let prepared = prepare_turn(prepare(turn.clone()), active, &channels).await;
                    local::check(active, &channels.cancel)?;
                    if channels.output.is_closed() { return Err(Failure::Output); }
                    match prepared {
                        Ok(Some(reply)) => {
                            emit(&channels.output, Event::Local { generation, reply })?;
                            actions(&channels.output, machine.handle(generation, turn::Event::LocalCompleted, start.elapsed()))?;
                            let listen = machine.start(start.elapsed()).ok_or(Failure::Input)?;
                            emit(&channels.output, Event::Action(listen))?;
                        },
                        Ok(None) => break Some(turn),
                        Err(reason) => return Ok(Some((turn, reason))),
                    }
                },
                Some(_) => {},
                None => break None,
            },
        }
    };
    let Some(first) = first else {
        return Ok(None);
    };
    if let Err(error) = actions(
        &channels.output,
        machine.handle(machine.generation(), turn::Event::InputCommitted, start.elapsed()),
    ) {
        local::check(active, &channels.cancel)?;
        return if channels.output.is_closed() {
            Err(Failure::Output)
        } else {
            Ok(Some((first, error)))
        };
    }
    let mut pending = Some(first);
    let (sender, input) = mpsc::channel(2);
    let (output, mut receive) = mpsc::channel(4);
    let (stop, cancel) = watch::channel(false);
    let provider = remote(transport::Channels {
        input,
        output,
        cancel,
    });
    tokio::pin!(provider);
    let (mut sender, mut ready, mut offset, mut closed, mut stopped, mut receiving) =
        (Some(sender), false, Some(0), false, false, true);
    let mut response: Option<Response<P>> = None;
    let mut repair: Option<Repair> = None;
    let mut terminal = None;
    let mut opening = None;
    let result = async { loop {
        if let Err(error) = local::check(active, &channels.cancel) { break Err(error); }
        let now = start.elapsed();
        if response.as_ref().is_some_and(|r| !r.output.route_verified()) {
            let invalid = machine.handle(machine.generation(), turn::Event::InputUnavailable, now);
            if let Err(error) = actions(&channels.output, invalid) { break Err(error); }
        }
        let tick = machine.handle(machine.generation(), turn::Event::Tick, now);
        if let Err(error) = actions(&channels.output, tick) { break Err(error); }
        if machine.phase() == turn::Phase::Idle && !closed && let Some(listen) = machine.start(now) {
            emit(&channels.output, Event::Action(listen))?;
        }
        if let Some(current) = &mut response {
            if stopped && receive.is_empty() && current.settled && current.frozen.is_some() && !current.truncated {
                current.local_only = true;
                current.truncated = true;
            }
            let cursor = current.frozen.unwrap_or_else(|| current.output.cursor());
            if matches!(cursor.phase, OutputPhase::Failed | OutputPhase::Stopped) && current.frozen.is_none() {
                break Err(Failure::Output);
            }
            if !current.first_played && let Some(at) = cursor.first_played {
                current.first_played = true;
                let event = turn::Event::FirstAudioPlayed { at:at.saturating_duration_since(start) };
                let out = machine.handle(current.generation, event, now);
                if let Err(error) = actions(&channels.output, out) { break Err(error); }
            }
            let done = current.settled && (if current.frozen.is_some() { current.truncated } else { cursor.phase == OutputPhase::Finished });
            if done {
                let Some(text) = current.transcript.take() else { break Err(Failure::Protocol); };
                let interrupted = current.frozen.is_some();
                let segments = [heard::Segment { text:&text.text, start_frame:0, end_frame:text.frames }];
                let mut heard = match heard::split(&segments, cursor.played_frames, text.frames, interrupted) {
                    Ok(heard) => heard,
                    Err(_) => break Err(Failure::Protocol),
                };
                let history = heard.history_text();
                repair = heard.take_nudge().map(|unheard| Repair { heard:history.clone(), unheard });
                if let Err(error) = emit(&channels.output, Event::History(Record {
                    identity:text.identity, text:history, interrupted:heard.interrupted, played_frames:cursor.played_frames,
                    total_frames:text.frames, server_synchronized: !current.local_only,
                })) { break Err(error); }
                if !interrupted {
                    let out = machine.handle(current.generation, turn::Event::PlaybackDrained, now);
                    if let Err(error) = actions(&channels.output, out) { break Err(error); }
                }
                response = None;
            }
        }
        if response.is_none() && pending.is_some() && ready && !stopped {
            match open(Arc::default()) {
                Ok(output) => { opening = None; response = Some(Response {
                    generation:machine.generation(), identity:None, output, transcript:None, settled:false,
                    first_played:false, frozen:None, truncated:false, local_only:false,
                    committed:false, tool_attempts:std::collections::BTreeSet::new(),
                }); },
                Err(Failure::Busy) if opening.get_or_insert_with(std::time::Instant::now).elapsed() < Duration::from_secs(3) => {},
                Err(error) => break Err(error),
            }
        }
        if stopped && receive.is_empty() && response.as_ref().is_none_or(|r| !r.settled || r.frozen.is_some()) {
            break terminal.take().unwrap_or(Err(Failure::Transport)).and_then(|()| {
                if pending.is_none() && response.is_none() { Ok(()) } else { Err(Failure::Transport) }
            });
        }
        if closed && pending.is_none() && response.is_none() { sender.take(); }
        let upload = ready && offset.is_some() && response.as_ref().is_some_and(|r|
            r.frozen.is_none() && r.generation == machine.generation() && r.output.cursor().phase == OutputPhase::Playing);
        tokio::select! { biased;
            _ = channels.cancel.changed() => {},
            _ = poll.tick() => {},
            event = receive.recv(), if receiving => {
                match event {
                    Some(transport::Event::Ready) => {
                        if response.as_ref().is_some_and(|r| r.frozen.is_some() && !r.truncated) { continue; }
                        if ready { break Err(Failure::Protocol); }
                        ready = true; emit(&channels.output, Event::Ready(machine.generation()))?;
                    },
                    Some(transport::Event::Audio { identity, pcm }) => {
                        let Some(current) = &mut response else { break Err(Failure::Protocol); };
                        if current.frozen.is_none() && (!current.committed || offset.is_some()) { break Err(Failure::Protocol); }
                        if current.tool_attempts.contains(&identity.attempt) || current.identity.as_ref().is_some_and(|known| *known != identity) { break Err(Failure::Protocol); }
                        current.identity = Some(identity);
                        if current.frozen.is_none() {
                            pcm.chunks(24_000).try_for_each(|chunk| current.output.push(chunk))?;
                        }
                    },
                    Some(transport::Event::Transcript(text)) => {
                        let Some(current) = &mut response else { break Err(Failure::Protocol); };
                        if current.identity.as_ref() != Some(&text.identity) || current.transcript.is_some() { break Err(Failure::Protocol); }
                        current.transcript = Some(text);
                    },
                    Some(transport::Event::ToolStep(receipt)) => {
                        let Some(current) = &mut response else { break Err(Failure::Protocol); };
                        if !current.committed || offset.is_some() || current.identity.is_some() || current.transcript.is_some()
                            || current.settled || current.frozen.is_some() || !receipt.ok || !receipt.within_bounds
                            || !current.tool_attempts.insert(receipt.attempt.clone()) || current.tool_attempts.len()>vults_chat::tools::MAX_STEPS {
                            break Err(Failure::Protocol);
                        }
                        // A consumed tool turn cannot return as local recovery or execute again after failure.
                        pending = None;
                        emit(&channels.output, Event::Settled(receipt))?;
                    },
                    Some(transport::Event::Finished(receipt)) => {
                        let Some(current) = &mut response else { break Err(Failure::Protocol); };
                        if current.settled || !receipt.within_bounds || current.tool_attempts.contains(&receipt.attempt) || current.identity.as_ref().is_none_or(|i| i.attempt != receipt.attempt)
                            || current.transcript.is_none() { break Err(Failure::Protocol); }
                        if !receipt.ok && current.frozen.is_none() { break Err(Failure::Response); }
                        current.settled = true;
                        if current.frozen.is_none() {
                            if offset.is_some() { break Err(Failure::Protocol); }
                            pending = None;
                            if let Err(error) = current.output.finish() { break Err(error); }
                            let out = machine.handle(current.generation, turn::Event::ResponseFinished, now);
                            if let Err(error) = actions(&channels.output, out) { break Err(error); }
                        }
                        emit(&channels.output, Event::Settled(receipt))?;
                    },
                    Some(transport::Event::Truncated(identity)) => {
                        let Some(current) = &mut response else { break Err(Failure::Protocol); };
                        if current.identity.as_ref() != Some(&identity) || current.frozen.is_none() || current.truncated { break Err(Failure::Protocol); }
                        current.truncated = true;
                    },
                    None => receiving = false,
                }
            },
            result = &mut provider, if !stopped => { stopped = true; terminal = Some(result); },
            command = channels.input.recv(), if !closed && (!stopped || response.is_some()) => {
                match command {
                    Some(Command::Turn { generation, turn }) => {
                        if stopped { continue; }
                        if generation != machine.generation() { continue; }
                        if pending.is_some() || machine.phase() != turn::Phase::Listening { break Err(Failure::Input); }
                        let prepared = prepare_turn(prepare(turn.clone()), active, &channels).await;
                        local::check(active, &channels.cancel)?;
                        match prepared {
                            Ok(Some(reply)) => {
                                emit(&channels.output, Event::Local { generation, reply })?;
                                let out = machine.handle(generation, turn::Event::LocalCompleted, start.elapsed());
                                actions(&channels.output, out)?;
                                continue;
                            },
                            Ok(None) => {},
                            Err(reason) => { pending = Some(turn); break Err(reason); },
                        }
                        pending = Some(turn); offset = Some(0);
                        let out = machine.handle(generation, turn::Event::InputCommitted, start.elapsed());
                        if let Err(error) = actions(&channels.output, out) { break Err(error); }
                    },
                    Some(Command::Signal { generation, signal }) => {
                        if response.as_ref().is_none_or(|r| !r.output.route_verified()) {
                            let invalid = machine.handle(machine.generation(), turn::Event::InputUnavailable, now);
                            if let Err(error) = actions(&channels.output, invalid) { break Err(error); }
                            continue;
                        }
                        let out = machine.handle(generation, signal.event(), now);
                        let interrupt = match actions(&channels.output, out) { Ok(interrupt) => interrupt, Err(error) => break Err(error) };
                        if interrupt && let Some(current) = &mut response {
                            let Some(identity) = current.identity.clone() else { break Err(Failure::Protocol); };
                            if current.frozen.is_some() { continue; }
                            let mut cursor = current.output.stop();
                            // The provider truncates at integer milliseconds; repair uses that same floor.
                            cursor.played_frames = cursor.played_frames / 24 * 24;
                            current.frozen = Some(cursor);
                            ready = false;
                            pending = None; offset = None;
                            if stopped {
                                current.local_only = true;
                                current.truncated = true;
                            } else if sender.as_ref().ok_or(Failure::Transport)?.try_send(transport::Command::Interrupt { identity, played_frames:cursor.played_frames }).is_err() { break Err(Failure::Output); }
                        }
                    },
                    None => closed = true,
                }
            },
            permit = async {
                sender.as_ref().ok_or(Failure::Transport)?.clone().reserve_owned().await.map_err(|_| Failure::Transport)
            }, if upload && !stopped => {
                let permit = match permit { Ok(permit) => permit, Err(error) => break Err(error) };
                let Some(turn) = &pending else { break Err(Failure::Input); };
                if let Some(hint) = repair.take() { permit.send(transport::Command::Repair(hint)); continue; }
                let at = offset.unwrap_or(0);
                if at < turn.samples().len() {
                    let samples = turn.samples();
                    let end = (at + 2400).min(samples.len());
                    permit.send(transport::Command::Audio(samples[at..end].to_vec())); offset = Some(end);
                } else {
                    permit.send(transport::Command::Commit); offset = None; ready = false;
                    response.as_mut().ok_or(Failure::Protocol)?.committed = true;
                }
            },
        }
    } }.await;
    if let Some(current) = response.take() {
        current.output.stop();
    }
    drop(sender);
    let _ = stop.send(true);
    if !stopped {
        let _ = tokio::time::timeout(Duration::from_secs(11), &mut provider).await;
    }
    local::check(active, &channels.cancel)?;
    if channels.output.is_closed() {
        return Err(Failure::Output);
    }
    match (result, pending) {
        (Err(Failure::Cancelled), _) => Err(Failure::Cancelled),
        (Err(reason), Some(turn)) => Ok(Some((turn, reason))),
        (Err(error), None) => Err(error),
        (Ok(()), _) => Ok(None),
    }
}

#[cfg(test)]
mod tests;
