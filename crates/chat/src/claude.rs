//! `claude -p` with stream-json both ways: the message goes in on stdin, text arrives as
//! `content_block_delta` events, and each permission arrives as a `control_request`
//! (`can_use_tool`, from `--permission-prompt-tool stdio`) answered on stdin. Stop is a
//! `control_request` too (`interrupt`): Claude Code answers it, ends the turn with an
//! `error_during_execution` result, and keeps the session (tested on 2.1.295).

use std::process::Stdio;
use std::time::Duration;

use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;

use vults_agents::driver::{Answer, Ask, EndReason, Event, Settings, Sink, Spec};

use crate::drivers::Stop;
use crate::{PERSONA, Turn, Usage, file_dirs, full, personal, prompt, target};

/// After an interrupt, how long Claude Code may take to end the turn before its process is ended.
const INTERRUPT_GRACE: Duration = Duration::from_secs(4);
/// How long the process may take to exit once its stdin is closed.
const EXIT_GRACE: Duration = Duration::from_secs(2);

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
    /// Tools that began (`tool_use` in an assistant message) or ended (`tool_result`).
    Tools(Vec<Event>),
    Done(Option<Usage>),
    /// The turn ended early (`error_during_execution`): what an interrupt brings.
    Stopped(Option<Usage>),
    Failed(String),
    Other,
}

pub(crate) fn command(spec: &Spec, turn: &Turn, session: Option<&str>) -> Command {
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
    .args(["--permission-prompt-tool", "stdio"]);
    match spec.settings {
        // No settings at all: no hooks and no permission rules that would skip asking. Not even
        // the project's: in a trusted folder its `.claude/settings.json` allow rules run commands
        // with no card (rule 2). Only our MCP servers, if any.
        Settings::Isolated => cmd.args(["--setting-sources", ""]).arg("--strict-mcp-config"),
        Settings::User => cmd.args(["--setting-sources", "user"]),
        Settings::Project => cmd.args(["--setting-sources", "user,project,local"]),
    };
    if spec.settings != Settings::Isolated {
        // The user's rules and MCPs, as in their terminal; but a `defaultMode` that skips the
        // cards (`acceptEdits`, `bypassPermissions`) never applies to the chat.
        cmd.args(["--permission-mode", "default"]);
    }
    if let Some(mcp) = &spec.mcp_config {
        cmd.arg("--mcp-config").arg(mcp);
    }
    let mut system = personal(PERSONA);
    if let Some(more) = &spec.system_append {
        system.push_str("\n\n");
        system.push_str(more);
    }
    cmd.args(["--append-system-prompt", &system]);
    if let Some(model) = &spec.model {
        cmd.args(["--model", model]);
    }
    if let Some(effort) = spec.effort {
        cmd.args(["--effort", effort.name()]);
    }
    for d in spec.add_dirs.iter().cloned().chain(file_dirs(turn)) {
        cmd.arg("--add-dir").arg(d);
    }
    if let Some(id) = session {
        cmd.args(["--resume", id]);
    }
    cmd.envs(spec.env.iter().map(|(k, v)| (k, v)));
    cmd.current_dir(&spec.dir);
    cmd
}

/// The part of a tool's input a card shows: its command, path, URL or pattern.
fn detail_of(input: &Value) -> Option<&str> {
    ["command", "file_path", "path", "url", "pattern"]
        .iter()
        .find_map(|k| input.get(*k).and_then(Value::as_str))
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
            let detail = detail_of(&input)
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
        // Whole messages: the tools the assistant called, and their results coming back.
        Some("assistant") | Some("user") => {
            let blocks = v["message"]["content"]
                .as_array()
                .map(Vec::as_slice)
                .unwrap_or_default();
            let events: Vec<Event> = blocks
                .iter()
                .filter_map(|b| match b["type"].as_str() {
                    Some("tool_use") => {
                        let tool = b["name"].as_str().unwrap_or("tool");
                        Some(Event::ToolStart {
                            id: b["id"].as_str().unwrap_or_default().to_string(),
                            tool: tool.to_string(),
                            target: target(tool, detail_of(&b["input"]).unwrap_or_default()),
                        })
                    }
                    Some("tool_result") => Some(Event::ToolEnd {
                        id: b["tool_use_id"].as_str().unwrap_or_default().to_string(),
                        ok: b["is_error"] != true,
                    }),
                    _ => None,
                })
                .collect();
            if events.is_empty() {
                Line::Other
            } else {
                Line::Tools(events)
            }
        }
        // How Claude Code ends an interrupted turn (2.1.295); its usage still counts.
        Some("result") if v["subtype"] == "error_during_execution" => Line::Stopped(usage_of(&v)),
        Some("result") if v["is_error"] == true => {
            Line::Failed(v["result"].as_str().unwrap_or("the turn failed").to_string())
        }
        Some("result") => Line::Done(usage_of(&v)),
        _ => Line::Other,
    }
}

