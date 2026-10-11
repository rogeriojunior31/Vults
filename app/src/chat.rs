//! Chat from the island, and the files dropped on it.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use vults_core::audit::{Act, Actor, Audit};

use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::{Mutex, mpsc, oneshot};
use vults_chat::providers::{self, Provider as ApiProvider};
use vults_chat::{Approver, Chat, Delta, Provider, Turn};
use vults_secrets as secrets;
use vults_store::conversations::{self, ConversationId};

use crate::settings::{self, SettingsState};
use crate::{ISLAND, NEST, paths};

/// Dropped files bigger than this are refused: the model would not read them whole anyway.
const MAX_FILE: u64 = 20 * 1024 * 1024;
/// Inbox copies older than this are deleted at start-up.
const INBOX_KEEP: Duration = Duration::from_secs(7 * 24 * 3600);

/// One conversation at a time; a turn holds the lock until it ends. Permissions the turn is
/// waiting on live outside that lock, so a click can answer them meanwhile.
#[derive(Debug)]
pub struct ChatState {
    chat: Mutex<Chat>,
    waiting: Arc<Waiting>,
    /// Stops the turn running now, if one is.
    stop: std::sync::Mutex<Option<oneshot::Sender<()>>>,
    /// The saved conversation the chat is in; none until its first message.
    current: std::sync::Mutex<Option<ConversationId>>,
}

/// A card the chat waits on: its answer's channel, and what it asks (for the audit log).
#[derive(Debug)]
struct Asked {
    answer: oneshot::Sender<vults_chat::Answer>,
    tool: String,
    target: String,
    /// When the card came up (Unix ms), and the turn it came in, for the tool's trace span.
    since_ms: i64,
    turn: Option<Arc<crate::traces::Turn>>,
}

/// The cards waiting, and the turn running now: a card keeps the turn it came in, so a late click
/// never files it under the next one.
#[derive(Debug, Default)]
struct Waiting(
    std::sync::Mutex<HashMap<String, Asked>>,
    std::sync::Mutex<Option<Arc<crate::traces::Turn>>>,
);

impl Waiting {
    /// Every card still waiting is closed unanswered (a no, for the CLI); each gets its span.
    fn close_all(&self) {
        let closed: Vec<Asked> = self
            .0
            .lock()
            .map(|mut m| m.drain().map(|(_, a)| a).collect())
            .unwrap_or_default();
        for a in closed {
            if let Some(turn) = &a.turn {
                crate::traces::tool(turn, &a.tool, &a.target, a.since_ms, "stopped");
            }
        }
    }
}

impl Approver for Waiting {
    fn wait(&self, id: &str, tool: &str, target: &str) -> oneshot::Receiver<vults_chat::Answer> {
        let (answer, rx) = oneshot::channel();
        if let Ok(mut map) = self.0.lock() {
            let asked = Asked {
                answer,
                tool: tool.to_owned(),
                target: target.to_owned(),
                since_ms: crate::traces::now_ms(),
                turn: self.1.lock().ok().and_then(|t| t.clone()),
            };
            map.insert(id.to_string(), asked);
        }
        rx
    }

    /// The user's rule for this conversation answered: on the record as the rule's, with its span.
    fn ruled(&self, tool: &str, target: &str) {
        crate::audit::keep(Audit::new(Actor::Rule, Act::Allow, "zeca", "", tool, target));
        if let Some(turn) = self.1.lock().ok().and_then(|t| t.clone()) {
            let now = crate::traces::now_ms();
            crate::traces::tool(&turn, tool, target, now, "ok");
        }
    }
}

impl ChatState {
    pub fn new() -> Self {
        Self {
            chat: Mutex::new(Chat::new(Provider::Claude, paths::chat_dir())),
            waiting: Arc::default(),
            stop: std::sync::Mutex::new(None),
            current: std::sync::Mutex::new(None),
        }
    }

    fn current(&self) -> Option<ConversationId> {
        self.current.lock().ok().and_then(|c| *c)
    }

    fn set_current(&self, id: Option<ConversationId>) {
        if let Ok(mut c) = self.current.lock() {
            *c = id;
        }
    }
}

/// To both of the chat's windows: the island and the Nest (if it is open).
fn emit_chat<S: serde::Serialize + Clone>(app: &AppHandle, event: &str, payload: &S) {
    let _ = app.emit_to(ISLAND, event, payload.clone());
    let _ = app.emit_to(NEST, event, payload.clone());
}

/// What one of the chat's windows did, for the other to show too (`chat-sync`): each window
/// skips its own.
#[derive(serde::Serialize, Clone)]
#[serde(tag = "kind", rename_all = "lowercase")]
enum Sync {
    Sent {
        from: String,
        text: String,
        files: Vec<String>,
    },
    Reset {
        from: String,
        provider: Provider,
    },
    Opened {
        from: String,
        conversation: Opened,
    },
    Answered {
        from: String,
        id: String,
        answer: ChatAnswer,
    },
}

fn sync(app: &AppHandle, s: Sync) {
    emit_chat(app, "chat-sync", &s);
}

