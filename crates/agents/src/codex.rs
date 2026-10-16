//! Codex CLI: https://learn.chatgpt.com/docs/hooks
//!
//! Same hook JSON shape as Claude Code. Differences: tools are `Bash`, `apply_patch` (the patch in
//! `tool_input.command`), `update_plan` and MCP tools; there is no Notification event; `Interrupt`
//! ends a turn; `SessionEnd` and `Interrupt` may run for 1 s at most. Codex runs a hook only after
//! the user trusts it with `/hooks`, which records a hash in config.toml that we never write.

use std::path::{Path, PathBuf};

use serde_json::Value;
use vults_agent_config::HookEntry;
use vults_core::{Activity, AgentEvent, AgentUpdate, RequestId, SessionKey, Step};
use vults_protocol::{AgentKind, Event};

use crate::{Agent, Dirs, detail, file_name, hook_command, target};

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
                detail: crate::mcp_label(&tool)
                    .or_else(|| patch_file(&tool, &input))
                    .or_else(|| detail(&input)),
                tool,
            }),
            "PostToolUse" => AgentEvent::ToolFinished {
                failed: false,
                target: Some(request_target(&tool, &input)),
                diff: (tool == "apply_patch")
                    .then(|| crate::diff::codex(input.get("command")?.as_str()?))
                    .flatten(),
            },
            "PermissionRequest" => AgentEvent::PermissionRequested {
                request: RequestId(e.id.clone()),
                target: request_target(&tool, &input),
                ask: crate::ask(&tool, &input),
                tool,
            },
            // A turn ending by the user's hand is a stop, not a failure.
            "Stop" | "Interrupt" => AgentEvent::Stopped {
                message: crate::summary_line(text("last_assistant_message")),
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
            received: None,
        })
    }

    fn config_file(&self, dirs: &Dirs) -> PathBuf {
        crate::codex_home(dirs).join("hooks.json")
    }

    fn hook_entries(&self, hook_exe: &Path) -> Vec<HookEntry> {
        EVENTS
            .iter()
            .map(|&(event, timeout)| HookEntry {
                event,
                command: hook_command(hook_exe, "codex", event),
                timeout,
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
    /// Our entries Codex does not run yet: never trusted, or turned off (`/hooks` in Codex).
    pub untrusted: usize,
    pub total: usize,
}

/// Codex keys a hook's state by `"<hooks.json path>:<event_in_snake_case>:<group>:<hook>"` under
/// `[hooks.state]`, and runs it only with a `trusted_hash` and without `enabled = false` (codex-rs
/// `hooks/src/engine/discovery.rs`). A hash that no longer matches the hook also stops it, but that
/// hash is Codex's own: such an entry counts as trusted here. `config_toml` is the file's text,
/// read as TOML: an unparsable one is unknown, never "nothing trusted".
pub fn trust(
    hooks_json: &Value,
    hooks_path: &Path,
    config_toml: &str,
    marker: &str,
) -> Result<Trust, String> {
    let config: toml::Table = config_toml
        .parse()
        .map_err(|e: toml::de::Error| e.message().to_string())?;
    let states = config
        .get("hooks")
        .and_then(|h| h.get("state"))
        .and_then(toml::Value::as_table);
    let runs = |key: &str| {
        states.and_then(|s| s.get(key)).is_some_and(|state| {
            state.get("trusted_hash").and_then(toml::Value::as_str).is_some()
                && state.get("enabled").and_then(toml::Value::as_bool) != Some(false)
        })
    };
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
                        .is_some_and(|c| vults_agent_config::runs_ours(c, marker));
                    if !ours {
                        continue;
                    }
                    total += 1;
                    if !runs(&format!("{path}:{}:{g}:{h}", snake(event))) {
                        untrusted += 1;
                    }
                }
            }
        }
    }
    let hooks_disabled = config
        .get("features")
        .and_then(|f| f.get("hooks"))
        .and_then(toml::Value::as_bool)
        == Some(false);
    Ok(Trust {
        hooks_disabled,
        untrusted,
        total,
    })
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

/// Files a patch touches, from its `*** Add|Update|Delete File: <path>` headers; a moved one as
/// `old → new`, since Allow lets it write the new path too.
fn patched(input: &Value) -> Vec<String> {
    let patch = input.get("command").and_then(Value::as_str).unwrap_or_default();
    let mut files: Vec<String> = Vec::new();
    for l in patch.lines() {
        if let Some(path) = ["*** Add File: ", "*** Update File: ", "*** Delete File: "]
            .iter()
            .find_map(|h| l.strip_prefix(h))
        {
            files.push(path.trim().to_string());
        } else if let (Some(to), Some(last)) = (l.strip_prefix("*** Move to: "), files.last_mut()) {
            *last = format!("{last} → {}", to.trim());
        }
    }
    files
}