/// A `result` line's usage, if it reports any.
fn usage_of(v: &Value) -> Option<Usage> {
    let mut usage = Usage::anthropic(&v["usage"]);
    usage.cost_usd = v["total_cost_usd"].as_f64();
    // `modelUsage` covers every model the turn used (a helper or a subagent too), as the
    // cost does: its sums are the turn's tokens, and the costliest model names it.
    if let Some(models) = v["modelUsage"].as_object().filter(|m| !m.is_empty()) {
        let n = |m: &Value, k: &str| m[k].as_u64().unwrap_or(0);
        let sum = |k: &str| models.values().map(|m| n(m, k)).sum::<u64>();
        let (read, write) = (sum("cacheReadInputTokens"), sum("cacheCreationInputTokens"));
        usage.input = sum("inputTokens") + read + write;
        usage.output = sum("outputTokens");
        usage.cache_read = read;
        usage.cache_write = write;
        let cost = |m: &Value| m["costUSD"].as_f64().unwrap_or(0.0);
        usage.model = models
            .iter()
            .max_by(|a, b| cost(a.1).total_cmp(&cost(b.1)))
            .map(|(k, _)| k.clone());
    }
    (!usage.is_empty()).then_some(usage)
}

/// The answer to a `can_use_tool` request.
pub(crate) fn decision(id: &str, allow: bool, input: &Value) -> Value {
    let response = if allow {
        json!({ "behavior": "allow", "updatedInput": input })
    } else {
        json!({ "behavior": "deny", "message": "The user said no in Vults." })
    };
    json!({ "type": "control_response", "response": { "subtype": "success", "request_id": id, "response": response } })
}

/// An interrupt, as Claude Code's control channel takes it.
fn interrupt_request() -> Value {
    json!({ "type": "control_request", "request_id": "vults-interrupt", "request": { "subtype": "interrupt" } })
}

/// One turn. The session id is written to `session` the moment Claude Code says it: a turn
/// dropped midway (after an interrupt that did not end it) still leaves the next one resuming.
pub(crate) async fn turn(
    spec: &Spec,
    session: &mut Option<String>,
    turn: &Turn,
    sink: &dyn Sink,
    stop: &Stop,
) -> Result<(), String> {
    let result = match run(spec, session, turn, sink, stop).await {
        Ok(_) if session.is_none() => Err("Claude returned no answer. Is it logged in?".to_string()),
        other => other,
    };
    let end = match &result {
        Ok(end) => end.clone(),
        Err(e) => EndReason::Failed(e.clone()),
    };
    sink.event(Event::TurnEnd(end)).await;
    result.map(|_| ())
}

