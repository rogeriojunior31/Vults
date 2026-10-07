//! Replays a recorded session into a running Vults through the real hook, to try the
//! island and the animations without running an agent.
//!
//! Usage: cargo run -p vults-hook --example replay -- [session.jsonl] [--delay-ms 1200] [--agent claude|codex]
//!
//! Each line is an agent's hook JSON, sent `--delay-ms` apart. A permission request waits for your
//! Allow / Deny on the island (or Ctrl+Alt+Y / N) before the replay goes on, as a real agent does.

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Duration;

fn hook() -> PathBuf {
    // target/<profile>/examples/replay → target/<profile>/vults-hook
    let beside = std::env::current_exe()
        .ok()
        .and_then(|e| Some(e.parent()?.parent()?.join("vults-hook")));
    match beside {
        Some(p) if p.exists() => p,
        _ => std::env::home_dir()
            .unwrap_or_default()
            .join(".local/share/vults/bin/vults-hook"),
    }
}

fn main() {
    let mut args = std::env::args().skip(1);
    let mut file = PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/examples/demo-session.jsonl"
    ));
    let mut delay = Duration::from_millis(1200);
    let mut agent = String::from("claude");
    while let Some(a) = args.next() {
        match a.as_str() {
            "--delay-ms" => {
                delay = Duration::from_millis(args.next().and_then(|v| v.parse().ok()).unwrap_or(1200))
            }
            "--agent" => agent = args.next().unwrap_or(agent),
            path => file = PathBuf::from(path),
        }
    }
    let Ok(text) = std::fs::read_to_string(&file) else {
        eprintln!("can't read {}", file.display());
        std::process::exit(1);
    };
    let hook = hook();
    for line in text.lines().filter(|l| !l.trim().is_empty()) {
        let event = serde_json::from_str::<serde_json::Value>(line)
            .ok()
            .and_then(|v| v["hook_event_name"].as_str().map(str::to_string))
            .unwrap_or_default();
        println!("→ {event}");
        let Ok(mut child) = Command::new(&hook)
            .args(["--agent", &agent])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
        else {
            eprintln!("can't run {}", hook.display());
            std::process::exit(1);
        };
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(line.as_bytes());
        }
        if event == "PermissionRequest" {
            println!("  waiting for your answer on the island…");
        }
        if let Ok(out) = child.wait_with_output()
            && event == "PermissionRequest"
        {
            let answer = String::from_utf8_lossy(&out.stdout);
            let said = if answer.contains("allow") {
                "allowed"
            } else if answer.contains("deny") {
                "denied"
            } else {
                "no answer"
            };
            println!("  {said}");
        }
        std::thread::sleep(delay);
    }
}
