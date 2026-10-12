//! Kimi Code (the `kimi` CLI): Claude Code's hook JSON, from `[[hooks]]` tables in its own
//! `config.toml` (`$KIMI_CODE_HOME`, or `~/.kimi-code`).
//!
//! Its sessions are another tool's, named `kimi` (violet, like any other tool), and the island
//! only watches them: Kimi fires its `PermissionRequest` hook without waiting for it (read from
//! its code), so a permission shows as waiting in its terminal and Kimi asks in its own prompt.
//! What differs from Claude Code: its file tools take `path` for `file_path`.

use std::path::{Path, PathBuf};

use serde_json::Value;
use vults_agent_config::toml::toml_edit::{Table, value};
use vults_core::AgentUpdate;
use vults_protocol::Event;

use crate::{Dirs, TomlAgent, hook_command};

/// The name its sessions go by, and the `--agent` its hooks run with.
pub(crate) const NAME: &str = "kimi";

pub static KIMI: TomlAgent = TomlAgent {
    name: NAME,
    file: config_file,
    key: "hooks",
    tables,
};

/// Events we register, with their timeout in seconds; none of them waits for an answer.
const EVENTS: &[(&str, i64)] = &[
    ("SessionStart", 5),
    ("SessionEnd", 5),
    ("UserPromptSubmit", 5),
    ("PreToolUse", 5),
    ("PostToolUse", 5),
    ("PostToolUseFailure", 5),
    ("PermissionRequest", 5),
    ("Stop", 5),
    ("StopFailure", 5),
    ("SubagentStop", 5),
];

fn config_file(dirs: &Dirs) -> PathBuf {
    dirs.agent("KIMI_CODE_HOME", ".kimi-code").join("config.toml")
}

fn tables(hook_exe: &Path) -> Vec<Table> {
    EVENTS
        .iter()
        .map(|&(event, timeout)| {
            let mut t = Table::new();
            t.insert("event", value(event));
            t.insert("command", value(hook_command(hook_exe, NAME, event)));
            t.insert("timeout", value(timeout));
            t
        })
        .collect()
}

pub(crate) fn parse(e: &Event) -> Option<AgentUpdate> {
    let mut e = e.clone();
    if let Some(input) = e.payload.get_mut("tool_input").and_then(Value::as_object_mut)
        && !input.contains_key("file_path")
        && let Some(path) = input.get("path").cloned()
    {
        input.insert("file_path".into(), path);
    }
    crate::other::as_claude(&e, NAME)
}

#[cfg(test)]
mod tests {
    use super::*;
    use vults_agent_config::toml as config;
    use vults_core::{Activity, AgentEvent};
    use vults_protocol::AgentKind;

    /// Hook calls recorded from a real Kimi Code 2.1.1 `-p` run (run a command, read, edit, write
    /// a file, run a failing one), through the spike kit, with the paths trimmed.
    const SESSION: &str = include_str!("../tests/fixtures/kimi-session.jsonl");

    fn event(payload: Value) -> Event {
        Event {
            v: vults_protocol::VERSION,
            id: "r1".into(),
            agent: AgentKind::Other,
            agent_name: Some(NAME.into()),
            event: payload["hook_event_name"].as_str().unwrap_or_default().into(),
            wants_reply: false,
            zeca: false,
            terminal: Default::default(),
            payload,
        }
    }

    fn events() -> Vec<Event> {
        SESSION
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|l| event(serde_json::from_str(l).unwrap()))
            .collect()
    }

    #[test]
    fn a_real_session_reads_as_claude_codes() {
        let updates: Vec<AgentUpdate> = events().iter().filter_map(crate::parse).collect();
        assert_eq!(updates.len(), events().len(), "every event is read");
        assert!(updates[0].session.session_id.starts_with("kimi/session_"));
        assert_eq!(updates[0].cwd.as_deref(), Some("/home/me/notes"));
        let started: Vec<(Activity, String)> = updates
            .iter()
            .filter_map(|u| match &u.event {
                AgentEvent::ToolStarted(s) => Some((s.activity, s.detail.clone().unwrap_or_default())),
                _ => None,
            })
            .collect();
        assert_eq!(
            started,
            [
                (Activity::Run, "echo hi > out.txt".to_string()),
                (Activity::Read, "notes.txt".to_string()),
                (Activity::Edit, "notes.txt".to_string()),
                (Activity::Edit, "new.txt".to_string()),
                (Activity::Run, "false".to_string()),
            ]
        );
        // The edit's lines, from its old and new text under `path`.
        let diff = updates
            .iter()
            .find_map(|u| match &u.event {
                AgentEvent::ToolFinished { diff: Some(d), .. } => Some(d.clone()),
                _ => None,
            })
            .expect("the edit's diff");
        assert_eq!((diff.files[0].added, diff.files[0].removed), (1, 1));
        // The failing command is a failure.
        assert!(
            updates
                .iter()
                .any(|u| matches!(u.event, AgentEvent::ToolFinished { failed: true, .. }))
        );
    }

    /// Its PermissionRequest, in the shape its code builds (`toolName`, `toolInput`, sent snake
    /// case like its other hooks): waiting in its terminal, never a card.
    #[test]
    fn a_permission_waits_in_its_terminal() {
        let p = serde_json::json!({ "hook_event_name": "PermissionRequest", "session_id": "session_1",
            "cwd": "/home/me/notes", "tool_name": "Bash", "tool_input": { "command": "rm -rf build" },
            "action": "run command" });
        let u = crate::parse(&event(p)).unwrap();
        assert!(
            matches!(u.event, AgentEvent::Question { ref message } if message.contains("rm -rf build")),
            "{:?}",
            u.event
        );
    }

    #[test]
    fn its_hooks_go_in_its_config_with_the_users_comments() {
        let home = Path::new("/home/me");
        assert_eq!(
            (KIMI.file)(&Dirs::home_only(home)),
            home.join(".kimi-code/config.toml")
        );
        let moved = Dirs::new(home, |v| (v == "KIMI_CODE_HOME").then(|| "/w/k".into()));
        assert_eq!((KIMI.file)(&moved), Path::new("/w/k/config.toml"));

        let mut doc: config::toml_edit::DocumentMut =
            "# mine\ndefault_model = \"k2\"\n\n[[hooks]]\nevent = \"Stop\"\ncommand = \"notify-send done\"\n"
                .parse()
                .unwrap();
        let exe = Path::new("/x/vults-hook");
        config::with_ours(&mut doc, KIMI.key, &(KIMI.tables)(exe), crate::MARKER);
        assert!(doc.to_string().starts_with("# mine\ndefault_model = \"k2\"\n"));
        assert!(config::ours_match(
            &doc,
            KIMI.key,
            &(KIMI.tables)(exe),
            crate::MARKER
        ));
        assert!(
            doc.to_string()
                .contains("command = \"'/x/vults-hook' --agent kimi PermissionRequest\"")
        );
        config::remove_ours(&mut doc, KIMI.key, crate::MARKER);
        assert!(doc.to_string().contains("notify-send done"));
        assert!(!doc.to_string().contains("vults-hook"));
    }
}
