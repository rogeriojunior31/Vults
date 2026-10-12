//! Factory Droid (the `droid` CLI): Claude Code's hooks, in `~/.factory/settings.json`.
//!
//! Its sessions are another tool's, named `droid` (violet, like any other tool), and the island
//! only watches them. Droid has no `PermissionRequest`: its `PreToolUse` runs before every tool,
//! whether Droid would have asked or not, so the island never answers it and Droid asks in its
//! own prompt. What differs from Claude Code: its tool names (`Execute`, `Create`, `LS`) and
//! `Edit`'s `old_str` / `new_str`; a command that fails sends no `PostToolUse`.

use std::path::{Path, PathBuf};

use serde_json::Value;
use vults_agent_config::HookEntry;
use vults_core::AgentUpdate;
use vults_protocol::{AgentKind, Event};

use crate::{Agent, Dirs, hook_command};

/// The name its sessions go by, and the `--agent` its hooks run with.
pub(crate) const NAME: &str = "droid";

#[derive(Debug)]
pub struct Droid;

/// Events we register, with their timeout in seconds; none of them waits for an answer.
const EVENTS: &[(&str, u64)] = &[
    ("SessionStart", 5),
    ("SessionEnd", 5),
    ("UserPromptSubmit", 5),
    ("PreToolUse", 5),
    ("PostToolUse", 5),
    ("Notification", 5),
    ("Stop", 5),
    ("SubagentStop", 5),
];

/// Droid's tool names under Claude Code's, where the island reads them differently.
const TOOLS: &[(&str, &str)] = &[("Execute", "Bash"), ("Create", "Write"), ("LS", "Glob")];

/// Its tool arguments under Claude Code's names.
const ARGS: &[(&str, &str)] = &[("old_str", "old_string"), ("new_str", "new_string")];

impl Agent for Droid {
    fn kind(&self) -> AgentKind {
        AgentKind::Other
    }

    fn parse(&self, e: &Event) -> Option<AgentUpdate> {
        let mut e = e.clone();
        let p = &mut e.payload;
        if let Some(tool) = p.get("tool_name").and_then(Value::as_str)
            && let Some((_, claude)) = TOOLS.iter().find(|(d, _)| *d == tool)
        {
            p["tool_name"] = (*claude).into();
        }
        if let Some(input) = p.get_mut("tool_input").and_then(Value::as_object_mut) {
            for (from, to) in ARGS {
                if let Some(v) = input.get(*from).cloned().filter(|_| !input.contains_key(*to)) {
                    input.insert((*to).into(), v);
                }
            }
        }
        crate::other::as_claude(&e, NAME)
    }

    fn config_file(&self, dirs: &Dirs) -> PathBuf {
        // FACTORY_HOME_OVERRIDE stands for the home: Droid keeps its `.factory` inside it.
        let home = dirs.var("FACTORY_HOME_OVERRIDE").unwrap_or(dirs.home());
        home.join(".factory").join("settings.json")
    }

    fn hook_entries(&self, hook_exe: &Path) -> Vec<HookEntry> {
        EVENTS
            .iter()
            .map(|&(event, timeout)| HookEntry {
                event,
                command: hook_command(hook_exe, NAME, event),
                timeout,
                status_message: None,
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vults_core::{Activity, AgentEvent};

    /// Hook calls recorded from a real droid 0.239.0 `exec` (run a command, read, edit, write a
    /// file, run a failing one), through the spike kit, as `[event, payload]`.
    const SESSION: &str = include_str!("../tests/fixtures/droid-session.jsonl");

    fn events() -> Vec<Event> {
        SESSION
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|l| {
                let payload: Value = serde_json::from_str(l).unwrap();
                Event {
                    v: vults_protocol::VERSION,
                    id: "r1".into(),
                    agent: AgentKind::Other,
                    agent_name: Some(NAME.into()),
                    event: payload["hook_event_name"].as_str().unwrap().into(),
                    wants_reply: false,
                    zeca: false,
                    terminal: Default::default(),
                    payload,
                }
            })
            .collect()
    }

    #[test]
    fn a_real_session_reads_as_claude_codes() {
        let updates: Vec<AgentUpdate> = events().iter().filter_map(crate::parse).collect();
        assert_eq!(updates.len(), events().len(), "every event is read");
        let first = &updates[0];
        assert_eq!(first.session.agent, AgentKind::Other);
        assert!(first.session.session_id.starts_with("droid/"));
        assert_eq!(first.cwd.as_deref(), Some("/home/me/notes"));
        assert_eq!(first.event, AgentEvent::SessionStarted);

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
        // The edit's lines, from its old and new text.
        let edit = updates
            .iter()
            .find_map(|u| match &u.event {
                AgentEvent::ToolFinished { diff: Some(d), .. } => Some(d.clone()),
                _ => None,
            })
            .expect("the edit's diff");
        assert_eq!((edit.files[0].added, edit.files[0].removed), (1, 1));
        assert!(matches!(updates.last().unwrap().event, AgentEvent::SessionEnded));
    }

    #[test]
    fn its_hooks_go_in_its_settings() {
        // Absolute on every OS: a relative override is ignored, and "/w/f" is relative on Windows.
        let home = &crate::dirs::abs("/home/me");
        let usual = Droid.config_file(&Dirs::home_only(home));
        assert_eq!(usual, home.join(".factory/settings.json"));
        let moved = Dirs::new(home, |v| {
            (v == "FACTORY_HOME_OVERRIDE").then(|| crate::dirs::abs("/w/f").into())
        });
        assert_eq!(
            Droid.config_file(&moved),
            crate::dirs::abs("/w/f").join(".factory/settings.json")
        );

        let exe = Path::new("/x/vults-hook");
        let file = Droid.install(&serde_json::json!({ "customModels": [] }), exe);
        assert!(Droid.installed(&file) && Droid.up_to_date(&file, exe));
        assert_eq!(file["customModels"], serde_json::json!([]));
        assert!(file["hooks"].get("PermissionRequest").is_none());
        assert_eq!(Droid.uninstall(&file)["customModels"], serde_json::json!([]));
    }
}