/// Streams the reply to the island as `chat` events: text, permissions to ask, then done or
/// error. `folder` (the focused session's project) is where a new conversation works.
#[tauri::command]
pub async fn chat_send(
    app: AppHandle,
    window: tauri::WebviewWindow,
    text: String,
    files: Vec<String>,
    folder: Option<String>,
) -> Result<(), String> {
    if !crate::settings::zeca(&app) {
        return Err("Zeca is off (Settings → Flock).".into());
    }
    // Only our own inbox copies may be attached, never an arbitrary path from the webview.
    let inbox = paths::inbox_dir();
    let files: Vec<PathBuf> = files
        .into_iter()
        .map(PathBuf::from)
        .filter(|f| f.starts_with(&inbox))
        .collect();
    let (tx, mut rx) = mpsc::channel::<Delta>(64);
    // What the turn cost and how it ended, for its trace span; the usage never reaches the chat.
    let ended = Arc::new(std::sync::Mutex::new((None::<vults_chat::Usage>, "ok")));
    // What Zeca said, and the error if one came, for the saved conversation.
    let said = Arc::new(std::sync::Mutex::new((String::new(), None::<String>)));
    let forward = {
        let app = app.clone();
        let ended = ended.clone();
        let said = said.clone();
        tauri::async_runtime::spawn(async move {
            while let Some(delta) = rx.recv().await {
                if let Ok(mut s) = said.lock() {
                    match &delta {
                        Delta::Text { text } => s.0.push_str(text),
                        Delta::Error { message } => s.1 = Some(message.clone()),
                        _ => {}
                    }
                }
                if let Ok(mut e) = ended.lock() {
                    match &delta {
                        Delta::Usage(u) => {
                            e.0 = Some(u.clone());
                            continue;
                        }
                        Delta::Error { .. } => e.1 = "error",
                        Delta::Stopped => e.1 = "stopped",
                        _ => {}
                    }
                }
                emit_chat(&app, "chat", &delta);
            }
        })
    };
    let (api, model) = api_choice(&app);
    let state = app.state::<ChatState>();
    let mut chat = state.chat.lock().await;
    // Turned off while this turn waited for the lock: `stop` found no turn to end.
    if !crate::settings::zeca(&app) {
        return Err("Zeca is off (Settings → Flock).".into());
    }
    // A conversation keeps the provider and model it began with, as a CLI keeps its pick.
    if !chat.started() {
        chat.set_api(api, model);
    }
    // Ignored once the conversation has started: it stays where it began. None is the chat's own
    // folder, so a pick made for it (and shown) is the one that runs.
    let dir = folder
        .map(PathBuf::from)
        .filter(|d| d.is_absolute())
        .unwrap_or_else(|| {
            let own = paths::chat_dir();
            let _ = std::fs::create_dir_all(&own);
            own
        });
    chat.set_folder(dir);
    if !chat.started() {
        let pick = pick_for(&app, chat.folder(), chat.provider());
        let effort = pick.effort.as_deref().and_then(vults_chat::Effort::parse);
        let effort = effort.filter(|e| effort_fits(chat.provider(), pick.model.as_deref(), *e));
        chat.set_pick(pick.model, effort);
        let native = settings::native_chat(&app);
        let dir = chat.folder().to_path_buf();
        let state = vults_agents::claude_state_file(&paths::agent_dirs());
        let read = tauri::async_runtime::spawn_blocking(move || cli_settings(native, &dir, &state));
        chat.set_settings(read.await.unwrap_or_default());
    }
    let names: Vec<String> = files
        .iter()
        .filter_map(|f| f.file_name().map(|n| n.to_string_lossy().into_owned()))
        .collect();
    let conversation = match state.current() {
        // Moved before its first reply (a failed first turn): the record follows.
        Some(id) if !chat.started() => {
            let new = new_conversation(&chat, &text, &names);
            let _ = store(move |s| s.conversation_restart(id, &new)).await;
            Some(id)
        }
        Some(id) => Some(id),
        None => {
            let new = new_conversation(&chat, &text, &names);
            let made = tauri::async_runtime::spawn_blocking(move || {
                vults_store::Store::open(&crate::history::dir())
                    .and_then(|mut s| s.conversation_new(&new, crate::traces::now_ms()))
            })
            .await;
            match made {
                Ok(Ok(id)) => {
                    state.set_current(Some(id));
                    Some(id)
                }
                // The chat still works; it just won't be there after a restart.
                _ => {
                    tracing::warn!("conversation not saved");
                    None
                }
            }
        }
    };
    sync(
        &app,
        Sync::Sent {
            from: window.label().to_string(),
            text: text.clone(),
            files: files.iter().map(|f| f.to_string_lossy().into_owned()).collect(),
        },
    );
    if let Some(id) = conversation {
        let now = crate::traces::now_ms();
        crate::writer::send(crate::writer::Job::Say(id, "you", text.clone(), names, now));
    }
    let approver: Arc<dyn Approver> = state.waiting.clone();
    let (stop_tx, stop) = oneshot::channel();
    if let Ok(mut s) = state.stop.lock() {
        *s = Some(stop_tx);
    }
    tracing::info!(provider = ?chat.provider(), files = files.len(), "chat turn");
    let (provider, model) = chat.engine();
    let turn = Arc::new(crate::traces::Turn::start(provider, model));
    if let Ok(mut t) = state.waiting.1.lock() {
        *t = Some(turn.clone());
    }
    chat.send(Turn { text, files }, tx, approver, stop).await;
    if let Ok(mut s) = state.stop.lock() {
        *s = None;
    }
    let (session, history) = (chat.session(), chat.history());
    drop(chat);
    let _ = forward.await;
    if let Some(id) = conversation {
        let now = crate::traces::now_ms();
        let (reply, error) = said.lock().map(|s| s.clone()).unwrap_or_default();
        let stopped = ended.lock().is_ok_and(|e| e.1 == "stopped");
        for (who, text) in [
            ("zeca", Some(reply).filter(|r| !r.is_empty())),
            ("error", error),
            ("stopped", stopped.then(String::new)),
        ] {
            if let Some(text) = text {
                crate::writer::send(crate::writer::Job::Say(id, who, text, Vec::new(), now));
            }
        }
        crate::writer::send(crate::writer::Job::Save(id, session, history, now));
    }
    if let Ok(mut t) = state.waiting.1.lock() {
        *t = None;
    }
    if let Ok(e) = ended.lock() {
        turn.end(e.0.as_ref(), e.1);
    }
    Ok(())
}

