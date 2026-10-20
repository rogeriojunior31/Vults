//! Fixed-origin, bounded PCM transport. Every connection and turn first needs app-owned admission.
use super::{MAX_EVENT_BYTES, Plan, Session, Settlement, State};
use base64::{Engine, engine::general_purpose::STANDARD};
use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use std::{future::Future, time::Duration};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    sync::{mpsc, watch},
    time::{Instant, sleep_until, timeout},
};
use tokio_tungstenite::{
    WebSocketStream,
    tungstenite::{Message, client::IntoClientRequest, protocol::WebSocketConfig},
};
use vults_agents::driver::BoxFuture;

const IO_TIMEOUT: Duration = Duration::from_secs(10);
const TURN_TIMEOUT: Duration = Duration::from_secs(120);
const INPUT_FRAMES: usize = 24_000 * 30;
const OUTPUT_FRAMES: usize = 24_000 * 120;
const CHUNK_FRAMES: usize = 2400;

/// The app binds consent, revocation, the keyring and durable accounting to one connection owner.
pub trait Gate: Send + Sync {
    fn active(&self) -> bool;
    fn now_ms(&self) -> i64;
    fn reserve(&self, ceiling_microusd: u64) -> BoxFuture<'_, Option<Admission>>;
    fn key(&self) -> BoxFuture<'_, Option<String>>;
    fn finish<'a>(&'a self, attempt: &'a str, actual_microusd: Option<u64>, ok: bool) -> BoxFuture<'a, bool>;
}

pub struct Admission {
    attempt: String,
    abandoned: Option<Box<dyn FnOnce() + Send>>,
}
impl std::fmt::Debug for Admission {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Admission")
            .field("attempt", &self.attempt)
            .finish_non_exhaustive()
    }
}
impl Admission {
    pub fn new(attempt: String, abandoned: impl FnOnce() + Send + 'static) -> Self {
        Self {
            attempt,
            abandoned: Some(Box::new(abandoned)),
        }
    }
    /// Transfer responsibility to a transport that will settle the exact attempt.
    pub fn claim(mut self) -> String {
        self.abandoned.take();
        std::mem::take(&mut self.attempt)
    }
}
impl Drop for Admission {
    fn drop(&mut self) {
        if let Some(abandoned) = self.abandoned.take() {
            abandoned();
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Failure {
    Off,
    Budget,
    Key,
    Transport,
    Protocol,
    Timeout,
    Cancelled,
    Input,
    Output,
    Ledger,
    Busy,
}

pub enum Command {
    Audio(Vec<i16>),
    Commit,
    Interrupt,
}
impl std::fmt::Debug for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Audio(pcm) => f.debug_tuple("AudioFrames").field(&pcm.len()).finish(),
            Self::Commit => f.write_str("Commit"),
            Self::Interrupt => f.write_str("Interrupt"),
        }
    }
}

pub enum Event {
    Ready,
    Audio { attempt: String, pcm: Vec<i16> },
    Finished(Settlement),
}
impl std::fmt::Debug for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ready => f.write_str("Ready"),
            Self::Audio { attempt, pcm } => f
                .debug_struct("Audio")
                .field("attempt", attempt)
                .field("frames", &pcm.len())
                .finish(),
            Self::Finished(receipt) => f.debug_tuple("Finished").field(receipt).finish(),
        }
    }
}

#[derive(Debug)]
pub struct Channels {
    pub input: mpsc::Receiver<Command>,
    pub output: mpsc::Sender<Event>,
    pub cancel: watch::Receiver<bool>,
}

pub trait Provider: Send + Sync {
    fn run<'a>(
        &'a self,
        plan: Plan,
        gate: &'a dyn Gate,
        channels: Channels,
    ) -> BoxFuture<'a, Result<(), Failure>>;
}

#[derive(Debug)]
pub struct OpenAi;
impl Provider for OpenAi {
    fn run<'a>(
        &'a self,
        plan: Plan,
        gate: &'a dyn Gate,
        channels: Channels,
    ) -> BoxFuture<'a, Result<(), Failure>> {
        Box::pin(start(plan, gate, channels, |model, key| async move {
            let mut request = format!("wss://api.openai.com/v1/realtime?model={model}")
                .into_client_request()
                .map_err(|_| Failure::Transport)?;
            let mut header = format!("Bearer {key}")
                .parse::<tokio_tungstenite::tungstenite::http::HeaderValue>()
                .map_err(|_| Failure::Key)?;
            header.set_sensitive(true);
            request.headers_mut().insert("Authorization", header);
            let config = WebSocketConfig::default()
                .read_buffer_size(16 * 1024)
                .write_buffer_size(0)
                .max_write_buffer_size(MAX_EVENT_BYTES * 2)
                .max_message_size(Some(MAX_EVENT_BYTES))
                .max_frame_size(Some(MAX_EVENT_BYTES));
            tokio_tungstenite::connect_async_with_config(request, Some(config), false)
                .await
                .map(|(socket, _)| socket)
                .map_err(|_| Failure::Transport)
        }))
    }
}

