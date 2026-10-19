//! Typed MCP questions; owned by the runtime, separate from permission and hook replies.
use serde::Serialize;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::oneshot;
use vults_core::questions::{Owner, Revocation, Slot, Ticket};
use vults_peer::requester::Requester;

mod capture;
pub(crate) mod install;
pub(crate) mod transport;
pub(crate) use capture::{admit_ordinary, cancel as cancel_capture, invalidate as invalidate_capture, key};
#[derive(Debug, Default)]
pub(crate) struct Active {
    live: std::sync::atomic::AtomicBool,
    external: std::sync::atomic::AtomicBool,
    access_changed: std::sync::Mutex<Option<Instant>>,
    route: std::sync::Mutex<capture::Routing>,
}
pub(crate) fn active(app: &AppHandle) -> bool {
    app.try_state::<Active>()
        .is_some_and(|state| state.live.load(std::sync::atomic::Ordering::Acquire))
}
const LEASE: Duration = Duration::from_secs(3);
const UNAVAILABLE: &str = "The question is no longer available. Ask a new question; do not retry this reply.";

type Answer = Result<String, String>;
#[derive(Debug)]
pub(crate) enum Command {
    Open {
        connection: u64,
        proof: Requester,
        request: String,
        question: String,
        received: Instant,
        reply: oneshot::Sender<Answer>,
    },
    Disconnect(u64),
    Status {
        surface: String,
        token: Option<String>,
        reply: oneshot::Sender<View>,
    },
    Act {
        surface: String,
        token: String,
        action: String,
        text: String,
        capture: Option<u64>,
        reply: oneshot::Sender<Result<(), String>>,
    },
    Depart(String),
    Leave {
        surface: String,
        reply: oneshot::Sender<View>,
    },
    VoiceStart(capture::Press),
    VoiceStop(String),
    VoiceResult {
        token: String,
        id: u64,
        result: Option<Result<String, String>>,
    },
    AccessChanged(oneshot::Sender<()>),
    Tick,
    Audited(u64, bool),
    Delivered(u64, bool),
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct View {
    seq: u64,
    surface: String,
    phase: String,
    offer: Option<Offer>,
}
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct Offer {
    token: String,
    question: String,
    draft: String,
    client_pid: u32,
    seconds: u64,
    dictation: Option<Dictation>,
}
#[derive(Clone, Debug, PartialEq, Serialize)]
struct Dictation {
    id: u64,
    phase: String,
    text: Option<String>,
}
struct Recording {
    press: capture::Press,
    id: u64,
    expires: Instant,
}
impl Drop for Recording {
    fn drop(&mut self) {
        self.press.cancel();
    }
}
struct Pending {
    connection: u64,
    proof: Requester,
    slot: Slot,
    ticket: Ticket,
    reply: Option<oneshot::Sender<Answer>>,
    expires: Instant,
    touched: Instant,
    presented: bool,
    external: bool,
}
struct Delivery {
    pending: Pending,
    text: String,
    action: oneshot::Sender<Result<(), String>>,
    sent: bool,
}
pub(crate) type AuditWork = (u64, Vec<vults_core::questions::Receipt>);
#[derive(Default)]
pub(crate) struct Questions {
    pending: Option<Pending>,
    delivery: Option<Delivery>,
    view: View,
    published: Option<View>,
    records: Vec<vults_core::audit::Audit>,
    recording: Option<Recording>,
    start_recording: bool,
    next_recording: u64,
    external_enabled: bool,
    external_changed: Option<Instant>,
}
impl Questions {
    pub(crate) fn allow_external(&mut self, on: bool, changed: Option<Instant>) {
        let revoked = !on || self.external_changed != changed;
        self.external_enabled = on;
        self.external_changed = changed;
        if revoked
            && self
                .pending
                .as_ref()
                .or_else(|| self.delivery.as_ref().map(|d| &d.pending))
                .is_some_and(|p| p.external)
        {
            self.finish(Revocation::Cancelled, "cancelled");
        }
    }

