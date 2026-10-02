//! Gemini CLI: https://geminicli.com/docs/hooks/reference/
//!
//! Its hook JSON is Claude Code's shape (`session_id`, `cwd`, `tool_name`, `tool_input`) under
//! its own event names, and timeouts are in milliseconds. A hook can deny a tool or force a
//! prompt, but not approve one: Gemini's own confirmation always runs. So a permission shows on
//! the island as needing the user in the terminal, and the hook never waits for a click.

use std::path::{Path, PathBuf};

use serde_json::Value;
use vultures_ai_agent_config::HookEntry;
use vultures_ai_core::{Activity, AgentEvent, AgentUpdate, SessionKey, Step};
use vultures_ai_protocol::{AgentKind, Event};

use crate::{Agent, detail, hook_command, target};

#[derive(Debug)]
pub struct Gemini;

/// Events we register, with their timeout in milliseconds. `BeforeModel` / `AfterModel` are left
/// out on purpose: `AfterModel` fires on every streamed chunk.
const EVENTS: &[(&str, u64)] = &[
    ("SessionStart", 10_000),
    ("SessionEnd", 5_000),
    ("BeforeAgent", 10_000),
    ("AfterAgent", 10_000),
    ("BeforeTool", 10_000),
    ("AfterTool", 10_000),
    ("Notification", 10_000),
];

impl Agent for Gemini {
    fn kind(&self) -> AgentKind {
        AgentKind::Gemini
    }