/// What the chat's CLI reads: nothing of the user's, their own setup, or with it the project's
/// where Claude Code already trusts the folder (as `claude` there in a terminal would).
/// `state` is Claude Code's state file ([`vults_agents::claude_state_file`]).
fn cli_settings(native: bool, dir: &Path, state: &Path) -> vults_chat::Settings {
    use vults_chat::Settings;
    if !native {
        return Settings::Isolated;
    }
    let state = std::fs::read(state)
        .ok()
        .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok());
    // Claude Code keys its projects by the real path.
    let dir = dir.canonicalize().unwrap_or_else(|_| dir.to_path_buf());
    if state.is_some_and(|s| vults_agents::claude_trusts(&s, &dir)) {
        Settings::Project
    } else {
        Settings::User
    }
}

/// Where a pick is kept: the folder, or `""` for the chat's own empty one.
fn folder_key(dir: Option<&Path>) -> String {
    match dir {
        Some(d) if d != paths::chat_dir() => d.to_string_lossy().into_owned(),
        _ => String::new(),
    }
}

fn engine_key(provider: Provider) -> Option<&'static str> {
    match provider {
        Provider::Claude => Some("claude"),
        Provider::Codex => Some("codex"),
        Provider::ClaudeApi => None,
    }
}

/// The model and effort a new conversation in `dir` starts with.
fn pick_for(app: &AppHandle, dir: &Path, provider: Provider) -> settings::Pick {
    let Some(engine) = engine_key(provider) else {
        return settings::Pick::default();
    };
    let state = app.state::<SettingsState>();
    let picks = state.0.lock().map(|s| s.chat_picks.clone()).unwrap_or_default();
    picks
        .get(&folder_key(Some(dir)))
        .and_then(|p| p.get(engine))
        .cloned()
        .unwrap_or_default()
}

/// Whether the model takes the effort, as far as its list says (a hand-edited or stale pick would
/// fail every turn); a model not listed, or a list not read yet, is let through to the CLI.
fn effort_fits(provider: Provider, model: Option<&str>, effort: vults_chat::Effort) -> bool {
    let Some(model) = model else { return true };
    let known = match provider {
        Provider::Claude => Some(vults_chat::claude_models()),
        Provider::Codex => CODEX_MODELS.lock().ok().and_then(|m| m.clone()),
        Provider::ClaudeApi => None,
    };
    known
        .and_then(|list| list.into_iter().find(|m| m.id == model))
        .is_none_or(|m| m.efforts.contains(&effort.name()))
}

/// A pick the settings may keep: an absolute folder, a model name the CLI can only read as one,
/// and a known effort.
fn check_pick(folder: Option<&str>, model: Option<&str>, effort: Option<&str>) -> Result<(), String> {
    if folder.is_some_and(|f| !Path::new(f).is_absolute()) {
        return Err("Unknown folder.".into());
    }
    if model.is_some_and(|m| !vults_chat::valid_model(m)) {
        return Err("Unknown model.".into());
    }
    if effort.is_some_and(|e| vults_chat::Effort::parse(e).is_none()) {
        return Err("Unknown effort.".into());
    }
    Ok(())
}

/// What the user's Codex lists, asked once per run: each ask starts an app-server.
static CODEX_MODELS: std::sync::Mutex<Option<Vec<vults_chat::ModelChoice>>> = std::sync::Mutex::new(None);

/// The models the chat's CLI offers: Claude Code's aliases, or the user's Codex's own list (asked
/// when the picker first opens with Codex; the codex they logged into fetches it).
#[tauri::command]
pub async fn chat_models(provider: Provider) -> Result<Vec<vults_chat::ModelChoice>, String> {
    match provider {
        Provider::Claude => Ok(vults_chat::claude_models()),
        Provider::Codex => {
            if let Some(known) = CODEX_MODELS.lock().ok().and_then(|m| m.clone()) {
                return Ok(known);
            }
            let models = vults_chat::codex_models(&paths::chat_dir()).await?;
            if let Ok(mut m) = CODEX_MODELS.lock() {
                *m = Some(models.clone());
            }
            Ok(models)
        }
        Provider::ClaudeApi => Ok(Vec::new()),
    }
}

