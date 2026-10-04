//! The one loop that owns the domain state. Hook connections, clicks and timers all become
//! messages to it; it runs `core::reduce`, executes the effects and pushes the view to the UI.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Instant;

use serde::Deserialize;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::mpsc;
use vultures_ai_core::{self as core, Effect, Input, Intent, RequestId, State, ViewModel};
use vultures_ai_ipc::{Endpoint, Incoming, ReplyHandle};
use vultures_ai_protocol::{Answer, Decision, limits};

use crate::ISLAND;

/// The last view, for a webview that (re)loads after it was pushed.
#[derive(Debug, Default)]
pub struct LastView(Mutex<Option<ViewModel>>);

#[derive(Debug)]
pub struct Inbox(mpsc::Sender<Msg>);

#[derive(Debug)]
enum Msg {
    Rules(Vec<core::Rule>),
    Flock(core::flock::Flock),
    Hook(Incoming),
    Connector(vultures_ai_connectors::Event),
    User(Intent),
    Tick,
}

pub fn start(app: AppHandle) {
    let (tx, rx) = mpsc::channel(64);
    app.manage(Inbox(tx.clone()));
    app.manage(LastView::default());

    let (hooks_tx, mut hooks_rx) = mpsc::channel(64);
    tauri::async_runtime::spawn(async move {
        match Endpoint::for_current_user() {
            Ok(endpoint) => {
                if let Err(err) = vultures_ai_ipc::serve(endpoint, hooks_tx).await {
                    tracing::error!("hook server stopped: {err}");
                }
            }
            Err(err) => tracing::error!("no hook endpoint: {err}"),
        }
    });
    let forward = tx.clone();
    tauri::async_runtime::spawn(async move {
        while let Some(incoming) = hooks_rx.recv().await {
            if forward.send(Msg::Hook(incoming)).await.is_err() {
                break;
            }
        }
    });
    let (alerts_tx, mut alerts_rx) = mpsc::channel(64);
    crate::connectors::start(&app, alerts_tx);
    let forward = tx.clone();
    tauri::async_runtime::spawn(async move {
        while let Some(event) = alerts_rx.recv().await {
            if forward.send(Msg::Connector(event)).await.is_err() {
                break;
            }
        }
    });
    // Once a minute, so silent sessions and stale cards age out. One message a minute is
    // all it costs when nothing is going on.
    let ticker = tx.clone();
    tauri::async_runtime::spawn(async move {
        let mut every = tokio::time::interval(std::time::Duration::from_secs(60));
        loop {
            every.tick().await;
            if ticker.send(Msg::Tick).await.is_err() {
                break;
            }
        }
    });
    tauri::async_runtime::spawn(run(app, rx, tx));
}

