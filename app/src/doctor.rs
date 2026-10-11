//! The agents' doctor (`docs/dev/plan-zeca.md` W1): for each agent Vults knows, what it can do
//! with Vults (`vults_agents::caps`) beside what this computer has: its CLI on the `PATH` and the
//! version it says, and whether its hooks are in and current. Read only: it changes nothing, and
//! its one process per agent (`--version`) answers locally.

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::Serialize;
use vults_agents::caps::{self, Caps};

/// A version that takes longer than this is no answer: the doctor must not hang on one CLI.
const VERSION_TIMEOUT: Duration = Duration::from_secs(3);
/// The answer kept this long: a CLI's version rarely changes while the app runs.
const KEEP: Duration = Duration::from_secs(600);
/// Longest version line kept.
const MAX_VERSION: usize = 80;

/// One agent's row in the Nest.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Row {
    #[serde(flatten)]
    pub caps: Caps,
    /// Where its CLI is, shown with `~`; none when it is not on the `PATH`.
    pub path: Option<String>,
    /// What `--version` printed, first line.
    pub version: Option<String>,
    /// `ok`, `outdated`, `missing`, `untrusted` (Codex won't run them) or `error`.
    pub hooks: &'static str,
}

/// `command` on the `PATH`, as the app would run it.
fn find(command: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|d| d.join(command))
        .find(|p| p.is_file())
}

/// The first line `--version` prints, cut; none when it fails or takes too long.
fn version(exe: &Path) -> Option<String> {
    let mut child = std::process::Command::new(exe)
        .arg("--version")
        .envs(vults_agents::shell_env())
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .ok()?;
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if started.elapsed() < VERSION_TIMEOUT => {
                std::thread::sleep(Duration::from_millis(50));
            }
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
    }
    let mut out = String::new();
    use std::io::Read;
    child.stdout.take()?.take(4096).read_to_string(&mut out).ok()?;
    let line = out.lines().map(str::trim).find(|l| !l.is_empty())?;
    Some(line.chars().take(MAX_VERSION).collect())
}

/// The hooks' state, from what Settings → Agents reads.
fn hooks(id: &str) -> &'static str {
    let Ok(s) = crate::installer::install_status(id.to_string()) else {
        return "error";
    };
    if s.error.is_some() {
        return "error";
    }
    if !s.installed {
        return "missing";
    }
    if s.outdated {
        return "outdated";
    }
    // Codex runs our hooks only when it trusts them and has them on.
    if let Some(c) = s.codex
        && (c.hooks_disabled || c.untrusted > 0)
    {
        return "untrusted";
    }
    "ok"
}

fn rows() -> Vec<Row> {
    caps::ALL
        .iter()
        .map(|c| {
            let exe = find(c.command);
            Row {
                caps: *c,
                path: exe.as_deref().map(crate::paths::shown),
                version: exe.as_deref().and_then(version),
                hooks: hooks(c.id),
            }
        })
        .collect()
}

/// The doctor's rows, kept ten minutes; `fresh` asks again (the Nest's Check again).
#[tauri::command]
pub async fn agents_doctor(fresh: bool) -> Vec<Row> {
    static KEPT: Mutex<Option<(Instant, Vec<Row>)>> = Mutex::new(None);
    if !fresh
        && let Ok(kept) = KEPT.lock()
        && let Some((at, rows)) = kept.as_ref()
        && at.elapsed() < KEEP
    {
        return rows.clone();
    }
    let rows = tauri::async_runtime::spawn_blocking(rows)
        .await
        .unwrap_or_default();
    if let Ok(mut kept) = KEPT.lock() {
        *kept = Some((Instant::now(), rows.clone()));
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn a_version_is_its_first_line_and_a_slow_cli_is_none() {
        let dir = std::env::temp_dir().join(format!("vults-doctor-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap_or_default();
        let script = |name: &str, body: &str| {
            use std::os::unix::fs::PermissionsExt;
            let p = dir.join(name);
            std::fs::write(&p, format!("#!/bin/sh\n{body}\n")).unwrap_or_default();
            std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap_or_default();
            p
        };
        let ok = script("ok", "echo; echo '2.1.295 (Claude Code)'; echo more");
        assert_eq!(version(&ok).as_deref(), Some("2.1.295 (Claude Code)"));
        let slow = script("slow", "sleep 10; echo late");
        let started = Instant::now();
        assert_eq!(version(&slow), None);
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "never waits on a slow CLI"
        );
        let failing = script("failing", "exit 3");
        assert_eq!(version(&failing), None);
        assert_eq!(find("surely-not-a-command-vults"), None);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
