//! Claude Code: https://code.claude.com/docs/en/hooks

use std::path::{Path, PathBuf};

use serde_json::Value;
use vultures_ai_agent_config::HookEntry;
use vultures_ai_core::{Activity, AgentEvent, AgentUpdate, Choice, Question, RequestId, SessionKey, Step};
use vultures_ai_protocol::{ASK_FLAG, AgentKind, Event, QUESTION_TOOL};

use crate::{Agent, detail, hook_command, target};

#[derive(Debug)]
pub struct Claude;

/// Events we register, with the timeout written to settings.json. PermissionRequest waits for
/// a human, and so does PreToolUse for an AskUserQuestion (Claude Code 2.1.85 and later ask it
/// there), so both get the hook's decision budget plus a margin.
const EVENTS: &[(&str, u64)] = &[
    ("SessionStart", 10),
    ("SessionEnd", 10),
    ("UserPromptSubmit", 10),
    ("PreToolUse", 120),
    ("PostToolUse", 10),
    ("PostToolUseFailure", 10),
    ("PermissionRequest", 120),
    ("Notification", 10),
    ("Stop", 10),
    ("StopFailure", 10),
    ("SubagentStart", 10),
    ("SubagentStop", 10),
];

/// Claude Code asks at most 4 questions of 2 to 4 choices; more is not something it sent.
const MAX_QUESTIONS: usize = 4;
const MAX_CHOICES: usize = 4;

impl Agent for Claude {
    fn kind(&self) -> AgentKind {
        AgentKind::Claude
    }

    fn parse(&self, e: &Event) -> Option<AgentUpdate> {
        let p = &e.payload;
        let text = |k: &str| p.get(k).and_then(Value::as_str).unwrap_or_default();
        let tool = || text("tool_name").to_string();
        let input = p.get("tool_input").cloned().unwrap_or(Value::Null);

        let event = match e.event.as_str() {
            "SessionStart" => AgentEvent::SessionStarted,
            "SessionEnd" => AgentEvent::SessionEnded,
            "UserPromptSubmit" => AgentEvent::PromptSubmitted,
            // Only an entry with the ask flag waits, so only then can the island answer it.
            "PreToolUse" if e.wants_reply && text("tool_name") == QUESTION_TOOL => match questions(&input) {
                Some(questions) => AgentEvent::QuestionAsked {
                    request: RequestId(e.id.clone()),
                    target: target(QUESTION_TOOL, &input),
                    questions,
                },
                // Nothing the island can show: the runtime lets it go and the terminal asks.
                None => AgentEvent::Question {
                    message: first_question(&input),
                },
            },
            "PreToolUse" => {
                let tool = tool();
                AgentEvent::ToolStarted(Step {
                    activity: activity(&tool),
                    detail: crate::mcp_label(&tool).or_else(|| detail(&input)),
                    tool,
                })
            }
            "PostToolUse" | "PostToolUseFailure" => AgentEvent::ToolFinished {
                failed: e.event == "PostToolUseFailure",
                target: Some(target(&tool(), &input)),
                diff: (e.event == "PostToolUse")
                    .then(|| crate::diff::claude(&tool(), &input, p.get("tool_response")))
                    .flatten(),
            },
            // The agent asking the user something is a question, not a permission: an Allow /
            // Deny card would swallow it. Here the terminal shows the question itself.
            "PermissionRequest" if text("tool_name") == QUESTION_TOOL => AgentEvent::Question {
                message: first_question(&input),
            },
            "PermissionRequest" => {
                let tool = tool();
                AgentEvent::PermissionRequested {
                    request: RequestId(e.id.clone()),
                    target: target(&tool, &input),
                    ask: crate::ask(&tool, &input),
                    tool,
                }
            }
            "Notification" => {
                let message = text("message");
                let lower = message.to_lowercase();
                if lower.contains("rate limit") || lower.contains("usage limit") {
                    AgentEvent::RateLimited
                } else if message.trim_end().ends_with('?') {
                    AgentEvent::Question {
                        message: message.to_string(),
                    }
                } else {
                    return None;
                }
            }
            "Stop" => AgentEvent::Stopped {
                message: crate::summary_line(text("last_assistant_message")),
            },
            "StopFailure" => AgentEvent::StopFailed {
                // The details say what went wrong; `error` is only its kind ("rate_limit"…).
                error: crate::filled(text("error_details")).or_else(|| crate::filled(text("error"))),
            },
            "SubagentStart" => AgentEvent::SubagentStarted,
            "SubagentStop" => AgentEvent::SubagentStopped,
            _ => return None,
        };

        Some(AgentUpdate {
            terminal: e.terminal.clone(),
            agent_id: crate::filled(text("agent_id")),
            session: SessionKey {
                agent: AgentKind::Claude,
                session_id: text("session_id").to_string(),
            },
            cwd: e
                .terminal
                .cwd
                .clone()
                .or_else(|| Some(text("cwd").to_string()).filter(|c| !c.is_empty())),
            event,
        })
    }

