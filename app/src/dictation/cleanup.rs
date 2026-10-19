//! Opted-in text filtering. Cancellation and failures retain the complete local result.
use super::worker::Ticket;
use std::{
    collections::BTreeMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
use tauri::{AppHandle, Manager};
use tokio::sync::watch;
use vults_agents::driver::BoxFuture;
use vults_chat::{
    filter::{self, Reply},
    providers::Provider,
};
use vults_dictation::{
    cleanup::{Options, Request, Style},
    text::Processed,
};
use vults_store::traces::Span;

pub(crate) struct State {
    changed: watch::Sender<u64>,
    styles: Mutex<Option<BTreeMap<String, Style>>>,
    focus: Mutex<Option<Arc<vults_platform::focus::Tracker>>>,
    loading: AtomicBool,
    closing: AtomicBool,
}
impl Default for State {
    fn default() -> Self {
        Self {
            changed: watch::channel(0).0,
            styles: Mutex::new(None),
            focus: Mutex::new(None),
            loading: AtomicBool::new(false),
            closing: AtomicBool::new(false),
        }
    }
}
impl State {
    pub(crate) fn reload_styles(&self, store: &vults_store::Store) {
        if let Ok(mut cached) = self.styles.lock() {
            *cached = store.dictation_styles().ok().and_then(|rows| {
                rows.into_iter()
                    .map(|(app, style)| Some((app, Style::parse(&style)?)))
                    .collect()
            });
        }
    }
    fn revision(&self) -> u64 {
        *self.changed.borrow()
    }
}

pub(crate) fn invalidate<R: tauri::Runtime>(app: &tauri::AppHandle<R>) {
    if let Some(state) = app.try_state::<State>() {
        state.changed.send_modify(|n| *n = n.wrapping_add(1));
    }
}

pub(super) fn start(app: &AppHandle) {
    let state = State::default();
    if let Ok(store) = vults_store::Store::open(&crate::history::dir()) {
        state.reload_styles(&store);
    }
    app.manage(state);
    ensure_focus(app);
}

pub(crate) fn ensure_focus<R: tauri::Runtime>(app: &tauri::AppHandle<R>) {
    let Some(state) = app.try_state::<State>() else {
        return;
    };
    let on = app
        .state::<crate::settings::SettingsState>()
        .0
        .lock()
        .is_ok_and(|s| s.dictation_cleanup && s.dictation_cleanup_budget.is_some());
    if !on || state.closing.load(Ordering::SeqCst) || !vults_platform::insert::wayland_available() {
        return;
    }
    if state.focus.lock().is_ok_and(|f| f.is_some()) || state.loading.swap(true, Ordering::SeqCst) {
        return;
    }
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let tracker = vults_platform::focus::Tracker::start().await.map(Arc::new);
        let state = app.state::<State>();
        let close = {
            match state.focus.lock() {
                Ok(mut focus) if !state.closing.load(Ordering::SeqCst) => {
                    *focus = tracker;
                    None
                }
                _ => tracker,
            }
        };
        if let Some(tracker) = close {
            tracker.close().await;
        }
        state.loading.store(false, Ordering::SeqCst);
    });
}

pub(crate) fn shutdown(app: &AppHandle) {
    let tracker = app.try_state::<State>().and_then(|s| {
        s.closing.store(true, Ordering::SeqCst);
        s.focus.lock().ok()?.take()
    });
    if let Some(tracker) = tracker {
        tauri::async_runtime::block_on(tracker.close());
    }
}

#[derive(Clone, Debug)]
pub(super) struct Choice {
    provider: &'static Provider,
    model: String,
    revision: u64,
    budget: String,
    app: String,
    resolved: vults_dictation::styles::Resolved,
}
impl Choice {
    fn options(&self, spellings: &[String]) -> Options {
        Options {
            style: self.resolved.style,
            write_in: self.resolved.write_in,
            app: self.app.clone(),
            spellings: spellings.to_vec(),
            ..Options::default()
        }
    }
}

