//! GitHub Copilot CLI: https://docs.github.com/copilot/how-tos/copilot-cli
//!
//! It reads every hooks file in `~/.copilot/hooks/` (`$COPILOT_HOME/hooks/`), so ours is a file of
//! our own, `vults.json`: `{"version": 1, "hooks": {"<event>": [{"type": "command", "bash": …,
//! "timeoutSec": …}]}}` with camelCase events. What differs from Claude Code, read here as its:
//! - the payload names neither its event nor in snake_case: each entry passes its event on the
//!   command line, and `sessionId`, `toolName`, `toolArgs` / `toolInput` are copied over;
//! - its tools and their arguments: `bash`, `view {path}`, `create {path, file_text}`,
//!   `edit {path, old_str, new_str}`;
//! - its `permissionRequest` takes a bare `{"behavior": …}`; with no answer its terminal asks,
//!   as Claude Code's does (checked with Copilot CLI 1.0.96, which has no question tool);
//! - `postToolUse` carries the tool's whole output in `toolResult`, which the hook drops, keeping
//!   only the error of a tool that failed.

use std::path::{Path, PathBuf};

use serde_json::{Map, Value, json};
use vults_agent_config::HookEntry;
use vults_core::AgentUpdate;
use vults_protocol::{AgentKind, Event};

use crate::{Agent, Claude, Dirs, MARKER, hook_command};

/// The name its hooks run with.
pub(crate) const NAME: &str = "copilot";

#[derive(Debug)]
pub struct Copilot;

/// Its events, the name our hook gets for each (Claude Code's), and the timeout in seconds. A
/// permission waits for a human: the hook's decision budget plus a margin.
const EVENTS: &[(&str, &str, u64)] = &[
    ("sessionStart", "SessionStart", 10),
    ("sessionEnd", "SessionEnd", 10),
    ("userPromptSubmitted", "UserPromptSubmit", 10),
    ("preToolUse", "PreToolUse", 10),
    ("postToolUse", "PostToolUse", 10),
    ("permissionRequest", "PermissionRequest", 120),
    ("notification", "Notification", 10),
    ("agentStop", "Stop", 10),
    ("subagentStop", "SubagentStop", 10),
    ("errorOccurred", "StopFailure", 10),
];

impl Agent for Copilot {
    fn kind(&self) -> AgentKind {
        AgentKind::Copilot
    }

    fn parse(&self, e: &Event) -> Option<AgentUpdate> {
        // The app's own events, for a session it runs over ACP (W10), are Claude Code's already.
        if e.payload.get("session_id").is_some() {
            let mut update = Claude.parse(e)?;
            update.session.agent = AgentKind::Copilot;
            return Some(update);
        }
        let mut e = e.clone();
        let p = e.payload.as_object()?.clone();
        let text = |k: &str| p.get(k).and_then(Value::as_str).map(str::to_owned);
        let mut out = Map::new();
        out.insert("hook_event_name".into(), json!(e.event));
        if let Some(id) = text("sessionId") {
            out.insert("session_id".into(), json!(id));
        }
        for key in ["cwd", "prompt", "message", "notification_type"] {
            if let Some(v) = p.get(key) {
                out.insert(key.into(), v.clone());
            }
        }
        // A permission for a file sends the change as a unified diff, not its text.
        let diff = e
            .payload
            .pointer("/toolInput/diff")
            .and_then(Value::as_str)
            .map(diff_lines);
        if let Some(tool) = text("toolName") {
            let creates = diff.is_some_and(|(_, _, creates)| creates);
            let tool = if creates {
                "Write"
            } else {
                claude_tool(&tool).unwrap_or(&tool)
            };
            out.insert("tool_name".into(), json!(tool));
            out.insert(
                "tool_input".into(),
                input(p.get("toolArgs").or_else(|| p.get("toolInput"))),
            );
        }
        // The hook keeps a failed tool's error under Claude Code's name.
        if let Some(response) = p.get("tool_response") {
            out.insert("tool_response".into(), response.clone());
        }
        if e.event == "PostToolUse" && p.get("tool_response").and_then(|r| r.get("error")).is_some() {
            e.event = "PostToolUseFailure".into();
        }
        if e.event == "StopFailure" {
            let error = p.get("error").map(|err| match err {
                Value::Object(o) => o.get("message").cloned().unwrap_or(Value::Null),
                other => other.clone(),
            });
            out.insert("error".into(), error.unwrap_or(Value::Null));
        }
        e.payload = Value::Object(out);
        let mut update = Claude.parse(&e)?;
        update.session.agent = AgentKind::Copilot;
        if let (vults_core::AgentEvent::PermissionRequested { ask, .. }, Some((added, removed, _))) =
            (&mut update.event, diff)
        {
            (ask.added, ask.removed) = (added, removed);
        }
        Some(update)
    }

