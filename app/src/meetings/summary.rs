//! Page-owned, explicitly requested notes. Failures never mutate the local transcript.
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::{AppHandle, Manager};
use tokio::sync::watch;
use vults_agents::driver::BoxFuture;
use vults_chat::{
    filter::{self, Reply},
    meeting_summary::{Request, Summary},
    providers::Provider,
};
use vults_store::traces::Span;

const TIMEOUT: Duration = Duration::from_secs(120);

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    id: u64,
    phase: &'static str,
    error: Option<&'static str>,
    notes: Option<Summary>,
}

#[derive(Debug)]
pub(super) struct Slot {
    attempt: u64,
    stop: watch::Sender<bool>,
    phase: &'static str,
    error: Option<&'static str>,
    notes: Option<Summary>,
}
impl Default for Slot {
    fn default() -> Self {
        Self {
            attempt: 0,
            stop: watch::channel(false).0,
            phase: "idle",
            error: None,
            notes: None,
        }
    }
}
impl Slot {
    #[cfg(target_os = "linux")]
    pub(super) fn archive_notes(&self) -> Result<Option<Summary>, String> {
        if self.phase == "generating" {
            return Err("Finish or cancel summary generation before saving this meeting.".into());
        }
        Ok(self.notes.clone())
    }

    fn snapshot(&self, id: u64) -> Snapshot {
        Snapshot {
            id,
            phase: self.phase,
            error: self.error,
            notes: self.notes.clone(),
        }
    }
    fn cancel(&mut self) {
        self.stop.send_replace(true);
        self.attempt = self.attempt.wrapping_add(1);
        if self.phase == "generating" {
            self.phase = "cancelled";
            self.error = None;
        }
    }
    pub(super) fn discard(&mut self) {
        self.cancel();
        self.phase = "idle";
        self.error = None;
        self.notes = None;
    }
    fn begin(&mut self) -> Result<(u64, watch::Sender<bool>), String> {
        if self.phase == "generating" {
            return Err("A meeting summary is already running.".into());
        }
        self.discard();
        self.stop = watch::channel(false).0;
        self.phase = "generating";
        Ok((self.attempt, self.stop.clone()))
    }
    fn complete(&mut self, attempt: u64, result: Result<Summary, &'static str>) {
        if self.attempt != attempt || self.phase != "generating" || *self.stop.borrow() {
            return;
        }
        match result {
            Ok(notes) => {
                self.notes = Some(notes);
                self.phase = "finished";
            }
            Err(error) => {
                self.error = (error != "cancelled").then_some(error);
                self.phase = if error == "cancelled" {
                    "cancelled"
                } else {
                    "failed"
                };
            }
        }
    }
}

pub(crate) fn invalidate<R: tauri::Runtime>(app: &tauri::AppHandle<R>) {
    if let Some(state) = app.try_state::<Arc<super::State>>()
        && let Ok(mut slot) = state.slot.lock()
    {
        slot.summary.cancel();
    }
}

#[derive(Clone)]
struct Choice {
    provider: &'static Provider,
    model: String,
    ledger: String,
}
impl Choice {
    fn matches(&self, settings: &crate::settings::Settings) -> bool {
        Self::read(settings).is_some_and(|choice| {
            choice.provider.id == self.provider.id
                && choice.model == self.model
                && choice.ledger == self.ledger
        })
    }
    fn read(settings: &crate::settings::Settings) -> Option<Self> {
        if !crate::gate::switched_on(settings, crate::gate::MEETING_SUMMARY) {
            return None;
        }
        let provider = vults_chat::providers::find(&settings.api_provider)?;
        let model = settings
            .api_models
            .get(provider.id)
            .map(String::as_str)
            .unwrap_or(provider.default_model);
        if !vults_chat::valid_model(model) || filter::endpoint(provider).is_err() {
            return None;
        }
        Some(Self {
            provider,
            model: model.into(),
            ledger: settings.meeting_summary_budget.clone()?,
        })
    }
}

#[tauri::command]
pub fn meeting_summary_status<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    window: tauri::Webview<R>,
    id: u64,
) -> Result<Snapshot, String> {
    super::authorized(window.label())?;
    if crate::lock::locked(&app) {
        return Err("Unlock the screen before reading meeting notes.".into());
    }
    let state = app.state::<Arc<super::State>>();
    let mut slot = state.slot.lock().map_err(|_| "Meeting notes are unavailable.")?;
    if slot.view.id != id {
        return Err("This meeting is no longer available.".into());
    }
    slot.touched = Instant::now();
    Ok(slot.summary.snapshot(id))
}

