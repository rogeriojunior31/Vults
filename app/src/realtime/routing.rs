//! Local commands are consumed by the state owner before any realtime upload.
use super::{Lease, Turn, playback::LocalReply};
use crate::{
    settings::Settings,
    voice_commands::{Commands, Feedback, Report},
};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tauri::{AppHandle, Manager};
use tokio::sync::oneshot;
use vults_chat::realtime::transport::Failure;
use vults_dictation::command::{self, Parsed};

pub(super) struct Router {
    commands: Mutex<Commands>,
    valid: Arc<dyn Fn(&Settings) -> bool + Send + Sync>,
    _lease: Arc<Lease>,
}
impl Router {
    pub fn new(lease: Arc<Lease>, valid: Arc<dyn Fn(&Settings) -> bool + Send + Sync>) -> Arc<Self> {
        Arc::new(Self {
            commands: Mutex::new(Commands::new(Instant::now())),
            valid,
            _lease: lease,
        })
    }
    pub async fn prepare(self: Arc<Self>, app: AppHandle, turn: Turn) -> Result<Option<LocalReply>, Failure> {
        let text = turn.transcript().ok_or(Failure::Input)?.to_owned();
        let (reply, receive) = oneshot::channel();
        let pending = Pending(Arc::new(Mutex::new(true)));
        crate::runtime::realtime_command(
            &app,
            Request {
                router: self,
                text,
                pending: pending.0.clone(),
                deadline: Instant::now() + super::playback::LOCAL_TIMEOUT,
                reply,
            },
        )?;
        let report = receive.await.map_err(|_| Failure::Cancelled)??;
        drop(pending);
        Ok(report.map(|r| LocalReply {
            feedback: r.feedback,
            text: r.text,
            choices: r.choices,
        }))
    }
}
// Dropping a timed-out/aborted native waiter revokes an unconsumed runtime message.
struct Pending(Arc<Mutex<bool>>);
impl Drop for Pending {
    fn drop(&mut self) {
        *self.0.lock().unwrap_or_else(|e| e.into_inner()) = false;
    }
}
type Reply = oneshot::Sender<Result<Option<Report>, Failure>>;
pub(crate) struct Request {
    router: Arc<Router>,
    text: String,
    pending: Arc<Mutex<bool>>,
    deadline: Instant,
    reply: Reply,
}
impl std::fmt::Debug for Request {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RealtimeCommand")
            .field("bytes", &self.text.len())
            .finish_non_exhaustive()
    }
}
impl Request {
    /// Settings and admission stay locked until the shadow-state batch commits.
    pub fn dispatch(self, app: &AppHandle, state: &mut vults_core::State) -> Option<(Reply, Report)> {
        let settings = app.state::<crate::settings::SettingsState>();
        let saved = settings.0.lock().ok()?;
        let mut commands = self.router.commands.lock().ok()?;
        let usage = serde_json::to_value(crate::usage::usage(app.clone())).unwrap_or_default();
        let report = consume(
            &self.pending,
            self.deadline,
            || !self.reply.is_closed() && (self.router.valid)(&saved) && saved.voice_realtime.enabled(),
            || {
                route(
                    &mut commands,
                    state,
                    &self.text,
                    saved.dictation_commands,
                    Instant::now(),
                    usage,
                )
            },
        )?;
        match report {
            Some(report) => Some((self.reply, report)),
            None => {
                let _ = self.reply.send(Ok(None));
                None
            }
        }
    }
}
fn consume<T>(
    pending: &Mutex<bool>,
    deadline: Instant,
    valid: impl FnOnce() -> bool,
    run: impl FnOnce() -> T,
) -> Option<T> {
    let mut pending = pending.lock().ok()?;
    if !*pending || Instant::now() >= deadline || !valid() {
        return None;
    }
    *pending = false;
    Some(run())
}
fn route(
    commands: &mut Commands,
    state: &mut vults_core::State,
    text: &str,
    enabled: bool,
    now: Instant,
    usage: serde_json::Value,
) -> Option<Report> {
    let mut report = if enabled {
        commands.execute(state, text, &[], usage, now)
    } else {
        commands.cancel();
        let feedback = match command::parse(text) {
            Parsed::NotCommand => return None,
            Parsed::Refused => Feedback::NotByVoice,
            Parsed::Commands(_) => Feedback::Unavailable,
        };
        Report {
            feedback,
            text: vults_core::i18n::voice_feedback(state.lang, feedback).into(),
            choices: Vec::new(),
            effects: Vec::new(),
        }
    };
    if report.chat_fallback(true) {
        return None;
    }
    // Launching/saving an effect is only a request, not proof its external work completed.
    if !report.effects.is_empty() && report.feedback == Feedback::Done {
        report.feedback = Feedback::Requested;
        report.text = vults_core::i18n::voice_feedback(state.lang, Feedback::Requested).into();
    }
    Some(report)
}

#[cfg(test)]
mod tests;
