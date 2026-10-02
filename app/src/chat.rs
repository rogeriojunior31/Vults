//! Chat from the island, and the files dropped on it.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::{Mutex, mpsc, oneshot};
use vultures_ai_chat::{Approver, Chat, Delta, Provider, Turn};
use vultures_ai_secrets::{self as secrets, Secret};

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

#[derive(Debug, Default)]
struct Waiting(std::sync::Mutex<HashMap<String, oneshot::Sender<bool>>>);

impl Approver for Waiting {
    fn wait(&self, id: &str) -> oneshot::Receiver<bool> {
        let (tx, rx) = oneshot::channel();
        if let Ok(mut map) = self.0.lock() {
            map.insert(id.to_string(), tx);
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
    let state = app.state::<ChatState>();
    let mut chat = state.chat.lock().await;
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
pub fn chat_decide(state: tauri::State<'_, ChatState>, id: String, allow: bool) {
    tracing::info!(allow, "chat permission answered");
    let sender = state.waiting.0.lock().ok().and_then(|mut m| m.remove(&id));
    if let Some(tx) = sender {
        let _ = tx.send(allow);
    }
}

/// Stop on the chat: ends the turn running now. Whatever it was waiting on is a no.
#[tauri::command]
pub fn chat_stop(state: tauri::State<'_, ChatState>) {
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

/// Whether an Anthropic API key is saved. The key itself never goes back to a window.
#[tauri::command]
pub async fn api_key_status() -> bool {
    tauri::async_runtime::spawn_blocking(|| secrets::has(Secret::AnthropicApiKey))
        .await
        .unwrap_or(false)
}

/// Saves the key in the OS keyring, the only place it is ever written.
#[tauri::command]
pub async fn api_key_set(app: AppHandle, key: String) -> Result<(), String> {
    let key = key.trim().to_string();
    if !looks_like_api_key(&key) {
        return Err("That doesn't look like an Anthropic API key (they start with sk-ant-).".into());
    }
    tauri::async_runtime::spawn_blocking(move || secrets::set(Secret::AnthropicApiKey, &key))
        .await
        .map_err(|_| "Can't save the key.".to_string())??;
    tracing::info!("api key saved");
    let _ = app.emit("settings", serde_json::json!({ "apiKey": true }));
    Ok(())
}

#[tauri::command]
pub async fn api_key_clear(app: AppHandle) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(|| secrets::delete(Secret::AnthropicApiKey))
        .await
        .map_err(|_| "Can't remove the key.".to_string())??;
    tracing::info!("api key removed");
    let _ = app.emit("settings", serde_json::json!({ "apiKey": false }));
    Ok(())
}

fn looks_like_api_key(key: &str) -> bool {
    key.starts_with("sk-ant-") && (20..=512).contains(&key.len()) && key.bytes().all(|b| b.is_ascii_graphic())
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
    let mut out = Dropped {
        copied: Vec::new(),
        refused: Vec::new(),
    };
    for path in dropped {
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
}

/// Something is being dragged over the island (true) or left it (false).
pub fn on_drag(app: &AppHandle, over: bool) {
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

/// The chat needs the keyboard; everything else must never take it from the user's terminal.
#[tauri::command]
pub fn island_keyboard(app: AppHandle, on: bool) {
    let Some(win) = app.get_webview_window(ISLAND) else {
        return;
    };
    let _ = app.run_on_main_thread(move || {
        #[cfg(target_os = "linux")]
        if let Ok(gtk) = win.gtk_window() {
            vultures_ai_platform::linux::set_keyboard(&gtk, on);
        }
        if on {
            let _ = win.set_focus();
        }
    });
}

#[cfg(test)]
mod tests {
    #[test]
    fn api_keys_are_checked_before_saving() {
        assert!(super::looks_like_api_key("sk-ant-api03-abcdefghijklmnop"));
        assert!(!super::looks_like_api_key("sk-ant-short"));
        assert!(!super::looks_like_api_key("sk-ant-api03-abc defghijklmnop"));
        assert!(!super::looks_like_api_key("ghp_abcdefghijklmnopqrstuvwxyz"));
    }
}
