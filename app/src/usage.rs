//! The subscriptions' usage on the island, each from the CLI itself. Codex is asked every few
//! minutes, read-only, through the `codex` the user logged into, and only through the network
//! gate (the read reaches OpenAI: ADR 0019); Claude Code hands it to our statusLine command, which
//! the hook relays. A failed read keeps the last answer (the island drops a window once its reset
//! time passes).

use std::collections::BTreeMap;
use std::future::Future;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use tauri::{AppHandle, Emitter, Manager};
use vults_agents::usage::Window;
use vults_protocol::AgentKind;

use crate::{ISLAND, paths};

/// Usage moves slowly; each read starts a short-lived `codex app-server`.
const EVERY: Duration = Duration::from_secs(5 * 60);
/// A failed read (no Codex, logged out) waits twice as long each time, up to this.
const MAX_WAIT: Duration = Duration::from_secs(60 * 60);

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
    let gate = crate::gate::shared(&app);
    tauri::async_runtime::spawn(async move {
        let read = || async {
            let dir = paths::chat_dir();
            let _ = std::fs::create_dir_all(&dir);
            vults_chat::codex_usage(&dir).await
        };
        let a = app.clone();
        run(&*gate, || locked(&app), read, |w| set(&a, AgentKind::Codex, w)).await;
    });
}

/// The read loop, forever: a read only when the screen is unlocked and the gate lets it out.
pub(crate) async fn run<R, F>(
    gate: &dyn vults_connectors::Gate,
    locked: impl Fn() -> bool,
    mut read: R,
    mut set: impl FnMut(Vec<Window>),
) where
    R: FnMut() -> F,
    F: Future<Output = Result<Vec<Window>, String>>,
{
    let mut wait = EVERY;
    loop {
        // Locked, nobody reads the meters: no process started for them.
        if !locked() && gate.allow(crate::gate::USAGE) {
            let started = Instant::now();
            let got = read().await;
            gate.done(crate::gate::USAGE, got.is_ok(), started.elapsed());
            wait = match got {
                Ok(windows) => {
                    set(windows);
                    EVERY
                }
                // No Codex, or not logged in: nothing to show, and nothing worth a warning.
                Err(e) => {
                    tracing::debug!("codex usage: {e}");
                    (wait * 2).min(MAX_WAIT)
                }
            };
        }
        tokio::time::sleep(wait).await;
    }
}

/// Is a `codex` on the PATH? Decides, once, the usage read's switch for a settings file from
/// before it existed: whoever saw Codex's meters keeps them.
pub fn codex_installed() -> bool {
    let names: &[&str] = if cfg!(windows) {
        &["codex.exe", "codex.cmd"]
    } else {
        &["codex"]
    };
    std::env::var_os("PATH").is_some_and(|path| {
        std::env::split_paths(&path).any(|dir| names.iter().any(|n| dir.join(n).is_file()))
    })
}

fn locked(app: &AppHandle) -> bool {
    app.state::<crate::runtime::LastView>().locked()
}

/// The last read, for an island that loads after it.
#[tauri::command]
pub fn usage(app: AppHandle) -> Vec<Window> {
    app.state::<UsageState>().all()
}