    fn config_file(&self, home: &Path) -> PathBuf {
        home.join(".claude").join("settings.json")
    }

    fn hook_entries(&self, hook_exe: &Path) -> Vec<HookEntry> {
        EVENTS
            .iter()
            .map(|&(event, timeout)| HookEntry {
                event,
                command: match event {
                    "PreToolUse" => hook_command(hook_exe, "claude", &format!("{ASK_FLAG} {event}")),
                    _ => hook_command(hook_exe, "claude", event),
                },
                timeout,
                status_message: (event == "PermissionRequest").then_some(crate::WAITING),
            })
            .collect()
    }

    /// The plan's usage reaches nothing but the statusLine command. Ours prints nothing, so
    /// Claude Code's screen stays as it was.
    fn status_line(&self, hook_exe: &Path) -> Option<String> {
        Some(hook_command(hook_exe, "claude", "--statusline"))
    }
}

/// AskUserQuestion's questions, as `{questions: [{question, header, options: [{label,
/// description}], multiSelect}]}`. `None` when there is nothing to put on a card.
fn questions(input: &Value) -> Option<Vec<Question>> {
    let text = |v: &Value, k: &str| {
        v.get(k)
            .and_then(Value::as_str)
            .map(str::trim)
            .unwrap_or_default()
            .to_string()
    };
    let list = input.get("questions")?.as_array()?;
    if list.is_empty() || list.len() > MAX_QUESTIONS {
        return None;
    }
    list.iter()
        .map(|q| {
            let options = q.get("options")?.as_array()?;
            if options.is_empty() || options.len() > MAX_CHOICES {
                return None;
            }
            let question = Question {
                question: text(q, "question"),
                header: text(q, "header"),
                options: options
                    .iter()
                    .map(|o| Choice {
                        label: text(o, "label"),
                        description: crate::filled(&text(o, "description")),
                    })
                    .collect(),
                multi: q.get("multiSelect").and_then(Value::as_bool).unwrap_or(false),
            };
            let blank = question.question.is_empty() || question.options.iter().any(|o| o.label.is_empty());
            (!blank).then_some(question)
        })
        .collect()
}