/// The picks for a folder (none: the chat's own), by engine.
#[tauri::command]
pub fn chat_pick(
    app: AppHandle,
    folder: Option<String>,
) -> std::collections::BTreeMap<String, settings::Pick> {
    let key = folder_key(folder.as_deref().map(Path::new));
    let state = app.state::<SettingsState>();
    state
        .0
        .lock()
        .ok()
        .and_then(|s| s.chat_picks.get(&key).cloned())
        .unwrap_or_default()
}

/// Remembers the model and effort for new conversations in `folder` with `provider`.
#[tauri::command]
pub fn set_chat_pick(
    app: AppHandle,
    folder: Option<String>,
    provider: Provider,
    model: Option<String>,
    effort: Option<String>,
) -> Result<(), String> {
    let engine = engine_key(provider).ok_or("The API chat's model is chosen in Settings → Chat.")?;
    check_pick(folder.as_deref(), model.as_deref(), effort.as_deref())?;
    let key = folder_key(folder.as_deref().map(Path::new));
    settings::edit(&app, |s| {
        let picks = s.chat_picks.entry(key.clone()).or_default();
        if model.is_none() && effort.is_none() {
            picks.remove(engine);
        } else {
            picks.insert(engine.to_string(), settings::Pick { model, effort });
        }
        if picks.is_empty() {
            s.chat_picks.remove(&key);
        }
    })
}

/// A click on the chat's card.
#[derive(Debug, Clone, Copy, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ChatAnswer {
    Deny,
    Allow,
    /// *Always in this conversation*: this whole command again, until the conversation ends.
    Always,
}

/// The user's answer to a permission card. Only a click calls this.
#[tauri::command]
pub fn chat_decide(
    window: tauri::WebviewWindow,
    state: tauri::State<'_, ChatState>,
    id: String,
    answer: ChatAnswer,
) {
    // Like an agent's card, the chat's is only answered on the island.
    if !crate::runtime::card_host(&window) {
        return;
    }
    tracing::info!(?answer, "chat permission answered");
    let asked = state.waiting.0.lock().ok().and_then(|mut m| m.remove(&id));
    if let Some(asked) = asked {
        let (reply, act) = match answer {
            ChatAnswer::Deny => (vults_chat::Answer::Deny, Act::Deny),
            ChatAnswer::Allow => (vults_chat::Answer::Allow, Act::Allow),
            ChatAnswer::Always => (vults_chat::Answer::AllowInConversation, Act::AllowInConversation),
        };
        let allow = reply.allows();
        let _ = asked.answer.send(reply);
        sync(
            window.app_handle(),
            Sync::Answered {
                from: window.label().to_string(),
                id: id.clone(),
                answer,
            },
        );
        // The chat's own CLI asked: Zeca's, not one of the flock's sessions.
        crate::audit::keep(Audit::new(
            Actor::Human,
            act,
            "zeca",
            "",
            &asked.tool,
            &asked.target,
        ));
        if let Some(turn) = &asked.turn {
            let status = if allow { "ok" } else { "denied" };
            crate::traces::tool(turn, &asked.tool, &asked.target, asked.since_ms, status);
        }
    }
}

/// A card asked in the Nest is answered on the island (ADR 0008): this brings the island's chat up.
#[tauri::command]
pub fn chat_to_island(app: AppHandle) {
    if crate::settings::zeca(&app) {
        let _ = app.emit_to(ISLAND, "open-chat", ());
    }
}

/// Stop on the chat: ends the turn running now. Whatever it was waiting on is a no.
#[tauri::command]
pub fn chat_stop(state: tauri::State<'_, ChatState>) {
    end_turn(&state);
}

/// Zeca switched off: ends the turn running now, then drops the conversation, and with it the
/// Codex app-server a chat may keep alive.
pub fn stop(app: &AppHandle) {
    end_turn(&app.state::<ChatState>());
    // The Nest is Zeca's window: it goes with him.
    if let Some(nest) = app.get_webview_window(NEST) {
        let _ = nest.close();
    }
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let state = app.state::<ChatState>();
        let mut chat = state.chat.lock().await;
        let provider = chat.provider();
        *chat = Chat::new(provider, paths::chat_dir());
        state.set_current(None);
    });
}

fn end_turn(state: &ChatState) {
    tracing::info!("chat turn stopped");
    state.waiting.close_all();
    if let Some(tx) = state.stop.lock().ok().and_then(|mut s| s.take()) {
        let _ = tx.send(());
    }
}

/// The saved record of the chat's conversation was removed (Clear history): the next message
/// saves it anew rather than into a record that is gone.
pub fn forget_current(app: &AppHandle) {
    app.state::<ChatState>().set_current(None);
}

/// A new conversation, optionally with the other provider. The one before stays saved.
#[tauri::command]
pub async fn chat_reset(
    window: tauri::WebviewWindow,
    state: tauri::State<'_, ChatState>,
    provider: Option<Provider>,
) -> Result<Provider, ()> {
    // Anything still waiting is a no: that conversation is over.
    state.waiting.close_all();
    let mut chat = state.chat.lock().await;
    let provider = provider.unwrap_or(chat.provider());
    *chat = Chat::new(provider, paths::chat_dir());
    state.set_current(None);
    let from = window.label().to_string();
    sync(window.app_handle(), Sync::Reset { from, provider });
    Ok(provider)
}