async fn guarded<T>(
    gate: &dyn Gate,
    cancel: &mut watch::Receiver<bool>,
    future: impl Future<Output = T>,
    deadline: Instant,
) -> Result<T, Failure> {
    tokio::pin!(future);
    let mut poll = tokio::time::interval(Duration::from_millis(25));
    loop {
        alive(gate, cancel)?;
        tokio::select! { biased;
            _ = cancel.changed() => {},
            _ = sleep_until(deadline) => return Err(Failure::Timeout),
            _ = poll.tick() => {},
            value = &mut future => return Ok(value),
        }
    }
}

fn alive(gate: &dyn Gate, cancel: &watch::Receiver<bool>) -> Result<(), Failure> {
    if !gate.active() {
        return Err(Failure::Off);
    }
    if *cancel.borrow() || cancel.has_changed().is_err() {
        return Err(Failure::Cancelled);
    }
    Ok(())
}

async fn settle(
    gate: &dyn Gate,
    attempt: &str,
    receipt: Option<&Settlement>,
    billable: bool,
) -> Result<(), Failure> {
    let (actual, ok) = receipt.map_or((if billable { None } else { Some(0) }, false), |r| {
        (r.actual_microusd, r.ok)
    });
    if timeout(IO_TIMEOUT, gate.finish(attempt, actual, ok))
        .await
        .unwrap_or(false)
    {
        Ok(())
    } else {
        Err(Failure::Ledger)
    }
}

