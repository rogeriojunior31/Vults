//! `codex exec --json`, the fallback when `codex app-server` is unavailable: one `agent_message`
//! per reply, no stream, and read-only, because nothing in exec mode can ask the user.

use std::path::Path;
use std::process::Stdio;

use serde_json::Value;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

use vults_agents::driver::{Event, Sink};

use crate::{Turn, Usage, is_image, prompt};

#[derive(Debug, PartialEq)]
pub(crate) enum Line {
    Session(String),
    Text(String),
    Failed(String),
    Other,
    Usage(Usage),
}

/// One turn through `codex exec`, updating `session` with the thread id.
pub(crate) async fn turn(
    dir: &Path,
    session: &mut Option<String>,
    turn: &Turn,
    sink: &dyn Sink,
) -> Result<(), String> {
    let mut cmd = command(turn, session.as_deref());
    let mut child = crate::dies_with_app(&mut cmd)
        .current_dir(dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => "The Codex CLI isn't installed.".to_string(),
            _ => format!("Can't start Codex: {e}"),
        })?;
    let mut lines = BufReader::new(child.stdout.take().ok_or("no stdout")?).lines();
    let mut said = false;
    while let Ok(Some(raw)) = lines.next_line().await {
        match parse(&raw) {
            Line::Session(id) => *session = Some(id),
            Line::Text(text) => {
                said = true;
                sink.event(Event::Text(text)).await;
            }
            Line::Failed(m) => return Err(format!("Codex: {m}")),
            Line::Usage(usage) => {
                sink.event(Event::Usage(usage)).await;
            }
            Line::Other => {}
        }
    }
    let _ = child.wait().await;
    if said {
        Ok(())
    } else {
        Err("Codex returned no answer. Is it logged in?".into())
    }
}

pub(crate) fn command(turn: &Turn, session: Option<&str>) -> Command {
    let mut cmd = Command::new("codex");
    cmd.arg("exec");
    if let Some(id) = session {
        cmd.args(["resume", id]);
    }
    cmd.args(["--json", "--skip-git-repo-check"])
        .args(["-c", "sandbox_mode=\"read-only\""])
        .args(["-c", "approval_policy=\"never\""])
        // No user hooks: the chat must never show up on the island as a session.
        .args(["-c", "features.hooks=false"]);
    for image in turn.files.iter().filter(|f| is_image(f)) {
        cmd.arg("-i").arg(image);
    }
    cmd.arg("--").arg(prompt(turn, session.is_none(), true));
    cmd
}

pub(crate) fn parse(raw: &str) -> Line {
    let Ok(v) = serde_json::from_str::<Value>(raw) else {
        return Line::Other;
    };
    match v.get("type").and_then(Value::as_str) {
        Some("thread.started") => v
            .get("thread_id")
            .and_then(Value::as_str)
            .map_or(Line::Other, |s| Line::Session(s.to_string())),
        Some("item.completed") if v["item"]["type"] == "agent_message" => v["item"]["text"]
            .as_str()
            .map_or(Line::Other, |t| Line::Text(t.to_string())),
        Some("turn.failed") => Line::Failed(
            v["error"]["message"]
                .as_str()
                .unwrap_or("the turn failed")
                .to_string(),
        ),
        // The fallback path (no app-server). Read as the turn's own usage; whether `exec resume`
        // reports the thread's running total instead is unchecked, and would overcount here.
        Some("turn.completed") => {
            let u = &v["usage"];
            let n = |k: &str| u[k].as_u64().unwrap_or(0);
            let usage = Usage {
                input: n("input_tokens"),
                output: n("output_tokens"),
                cache_read: n("cached_input_tokens"),
                ..Usage::default()
            };
            if usage.is_empty() {
                Line::Other
            } else {
                Line::Usage(usage)
            }
        }
        Some("error") => Line::Failed(v["message"].as_str().unwrap_or("error").to_string()),
        _ => Line::Other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Lines recorded from codex-cli 0.159.
    #[test]
    fn json_lines() {
        assert_eq!(
            parse(r#"{"type":"thread.started","thread_id":"01a0f93e-1"}"#),
            Line::Session("01a0f93e-1".into())
        );
        assert_eq!(
            parse(
                r#"{"type":"item.completed","item":{"id":"item_0","type":"agent_message","text":"hello there"}}"#
            ),
            Line::Text("hello there".into())
        );
        assert_eq!(
            parse(r#"{"type":"item.completed","item":{"type":"reasoning","text":"…"}}"#),
            Line::Other
        );
        assert_eq!(
            parse(r#"{"type":"turn.failed","error":{"message":"usage limit reached"}}"#),
            Line::Failed("usage limit reached".into())
        );
        assert_eq!(parse("Reading additional input from stdin..."), Line::Other);
    }

    #[test]
    fn resumes_read_only_without_hooks() {
        let turn = Turn {
            text: "hi".into(),
            files: vec!["/inbox/shot.png".into()],
        };
        let args: Vec<String> = command(&turn, Some("t1"))
            .as_std()
            .get_args()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        assert_eq!(&args[..3], ["exec", "resume", "t1"]);
        assert!(args.windows(2).any(|w| w == ["-c", "features.hooks=false"]));
        assert!(args.windows(2).any(|w| w == ["-c", "sandbox_mode=\"read-only\""]));
        assert!(args.windows(2).any(|w| w == ["-i", "/inbox/shot.png"]));
    }

    #[test]
    fn a_completed_turn_says_its_tokens() {
        assert_eq!(
            parse(
                r#"{"type":"turn.completed","usage":{"input_tokens":2400,"cached_input_tokens":2000,"output_tokens":31}}"#
            ),
            Line::Usage(Usage {
                input: 2400,
                output: 31,
                cache_read: 2000,
                ..Usage::default()
            })
        );
    }
}