/// The conversation the chat is in now, as saved, for a window that opens on it (the Nest).
#[tauri::command]
pub async fn chat_current(state: tauri::State<'_, ChatState>) -> Result<Option<Opened>, String> {
    let Some(id) = state.current() else {
        return Ok(None);
    };
    let started = state.chat.lock().await.started();
    let (c, lines) = store(move |s| Ok((s.conversation(id)?, s.conversation_messages(id)?))).await?;
    Ok(c.and_then(|c| opened(c, lines, started)))
}

/// A saved conversation in the list.
#[derive(serde::Serialize)]
pub struct ConversationView {
    id: i64,
    title: String,
    provider: Provider,
    /// Who it talks to: Claude, Codex, or the API provider it was pinned to.
    label: String,
    /// The folder's name, or none for the chat's own.
    folder: Option<String>,
    #[serde(rename = "updatedMs")]
    updated_ms: i64,
    /// The one the chat is in now.
    current: bool,
}

/// One line of a saved conversation, as the chat shows it.
#[derive(serde::Serialize, Clone)]
pub struct LineView {
    who: String,
    text: String,
    files: Vec<String>,
}

/// A saved conversation, opened: what the chat needs to go on with it.
#[derive(serde::Serialize, Clone)]
pub struct Opened {
    provider: Provider,
    label: String,
    folder: Option<String>,
    started: bool,
    model: Option<String>,
    effort: Option<String>,
    lines: Vec<LineView>,
}

/// `claude`, `codex`, or `api:<provider>`: what a saved conversation runs on.
fn engine_name(chat: &Chat) -> String {
    match chat.provider() {
        Provider::Claude => "claude".into(),
        Provider::Codex => "codex".into(),
        Provider::ClaudeApi => format!("api:{}", chat.api().0.id),
    }
}

/// The name the chat shows for a saved conversation's engine.
fn engine_label(engine: &str) -> String {
    match engine {
        "claude" => "Claude".into(),
        "codex" => "Codex".into(),
        e => e
            .strip_prefix("api:")
            .and_then(providers::find)
            .map_or_else(|| e.to_string(), |p| p.label.to_string()),
    }
}

fn engine_provider(engine: &str) -> Option<Provider> {
    match engine {
        "claude" => Some(Provider::Claude),
        "codex" => Some(Provider::Codex),
        e if e.starts_with("api:") => Some(Provider::ClaudeApi),
        _ => None,
    }
}

fn settings_name(s: vults_chat::Settings) -> &'static str {
    match s {
        vults_chat::Settings::Isolated => "isolated",
        vults_chat::Settings::User => "user",
        vults_chat::Settings::Project => "project",
    }
}

fn settings_of(name: &str) -> vults_chat::Settings {
    match name {
        "user" => vults_chat::Settings::User,
        "project" => vults_chat::Settings::Project,
        _ => vults_chat::Settings::Isolated,
    }
}

/// The record a conversation starts with, from its first message (or its files' names).
fn new_conversation(chat: &Chat, text: &str, files: &[String]) -> conversations::New {
    let (model, effort) = chat.pick();
    let model = match chat.provider() {
        Provider::ClaudeApi => Some(chat.api().1.to_string()),
        _ => model.map(str::to_string),
    };
    conversations::New {
        engine: engine_name(chat),
        folder: folder_key(Some(chat.folder())),
        title: if text.trim().is_empty() {
            files.join(", ")
        } else {
            text.to_string()
        },
        model,
        effort: effort.map(|e| e.name().to_string()),
        settings: settings_name(chat.settings()).into(),
    }
}

/// One thing done with the database, off the async runtime.
async fn store<T: Send + 'static>(
    work: impl FnOnce(&mut vults_store::Store) -> Result<T, vults_store::Error> + Send + 'static,
) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(move || {
        vults_store::Store::open(&crate::history::dir()).and_then(|mut s| work(&mut s))
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())
}

/// The saved conversations, the latest used first.
#[tauri::command]
pub async fn chat_conversations(state: tauri::State<'_, ChatState>) -> Result<Vec<ConversationView>, String> {
    let current = state.current();
    let list = store(|s| s.conversations(50)).await?;
    Ok(list
        .into_iter()
        .filter_map(|c| {
            Some(ConversationView {
                provider: engine_provider(&c.engine)?,
                label: engine_label(&c.engine),
                current: current == Some(c.id),
                id: c.id.0,
                title: c.title,
                folder: (!c.folder.is_empty()).then_some(c.folder),
                updated_ms: c.updated_ms,
            })
        })
        .collect())
}

