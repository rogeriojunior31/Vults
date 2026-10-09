//! Qwen Code: https://github.com/QwenLM/qwen-code/blob/main/docs/users/features/hooks.md
//!
//! A Claude Code fork in its hooks: the same events, JSON and PermissionRequest answer, in
//! `~/.qwen/settings.json`. Two things differ. Its tools go by Gemini CLI's names
//! (`run_shell_command`, `write_file`…), read here as Claude Code's. And its questions
//! (`ask_user_question`) arrive only as a PermissionRequest whose `allow` it ignores, its
//! PreToolUse coming after the user answered: they stay in its terminal.

use std::path::{Path, PathBuf};

use serde_json::Value;
use vults_agent_config::HookEntry;
use vults_core::AgentUpdate;
use vults_protocol::{AgentKind, Event};

use crate::{Agent, Claude, hook_command};

#[derive(Debug)]
pub struct Qwen;

/// Events we register, with the timeout written to settings.json (seconds, as Claude Code's).
/// PermissionRequest waits for a human: the hook's decision budget plus a margin. Qwen runs it
/// before showing its own prompt, which comes up as soon as the hook prints nothing.
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

impl Agent for Qwen {
    fn kind(&self) -> AgentKind {
        AgentKind::Qwen
    }

    fn parse(&self, e: &Event) -> Option<AgentUpdate> {
        let mut e = e.clone();
        // Qwen runs UserPromptSubmit again, with an empty prompt, when it hands a tool's result
        // back to the model: the same turn going on, not a new prompt.
        if e.event == "UserPromptSubmit" && e.payload.get("prompt").and_then(Value::as_str) == Some("") {
            return None;
        }
        if let Some(name) = e.payload.get_mut("tool_name")
            && let Some(claude) = name.as_str().and_then(claude_tool)
        {
            *name = Value::String(claude.into());
        }
        let mut update = Claude.parse(&e)?;
        update.session.agent = AgentKind::Qwen;
        Some(update)
    }

    fn config_file(&self, home: &Path) -> PathBuf {
        home.join(".qwen").join("settings.json")
    }

    fn hook_entries(&self, hook_exe: &Path) -> Vec<HookEntry> {
        EVENTS
            .iter()
            .map(|&(event, timeout)| HookEntry {
                event,
                command: hook_command(hook_exe, "qwen", event),
                timeout,
                status_message: (event == "PermissionRequest").then_some(crate::WAITING),
            })
            .collect()
    }
}