    fn config_file(&self, dirs: &Dirs) -> PathBuf {
        dirs.agent("COPILOT_HOME", ".copilot")
            .join("hooks")
            .join(format!("{}.json", vults_brand::SLUG))
    }

    fn hook_entries(&self, hook_exe: &Path) -> Vec<HookEntry> {
        EVENTS
            .iter()
            .map(|&(event, ours, timeout)| HookEntry {
                event,
                command: hook_command(hook_exe, NAME, ours),
                timeout,
                status_message: None,
            })
            .collect()
    }

    // The file is ours whole: installing writes all of it, removing leaves no hook in it.
    fn install(&self, _: &Value, hook_exe: &Path) -> Value {
        self.ours(hook_exe)
    }

    fn install_blocked(&self, config: &Value) -> Option<String> {
        let theirs = match config {
            Value::Null => false,
            Value::Object(m) => !m.is_empty() && !self.installed(config),
            _ => true,
        };
        theirs.then(|| "This hooks file is not one Vults wrote. Rename or remove it, then install.".into())
    }

    fn uninstall(&self, _: &Value) -> Value {
        json!({ "version": 1, "hooks": {} })
    }

    fn installed(&self, config: &Value) -> bool {
        self.our_command(config).is_some()
    }

    fn up_to_date(&self, config: &Value, hook_exe: &Path) -> bool {
        *config == self.ours(hook_exe)
    }

    fn our_command<'a>(&self, config: &'a Value) -> Option<&'a str> {
        config
            .get("hooks")?
            .as_object()?
            .values()
            .filter_map(Value::as_array)
            .flatten()
            .filter_map(|h| h.get("bash").and_then(Value::as_str))
            .find(|c| c.contains(MARKER))
    }
}

impl Copilot {
    fn ours(&self, hook_exe: &Path) -> Value {
        let hooks: Map<String, Value> = self
            .hook_entries(hook_exe)
            .into_iter()
            .map(|h| {
                let handler = json!({ "type": "command", "bash": h.command, "timeoutSec": h.timeout });
                (h.event.to_string(), json!([handler]))
            })
            .collect();
        json!({ "version": 1, "hooks": hooks })
    }
}

/// The lines a unified diff adds and removes, and whether it creates its file.
fn diff_lines(diff: &str) -> (u32, u32, bool) {
    let (mut added, mut removed, mut creates) = (0, 0, false);
    for line in diff.lines() {
        if line == "--- a/dev/null" || line == "--- /dev/null" || line.starts_with("create file mode") {
            creates = true;
        } else if line.starts_with('+') && !line.starts_with("+++") {
            added += 1;
        } else if line.starts_with('-') && !line.starts_with("---") {
            removed += 1;
        }
    }
    (added, removed, creates)
}

/// Copilot CLI's built-in tools (1.0), under Claude Code's names.
fn claude_tool(name: &str) -> Option<&'static str> {
    Some(match name {
        "bash" | "read_bash" | "write_bash" | "stop_bash" => "Bash",
        "view" => "Read",
        "create" => "Write",
        "edit" => "Edit",
        "grep" => "Grep",
        "glob" => "Glob",
        "web_fetch" | "fetch" => "WebFetch",
        "task" => "Task",
        _ => return None,
    })
}

