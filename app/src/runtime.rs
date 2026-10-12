//! The one loop that owns the domain state. Hook connections, clicks and timers all become
//! messages to it; it runs `core::reduce`, executes the effects and pushes the view to the UI.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::Deserialize;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::mpsc;
use vults_core::{self as core, Effect, Input, Intent, RequestId, State, ViewModel};
use vults_ipc::{Endpoint, Incoming, ReplyHandle};
use vults_protocol::{Answer, Decision};

use crate::ISLAND;

/// The last view, for a surface that (re)loads or opens after it was pushed: every page asks
/// for it on load (`current_view`).
#[derive(Debug, Default)]
pub struct LastView(Mutex<Option<ViewModel>>);

impl LastView {
    /// The last view, as every window got it.
    pub fn get(&self) -> Option<ViewModel> {
        self.0.lock().ok().and_then(|v| v.clone())
    }

    /// The screen is locked, by the last view.
    pub fn locked(&self) -> bool {
        self.0.lock().is_ok_and(|v| v.as_ref().is_some_and(|v| v.locked))
    }
}

/// Sends the view to every live surface and keeps it for the ones still to load (ADR 0008: one
/// view for every surface). One broadcast, not one emit per window: a page's `listen` hears
/// every target, so an emit per window would reach each page once per window.
pub fn publish_view<R: tauri::Runtime>(app: &AppHandle<R>, view: &ViewModel) {
    if let Ok(mut slot) = app.state::<LastView>().0.lock() {
        *slot = Some(view.clone());
    }
    crate::mcp::keep_flock(view);
    let _ = app.emit("view", view);
}

#[derive(Debug)]
pub struct Inbox(mpsc::Sender<Msg>);

#[derive(Debug)]
enum Msg {
    Rules(Vec<core::Rule>),
    Policies(Vec<core::policies::Active>),
    Flock(core::flock::Flock),
    Outfit(core::looks::Outfit),
    Presence(core::Presence),
    Lang(core::i18n::Lang),
    Dnd(Option<Instant>),
    #[cfg_attr(not(target_os = "linux"), allow(dead_code))]
    Locked(bool),
    Project {
        cwd: String,
        prefs: core::ProjectPrefs,
    },
    Hook(Incoming),
    Connector(vults_connectors::Update),
    /// A paired phone's answer, its signature and key already checked (`phone`).
    Phone(vults_link::answer::Verified),
    User(Intent),
    Tick,
    /// `/continue` asks which sessions in a folder may be continued (W7); only the loop knows.
    Continuable {
        cwd: String,
        reply: tokio::sync::oneshot::Sender<Vec<core::handoff::Candidate>>,
    },
    /// A session's repository, read off the loop for the folder it was in (`Input::Repo`).
    Repo {
        session: core::SessionKey,
        cwd: String,
        repo: Option<core::Repo>,
    },
    /// The island asks for a step's whole diff; only the loop holds it.
    Diff {
        session: core::SessionKey,
        step: u32,
        reply: tokio::sync::oneshot::Sender<Option<core::Diff>>,
    },
}

/// How long a session's branch is trusted before its folder is read again on its next event: a
/// checkout shows up within it (a silent session's waits for its next event), and a session reads
/// its `.git` at most this often.
const REPO_FRESH: Duration = Duration::from_secs(60);

/// When a session's repository was last asked for, for which folder, and whether that reading is
/// still out.
type Readings = HashMap<core::SessionKey, (String, Instant, bool)>;

/// Reads the repository of a session just heard from, off the loop, when its folder is new to us
/// or the last reading is older than [`REPO_FRESH`]. Never a second one while one is out: a folder
/// that hangs (a stuck network share) holds one blocking thread, not one per minute.
fn read_repo(
    state: &core::State,
    repos: &mut Readings,
    key: core::SessionKey,
    now: Instant,
    tx: &mpsc::Sender<Msg>,
) {
    // A session core forgot (it ended) is read anew if it comes back.
    let Some(cwd) = state.sessions.get(&key).and_then(|s| s.cwd.clone()) else {
        repos.remove(&key);
        return;
    };
    if repos
        .get(&key)
        .is_some_and(|(at, when, out)| *out || (*at == cwd && now.duration_since(*when) < REPO_FRESH))
    {
        return;
    }
    repos.insert(key.clone(), (cwd.clone(), now, true));
    let tx = tx.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let repo = crate::roosts::repo_of(std::path::Path::new(&cwd)).map(|r| core::Repo {
            key: r.key(),
            name: r.name(),
            branch: r.branch,
        });
        let _ = tx.blocking_send(Msg::Repo {
            session: key,
            cwd,
            repo,
        });
    });
}