pub(super) fn snapshot(
    app: &AppHandle,
    s: &crate::settings::Settings,
    target: Option<&vults_platform::focus::Window>,
    pinned_session: bool,
) -> Option<Choice> {
    if !s.dictation_cleanup {
        return None;
    }
    let provider = vults_chat::providers::find(&s.api_provider)?;
    let model = s
        .api_models
        .get(provider.id)
        .map(String::as_str)
        .unwrap_or(provider.default_model);
    if !vults_chat::valid_model(model) {
        return None;
    }
    let state = app.try_state::<State>()?;
    let window = target
        .cloned()
        .or_else(|| state.focus.lock().ok()?.as_ref()?.snapshot())
        .or_else(vults_platform::focus::key_down);
    let prefs = vults_dictation::styles::Preferences {
        default: s.dictation_style,
        write_in: s.dictation_write_in,
        sessions_english: s.dictation_sessions_english,
        apps: state.styles.lock().ok()?.clone()?,
    };
    let session = pinned_session
        || window.as_ref().filter(|w| w.is_terminal()).is_some_and(|w| {
            crate::runtime::dictation_sessions(app)
                .recv_timeout(Duration::from_millis(50))
                .ok()
                .is_some_and(|sessions| vults_platform::focus::session(w, &sessions).is_some())
        });
    let resolved = vults_dictation::styles::resolve(
        true,
        window.as_ref().map(|w| w.class.as_str()),
        window.as_ref().is_some_and(|w| w.is_terminal()),
        session,
        &prefs,
    );
    if resolved.style == Style::Verbatim {
        return None;
    }
    Some(Choice {
        app: window.map(|w| w.class).unwrap_or_default(),
        resolved,
        provider,
        model: model.into(),
        revision: app.try_state::<State>()?.revision(),
        budget: s.dictation_cleanup_budget.clone()?,
    })
}

fn timeout(options: &Options) -> Duration {
    Duration::from_secs(if options.style == Style::Prompt {
        12
    } else if options.write_in.is_some() {
        8
    } else {
        4
    })
}

trait Net: Send + Sync {
    type Prepared: Send + Sync;
    fn current(&self) -> bool;
    fn changed(&self) -> BoxFuture<'_, ()>;
    fn prepare(&self) -> BoxFuture<'_, Result<Self::Prepared, ()>>;
    fn reserve(&self) -> BoxFuture<'_, Option<Span>>;
    fn call<'a>(
        &'a self,
        prepared: &'a Self::Prepared,
        request: &'a Request,
    ) -> BoxFuture<'a, Result<Reply, filter::Error>>;
    fn done(&self, span: Span, outcome: &str, usage: Option<(Option<u64>, Option<u64>)>, took: Duration);
}
async fn cancelled(net: &impl Net) {
    loop {
        if !net.current() {
            return;
        }
        tokio::select! { _=net.changed()=>return, _=tokio::time::sleep(Duration::from_millis(20))=>{} }
    }
}

struct Attempt<'a, N: Net> {
    net: &'a N,
    span: Option<Span>,
    started: Instant,
    outcome: &'static str,
    usage: Option<(Option<u64>, Option<u64>)>,
}
impl<N: Net> Drop for Attempt<'_, N> {
    fn drop(&mut self) {
        if let Some(span) = self.span.take() {
            self.net
                .done(span, self.outcome, self.usage, self.started.elapsed());
        }
    }
}