/// The tool's arguments (an object, or JSON in a string) with Copilot's names copied to Claude
/// Code's: what a card shows and a diff is built from.
fn input(args: Option<&Value>) -> Value {
    let parsed;
    let args = match args {
        Some(Value::String(s)) => {
            parsed = serde_json::from_str::<Value>(s).unwrap_or(Value::Null);
            &parsed
        }
        Some(v) => v,
        None => return Value::Null,
    };
    let Value::Object(args) = args else {
        return Value::Null;
    };
    let mut out = args.clone();
    for (from, to) in [
        ("path", "file_path"),
        ("old_str", "old_string"),
        ("new_str", "new_string"),
        ("file_text", "content"),
    ] {
        if let Some(v) = args.get(from).filter(|_| !out.contains_key(to)) {
            out.insert(to.into(), v.clone());
        }
    }
    Value::Object(out)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use vults_core::{Activity, AgentEvent, Step};
    use vults_protocol::Terminal;

    /// Hook calls recorded from Copilot CLI 1.0.96 with `scripts/spike.sh copilot` against a fake
    /// model, its permissions allowed by the hook: a command, a file read, edited and written, a
    /// failing command.
    const SESSION: &str = include_str!("../tests/fixtures/copilot-session.jsonl");

    fn session() -> Vec<AgentUpdate> {
        SESSION
            .lines()
            .filter_map(|l| {
                let [event, payload]: [Value; 2] = serde_json::from_str(l).unwrap();
                let event = event.as_str().unwrap().to_string();
                Copilot.parse(&Event {
                    v: vults_protocol::VERSION,
                    id: "r1".into(),
                    agent: AgentKind::Copilot,
                    agent_name: None,
                    wants_reply: event == "PermissionRequest",
                    zeca: false,
                    event,
                    terminal: Terminal::default(),
                    payload,
                })
            })
            .collect()
    }

    #[test]
    fn a_real_session_reads_as_claude_codes() {
        let updates = session();
        assert!(updates.iter().all(|u| u.session.agent == AgentKind::Copilot
            && !u.session.session_id.is_empty()
            && u.cwd.as_deref() == Some("/home/me/notes")));
        let events: Vec<&AgentEvent> = updates.iter().map(|u| &u.event).collect();
        assert!(events.contains(&&AgentEvent::SessionStarted));
        assert!(events.contains(&&AgentEvent::PromptSubmitted));
        assert_eq!(events.last(), Some(&&AgentEvent::SessionEnded));
        assert!(events.contains(&&AgentEvent::ToolStarted(Step {
            activity: Activity::Run,
            detail: Some("echo hi > out.txt".into()),
            tool: "Bash".into(),
        })));
        assert!(events.contains(&&AgentEvent::ToolStarted(Step {
            activity: Activity::Edit,
            detail: Some("notes.txt".into()),
            tool: "Edit".into(),
        })));
    }

    #[test]
    fn its_permissions_get_a_card_with_what_they_change() {
        let cards: Vec<(String, u32, u32)> = session()
            .into_iter()
            .filter_map(|u| match u.event {
                AgentEvent::PermissionRequested { target, ask, .. } => Some((target, ask.added, ask.removed)),
                _ => None,
            })
            .collect();
        assert!(
            cards.contains(&("Bash · echo hi > out.txt".into(), 0, 0)),
            "{cards:?}"
        );
        assert!(
            cards.contains(&("Edit · /home/me/notes/notes.txt".into(), 1, 1)),
            "{cards:?}"
        );
        assert!(
            cards.contains(&("Write · /home/me/notes/new.txt".into(), 2, 0)),
            "{cards:?}"
        );
    }

    #[test]
    fn an_edit_shows_its_diff_and_a_failure_fails() {
        let updates = session();
        assert!(updates.iter().any(|u| matches!(&u.event,
            AgentEvent::ToolFinished { diff: Some(d), target: Some(t), .. }
                if t.starts_with("Edit") && (d.added(), d.removed()) == (1, 1))));
        assert!(updates.iter().any(|u| matches!(&u.event,
            AgentEvent::ToolFinished { failed: true, target: Some(t), .. } if t == "Bash · false")));
    }

    #[test]
    fn a_run_vults_started_asks_in_claude_codes_words() {
        let e = Event {
            v: vults_protocol::VERSION,
            id: "r9".into(),
            agent: AgentKind::Copilot,
            agent_name: None,
            wants_reply: true,
            zeca: false,
            event: "PermissionRequest".into(),
            terminal: Terminal::default(),
            payload: json!({ "hook_event_name": "PermissionRequest", "session_id": "s", "cwd": "/w",
                             "tool_name": "Bash", "tool_input": { "command": "make" } }),
        };
        let u = Copilot.parse(&e).unwrap();
        assert!(
            matches!(u.event, AgentEvent::PermissionRequested { ref target, .. } if target == "Bash · make")
        );
        assert_eq!(
            (u.session.agent, u.session.session_id.as_str()),
            (AgentKind::Copilot, "s")
        );
    }

    #[test]
    fn the_file_is_ours_whole() {
        let exe = Path::new("/opt/vults-hook");
        assert_eq!(
            Copilot.config_file(&Dirs::home_only(Path::new("/home/me"))),
            Path::new("/home/me/.copilot/hooks/vults.json")
        );
        let ours = Copilot.install(&Value::Null, exe);
        assert_eq!(
            ours["hooks"]["permissionRequest"][0],
            json!({ "type": "command", "bash": "'/opt/vults-hook' --agent copilot PermissionRequest", "timeoutSec": 120 })
        );
        assert!(Copilot.installed(&ours) && Copilot.up_to_date(&ours, exe));
        assert!(Copilot.install_blocked(&Value::Null).is_none());
        assert!(Copilot.install_blocked(&ours).is_none());
        // Someone else's file under our name is never written over.
        let theirs =
            json!({ "version": 1, "hooks": { "preToolUse": [{ "type": "command", "bash": "lint" }] } });
        assert!(Copilot.install_blocked(&theirs).is_some());
        assert!(!Copilot.installed(&Copilot.uninstall(&ours)));
    }
}
