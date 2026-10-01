//! Installing the hooks into an agent's config: preview (a diff), then apply exactly what the
//! user saw, after a dated backup. Only Claude Code until Codex lands (M3).

use std::path::PathBuf;
use std::time::SystemTime;

use serde::Serialize;
use tauri::{AppHandle, Manager};
use vultures_ai_agent_config::{self as config, HookEntry};
use vultures_ai_agents::{MARKER, agent};
use vultures_ai_protocol::AgentKind;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub agent: AgentKind,
    pub config_path: String,
    pub hook_path: String,
    pub hook_ready: bool,
    pub installed: bool,
    /// Set when the config cannot be read: the UI shows it and offers nothing to write.
    pub error: Option<String>,
    /// Codex only: whether it will actually run our hooks.
    pub codex: Option<CodexTrust>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexTrust {
    pub hooks_disabled: bool,
    pub untrusted: usize,
    pub total: usize,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Preview {
    pub diff: String,
    pub fingerprint: String,
}

/// `~/.local/share/vultures-ai/bin/vultures-ai-hook`: a stable path for the agents' configs,
/// independent of where the app itself is installed or rebuilt.
fn hook_exe() -> PathBuf {
    data_dir().join("bin").join(vultures_ai_brand::HOOK_EXE)
}

fn data_dir() -> PathBuf {
    if cfg!(windows) {
        let base = std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(home);
        return base.join(vultures_ai_brand::NAME);
    }
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home().join(".local").join("share"))
        .join(vultures_ai_brand::SLUG)
}

fn home() -> PathBuf {
    std::env::home_dir().unwrap_or_else(|| PathBuf::from("."))
}

fn target(kind: AgentKind) -> Result<(PathBuf, Vec<HookEntry>), String> {
    let a = agent(kind).ok_or_else(|| format!("{kind:?} is not supported yet"))?;
    Ok((a.config_file(&home()), a.hook_entries(&hook_exe())))
}

fn change(install: bool, entries: Vec<HookEntry>) -> impl FnOnce(&serde_json::Value) -> serde_json::Value {
    move |current| {
        if install {
            config::with_ours(current, &entries, MARKER)
        } else {
            config::remove_ours(current, MARKER)
        }
    }
}

#[tauri::command]
pub fn install_status(agent: AgentKind) -> Result<Status, String> {
    let (path, _) = target(agent)?;
    let (installed, error, current) = match config::read_json(&path) {
        Ok(v) => (config::has_ours(&v, MARKER), None, v),
        Err(e) => (false, Some(e.to_string()), serde_json::Value::Null),
    };
    // Read-only: trust lives in Codex's config.toml, which only Codex writes.
    let codex = (agent == AgentKind::Codex).then(|| {
        let toml = std::fs::read_to_string(home().join(".codex").join("config.toml")).unwrap_or_default();
        let t = vultures_ai_agents::codex_trust(&current, &path, &toml, MARKER);
        CodexTrust {
            hooks_disabled: t.hooks_disabled,
            untrusted: t.untrusted,
            total: t.total,
        }
    });
    Ok(Status {
        agent,
        config_path: path.display().to_string(),
        hook_path: hook_exe().display().to_string(),
        hook_ready: hook_exe().exists(),
        installed,
        error,
        codex,
    })
}

#[tauri::command]
pub fn install_preview(agent: AgentKind, install: bool) -> Result<Preview, String> {
    let (path, entries) = target(agent)?;
    let p = config::preview(&path, change(install, entries)).map_err(|e| e.to_string())?;
    Ok(Preview {
        diff: p.diff,
        fingerprint: p.fingerprint,
    })
}

/// Returns the backup's path, if there was a file to back up.
#[tauri::command]
pub fn install_apply(agent: AgentKind, install: bool, fingerprint: String) -> Result<Option<String>, String> {
    let (path, entries) = target(agent)?;
    config::apply(&path, &fingerprint, change(install, entries), SystemTime::now())
        .map(|backup| backup.map(|b| b.display().to_string()))
        .map_err(|e| e.to_string())
}

/// Copies the hook next to the app's data, where the agents' configs point. In a bundle it is a
/// resource; in development it sits next to the app in `target/` (built by `npm run predev`).
pub fn ensure_hook_exe(app: &AppHandle) {
    let dest = hook_exe();
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|e| e.parent().map(PathBuf::from));
    let mut candidates = Vec::new();
    // Development first: the release hook (fast, small) beats the debug one, and in `tauri dev`
    // the resource directory is target/debug itself.
    if let Some(dir) = &exe_dir {
        candidates.push(dir.join("../release").join(vultures_ai_brand::HOOK_EXE));
    }
    if let Ok(p) = app
        .path()
        .resolve(vultures_ai_brand::HOOK_EXE, tauri::path::BaseDirectory::Resource)
    {
        candidates.push(p);
    }
    if let Some(dir) = &exe_dir {
        candidates.push(dir.join(vultures_ai_brand::HOOK_EXE));
    }
    let Some(src) = candidates.into_iter().find(|p| p.is_file()) else {
        eprintln!(
            "{} not found next to the app: hooks cannot work",
            vultures_ai_brand::HOOK_EXE
        );
        return;
    };
    let same = match (std::fs::read(&src), std::fs::read(&dest)) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    };
    if same {
        return;
    }
    if let Some(dir) = dest.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    // Copy beside and rename: an agent may be running the old hook right now.
    let temp = dest.with_extension("new");
    if std::fs::copy(&src, &temp)
        .and_then(|_| std::fs::rename(&temp, &dest))
        .is_err()
    {
        let _ = std::fs::remove_file(&temp);
        eprintln!("could not install {}", dest.display());
    }
}
