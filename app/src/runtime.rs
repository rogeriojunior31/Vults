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

/// The last view, for a surface that (re)loads or opens after it was pushed: every page asks
/// for it on load (`current_view`).
#[derive(Debug, Default)]
pub struct LastView(Mutex<Option<ViewModel>>);

/// Sends the view to every live surface and keeps it for the ones still to load (ADR 0008: one
/// view for every surface). One broadcast, not one emit per window: a page's `listen` hears
/// every target, so an emit per window would reach each page once per window.
pub fn publish_view<R: tauri::Runtime>(app: &AppHandle<R>, view: &ViewModel) {
    if let Ok(mut slot) = app.state::<LastView>().0.lock() {
        *slot = Some(view.clone());
    }
    let _ = app.emit("view", view);
}

#[derive(Debug)]
pub struct Inbox(mpsc::Sender<Msg>);

#[derive(Debug)]
enum Msg {
    Rules(Vec<core::Rule>),
    Flock(core::flock::Flock),
    Outfit(core::looks::Outfit),
    Presence(core::Presence),
    Dnd(Option<Instant>),
    Locked(bool),
    Project {
        cwd: String,
        prefs: core::ProjectPrefs,
    },
    Hook(Incoming),
    Connector(vultures_ai_connectors::Update),
    User(Intent),
    Tick,
    /// The island asks for a step's whole diff; only the loop holds it.
    Diff {
        session: core::SessionKey,
        step: u32,
        reply: tokio::sync::oneshot::Sender<Option<core::Diff>>,
    },
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
        state.outfit = s.zeca_look;
        state.presence = s.presence;
        state.projects = s.projects.clone();
        state.dnd_until = crate::settings::dnd_instant(s.dnd_until);
    }
    if let Some(date) = today() {
        core::reduce(&mut state, Input::Today(date), Instant::now());
    }
    // A new season on every start: the flock draws its species anew.
    state.season = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos() as u64);
    let mut waiting: HashMap<RequestId, ReplyHandle> = HashMap::new();
    let mut last_view: Option<ViewModel> = None;
    let mut notifier = core::notify::Notifier::default();

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
            Msg::Connector(vultures_ai_connectors::Update::Event(e)) => {
                tracing::info!(connector = %e.connector, level = ?e.level, "connector news");
                Some(Input::Connector(alert(e)))
            }
            Msg::Connector(vultures_ai_connectors::Update::Board { connector, rows }) => Some(Input::Board {
                connector,
                rows: rows.map(|rows| rows.into_iter().map(row).collect()),
            }),
            Msg::User(intent) => Some(Input::User(intent)),
            Msg::Rules(rules) => Some(Input::SetRules(rules)),
            Msg::Flock(flock) => Some(Input::SetFlock(flock)),
            Msg::Outfit(outfit) => Some(Input::SetOutfit(outfit)),
            Msg::Presence(presence) => Some(Input::SetPresence(presence)),
            Msg::Dnd(until) => Some(Input::SetDnd(until)),
            // Back at the screen: the news the notifications held back joins the digest.
            Msg::Locked(locked) => Some(Input::Locked {
                locked,
                missed: if locked { Vec::new() } else { notifier.missed() },
            }),
            Msg::Project { cwd, prefs } => Some(Input::SetProject { cwd, prefs }),
            Msg::Tick => {
                // A new day may bring a new look: the date rides on the minute's tick.
                if let Some(date) = today() {
                    core::reduce(&mut state, Input::Today(date), now);
                }
                // Do not disturb ends by the wall clock, which goes on through a suspend.
                let dnd = crate::settings::dnd_instant(crate::settings::dnd_until(&app));
                let was = state.dnd_until.is_some();
                core::reduce(&mut state, Input::SetDnd(dnd), now);
                if was && state.dnd_until.is_none() {
                    // An open Settings page shows it off.
                    let _ = app.emit("settings", serde_json::json!({ "dndUntil": null }));
                }
                Some(Input::Tick)
            }
            Msg::Diff { session, step, reply } => {
                let _ = reply.send(state.diff(&session, step).cloned());
                continue;
            }
        };
        let Some(input) = input else { continue };

        for effect in core::reduce(&mut state, input, now) {
            match effect {
                Effect::AckPermission(id) => {
                    if let Some(h) = waiting.get(&id) {
                        h.ack();
                    }
                    // A tick at each step of the card's attention ladder (its notification, each
                    // reminder); and the card's lifetime is the hook's: one tick after it, the
                    // core drops it.
                    let tick = tx.clone();
                    tauri::async_runtime::spawn(async move {
                        let start = tokio::time::Instant::now();
                        let steps = core::notify::ladder()
                            .into_iter()
                            .chain([limits::SERVER_DECISION_TIMEOUT]);
                        for at in steps {
                            tokio::time::sleep_until(start + at).await;
                            if tick.send(Msg::Tick).await.is_err() {
                                return;
                            }
                        }
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
                    let count = rules.len();
                    match crate::settings::edit(&app, |s| s.rules = rules) {
                        Ok(()) => tracing::info!(count, "always-allow rules saved"),
                        // The rule still applies until the app quits; only the file is behind.
                        Err(e) => tracing::warn!(count, error = %e, "always-allow rules not saved"),
                    }
                }
                Effect::SaveProjects(projects) => {
                    let count = projects.len();
                    match crate::settings::edit(&app, |s| s.projects = projects.clone()) {
                        Ok(()) => tracing::info!(count, "project choices saved"),
                        Err(e) => tracing::warn!(count, error = %e, "project choices not saved"),
                    }
                    // An open Settings window lists them.
                    let _ = app.emit("settings", serde_json::json!({ "projects": projects }));
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
                // Checks the disk and may start the editor: off the loop.
                Effect::OpenFolder(path) => {
                    let app = app.clone();
                    tauri::async_runtime::spawn_blocking(move || crate::open::folder(&app, &path));
                }
                Effect::OpenFile { path, line } => {
                    let app = app.clone();
                    tauri::async_runtime::spawn_blocking(move || crate::open::file(&app, &path, line));
                }
                Effect::OpenUrl(url) => {
                    use tauri_plugin_opener::OpenerExt;
                    let _ = app.opener().open_url(url.as_str(), None::<&str>);
                }
            }
        }

        let view = state.view();
        if last_view.as_ref() != Some(&view) {
            publish_view(&app, &view);
            crate::tray::show(&app, &view);
            last_view = Some(view);
        }
        let prefs = core::notify::Prefs {
            on: crate::settings::notifications(&app),
        };
        crate::notify::send(&app, notifier.update(&state, now, prefs));
    }
}

/// Weighs the notifications again now (a setting changed), rather than at the next event.
pub fn recheck(app: &AppHandle) {
    if let Some(inbox) = app.try_state::<Inbox>() {
        let _ = inbox.0.try_send(Msg::Tick);
    }
}

fn alert(e: vultures_ai_connectors::Event) -> core::Alert {
    use vultures_ai_connectors::Level;
    core::Alert {
        key: e.key,
        topic: e.topic,
        seq: 0,
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

fn row(r: vultures_ai_connectors::Row) -> core::board::Row {
    use core::board::{Checks, Group, Verdict};
    use vultures_ai_connectors as c;
    core::board::Row {
        item: r.item,
        group: match r.group {
            c::Group::Yours => Group::Yours,
            c::Group::ToReview => Group::ToReview,
            c::Group::Branches => Group::Branches,
        },
        name: r.name,
        title: r.title,
        checks: r.checks.map(|c| match c {
            c::Checks::Passing => Checks::Passing,
            c::Checks::Failing => Checks::Failing,
            c::Checks::Running => Checks::Running,
        }),
        review: r.review.map(|v| match v {
            c::Review::Approved => Verdict::Approved,
            c::Review::Changes => Verdict::Changes,
        }),
        // Checked here, once, as an alert's link.
        url: r.url.as_deref().and_then(core::SafeUrl::parse),
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

/// Only the island answers a card (ADR 0008): another window's call is dropped, whatever its
/// page does.
fn card_host(window: &tauri::WebviewWindow) -> bool {
    window.label() == ISLAND
}

/// Allow / Deny. The core ignores ids that are not the card on screen.
#[tauri::command]
pub async fn decide(
    window: tauri::WebviewWindow,
    request: String,
    decision: UiDecision,
    inbox: tauri::State<'_, Inbox>,
) -> Result<(), ()> {
    if !card_host(&window) {
        return Err(());
    }
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
pub async fn decide_always(
    window: tauri::WebviewWindow,
    request: String,
    inbox: tauri::State<'_, Inbox>,
) -> Result<(), ()> {
    if !card_host(&window) {
        return Err(());
    }
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
    window: tauri::WebviewWindow,
    request: String,
    answers: Vec<Answer>,
    inbox: tauri::State<'_, Inbox>,
) -> Result<(), ()> {
    if !card_host(&window) {
        return Err(());
    }
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
pub async fn question_release(
    window: tauri::WebviewWindow,
    request: String,
    inbox: tauri::State<'_, Inbox>,
) -> Result<(), ()> {
    if !card_host(&window) {
        return Err(());
    }
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

/// The project choices, for the settings window.
#[tauri::command]
pub fn projects_list(
    state: tauri::State<'_, crate::settings::SettingsState>,
) -> std::collections::BTreeMap<String, core::ProjectPrefs> {
    state.0.lock().map(|s| s.projects.clone()).unwrap_or_default()
}

/// One project's choices, from the settings (unhide, unmute, unpin, forget). Core applies them
/// and saves the list (`Effect::SaveProjects`), as for a quick action: one writer, no race.
#[tauri::command]
pub async fn project_set(
    cwd: String,
    prefs: core::ProjectPrefs,
    inbox: tauri::State<'_, Inbox>,
) -> Result<(), String> {
    inbox
        .0
        .send(Msg::Project { cwd, prefs })
        .await
        .map_err(|_| "the app is busy".to_string())
}

/// A quick action: mute, pin or hide the session's project, or undo it.
#[tauri::command]
pub async fn session_project_pref(
    agent: vultures_ai_protocol::AgentKind,
    id: String,
    pref: UiProjectPref,
    on: bool,
    inbox: tauri::State<'_, Inbox>,
) -> Result<(), ()> {
    let session = core::SessionKey {
        agent,
        session_id: id,
    };
    let pref = match pref {
        UiProjectPref::Mute => core::ProjectPref::Mute,
        UiProjectPref::Pin => core::ProjectPref::Pin,
        UiProjectPref::Hide => core::ProjectPref::Hide,
    };
    inbox
        .0
        .send(Msg::User(Intent::SetProjectPref { session, pref, on }))
        .await
        .map_err(|_| ())
}

/// The answer to a quiet bird: only its flag changes; nothing reaches the agent.
#[tauri::command]
pub async fn session_hush(
    agent: vultures_ai_protocol::AgentKind,
    id: String,
    hush: UiHush,
    inbox: tauri::State<'_, Inbox>,
) -> Result<(), ()> {
    let session = core::SessionKey {
        agent,
        session_id: id,
    };
    let hush = match hush {
        UiHush::Snooze => core::silence::Hush::Snooze,
        UiHush::KeepGoing => core::silence::Hush::KeepGoing,
        UiHush::Dismiss => core::silence::Hush::Dismiss,
    };
    inbox
        .0
        .send(Msg::User(Intent::Hush { session, hush }))
        .await
        .map_err(|_| ())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum UiHush {
    Snooze,
    KeepGoing,
    Dismiss,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UiProjectPref {
    Mute,
    Pin,
    Hide,
}

/// The pool the flock draws from: saved, and the sessions' birds are drawn again from it.
#[tauri::command]
pub async fn set_flock(
    app: AppHandle,
    flock: core::flock::Flock,
    inbox: tauri::State<'_, Inbox>,
) -> Result<(), String> {
    // Sent while the settings are locked, so the core sees choices in the order the file does.
    let mut sent = Ok(());
    crate::settings::edit(&app, |s| {
        sent = inbox.0.try_send(Msg::Flock(flock));
        // Saved only once the core has it: a full inbox leaves both as they were.
        if sent.is_ok() {
            s.flock = flock;
        }
    })?;
    sent.map_err(|_| "the app is busy".to_string())
}

/// The screen locked or unlocked (`lock`).
pub fn set_locked(app: &AppHandle, locked: bool) {
    if let Some(inbox) = app.try_state::<Inbox>() {
        let inbox = inbox.0.clone();
        // From the D-Bus task: wait for room rather than lose an unlock.
        tauri::async_runtime::spawn(async move {
            let _ = inbox.send(Msg::Locked(locked)).await;
        });
    }
}

/// The digest ("While you were away") read and closed on the island.
#[tauri::command]
pub async fn digest_dismiss(inbox: tauri::State<'_, Inbox>) -> Result<(), ()> {
    inbox
        .0
        .send(Msg::User(Intent::DismissDigest))
        .await
        .map_err(|_| ())
}

/// Do not disturb until then (epoch seconds), or off: the core hears it now, and a tick comes
/// when it ends so it ends on time.
pub fn set_dnd(app: &AppHandle, until: Option<u64>) -> Result<(), String> {
    let inbox = app.state::<Inbox>();
    let at = crate::settings::dnd_instant(until);
    inbox
        .0
        .try_send(Msg::Dnd(at))
        .map_err(|_| "the app is busy".to_string())?;
    if let Some(at) = at {
        let tick = inbox.0.clone();
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep_until(at.into()).await;
            let _ = tick.send(Msg::Tick).await;
        });
    }
    Ok(())
}

/// The presence preset: core and the file change together, or neither does (as `set_flock`).
pub fn set_presence(app: &AppHandle, presence: core::Presence) -> Result<(), String> {
    let inbox = app.state::<Inbox>();
    let mut sent = Ok(());
    crate::settings::edit(app, |s| {
        sent = inbox.0.try_send(Msg::Presence(presence));
        if sent.is_ok() {
            s.presence = presence;
        }
    })?;
    sent.map_err(|_| "the app is busy".to_string())
}

/// Zeca's look: saved, and the island wears it with the next view.
#[tauri::command]
pub async fn set_zeca_look(
    app: AppHandle,
    look: core::looks::Outfit,
    inbox: tauri::State<'_, Inbox>,
) -> Result<(), String> {
    // As `set_flock`: the core and the file change together, or neither does.
    let mut sent = Ok(());
    crate::settings::edit(&app, |s| {
        sent = inbox.0.try_send(Msg::Outfit(look));
        if sent.is_ok() {
            s.zeca_look = look;
        }
    })?;
    sent.map_err(|_| "the app is busy".to_string())?;
    // Picked on the island or in Settings: the other one marks it too.
    let _ = app.emit("settings", serde_json::json!({ "zecaLook": look }));
    Ok(())
}

/// The user's date, in their time zone; none where the OS can't say (the looks then wait).
fn today() -> Option<core::looks::Date> {
    #[cfg(target_os = "linux")]
    return vultures_ai_platform::linux::today().map(|(y, m, d)| core::looks::Date::new(y, m, d));
    #[cfg(not(target_os = "linux"))]
    None
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

/// A global shortcut's intent (next or previous session), or a notification's *Open*. Dropped
/// when the inbox is full: a key press is not worth waiting for.
#[cfg(target_os = "linux")]
pub fn shortcut_intent(app: &AppHandle, intent: Intent) {
    if let Some(inbox) = app.try_state::<Inbox>() {
        let _ = inbox.0.try_send(Msg::User(intent));
    }
}

/// A click on a session's bird or row: put it in front. No agent and id gives the choice back to
/// core's rule.
#[tauri::command]
pub async fn session_focus(
    agent: Option<vultures_ai_protocol::AgentKind>,
    id: Option<String>,
    inbox: tauri::State<'_, Inbox>,
) -> Result<(), ()> {
    let session = agent
        .zip(id)
        .map(|(agent, session_id)| core::SessionKey { agent, session_id });
    inbox
        .0
        .send(Msg::User(Intent::Focus { session }))
        .await
        .map_err(|_| ())
}

/// A quick action: the session's folder in the editor or the file manager.
#[tauri::command]
pub async fn session_open_folder(
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
        .send(Msg::User(Intent::OpenFolder { session }))
        .await
        .map_err(|_| ())
}

/// A quick action: one file of a kept step's diff, in the editor.
#[tauri::command]
pub async fn session_open_file(
    agent: vultures_ai_protocol::AgentKind,
    id: String,
    step: u32,
    file: usize,
    inbox: tauri::State<'_, Inbox>,
) -> Result<(), ()> {
    let session = core::SessionKey {
        agent,
        session_id: id,
    };
    inbox
        .0
        .send(Msg::User(Intent::OpenFile { session, step, file }))
        .await
        .map_err(|_| ())
}

/// A step's whole diff, for the island's diff card; `None` once the step is gone.
#[tauri::command]
pub async fn step_diff(
    agent: vultures_ai_protocol::AgentKind,
    id: String,
    step: u32,
    inbox: tauri::State<'_, Inbox>,
) -> Result<Option<core::Diff>, ()> {
    let (reply, answer) = tokio::sync::oneshot::channel();
    let session = core::SessionKey {
        agent,
        session_id: id,
    };
    inbox
        .0
        .send(Msg::Diff { session, step, reply })
        .await
        .map_err(|_| ())?;
    answer.await.map_err(|_| ())
}

#[tauri::command]
pub async fn alert_open(key: String, inbox: tauri::State<'_, Inbox>) -> Result<(), ()> {
    inbox
        .0
        .send(Msg::User(Intent::OpenAlert { key }))
        .await
        .map_err(|_| ())
}

/// A click on a row of a connector's card: the core opens its link, if it has a safe one.
#[tauri::command]
pub async fn board_open(connector: String, item: String, inbox: tauri::State<'_, Inbox>) -> Result<(), ()> {
    inbox
        .0
        .send(Msg::User(Intent::OpenRow { connector, item }))
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

/// The page measured what it draws; `width == 0` means nothing is shown. Each window sets its
/// own surface's region, never another's.
#[tauri::command]
pub fn layout(app: AppHandle, window: tauri::WebviewWindow, x: i32, y: i32, width: i32, height: i32) {
    if !crate::SURFACES.contains(&window.label()) {
        return;
    }
    let _ = app.run_on_main_thread(move || {
        #[cfg(target_os = "linux")]
        if let Ok(gtk) = window.gtk_window() {
            use vultures_ai_platform::linux::{Rect, set_input_region};
            set_input_region(&gtk, window.label(), Some(Rect { x, y, width, height }));
        }
        #[cfg(not(target_os = "linux"))]
        let _ = (window, x, y, width, height);
    });
}

/// The island's chat needs the keyboard; everything else must never take it from the user's
/// terminal. Asked by the window itself.
#[tauri::command]
pub fn surface_keyboard(app: AppHandle, window: tauri::WebviewWindow, on: bool) {
    if !crate::SURFACES.contains(&window.label()) {
        return;
    }
    let _ = app.run_on_main_thread(move || {
        #[cfg(target_os = "linux")]
        if let Ok(gtk) = window.gtk_window() {
            vultures_ai_platform::linux::set_keyboard(&gtk, on);
        }
        if on {
            let _ = window.set_focus();
        }
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

#[cfg(test)]
#[cfg(not(windows))]
mod tests {
    use std::sync::{Arc, Mutex};

    use tauri::{Listener, Manager, WebviewUrl, WebviewWindowBuilder};

    use super::*;

    /// What each window heard, as (event, payload).
    type Heard = Arc<Mutex<Vec<(String, String)>>>;

    fn window(app: &tauri::App<tauri::test::MockRuntime>, label: &str) -> Heard {
        let win = WebviewWindowBuilder::new(app, label, WebviewUrl::default())
            .build()
            .unwrap();
        let heard: Heard = Arc::default();
        for event in ["view", "pointer"] {
            let heard = heard.clone();
            win.listen(event, move |e| {
                heard
                    .lock()
                    .unwrap()
                    .push((event.to_string(), e.payload().to_string()));
            });
        }
        heard
    }

    #[test]
    fn every_surface_gets_the_same_view_and_island_events_stay_on_the_island() {
        let app = tauri::test::mock_app();
        app.manage(LastView::default());
        let island = window(&app, ISLAND);
        let other = window(&app, "test-surface");
        // Like a page's plain `listen` (bridge.ts): it hears every target, so it counts copies.
        let copies = Arc::new(Mutex::new(0));
        let count = copies.clone();
        app.listen_any("view", move |_| *count.lock().unwrap() += 1);

        let view = State::default().view();
        publish_view(app.handle(), &view);
        assert_eq!(
            *copies.lock().unwrap(),
            1,
            "one view per change, however many windows"
        );
        let _ = app.emit_to(ISLAND, "pointer", true);

        let payload = serde_json::to_string(&view).unwrap();
        let island = island.lock().unwrap().clone();
        let other = other.lock().unwrap().clone();
        assert_eq!(
            island,
            vec![
                ("view".into(), payload.clone()),
                ("pointer".into(), "true".into())
            ]
        );
        assert_eq!(
            other,
            vec![("view".into(), payload)],
            "one view, no island-only event"
        );
        // A surface that opens later asks for the view it missed.
        assert_eq!(current_view(app.state::<LastView>()), Some(view));
    }
}
