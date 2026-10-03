//! Installing the hooks into an agent's config: preview (a diff), then apply exactly what the
//! user saw, after a dated backup. Only Claude Code until Codex lands (M3).

use std::path::PathBuf;
use std::time::SystemTime;

use serde::Serialize;
use tauri::{AppHandle, Manager};

use crate::paths::{home, hook_exe};
use vultures_ai_agent_config::{self as config, HookEntry, status_line};
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
    /// Installed, but not what this version would write (an older timeout, a missing entry):
    /// reinstalling brings what is new, such as answering Claude Code's questions on the island.
    pub outdated: bool,
    /// Set when the config cannot be read: the UI shows it and offers nothing to write.
    pub error: Option<String>,
    /// Codex only: whether it will actually run our hooks.
    pub codex: Option<CodexTrust>,
    /// Claude Code only: whose statusLine the config has, "none", "ours" or "theirs". Ours
    /// brings the plan's usage to the island; theirs is never replaced.
    pub status_line: Option<&'static str>,
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

struct Target {
    path: PathBuf,
    entries: Vec<HookEntry>,
    status_line: Option<String>,
}

fn target(kind: AgentKind) -> Result<Target, String> {
    let a = agent(kind).ok_or_else(|| format!("{kind:?} is not supported yet"))?;
    Ok(Target {
        path: a.config_file(&home()),
        entries: a.hook_entries(&hook_exe()),
        status_line: a.status_line(&hook_exe()),
    })
}

/// The hooks and, where the agent has one, the statusLine: one diff, one backup, one click.
fn change(install: bool, t: Target) -> impl FnOnce(&serde_json::Value) -> serde_json::Value {
    move |current| {
        if install {
            let next = config::with_ours(current, &t.entries, MARKER);
            match &t.status_line {
                Some(command) => status_line::with_ours(&next, command, MARKER),
                None => next,
            }
        } else {
            status_line::remove_ours(&config::remove_ours(current, MARKER), MARKER)
        }
    }
}

#[tauri::command]
pub fn install_status(agent: AgentKind) -> Result<Status, String> {
    let t = target(agent)?;
    let path = t.path;
    let (installed, error, current) = match config::read_json(&path) {
        Ok(v) => (config::has_ours(&v, MARKER), None, v),
        Err(e) => (false, Some(e.to_string()), serde_json::Value::Null),
    };
    // Only the hooks count: a missing statusLine of ours has its own note, and theirs stays.
    let outdated = installed && !config::ours_match(&current, &t.entries, MARKER);
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
        outdated,
        error,
        codex,
        status_line: t.status_line.map(|_| match status_line::owner(&current, MARKER) {
            status_line::Owner::None => "none",
            status_line::Owner::Ours => "ours",
            status_line::Owner::Theirs => "theirs",
        }),
    })
}

#[tauri::command]
pub fn install_preview(agent: AgentKind, install: bool) -> Result<Preview, String> {
    let t = target(agent)?;
    let path = t.path.clone();
    let p = config::preview(&path, change(install, t)).map_err(|e| e.to_string())?;
    Ok(Preview {
        diff: p.diff,
        fingerprint: p.fingerprint,
    })
}

/// Returns the backup's path, if there was a file to back up.
#[tauri::command]
pub fn install_apply(agent: AgentKind, install: bool, fingerprint: String) -> Result<Option<String>, String> {
    let t = target(agent)?;
    let path = t.path.clone();
    let result = config::apply(&path, &fingerprint, change(install, t), SystemTime::now());
    match &result {
        Ok(_) => tracing::info!(?agent, install, "agent config written"),
        Err(e) => tracing::warn!(?agent, install, "agent config not written: {e}"),
    }
    result
        .map(|backup| backup.map(|b| b.display().to_string()))
        .map_err(|e| e.to_string())
}

/// Copies the hook next to the app's data, where the agents' configs point. In a bundle it is a
/// resource; in development it sits next to the app in `target/` (built by `npm run predev`).
pub fn ensure_hook_exe(app: &AppHandle) {
    // `npm run tauri dev` builds the hook once, when it starts; its watcher then rebuilds only
    // the app, which would hand the agents a hook older than the code. Off the main thread: a
    // hook that changed takes seconds to build.
    #[cfg(debug_assertions)]
    {
        let app = app.clone();
        std::thread::spawn(move || {
            build_dev_hook();
            install_hook_exe(&app);
        });
    }
    #[cfg(not(debug_assertions))]
    install_hook_exe(app);
}

/// The release hook from this checkout's sources; a no-op when it is up to date.
#[cfg(debug_assertions)]
fn build_dev_hook() {
    let workspace = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let cargo = std::path::Path::new(env!("CARGO"));
    let mut build = std::process::Command::new(cargo);
    build
        .args(["build", "--release", "--quiet", "-p", "vultures-ai-hook"])
        .current_dir(&workspace);
    // The toolchain that built the app, not whatever `rustc` the PATH finds (a version shim).
    let rustc = cargo.with_file_name("rustc");
    if rustc.is_file() {
        build.env("RUSTC", rustc);
    }
    let built = build.status();
    if !built.is_ok_and(|s| s.success()) {
        tracing::warn!("could not rebuild the hook relay; the agents may get an old one");
    }
}

fn install_hook_exe(app: &AppHandle) {
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
        tracing::warn!(
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
        tracing::warn!("could not install the hook relay");
    }
}
