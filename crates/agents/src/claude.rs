//! Claude Code: https://code.claude.com/docs/en/hooks

use std::path::{Path, PathBuf};

use serde_json::Value;
use vultures_ai_agent_config::HookEntry;
use vultures_ai_core::{Activity, AgentEvent, AgentUpdate, RequestId, SessionKey, Step};
use vultures_ai_protocol::{AgentKind, Event};

use crate::{Agent, detail, hook_command, target};

#[derive(Debug)]
pub struct Claude;

/// Events we register, with the timeout written to settings.json. PermissionRequest waits for
/// a human, so it gets the hook's decision budget plus a margin.
const EVENTS: &[(&str, u64)] = &[
    ("SessionStart", 10),
    ("SessionEnd", 10),
    ("UserPromptSubmit", 10),
    ("PreToolUse", 10),
    ("PostToolUse", 10),
    ("PostToolUseFailure", 10),
    ("PermissionRequest", 120),
    ("Notification", 10),
    ("Stop", 10),
    ("StopFailure", 10),
    ("SubagentStart", 10),
    ("SubagentStop", 10),
];

/// Tools whose "permission" is really the agent asking the user something.
const QUESTION_TOOLS: &[&str] = &["AskUserQuestion"];

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
            "PreToolUse" => {
                let tool = tool();
                AgentEvent::ToolStarted(Step {
                    activity: activity(&tool),
                    detail: detail(&input),
                    tool,
                })
            }
            "PostToolUse" => AgentEvent::ToolFinished { failed: false },
            "PostToolUseFailure" => AgentEvent::ToolFinished { failed: true },
            // The agent asking the user something is a question, not a permission: an Allow /
            // Deny card would swallow it. The terminal shows the question itself.
            "PermissionRequest" if QUESTION_TOOLS.contains(&text("tool_name")) => AgentEvent::Question {
                message: input
                    .get("questions")
                    .and_then(|q| q.get(0))
                    .and_then(|q| q.get("question"))
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
            },
            "PermissionRequest" => {
                let tool = tool();
                AgentEvent::PermissionRequested {
                    request: RequestId(e.id.clone()),
                    target: target(&tool, &input),
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
            "Stop" => AgentEvent::Stopped,
            "StopFailure" => AgentEvent::StopFailed,
            "SubagentStart" => AgentEvent::SubagentStarted,
            "SubagentStop" => AgentEvent::SubagentStopped,
            _ => return None,
        };

        Some(AgentUpdate {
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
            .map(|&(event, timeout_secs)| HookEntry {
                event,
                command: hook_command(hook_exe, "claude", event),
                timeout_secs,
            })
            .collect()
    }
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
    use vultures_ai_protocol::Terminal;

    fn event(name: &str, payload: Value) -> Event {
        Event {
            v: 1,
            id: "req-1".into(),
            agent: AgentKind::Claude,
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
            })
        );
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
        assert_eq!(permission.timeout_secs, 120);
        assert_eq!(
            permission.command,
            "'/opt/vultures-ai-hook' --agent claude PermissionRequest"
        );
    }
}