async fn run(
    spec: &Spec,
    id: &mut Option<String>,
    turn: &Turn,
    sink: &dyn Sink,
    stop: &Stop,
) -> Result<EndReason, String> {
    let mut cmd = command(spec, turn, id.as_deref());
    let mut child = crate::dies_with_app(&mut cmd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        // Never read: a full pipe would stall the CLI mid-turn.
        .stderr(Stdio::null())
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
    // Once Stop was pressed and the interrupt sent: Claude Code has this long to end the turn.
    let mut deadline: Option<tokio::time::Instant> = None;
    let end = loop {
        let next = tokio::select! {
            biased;
            () = stop.wait(), if deadline.is_none() => {
                // A process already gone can't take it: the turn ends all the same.
                let _ = write(&mut stdin, &interrupt_request()).await;
                deadline = Some(tokio::time::Instant::now() + INTERRUPT_GRACE);
                continue;
            }
            () = sleep_until(deadline) => break EndReason::Interrupted,
            line = lines.next_line() => line,
        };
        let Ok(Some(raw)) = next else {
            break if deadline.is_some() {
                EndReason::Interrupted
            } else {
                EndReason::Done
            };
        };
        match parse(&raw) {
            Line::Session(s) => *id = Some(s),
            Line::Text(t) => sink.event(Event::Text(t)).await,
            Line::Tools(events) => {
                for e in events {
                    sink.event(e).await;
                }
            }
            Line::Permission {
                id: req,
                tool,
                input,
                detail,
            } => {
                // After Stop no card is shown: a request already on its way is a no.
                let answer = if deadline.is_some() {
                    None
                } else {
                    let ask = Ask {
                        id: req.clone(),
                        target: target(&tool, &detail),
                        full: full(&detail),
                        tool,
                        input: input.clone(),
                    };
                    // Stop while a card waits: that card is a no, and the turn is interrupted.
                    tokio::select! {
                        biased;
                        () = stop.wait() => None,
                        answer = sink.ask(ask) => Some(answer),
                    }
                };
                let allow = answer.is_some_and(Answer::allows);
                let written = write(&mut stdin, &decision(&req, allow, &input)).await;
                if answer.is_none() {
                    if deadline.is_none() {
                        let _ = write(&mut stdin, &interrupt_request()).await;
                        deadline = Some(tokio::time::Instant::now() + INTERRUPT_GRACE);
                    }
                } else {
                    written?;
                }
            }
            Line::Stopped(usage) if deadline.is_some() => {
                if let Some(usage) = usage {
                    sink.event(Event::Usage(usage)).await;
                }
                break EndReason::Interrupted;
            }
            Line::Stopped(_) => return Err("Claude: the turn stopped".into()),
            Line::Failed(_) if deadline.is_some() => break EndReason::Interrupted,
            Line::Failed(m) => return Err(format!("Claude: {m}")),
            Line::Done(usage) => {
                if let Some(usage) = usage {
                    sink.event(Event::Usage(usage)).await;
                }
                break if deadline.is_some() {
                    EndReason::Interrupted
                } else {
                    EndReason::Done
                };
            }
            Line::Other => {}
        }
    };
    // Closing stdin ends the process; one that won't is ended.
    drop(stdin);
    if tokio::time::timeout(EXIT_GRACE, child.wait()).await.is_err() {
        let _ = child.kill().await;
    }
    Ok(end)
}

/// Pending forever when there is no deadline.
async fn sleep_until(deadline: Option<tokio::time::Instant>) {
    match deadline {
        Some(at) => tokio::time::sleep_until(at).await,
        None => std::future::pending().await,
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
            Line::Done(None)
        );
        assert_eq!(
            parse(r#"{"type":"result","is_error":true,"result":"Credit balance is too low"}"#),
            Line::Failed("Credit balance is too low".into())
        );
    }

    #[test]
    fn tools_pair_up_and_an_interrupt_is_not_a_failure_line() {
        assert_eq!(
            parse(
                r#"{"type":"assistant","message":{"content":[{"type":"text","text":"x"},{"type":"tool_use","id":"t1","name":"Bash","input":{"command":"cargo test"}}]}}"#
            ),
            Line::Tools(vec![Event::ToolStart {
                id: "t1".into(),
                tool: "Bash".into(),
                target: "Bash · cargo test".into()
            }])
        );
        assert_eq!(
            parse(
                r#"{"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"t1","is_error":false,"content":"ok"}]}}"#
            ),
            Line::Tools(vec![Event::ToolEnd {
                id: "t1".into(),
                ok: true
            }])
        );
        assert_eq!(
            parse(
                r#"{"type":"user","message":{"content":[{"type":"text","text":"[Request interrupted by user]"}]}}"#
            ),
            Line::Other
        );
        // Claude Code 2.1.295 after an interrupt: an error result, read as the end of the turn.
        assert!(matches!(
            parse(r#"{"type":"result","subtype":"error_during_execution","is_error":true,"session_id":"s"}"#),
            Line::Stopped(None)
        ));
    }

    #[test]
    fn the_spec_reaches_the_command_line() {
        use vults_agents::driver::Effort;
        let spec = Spec {
            model: Some("claude-sonnet-5-5".into()),
            effort: Some(Effort::High),
            settings: Settings::Project,
            env: vec![("VULTS_ZECA".into(), "1".into())],
            mcp_config: Some("/run/vults/mcp.json".into()),
            system_append: Some("Frozen context.".into()),
            add_dirs: vec!["/extra".into()],
            ..Spec::in_dir("/w".into())
        };
        let cmd = command(&spec, &Turn::default(), None);
        let args: Vec<String> = cmd
            .as_std()
            .get_args()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        for pair in [
            ["--model", "claude-sonnet-5-5"],
            ["--effort", "high"],
            ["--setting-sources", "user,project,local"],
            ["--permission-mode", "default"],
            ["--mcp-config", "/run/vults/mcp.json"],
            ["--add-dir", "/extra"],
        ] {
            assert!(args.windows(2).any(|w| w == pair), "{pair:?} in {args:?}");
        }
        // The user's own MCP servers join ours.
        assert!(!args.contains(&"--strict-mcp-config".to_string()));
        let system = args
            .windows(2)
            .find(|w| w[0] == "--append-system-prompt")
            .unwrap();
        assert!(system[1].ends_with("Frozen context."));
        let envs: Vec<_> = cmd.as_std().get_envs().collect();
        assert!(envs.contains(&(
            std::ffi::OsStr::new("VULTS_ZECA"),
            Some(std::ffi::OsStr::new("1"))
        )));
        assert_eq!(cmd.as_std().get_current_dir(), Some(std::path::Path::new("/w")));
    }

    #[test]
    fn only_the_users_own_settings_outside_a_trusted_folder() {
        let spec = Spec {
            settings: Settings::User,
            ..Spec::in_dir("/w".into())
        };
        let args: Vec<String> = command(&spec, &Turn::default(), None)
            .as_std()
            .get_args()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        assert!(
            args.windows(2).any(|w| w == ["--setting-sources", "user"]),
            "{args:?}"
        );
        assert!(
            args.windows(2).any(|w| w == ["--permission-mode", "default"]),
            "{args:?}"
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
        let args: Vec<String> = command(&Spec::in_dir("/w".into()), &turn, Some("abc"))
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
            args.windows(2).any(|w| w == ["--setting-sources", ""]),
            "no settings source: a project's allow rules would skip the card"
        );
        assert!(
            !args.contains(&"--tools".to_string()),
            "the chat may use every tool, each one asked"
        );
    }

    // A result line as Claude Code 2.1 writes it (trimmed).
    #[test]
    fn a_result_says_what_the_turn_cost() {
        // A helper model ran first; the main one cost the most.
        let line = r#"{"type":"result","subtype":"success","is_error":false,"result":"ok","total_cost_usd":0.0123,"usage":{"input_tokens":12,"cache_creation_input_tokens":3000,"cache_read_input_tokens":9000,"output_tokens":45},"modelUsage":{"claude-haiku-5-5":{"inputTokens":300,"outputTokens":20,"cacheReadInputTokens":0,"cacheCreationInputTokens":0,"costUSD":0.0003},"claude-sonnet-5-5":{"inputTokens":12,"outputTokens":45,"cacheReadInputTokens":9000,"cacheCreationInputTokens":3000,"costUSD":0.012}}}"#;
        assert_eq!(
            parse(line),
            Line::Done(Some(Usage {
                model: Some("claude-sonnet-5-5".into()),
                input: 312 + 9000 + 3000,
                output: 65,
                cache_read: 9000,
                cache_write: 3000,
                cost_usd: Some(0.0123),
            }))
        );
        // Without modelUsage, the main loop's own usage, the cache counted in.
        let bare = r#"{"type":"result","is_error":false,"result":"ok","usage":{"input_tokens":12,"cache_read_input_tokens":9000,"output_tokens":45}}"#;
        assert!(matches!(parse(bare), Line::Done(Some(Usage { input: 9012, .. }))));
    }
}
