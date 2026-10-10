//! Installing the hooks into an agent's config: preview (a diff), then apply exactly what the
//! user saw, after a dated backup. Agents go by the name their hooks run with: `claude`, `codex`,
//! `gemini`, `antigravity`, `qwen`, and `opencode`, which loads a plugin file of ours instead of hooks.

use std::path::PathBuf;
use std::time::SystemTime;

use serde::Serialize;
use tauri::{AppHandle, Manager};

use crate::paths::{config_home, home, hook_exe};
use vults_agent_config::{self as config, plugin, status_line};
use vults_agents::{Agent, MARKER, PluginAgent, installable, plugin_marker};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub agent: String,
    pub config_path: String,
    pub hook_path: String,
    pub hook_ready: bool,
    pub installed: bool,
    /// Installed, but not what this version would write (an older timeout, a missing entry):
    /// reinstalling brings what is new, such as answering Claude Code's questions on the island.
    pub outdated: bool,
    /// Outdated only because the hooks run another copy of the hook (another data folder,
    /// another build): the path they run. Updating points them to this app's.
    pub other_hook_path: Option<String>,
    /// Set when the config cannot be read: the UI shows it and offers nothing to write.
    pub error: Option<String>,
    /// Why installing (or updating) would be refused right now, so the UI does not offer it: the
    /// user's status line is saved beside another data folder's hook.
    pub install_blocked: Option<String>,
    /// Codex only: whether it will actually run our hooks.
    pub codex: Option<CodexTrust>,
    /// Claude Code only: whose statusLine the config has, "none", "ours" or "theirs". Ours
    /// brings the plan's usage to the island; theirs is saved beside the hook and kept running.
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
    agent: &'static dyn Agent,
    path: PathBuf,
    status_line: Option<String>,
}

fn target(name: &str) -> Result<Target, String> {
    let agent = installable(name).ok_or_else(|| format!("{name} is not supported yet"))?;
    Ok(Target {
        agent,
        path: agent.config_file(&home()),
        status_line: agent.status_line(&hook_exe()),
    })
}

/// The hooks and, where the agent has one, the statusLine: one diff, one backup, one click.
/// The second half is the sidecar beside the hook that keeps the user's own status line.
fn change(
    install: bool,
    t: &Target,
) -> impl Fn(&serde_json::Value, Option<&serde_json::Value>) -> (serde_json::Value, Option<serde_json::Value>) + '_
{
    move |current, saved| {
        if install {
            let next = t.agent.install(current, &hook_exe());
            match &t.status_line {
                Some(command) => status_line::install(&next, saved, command, MARKER),
                None => (next, saved.cloned()),
            }
        } else {
            status_line::uninstall(&t.agent.uninstall(current), saved, MARKER)
        }
    }
}

/// Where the user's old statusLine is kept: beside the hook, which runs it from there. That is
/// the hook the config's statusLine runs now, which may be another data folder's: removing the
/// hooks must put back what that one saved.
fn sidecar(t: &Target, install: bool) -> Result<PathBuf, String> {
    let ours = hook_exe().with_file_name(status_line::PREVIOUS_FILE);
    let current = config::read_json(&t.path).unwrap_or_default();
    let running = current["statusLine"]["command"]
        .as_str()
        .filter(|c| config::runs_ours(c, MARKER))
        .and_then(vults_agents::hook_exe_of)
        .map(|exe| exe.with_file_name(status_line::PREVIOUS_FILE));
    match running {
        Some(other) if other != ours && install && other.exists() => Err(format!(
            "Your own status line is saved beside the other hook, in {}. Remove the hooks first, which puts it back, then install them again.",
            other.display()
        )),
        Some(other) if !install => Ok(other),
        _ => Ok(ours),
    }
}

/// Installing must not overwrite a hook of the user's that has our hook's name.
fn refuse_overwrite(t: &Target, install: bool) -> Result<(), String> {
    match config::read_json(&t.path) {
        Ok(current) if install => t.agent.install_blocked(&current).map_or(Ok(()), Err),
        _ => Ok(()),
    }
}

/// A plugin agent's file, and what this version writes there.
fn plugin_target(p: &PluginAgent) -> (PathBuf, String) {
    ((p.file)(&config_home()), (p.text)(&hook_exe()))
}

fn plugin_status(p: &PluginAgent, agent: String) -> Status {
    let (path, text) = plugin_target(p);
    let marker = plugin_marker();
    let installed = plugin::installed(&path, &marker);
    Status {
        config_path: path.display().to_string(),
        hook_path: hook_exe().display().to_string(),
        hook_ready: hook_exe().exists(),
        installed,
        outdated: installed && !plugin::up_to_date(&path, &text),
        other_hook_path: None,
        error: None,
        install_blocked: plugin::blocked(&path, &marker),
        codex: None,
        status_line: None,
        agent,
    }
}