#[tauri::command]
pub fn meeting_summary_cancel<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    window: tauri::Webview<R>,
    id: u64,
) -> Result<(), String> {
    super::authorized(window.label())?;
    let state = app.state::<Arc<super::State>>();
    let mut slot = state.slot.lock().map_err(|_| "Meeting notes are unavailable.")?;
    if slot.view.id == id {
        slot.summary.cancel();
    }
    Ok(())
}

#[tauri::command]
pub fn meeting_summary_begin(app: AppHandle, window: tauri::Webview, id: u64) -> Result<Snapshot, String> {
    super::authorized(window.label())?;
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (app, id);
        Err("Meeting summaries are unavailable on this platform.".into())
    }
    #[cfg(target_os = "linux")]
    {
        if crate::lock::locked(&app) {
            return Err("Unlock the screen before summarizing a meeting.".into());
        }
        let choice = app
            .state::<crate::settings::SettingsState>()
            .0
            .lock()
            .ok()
            .and_then(|s| Choice::read(&s))
            .ok_or("Enable meeting summaries and choose an API chat model first.")?;
        let state = app.state::<Arc<super::State>>();
        let (request, attempt, stop, view) = {
            let mut slot = state.slot.lock().map_err(|_| "Meeting notes are unavailable.")?;
            let transcript = slot
                .transcript
                .as_ref()
                .filter(|t| t.id == id && slot.view.id == id && slot.view.phase == "finished")
                .ok_or("Finish a meeting transcription first.")?;
            let request = Request::build(
                &transcript.microphone.summary_source(),
                &transcript.system.summary_source(),
            )
            .map_err(|_| "This transcript cannot be summarized; the local text is still available.")?;
            let (attempt, stop) = slot.summary.begin()?;
            slot.touched = Instant::now();
            (request, attempt, stop, slot.summary.snapshot(id))
        };
        let net = Native {
            app: app.clone(),
            id,
            attempt,
            stop,
            choice,
        };
        tauri::async_runtime::spawn(async move {
            let result = run(&net, &request).await;
            publish(&net.app, id, attempt, &net.stop, &net.choice, result);
        });
        Ok(view)
    }
}

fn publish<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    id: u64,
    attempt: u64,
    stop: &watch::Sender<bool>,
    choice: &Choice,
    result: Result<Summary, &'static str>,
) {
    let state = app.state::<Arc<super::State>>();
    if let Ok(mut slot) = state.slot.lock()
        && slot.view.id == id
        && slot.view.phase == "finished"
    {
        // Slot -> settings matches recording's lock order; consent stays stable through publication.
        let settings = app.state::<crate::settings::SettingsState>();
        let saved = settings.0.lock();
        let current = saved.as_ref().is_ok_and(|s| choice.matches(s))
            && !*stop.borrow()
            && !crate::lock::locked(app)
            && !state.shutdown.load(std::sync::atomic::Ordering::SeqCst)
            && slot.touched.elapsed() <= super::LEASE;
        slot.summary
            .complete(attempt, if current { result } else { Err("cancelled") });
    }
}

trait Net: Send + Sync {
    type Prepared: Send + Sync;
    fn current(&self) -> bool;
    fn cancelled(&self) -> BoxFuture<'_, ()>;
    fn prepare(&self) -> BoxFuture<'_, Result<Self::Prepared, ()>>;
    fn reserve(&self) -> BoxFuture<'_, Option<Span>>;
    fn call<'a>(
        &'a self,
        prepared: &'a Self::Prepared,
        request: &'a Request,
    ) -> BoxFuture<'a, Result<Reply, filter::Error>>;
    fn done(&self, span: Span, outcome: &str, usage: Option<&Reply>, took: Duration);
}
struct Attempt<'a, N: Net> {
    net: &'a N,
    span: Option<Span>,
    started: Instant,
    outcome: &'static str,
    reply: Option<Reply>,
}
impl<N: Net> Drop for Attempt<'_, N> {
    fn drop(&mut self) {
        if let Some(span) = self.span.take() {
            self.net
                .done(span, self.outcome, self.reply.as_ref(), self.started.elapsed());
        }
    }
}
async fn run(net: &impl Net, request: &Request) -> Result<Summary, &'static str> {
    if !net.current() {
        return Err("cancelled");
    }
    let deadline = tokio::time::Instant::now() + TIMEOUT;
    let prepared = tokio::select! { biased;
        _=net.cancelled()=>return Err("cancelled"),
        result=tokio::time::timeout_at(deadline, net.prepare())=>result.map_err(|_| "timeout")?.map_err(|_| "configuration")?,
    };
    if !net.current() {
        return Err("cancelled");
    }
    let span = tokio::select! { biased;
        _=net.cancelled()=>return Err("cancelled"),
        result=tokio::time::timeout_at(deadline, net.reserve())=>result.map_err(|_| "timeout")?.ok_or("quota")?,
    };
    let mut attempt = Attempt {
        net,
        span: Some(span),
        started: Instant::now(),
        outcome: "cancelled",
        reply: None,
    };
    if !net.current() {
        return Err("cancelled");
    }
    let result = tokio::select! { biased;
        _=net.cancelled()=>return Err("cancelled"),
        _=tokio::time::sleep_until(deadline)=>Err("timeout"),
        reply=net.call(&prepared, request)=>reply.map_err(|_| "provider"),
    };
    if !net.current() {
        return Err("cancelled");
    }
    let result = result.and_then(|reply| {
        let notes = request.validate(&reply.text).map_err(|_| "invalid");
        attempt.reply = Some(reply);
        notes
    });
    attempt.outcome = result.as_ref().map_or_else(|error| *error, |_| "generated");
    result
}