async fn start<S, F, Fut>(
    plan: Plan,
    gate: &dyn Gate,
    mut channels: Channels,
    connect: F,
) -> Result<(), Failure>
where
    S: AsyncRead + AsyncWrite + Unpin + Send,
    F: FnOnce(&'static str, String) -> Fut,
    Fut: Future<Output = Result<WebSocketStream<S>, Failure>>,
{
    let ceiling = plan.ceiling_microusd(gate.now_ms()).ok_or(Failure::Budget)?;
    let attempt = guarded(
        gate,
        &mut channels.cancel,
        gate.reserve(ceiling),
        Instant::now() + IO_TIMEOUT,
    )
    .await?
    .ok_or(Failure::Budget)?
    .claim();
    let before_connect = async {
        let key = guarded(
            gate,
            &mut channels.cancel,
            gate.key(),
            Instant::now() + IO_TIMEOUT,
        )
        .await?
        .ok_or(Failure::Key)?;
        guarded(
            gate,
            &mut channels.cancel,
            connect(plan.model(), key),
            Instant::now() + IO_TIMEOUT,
        )
        .await?
    }
    .await;
    let mut socket = match before_connect {
        Ok(socket) => socket,
        Err(error) => {
            settle(gate, &attempt, None, false).await?;
            return Err(error);
        }
    };
    let mut pending = Some(attempt);
    let mut billable = false;
    let result = drive(
        &mut socket,
        plan,
        gate,
        &mut channels,
        &mut pending,
        &mut billable,
    )
    .await;
    // Drop the socket before resolving uncertain in-flight work. A task abort leaves its hold open.
    drop(socket);
    if let Some(attempt) = pending {
        settle(gate, &attempt, None, billable).await?;
    }
    result
}

async fn send<S: AsyncRead + AsyncWrite + Unpin>(
    socket: &mut WebSocketStream<S>,
    gate: &dyn Gate,
    cancel: &mut watch::Receiver<bool>,
    value: Value,
    deadline: Instant,
) -> Result<(), Failure> {
    guarded(
        gate,
        cancel,
        socket.send(Message::Text(value.to_string().into())),
        deadline.min(Instant::now() + IO_TIMEOUT),
    )
    .await?
    .map_err(|_| Failure::Transport)
}

fn emit(output: &mpsc::Sender<Event>, event: Event) -> Result<(), Failure> {
    output.try_send(event).map_err(|_| Failure::Output)
}

async fn drive<S: AsyncRead + AsyncWrite + Unpin>(
    socket: &mut WebSocketStream<S>,
    plan: Plan,
    gate: &dyn Gate,
    channels: &mut Channels,
    pending: &mut Option<String>,
    billable: &mut bool,
) -> Result<(), Failure> {
    let mut session = Session::new(plan);
    let mut deadline = Instant::now() + IO_TIMEOUT;
    send(
        socket,
        gate,
        &mut channels.cancel,
        session.plan.session_update(),
        deadline,
    )
    .await?;
    while !matches!(session.state, State::Ready) {
        let message = guarded(gate, &mut channels.cancel, socket.next(), deadline)
            .await?
            .ok_or(Failure::Transport)?
            .map_err(|_| Failure::Transport)?;
        if let Message::Text(text) = message {
            if serde_json::from_str::<Value>(&text)
                .ok()
                .and_then(|v| v["type"].as_str().map(str::to_owned))
                .is_some_and(|t| t.starts_with("response."))
            {
                *billable = true;
            }
            session.receive(text.as_bytes()).map_err(|_| Failure::Protocol)?;
        } else if matches!(message, Message::Ping(_) | Message::Pong(_)) {
            guarded(gate, &mut channels.cancel, socket.flush(), deadline)
                .await?
                .map_err(|_| Failure::Transport)?;
        } else {
            return Err(Failure::Protocol);
        }
    }
    alive(gate, &channels.cancel)?;
    emit(&channels.output, Event::Ready)?;
    let (mut input_frames, mut output_frames, mut committed) = (0_usize, 0_usize, false);
    deadline = Instant::now() + TURN_TIMEOUT;
    let mut prepared = None;
    let mut poll = tokio::time::interval(Duration::from_millis(25));
    loop {
        alive(gate, &channels.cancel)?;
        tokio::select! { biased;
            _ = channels.cancel.changed() => {},
            _ = sleep_until(deadline) => return Err(Failure::Timeout),
            _ = poll.tick() => {},
            command = channels.input.recv() => {
                match command {
                    None => return Ok(()),
                    Some(Command::Interrupt) => {
                        if !committed { return Err(Failure::Cancelled); }
                        send(socket, gate, &mut channels.cancel, json!({"type":"response.cancel"}), deadline).await?;
                        deadline = deadline.min(Instant::now()+IO_TIMEOUT);
                    }
                    Some(Command::Audio(pcm)) => {
                        if committed || pcm.is_empty() || pcm.len() > 24_000 || input_frames.saturating_add(pcm.len()) > INPUT_FRAMES { return Err(Failure::Input); }
                        if prepared.is_none() {
                            if pending.is_none() {
                                let ceiling = session.plan.ceiling_microusd(gate.now_ms()).ok_or(Failure::Budget)?;
                                *pending = Some(guarded(gate, &mut channels.cancel, gate.reserve(ceiling), Instant::now()+IO_TIMEOUT).await?.ok_or(Failure::Budget)?.claim());
                            }
                            prepared = Some(session.prepare(pending.as_deref().ok_or(Failure::Ledger)?, gate.now_ms()).map_err(|_| Failure::Protocol)?);
                        }
                        input_frames += pcm.len();
                        for chunk in pcm.chunks(CHUNK_FRAMES) {
                            let bytes: Vec<u8> = chunk.iter().flat_map(|s| s.to_le_bytes()).collect();
                            send(socket, gate, &mut channels.cancel, json!({"type":"input_audio_buffer.append","audio":STANDARD.encode(bytes)}), deadline).await?;
                        }
                    }
                    Some(Command::Commit) => {
                        if committed || input_frames < 2400 { return Err(Failure::Input); }
                        send(socket, gate, &mut channels.cancel, json!({"type":"input_audio_buffer.commit"}), deadline).await?;
                        // A partial send may already have started inference, so never refund it as zero.
                        *billable = true;
                        send(socket, gate, &mut channels.cancel, prepared.take().ok_or(Failure::Input)?.event, deadline).await?;
                        committed = true;
                    }
                }
            }
            message = socket.next() => {
                let message = message.ok_or(Failure::Transport)?.map_err(|_| Failure::Transport)?;
                match message {
                    Message::Ping(_) | Message::Pong(_) => {},
                    Message::Close(_) => return Err(Failure::Transport),
                    Message::Text(text) => {
                        let event: Value = serde_json::from_str(&text).map_err(|_| Failure::Protocol)?;
                        if event["type"].as_str().is_some_and(|t| t.starts_with("response.")) { *billable = true; }
                        let receipt = session.receive(text.as_bytes()).map_err(|_| Failure::Protocol)?;
                        if event["type"] == "response.output_audio.delta" {
                            let encoded = event["delta"].as_str().filter(|s| s.len()<=128*1024).ok_or(Failure::Protocol)?;
                            let bytes = STANDARD.decode(encoded).map_err(|_| Failure::Protocol)?;
                            if bytes.is_empty() || bytes.len()%2 != 0 || output_frames.saturating_add(bytes.len()/2)>OUTPUT_FRAMES { return Err(Failure::Output); }
                            output_frames += bytes.len()/2;
                            let pcm = bytes.as_chunks::<2>().0.iter().copied().map(i16::from_le_bytes).collect();
                            alive(gate,&channels.cancel)?;
                            emit(&channels.output, Event::Audio { attempt: pending.clone().ok_or(Failure::Ledger)?, pcm })?;
                        }
                        if let Some(receipt) = receipt {
                            if pending.as_deref() != Some(receipt.attempt.as_str()) { return Err(Failure::Ledger); }
                            // The final receipt owns settlement even if its durable write is delayed.
                            // Never race it with a weaker unknown-cost fallback on timeout.
                            *pending = None;
                            settle(gate, &receipt.attempt, Some(&receipt), true).await?;
                            *billable = false;
                            let valid = receipt.within_bounds;
                            alive(gate,&channels.cancel)?;
                            emit(&channels.output, Event::Finished(receipt))?;
                            if !valid { return Err(Failure::Protocol); }
                            input_frames=0; output_frames=0; committed=false;
                            deadline = Instant::now()+TURN_TIMEOUT;
                            emit(&channels.output, Event::Ready)?;
                        }
                    }
                    _ => return Err(Failure::Protocol),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests;
