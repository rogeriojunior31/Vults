//! Chat from the island, and the files dropped on it.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::{Mutex, mpsc, oneshot};
use vultures_ai_chat::{Approver, Chat, Delta, Provider, Turn};

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
    tracing::info!(provider = ?chat.provider(), files = files.len(), "chat turn");
    chat.send(Turn { text, files }, tx, approver).await;
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

/// Copies dropped files into the inbox and tells the island about the copies.
pub fn on_drop(app: &AppHandle, dropped: &[PathBuf]) {
    let copies: Vec<String> = dropped.iter().filter_map(|p| copy_to_inbox(p)).collect();
    if !copies.is_empty() {
        let _ = app.emit_to(ISLAND, "files", &copies);
    }
}

fn copy_to_inbox(src: &Path) -> Option<String> {
    let meta = std::fs::metadata(src).ok()?;
    if !meta.is_file() || meta.len() > MAX_FILE {
        return None;
    }
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
