//! `claude -p` with stream-json both ways: the message goes in on stdin, text arrives as
//! `content_block_delta` events, and each permission arrives as a `control_request`
//! (`can_use_tool`, from `--permission-prompt-tool stdio`) answered on stdin.

use std::path::Path;
use std::process::Stdio;

use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;

use crate::{Approver, Delta, PERSONA, Turn, ask, file_dirs, prompt, target};

/// What one stdout line means.
#[derive(Debug, PartialEq)]
pub(crate) enum Line {
    Session(String),
    Text(String),
    Permission {
        id: String,
        tool: String,
        input: Value,
        detail: String,
    },
    Done,
    Failed(String),
    Other,
}

pub(crate) fn command(dir: &Path, turn: &Turn, session: Option<&str>) -> Command {
    let mut cmd = Command::new("claude");
    cmd.args([
        "-p",
        "--input-format",
        "stream-json",
        "--output-format",
        "stream-json",
        "--verbose",
    ])
    .arg("--include-partial-messages")
    .args(["--permission-prompt-tool", "stdio"])
    // Only project settings and no MCP servers: no user hooks (the chat never shows up on the
    // island as a session), no permission rules that would skip asking.
    .args(["--setting-sources", "project", "--strict-mcp-config"])
    .args(["--append-system-prompt", PERSONA]);
    for d in file_dirs(turn) {
        cmd.arg("--add-dir").arg(d);
    }
    if let Some(id) = session {
        cmd.args(["--resume", id]);
    }
    cmd.current_dir(dir);
    cmd
}

pub(crate) fn parse(raw: &str) -> Line {
    let Ok(v) = serde_json::from_str::<Value>(raw) else {
        return Line::Other;
    };
    match v.get("type").and_then(Value::as_str) {
        Some("system") if v["subtype"] == "init" => v["session_id"]
            .as_str()
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
        Some("control_request") if v["request"]["subtype"] == "can_use_tool" => {
            let r = &v["request"];
            let input = r["input"].clone();
            let detail = ["command", "file_path", "path", "url", "pattern"]
                .iter()
                .find_map(|k| input.get(*k).and_then(Value::as_str))
                .or_else(|| r["description"].as_str())
                .unwrap_or_default()
                .to_string();
            Line::Permission {
                id: v["request_id"].as_str().unwrap_or_default().to_string(),
                tool: r["tool_name"].as_str().unwrap_or("tool").to_string(),
                input,
                detail,
            }
        }
        Some("result") if v["is_error"] == true => {
            Line::Failed(v["result"].as_str().unwrap_or("the turn failed").to_string())
        }
        Some("result") => Line::Done,
        _ => Line::Other,
    }
}

/// The answer to a `can_use_tool` request.
pub(crate) fn decision(id: &str, allow: bool, input: &Value) -> Value {
    let response = if allow {
        json!({ "behavior": "allow", "updatedInput": input })
    } else {
        json!({ "behavior": "deny", "message": "The user said no in Vultures AI." })
    };
    json!({ "type": "control_response", "response": { "subtype": "success", "request_id": id, "response": response } })
}

/// One turn; returns the session id to resume next time.
pub(crate) async fn turn(
    dir: &Path,
    session: Option<&str>,
    turn: &Turn,
    out: &mpsc::Sender<Delta>,
    approver: &dyn Approver,
) -> Result<String, String> {
    let mut child = command(dir, turn, session)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => "The Claude CLI isn't installed.".to_string(),
            _ => format!("Can't start Claude: {e}"),
        })?;
    let mut stdin = child.stdin.take().ok_or("no stdin")?;
    let message =
        json!({ "type": "user", "message": { "role": "user", "content": prompt(turn, false, false) } });
    write(&mut stdin, &message).await?;

    let mut lines = BufReader::new(child.stdout.take().ok_or("no stdout")?).lines();
    let mut id = session.map(str::to_string);
    let mut said_anything = false;
    while let Ok(Some(raw)) = lines.next_line().await {
        match parse(&raw) {
            Line::Session(s) => id = Some(s),
            Line::Text(t) => {
                said_anything = true;
                let _ = out.send(Delta::Text { text: t }).await;
            }
            Line::Permission {
                id: req,
                tool,
                input,
                detail,
            } => {
                let yes = ask(approver, out, req.clone(), tool.clone(), target(&tool, &detail)).await;
                write(&mut stdin, &decision(&req, yes, &input)).await?;
            }
            Line::Failed(m) => return Err(format!("Claude: {m}")),
            Line::Done => break,
            Line::Other => {}
        }
    }
    // Closing stdin ends the process.
    drop(stdin);
    let _ = child.wait().await;
    match id {
        Some(id) if said_anything || session.is_some() => Ok(id),
        Some(id) => Ok(id),
        None => Err("Claude returned no answer. Is it logged in?".into()),
    }
}

async fn write(stdin: &mut tokio::process::ChildStdin, msg: &Value) -> Result<(), String> {
    let mut line = msg.to_string();
    line.push('\n');
    stdin
        .write_all(line.as_bytes())
        .await
        .map_err(|e| e.to_string())?;
    stdin.flush().await.map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Lines recorded from claude 2.1.285.
    #[test]
    fn stream_lines() {
        assert_eq!(
            parse(r#"{"type":"system","subtype":"init","session_id":"fb2e2e41-1"}"#),
            Line::Session("fb2e2e41-1".into())
        );
        assert_eq!(
            parse(
                r#"{"type":"stream_event","event":{"type":"content_block_delta","delta":{"type":"text_delta","text":"hi"}}}"#
            ),
            Line::Text("hi".into())
        );
        assert_eq!(
            parse(
                r#"{"type":"control_request","request_id":"fc62","request":{"subtype":"can_use_tool","tool_name":"Bash","input":{"command":"touch perm-test.txt","description":"Create it"},"description":"Create it"}}"#
            ),
            Line::Permission {
                id: "fc62".into(),
                tool: "Bash".into(),
                input: json!({ "command": "touch perm-test.txt", "description": "Create it" }),
                detail: "touch perm-test.txt".into(),
            }
        );
        assert_eq!(
            parse(r#"{"type":"result","subtype":"success","is_error":false,"result":"ok"}"#),
            Line::Done
        );
        assert_eq!(
            parse(r#"{"type":"result","is_error":true,"result":"Credit balance is too low"}"#),
            Line::Failed("Credit balance is too low".into())
        );
    }

    #[test]
    fn answers() {
        let input = json!({ "command": "ls" });
        let yes = decision("r1", true, &input);
        assert_eq!(yes["response"]["request_id"], "r1");
        assert_eq!(yes["response"]["response"]["behavior"], "allow");
        assert_eq!(yes["response"]["response"]["updatedInput"], input);
        assert_eq!(
            decision("r1", false, &input)["response"]["response"]["behavior"],
            "deny"
        );
    }

    #[test]
    fn args() {
        let turn = Turn {
            text: "-x?".into(),
            files: vec!["/inbox/a.txt".into()],
        };
        let args: Vec<String> = command(Path::new("/w"), &turn, Some("abc"))
            .as_std()
            .get_args()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        for pair in [
            ["--resume", "abc"],
            ["--add-dir", "/inbox"],
            ["--permission-prompt-tool", "stdio"],
        ] {
            assert!(args.windows(2).any(|w| w == pair), "{pair:?}");
        }
        assert!(args.contains(&"--strict-mcp-config".to_string()));
        assert!(
            !args.contains(&"--tools".to_string()),
            "the chat may use every tool, each one asked"
        );
    }
}
