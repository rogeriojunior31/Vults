//! The agents' doctor (`docs/dev/plan-zeca.md` W1): for each agent Vults knows, what it can do
//! with Vults (`vults_agents::caps`) beside what this computer has: its CLI on the `PATH` and the
//! version it says, and whether its hooks are in and current. Read only: it changes nothing. Its
//! one process per agent (`--version`) answered offline, at once, for each of the six CLIs
//! (checked under `unshare -n`); that is an observation of today's versions, not a guarantee.
//!
//! The `PATH` is the app's own, the one the chat runs its engines by: an agent a desktop launch
//! can't see (a shell rc's PATH) shows as not installed though its hooks still work.

pub(crate) mod dictation;

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Mutex, mpsc};
use std::time::{Duration, Instant};

use serde::Serialize;
use vults_agents::caps::Caps;

/// A version that takes longer than this is no answer: the doctor must not hang on one CLI.
const VERSION_TIMEOUT: Duration = Duration::from_secs(3);
/// The answer kept this long: a CLI's version rarely changes while the app runs.
const KEEP: Duration = Duration::from_secs(600);
/// Longest version line kept.
const MAX_VERSION: usize = 80;

/// One agent's card in Settings → Agents.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Row {
    /// The installer's id (`claude`, `codex`…).
    pub id: &'static str,
    pub name: &'static str,
    /// The command its CLI is run by.
    pub command: &'static str,
    #[serde(flatten)]
    pub caps: Caps,
    /// Where its CLI is, shown with `~`; none when it is not on the `PATH`.
    pub path: Option<String>,
    /// What `--version` printed, first line.
    pub version: Option<String>,
    /// `ok`, `outdated`, `missing`, `untrusted` (Codex won't run them), `error` (a config could
    /// not be read) or `broken` (this app's hook program is missing).
    pub hooks: &'static str,
}

/// `command` on the `PATH`, as the app would run it.
fn find(command: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    find_in(command, &path)
}

fn find_in(command: &str, path: &std::ffi::OsStr) -> Option<PathBuf> {
    std::env::split_paths(path)
        .filter(|dir| dir.is_absolute())
        .find_map(|dir| {
            #[cfg(windows)]
            {
                ["exe", "cmd", "bat"]
                    .into_iter()
                    .map(|ext| dir.join(command).with_extension(ext))
                    .find(|p| p.is_file())
            }
            #[cfg(not(windows))]
            {
                let file = dir.join(command);
                file.is_file().then_some(file)
            }
        })
}

/// The first line `--version` prints, cut; none when it fails or takes too long.
fn version(exe: &Path) -> Option<String> {
    let started = Instant::now();
    let mut cmd = Command::new(exe);
    #[cfg(windows)]
    std::os::windows::process::CommandExt::creation_flags(&mut cmd, 0x0800_0000);
    cmd.arg("--version")
        .envs(vults_agents::shell_env())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    // A group of its own, so a timeout takes what it relaunched too (a CLI may re-exec node).
    #[cfg(unix)]
    std::os::unix::process::CommandExt::process_group(&mut cmd, 0);
    #[cfg(windows)]
    let mut owned = vults_platform::windows_process::DiagnosticProcess::spawn(&mut cmd).ok()?;
    #[cfg(windows)]
    let child = owned.child_mut();
    #[cfg(not(windows))]
    let mut child = cmd.spawn().ok()?;
    // Read on a thread until stdout closes: a child that inherits it must not hold us past the
    // deadline either.
    let stdout = child.stdout.take()?;
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let mut out = Vec::new();
        let _ = stdout.take(4096).read_to_end(&mut out);
        let _ = tx.send(out);
    });
    let Ok(out) = rx.recv_timeout(VERSION_TIMEOUT) else {
        #[cfg(unix)]
        let _ = Command::new("kill")
            .args(["-KILL", "--", &format!("-{}", child.id())])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        let _ = child.kill();
        let _ = child.wait();
        return None;
    };
    loop {
        match child.try_wait() {
            Ok(Some(status)) if status.success() => break,
            Ok(Some(_)) | Err(_) => return None,
            Ok(None) if started.elapsed() < VERSION_TIMEOUT => std::thread::sleep(Duration::from_millis(10)),
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
    }
    let out = String::from_utf8_lossy(&out);
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
    // Hooks in, but what they run is not there: they exit 0 and the agent never shows.
    if !s.hook_ready {
        return "broken";
    }
    if s.outdated {
        return "outdated";
    }
    // Codex runs our hooks only when it trusts them and has them on; an unreadable config.toml
    // says nothing about either.
    if let Some(c) = s.codex {
        if c.error.is_some() {
            return "error";
        }
        if c.hooks_disabled || c.untrusted > 0 {
            return "untrusted";
        }
    }
    "ok"
}