pub fn start(app: AppHandle) {
    let (tx, rx) = mpsc::channel(64);
    app.manage(Inbox(tx.clone()));
    app.manage(LastView::default());

    let (hooks_tx, mut hooks_rx) = mpsc::channel(64);
    tauri::async_runtime::spawn(async move {
        match Endpoint::for_current_user() {
            Ok(endpoint) => {
                if let Err(err) = vults_ipc::serve(endpoint, hooks_tx).await {
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
    let (phone_tx, mut phone_rx) = mpsc::channel(16);
    crate::phone::start(&app, phone_tx);
    let forward = tx.clone();
    tauri::async_runtime::spawn(async move {
        while let Some(answer) = phone_rx.recv().await {
            if forward.send(Msg::Phone(answer)).await.is_err() {
                break;
            }
        }
    });
    // Once a minute, so silent sessions and stale cards age out. One message a minute is
    // all it costs when nothing is going on.
    let ticker = tx.clone();
    tauri::async_runtime::spawn(async move {
        let mut every = tokio::time::interval(Duration::from_secs(60));
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
        state.policies = crate::policies::active();
        state.flock = s.flock;
        state.outfit = s.zeca_look;
        state.presence = s.presence;
        state.projects = s.projects.clone();
        state.dnd_until = crate::settings::dnd_instant(s.dnd_until);
        state.lang = crate::settings::resolve(s.language.as_deref());
    }
    if let Some(date) = today() {
        core::reduce(&mut state, Input::Today(date), Instant::now());
    }
    // Each paired phone's highest counter, so an answer replayed after a restart is still one.
    // Without them no phone is heard this run: a replay could pass.
    let counters_read = match vults_store::Store::open(&crate::history::dir()).and_then(|s| s.devices()) {
        Ok(devices) => {
            let counters = devices.into_iter().map(|d| (d.id, d.counter)).collect();
            core::reduce(&mut state, Input::DeviceCounters(counters), Instant::now());
            true
        }
        Err(e) => {
            tracing::warn!(error = %e, "paired devices not read: phone answers refused");
            false
        }
    };
    // The history, the audit log and the traces forget what is past their keep, once per start.
    tauri::async_runtime::spawn_blocking(|| {
        let today = crate::history::today();
        if let Err(e) = crate::history::prune(&crate::history::dir(), today) {
            tracing::warn!(error = %e, "history not pruned");
        }
        if let Err(e) = crate::audit::prune() {
            tracing::warn!(error = %e, "audit log not pruned");
        }
        if let Err(e) = crate::traces::prune() {
            tracing::warn!(error = %e, "traces not pruned");
        }
    });
    // A new season on every start: the flock draws its species anew.
    state.season = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos() as u64);
    let mut waiting: HashMap<RequestId, ReplyHandle> = HashMap::new();
    let mut last_view: Option<ViewModel> = None;
    let mut notifier = core::notify::Notifier::default();
    // The Monday card's week once looked at, with or without turns: the history is read once a week.
    let mut recap_looked: Option<core::looks::Date> = None;
    recap_due(&app, &mut state, &mut recap_looked);
    // When each session's repository was last read, and for which folder.
    let mut repos = Readings::new();

    while let Some(msg) = rx.recv().await {
        let now = Instant::now();
        let input = match msg {
            Msg::Hook(Incoming::Event(event)) => {
                tracing::debug!(agent = ?event.agent, event = %event.event, "hook");
                // The plan's usage, not a session's: it goes to the header, never to the core. The
                // session's cost goes to its card.
                if event.agent == vults_protocol::AgentKind::Claude
                    && event.event == vults_protocol::STATUS_LINE_EVENT
                {
                    let windows = vults_agents::usage::claude(&event.payload, crate::usage::epoch_now());
                    // Only a subscription reports its windows: its costs are at API prices.
                    let plan = if windows.is_empty() { "" } else { "subscription" };
                    if !windows.is_empty() {
                        crate::usage::set(&app, event.agent, windows);
                    }
                    let Some(usd) = vults_agents::usage::claude_cost(&event.payload) else {
                        continue;
                    };
                    let Some(id) = event.payload.get("session_id").and_then(|v| v.as_str()) else {
                        continue;
                    };
                    let session = core::SessionKey {
                        agent: event.agent,
                        session_id: id.to_string(),
                    };
                    // The day's spend (W9): what the session's total grew by. Only a session the
                    // flock knows, so a stray statusLine never adds to the user's spend.
                    if state
                        .sessions
                        .get(&session)
                        .is_some_and(|s| s.cost_usd != Some(usd))
                    {
                        let cwd = state.sessions.get(&session).and_then(|s| s.cwd.clone());
                        crate::spend::watched(&app, event.agent, id.to_string(), cwd, plan, usd);
                    }
                    Some(Input::Cost { session, usd })
                } else {
                    parse(&event)
                }
            }
            Msg::Hook(Incoming::Request {
                event,
                reply,
                received,
            }) => {
                tracing::info!(agent = ?event.agent, event = %event.event, "request for a human");
                let mut input = parse(&event);
                if let Some(Input::Agent(update)) = &mut input {
                    update.received = Some(received);
                }
                if is_card(&input) {
                    waiting.insert(RequestId(event.id.clone()), reply);
                } else {
                    // Nothing here can decide it (a question, an unknown event): the terminal asks now.
                    reply.decline();
                }
                input
            }
            Msg::Connector(vults_connectors::Update::Event(e)) => {
                tracing::info!(connector = %e.connector, level = ?e.level, "connector news");
                Some(Input::Connector(alert(e, state.lang)))
            }
            Msg::Connector(vults_connectors::Update::Board { connector, rows }) => Some(Input::Board {
                connector,
                rows: rows.map(|rows| rows.into_iter().map(row).collect()),
            }),
            Msg::User(intent) => Some(Input::User(intent)),
            // Its counter is saved before core sees it, so nothing it does (an Always's rule
            // included) can outlive a counter that was not kept. A counter spent on an answer the
            // ledger then refuses costs nothing.
            Msg::Phone(answer) if counters_read => {
                match crate::phone::save_counter(answer.device.clone(), answer.counter).await {
                    Ok(()) => Some(answer.input()),
                    Err(e) => {
                        tracing::warn!(error = %e, "phone answer dropped: its counter was not saved");
                        None
                    }
                }
            }
            Msg::Phone(_) => None,
            Msg::Rules(rules) => Some(Input::SetRules(rules)),
            Msg::Policies(list) => Some(Input::SetPolicies(list)),
            Msg::Flock(flock) => Some(Input::SetFlock(flock)),
            Msg::Outfit(outfit) => Some(Input::SetOutfit(outfit)),
            Msg::Presence(presence) => Some(Input::SetPresence(presence)),
            Msg::Lang(lang) => Some(Input::SetLang(lang)),
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
                recap_due(&app, &mut state, &mut recap_looked);
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
            Msg::Repo { session, cwd, repo } => {
                if let Some(r) = repos.get_mut(&session) {
                    r.2 = false;
                }
                Some(Input::Repo { session, cwd, repo })
            }
            Msg::Continuable { cwd, reply } => {
                let _ = reply.send(state.continuable(&cwd, now));
                continue;
            }
        };
        let Some(input) = input else { continue };
        let heard = match &input {
            Input::Agent(update) => Some(update.session.clone()),
            _ => None,
        };
        if matches!(input, Input::Tick) {
            repos.retain(|key, _| state.sessions.contains_key(key));
        }
        // Each step keeps the wall-clock time it started, which the core cannot read itself.
        if matches!(input, Input::Agent(_) | Input::Tick)
            && let Ok(d) = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
        {
            core::reduce(
                &mut state,
                Input::Clock(i64::try_from(d.as_secs()).unwrap_or(0)),
                now,
            );
        }

        let effects = core::reduce(&mut state, input, now);
        if let Some(key) = heard {
            read_repo(&state, &mut repos, key, now, &tx);
        }
        for effect in effects {
            match effect {
                Effect::AckPermission(id) => {
                    if let Some(h) = waiting.get(&id) {
                        h.ack();
                    }
                    // A tick at each step of the card's attention ladder (each reminder); and one when
                    // its offer ends, so the core drops it then. Both count from the card's `since`:
                    // when ipc read the hook's line.
                    let tick = tx.clone();
                    let since = state
                        .pending
                        .iter()
                        .find(|p| p.request == id)
                        .map_or_else(tokio::time::Instant::now, |p| {
                            tokio::time::Instant::from_std(p.since)
                        });
                    tauri::async_runtime::spawn(async move {
                        let start = since;
                        let steps = core::notify::ladder().into_iter().chain([core::PENDING_TTL]);
                        for at in steps {
                            tokio::time::sleep_until(start + at).await;
                            if tick.send(Msg::Tick).await.is_err() {
                                return;
                            }
                        }
                    });
                }
                Effect::RespondPermission { request, decision } => {
                    tracing::info!(?decision, "permission answered");
                    if let Some(h) = waiting.remove(&request) {
                        h.decide(decision);
                    }
                }
                Effect::AnswerQuestion { request, answers } => {
                    tracing::info!(count = answers.len(), "question answered");
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
                    // An open Settings lists them.
                    let _ = app.emit("settings", serde_json::json!({ "projects": projects }));
                }
                // The local history (docs/guide/activity.md): off the loop, a line and a day.
                Effect::Turn(turn) => {
                    // Zeca's journal (M4) has its own switch, apart from the history's.
                    crate::journal::turn(&app, &turn);
                    if crate::settings::history(&app) {
                        tauri::async_runtime::spawn_blocking(move || keep_turn(&turn));
                    }
                }
                Effect::Audit(line) => {
                    crate::journal::decision(&app, &line);
                    crate::audit::keep(line);
                }
                // Saved before the answer reached core (`Msg::Phone`).
                Effect::DeviceCounter { .. } => {}
                Effect::RecapSeen(monday) => {
                    if let Err(e) = crate::settings::edit(&app, |s| s.recap_shown_week = Some(monday)) {
                        tracing::warn!(error = %e, "recap card not saved as read");
                    }
                }
                Effect::JumpToTerminal(terminal) => {
                    // Shells out (herdr, tmux, gdbus…): off the loop. The island says so when
                    // there was nothing to try.
                    let app = app.clone();
                    tauri::async_runtime::spawn_blocking(move || {
                        #[cfg(target_os = "linux")]
                        let found = vults_platform::jump::jump(&terminal);
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
                Effect::MergeRow {
                    connector,
                    item,
                    head,
                } => {
                    tauri::async_runtime::spawn(crate::connectors::merge(app.clone(), connector, item, head));
                }
            }
        }

        crate::phone::sync(&app, &state, now);
        let view = state.view();
        if last_view.as_ref() != Some(&view) {
            publish_view(&app, &view);
            crate::tray::show(&app, &view);
            last_view = Some(view);
        }
        let prefs = core::notify::Prefs {
            on: crate::settings::notifications(&app),
        };
        crate::notify::send(&app, notifier.update(&state, prefs));
    }
}

/// Weighs the notifications again now (a setting changed), rather than at the next event.
pub fn recheck(app: &AppHandle) {
    if let Some(inbox) = app.try_state::<Inbox>() {
        let _ = inbox.0.try_send(Msg::Tick);
    }
}

fn alert(e: vults_connectors::Event, lang: core::i18n::Lang) -> core::Alert {
    use core::i18n::News;
    use vults_connectors::{Level, StoryKind};
    // The connector's English title, said in the user's language when it says what the news is.
    let title = match &e.story {
        Some(s) => core::i18n::news(
            lang,
            match s.kind {
                StoryKind::ReviewRequested => News::ReviewRequested,
                StoryKind::ChecksFailed => News::ChecksFailed,
                StoryKind::ChecksPassed => News::ChecksPassed,
                StoryKind::Approved => News::Approved,
                StoryKind::ChangesRequested => News::ChangesRequested,
            },
            &s.name,
            s.branch.as_deref(),
        ),
        None => e.title,
    };
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
        title,
        detail: e.detail,
        // Checked here, once: the UI can only ask to open an alert, never a URL.
        url: e.url.as_deref().and_then(core::SafeUrl::parse),
    }
}

fn row(r: vults_connectors::Row) -> core::board::Row {
    use core::board::{Checks, Group, Verdict};
    use vults_connectors as c;
    core::board::Row {
        item: r.item,
        group: match r.group {
            c::Group::Yours => Group::Yours,
            c::Group::ToReview => Group::ToReview,
            c::Group::Branches => Group::Branches,
            c::Group::Releases => Group::Releases,
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
        head: r.head,
        facts: facts(r.facts),
    }
}

fn facts(f: vults_connectors::Facts) -> core::board::Facts {
    use core::board::{CheckRun, CheckState, Release, Reviewer, RowDiff, Said};
    use vults_connectors as c;
    core::board::Facts {
        repo: f.repo,
        branch: f.branch,
        diff: f.diff.map(|d| RowDiff {
            added: d.added,
            removed: d.removed,
            files: d.files,
        }),
        author: f.author,
        reviewers: f
            .reviewers
            .into_iter()
            .map(|r| Reviewer {
                login: r.login,
                said: match r.said {
                    c::Said::Approved => Said::Approved,
                    c::Said::Changes => Said::Changes,
                    c::Said::Commented => Said::Commented,
                    c::Said::Waiting => Said::Waiting,
                },
            })
            .collect(),
        updated: f.updated,
        runs: f
            .runs
            .into_iter()
            .map(|r| CheckRun {
                name: r.name,
                state: match r.state {
                    c::RunState::Passed => CheckState::Passed,
                    c::RunState::Failed => CheckState::Failed,
                    c::RunState::Running => CheckState::Running,
                    c::RunState::Waiting => CheckState::Waiting,
                    c::RunState::Skipped => CheckState::Skipped,
                },
            })
            .collect(),
        release: f.release.map(|r| match r {
            c::Release::Latest => Release::Latest,
            c::Release::Prerelease => Release::Prerelease,
            c::Release::Draft => Release::Draft,
        }),
        command: f.command,
    }
}

/// A request a card can answer: a session's permission or question. Anything else, the chat's own
/// among them, is declined at once and the terminal asks.
fn is_card(input: &Option<Input>) -> bool {
    matches!(
        input,
        Some(Input::Agent(core::AgentUpdate {
            event: core::AgentEvent::PermissionRequested { .. } | core::AgentEvent::QuestionAsked { .. },
            ..
        }))
    )
}

fn parse(event: &vults_protocol::Event) -> Option<Input> {
    let update = vults_agents::parse(event)?;
    // The chat's own CLI (native mode loads the user's hooks): Zeca's work, never a session.
    Some(if event.zeca {
        Input::Chat(update)
    } else {
        Input::Agent(update)
    })
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
pub(crate) fn card_host(window: &tauri::Webview) -> bool {
    window.label() == ISLAND
}

/// Allow / Deny. The core ignores ids that are not the card on screen.
#[tauri::command]
pub async fn decide(
    window: tauri::Webview,
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
    window: tauri::Webview,
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
    window: tauri::Webview,
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
    window: tauri::Webview,
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

/// The always-allow rules, for Settings.
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

/// A request for a human that the app makes itself, for an agent it runs whose approvals come on
/// its own channel (a task's Codex, W4): the same path as a hook's request, so the same card,
/// rules, policies, phone and audit; its answer comes back here. No answer in a hook's time is a
/// decline.
pub async fn ask_human(app: &AppHandle, event: vults_protocol::Event) -> vults_ipc::Local {
    let (reply, answer) = vults_ipc::ReplyHandle::local();
    let request = Msg::Hook(Incoming::Request {
        event,
        reply,
        received: std::time::Instant::now(),
    });
    if app.state::<Inbox>().0.send(request).await.is_err() {
        return vults_ipc::Local::Decline;
    }
    answer
        .answer(vults_protocol::limits::SERVER_DECISION_TIMEOUT)
        .await
}

/// An event the app sends for an agent it runs, as its hook would (a task's Codex ending: its cards
/// still on the island are withdrawn).
pub async fn hook_event(app: &AppHandle, event: vults_protocol::Event) {
    let _ = app
        .state::<Inbox>()
        .0
        .send(Msg::Hook(Incoming::Event(event)))
        .await;
}

/// The sessions in `cwd` that may be continued (`handoff`), the latest first.
pub async fn continuable(app: &AppHandle, cwd: String) -> Vec<core::handoff::Candidate> {
    let (reply, answer) = tokio::sync::oneshot::channel();
    if app
        .state::<Inbox>()
        .0
        .send(Msg::Continuable { cwd, reply })
        .await
        .is_err()
    {
        return Vec::new();
    }
    answer.await.unwrap_or_default()
}

/// Hands the core the policies that stand now (`policies`).
pub async fn set_policies(app: &AppHandle, list: Vec<core::policies::Active>) -> Result<(), String> {
    app.state::<Inbox>()
        .0
        .send(Msg::Policies(list))
        .await
        .map_err(|_| "the app is busy".to_string())
}

/// The project choices, for Settings.
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
    agent: vults_protocol::AgentKind,
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

/// Settings → Activity: the week of `monday` (this week's when none), the weeks there are, and
/// the grid.
#[tauri::command]
pub async fn activity(app: AppHandle, monday: Option<String>) -> Result<crate::history::Activity, String> {
    let history = crate::settings::history(&app);
    let lang = crate::settings::lang(&app);
    tauri::async_runtime::spawn_blocking(move || {
        crate::history::activity(
            &crate::history::dir(),
            history,
            crate::history::today(),
            monday.as_deref(),
            lang,
        )
    })
    .await
    .map_err(|e| e.to_string())
}

/// Removes the local history: every turn and day, at once.
#[tauri::command]
pub async fn clear_history(app: AppHandle) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(|| crate::history::clear(&crate::history::dir()))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| format!("can't clear the history: {e}"))?;
    // Its saved record is gone with the rest: the chat's next message starts a new one.
    crate::chat::forget_current(&app);
    Ok(())
}

/// A quick action: the session's project's bird, or none to go back to the pool's draw.
#[tauri::command]
pub async fn session_project_bird(
    agent: vults_protocol::AgentKind,
    id: String,
    species: Option<String>,
    inbox: tauri::State<'_, Inbox>,
) -> Result<(), ()> {
    let session = core::SessionKey {
        agent,
        session_id: id,
    };
    inbox
        .0
        .send(Msg::User(Intent::SetProjectBird { session, species }))
        .await
        .map_err(|_| ())
}

/// The answer to a quiet bird: only its flag changes; nothing reaches the agent.
#[tauri::command]
pub async fn session_hush(
    agent: vults_protocol::AgentKind,
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
    sent.map_err(|_| "the app is busy".to_string())?;
    // Every page that shows it (Settings, the island's gear) follows.
    let _ = app.emit("settings", serde_json::json!({ "flock": flock }));
    Ok(())
}

/// The screen locked or unlocked (`lock`): waits for room rather than lose an unlock, and keeps
/// the order of the changes.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub async fn set_locked(app: &AppHandle, locked: bool) {
    let Some(inbox) = app.try_state::<Inbox>().map(|i| i.0.clone()) else {
        return;
    };
    let _ = inbox.send(Msg::Locked(locked)).await;
}

/// Last week's recap card read, or opened in Settings: it goes, and never comes back.
#[tauri::command]
pub async fn recap_dismiss(inbox: tauri::State<'_, Inbox>) -> Result<(), ()> {
    inbox
        .0
        .send(Msg::User(Intent::DismissRecap))
        .await
        .map_err(|_| ())
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

/// The language changed: the core speaks it from the next view on.
pub fn set_lang(app: &AppHandle, lang: core::i18n::Lang) {
    let _ = app.state::<Inbox>().0.try_send(Msg::Lang(lang));
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
/// On Monday from 08:00, last week's recap, once: the core gets it if last week had a turn and its
/// card was never read. The history is read at most once for each week.
fn recap_due(app: &AppHandle, state: &mut core::State, looked: &mut Option<core::looks::Date>) {
    let now = crate::history::unix_now();
    let shown = crate::settings::recap_shown_week(app);
    let Some(monday) = core::recap::card_due(now, crate::history::local_offset(now), shown.as_deref()) else {
        return;
    };
    if *looked == Some(monday) {
        return;
    }
    *looked = Some(monday);
    let entries = crate::history::turns(&crate::history::dir());
    let week = core::recap::week(state.lang, &entries, monday);
    if week.turns > 0 {
        core::reduce(
            state,
            Input::Recap {
                monday: week.monday,
                headline: week.headline,
            },
            Instant::now(),
        );
    }
}

fn keep_turn(turn: &core::turns::Turn) {
    let now = crate::history::unix_now();
    let record = crate::history::Record::new(turn, now, crate::history::local_offset(now));
    if let Err(e) = crate::history::append(&crate::history::dir(), &record) {
        tracing::warn!(error = %e, "turn not kept in the history");
    }
}

fn today() -> Option<core::looks::Date> {
    #[cfg(target_os = "linux")]
    return vults_platform::linux::today().map(|(y, m, d)| core::looks::Date::new(y, m, d));
    #[cfg(not(target_os = "linux"))]
    None
}

/// A click on a session row: bring its terminal forward.
#[tauri::command]
pub async fn session_jump(
    agent: vults_protocol::AgentKind,
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
    agent: Option<vults_protocol::AgentKind>,
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
    agent: vults_protocol::AgentKind,
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
    agent: vults_protocol::AgentKind,
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
    agent: vults_protocol::AgentKind,
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

/// *Squash and merge* confirmed on a row: the core checks it is one the card offers.
#[tauri::command]
pub async fn board_merge(
    connector: String,
    item: String,
    window: tauri::Webview,
    inbox: tauri::State<'_, Inbox>,
) -> Result<(), ()> {
    // A click in the island or the Nest, never another page's request.
    if window.label() != ISLAND && window.label() != crate::NEST {
        return Err(());
    }
    inbox
        .0
        .send(Msg::User(Intent::MergeRow { connector, item }))
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
/// Tall enough for the open island at its zoom (ui render.ts OPEN_ZOOM: 556 x 1.14).
pub const ISLAND_SIZE: (i32, i32) = (720, 640);

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
            use vults_platform::linux::{Rect, set_input_region};
            set_input_region(&gtk, window.label(), Some(Rect { x, y, width, height }));
        }
        #[cfg(not(target_os = "linux"))]
        let _ = (window, x, y, width, height);
    });
}

/// The island's chat needs the keyboard; everything else must never take it from the user's
/// terminal. Asked by the window itself.
#[tauri::command]
pub fn surface_keyboard(app: AppHandle, window: tauri::WebviewWindow, on: bool, exclusive: Option<bool>) {
    if !crate::SURFACES.contains(&window.label()) {
        return;
    }
    let _ = app.run_on_main_thread(move || {
        #[cfg(target_os = "linux")]
        if let Ok(gtk) = window.gtk_window() {
            vults_platform::linux::set_keyboard(&gtk, on, exclusive.unwrap_or(false));
        }
        // Only a layer-shell surface has an exclusive mode.
        #[cfg(not(target_os = "linux"))]
        let _ = exclusive;
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

    #[test]
    fn the_chats_own_request_is_never_a_card() {
        let event = |zeca: bool| vults_protocol::Event {
            v: vults_protocol::VERSION,
            id: "r".into(),
            agent: vults_protocol::AgentKind::Claude,
            agent_name: None,
            event: "PermissionRequest".into(),
            wants_reply: true,
            zeca,
            terminal: Default::default(),
            payload: serde_json::json!({
                "hook_event_name": "PermissionRequest",
                "session_id": "s",
                "tool_name": "Bash",
                "tool_input": { "command": "ls" },
            }),
        };
        let session = parse(&event(false));
        assert!(matches!(session, Some(Input::Agent(_))) && is_card(&session));
        // Even one that says it waits (a hand-written line): declined, the terminal asks.
        let chat = parse(&event(true));
        assert!(matches!(chat, Some(Input::Chat(_))) && !is_card(&chat));
    }
}