async fn run(app: AppHandle, mut rx: mpsc::Receiver<Msg>, tx: mpsc::Sender<Msg>) {
    let mut state = State::default();
    if let Ok(s) = app.state::<crate::settings::SettingsState>().0.lock() {
        state.rules = s.rules.clone();
        state.flock = s.flock;
    }
    // A new season on every start: the flock draws its species anew.
    state.season = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos() as u64);
    let mut waiting: HashMap<RequestId, ReplyHandle> = HashMap::new();
    let mut last_view: Option<ViewModel> = None;

    while let Some(msg) = rx.recv().await {
        let now = Instant::now();
        let input = match msg {
            Msg::Hook(Incoming::Event(event)) => {
                tracing::debug!(agent = ?event.agent, event = %event.event, "hook");
                // The plan's usage, not a session's: it goes to the header, never to the core.
                if event.agent == vultures_ai_protocol::AgentKind::Claude
                    && event.event == vultures_ai_protocol::STATUS_LINE_EVENT
                {
                    let windows =
                        vultures_ai_agents::usage::claude(&event.payload, crate::usage::epoch_now());
                    if !windows.is_empty() {
                        crate::usage::set(&app, event.agent, windows);
                    }
                    continue;
                }
                parse(&event)
            }
            Msg::Hook(Incoming::Request { event, reply }) => {
                tracing::info!(agent = ?event.agent, event = %event.event, "request for a human");
                let input = parse(&event);
                let is_card = matches!(
                    &input,
                    Some(Input::Agent(core::AgentUpdate {
                        event: core::AgentEvent::PermissionRequested { .. }
                            | core::AgentEvent::QuestionAsked { .. },
                        ..
                    }))
                );
                if is_card {
                    waiting.insert(RequestId(event.id.clone()), reply);
                } else {
                    // Nothing here can decide it (a question, an unknown event): the terminal asks now.
                    reply.decline();
                }
                input
            }
            Msg::Connector(e) => {
                tracing::info!(connector = %e.connector, level = ?e.level, "connector news");
                Some(Input::Connector(alert(e)))
            }
            Msg::User(intent) => Some(Input::User(intent)),
            Msg::Rules(rules) => Some(Input::SetRules(rules)),
            Msg::Flock(flock) => Some(Input::SetFlock(flock)),
            Msg::Tick => Some(Input::Tick),
        };
        let Some(input) = input else { continue };

        for effect in core::reduce(&mut state, input, now) {
            match effect {
                Effect::AckPermission(id) => {
                    if let Some(h) = waiting.get(&id) {
                        h.ack();
                    }
                    // The card's lifetime is the hook's; one tick after it, the core drops it.
                    let tick = tx.clone();
                    tauri::async_runtime::spawn(async move {
                        tokio::time::sleep(limits::SERVER_DECISION_TIMEOUT).await;
                        let _ = tick.send(Msg::Tick).await;
                    });
                }
                Effect::RespondPermission { request, decision } => {
                    tracing::info!(?decision, "permission answered from the island");
                    if let Some(h) = waiting.remove(&request) {
                        h.decide(decision);
                    }
                }
                Effect::AnswerQuestion { request, answers } => {
                    tracing::info!(count = answers.len(), "question answered from the island");
                    if let Some(h) = waiting.remove(&request) {
                        h.answer(answers);
                    }
                }
                Effect::ReleasePermission(id) => {
                    tracing::debug!("permission released to the terminal");
                    if let Some(h) = waiting.remove(&id) {
                        h.decline();
                    }
                }
                Effect::SaveRules(rules) => {
                    tracing::info!(count = rules.len(), "always-allow rules saved");
                    let _ = crate::settings::edit(&app, |s| s.rules = rules);
                }
                Effect::JumpToTerminal(terminal) => {
                    // Shells out (herdr, tmux, gdbus…): off the loop. The island says so when
                    // there was nothing to try.
                    let app = app.clone();
                    tauri::async_runtime::spawn_blocking(move || {
                        #[cfg(target_os = "linux")]
                        let found = vultures_ai_platform::jump::jump(&terminal);
                        #[cfg(not(target_os = "linux"))]
                        let found = {
                            let _ = terminal;
                            false
                        };
                        if !found {
                            tracing::info!("no terminal to bring forward");
                            let _ = app.emit_to(ISLAND, "jump-failed", ());
                        }
                    });
                }
                Effect::OpenUrl(url) => {
                    use tauri_plugin_opener::OpenerExt;
                    let _ = app.opener().open_url(url.as_str(), None::<&str>);
                }
            }
        }

        let view = state.view();
        if last_view.as_ref() != Some(&view) {
            let _ = app.emit_to(ISLAND, "view", &view);
            if let Ok(mut slot) = app.state::<LastView>().0.lock() {
                *slot = Some(view.clone());
            }
            last_view = Some(view);
        }
    }
}

fn alert(e: vultures_ai_connectors::Event) -> core::Alert {
    use vultures_ai_connectors::Level;
    core::Alert {
        key: e.key,
        connector: e.connector,
        level: match e.level {
            Level::Info => core::AlertLevel::Info,
            Level::Ok => core::AlertLevel::Ok,
            Level::Warn => core::AlertLevel::Warn,
            Level::Error => core::AlertLevel::Error,
        },
        title: e.title,
        detail: e.detail,
        // Checked here, once: the UI can only ask to open an alert, never a URL.
        url: e.url.as_deref().and_then(core::SafeUrl::parse),
    }
}

fn parse(event: &vultures_ai_protocol::Event) -> Option<Input> {
    vultures_ai_agents::parse(event).map(Input::Agent)
}