fn first_question(input: &Value) -> String {
    input
        .get("questions")
        .and_then(|q| q.get(0))
        .and_then(|q| q.get("question"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

fn activity(tool: &str) -> Activity {
    match tool {
        "Read" | "LS" | "NotebookRead" => Activity::Read,
        "Grep" | "Glob" => Activity::Search,
        "Edit" | "MultiEdit" | "Write" | "NotebookEdit" => Activity::Edit,
        "Bash" | "PowerShell" | "BashOutput" | "KillShell" => Activity::Run,
        "WebSearch" | "WebFetch" => Activity::Web,
        "TodoWrite" | "ExitPlanMode" => Activity::Plan,
        "Task" | "Agent" => Activity::Subagent,
        t if t.starts_with("mcp__") => Activity::Web,
        _ => Activity::Work,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use vultures_ai_core::Ask;
    use vultures_ai_protocol::Terminal;

    fn event(name: &str, payload: Value) -> Event {
        Event {
            v: vultures_ai_protocol::VERSION,
            id: "req-1".into(),
            agent: AgentKind::Claude,
            agent_name: None,
            event: name.into(),
            wants_reply: name == "PermissionRequest",
            terminal: Terminal {
                cwd: Some("/home/me/site".into()),
                ..Default::default()
            },
            payload,
        }
    }

    fn parse(name: &str, payload: Value) -> Option<AgentEvent> {
        Claude.parse(&event(name, payload)).map(|u| u.event)
    }

    #[test]
    fn tool_use_becomes_an_activity_with_a_short_detail() {
        let u = Claude
            .parse(&event(
                "PreToolUse",
                json!({ "session_id": "s1", "tool_name": "Edit", "tool_input": { "file_path": "/home/me/site/src/main.rs" } }),
            ))
            .unwrap();
        assert_eq!(
            u.session,
            SessionKey {
                agent: AgentKind::Claude,
                session_id: "s1".into()
            }
        );
        assert_eq!(u.cwd.as_deref(), Some("/home/me/site"));
        assert_eq!(
            u.event,
            AgentEvent::ToolStarted(Step {
                activity: Activity::Edit,
                tool: "Edit".into(),
                detail: Some("main.rs".into())
            })
        );
    }

    #[test]
    fn activities() {
        assert_eq!(activity("Grep"), Activity::Search);
        assert_eq!(activity("Bash"), Activity::Run);
        assert_eq!(activity("mcp__github__list_prs"), Activity::Web);
        assert_eq!(activity("SomethingNew"), Activity::Work);
    }

    #[test]
    fn a_permission_shows_what_allow_authorizes() {
        let e = parse(
            "PermissionRequest",
            json!({ "session_id": "s1", "tool_name": "Bash", "tool_input": { "command": "rm -rf build", "description": "Clean" } }),
        );
        assert_eq!(
            e,
            Some(AgentEvent::PermissionRequested {
                request: RequestId("req-1".into()),
                tool: "Bash".into(),
                target: "Bash · rm -rf build".into(),
                ask: Ask {
                    description: Some("Clean".into()),
                    ..Ask::default()
                },
            })
        );
    }

    /// Edit, Write of a new file, Write over it: recorded from Claude Code 2.1.287 (paths replaced).
    #[test]
    fn a_finished_edit_carries_its_diff() {
        let diffs: Vec<vultures_ai_core::Diff> = include_str!("../tests/fixtures/claude-edits.jsonl")
            .lines()
            .map(|l| serde_json::from_str::<Value>(l).unwrap())
            .map(|p| match parse("PostToolUse", p) {
                Some(AgentEvent::ToolFinished { diff: Some(d), .. }) => d,
                other => panic!("no diff: {other:?}"),
            })
            .collect();
        let lines = |d: &vultures_ai_core::Diff| d.files[0].hunks[0].lines.clone();
        assert_eq!(diffs[0].files[0].path, "/home/me/notes/notes.txt");
        assert_eq!(lines(&diffs[0]), [" alpha", "-beta", "+BETA", " gamma", " delta"]);
        assert_eq!(diffs[0].files[0].hunks[0].new_start, Some(1));
        assert_eq!(lines(&diffs[1]), ["+one", "+two"]);
        // "\ No newline at end of file" is left out.
        assert_eq!(lines(&diffs[2]), [" one", "-two", "+three"]);
        assert!(diffs.iter().all(|d| !d.cut));
        // A failed edit changed nothing.
        let failed = parse(
            "PostToolUseFailure",
            json!({ "tool_name": "Edit", "tool_input": { "file_path": "/a" } }),
        );
        assert!(matches!(
            failed,
            Some(AgentEvent::ToolFinished { diff: None, .. })
        ));
    }

    #[test]
    fn a_finished_call_names_its_card() {
        let call = json!({ "tool_name": "WebFetch", "tool_input": { "url": "https://a.dev" } });
        let Some(AgentEvent::PermissionRequested { target, .. }) = parse("PermissionRequest", call.clone())
        else {
            panic!("not a permission");
        };
        assert_eq!(
            parse("PostToolUse", call),
            Some(AgentEvent::ToolFinished {
                failed: false,
                target: Some(target),
                diff: None,
            })
        );
    }

    #[test]
    fn a_card_gets_the_whole_command_and_an_edit_its_line_counts() {
        let ask = |tool: &str, input: Value| match parse(
            "PermissionRequest",
            json!({ "tool_name": tool, "tool_input": input }),
        ) {
            Some(AgentEvent::PermissionRequested { ask, .. }) => ask,
            other => panic!("{other:?}"),
        };
        let long = ask("Bash", json!({ "command": "cargo build\ncargo test" }));
        assert_eq!(long.full.as_deref(), Some("cargo build\ncargo test"));
        assert_eq!(ask("Bash", json!({ "command": "ls" })).full, None);
        let edit = ask(
            "Edit",
            json!({ "file_path": "/a.rs", "old_string": "fn a() {\n    1\n}", "new_string": "fn a() {\n    2\n    3\n}" }),
        );
        assert_eq!((edit.added, edit.removed), (2, 1));
        let write = ask("Write", json!({ "file_path": "/b.rs", "content": "a\nb\nc" }));
        assert_eq!((write.added, write.removed), (3, 0));
        let multi = ask(
            "MultiEdit",
            json!({ "file_path": "/c.rs", "edits": [ { "old_string": "x", "new_string": "y" }, { "old_string": "", "new_string": "z\nw" } ] }),
        );
        assert_eq!((multi.added, multi.removed), (3, 1));
    }

    #[test]
    fn a_subagent_is_named_on_its_events() {
        let u = Claude
            .parse(&event(
                "PreToolUse",
                json!({ "session_id": "s", "agent_id": "a-7", "tool_name": "Read", "tool_input": {} }),
            ))
            .unwrap();
        assert_eq!(u.agent_id.as_deref(), Some("a-7"));
        let main = Claude
            .parse(&event(
                "PreToolUse",
                json!({ "session_id": "s", "tool_name": "Read", "tool_input": {} }),
            ))
            .unwrap();
        assert_eq!(main.agent_id, None);
    }

    #[test]
    fn a_question_is_never_an_approval_card() {
        let e = parse(
            "PermissionRequest",
            json!({ "tool_name": "AskUserQuestion", "tool_input": { "questions": [ { "question": "Which theme?" } ] } }),
        );
        assert_eq!(
            e,
            Some(AgentEvent::Question {
                message: "Which theme?".into()
            })
        );
    }

    #[test]
    fn a_question_from_the_ask_entry_is_a_card() {
        // Recorded from Claude Code 2.1.286 (PreToolUse, then the answer it accepted).
        let fixture = include_str!("../tests/fixtures/claude-ask-user-question.jsonl");
        let payload: Value = serde_json::from_str(fixture.lines().next().unwrap()).unwrap();
        let mut e = event("PreToolUse", payload);
        e.wants_reply = true;
        let Some(AgentEvent::QuestionAsked {
            request,
            target,
            questions,
        }) = Claude.parse(&e).map(|u| u.event)
        else {
            panic!("not a question card");
        };
        assert_eq!(request, RequestId("req-1".into()));
        assert_eq!(target, "AskUserQuestion");
        assert_eq!(questions.len(), 2);
        assert_eq!(questions[0].question, "Which color?");
        assert_eq!(questions[0].header, "Color");
        assert!(!questions[0].multi && questions[1].multi);
        assert_eq!(questions[1].options[0].label, "S");
        assert_eq!(questions[1].options[0].description.as_deref(), Some("Small size"));

        // From an entry without the flag it is only a step: that hook can't wait.
        e.wants_reply = false;
        assert!(matches!(
            Claude.parse(&e).map(|u| u.event),
            Some(AgentEvent::ToolStarted(_))
        ));
    }

    #[test]
    fn a_question_the_island_cant_show_goes_to_the_terminal() {
        let mut e = event(
            "PreToolUse",
            json!({ "tool_name": "AskUserQuestion", "tool_input": { "questions": [ { "question": "Which?", "options": [] } ] } }),
        );
        e.wants_reply = true;
        assert_eq!(
            Claude.parse(&e).map(|u| u.event),
            Some(AgentEvent::Question {
                message: "Which?".into()
            })
        );
    }

    #[test]
    fn notifications() {
        assert_eq!(
            parse("Notification", json!({ "message": "Claude hit the usage limit" })),
            Some(AgentEvent::RateLimited)
        );
        assert_eq!(
            parse("Notification", json!({ "message": "Which file?" })),
            Some(AgentEvent::Question {
                message: "Which file?".into()
            })
        );
        assert_eq!(
            parse(
                "Notification",
                json!({ "message": "Claude is waiting for your input" })
            ),
            None
        );
        assert_eq!(parse("SomethingElse", json!({})), None);
    }

    #[test]
    fn install_entries_cover_every_event() {
        let entries = Claude.hook_entries(Path::new("/opt/vultures-ai-hook"));
        assert_eq!(entries.len(), EVENTS.len());
        let permission = entries.iter().find(|e| e.event == "PermissionRequest").unwrap();
        assert_eq!(permission.timeout, 120);
        assert_eq!(permission.status_message, Some(crate::WAITING));
        assert_eq!(
            permission.command,
            "'/opt/vultures-ai-hook' --agent claude PermissionRequest"
        );
        // A question waits for the island there too.
        let pre = entries.iter().find(|e| e.event == "PreToolUse").unwrap();
        assert_eq!(pre.timeout, 120);
        assert_eq!(
            pre.command,
            "'/opt/vultures-ai-hook' --agent claude --ask PreToolUse"
        );
    }

    #[test]
    fn a_stop_carries_the_last_reply_and_a_failure_its_details() {
        assert_eq!(
            parse(
                "Stop",
                json!({ "last_assistant_message": "  All 42 tests pass.\n" })
            ),
            Some(AgentEvent::Stopped {
                message: Some("All 42 tests pass.".into())
            })
        );
        assert_eq!(
            parse("Stop", json!({})),
            Some(AgentEvent::Stopped { message: None })
        );
        assert_eq!(
            parse(
                "StopFailure",
                json!({ "error": "server_error", "error_details": "API Error: 529 overloaded" })
            ),
            Some(AgentEvent::StopFailed {
                error: Some("API Error: 529 overloaded".into())
            })
        );
        assert_eq!(
            parse("StopFailure", json!({ "error": "rate_limit" })),
            Some(AgentEvent::StopFailed {
                error: Some("rate_limit".into())
            })
        );
    }

    #[test]
    fn a_markdown_reply_stops_with_its_first_paragraph() {
        let reply = "## Summary\n\n**Done.** The `ipc` limits now apply per peer.\n\n---\n\n\
                     | File | Change |\n|---|---|\n| lib.rs | +12 -3 |\n\n- Tests pass\n";
        assert_eq!(
            parse(
                "Stop",
                json!({ "session_id": "s1", "hook_event_name": "Stop", "stop_hook_active": false,
                        "last_assistant_message": reply })
            ),
            Some(AgentEvent::Stopped {
                message: Some("Done. The ipc limits now apply per peer.".into())
            })
        );
    }
}
