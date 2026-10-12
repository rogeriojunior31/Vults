//! Gemini CLI, no longer supported: Antigravity took its place. What is left is the way out: the
//! installer still finds the hooks an earlier Vults wrote in `.gemini/settings.json` and removes
//! them (with the diff, the backup and the click), and never installs them again. Meanwhile those
//! hooks send nothing: `gemini` stays a taken name, so the relay treats them as a nameless tool.

use std::path::{Path, PathBuf};

use vults_agent_config::HookEntry;
use vults_core::AgentUpdate;
use vults_protocol::{AgentKind, Event};

use crate::{Agent, Dirs, hook_command};

/// The name its hooks ran with, and the installer's id for the way out.
pub(crate) const NAME: &str = "gemini";

#[derive(Debug)]
pub struct Gemini;

/// The events an earlier Vults registered, timeouts in milliseconds: kept so its entries still
/// read as up to date and only removal is offered.
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
        AgentKind::Other
    }

    fn parse(&self, _: &Event) -> Option<AgentUpdate> {
        None
    }

    fn config_file(&self, dirs: &Dirs) -> PathBuf {
        // Gemini CLI makes its `.gemini` inside the folder GEMINI_CLI_HOME names.
        dirs.var("GEMINI_CLI_HOME")
            .unwrap_or(dirs.home())
            .join(".gemini")
            .join("settings.json")
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

    fn install(&self, config: &serde_json::Value, _: &Path) -> serde_json::Value {
        config.clone()
    }

    fn install_blocked(&self, _: &serde_json::Value) -> Option<String> {
        Some("Vults no longer supports Gemini CLI: its hooks can only be removed.".into())
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use serde_json::json;

    #[test]
    fn its_hooks_can_be_removed_but_never_installed() {
        let exe = Path::new("/opt/vults-hook");
        let ours = vults_agent_config::with_ours(&json!({}), &Gemini.hook_entries(exe), crate::MARKER);
        let theirs =
            json!({ "hooks": { "BeforeTool": [{ "hooks": [{ "type": "command", "command": "lint" }] }] } });
        let both = vults_agent_config::with_ours(&theirs, &Gemini.hook_entries(exe), crate::MARKER);

        assert!(Gemini.installed(&ours));
        assert!(Gemini.up_to_date(&ours, exe));
        assert!(Gemini.install_blocked(&json!({})).is_some());
        assert_eq!(Gemini.install(&theirs, exe), theirs);
        assert!(!Gemini.installed(&Gemini.uninstall(&both)));
        assert_eq!(
            Gemini.uninstall(&both)["hooks"]["BeforeTool"],
            theirs["hooks"]["BeforeTool"]
        );
    }

    #[test]
    fn its_events_make_no_session() {
        let e = Event {
            v: vults_protocol::VERSION,
            id: "r1".into(),
            agent: AgentKind::Other,
            agent_name: None,
            event: "BeforeTool".into(),
            wants_reply: false,
            zeca: false,
            terminal: Default::default(),
            payload: json!({ "session_id": "s", "tool_name": "read_file" }),
        };
        assert!(Gemini.parse(&e).is_none());
    }
}