    fn finish(&mut self, reason: Revocation, phase: &str) {
        self.stop_recording();
        let consumed = self.delivery.is_some();
        if let Some(delivery) = self.delivery.take() {
            self.records.push(
                vults_core::questions::Receipt {
                    owner: Owner {
                        relay_pid: delivery.pending.proof.relay().pid,
                        relay_started: delivery.pending.proof.relay().started,
                        client_pid: delivery.pending.proof.client().pid,
                        client_started: delivery.pending.proof.client().started,
                    },
                    ticket: delivery.pending.ticket,
                    outcome: vults_core::questions::Outcome::Revoked(reason),
                }
                .audit(),
            );
            let _ = delivery.action.send(Err(UNAVAILABLE.into()));
            self.pending = Some(delivery.pending);
        }
        if let Some(mut pending) = self.pending.take() {
            pending.slot.revoke(reason);
            for record in pending.slot.take_records() {
                self.records.push(record.audit());
            }
            if let Some(reply) = pending.reply {
                let _ = reply.send(Err(UNAVAILABLE.into()));
            }
            self.view.offer = None;
            self.view.phase = if consumed { "failed" } else { phase }.into();
            self.view.seq += 1;
        }
    }
    fn stop_recording(&mut self) {
        self.recording = None;
        self.start_recording = false;
        if let Some(offer) = &mut self.view.offer
            && offer.dictation.take().is_some()
        {
            self.view.seq += 1;
        }
    }
    fn check(&mut self, locked: bool, visible: bool, now: Instant) {
        if self
            .recording
            .as_ref()
            .is_some_and(|r| r.press.cancelled.load(std::sync::atomic::Ordering::Acquire))
        {
            self.stop_recording();
        }
        let pending = self
            .pending
            .as_mut()
            .or_else(|| self.delivery.as_mut().map(|d| &mut d.pending));
        let Some(pending) = pending else { return };
        let ended = if locked {
            Some((Revocation::Locked, "cancelled"))
        } else if !visible || now.saturating_duration_since(pending.touched) >= LEASE {
            Some((Revocation::Hidden, "cancelled"))
        } else if pending.reply.as_ref().is_some_and(oneshot::Sender::is_closed)
            || !pending.proof.is_current()
        {
            Some((Revocation::Disconnected, "unavailable"))
        } else if now >= pending.expires {
            Some((Revocation::Expired, "expired"))
        } else {
            None
        };
        if let Some((reason, phase)) = ended {
            self.finish(reason, phase);
        }
    }
    pub(crate) fn surface(&self) -> &str {
        &self.view.surface
    }
    pub(crate) fn active(&self) -> bool {
        self.pending.is_some() || self.delivery.is_some()
    }