/// Goes on with a saved conversation: the chat becomes that one, as it was.
#[tauri::command]
pub async fn chat_open(app: AppHandle, window: tauri::WebviewWindow, id: i64) -> Result<Opened, String> {
    let id = ConversationId(id);
    let (c, lines) = store(move |s| Ok((s.conversation(id)?, s.conversation_messages(id)?))).await?;
    let c = c.ok_or("That conversation is gone.")?;
    let provider = engine_provider(&c.engine).ok_or("That conversation's engine is unknown.")?;
    let dir = if c.folder.is_empty() {
        paths::chat_dir()
    } else {
        PathBuf::from(&c.folder)
    };
    // Never brought back: a folder the user removed stays removed.
    if !c.folder.is_empty() && !dir.is_dir() {
        return Err(format!("That conversation's folder is gone: {}", c.folder));
    }
    let api = c
        .engine
        .strip_prefix("api:")
        .map(|p| {
            self::provider(p).map(|p| (p, c.model.clone().unwrap_or_else(|| p.default_model.to_string())))
        })
        .transpose()?;
    let spec = vults_agents::driver::Spec {
        model: c
            .model
            .clone()
            .filter(|m| vults_chat::valid_model(m) && api.is_none()),
        effort: c.effort.as_deref().and_then(vults_chat::Effort::parse),
        settings: settings_of(&c.settings),
        ..vults_agents::driver::Spec::in_dir(dir)
    };
    // A session id reaches Claude Code as `--resume`'s value: never anything that reads as a flag.
    let session = c.session.clone().filter(|s| valid_session(s));
    let state = app.state::<ChatState>();
    state.waiting.close_all();
    let mut chat = state.chat.lock().await;
    *chat = Chat::resume(provider, spec, session, c.history.as_deref(), api);
    state.set_current(Some(c.id));
    let conversation = opened(c, lines, chat.started()).ok_or("That conversation's engine is unknown.")?;
    let from = window.label().to_string();
    sync(
        &app,
        Sync::Opened {
            from,
            conversation: conversation.clone(),
        },
    );
    Ok(conversation)
}

/// A saved conversation as the chat shows it.
fn opened(
    c: conversations::Conversation,
    lines: Vec<conversations::Message>,
    started: bool,
) -> Option<Opened> {
    Some(Opened {
        provider: engine_provider(&c.engine)?,
        label: engine_label(&c.engine),
        folder: (!c.folder.is_empty()).then_some(c.folder),
        started,
        model: c.model,
        effort: c.effort,
        lines: lines
            .into_iter()
            .map(|l| LineView {
                who: l.who,
                text: l.text,
                files: l.files,
            })
            .collect(),
    })
}

/// A CLI's session or thread id: letters, digits, `-` and `_`, never first a `-`.
fn valid_session(id: &str) -> bool {
    (1..=100).contains(&id.len())
        && !id.starts_with('-')
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

#[tauri::command]
pub async fn chat_rename(id: i64, title: String) -> Result<(), String> {
    store(move |s| s.conversation_rename(ConversationId(id), &title)).await
}

/// Deletes a saved conversation; if the chat is in it, the chat starts a new one. True then.
#[tauri::command]
pub async fn chat_delete(
    window: tauri::WebviewWindow,
    state: tauri::State<'_, ChatState>,
    id: i64,
) -> Result<bool, String> {
    let id = ConversationId(id);
    let was_current = state.current() == Some(id);
    if was_current {
        state.waiting.close_all();
        let mut chat = state.chat.lock().await;
        let provider = chat.provider();
        *chat = Chat::new(provider, paths::chat_dir());
        state.set_current(None);
        let from = window.label().to_string();
        sync(window.app_handle(), Sync::Reset { from, provider });
    }
    store(move |s| s.conversation_delete(id)).await?;
    Ok(was_current)
}

/// The API chat as the island sees it: usable now (a key saved, or a local server), and by whom.
#[derive(serde::Serialize, Clone)]
pub struct ApiStatus {
    ready: bool,
    label: &'static str,
}

/// One provider in Settings → Chat. Whether a key is saved, never the key itself.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiProviderView {
    #[serde(flatten)]
    provider: &'static ApiProvider,
    has_key: bool,
    model: String,
}

#[derive(serde::Serialize)]
pub struct ApiProviders {
    selected: &'static str,
    providers: Vec<ApiProviderView>,
}

fn provider(id: &str) -> Result<&'static ApiProvider, String> {
    providers::find(id).ok_or_else(|| format!("Unknown provider {id}."))
}

/// The selected provider and its model, from the settings.
fn api_choice(app: &AppHandle) -> (&'static ApiProvider, String) {
    let state = app.state::<SettingsState>();
    let s = state.0.lock().map(|s| s.clone()).unwrap_or_default();
    let p = providers::find(&s.api_provider).unwrap_or(&providers::PROVIDERS[0]);
    let model = s
        .api_models
        .get(p.id)
        .cloned()
        .unwrap_or_else(|| p.default_model.to_string());
    (p, model)
}

async fn has_key(p: &'static ApiProvider) -> bool {
    p.local
        || tauri::async_runtime::spawn_blocking(move || secrets::has(p.secret()))
            .await
            .unwrap_or(false)
}

#[tauri::command]
pub async fn api_key_status(app: AppHandle) -> ApiStatus {
    let (p, _) = api_choice(&app);
    ApiStatus {
        ready: has_key(p).await,
        label: p.label,
    }
}

/// Tells the windows what the API chat can do now.
async fn announce(app: &AppHandle) {
    let status = api_key_status(app.clone()).await;
    let _ = app.emit("settings", serde_json::json!({ "api": status }));
}