struct Native {
    app: AppHandle,
    id: u64,
    attempt: u64,
    stop: watch::Sender<bool>,
    choice: Choice,
}

fn finish_span(span: &mut Span, outcome: &str, usage: Option<&Reply>, took: Duration) {
    span.end_ms = span
        .start_ms
        .saturating_add(i64::try_from(took.as_millis()).unwrap_or(i64::MAX));
    span.status = if outcome == "generated" { "ok" } else { "error" }.into();
    span.attrs["vults.outcome"] = serde_json::json!(outcome);
    if let Some(reply) = usage {
        if let Some(tokens) = reply.input_tokens {
            span.attrs["gen_ai.usage.input_tokens"] = serde_json::json!(tokens);
        }
        if let Some(tokens) = reply.output_tokens {
            span.attrs["gen_ai.usage.output_tokens"] = serde_json::json!(tokens);
        }
    }
}

impl Net for Native {
    type Prepared = Option<String>;
    fn current(&self) -> bool {
        if *self.stop.borrow() || crate::lock::locked(&self.app) {
            return false;
        }
        let current = self
            .app
            .state::<crate::settings::SettingsState>()
            .0
            .lock()
            .ok()
            .is_some_and(|s| self.choice.matches(&s));
        if !current {
            return false;
        }
        let state = self.app.state::<Arc<super::State>>();
        state.slot.lock().is_ok_and(|slot| {
            !state.shutdown.load(std::sync::atomic::Ordering::SeqCst)
                && slot.view.id == self.id
                && slot.view.phase == "finished"
                && slot.summary.attempt == self.attempt
                && slot.touched.elapsed() <= super::LEASE
        })
    }
    fn cancelled(&self) -> BoxFuture<'_, ()> {
        let mut receiver = self.stop.subscribe();
        Box::pin(async move {
            if !*receiver.borrow_and_update() {
                let _ = receiver.changed().await;
            }
        })
    }
    fn prepare(&self) -> BoxFuture<'_, Result<Self::Prepared, ()>> {
        let provider = self.choice.provider;
        Box::pin(async move {
            if provider.local {
                return Ok(None);
            }
            tauri::async_runtime::spawn_blocking(move || {
                vults_secrets::get(provider.secret())
                    .ok()
                    .flatten()
                    .filter(|key| provider.accepts_key(key))
                    .map(Some)
                    .ok_or(())
            })
            .await
            .map_err(|_| ())?
        })
    }
    fn reserve(&self) -> BoxFuture<'_, Option<Span>> {
        let (app, choice) = (self.app.clone(), self.choice.clone());
        Box::pin(async move {
            let reserved = tauri::async_runtime::spawn_blocking(move || {
                let directory = crate::history::dir();
                let mut store = vults_store::Store::open_current(&directory).ok()?;
                let span = crate::gate::shared(&app).reserve_meeting_summary(
                    &mut store,
                    &choice.ledger,
                    choice.provider.id,
                    &choice.model,
                    choice.provider.local,
                )?;
                Some(crate::gate::Reservation::new(span, directory))
            })
            .await
            .ok()
            .flatten()?;
            reserved.take()
        })
    }
    fn call<'a>(
        &'a self,
        prepared: &'a Self::Prepared,
        request: &'a Request,
    ) -> BoxFuture<'a, Result<Reply, filter::Error>> {
        Box::pin(async move {
            filter::complete(
                self.choice.provider,
                prepared.as_deref(),
                &self.choice.model,
                &request.messages(),
                TIMEOUT,
            )
            .await
        })
    }
    fn done(&self, mut span: Span, outcome: &str, usage: Option<&Reply>, took: Duration) {
        finish_span(&mut span, outcome, usage, took);
        crate::writer::send(crate::writer::Job::FinishRequest(span));
    }
}

#[cfg(test)]
mod tests;
