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

/// Wakes the read loop: the switch just changed, so it need not wait out its sleep.
static WAKE: tokio::sync::Notify = tokio::sync::Notify::const_new();

pub fn wake() {
    WAKE.notify_one();
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
        // A read that lands after the switch went off is dropped: off means no meters.
        let keep = |w| {
            if crate::settings::codex_usage(&a) {
                set(&a, AgentKind::Codex, w);
            }
        };
        run(&*gate, || locked(&app), read, keep, &WAKE).await;
    });
}

/// How long a window takes, past its reset, to show its fresh count.
const AFTER_RESET: Duration = Duration::from_secs(30);

/// The wait after a good read: [`EVERY`], or less when a window resets before then, so its meter
/// shows the new count half a minute after the reset rather than up to five minutes later. A reset
/// already past is not waited for.
pub(crate) fn after_read(windows: &[Window], now: i64) -> Duration {
    windows
        .iter()
        .filter_map(|w| w.resets_at)
        .filter(|&at| at > now)
        .map(|at| Duration::from_secs((at - now) as u64) + AFTER_RESET)
        .fold(EVERY, Duration::min)
}

/// The read loop, forever: a read only when the screen is unlocked and the gate lets it out.
pub(crate) async fn run<R, F>(
    gate: &dyn vults_connectors::Gate,
    locked: impl Fn() -> bool,
    mut read: R,
    mut set: impl FnMut(Vec<Window>),
    wake: &tokio::sync::Notify,
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
                    let next = after_read(&windows, epoch_now());
                    set(windows);
                    next
                }
                // No Codex, or not logged in: nothing to show, and nothing worth a warning.
                Err(e) => {
                    tracing::debug!("codex usage: {e}");
                    (wait * 2).min(MAX_WAIT)
                }
            };
        }
        tokio::select! {
            () = tokio::time::sleep(wait) => {}
            // Switched on (or off): try now, from a fresh wait.
            () = wake.notified() => wait = EVERY,
        }
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

/// Before the runtime is up there is no view yet: counted as locked, so nothing is read.
fn locked(app: &AppHandle) -> bool {
    app.try_state::<crate::runtime::LastView>()
        .is_none_or(|v| v.locked())
}

/// How much of `agent`'s plan is in use: its fullest window not yet reset; none when unknown (W8).
pub fn quota(app: &AppHandle, agent: AgentKind) -> Option<u8> {
    let state = app.state::<UsageState>();
    let by_agent = state.0.lock().unwrap_or_else(|e| e.into_inner());
    let now = epoch_now();
    by_agent
        .get(&agent)?
        .iter()
        .filter(|w| w.resets_at.is_none_or(|r| r > now))
        .map(|w| w.used_percent)
        .max()
}

/// The last read, for an island that loads after it.
#[tauri::command]
pub fn usage(app: AppHandle) -> Vec<Window> {
    app.state::<UsageState>().all()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn window(resets_at: Option<i64>) -> Window {
        Window {
            agent: AgentKind::Codex,
            minutes: 300,
            used_percent: 40,
            resets_at,
        }
    }

    #[test]
    fn a_reset_before_the_next_read_brings_the_read_forward() {
        let now = 1_790_000_000;
        assert_eq!(after_read(&[], now), EVERY);
        // Resets in 2 minutes: read 30 s after it.
        let soon = [window(Some(now + 3600)), window(Some(now + 120)), window(None)];
        assert_eq!(after_read(&soon, now), Duration::from_secs(150));
        // Further out than the usual wait, or already past: the usual wait.
        assert_eq!(after_read(&[window(Some(now + 3600))], now), EVERY);
        assert_eq!(after_read(&[window(Some(now - 10)), window(None)], now), EVERY);
        // Resetting this second: just the half minute.
        assert_eq!(after_read(&[window(Some(now + 1))], now), Duration::from_secs(31));
    }
}
