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

use crate::settings::{self, SettingsState};
use crate::{ISLAND, paths};

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
}

/// A card the chat waits on: its answer's channel, and what it asks (for the audit log).
#[derive(Debug)]
struct Asked {
    answer: oneshot::Sender<bool>,
    tool: String,
    target: String,
}

#[derive(Debug, Default)]
struct Waiting(std::sync::Mutex<HashMap<String, Asked>>);

impl Approver for Waiting {
    fn wait(&self, id: &str, tool: &str, target: &str) -> oneshot::Receiver<bool> {
        let (answer, rx) = oneshot::channel();
        if let Ok(mut map) = self.0.lock() {
            let asked = Asked {
                answer,
                tool: tool.to_owned(),
                target: target.to_owned(),
            };
            map.insert(id.to_string(), asked);
        }
        rx
    }
}

impl ChatState {
    pub fn new() -> Self {
        Self {
            chat: Mutex::new(Chat::new(Provider::Claude, paths::chat_dir())),
            waiting: Arc::default(),
            stop: std::sync::Mutex::new(None),
        }
    }
}

/// Streams the reply to the island as `chat` events: text, permissions to ask, then done or
/// error. `folder` (the focused session's project) is where a new conversation works.
#[tauri::command]
pub async fn chat_send(
    app: AppHandle,
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
    let forward = {
        let app = app.clone();
        tauri::async_runtime::spawn(async move {
            while let Some(delta) = rx.recv().await {
                let _ = app.emit_to(ISLAND, "chat", &delta);
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
    chat.set_api(api, model);
    if let Some(dir) = folder.map(PathBuf::from).filter(|d| d.is_absolute()) {
        // Ignored once the conversation has started: it stays where it began.
        chat.set_folder(dir);
    }
    let approver: Arc<dyn Approver> = state.waiting.clone();
    let (stop_tx, stop) = oneshot::channel();
    if let Ok(mut s) = state.stop.lock() {
        *s = Some(stop_tx);
    }
    tracing::info!(provider = ?chat.provider(), files = files.len(), "chat turn");
    chat.send(Turn { text, files }, tx, approver, stop).await;
    if let Ok(mut s) = state.stop.lock() {
        *s = None;
    }
    drop(chat);
    let _ = forward.await;
    Ok(())
}

/// The user's answer to a permission card. Only a click calls this.
#[tauri::command]
pub fn chat_decide(
    window: tauri::WebviewWindow,
    state: tauri::State<'_, ChatState>,
    id: String,
    allow: bool,
) {
    // Like an agent's card, the chat's is only answered on the island.
    if !crate::runtime::card_host(&window) {
        return;
    }
    tracing::info!(allow, "chat permission answered");
    let asked = state.waiting.0.lock().ok().and_then(|mut m| m.remove(&id));
    if let Some(asked) = asked {
        let _ = asked.answer.send(allow);
        let act = if allow { Act::Allow } else { Act::Deny };
        // The chat's own CLI asked: Zeca's, not one of the flock's sessions.
        crate::audit::keep(Audit::new(
            Actor::Human,
            act,
            "zeca",
            "",
            &asked.tool,
            &asked.target,
        ));
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
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let state = app.state::<ChatState>();
        let mut chat = state.chat.lock().await;
        let provider = chat.provider();
        *chat = Chat::new(provider, paths::chat_dir());
    });
}

fn end_turn(state: &ChatState) {
    tracing::info!("chat turn stopped");
    if let Ok(mut m) = state.waiting.0.lock() {
        m.clear();
    }
    if let Some(tx) = state.stop.lock().ok().and_then(|mut s| s.take()) {
        let _ = tx.send(());
    }
}

/// A new conversation, optionally with the other provider.
#[tauri::command]
pub async fn chat_reset(
    state: tauri::State<'_, ChatState>,
    provider: Option<Provider>,
) -> Result<Provider, ()> {
    // Anything still waiting is a no: that conversation is gone.
    if let Ok(mut m) = state.waiting.0.lock() {
        m.clear();
    }
    let mut chat = state.chat.lock().await;
    let provider = provider.unwrap_or(chat.provider());
    *chat = Chat::new(provider, paths::chat_dir());
    Ok(provider)
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