    pub(crate) fn handle(
        &mut self,
        command: Command,
        surface: &str,
        locked: bool,
        visible: bool,
    ) -> Option<AuditWork> {
        let now = Instant::now();
        self.check(locked, visible, now);
        match command {
            Command::Open {
                connection,
                proof,
                request,
                question,
                received,
                reply,
            } => {
                let external = !proof.launched_by(std::process::id());
                if locked
                    || surface.is_empty()
                    || (external
                        && (!self.external_enabled || self.external_changed.is_some_and(|at| received < at)))
                {
                    let _ = reply.send(Err(
                        "External text questions are off. Enable them in Dictation > Agents before asking."
                            .into(),
                    ));
                    return None;
                }
                let mut nonce = [0; 16];
                if std::fs::File::open("/dev/urandom")
                    .and_then(|mut f| std::io::Read::read_exact(&mut f, &mut nonce))
                    .is_err()
                {
                    let _ = reply.send(Err(UNAVAILABLE.into()));
                    return None;
                }
                let owner = Owner {
                    relay_pid: proof.relay().pid,
                    relay_started: proof.relay().started,
                    client_pid: proof.client().pid,
                    client_started: proof.client().started,
                };
                let Ok(mut slot) = Slot::new(owner, nonce) else {
                    let _ = reply.send(Err(UNAVAILABLE.into()));
                    return None;
                };
                let Ok(offer) = slot.open(&request, &question, received) else {
                    let _ = reply.send(Err("Give a nonempty question of at most 4096 bytes.".into()));
                    return None;
                };
                if reply.is_closed() || now >= offer.expires || !proof.is_current() {
                    let _ = reply.send(Err(UNAVAILABLE.into()));
                    return None;
                }
                self.finish(Revocation::Replaced, "cancelled");
                self.view = View {
                    seq: self.view.seq + 1,
                    surface: surface.into(),
                    phase: "pending".into(),
                    offer: Some(Offer {
                        token: token(offer.ticket),
                        question: offer.question,
                        draft: offer.draft,
                        client_pid: owner.client_pid,
                        seconds: offer.expires.saturating_duration_since(now).as_secs(),
                        dictation: None,
                    }),
                };
                self.pending = Some(Pending {
                    connection,
                    proof,
                    slot,
                    ticket: offer.ticket,
                    reply: Some(reply),
                    expires: offer.expires,
                    touched: now,
                    presented: false,
                    external,
                });
            }
            Command::Disconnect(connection) => {
                if self
                    .pending
                    .as_ref()
                    .or_else(|| self.delivery.as_ref().map(|d| &d.pending))
                    .is_some_and(|p| p.connection == connection)
                {
                    self.finish(Revocation::Disconnected, "unavailable");
                }
            }
            Command::Depart(surface) => {
                if surface == self.view.surface {
                    self.finish(Revocation::Hidden, "cancelled");
                }
            }
            Command::Leave { surface, reply } => {
                if surface == self.view.surface {
                    self.finish(Revocation::Hidden, "cancelled");
                }
                let _ = reply.send(View {
                    seq: self.view.seq,
                    ..View::default()
                });
            }
            Command::Status {
                surface,
                token,
                reply,
            } => {
                if surface == self.view.surface {
                    if let Some(delivery) = &mut self.delivery {
                        delivery.pending.touched = now;
                    }
                    if let Some(pending) = &mut self.pending {
                        if token.as_deref() == self.view.offer.as_ref().map(|o| o.token.as_str()) {
                            pending.touched = now;
                            pending.presented = true;
                        }
                        if let (Some(offer), Some(current)) = (&mut self.view.offer, pending.slot.view(now)) {
                            offer.seconds = current.expires.saturating_duration_since(now).as_secs();
                        }
                    }
                    let _ = reply.send(self.view.clone());
                } else {
                    let _ = reply.send(View {
                        seq: self.view.seq,
                        ..View::default()
                    });
                }
            }
            Command::Act {
                surface,
                token,
                action,
                text,
                capture,
                reply,
            } => {
                let valid = surface == self.view.surface
                    && self.view.offer.as_ref().is_some_and(|o| o.token == token)
                    && self.pending.as_ref().is_some_and(|p| {
                        p.reply.as_ref().is_some_and(|reply| !reply.is_closed()) && p.proof.is_current()
                    });
                if !valid {
                    let _ = reply.send(Err(UNAVAILABLE.into()));
                    return None;
                }
                if action == "cancel" {
                    self.finish(Revocation::Cancelled, "cancelled");
                    let _ = reply.send(Ok(()));
                    return None;
                }
                if action == "stop-dictation" {
                    self.stop_recording();
                    let _ = reply.send(Ok(()));
                    return None;
                }
                if action == "send" && self.recording.is_some() {
                    let _ = reply.send(Err("Review the transcription before sending.".into()));
                    return None;
                }
                let apply = action == "apply-dictation";
                if apply && !self.recording.as_ref().is_some_and(|r| Some(r.id) == capture)
                    || apply
                        && !self
                            .view
                            .offer
                            .as_ref()
                            .and_then(|o| o.dictation.as_ref())
                            .is_some_and(|d| d.phase == "ready")
                {
                    let _ = reply.send(Err(UNAVAILABLE.into()));
                    return None;
                }
                if !matches!(action.as_str(), "edit" | "send" | "apply-dictation") {
                    let _ = reply.send(Err(UNAVAILABLE.into()));
                    return None;
                }
                let pending = self.pending.as_mut().expect("validated pending question");
                if pending.slot.edit(pending.ticket, &text, now).is_err() {
                    let _ = reply.send(Err(
                        "Keep the reply within 2000 bytes and remove control characters.".into(),
                    ));
                    return None;
                }
                if action == "edit" || apply {
                    if let Some(offer) = &mut self.view.offer {
                        offer.draft = text;
                    }
                    pending.touched = now;
                    if apply {
                        self.stop_recording();
                    }
                    let _ = reply.send(Ok(()));
                    return None;
                }
                let Ok(answer) = pending.slot.confirm(pending.ticket, now) else {
                    let _ = reply.send(Err("Write a reply before sending.".into()));
                    return None;
                };
                // Consumption precedes persistence/delivery; neither a timeout nor a lost ack retries text.
                let mut pending = self.pending.take().expect("confirmed pending question");
                let records = pending.slot.take_records();
                pending.touched = now;
                let connection = pending.connection;
                self.delivery = Some(Delivery {
                    pending,
                    text: answer.text,
                    action: reply,
                    sent: false,
                });
                self.view.offer = None;
                self.view.phase = "sending".into();
                self.view.seq += 1;
                return Some((connection, records));
            }
            Command::Audited(connection, kept) => {
                if !self
                    .delivery
                    .as_ref()
                    .is_some_and(|d| d.pending.connection == connection && !d.sent)
                {
                    return None;
                }
                let delivery = self.delivery.as_mut().expect("matching audited question");
                let live = delivery.pending.proof.is_current()
                    && delivery
                        .pending
                        .reply
                        .as_ref()
                        .is_some_and(|reply| !reply.is_closed());
                if !kept || !live {
                    self.finish(Revocation::Disconnected, "failed");
                    return None;
                }
                let sender = delivery
                    .pending
                    .reply
                    .take()
                    .expect("undelivered audited question");
                if sender.send(Ok(std::mem::take(&mut delivery.text))).is_err() {
                    self.finish(Revocation::Disconnected, "failed");
                } else {
                    delivery.sent = true;
                }
            }
            Command::Delivered(connection, sent) => {
                if !self
                    .delivery
                    .as_ref()
                    .is_some_and(|d| d.pending.connection == connection && d.sent)
                {
                    return None;
                }
                if !sent {
                    self.finish(Revocation::Disconnected, "failed");
                    return None;
                }
                let delivery = self.delivery.take().expect("matching delivered question");
                self.view.phase = if sent { "sent" } else { "failed" }.into();
                self.view.seq += 1;
                let _ = delivery
                    .action
                    .send(if sent { Ok(()) } else { Err(UNAVAILABLE.into()) });
            }
            Command::VoiceStart(press) => {
                let valid = self.recording.is_none()
                    && !press.cancelled.load(std::sync::atomic::Ordering::Acquire)
                    && self.view.offer.as_ref().is_some_and(|o| o.token == press.token)
                    && self.pending.as_ref().is_some_and(|p| p.presented);
                if valid {
                    self.next_recording += 1;
                    let id = self.next_recording;
                    self.recording = Some(Recording {
                        press,
                        id,
                        expires: self.pending.as_ref().expect("validated pending question").expires,
                    });
                    self.start_recording = true;
                    self.view.seq += 1;
                    if let Some(offer) = &mut self.view.offer {
                        offer.dictation = Some(Dictation {
                            id,
                            phase: "listening".into(),
                            text: None,
                        });
                    }
                } else {
                    press.cancel();
                }
            }
            Command::VoiceStop(token) => {
                if self.view.offer.as_ref().is_some_and(|o| o.token == token) {
                    self.stop_recording();
                }
            }
            Command::VoiceResult { token, id, result } => {
                if self
                    .recording
                    .as_ref()
                    .is_some_and(|r| r.id == id && r.press.token == token)
                    && let Some(offer) = &mut self.view.offer
                {
                    if offer.dictation.as_ref().is_some_and(|d| d.phase == "ready") {
                        return None;
                    }
                    self.view.seq += 1;
                    match result {
                        None => {
                            offer.dictation = Some(Dictation {
                                id,
                                phase: "transcribing".into(),
                                text: None,
                            })
                        }
                        Some(Ok(text))
                            if !text.trim().is_empty()
                                && text.len() <= vults_core::MAX_ANSWER_LEN
                                && !text.chars().any(|c| c.is_control() && c != '\n' && c != '\t') =>
                        {
                            offer.dictation = Some(Dictation {
                                id,
                                phase: "ready".into(),
                                text: Some(text),
                            });
                        }
                        Some(_) => {
                            self.stop_recording();
                            if let Some(offer) = &mut self.view.offer {
                                offer.dictation = Some(Dictation {
                                    id,
                                    phase: "failed".into(),
                                    text: None,
                                });
                            }
                        }
                    }
                }
            }
            Command::AccessChanged(reply) => {
                let _ = reply.send(());
            }
            Command::Tick => {}
        }
        None
    }
}
fn token(ticket: Ticket) -> String {
    ticket.binding.0.iter().map(|b| format!("{b:02x}")).collect()
}
pub(crate) async fn persist(records: Vec<vults_core::questions::Receipt>) -> bool {
    persist_at(crate::history::dir(), records).await
}
async fn persist_at(dir: std::path::PathBuf, records: Vec<vults_core::questions::Receipt>) -> bool {
    let write = tauri::async_runtime::spawn_blocking(move || {
        let mut store = vults_store::Store::open_current(&dir)?;
        for record in records {
            store.append_audit(crate::history::unix_now(), &record.audit())?;
        }
        Ok::<_, vults_store::Error>(())
    });
    matches!(
        tokio::time::timeout(Duration::from_secs(2), write).await,
        Ok(Ok(Ok(())))
    )
}

pub(crate) fn publish(app: &AppHandle, questions: &mut Questions) {
    for record in std::mem::take(&mut questions.records) {
        crate::audit::keep(record);
    }
    let live = questions.active();
    if let Some(state) = app.try_state::<Active>() {
        // Fence native shortcuts before cancelling the ordinary capture they might race with.
        if let Ok(mut route) = state.route.lock() {
            if live {
                route
                    .ordinary_stop
                    .store(true, std::sync::atomic::Ordering::Release);
            } else if route.blocking {
                route.ordinary_stop = std::sync::Arc::default();
            }
            route.blocking = live;
            // A native key cannot select a question before its renderer acknowledges displaying it.
            route.token = questions
                .pending
                .as_ref()
                .filter(|p| p.presented)
                .and(questions.view.offer.as_ref())
                .map(|o| o.token.clone());
            route.current = questions.recording.as_ref().map(|r| r.press.clone());
        }
        state.external.store(
            questions
                .pending
                .as_ref()
                .or_else(|| questions.delivery.as_ref().map(|d| &d.pending))
                .is_some_and(|p| p.external),
            std::sync::atomic::Ordering::Release,
        );
        let was = state.live.swap(live, std::sync::atomic::Ordering::AcqRel);
        if live && !was {
            crate::voice::cancel(app);
            crate::dictation::cancel(app, false);
        }
    }
    if questions.start_recording {
        questions.start_recording = false;
        if let Some(recording) = &questions.recording {
            capture::start(
                app.clone(),
                recording.press.clone(),
                recording.id,
                recording.expires,
            );
        }
    }
    if questions.published.as_ref() != Some(&questions.view) {
        let _ = app.emit("text-question", &questions.view);
        questions.published = Some(questions.view.clone());
    }
}
pub(crate) fn depart(app: &AppHandle, surface: &str) {
    invalidate_capture(app);
    crate::runtime::question_try(app, Command::Depart(surface.into()));
}
pub(crate) fn external_enabled(app: &AppHandle) -> bool {
    app.state::<crate::settings::SettingsState>()
        .0
        .lock()
        .is_ok_and(|s| s.question_agents)
}
fn with_consent<T>(
    settings: &std::sync::Mutex<crate::settings::Settings>,
    changed: &std::sync::Mutex<Option<Instant>>,
    transition: impl FnOnce(bool, Option<Instant>) -> T,
) -> T {
    let Ok(saved) = settings.lock() else {
        return transition(false, None);
    };
    let Ok(changed) = changed.lock() else {
        return transition(false, None);
    };
    // A reply handoff and revoking consent must have one order. No native or model IO runs here.
    transition(saved.question_agents, *changed)
}
pub(crate) fn with_access<T>(app: &AppHandle, transition: impl FnOnce(bool, Option<Instant>) -> T) -> T {
    let settings = app.state::<crate::settings::SettingsState>();
    let active = app.state::<Active>();
    with_consent(&settings.0, &active.access_changed, transition)
}

#[tauri::command]
pub(crate) async fn question_agents_set(
    app: AppHandle,
    window: tauri::Webview,
    on: bool,
) -> Result<(), String> {
    crate::settings_page(&window)?;
    {
        let state = app.state::<crate::settings::SettingsState>();
        let mut saved = state.0.lock().map_err(|_| "Settings are busy.")?;
        let mut next = saved.clone();
        next.question_agents = on;
        let state = app.state::<Active>();
        let mut changed = state
            .access_changed
            .lock()
            .map_err(|_| "Question access is busy.")?;
        crate::settings::save(&next).map_err(|_| "Could not save agent question access.")?;
        *saved = next;
        *changed = Some(Instant::now());
    }
    if app
        .state::<Active>()
        .external
        .load(std::sync::atomic::Ordering::Acquire)
    {
        invalidate_capture(&app);
    }
    install::clear(&app);
    let (reply, rx) = oneshot::channel();
    // The saved choice remains authoritative even if the runtime is already closing.
    if crate::runtime::question(&app, Command::AccessChanged(reply))
        .await
        .is_ok()
    {
        let _ = rx.await;
    }
    let _ = app.emit(
        "settings",
        serde_json::json!({"questionAgents": external_enabled(&app)}),
    );
    Ok(())
}

fn page(window: &tauri::Webview) -> Result<String, String> {
    match window.label() {
        crate::ISLAND | crate::NEST => Ok(window.label().into()),
        _ => Err("Text questions belong to the island or Nest.".into()),
    }
}
#[tauri::command]
pub(crate) async fn text_question_status(
    window: tauri::Webview,
    token: Option<String>,
) -> Result<View, String> {
    let surface = page(&window)?;
    let (reply, rx) = oneshot::channel();
    crate::runtime::question(
        window.app_handle(),
        Command::Status {
            surface,
            token,
            reply,
        },
    )
    .await?;
    rx.await.map_err(|_| UNAVAILABLE.into())
}
#[tauri::command]
pub(crate) async fn text_question_action(
    window: tauri::Webview,
    token: String,
    action: String,
    text: String,
    capture: Option<u64>,
) -> Result<(), String> {
    let surface = page(&window)?;
    if token.len() != 64 || text.len() > vults_core::MAX_ANSWER_LEN {
        return Err("Keep the reply within 2000 bytes and remove control characters.".into());
    }
    let (reply, rx) = oneshot::channel();
    crate::runtime::question(
        window.app_handle(),
        Command::Act {
            surface,
            token,
            action,
            text,
            capture,
            reply,
        },
    )
    .await?;
    rx.await.map_err(|_| UNAVAILABLE.to_string())?
}
#[tauri::command]
pub(crate) async fn text_question_depart(window: tauri::Webview) -> Result<View, String> {
    let surface = page(&window)?;
    let (reply, rx) = oneshot::channel();
    crate::runtime::question(window.app_handle(), Command::Leave { surface, reply }).await?;
    rx.await.map_err(|_| UNAVAILABLE.into())
}

#[cfg(test)]
mod tests;