#[tauri::command]
pub async fn api_providers(app: AppHandle) -> ApiProviders {
    let (selected, _) = api_choice(&app);
    let models = app
        .state::<SettingsState>()
        .0
        .lock()
        .map(|s| s.api_models.clone())
        .unwrap_or_default();
    let mut list = Vec::new();
    for p in providers::PROVIDERS {
        list.push(ApiProviderView {
            provider: p,
            has_key: !p.local && has_key(p).await,
            model: models
                .get(p.id)
                .cloned()
                .unwrap_or_else(|| p.default_model.to_string()),
        });
    }
    ApiProviders {
        selected: selected.id,
        providers: list,
    }
}

#[tauri::command]
pub async fn api_provider_set(app: AppHandle, provider: String) -> Result<(), String> {
    let p = self::provider(&provider)?;
    settings::edit(&app, |s| s.api_provider = p.id.to_string())?;
    announce(&app).await;
    Ok(())
}

#[tauri::command]
pub async fn api_model_set(app: AppHandle, provider: String, model: String) -> Result<(), String> {
    let p = self::provider(&provider)?;
    let model = model.trim().to_string();
    if model.is_empty() || model.len() > 200 {
        return Err("Choose a model from the list.".into());
    }
    settings::edit(&app, |s| {
        s.api_models.insert(p.id.to_string(), model);
    })
}

/// The provider's chat models, asked live with the saved key.
#[tauri::command]
pub async fn api_models(provider: String) -> Result<Vec<String>, String> {
    let p = self::provider(&provider)?;
    let key = if p.local {
        None
    } else {
        tauri::async_runtime::spawn_blocking(move || secrets::get(p.secret()))
            .await
            .map_err(|_| "Can't read the key.".to_string())??
    };
    if !p.local && key.is_none() {
        return Err(format!("Save a {} key first.", p.label));
    }
    providers::models(p, key.as_deref()).await
}

/// Saves the key in the OS keyring, the only place it is ever written.
#[tauri::command]
pub async fn api_key_set(
    window: tauri::WebviewWindow,
    app: AppHandle,
    provider: String,
    key: String,
) -> Result<(), String> {
    crate::settings_page(&window)?;
    let p = self::provider(&provider)?;
    let key = key.trim().to_string();
    if p.local || !p.accepts_key(&key) {
        return Err(if p.key_hint.is_empty() {
            format!("That doesn't look like a {} API key.", p.label)
        } else {
            format!(
                "That doesn't look like a {} API key (they start with {}).",
                p.label, p.key_hint
            )
        });
    }
    tauri::async_runtime::spawn_blocking(move || secrets::set(p.secret(), &key))
        .await
        .map_err(|_| "Can't save the key.".to_string())??;
    tracing::info!(provider = p.id, "api key saved");
    announce(&app).await;
    Ok(())
}

#[tauri::command]
pub async fn api_key_clear(
    window: tauri::WebviewWindow,
    app: AppHandle,
    provider: String,
) -> Result<(), String> {
    crate::settings_page(&window)?;
    let p = self::provider(&provider)?;
    tauri::async_runtime::spawn_blocking(move || secrets::delete(p.secret()))
        .await
        .map_err(|_| "Can't remove the key.".to_string())??;
    tracing::info!(provider = p.id, "api key removed");
    announce(&app).await;
    Ok(())
}

/// What became of the files dropped on the island: the inbox copies, and the ones refused with why.
#[derive(serde::Serialize)]
struct Dropped {
    copied: Vec<String>,
    refused: Vec<Refused>,
}

#[derive(serde::Serialize)]
struct Refused {
    name: String,
    /// "folder", "too-big" or "unreadable".
    reason: &'static str,
}

/// Copies dropped files into the inbox and tells the island about the copies and the refusals.
pub fn on_drop(app: &AppHandle, dropped: &[PathBuf]) {
    // Files go to Zeca's chat: with him off, nothing is copied.
    if !crate::settings::zeca(app) {
        return;
    }
    // Up to MAX_FILE each, maybe from a slow disk: never on the main thread, where the island
    // would freeze until the last copy.
    let app = app.clone();
    let dropped = dropped.to_vec();
    tauri::async_runtime::spawn_blocking(move || {
        let mut out = Dropped {
            copied: Vec::new(),
            refused: Vec::new(),
        };
        for path in &dropped {
            match copy_to_inbox(path) {
                Ok(copy) => out.copied.push(copy),
                Err(reason) => out.refused.push(Refused {
                    name: path
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_default(),
                    reason,
                }),
            }
        }
        let _ = app.emit_to(ISLAND, "files", &out);
    });
}

/// Something is being dragged over the island (true) or left it (false).
pub fn on_drag(app: &AppHandle, over: bool) {
    if !crate::settings::zeca(app) {
        return;
    }
    let _ = app.emit_to(ISLAND, "drag", over);
}

fn copy_to_inbox(src: &Path) -> Result<String, &'static str> {
    let meta = std::fs::metadata(src).map_err(|_| "unreadable")?;
    if meta.is_dir() {
        return Err("folder");
    }
    if !meta.is_file() {
        return Err("unreadable");
    }
    if meta.len() > MAX_FILE {
        return Err("too-big");
    }
    copy_file(src).ok_or("unreadable")
}

fn copy_file(src: &Path) -> Option<String> {
    let dir = paths::inbox_dir();
    std::fs::create_dir_all(&dir).ok()?;
    let stamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let name = src.file_name()?.to_string_lossy();
    let dest = dir.join(format!("{stamp}-{name}"));
    std::fs::copy(src, &dest).ok()?;
    Some(dest.display().to_string())
}