async fn run(net: &impl Net, request: &Request, options: &Options) -> Option<String> {
    if !net.current() {
        return None;
    }
    let deadline = tokio::time::Instant::now() + timeout(options);
    let prepared = tokio::select! {biased;
        _=cancelled(net)=>return None,
        result=tokio::time::timeout_at(deadline,net.prepare())=>result.ok()?.ok()?,
    };
    if !net.current() {
        return None;
    }
    let span = tokio::select! {biased;
        _=cancelled(net)=>return None,
        result=tokio::time::timeout_at(deadline,net.reserve())=>result.ok()??,
    };
    let mut attempt = Attempt {
        net,
        span: Some(span),
        started: Instant::now(),
        outcome: "cancelled",
        usage: None,
    };
    if !net.current() {
        return None;
    }
    let reply = tokio::select! {biased;
        _=cancelled(net)=>return None,
        _=tokio::time::sleep_until(deadline)=>{attempt.outcome="timeout";return None;},
        result=net.call(&prepared,request)=>match result {Ok(reply)=>reply,Err(_)=>{attempt.outcome="provider-error";return None;}},
    };
    attempt.usage = Some((reply.input_tokens, reply.output_tokens));
    if !net.current() {
        return None;
    }
    match request.validate(&reply.text) {
        Ok(text) => {
            attempt.outcome = "applied";
            Some(text)
        }
        Err(_) => {
            attempt.outcome = "invalid";
            None
        }
    }
}

struct Native {
    app: AppHandle,
    ticket: Ticket,
    choice: Choice,
}
impl Net for Native {
    type Prepared = Option<String>;
    fn current(&self) -> bool {
        let settings = self.app.state::<crate::settings::SettingsState>();
        let Ok(s) = settings.0.lock() else {
            return false;
        };
        s.dictation_cleanup
            && s.dictation_cleanup_budget.as_ref() == Some(&self.choice.budget)
            && s.api_provider == self.choice.provider.id
            && s.api_models
                .get(self.choice.provider.id)
                .map(String::as_str)
                .unwrap_or(self.choice.provider.default_model)
                == self.choice.model
            && self.app.state::<State>().revision() == self.choice.revision
            && !crate::lock::locked(&self.app)
            && self.ticket.valid()
            && !self.ticket.is_command()
    }
    fn changed(&self) -> BoxFuture<'_, ()> {
        let mut receiver = self.app.state::<State>().changed.subscribe();
        Box::pin(async move {
            let _ = receiver.changed().await;
        })
    }
    fn prepare(&self) -> BoxFuture<'_, Result<Self::Prepared, ()>> {
        let provider = self.choice.provider;
        Box::pin(async move {
            filter::endpoint(provider).map_err(|_| ())?;
            if provider.local {
                return Ok(None);
            }
            tauri::async_runtime::spawn_blocking(move || {
                vults_secrets::get(provider.secret())
                    .ok()
                    .flatten()
                    .filter(|s| provider.accepts_key(s))
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
                let span = crate::gate::shared(&app).reserve_cleanup(
                    &mut store,
                    &choice.budget,
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
        Box::pin(filter::complete(
            self.choice.provider,
            prepared.as_deref(),
            &self.choice.model,
            request.messages(),
            Duration::from_secs(12),
        ))
    }
    fn done(&self, mut span: Span, outcome: &str, usage: Option<(Option<u64>, Option<u64>)>, took: Duration) {
        span.end_ms = span
            .start_ms
            .saturating_add(i64::try_from(took.as_millis()).unwrap_or(i64::MAX));
        span.status = if outcome == "applied" { "ok" } else { "error" }.into();
        span.attrs["vults.outcome"] = serde_json::json!(outcome);
        if let Some((input, output)) = usage {
            if let Some(input) = input {
                span.attrs["gen_ai.usage.input_tokens"] = serde_json::json!(input);
            }
            if let Some(output) = output {
                span.attrs["gen_ai.usage.output_tokens"] = serde_json::json!(output);
            }
        }
        crate::writer::send(crate::writer::Job::FinishRequest(span));
    }
}

pub(super) fn apply(
    app: &AppHandle,
    ticket: &Ticket,
    choice: &Choice,
    input: &Processed,
    spellings: &[String],
) -> Option<String> {
    let options = choice.options(spellings);
    let request = Request::build(input, &options).ok().flatten()?;
    let net = Native {
        app: app.clone(),
        ticket: ticket.clone(),
        choice: choice.clone(),
    };
    let started = Instant::now();
    let result = tauri::async_runtime::block_on(run(&net, &request, &options));
    ticket.history_cleanup(started.elapsed(), result.as_ref().map(|_| choice.resolved));
    result
}

#[cfg(test)]
mod tests;
