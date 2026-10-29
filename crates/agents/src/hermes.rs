//! Hermes 0.22.2 Python observer plugin. No tools, provider calls or approval decisions.
use crate::{Dirs, PluginAgent};
use std::path::{Path, PathBuf};
use vults_core::{AgentEvent, AgentUpdate};
use vults_protocol::Event;

pub const NAME: &str = "hermes";
pub static HERMES: PluginAgent = PluginAgent { name: NAME, files };

fn files(dirs: &Dirs, _: &Path, hook: &Path) -> Vec<(PathBuf, String)> {
    let root = dirs.agent("HERMES_HOME", ".hermes").join("plugins/vults");
    let marker = crate::plugin_marker();
    let manifest = format!(
        "# {marker}\nname: vults\nversion: 1.0.0\nrequires_hermes: '>=0.22'\ndescription: Local Vults Flock session observer\nprovides_hooks:\n  - on_session_start\n  - on_session_end\n  - on_session_finalize\n  - on_session_reset\n  - on_session_delete\n  - pre_tool_call\n  - post_tool_call\n  - post_api_request\n  - subagent_start\n  - subagent_stop\n"
    );
    let code = include_str!("../templates/hermes.py")
        .replace("__VULTS_MARKER__", &marker)
        .replace(
            "__VULTS_RELAY__",
            &serde_json::to_string(&hook.to_string_lossy()).unwrap_or_default(),
        );
    vec![
        (root.join("__init__.py"), code),
        (root.join("plugin.yaml"), manifest),
    ]
}

pub(crate) fn parse(event: &Event) -> Option<AgentUpdate> {
    let mut e = event.clone();
    let name = e
        .payload
        .get("tool_name")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let tool = match name.as_str() {
        "terminal" => "Bash",
        "read_file" => "Read",
        "write_file" => "Write",
        "patch" => "Edit",
        "search_files" => "Grep",
        "web_search" => "WebSearch",
        "web_extract" | "browse" => "WebFetch",
        "delegate_task" => "Task",
        other => other,
    };
    if let Some(p) = e.payload.as_object_mut() {
        p.insert("tool_name".into(), tool.into());
        if let Some(input) = p.get_mut("tool_input").and_then(|v| v.as_object_mut())
            && let Some(path) = input.get("path").cloned()
        {
            input.entry("file_path").or_insert(path);
        }
    }
    let mut update = crate::other::as_claude(&e, NAME)?;
    // Hermes also accepts a Codex-style multi-file patch; preserve it rather than claiming an empty edit.
    if name == "patch"
        && event.event == "PostToolUse"
        && let Some(patch) = event.payload["tool_input"]["patch"].as_str()
        && let AgentEvent::ToolFinished { diff, .. } = &mut update.event
    {
        *diff = crate::diff::codex(patch);
    }
    Some(update)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn patch_events_keep_native_ids_and_use_the_hermes_namespace() {
        let event = Event {
            v: vults_protocol::VERSION,
            id: "wire".into(),
            agent: vults_protocol::AgentKind::Other,
            agent_name: Some(NAME.into()),
            event: "PostToolUse".into(),
            wants_reply: false,
            zeca: false,
            terminal: Default::default(),
            payload: json!({"session_id":"s","cwd":"/w","tool_name":"patch","tool_use_id":"call", "tool_input":{"path":"/w/a.py","old_string":"old","new_string":"new"}}),
        };
        let parsed = crate::parse(&event).unwrap();
        assert_eq!(parsed.session.session_id, "hermes/s");
        assert_eq!(parsed.call_id.as_deref(), Some("call"));
        let AgentEvent::ToolFinished { diff: Some(diff), .. } = parsed.event else {
            panic!("patch diff")
        };
        assert_eq!(diff.files[0].path, "/w/a.py");
        assert_eq!((diff.files[0].added, diff.files[0].removed), (1, 1));
        let mut lifecycle = event;
        lifecycle.event = "SessionStart".into();
        lifecycle.payload = json!({"session_id":"s","cwd":"/w"});
        assert!(matches!(
            crate::parse(&lifecycle).unwrap().event,
            AgentEvent::SessionStarted
        ));
    }

    #[test]
    fn plugin_respects_the_hermes_home_override() {
        let home = crate::dirs::abs("/home/me");
        let dirs = Dirs::new(&home, |key| {
            (key == "HERMES_HOME").then(|| crate::dirs::abs("/profiles/h").into_os_string())
        });
        let parts = files(&dirs, &home.join(".config"), &home.join("relay"));
        assert_eq!(parts.len(), 2);
        assert!(
            parts
                .iter()
                .all(|(p, _)| p.starts_with(crate::dirs::abs("/profiles/h/plugins/vults")))
        );
    }

    #[cfg(unix)]
    #[test]
    fn generated_python_observer_preserves_events_and_cannot_answer_or_leak_results() {
        let dir = std::env::temp_dir().join(format!("vults-hermes-plugin-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let code = files(&Dirs::home_only(&dir), &dir, &dir.join("hook"))[0]
            .1
            .clone();
        let module = dir.join("plugin.py");
        std::fs::write(&module, code).unwrap();
        let result = std::process::Command::new("python3")
            .arg(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/hermes-plugin.py"))
            .arg(module)
            .env_remove("VULTS_RUN")
            .env_remove("VULTS_ZECA")
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        std::fs::remove_dir_all(dir).unwrap();
    }
}
