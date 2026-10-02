//! Codex CLI: https://learn.chatgpt.com/docs/hooks
//!
//! Same hook JSON shape as Claude Code. Differences: tools are `Bash`, `apply_patch` (the patch in
//! `tool_input.command`), `update_plan` and MCP tools; there is no Notification event; `Interrupt`
//! ends a turn; `SessionEnd` and `Interrupt` may run for 1 s at most. Codex runs a hook only after
//! the user trusts it with `/hooks`, which records a hash in config.toml that we never write.

use std::path::{Path, PathBuf};

use serde_json::Value;
use vultures_ai_agent_config::HookEntry;
use vultures_ai_core::{Activity, AgentEvent, AgentUpdate, RequestId, SessionKey, Step};
use vultures_ai_protocol::{AgentKind, Event};

use crate::{Agent, detail, file_name, hook_command, target};

#[derive(Debug)]
pub struct Codex;

/// Events we register, with the timeout written to hooks.json.
const EVENTS: &[(&str, u64)] = &[
    ("SessionStart", 10),
    ("SessionEnd", 1),
    ("UserPromptSubmit", 10),
    ("PreToolUse", 10),
    ("PostToolUse", 10),
    ("PermissionRequest", 120),
    ("Stop", 10),
    ("SubagentStart", 10),
    ("SubagentStop", 10),
    ("Interrupt", 1),
];

impl Agent for Codex {
    fn kind(&self) -> AgentKind {
        AgentKind::Codex
    }

