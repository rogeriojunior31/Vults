//! Cursor (the editor's agent and the `cursor-agent` CLI share one hooks file):
//! `~/.cursor/hooks.json`, always in the home folder.
//!
//! Its sessions are another tool's, named `cursor` (violet, like any other tool), and the island
//! only watches them: Cursor asks its permissions in its own prompt. What differs from Claude
//! Code's hook JSON is met in [`crate::other::normalized`] (camelCase events,
//! `conversation_id`, `workspace_roots`, `error_message`); here, its tool names.
//! - The file holds a flat list of handlers per event, shared with other tools.
//! - `preToolUse`, `beforeSubmitPrompt` and the `before…` hooks are left out: Cursor reads their
//!   output as a decision, so the island sees finished steps, never steps starting. An edit comes
//!   whole from `afterFileEdit` (its file and each replacement), read by nothing.
//! - A hook runs in `~/.cursor`, not the project: the folder is `workspace_roots`.
//! - Every command ends in `|| exit 0`, so a missing hook binary is never read as a failure.

use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use vults_agent_config::{HookEntry, flat};
use vults_core::{Activity, AgentEvent, AgentUpdate, SessionKey, Step};
use vults_protocol::{AgentKind, Event};

use crate::{Agent, Dirs, MARKER, hook_command};

/// The name its sessions go by, and the `--agent` its hooks run with.
pub(crate) const NAME: &str = "cursor";

/// The hooks file's format version Cursor reads.
const VERSION: u64 = 1;

#[derive(Debug)]
pub struct Cursor;

/// Events we register, with their timeout in seconds: none of them waits for an answer.
const EVENTS: &[(&str, u64)] = &[
    ("sessionStart", 5),
    ("sessionEnd", 5),
    ("postToolUse", 5),
    ("postToolUseFailure", 5),
    ("afterFileEdit", 5),
    ("subagentStop", 5),
    ("stop", 5),
    ("preCompact", 5),
];

/// Cursor's tool names under Claude Code's, where the island reads them differently.
const TOOLS: &[(&str, &str)] = &[("Shell", "Bash")];

impl Agent for Cursor {
    fn kind(&self) -> AgentKind {
        AgentKind::Other
    }

    fn parse(&self, e: &Event) -> Option<AgentUpdate> {
        let mut e = e.clone();
        if let Some(tool) = e.payload.get("tool_name").and_then(Value::as_str) {
            let claude = TOOLS
                .iter()
                .find(|(c, _)| *c == tool)
                .map(|(_, to)| *to)
                // `MCP:<tool>`: Claude Code's `mcp__` names read the same way.
                .map(str::to_string)
                .or_else(|| tool.strip_prefix("MCP:").map(|t| format!("mcp__{t}")));
            if let Some(claude) = claude {
                e.payload["tool_name"] = claude.into();
            }
        }
        let mut update = match e.event.as_str() {
            "afterFileEdit" => edited(&e)?,
            _ => crate::other::as_claude(&e, NAME)?,
        };
        // `afterFileEdit` brings each edit's diff: a finished tool's would count it twice.
        if let AgentEvent::ToolFinished { diff, .. } = &mut update.event {
            *diff = None;
        }
        // Its hooks run in ~/.cursor: the project is the workspace's first folder.
        if let Some(root) = e
            .payload
            .pointer("/workspace_roots/0")
            .and_then(Value::as_str)
            .filter(|r| !r.is_empty())
        {
            update.cwd = Some(root.to_string());
        }
        Some(update)
    }

    fn config_file(&self, dirs: &Dirs) -> PathBuf {
        // Hard-coded to the home folder: CURSOR_CONFIG_DIR does not move it.
        dirs.home().join(".cursor").join("hooks.json")
    }

    fn hook_entries(&self, hook_exe: &Path) -> Vec<HookEntry> {
        EVENTS
            .iter()
            .map(|&(event, timeout)| HookEntry {
                event,
                command: format!("{} || exit 0", hook_command(hook_exe, NAME, event)),
                timeout,
                status_message: None,
            })
            .collect()
    }

