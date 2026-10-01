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
use vultures_ai_protocol::{Decision, limits};

use crate::ISLAND;

/// The last view, for a webview that (re)loads after it was pushed.
#[derive(Debug, Default)]
pub struct LastView(Mutex<Option<ViewModel>>);

#[derive(Debug)]
pub struct Inbox(mpsc::Sender<Msg>);

#[derive(Debug)]
enum Msg {
    Hook(Incoming),
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
                    eprintln!("hook server stopped: {err}");
                }
            }
            Err(err) => eprintln!("no hook endpoint: {err}"),
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
    tauri::async_runtime::spawn(run(app, rx, tx));
}

async fn run(app: AppHandle, mut rx: mpsc::Receiver<Msg>, tx: mpsc::Sender<Msg>) {
    let mut state = State::default();
    let mut waiting: HashMap<RequestId, ReplyHandle> = HashMap::new();
    let mut last_view: Option<ViewModel> = None;

    while let Some(msg) = rx.recv().await {
        let now = Instant::now();
        let input = match msg {
            Msg::Hook(Incoming::Event(event)) => parse(&event),
            Msg::Hook(Incoming::Request { event, reply }) => {
                let input = parse(&event);
                let is_card = matches!(
                    &input,
                    Some(Input::Agent(core::AgentUpdate {
                        event: core::AgentEvent::PermissionRequested { .. },
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
            Msg::User(intent) => Some(Input::User(intent)),
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
                    if let Some(h) = waiting.remove(&request) {
                        h.decide(decision);
                    }
                }
                Effect::ReleasePermission(id) => {
                    if let Some(h) = waiting.remove(&id) {
                        h.decline();
                    }
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

fn parse(event: &vultures_ai_protocol::Event) -> Option<Input> {
    vultures_ai_agents::agent(event.agent)?
        .parse(event)
        .map(Input::Agent)
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

/// Logical size of the island window. Fixed: the UI draws the island inside it and reports
/// the island's rectangle, which becomes the only part that takes the mouse.
pub const ISLAND_SIZE: (i32, i32) = (540, 560);

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