    fn parse(&self, e: &Event) -> Option<AgentUpdate> {
        let p = &e.payload;
        let text = |k: &str| p.get(k).and_then(Value::as_str).unwrap_or_default();
        let tool = text("tool_name").to_string();
        let input = p.get("tool_input").cloned().unwrap_or(Value::Null);

        let event = match e.event.as_str() {
            "SessionStart" => AgentEvent::SessionStarted,
            "SessionEnd" => AgentEvent::SessionEnded,
            "UserPromptSubmit" => AgentEvent::PromptSubmitted,
            "PreToolUse" => AgentEvent::ToolStarted(Step {
                activity: activity(&tool),
                detail: patch_file(&tool, &input).or_else(|| detail(&input)),
                tool,
            }),
            "PostToolUse" => AgentEvent::ToolFinished {
                failed: false,
                target: Some(request_target(&tool, &input)),
            },
            "PermissionRequest" => AgentEvent::PermissionRequested {
                request: RequestId(e.id.clone()),
                target: request_target(&tool, &input),
                ask: crate::ask(&tool, &input),
                tool,
            },
            // A turn ending by the user's hand is a stop, not a failure.
            "Stop" | "Interrupt" => AgentEvent::Stopped {
                message: crate::filled(text("last_assistant_message")),
            },
            "SubagentStart" => AgentEvent::SubagentStarted,
            "SubagentStop" => AgentEvent::SubagentStopped,
            _ => return None,
        };

        Some(AgentUpdate {
            terminal: e.terminal.clone(),
            agent_id: crate::filled(text("agent_id")),
            session: SessionKey {
                agent: AgentKind::Codex,
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
        home.join(".codex").join("hooks.json")
    }

    fn hook_entries(&self, hook_exe: &Path) -> Vec<HookEntry> {
        EVENTS
            .iter()
            .map(|&(event, timeout_secs)| HookEntry {
                event,
                command: hook_command(hook_exe, "codex", event),
                timeout_secs,
                status_message: (event == "PermissionRequest").then_some(crate::WAITING),
            })
            .collect()
    }
}

/// What Codex will do with our hooks, read from its config.toml (never written).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Trust {
    /// `[features] hooks = false` turns every hook off.
    pub hooks_disabled: bool,
    /// Our entries Codex has not been told to trust yet (`/hooks` in Codex).
    pub untrusted: usize,
    pub total: usize,
}

/// Codex keys trust by `"<hooks.json path>:<event_in_snake_case>:<group>:<hook>"` under
/// `[hooks.state]`. An entry without a key there is not run until the user trusts it.
pub fn trust(hooks_json: &Value, hooks_path: &Path, config_toml: &str, marker: &str) -> Trust {
    let path = hooks_path.display().to_string();
    let mut total = 0;
    let mut untrusted = 0;
    if let Some(events) = hooks_json.get("hooks").and_then(Value::as_object) {
        for (event, groups) in events {
            for (g, group) in groups.as_array().into_iter().flatten().enumerate() {
                for (h, hook) in group
                    .get("hooks")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .enumerate()
                {
                    let ours = hook
                        .get("command")
                        .and_then(Value::as_str)
                        .is_some_and(|c| c.contains(marker));
                    if !ours {
                        continue;
                    }
                    total += 1;
                    let key = format!("[hooks.state.\"{path}:{}:{g}:{h}\"]", snake(event));
                    if !config_toml.contains(&key) {
                        untrusted += 1;
                    }
                }
            }
        }
    }
    Trust {
        hooks_disabled: features_hooks_off(config_toml),
        untrusted,
        total,
    }
}

fn snake(event: &str) -> String {
    let mut out = String::new();
    for (i, c) in event.chars().enumerate() {
        if c.is_ascii_uppercase() && i > 0 {
            out.push('_');
        }
        out.push(c.to_ascii_lowercase());
    }
    out
}

fn features_hooks_off(config: &str) -> bool {
    let mut in_features = false;
    for line in config.lines().map(str::trim) {
        if line.starts_with('[') {
            in_features = line == "[features]";
        } else if in_features && line.replace(' ', "") == "hooks=false" {
            return true;
        }
    }
    false
}

fn activity(tool: &str) -> Activity {
    match tool {
        "view_image" | "read_file" | "list_dir" => Activity::Read,
        "grep" | "find" | "search" => Activity::Search,
        "apply_patch" | "write_file" => Activity::Edit,
        "Bash" | "shell" | "exec_command" | "local_shell" | "unified_exec" => Activity::Run,
        "web_search" | "web_fetch" => Activity::Web,
        "update_plan" => Activity::Plan,
        "spawn_agent" => Activity::Subagent,
        t if t.starts_with("mcp__") => Activity::Web,
        _ => Activity::Work,
    }
}

/// Files a patch touches, from its `*** Add|Update|Delete File: <path>` headers.
fn patched(input: &Value) -> Vec<String> {
    let patch = input.get("command").and_then(Value::as_str).unwrap_or_default();
    patch
        .lines()
        .filter_map(|l| {
            ["*** Add File: ", "*** Update File: ", "*** Delete File: "]
                .iter()
                .find_map(|h| l.strip_prefix(h))
        })
        .map(|path| path.trim().to_string())
        .collect()
}

/// `main.rs`, or `main.rs +2` when the patch touches more files.
fn patch_file(tool: &str, input: &Value) -> Option<String> {
    if tool != "apply_patch" {
        return None;
    }
    let files = patched(input);
    let first = file_name(files.first()?);
    Some(if files.len() > 1 {
        format!("{first} +{}", files.len() - 1)
    } else {
        first
    })
}

/// The full paths, for an approval card: Allow should show exactly what it permits.
fn patch_files(tool: &str, input: &Value) -> Option<String> {
    if tool != "apply_patch" {
        return None;
    }
    let files = patched(input);
    (!files.is_empty()).then(|| files.join(", "))
}

/// What a permission card names; a finished call is matched to its card by the same string.
fn request_target(tool: &str, input: &Value) -> String {
    match patch_files(tool, input) {
        Some(files) => format!("Edit · {files}"),
        None => target(tool, input),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use vultures_ai_protocol::Terminal;

    fn event(name: &str, payload: Value) -> Event {
        Event {
            v: vultures_ai_protocol::VERSION,
            id: "req-1".into(),
            agent: AgentKind::Codex,
            agent_name: None,
            event: name.into(),
            wants_reply: name == "PermissionRequest",
            terminal: Terminal::default(),
            payload,
        }
    }

    /// A real session recorded from codex-cli 0.159 (paths replaced).
    #[test]
    fn a_recorded_session_parses_end_to_end() {
        let lines = include_str!("../tests/fixtures/codex-session.jsonl");
        let events: Vec<AgentEvent> = lines
            .lines()
            .map(|l| serde_json::from_str::<Value>(l).unwrap())
            .map(|p| {
                let name = p["hook_event_name"].as_str().unwrap().to_string();
                Codex.parse(&event(&name, p)).unwrap()
            })
            .map(|u| {
                assert_eq!(u.session.agent, AgentKind::Codex);
                assert!(!u.session.session_id.is_empty());
                assert_eq!(u.cwd.as_deref(), Some("/home/me/notes"));
                u.event
            })
            .collect();
        assert_eq!(events.first(), Some(&AgentEvent::SessionStarted));
        assert_eq!(events.get(1), Some(&AgentEvent::PromptSubmitted));
        assert!(events.contains(&AgentEvent::ToolStarted(Step {
            activity: Activity::Run,
            tool: "Bash".into(),
            detail: Some("wc -l notes.txt".into()),
        })));
        assert!(events.contains(&AgentEvent::ToolStarted(Step {
            activity: Activity::Edit,
            tool: "apply_patch".into(),
            detail: Some("notes.txt".into()),
        })));
        assert!(matches!(events.last(), Some(AgentEvent::Stopped { .. })));
    }

    #[test]
    fn a_patch_approval_names_every_file() {
        let patch = "*** Begin Patch\n*** Update File: src/a.rs\n@@\n-x\n+y\n*** Add File: src/b.rs\n+z\n*** End Patch";
        let e = Codex.parse(&event(
            "PermissionRequest",
            json!({ "session_id": "s", "tool_name": "apply_patch", "tool_input": { "command": patch } }),
        ));
        assert_eq!(
            e.map(|u| u.event),
            Some(AgentEvent::PermissionRequested {
                request: RequestId("req-1".into()),
                tool: "apply_patch".into(),
                target: "Edit · src/a.rs, src/b.rs".into(),
                ask: vultures_ai_core::Ask {
                    added: 2,
                    removed: 1,
                    ..Default::default()
                },
            })
        );
        assert_eq!(
            patch_file("apply_patch", &json!({ "command": patch })).as_deref(),
            Some("a.rs +1")
        );
    }

    #[test]
    fn interrupt_is_a_stop_and_unknown_events_are_ignored() {
        assert_eq!(
            Codex.parse(&event("Interrupt", json!({}))).map(|u| u.event),
            Some(AgentEvent::Stopped { message: None })
        );
        assert!(Codex.parse(&event("PreCompact", json!({}))).is_none());
    }

    #[test]
    fn trust_is_read_from_hooks_state() {
        let hooks = json!({ "hooks": {
            "PreToolUse": [
                { "hooks": [ { "command": "other-tool" } ] },
                { "hooks": [ { "command": "'/x/vultures-ai-hook' --agent codex PreToolUse" } ] }
            ],
            "SessionStart": [ { "hooks": [ { "command": "'/x/vultures-ai-hook' --agent codex SessionStart" } ] } ]
        }});
        let path = Path::new("/home/me/.codex/hooks.json");
        let config = "[features]\nhooks = true\n\n[hooks.state.\"/home/me/.codex/hooks.json:pre_tool_use:1:0\"]\ntrusted_hash = \"x\"\n";
        assert_eq!(
            trust(&hooks, path, config, "vultures-ai-hook"),
            Trust {
                hooks_disabled: false,
                untrusted: 1,
                total: 2
            }
        );
        assert!(trust(&hooks, path, "[features]\nhooks = false\n", "vultures-ai-hook").hooks_disabled);
        assert_eq!(snake("UserPromptSubmit"), "user_prompt_submit");
    }

    #[test]
    fn install_entries() {
        let entries = Codex.hook_entries(Path::new("/opt/vultures-ai-hook"));
        assert_eq!(entries.len(), EVENTS.len());
        let end = entries.iter().find(|e| e.event == "SessionEnd").unwrap();
        assert_eq!(end.timeout_secs, 1, "Codex caps SessionEnd at a few seconds");
        assert_eq!(end.command, "'/opt/vultures-ai-hook' --agent codex SessionEnd");
        assert_eq!(end.status_message, None);
        let permission = entries.iter().find(|e| e.event == "PermissionRequest").unwrap();
        assert_eq!(permission.status_message, Some(crate::WAITING));
    }
}