    fn install(&self, config: &Value, hook_exe: &Path) -> Value {
        flat::with_ours(config, &self.ours(hook_exe), MARKER, VERSION)
    }

    fn install_blocked(&self, config: &Value) -> Option<String> {
        let hooks = config.get("hooks")?;
        (!hooks.is_object())
            .then(|| "Its \"hooks\" is not a list of events. Fix the file, then install.".to_string())
    }

    fn uninstall(&self, config: &Value) -> Value {
        flat::remove_ours(config, MARKER)
    }

    fn installed(&self, config: &Value) -> bool {
        flat::has_ours(config, MARKER)
    }

    fn up_to_date(&self, config: &Value, hook_exe: &Path) -> bool {
        flat::ours_match(config, &self.ours(hook_exe), MARKER)
    }

    fn our_command<'a>(&self, config: &'a Value) -> Option<&'a str> {
        flat::our_command(config, MARKER)
    }
}

impl Cursor {
    fn ours(&self, hook_exe: &Path) -> Vec<(&'static str, Value)> {
        self.hook_entries(hook_exe)
            .into_iter()
            .map(|e| (e.event, json!({ "command": e.command, "timeout": e.timeout })))
            .collect()
    }
}

/// `afterFileEdit`: a finished edit, its file and each replacement's old and new text.
fn edited(e: &Event) -> Option<AgentUpdate> {
    let n = crate::other::normalized(e)?;
    let path = n.payload.get("file_path")?.as_str()?;
    let edits = n.payload.get("edits")?;
    let diff = crate::diff::from_input("MultiEdit", &json!({ "file_path": path, "edits": edits }))?;
    Some(AgentUpdate {
        session: SessionKey {
            agent: AgentKind::Other,
            session_id: format!("{NAME}/{}", n.payload.get("session_id")?.as_str()?),
        },
        cwd: n.payload.get("cwd").and_then(Value::as_str).map(str::to_string),
        terminal: e.terminal.clone(),
        agent_id: None,
        call_id: None,
        received: None,
        event: AgentEvent::Edited {
            step: Step {
                activity: Activity::Edit,
                tool: "Edit".into(),
                detail: Some(crate::file_name(path)),
            },
            diff,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use vults_protocol::Terminal;

    fn event(name: &str, payload: Value) -> Event {
        Event {
            v: vults_protocol::VERSION,
            id: "r1".into(),
            agent: AgentKind::Other,
            agent_name: Some(NAME.into()),
            event: name.into(),
            wants_reply: false,
            zeca: false,
            // Cursor runs its hooks in ~/.cursor.
            terminal: Terminal {
                cwd: Some("/home/me/.cursor".into()),
                ..Terminal::default()
            },
            payload,
        }
    }

    /// The fields its docs and captured sessions show: no `cwd`, the project in
    /// `workspace_roots`, `conversation_id` and `session_id` alike.
    fn base(extra: Value) -> Value {
        let mut p = json!({
            "conversation_id": "98187bc5", "session_id": "98187bc5", "generation_id": "g1",
            "hook_event_name": "postToolUse", "cursor_version": "2026.09.15-d2fe57e",
            "workspace_roots": ["/home/me/notes"], "model": "auto", "transcript_path": null,
        });
        p.as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        p
    }

    #[test]
    fn a_finished_edit_carries_its_step_and_diff_once() {
        let edit = base(json!({
            "hook_event_name": "afterFileEdit", "file_path": "/home/me/notes/a.md",
            "edits": [{ "old_string": "old", "new_string": "new\nmore" }],
        }));
        let u = crate::parse(&event("afterFileEdit", edit)).unwrap();
        assert_eq!(u.session.session_id, "cursor/98187bc5");
        assert_eq!(u.cwd.as_deref(), Some("/home/me/notes"));
        match u.event {
            AgentEvent::Edited { step, diff } => {
                assert_eq!(step.detail.as_deref(), Some("a.md"));
                assert_eq!((diff.added(), diff.removed()), (2, 1));
            }
            other => panic!("{other:?}"),
        }
        let write = base(
            json!({ "tool_name": "Write", "tool_input": { "file_path": "/home/me/notes/a.md", "content": "x" } }),
        );
        let u = crate::parse(&event("postToolUse", write)).unwrap();
        assert!(
            matches!(u.event, AgentEvent::ToolFinished { diff: None, .. }),
            "{:?}",
            u.event
        );
    }

    #[test]
    fn a_session_shows_in_its_project_under_its_name() {
        let start = crate::parse(&event(
            "sessionStart",
            base(json!({ "hook_event_name": "sessionStart" })),
        ))
        .unwrap();
        assert_eq!(start.session.agent, AgentKind::Other);
        assert_eq!(start.session.session_id, "cursor/98187bc5");
        assert_eq!(start.cwd.as_deref(), Some("/home/me/notes"));
        assert!(
            matches!(start.event, AgentEvent::SessionStarted),
            "{:?}",
            start.event
        );

        let shell = base(json!({ "tool_name": "Shell", "tool_input": { "command": "cargo test" } }));
        let u = crate::parse(&event("postToolUse", shell)).unwrap();
        assert!(
            matches!(u.event, AgentEvent::ToolFinished { failed: false, .. }),
            "{:?}",
            u.event
        );

        let failed = base(json!({
            "hook_event_name": "postToolUseFailure", "tool_name": "Shell",
            "tool_input": { "command": "false" }, "error_message": "exit code 1", "failure_type": "error",
        }));
        let u = crate::parse(&event("postToolUseFailure", failed)).unwrap();
        assert!(
            matches!(u.event, AgentEvent::ToolFinished { failed: true, .. }),
            "{:?}",
            u.event
        );

        let stop = base(json!({ "hook_event_name": "stop", "status": "completed", "loop_count": 0 }));
        assert!(matches!(
            crate::parse(&event("stop", stop)).unwrap().event,
            AgentEvent::Stopped { .. }
        ));
        let end = base(json!({ "hook_event_name": "sessionEnd", "reason": "completed" }));
        assert!(matches!(
            crate::parse(&event("sessionEnd", end)).unwrap().event,
            AgentEvent::SessionEnded
        ));
    }

    #[test]
    fn its_tools_read_as_claude_codes() {
        let p = base(
            json!({ "hook_event_name": "preToolUse", "tool_name": "Shell", "tool_input": { "command": "make" } }),
        );
        let u = Cursor.parse(&event("preToolUse", p)).unwrap();
        assert!(
            matches!(u.event, AgentEvent::ToolStarted(ref s) if s.activity == Activity::Run && s.detail.as_deref() == Some("make")),
            "{:?}",
            u.event
        );
    }

    #[test]
    fn the_file_keeps_other_tools_hooks() {
        let exe = Path::new("/x/vults-hook");
        let theirs = json!({ "command": "audit.sh" });
        let file = json!({ "version": 1, "hooks": { "stop": [theirs] } });
        let next = Cursor.install(&file, exe);
        assert!(Cursor.installed(&next) && Cursor.up_to_date(&next, exe));
        assert_eq!(next["hooks"]["stop"][0], theirs);
        let cmd = next["hooks"]["stop"][1]["command"].as_str().unwrap();
        assert_eq!(cmd, "'/x/vults-hook' --agent cursor stop || exit 0");
        // No hook that Cursor reads as a decision.
        for decides in [
            "preToolUse",
            "beforeSubmitPrompt",
            "beforeShellExecution",
            "beforeReadFile",
        ] {
            assert!(next["hooks"].get(decides).is_none(), "{decides}");
        }
        assert!(!Cursor.up_to_date(&next, Path::new("/y/vults-hook")));
        assert_eq!(Cursor.uninstall(&next), file);
        assert!(Cursor.install_blocked(&json!({ "hooks": [] })).is_some());
        assert!(Cursor.install_blocked(&json!({})).is_none());
    }
}
