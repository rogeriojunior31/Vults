//! The subscriptions' usage on the island. Codex is asked every few minutes, read-only, through
//! the `codex` the user logged into; a failed read keeps the last answer (the island drops a
//! window once its reset time passes).

use std::sync::Mutex;
use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager};
use vultures_ai_agents::usage::Window;

use crate::{ISLAND, paths};

/// Usage moves slowly; each read starts a short-lived `codex app-server`.
const EVERY: Duration = Duration::from_secs(5 * 60);

#[derive(Debug, Default)]
pub struct UsageState(Mutex<Vec<Window>>);

pub fn start(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        loop {
            let dir = paths::chat_dir();
            let _ = std::fs::create_dir_all(&dir);
            match vultures_ai_chat::codex_usage(&dir).await {
                Ok(windows) => {
                    *app.state::<UsageState>()
                        .0
                        .lock()
                        .unwrap_or_else(|e| e.into_inner()) = windows.clone();
                    let _ = app.emit_to(ISLAND, "usage", windows);
                }
                // No Codex, or not logged in: nothing to show, and nothing worth a warning.
                Err(e) => tracing::debug!("codex usage: {e}"),
            }
            tokio::time::sleep(EVERY).await;
        }
    });
}

/// The last read, for an island that loads after it.
#[tauri::command]
pub fn usage(app: AppHandle) -> Vec<Window> {
    app.state::<UsageState>()
        .0
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
}
