//! Typed MCP questions; owned by the runtime, separate from permission and hook replies.
use serde::Serialize;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::oneshot;
use vults_core::questions::{Owner, Revocation, Slot, Ticket};
use vults_peer::requester::Requester;

pub(crate) mod transport;
#[derive(Debug, Default)]
pub(crate) struct Active(std::sync::atomic::AtomicBool);
pub(crate) fn active(app: &AppHandle) -> bool {
    app.try_state::<Active>()
        .is_some_and(|state| state.0.load(std::sync::atomic::Ordering::Acquire))
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
        reply: oneshot::Sender<Result<(), String>>,
    },
    Depart(String),
    Leave {
        surface: String,
        reply: oneshot::Sender<View>,
    },
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
}
struct Pending {
    connection: u64,
    proof: Requester,
    slot: Slot,
    ticket: Ticket,
    reply: Option<oneshot::Sender<Answer>>,
    expires: Instant,
    touched: Instant,
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
}
impl Questions {
    fn finish(&mut self, reason: Revocation, phase: &str) {
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
    fn check(&mut self, locked: bool, visible: bool, now: Instant) {
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
                if locked || surface.is_empty() || !proof.launched_by(std::process::id()) {
                    let _ = reply.send(Err(
                        "Text questions are available only to a live agent started by Vults.".into(),
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
                if !matches!(action.as_str(), "edit" | "send") {
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
                if action == "edit" {
                    if let Some(offer) = &mut self.view.offer {
                        offer.draft = text;
                    }
                    pending.touched = now;
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
        let was = state.0.swap(live, std::sync::atomic::Ordering::AcqRel);
        if live && !was {
            crate::voice::cancel(app);
            crate::dictation::cancel(app, false);
        }
    }
    if questions.published.as_ref() != Some(&questions.view) {
        let _ = app.emit("text-question", &questions.view);
        questions.published = Some(questions.view.clone());
    }
}
pub(crate) fn depart(app: &AppHandle, surface: &str) {
    crate::runtime::question_try(app, Command::Depart(surface.into()));
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