#[tauri::command]
pub fn install_status(agent: String) -> Result<Status, String> {
    if let Some(p) = vults_agents::plugin(&agent) {
        return Ok(plugin_status(p, agent));
    }
    let t = target(&agent)?;
    let path = &t.path;
    let (installed, error, current) = match config::read_json(path) {
        Ok(v) => (t.agent.installed(&v), None, v),
        Err(e) => (false, Some(e.to_string()), serde_json::Value::Null),
    };
    let install_blocked = sidecar(&t, true)
        .err()
        .or_else(|| t.agent.install_blocked(&current));
    // Only the hooks count: a missing statusLine of ours has its own note, and theirs stays.
    let outdated = installed && !t.agent.up_to_date(&current, &hook_exe());
    // Read-only: trust lives in Codex's config.toml, which only Codex writes.
    let codex = (agent == "codex").then(|| {
        let toml = std::fs::read_to_string(home().join(".codex").join("config.toml")).unwrap_or_default();
        let t = vults_agents::codex_trust(&current, path, &toml, MARKER);
        CodexTrust {
            hooks_disabled: t.hooks_disabled,
            untrusted: t.untrusted,
            total: t.total,
        }
    });
    Ok(Status {
        config_path: path.display().to_string(),
        hook_path: hook_exe().display().to_string(),
        hook_ready: hook_exe().exists(),
        installed,
        outdated,
        other_hook_path: vults_agents::other_hook(t.agent, &current, &hook_exe())
            .map(|p| p.display().to_string()),
        error,
        install_blocked,
        codex,
        status_line: t.status_line.map(|_| match status_line::owner(&current, MARKER) {
            status_line::Owner::None => "none",
            status_line::Owner::Ours => "ours",
            status_line::Owner::Theirs => "theirs",
        }),
        agent,
    })
}

#[tauri::command]
pub fn install_preview(agent: String, install: bool) -> Result<Preview, String> {
    if let Some(p) = vults_agents::plugin(&agent) {
        let (path, text) = plugin_target(p);
        let next = install.then_some(text.as_str());
        let p = plugin::preview(&path, next, &plugin_marker()).map_err(|e| e.to_string())?;
        return Ok(Preview {
            diff: p.diff,
            fingerprint: p.fingerprint,
        });
    }
    let t = target(&agent)?;
    refuse_overwrite(&t, install)?;
    let p = if t.status_line.is_some() {
        status_line::preview(&t.path, &sidecar(&t, install)?, change(install, &t))
    } else {
        config::preview(&t.path, |v| change(install, &t)(v, None).0)
    }
    .map_err(|e| e.to_string())?;
    let mut diff = p.diff;
    // A JSON file can't say it, so the review does: their status line is not lost.
    let current = config::read_json(&t.path).unwrap_or_default();
    if install
        && t.status_line.is_some()
        && status_line::owner(&current, MARKER) == status_line::Owner::Theirs
    {
        let command = current["statusLine"]["command"].as_str().unwrap_or_default();
        diff = format!(
            "Your status line keeps running: it is saved beside the hook, which runs `{command}` and shows what it prints. Removing the hooks puts it back.\n\n{diff}"
        );
    }
    Ok(Preview {
        diff,
        fingerprint: p.fingerprint,
    })
}

/// Returns the backup's path, if there was a file to back up.
#[tauri::command]
pub fn install_apply(
    window: tauri::WebviewWindow,
    agent: String,
    install: bool,
    fingerprint: String,
) -> Result<Option<String>, String> {
    crate::settings_page(&window)?;
    let now = SystemTime::now();
    if let Some(p) = vults_agents::plugin(&agent) {
        let (path, text) = plugin_target(p);
        let next = install.then_some(text.as_str());
        let result = plugin::apply(&path, &fingerprint, next, &plugin_marker(), now);
        match &result {
            Ok(_) => {
                tracing::info!(agent, install, "agent plugin written");
                audit_config(&agent, install, &path);
            }
            Err(e) => tracing::warn!(agent, install, "agent plugin not written: {e}"),
        }
        return result
            .map(|backup| backup.map(|b| b.display().to_string()))
            .map_err(|e| e.to_string());
    }
    let t = target(&agent)?;
    refuse_overwrite(&t, install)?;
    let result = if t.status_line.is_some() {
        status_line::apply(
            &t.path,
            &sidecar(&t, install)?,
            &fingerprint,
            change(install, &t),
            now,
        )
    } else {
        config::apply(&t.path, &fingerprint, |v| change(install, &t)(v, None).0, now)
    };
    match &result {
        Ok(_) => {
            tracing::info!(agent, install, "agent config written");
            audit_config(&agent, install, &t.path);
        }
        Err(e) => tracing::warn!(agent, install, "agent config not written: {e}"),
    }
    result
        .map(|backup| backup.map(|b| b.display().to_string()))
        .map_err(|e| e.to_string())
}

/// The audit log's line for a config the user's click wrote (ADR 0005).
fn audit_config(agent: &str, install: bool, path: &std::path::Path) {
    use vults_core::audit::{Act, Actor, Audit};
    let act = if install {
        Act::ConfigInstall
    } else {
        Act::ConfigRemove
    };
    let shown = crate::paths::shown(path);
    crate::audit::keep(Audit::new(Actor::Human, act, agent, "", "config", &shown));
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
        .args(["build", "--release", "--quiet", "-p", "vults-hook"])
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
        candidates.push(dir.join("../release").join(vults_brand::HOOK_EXE));
    }
    if let Ok(p) = app
        .path()
        .resolve(vults_brand::HOOK_EXE, tauri::path::BaseDirectory::Resource)
    {
        candidates.push(p);
    }
    if let Some(dir) = &exe_dir {
        candidates.push(dir.join(vults_brand::HOOK_EXE));
    }
    let Some(src) = candidates.into_iter().find(|p| p.is_file()) else {
        tracing::warn!(
            "{} not found next to the app: hooks cannot work",
            vults_brand::HOOK_EXE
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