/// Qwen Code's built-in tools (0.25), under Claude Code's names.
fn claude_tool(name: &str) -> Option<&'static str> {
    Some(match name {
        "run_shell_command" => "Bash",
        "read_file" => "Read",
        "write_file" => "Write",
        "edit" => "Edit",
        "list_directory" => "LS",
        "glob" => "Glob",
        "grep_search" => "Grep",
        "web_fetch" => "WebFetch",
        "web_search" => "WebSearch",
        "todo_write" => "TodoWrite",
        "exit_plan_mode" => "ExitPlanMode",
        "agent" => "Task",
        // So its question reads as one, not as a permission to allow.
        "ask_user_question" => vults_protocol::QUESTION_TOOL,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use vults_core::{Activity, AgentEvent, Step};
    use vults_protocol::Terminal;

    /// Hook calls recorded from a real Qwen Code 0.25.0 session in its default approval mode
    /// (a shell command, a file written, edited and read, a failing command, a question), with
    /// `transcript_path` and the temporary folders trimmed. Its permissions were allowed by the
    /// hook; the question was answered in its terminal.
    const SESSION: &str = include_str!("../tests/fixtures/qwen-session.jsonl");

    fn event(v: Value) -> Event {
        let name = v["hook_event_name"].as_str().unwrap().to_string();
        Event {
            v: vults_protocol::VERSION,
            id: "r1".into(),
            agent: AgentKind::Qwen,
            agent_name: None,
            wants_reply: name == "PermissionRequest",
            event: name,
            terminal: Terminal::default(),
            payload: v,
        }
    }

    fn session() -> Vec<AgentUpdate> {
        SESSION
            .lines()
            .filter_map(|l| Qwen.parse(&event(serde_json::from_str(l).unwrap())))
            .collect()
    }

    #[test]
    fn a_real_session_reads_as_claude_codes() {
        let updates = session();
        assert!(updates.iter().all(|u| u.session.agent == AgentKind::Qwen
            && u.session.session_id == "a2de7f56-6b26-484f-81bb-d3f822dc25e2"
            && u.cwd.as_deref() == Some("/home/me/notes")));
        let events: Vec<&AgentEvent> = updates.iter().map(|u| &u.event).collect();
        assert_eq!(events.first(), Some(&&AgentEvent::SessionStarted));
        assert_eq!(events.last(), Some(&&AgentEvent::SessionEnded));
        // One prompt per turn: the empty ones Qwen sends with a tool's result are dropped.
        let prompts = events
            .iter()
            .filter(|e| matches!(e, AgentEvent::PromptSubmitted))
            .count();
        assert_eq!(prompts, 6);
        assert!(events.contains(&&AgentEvent::ToolStarted(Step {
            activity: Activity::Run,
            detail: Some("echo hi".into()),
            tool: "Bash".into(),
        })));
        assert!(events.contains(&&AgentEvent::ToolStarted(Step {
            activity: Activity::Edit,
            detail: Some("notes.txt".into()),
            tool: "Write".into(),
        })));
        assert!(events.contains(&&AgentEvent::Stopped {
            message: Some("Done: it worked.".into()),
        }));
    }

    #[test]
    fn its_permissions_get_a_card_and_its_question_does_not() {
        let updates = session();
        let cards: Vec<&str> = updates
            .iter()
            .filter_map(|u| match &u.event {
                AgentEvent::PermissionRequested { target, .. } => Some(target.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(
            cards,
            [
                "Write · /home/me/notes/notes.txt",
                "Edit · /home/me/notes/notes.txt",
                "Bash · false",
            ]
        );
        assert!(updates.iter().any(|u| matches!(&u.event,
            AgentEvent::Question { message } if message.contains("Which color?"))));
    }

    #[test]
    fn an_edit_shows_its_diff_and_a_failure_fails() {
        let updates = session();
        let edit = updates.iter().find_map(|u| match &u.event {
            AgentEvent::ToolFinished {
                target: Some(t),
                diff: Some(d),
                ..
            } if t.starts_with("Edit") => Some(d),
            _ => None,
        });
        let lines = &edit.expect("an edit with a diff").files[0].hunks[0].lines;
        assert!(lines.contains(&"-two".to_string()) && lines.contains(&"+three".to_string()));
        assert!(updates.iter().any(|u| matches!(&u.event,
            AgentEvent::ToolFinished { failed: true, target: Some(t), .. } if t == "Bash · false")));
    }

    #[test]
    fn the_installer_writes_claude_codes_hooks_in_qwens_settings() {
        assert_eq!(
            Qwen.config_file(Path::new("/home/me")),
            Path::new("/home/me/.qwen/settings.json")
        );
        let entries = Qwen.hook_entries(Path::new("/opt/vults-hook"));
        assert_eq!(entries.len(), EVENTS.len());
        let permission = entries.iter().find(|e| e.event == "PermissionRequest").unwrap();
        assert_eq!(
            permission.command,
            "'/opt/vults-hook' --agent qwen PermissionRequest"
        );
        assert_eq!(permission.timeout, 120);
        assert!(permission.status_message.is_some());
        // Nothing waits on PreToolUse: Qwen asks its questions before it.
        let pre = entries.iter().find(|e| e.event == "PreToolUse").unwrap();
        assert_eq!(pre.command, "'/opt/vults-hook' --agent qwen PreToolUse");
        assert_eq!(pre.timeout, 10);
    }
}