    fn parse(&self, e: &Event) -> Option<AgentUpdate> {
        let p = &e.payload;
        let text = |k: &str| p.get(k).and_then(Value::as_str).unwrap_or_default();
        let tool = || text("tool_name").to_string();
        let input = p.get("tool_input").cloned().unwrap_or(Value::Null);

        let event = match e.event.as_str() {
            "SessionStart" => AgentEvent::SessionStarted,
            "SessionEnd" => AgentEvent::SessionEnded,
            "BeforeAgent" => AgentEvent::PromptSubmitted,
            "BeforeTool" => {
                let tool = tool();
                AgentEvent::ToolStarted(Step {
                    activity: activity(&tool),
                    detail: detail(&input),
                    tool,
                })
            }
            "AfterTool" => AgentEvent::ToolFinished {
                failed: p.pointer("/tool_response/error").is_some_and(|e| !e.is_null()),
                target: Some(target(&tool(), &input)),
            },
            "AfterAgent" => AgentEvent::Stopped {
                message: crate::filled(text("prompt_response")),
            },
            // Gemini is asking in its terminal; only there can the user answer.
            "Notification" if text("notification_type") == "ToolPermission" => AgentEvent::Question {
                message: permission(p),
            },
            _ => return None,
        };

        Some(AgentUpdate {
            terminal: e.terminal.clone(),
            agent_id: None,
            session: SessionKey {
                agent: AgentKind::Gemini,
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
        home.join(".gemini").join("settings.json")
    }

    fn hook_entries(&self, hook_exe: &Path) -> Vec<HookEntry> {
        EVENTS
            .iter()
            .map(|&(event, timeout)| HookEntry {
                event,
                command: hook_command(hook_exe, "gemini", event),
                timeout,
                status_message: None,
            })
            .collect()
    }
}

/// What the permission is for, from the notification's details: the command, the file, or
/// Gemini's own title for it.
fn permission(p: &Value) -> String {
    let details = &p["details"];
    let field = |k: &str| details.get(k).and_then(Value::as_str).filter(|s| !s.is_empty());
    let what = field("command")
        .map(|c| format!("Run {c}"))
        .or_else(|| field("filePath").map(|f| format!("Edit {f}")))
        .or_else(|| field("title").map(str::to_string))
        .unwrap_or_else(|| "A tool".into());
    format!("{what}? Answer in Gemini's terminal.")
}

fn activity(tool: &str) -> Activity {
    match tool {
        "read_file" | "read_many_files" | "list_directory" | "read_mcp_resource" | "list_mcp_resources" => {
            Activity::Read
        }
        "glob" | "grep_search" | "search_file_content" => Activity::Search,
        "write_file" | "replace" => Activity::Edit,
        "run_shell_command" => Activity::Run,
        "google_web_search" | "web_fetch" => Activity::Web,
        "write_todos" | "enter_plan_mode" | "exit_plan_mode" => Activity::Plan,
        "invoke_agent" => Activity::Subagent,
        t if t.starts_with("mcp_") => Activity::Web,
        _ => Activity::Work,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use vultures_ai_protocol::Terminal;

    /// Hook calls recorded from a real Gemini CLI 0.62 session (`echo hi` in the shell), with
    /// `transcript_path` and the temporary folders trimmed.
    const SESSION: &str = include_str!("../tests/fixtures/gemini-session.jsonl");

    fn event(name: &str, payload: Value) -> Event {
        Event {
            v: vultures_ai_protocol::VERSION,
            id: "r1".into(),
            agent: AgentKind::Gemini,
            agent_name: None,
            event: name.into(),
            wants_reply: false,
            terminal: Terminal::default(),
            payload,
        }
    }

    #[test]
    fn a_real_session_reads_as_steps() {
        let updates: Vec<AgentUpdate> = SESSION
            .lines()
            .map(|l| serde_json::from_str::<Value>(l).unwrap())
            .filter_map(|v| {
                let name = v["hook_event_name"].as_str().unwrap().to_string();
                Gemini.parse(&event(&name, v))
            })
            .collect();
        let events: Vec<&AgentEvent> = updates.iter().map(|u| &u.event).collect();
        assert_eq!(
            events,
            vec![
                &AgentEvent::SessionStarted,
                &AgentEvent::PromptSubmitted,
                &AgentEvent::ToolStarted(Step {
                    activity: Activity::Run,
                    detail: Some("echo hi".into()),
                    tool: "run_shell_command".into(),
                }),
                &AgentEvent::ToolFinished {
                    failed: false,
                    target: Some("run_shell_command · echo hi".into()),
                },
                &AgentEvent::Stopped {
                    message: Some("Done: it printed hi.".into()),
                },
                &AgentEvent::SessionEnded,
            ]
        );
        let first = &updates[0];
        assert_eq!(first.session.agent, AgentKind::Gemini);
        assert_eq!(first.session.session_id, "3b69689a-3884-4fb8-be1f-6e0ecfd6649d");
        assert_eq!(first.cwd.as_deref(), Some("/home/me/proj"));
    }

    #[test]
    fn a_permission_is_a_question_for_its_terminal() {
        let ask = |details: Value| {
            Gemini
                .parse(&event(
                    "Notification",
                    json!({ "session_id": "s", "notification_type": "ToolPermission", "details": details }),
                ))
                .map(|u| u.event)
        };
        assert_eq!(
            ask(json!({ "type": "exec", "title": "Shell", "command": "rm -rf build" })),
            Some(AgentEvent::Question {
                message: "Run rm -rf build? Answer in Gemini's terminal.".into()
            })
        );
        assert_eq!(
            ask(json!({ "type": "edit", "title": "Edit", "filePath": "/p/a.rs" })),
            Some(AgentEvent::Question {
                message: "Edit /p/a.rs? Answer in Gemini's terminal.".into()
            })
        );
        // Any other notification is not ours to show.
        let other = event(
            "Notification",
            json!({ "session_id": "s", "notification_type": "Other" }),
        );
        assert_eq!(Gemini.parse(&other), None);
    }

    #[test]
    fn a_failed_tool_says_so() {
        let u = Gemini
            .parse(&event(
                "AfterTool",
                json!({ "session_id": "s", "tool_name": "read_file", "tool_input": { "file_path": "/x" },
                        "tool_response": { "llmContent": "", "error": { "message": "no such file" } } }),
            ))
            .unwrap();
        assert!(matches!(u.event, AgentEvent::ToolFinished { failed: true, .. }));
    }

    #[test]
    fn install_entries_use_milliseconds_and_no_model_events() {
        let entries = Gemini.hook_entries(Path::new("/opt/vultures-ai-hook"));
        let tool = entries.iter().find(|e| e.event == "BeforeTool").unwrap();
        assert_eq!(tool.timeout, 10_000);
        assert_eq!(tool.command, "'/opt/vultures-ai-hook' --agent gemini BeforeTool");
        assert!(entries.iter().all(|e| !e.event.ends_with("Model")));
        assert!(entries.iter().all(|e| e.status_message.is_none()));
    }
}
