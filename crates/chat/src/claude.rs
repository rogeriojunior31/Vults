//! `claude -p --output-format stream-json`: text arrives as `content_block_delta` events.

use serde_json::Value;
use tokio::process::Command;

use crate::{Line, PERSONA, Turn, file_dirs, prompt};

pub(crate) fn command(turn: &Turn, session: Option<&str>) -> Command {
    let mut cmd = Command::new("claude");
    cmd.args([
        "-p",
        "--output-format",
        "stream-json",
        "--verbose",
        "--include-partial-messages",
    ])
    // Only project settings, from our empty folder: no user hooks, so the chat never shows up
    // on the island as a session, and no user permission rules.
    .args(["--setting-sources", "project"])
    .args(["--tools", "Read", "Glob", "Grep"])
    .args(["--append-system-prompt", PERSONA]);
    for dir in file_dirs(turn) {
        cmd.arg("--add-dir").arg(dir);
    }
    if let Some(id) = session {
        cmd.args(["--resume", id]);
    }
    // `--` so a message starting with a dash is never read as a flag.
    cmd.arg("--").arg(prompt(turn, session.is_none(), false));
    cmd
}

pub(crate) fn parse(raw: &str) -> Line {
    let Ok(v) = serde_json::from_str::<Value>(raw) else {
        return Line::Other;
    };
    match v.get("type").and_then(Value::as_str) {
        Some("system") if v.get("subtype").and_then(Value::as_str) == Some("init") => v
            .get("session_id")
            .and_then(Value::as_str)
            .map_or(Line::Other, |s| Line::Session(s.to_string())),
        Some("stream_event") => {
            let delta = &v["event"]["delta"];
            if v["event"]["type"] == "content_block_delta" && delta["type"] == "text_delta" {
                delta["text"]
                    .as_str()
                    .map_or(Line::Other, |t| Line::Text(t.to_string()))
            } else {
                Line::Other
            }
        }
        Some("result") if v.get("is_error").and_then(Value::as_bool) == Some(true) => Line::Failed(
            v.get("result")
                .and_then(Value::as_str)
                .unwrap_or("the turn failed")
                .to_string(),
        ),
        _ => Line::Other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Lines recorded from claude 2.1.285.
    #[test]
    fn stream_lines() {
        assert_eq!(
            parse(r#"{"type":"system","subtype":"init","session_id":"fb2e2e41-1","tools":["Read"]}"#),
            Line::Session("fb2e2e41-1".into())
        );
        assert_eq!(
            parse(
                r#"{"type":"stream_event","event":{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"hello there"}},"session_id":"x"}"#
            ),
            Line::Text("hello there".into())
        );
        assert_eq!(
            parse(r#"{"type":"stream_event","event":{"type":"message_stop"}}"#),
            Line::Other
        );
        assert_eq!(
            parse(r#"{"type":"result","subtype":"success","is_error":false,"result":"hello there"}"#),
            Line::Other
        );
        assert_eq!(
            parse(
                r#"{"type":"result","subtype":"error","is_error":true,"result":"Credit balance is too low"}"#
            ),
            Line::Failed("Credit balance is too low".into())
        );
        assert_eq!(parse("not json"), Line::Other);
    }

    #[test]
    fn resumes_and_reads_dropped_files() {
        let turn = Turn {
            text: "-x?".into(),
            files: vec!["/inbox/a.txt".into()],
        };
        let args: Vec<String> = command(&turn, Some("abc"))
            .as_std()
            .get_args()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        assert!(args.windows(2).any(|w| w == ["--resume", "abc"]));
        assert!(args.windows(2).any(|w| w == ["--add-dir", "/inbox"]));
        assert!(args.windows(2).any(|w| w == ["--setting-sources", "project"]));
        assert_eq!(args.iter().rev().nth(1).map(String::as_str), Some("--"));
    }
}
