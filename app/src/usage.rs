//! The subscriptions' usage on the island, each from the CLI itself. Codex is asked every few
//! minutes, read-only, through the `codex` the user logged into; Claude Code hands it to our
//! statusLine command, which the hook relays. A failed read keeps the last answer (the island
//! drops a window once its reset time passes).

use std::collections::BTreeMap;
use std::sync::Mutex;
use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager};
use vults_agents::usage::Window;
use vults_protocol::AgentKind;

use crate::{ISLAND, paths};

/// Usage moves slowly; each read starts a short-lived `codex app-server`.
const EVERY: Duration = Duration::from_secs(5 * 60);

#[derive(Debug, Default)]
pub struct UsageState(Mutex<BTreeMap<AgentKind, Vec<Window>>>);

impl UsageState {
    fn all(&self) -> Vec<Window> {
        let by_agent = self.0.lock().unwrap_or_else(|e| e.into_inner());
        by_agent.values().flatten().cloned().collect()
    }
}

/// Epoch seconds, to judge the reset times the CLIs send.
pub fn epoch_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64)
}

/// An agent's latest windows. The island hears only of a change: Claude Code runs its
/// statusLine on every redraw.
pub fn set(app: &AppHandle, agent: AgentKind, windows: Vec<Window>) {
    let state = app.state::<UsageState>();
    {
        let mut by_agent = state.0.lock().unwrap_or_else(|e| e.into_inner());
        if by_agent.get(&agent) == Some(&windows) {
            return;
        }
        by_agent.insert(agent, windows);
    }
    let _ = app.emit_to(ISLAND, "usage", state.all());
}

pub fn start(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        loop {
            let dir = paths::chat_dir();
            let _ = std::fs::create_dir_all(&dir);
            match vults_chat::codex_usage(&dir).await {
                Ok(windows) => set(&app, AgentKind::Codex, windows),
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
    app.state::<UsageState>().all()
}