#[tauri::command]
pub fn current_view(views: tauri::State<'_, LastView>) -> Option<ViewModel> {
    views.0.lock().ok().and_then(|v| v.clone())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UiDecision {
    Allow,
    Deny,
}

/// Allow / Deny. The core ignores ids that are not the card on screen.
#[tauri::command]
pub async fn decide(request: String, decision: UiDecision, inbox: tauri::State<'_, Inbox>) -> Result<(), ()> {
    let decision = match decision {
        UiDecision::Allow => Decision::Allow,
        UiDecision::Deny => Decision::Deny,
    };
    inbox
        .0
        .send(Msg::User(Intent::Decide {
            request: RequestId(request),
            decision,
        }))
        .await
        .map_err(|_| ())
}

/// A click on Always: allow it, and every identical request in this project.
#[tauri::command]
pub async fn decide_always(request: String, inbox: tauri::State<'_, Inbox>) -> Result<(), ()> {
    inbox
        .0
        .send(Msg::User(Intent::DecideAlways {
            request: RequestId(request),
        }))
        .await
        .map_err(|_| ())
}

/// The replies to a question card, one per question. The core drops any that don't fit it.
#[tauri::command]
pub async fn question_answer(
    request: String,
    answers: Vec<Answer>,
    inbox: tauri::State<'_, Inbox>,
) -> Result<(), ()> {
    inbox
        .0
        .send(Msg::User(Intent::Answer {
            request: RequestId(request),
            answers,
        }))
        .await
        .map_err(|_| ())
}

/// "Reply in the terminal": the card goes, and the agent asks there.
#[tauri::command]
pub async fn question_release(request: String, inbox: tauri::State<'_, Inbox>) -> Result<(), ()> {
    inbox
        .0
        .send(Msg::User(Intent::Release {
            request: RequestId(request),
        }))
        .await
        .map_err(|_| ())
}

/// The always-allow rules, for the settings window.
#[tauri::command]
pub fn rules_list(state: tauri::State<'_, crate::settings::SettingsState>) -> Vec<core::Rule> {
    state.0.lock().map(|s| s.rules.clone()).unwrap_or_default()
}

/// Removes a rule (by its position in the list) and tells the core.
#[tauri::command]
pub async fn rule_remove(app: AppHandle, index: usize, inbox: tauri::State<'_, Inbox>) -> Result<(), String> {
    let mut rules = Vec::new();
    crate::settings::edit(&app, |s| {
        if index < s.rules.len() {
            s.rules.remove(index);
        }
        rules = s.rules.clone();
    })?;
    tracing::info!(count = rules.len(), "always-allow rule removed");
    inbox
        .0
        .send(Msg::Rules(rules))
        .await
        .map_err(|_| "the app is busy".to_string())
}

/// The pool the flock draws from: saved, and the sessions' birds are drawn again from it.
#[tauri::command]
pub async fn set_flock(
    app: AppHandle,
    flock: core::flock::Flock,
    inbox: tauri::State<'_, Inbox>,
) -> Result<(), String> {
    crate::settings::edit(&app, |s| s.flock = flock)?;
    inbox
        .0
        .send(Msg::Flock(flock))
        .await
        .map_err(|_| "the app is busy".to_string())
}

/// A click on a session row: bring its terminal forward.
#[tauri::command]
pub async fn session_jump(
    agent: vultures_ai_protocol::AgentKind,
    id: String,
    inbox: tauri::State<'_, Inbox>,
) -> Result<(), ()> {
    let session = core::SessionKey {
        agent,
        session_id: id,
    };
    inbox
        .0
        .send(Msg::User(Intent::Jump { session }))
        .await
        .map_err(|_| ())
}

#[tauri::command]
pub async fn alert_open(key: String, inbox: tauri::State<'_, Inbox>) -> Result<(), ()> {
    inbox
        .0
        .send(Msg::User(Intent::OpenAlert { key }))
        .await
        .map_err(|_| ())
}

#[tauri::command]
pub async fn alert_dismiss(key: String, inbox: tauri::State<'_, Inbox>) -> Result<(), ()> {
    inbox
        .0
        .send(Msg::User(Intent::DismissAlert { key }))
        .await
        .map_err(|_| ())
}

/// Logical size of the island window. Fixed: the UI draws the island inside it and reports
/// the island's rectangle, which becomes the only part that takes the mouse.
pub const ISLAND_SIZE: (i32, i32) = (720, 560);

/// The UI measured the island; `width == 0` means nothing is shown.
#[tauri::command]
pub fn layout(app: AppHandle, x: i32, y: i32, width: i32, height: i32) {
    let Some(win) = app.get_webview_window(ISLAND) else {
        return;
    };
    let _ = app.run_on_main_thread(move || {
        #[cfg(target_os = "linux")]
        if let Ok(gtk) = win.gtk_window() {
            use vultures_ai_platform::linux::{Rect, set_input_region};
            set_input_region(&gtk, Some(Rect { x, y, width, height }));
        }
        #[cfg(not(target_os = "linux"))]
        let _ = (win, x, y, width, height);
    });
}

/// Without a compositor to center it: the top center of the current monitor.
pub fn place_top_center(win: &tauri::WebviewWindow) {
    let (width, height) = ISLAND_SIZE;
    let _ = win.set_size(tauri::LogicalSize::new(width, height));
    if let Ok(Some(monitor)) = win.current_monitor() {
        let scale = monitor.scale_factor();
        let screen = monitor.size().to_logical::<f64>(scale);
        let origin = monitor.position().to_logical::<f64>(scale);
        let x = origin.x + (screen.width - f64::from(width)) / 2.0;
        let _ = win.set_position(tauri::LogicalPosition::new(x, origin.y));
    }
}