/// A version-specific capability must never be promised from a different installed version.
fn installed_caps(id: &str, mut caps: Caps, version: Option<&str>) -> Caps {
    if id == "opencode" {
        let version = version
            .unwrap_or("")
            .trim()
            .trim_start_matches("opencode ")
            .trim_start_matches('v');
        caps.questions = if version.split('.').next() == Some("1") {
            vults_agents::caps::Questions::Card
        } else {
            vults_agents::caps::Questions::Terminal
        };
    }
    caps
}

/// Every row; the CLIs answer side by side, so the slowest sets the wait, not their sum.
fn rows() -> Vec<Row> {
    std::thread::scope(|scope| {
        let each: Vec<_> = vults_agents::registry::listed()
            .map(|(e, caps)| {
                scope.spawn(move || {
                    let exe = find(e.command);
                    let version = exe.as_deref().and_then(version);
                    Row {
                        id: e.id,
                        name: e.name,
                        command: e.command,
                        caps: installed_caps(e.id, caps, version.as_deref()),
                        path: exe.as_deref().map(crate::paths::shown),
                        version,
                        hooks: hooks(e.id),
                    }
                })
            })
            .collect();
        each.into_iter().filter_map(|h| h.join().ok()).collect()
    })
}

/// The doctor's rows, kept ten minutes; `fresh` asks again (Settings → Agents' Check again).
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
    let asked = Instant::now();
    tauri::async_runtime::spawn_blocking(move || {
        // One scan at a time: a call that waited on another takes its fresh answer.
        static SCAN: Mutex<()> = Mutex::new(());
        let _scan = SCAN.lock();
        if let Ok(kept) = KEPT.lock()
            && let Some((at, rows)) = kept.as_ref()
            && *at >= asked
        {
            return rows.clone();
        }
        let rows = rows();
        if let Ok(mut kept) = KEPT.lock() {
            *kept = Some((Instant::now(), rows.clone()));
        }
        rows
    })
    .await
    .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opencode_questions_follow_the_installed_major_version() {
        let caps = vults_agents::registry::of("opencode").unwrap().caps.unwrap();
        for version in [
            Some("2.2.1"),
            Some("opencode v2.2.1"),
            Some("3.0.0"),
            Some("unknown"),
            None,
        ] {
            assert_eq!(
                installed_caps("opencode", caps, version).questions,
                vults_agents::caps::Questions::Terminal
            );
        }
        assert_eq!(
            installed_caps("opencode", caps, Some("1.18.35")).questions,
            vults_agents::caps::Questions::Card
        );
    }

    #[test]
    #[cfg(unix)]
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
        // A child that keeps stdout open is waited on no longer than a slow CLI.
        let relaunched = script("relaunched", "sleep 10 & echo 1.0");
        let started = Instant::now();
        assert_eq!(version(&relaunched), None);
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "never waits on a child"
        );
        let failing = script("failing", "echo 9.9; exit 3");
        assert_eq!(version(&failing), None);
        assert_eq!(find("surely-not-a-command-vults"), None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(windows)]
    #[test]
    fn windows_doctor_finds_native_and_npm_launchers_and_reads_their_version() {
        let dir = std::env::temp_dir().join(format!("vults-doctor-windows-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let shim = dir.join("fixture.cmd");
        std::fs::write(&shim, "@echo off\r\necho 1.2.3\r\n").unwrap();
        assert_eq!(find_in("fixture", dir.as_os_str()), Some(shim.clone()));
        assert_eq!(version(&shim).as_deref(), Some("1.2.3"));
        assert_eq!(find_in("fixture", std::ffi::OsStr::new("relative")), None);
        std::fs::remove_file(shim).unwrap();
        std::fs::remove_dir(dir).unwrap();
    }

    #[cfg(windows)]
    #[test]
    #[ignore = "private Windows process fixture, invoked by the doctor descendant test"]
    fn windows_doctor_process_child() {
        use std::io::Write;
        use std::os::windows::io::AsRawHandle;
        use std::os::windows::process::CommandExt;
        let Some(dir) = std::env::var_os("VULTS_DOCTOR_FIXTURE_DIR").map(PathBuf::from) else {
            return;
        };
        let role = std::env::var("VULTS_DOCTOR_FIXTURE_ROLE").unwrap_or_default();
        if role.starts_with("parent") {
            if role != "parent" {
                #[link(name = "kernel32")]
                unsafe extern "system" {
                    fn SetHandleInformation(handle: *mut std::ffi::c_void, mask: u32, flags: u32) -> i32;
                }
                // SAFETY: only this private fixture's stdout loses inheritance. Otherwise a
                // Windows child can retain the pipe even after redirecting its own stdout.
                assert_ne!(
                    unsafe { SetHandleInformation(std::io::stdout().as_raw_handle(), 1, 0) },
                    0
                );
            }
            let child = Command::new(std::env::current_exe().unwrap())
                .args([
                    "--ignored",
                    "--exact",
                    "doctor::tests::windows_doctor_process_child",
                    "--nocapture",
                ])
                .env("VULTS_DOCTOR_FIXTURE_ROLE", "worker")
                .stdin(Stdio::null())
                .stdout(if role == "parent" {
                    Stdio::inherit()
                } else {
                    Stdio::null()
                })
                .stderr(Stdio::null())
                .creation_flags(0x0800_0000)
                .spawn()
                .unwrap();
            drop(child);
            let until = Instant::now() + Duration::from_secs(2);
            while std::fs::metadata(dir.join("heartbeat")).map_or(true, |m| m.len() == 0)
                && Instant::now() < until
            {
                std::thread::sleep(Duration::from_millis(5));
            }
            println!("fixture 1.0");
            if role == "parent-failure" {
                std::process::exit(7);
            }
        } else {
            let until = Instant::now() + Duration::from_secs(20);
            let mut tick = 0u64;
            while Instant::now() < until && !dir.join("stop").exists() {
                let mut heartbeat = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(dir.join("heartbeat"))
                    .unwrap();
                writeln!(heartbeat, "{tick}").unwrap();
                tick += 1;
                std::thread::sleep(Duration::from_millis(20));
            }
            let _ = std::fs::write(dir.join("stopped"), "done");
        }
    }

    #[cfg(windows)]
    #[test]
    fn windows_doctor_timeout_does_not_leave_a_relaunched_worker_alive() {
        windows_doctor_owned_tree("parent", false);
    }
    #[cfg(windows)]
    #[test]
    fn windows_doctor_completion_and_failure_also_stop_relaunched_workers() {
        windows_doctor_owned_tree("parent-success", true);
        windows_doctor_owned_tree("parent-failure", false);
    }
    #[cfg(windows)]
    fn windows_doctor_owned_tree(role: &str, success: bool) {
        let dir = std::env::temp_dir().join(format!(
            "vults doctor tree {} {}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let shim = dir.join("fixture.cmd");
        let exe = std::env::current_exe().unwrap();
        std::fs::write(&shim,format!("@echo off\r\nset \"VULTS_DOCTOR_FIXTURE_DIR={}\"\r\nset \"VULTS_DOCTOR_FIXTURE_ROLE={role}\"\r\n\"{}\" --ignored --exact doctor::tests::windows_doctor_process_child --nocapture\r\n",dir.display(),exe.display())).unwrap();
        let started = Instant::now();
        let result = version(&shim);
        std::thread::sleep(Duration::from_millis(100));
        let heartbeat = || {
            (0..20).find_map(|_| {
                let value = std::fs::metadata(dir.join("heartbeat"))
                    .ok()
                    .map(|m| m.len())
                    .filter(|n| *n > 0);
                if value.is_none() {
                    std::thread::sleep(Duration::from_millis(5));
                }
                value
            })
        };
        let first = heartbeat();
        std::thread::sleep(Duration::from_millis(200));
        let second = heartbeat();
        // Stop the private fixture even when this regression fails on the old implementation.
        std::fs::write(dir.join("stop"), "stop").unwrap();
        let until = Instant::now() + Duration::from_secs(2);
        while first != second && !dir.join("stopped").exists() && Instant::now() < until {
            std::thread::sleep(Duration::from_millis(20));
        }
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(result.is_some(), success);
        assert!(started.elapsed() < Duration::from_secs(6));
        assert!(first.is_some(), "the private descendant must have started");
        assert_eq!(
            first, second,
            "a diagnostic must stop its owned descendants, not only the launcher"
        );
    }
}