/// Deletes inbox copies older than a week.
pub fn clean_inbox() {
    let Ok(entries) = std::fs::read_dir(paths::inbox_dir()) else {
        return;
    };
    let now = SystemTime::now();
    for entry in entries.flatten() {
        let old = entry
            .metadata()
            .and_then(|m| m.modified())
            .ok()
            .and_then(|m| now.duration_since(m).ok())
            .is_some_and(|age| age > INBOX_KEEP);
        if old {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vults_chat::Settings;

    #[test]
    fn a_saved_conversation_names_its_engine_and_settings_the_way_back() {
        for p in [Provider::Claude, Provider::Codex, Provider::ClaudeApi] {
            let chat = Chat::new(p, paths::chat_dir());
            assert_eq!(engine_provider(&engine_name(&chat)), Some(p));
        }
        assert_eq!(engine_provider("gemini"), None);
        for s in [
            vults_chat::Settings::Isolated,
            vults_chat::Settings::User,
            vults_chat::Settings::Project,
        ] {
            assert_eq!(settings_of(settings_name(s)), s);
        }
        // An unknown word reads nothing of the user's.
        assert_eq!(settings_of("everything"), vults_chat::Settings::Isolated);
    }

    #[test]
    fn a_saved_session_never_reads_as_a_flag() {
        assert!(valid_session("480b2798-5d6b-4fa2-9bcd-7fbe117db81b"));
        assert!(!valid_session("--dangerously-skip-permissions"));
        assert!(!valid_session("a b"));
        assert!(!valid_session(""));
    }

    #[test]
    fn a_conversation_is_named_by_its_first_message_or_its_files() {
        let mut chat = Chat::new(Provider::ClaudeApi, paths::chat_dir());
        chat.set_api(&vults_chat::providers::PROVIDERS[1], "m-1".into());
        let new = new_conversation(&chat, "  ", &["shot.png".into()]);
        assert_eq!(new.title, "shot.png");
        assert_eq!(
            new.engine,
            format!("api:{}", vults_chat::providers::PROVIDERS[1].id)
        );
        assert_eq!(new.model.as_deref(), Some("m-1"));
        assert_eq!(new.folder, "");
        assert_eq!(new_conversation(&chat, "Hi", &[]).title, "Hi");
    }

    #[test]
    fn a_pick_is_kept_by_folder_with_the_chats_own_as_empty() {
        assert_eq!(folder_key(None), "");
        assert_eq!(folder_key(Some(&paths::chat_dir())), "");
        assert_eq!(folder_key(Some(Path::new("/home/me/site"))), "/home/me/site");
    }

    #[test]
    fn only_a_pick_the_cli_can_read_is_kept() {
        // Absolute on every OS (a path with no drive is relative on Windows).
        let site = std::env::temp_dir();
        assert!(check_pick(site.to_str(), Some("sonnet"), Some("high")).is_ok());
        assert!(check_pick(None, None, None).is_ok());
        assert!(check_pick(Some("site"), None, None).is_err());
        assert!(check_pick(None, Some("-x"), None).is_err());
        assert!(check_pick(None, None, Some("ultra")).is_err());
    }

    #[test]
    fn an_effort_the_model_does_not_take_is_left_to_the_cli() {
        use vults_chat::Effort;
        assert!(effort_fits(Provider::Claude, Some("sonnet"), Effort::Max));
        assert!(!effort_fits(Provider::Claude, Some("haiku"), Effort::Max));
        assert!(effort_fits(Provider::Claude, None, Effort::Max));
        assert!(
            effort_fits(Provider::Claude, Some("claude-sonnet-5-5"), Effort::Low),
            "not listed: the CLI decides"
        );
    }

    #[test]
    fn the_cli_reads_a_projects_settings_only_where_claude_code_trusts_it() {
        let tmp = std::env::temp_dir().join(format!("vults-cli-settings-{}", std::process::id()));
        let real = tmp.join("real");
        std::fs::create_dir_all(&real).unwrap();
        let real = real.canonicalize().unwrap();
        let state = tmp.join(".claude.json");
        let trusted =
            serde_json::json!({ "projects": { real.to_str().unwrap(): { "hasTrustDialogAccepted": true } } });
        std::fs::write(&state, trusted.to_string()).unwrap();
        assert_eq!(cli_settings(false, &real, &state), Settings::Isolated);
        assert_eq!(cli_settings(true, &real, &state), Settings::Project);
        // Claude Code keys the folder by its real path: a link to it is the same folder.
        #[cfg(unix)]
        {
            let link = tmp.join("link");
            std::os::unix::fs::symlink(&real, &link).unwrap();
            assert_eq!(cli_settings(true, &link, &state), Settings::Project);
        }
        assert_eq!(cli_settings(true, &tmp, &state), Settings::User);
        std::fs::write(&state, "not json").unwrap();
        assert_eq!(cli_settings(true, &real, &state), Settings::User);
        assert_eq!(
            cli_settings(true, &real, &tmp.join("missing.json")),
            Settings::User
        );
        std::fs::remove_dir_all(&tmp).unwrap();
    }
}
