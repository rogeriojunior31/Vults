//! The M0 milestone: the real hook binary against the real server.
#![allow(clippy::unwrap_used)]

use std::io::Write;
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

const HOOK: &str = env!("CARGO_BIN_EXE_vultures-ai-hook");

fn run_hook(runtime_dir: &std::path::Path, args: &[&str], stdin: &str) -> Output {
    let mut child = Command::new(HOOK)
        .args(args)
        .env("XDG_RUNTIME_DIR", runtime_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(stdin.as_bytes()).unwrap();
    child.wait_with_output().unwrap()
}

fn runtime_dir(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("vultures-ai-hook-{}-{name}", std::process::id()));
    // Like a real $XDG_RUNTIME_DIR: both sides refuse a socket folder others can open.
    let mut builder = std::fs::DirBuilder::new();
    builder.recursive(true);
    #[cfg(unix)]
    std::os::unix::fs::DirBuilderExt::mode(&mut builder, 0o700);
    builder.create(&dir).unwrap();
    dir
}

#[test]
fn exits_fast_and_silent_without_the_app() {
    let dir = runtime_dir("absent");
    let start = Instant::now();
    let out = run_hook(
        &dir,
        &["--agent", "claude"],
        r#"{"hook_event_name":"PermissionRequest"}"#,
    );
    assert!(
        start.elapsed() < Duration::from_millis(300),
        "took {:?}",
        start.elapsed()
    );
    assert!(out.status.success());
    assert!(out.stdout.is_empty());
}

#[test]
fn garbage_on_stdin_is_ignored() {
    let out = run_hook(&runtime_dir("garbage"), &[], "not json");
    assert!(out.status.success());
    assert!(out.stdout.is_empty());
}

/// The hook copied into its own folder, with the installer's sidecar beside it.
#[cfg(unix)]
fn hook_with_previous(name: &str, saved: Option<&str>) -> std::path::PathBuf {
    let dir = runtime_dir(name);
    let hook = dir.join("vultures-ai-hook");
    std::fs::copy(HOOK, &hook).unwrap();
    if let Some(saved) = saved {
        std::fs::write(dir.join("statusline-previous.json"), saved).unwrap();
    }
    hook
}

/// Claude Code's status line: the user's own keeps showing, and nothing breaks without one.
#[cfg(unix)]
#[test]
fn the_status_line_runs_the_users_own() {
    let run = |hook: &std::path::Path, stdin: &str| {
        let mut child = Command::new(hook)
            .args(["--agent", "claude", "--statusline"])
            .env("XDG_RUNTIME_DIR", hook.parent().unwrap())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        child.stdin.take().unwrap().write_all(stdin.as_bytes()).unwrap();
        child.wait_with_output().unwrap()
    };
    let input = r#"{"session_id":"s","rate_limits":{"five_hour":{"used_percentage":5}}}"#;
    let mine = r#"{"type":"command","command":"printf 'mine: '; cat","padding":0}"#;
    let out = run(&hook_with_previous("sl-mine", Some(mine)), input);
    assert!(out.status.success());
    assert_eq!(String::from_utf8_lossy(&out.stdout), format!("mine: {input}"));
    // Bad JSON on stdin still reaches the user's line, which decides what to make of it.
    let out = run(&hook_with_previous("sl-garbage", Some(mine)), "not json");
    assert_eq!(out.stdout, b"mine: not json");
    // A command that is gone, or no sidecar at all: silent, exit 0.
    let gone = r#"{"type":"command","command":"/no/such/line.sh"}"#;
    for hook in [
        hook_with_previous("sl-gone", Some(gone)),
        hook_with_previous("sl-none", None),
    ] {
        let out = run(&hook, input);
        assert!(out.status.success());
        assert!(out.stdout.is_empty());
    }
}

#[cfg(unix)]
mod with_server {
    use super::*;
    use tokio::sync::mpsc;
    use vultures_ai_ipc::{Endpoint, Incoming};
    use vultures_ai_protocol::{AgentKind, Decision};

    async fn start(name: &str) -> (std::path::PathBuf, mpsc::Receiver<Incoming>) {
        let dir = runtime_dir(name);
        let (tx, rx) = mpsc::channel(8);
        tokio::spawn(vultures_ai_ipc::serve(
            Endpoint::Unix(dir.join("vultures-ai.sock")),
            tx,
        ));
        while !dir.join("vultures-ai.sock").exists() {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        (dir, rx)
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn permission_round_trip() {
        let (dir, mut rx) = start("allow").await;
        let hook = tokio::task::spawn_blocking(move || {
            run_hook(
                &dir,
                &["--agent", "claude"],
                r#"{"hook_event_name":"PermissionRequest","tool_name":"Bash"}"#,
            )
        });

        let Some(Incoming::Request { event, reply }) = rx.recv().await else {
            panic!("expected a request")
        };
        assert_eq!(event.agent, AgentKind::Claude);
        assert_eq!(event.payload["tool_name"], "Bash");
        reply.ack();
        reply.decide(Decision::Allow);

        let out = hook.await.unwrap();
        assert!(out.status.success());
        assert_eq!(
            String::from_utf8(out.stdout).unwrap(),
            "{\"hookSpecificOutput\":{\"hookEventName\":\"PermissionRequest\",\"decision\":{\"behavior\":\"allow\"}}}\n"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn codex_gets_its_deny_too() {
        let (dir, mut rx) = start("codex").await;
        let hook = tokio::task::spawn_blocking(move || {
            run_hook(
                &dir,
                &["--agent", "codex", "PermissionRequest"],
                r#"{"hook_event_name":"PermissionRequest","session_id":"s","tool_name":"Bash","tool_input":{"command":"rm -rf /"}}"#,
            )
        });
        let Some(Incoming::Request { event, reply }) = rx.recv().await else {
            panic!("expected a request")
        };
        assert_eq!(event.agent, AgentKind::Codex);
        reply.ack();
        reply.decide(Decision::Deny);
        let out = hook.await.unwrap();
        assert_eq!(
            String::from_utf8(out.stdout).unwrap(),
            "{\"hookSpecificOutput\":{\"hookEventName\":\"PermissionRequest\",\"decision\":{\"behavior\":\"deny\",\"message\":\"Denied from Vultures AI\"}}}\n"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn events_are_fire_and_forget() {
        let (dir, mut rx) = start("event").await;
        let out = tokio::task::spawn_blocking(move || {
            run_hook(&dir, &["--agent", "codex", "PostToolUse"], r#"{"cwd":"/work"}"#)
        })
        .await
        .unwrap();
        assert!(out.stdout.is_empty());

        let Some(Incoming::Event(event)) = rx.recv().await else {
            panic!("expected an event")
        };
        assert_eq!(event.agent, AgentKind::Codex);
        assert_eq!(event.event, "PostToolUse");
        assert_eq!(event.terminal.cwd.as_deref(), Some("/work"));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_decline_lets_the_terminal_ask() {
        let (dir, mut rx) = start("decline").await;
        let hook = tokio::task::spawn_blocking(move || {
            run_hook(&dir, &[], r#"{"hook_event_name":"PermissionRequest"}"#)
        });
        let Some(Incoming::Request { reply, .. }) = rx.recv().await else {
            panic!("expected a request")
        };
        reply.decline();
        let out = hook.await.unwrap();
        assert!(out.status.success());
        assert!(out.stdout.is_empty());
    }
}