/// `main.rs`, or `main.rs +2` when the patch touches more files.
fn patch_file(tool: &str, input: &Value) -> Option<String> {
    if tool != "apply_patch" {
        return None;
    }
    let files = patched(input);
    // A moved file goes by its new name: its diff does too.
    let first = files.first()?;
    let first = file_name(first.rsplit(" → ").next().unwrap_or(first));
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
    use vults_protocol::Terminal;

    fn event(name: &str, payload: Value) -> Event {
        Event {
            v: vults_protocol::VERSION,
            id: "req-1".into(),
            agent: AgentKind::Codex,
            agent_name: None,
            event: name.into(),
            wants_reply: name == "PermissionRequest",
            zeca: false,
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

    /// A two-file patch recorded from codex-cli 0.160 (paths replaced).
    #[test]
    fn a_finished_patch_carries_its_diff() {
        let p: Value =
            serde_json::from_str(include_str!("../tests/fixtures/codex-apply-patch.jsonl").trim()).unwrap();
        let Some(AgentEvent::ToolFinished { diff: Some(d), .. }) =
            Codex.parse(&event("PostToolUse", p)).map(|u| u.event)
        else {
            panic!("no diff");
        };
        let files: Vec<_> = d
            .files
            .iter()
            .map(|f| (f.path.as_str(), f.added, f.removed))
            .collect();
        assert_eq!(
            files,
            [
                ("/home/me/notes/notes.txt", 1, 1),
                ("/home/me/notes/new.txt", 1, 0)
            ]
        );
        assert!(!d.cut);
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
                ask: vults_core::Ask {
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
    fn a_moved_file_shows_both_paths_and_a_cut_patch_is_cut() {
        let moved =
            "*** Begin Patch\n*** Update File: src/a.rs\n*** Move to: src/b.rs\n@@\n-x\n+y\n*** End Patch";
        let ask = |patch: &str| match Codex
            .parse(&event(
                "PermissionRequest",
                json!({ "session_id": "s", "tool_name": "apply_patch", "tool_input": { "command": patch } }),
            ))
            .map(|u| u.event)
        {
            Some(AgentEvent::PermissionRequested { target, ask, .. }) => (target, ask.cut),
            other => panic!("{other:?}"),
        };
        assert_eq!(ask(moved), ("Edit · src/a.rs → src/b.rs".to_string(), false));
        let d = crate::diff::codex(moved).unwrap();
        assert_eq!(d.files[0].path, "src/b.rs");
        // The step goes by the new name, as its diff does, even at the top of the folder.
        let top = "*** Begin Patch\n*** Update File: a.rs\n*** Move to: b.rs\n@@\n-x\n+y\n*** End Patch";
        assert_eq!(
            patch_file("apply_patch", &json!({ "command": top })).as_deref(),
            Some("b.rs")
        );
        // The hook cut it before its end: a file may be missing, so no Always.
        assert!(ask("*** Begin Patch\n*** Update File: src/a.rs\n@@\n-x\n+y").1);
    }

    #[test]
    fn interrupt_is_a_stop_and_unknown_events_are_ignored() {
        assert_eq!(
            Codex.parse(&event("Interrupt", json!({}))).map(|u| u.event),
            Some(AgentEvent::Stopped { message: None })
        );
        assert!(Codex.parse(&event("PreCompact", json!({}))).is_none());
    }

    fn our_hooks() -> Value {
        json!({ "hooks": {
            "PreToolUse": [
                { "hooks": [ { "command": "other-tool" } ] },
                { "hooks": [ { "command": "'/x/vults-hook' --agent codex PreToolUse" } ] }
            ],
            "SessionStart": [ { "hooks": [ { "command": "'/x/vults-hook' --agent codex SessionStart" } ] } ]
        }})
    }

    #[test]
    fn trust_is_read_from_hooks_state() {
        let path = Path::new("/home/me/.codex/hooks.json");
        // In the shape Codex 0.160.1 wrote it.
        let config = "[features]\nhooks = true\n\n[hooks.state]\n\n[hooks.state.\"/home/me/.codex/hooks.json:pre_tool_use:1:0\"]\ntrusted_hash = \"sha256:0d31155fedae527d077d88fa14701ca87af14ae7a20a5b1885a36deca30b1e0d\"\n";
        assert_eq!(
            trust(&our_hooks(), path, config, "vults-hook"),
            Ok(Trust {
                hooks_disabled: false,
                untrusted: 1,
                total: 2
            })
        );
        assert_eq!(snake("UserPromptSubmit"), "user_prompt_submit");
    }

    /// Codex reads TOML, not lines: every way of writing the same thing reads the same.
    #[test]
    fn trust_reads_toml_written_any_way() {
        let path = Path::new("/home/me/.codex/hooks.json");
        let off = |config: &str| trust(&our_hooks(), path, config, "vults-hook").map(|t| t.hooks_disabled);
        assert_eq!(off("[features]\nhooks = false\n"), Ok(true));
        assert_eq!(off("[features]\nhooks = false # off for now\n"), Ok(true));
        assert_eq!(off("[ features ]\nhooks=false\n"), Ok(true));
        assert_eq!(off("features.hooks = false\n"), Ok(true));
        assert_eq!(off("features = { hooks = false }\n"), Ok(true));
        assert_eq!(
            off("[features]\nhooks = true\n[other]\nhooks = false\n"),
            Ok(false)
        );
        assert_eq!(off(""), Ok(false));
        // A dotted key and an inline table name the same state as Codex's own section.
        let key = "\"/home/me/.codex/hooks.json:pre_tool_use:1:0\"";
        let untrusted = |config: &str| trust(&our_hooks(), path, config, "vults-hook").map(|t| t.untrusted);
        assert_eq!(
            untrusted(&format!("[hooks.state]\n{key}.trusted_hash = \"x\"\n")),
            Ok(1)
        );
        assert_eq!(
            untrusted(&format!(
                "[hooks]\nstate = {{ {key} = {{ trusted_hash = \"x\" }} }}\n"
            )),
            Ok(1)
        );
        // A Windows path, escaped as TOML wants it, is the path Codex keys by.
        let windows = Path::new(r"C:\Users\me\.codex\hooks.json");
        let config = r#"[hooks.state."C:\\Users\\me\\.codex\\hooks.json:session_start:0:0"]
trusted_hash = "x"
"#;
        assert_eq!(
            trust(&our_hooks(), windows, config, "vults-hook").map(|t| t.untrusted),
            Ok(1)
        );
    }

    /// Codex runs a hook only trusted and not turned off (discovery.rs, `hook_enabled`).
    #[test]
    fn a_hook_turned_off_or_never_hashed_does_not_run() {
        let path = Path::new("/home/me/.codex/hooks.json");
        let both = |pre: &str, start: &str| {
            format!(
                "[hooks.state.\"/home/me/.codex/hooks.json:pre_tool_use:1:0\"]\n{pre}\n[hooks.state.\"/home/me/.codex/hooks.json:session_start:0:0\"]\n{start}\n"
            )
        };
        let untrusted =
            |config: String| trust(&our_hooks(), path, &config, "vults-hook").map(|t| t.untrusted);
        assert_eq!(
            untrusted(both("trusted_hash = \"x\"", "trusted_hash = \"y\"")),
            Ok(0)
        );
        assert_eq!(
            untrusted(both(
                "trusted_hash = \"x\"\nenabled = false",
                "trusted_hash = \"y\""
            )),
            Ok(1)
        );
        assert_eq!(
            untrusted(both("trusted_hash = \"x\"\nenabled = true", "enabled = true")),
            Ok(1)
        );
    }

    /// A file Codex itself would refuse says nothing about trust: unknown, never "none trusted".
    #[test]
    fn an_unparsable_config_is_unknown() {
        let path = Path::new("/home/me/.codex/hooks.json");
        assert!(trust(&our_hooks(), path, "[features\nhooks = false", "vults-hook").is_err());
        assert!(trust(&our_hooks(), path, "a = 1\na = 2\n", "vults-hook").is_err());
    }

    #[test]
    fn install_entries() {
        let entries = Codex.hook_entries(Path::new("/opt/vults-hook"));
        assert_eq!(entries.len(), EVENTS.len());
        let end = entries.iter().find(|e| e.event == "SessionEnd").unwrap();
        assert_eq!(end.timeout, 1, "Codex caps SessionEnd at a few seconds");
        assert_eq!(end.command, "'/opt/vults-hook' --agent codex SessionEnd");
        assert_eq!(end.status_message, None);
        let permission = entries.iter().find(|e| e.event == "PermissionRequest").unwrap();
        assert_eq!(permission.status_message, Some(crate::WAITING));
    }
}
