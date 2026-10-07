//! Antigravity (the `agy` CLI, the desktop app and the IDE share one hooks file):
//! `~/.gemini/config/hooks.json`, documented in agy's own `agy-customizations` skill.
//!
//! It has no `--agent` of its own on the wire: its sessions are another tool's, named
//! `antigravity` (violet, like any other tool). What differs from Claude Code's hook JSON:
//! - the payload does not name its event, so each entry passes it on the command line;
//! - camelCase fields: `conversationId`, `workspacePaths`, `toolCall: { name, args }` with
//!   PascalCase args (`CommandLine`, `AbsolutePath`);
//! - no session start or end: a session shows with its first event and leaves when quiet;
//! - the file's top-level keys name hooks; ours is one key, timeouts are in seconds;
//! - a hook that fails (exit status not 0) blocks the tool call, so every command ends in
//!   `|| exit 0`: a missing or broken hook binary must not stop the agent.
//!
//! A `PreToolUse` hook could answer allow or deny, but it runs before every tool, read-only ones
//! included, and says nothing of whether agy would have asked: the island leaves permissions to
//! agy's terminal and never waits.

use std::path::{Path, PathBuf};

use serde_json::{Map, Value, json};
use vultures_ai_agent_config::{HookEntry, named};
use vultures_ai_core::{Activity, AgentEvent, AgentUpdate, SessionKey, Step};
use vultures_ai_protocol::{AgentKind, Event};

use crate::{Agent, MARKER, detail, hook_command, target};

/// The name its sessions go by, and the `--agent` its hooks run with.
pub(crate) const NAME: &str = "antigravity";
/// Our key in the hooks file.
const KEY: &str = vultures_ai_brand::SLUG;

#[derive(Debug)]
pub struct Antigravity;

/// Events we register, with their timeout in seconds. `PostInvocation` says nothing new: the
/// next tool or the stop does. Hooks run in agy's loop, one after the other, so fewer is faster.
const EVENTS: &[(&str, u64)] = &[
    ("PreInvocation", 5),
    ("PreToolUse", 5),
    ("PostToolUse", 5),
    ("Stop", 5),
];

/// Only tool events take a `matcher` group; the others are a flat list of handlers.
fn grouped(event: &str) -> bool {
    matches!(event, "PreToolUse" | "PostToolUse")
}

/// agy's PascalCase arguments under the names the island reads.
const ARGS: &[(&str, &str)] = &[
    ("CommandLine", "command"),
    ("AbsolutePath", "file_path"),
    ("TargetFile", "file_path"),
    ("File", "file_path"),
    ("DirectoryPath", "path"),
    ("SearchPath", "path"),
    ("Url", "url"),
    ("Query", "query"),
    ("Pattern", "pattern"),
    ("Description", "description"),
];

impl Agent for Antigravity {
    fn kind(&self) -> AgentKind {
        AgentKind::Other
    }

    fn parse(&self, e: &Event) -> Option<AgentUpdate> {
        let p = &e.payload;
        let text = |k: &str| p.get(k).and_then(Value::as_str).unwrap_or_default();
        let id = text("conversationId");
        if id.is_empty() {
            return None;
        }
        let tool = p
            .pointer("/toolCall/name")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let input = input(p.pointer("/toolCall/args"));

        let event = match e.event.as_str() {
            // Before every model call: the bird thinks between tools too.
            "PreInvocation" => AgentEvent::PromptSubmitted,
            "PreToolUse" => AgentEvent::ToolStarted(Step {
                activity: activity(tool),
                detail: detail(&input).or_else(|| summary(p)),
                tool: tool.to_string(),
            }),
            "PostToolUse" => AgentEvent::ToolFinished {
                failed: !text("error").is_empty(),
                target: Some(target(tool, &input)),
                diff: None,
            },
            "Stop" => match crate::filled(text("error")) {
                Some(error) => AgentEvent::StopFailed { error: Some(error) },
                None => AgentEvent::Stopped { message: None },
            },
            _ => return None,
        };

        Some(AgentUpdate {
            terminal: e.terminal.clone(),
            agent_id: None,
            session: SessionKey {
                agent: AgentKind::Other,
                session_id: format!("{NAME}/{id}"),
            },
            // The hook runs in the hooks file's folder; the workspace is where agy works.
            cwd: p
                .pointer("/workspacePaths/0")
                .and_then(Value::as_str)
                .filter(|c| !c.is_empty())
                .map(str::to_string)
                .or_else(|| e.terminal.cwd.clone()),
            event,
        })
    }

    fn config_file(&self, home: &Path) -> PathBuf {
        home.join(".gemini").join("config").join("hooks.json")
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
        named::with_ours(config, KEY, MARKER, self.ours(hook_exe))
    }

    fn install_blocked(&self, config: &Value) -> Option<String> {
        named::taken(config, KEY, MARKER).then(|| {
            format!("A hook named \"{KEY}\" that does not run ours is already in this file. Rename or remove it, then install.")
        })
    }

    fn uninstall(&self, config: &Value) -> Value {
        named::remove_ours(config, KEY, MARKER)
    }

    fn installed(&self, config: &Value) -> bool {
        named::has_ours(config, KEY, MARKER)
    }

    fn up_to_date(&self, config: &Value, hook_exe: &Path) -> bool {
        named::ours_match(config, KEY, &self.ours(hook_exe))
    }

    fn our_command<'a>(&self, config: &'a Value) -> Option<&'a str> {
        named::our_command(config, KEY, MARKER)
    }
}

impl Antigravity {
    /// Our key's value: `{ "<Event>": [ … ] }`, one handler per event.
    fn ours(&self, hook_exe: &Path) -> Value {
        let mut events = Map::new();
        for entry in self.hook_entries(hook_exe) {
            let handler = json!({ "type": "command", "command": entry.command, "timeout": entry.timeout });
            let list = if grouped(entry.event) {
                json!([{ "matcher": "*", "hooks": [handler] }])
            } else {
                json!([handler])
            };
            events.insert(entry.event.into(), list);
        }
        Value::Object(events)
    }
}

/// The tool's arguments with agy's names copied to the ones the island reads.
fn input(args: Option<&Value>) -> Value {
    let Some(Value::Object(args)) = args else {
        return Value::Null;
    };
    let mut input = args.clone();
    for (from, to) in ARGS {
        if let Some(v) = args.get(*from).filter(|_| !input.contains_key(*to)) {
            input.insert((*to).into(), v.clone());
        }
    }
    Value::Object(input)
}

/// agy's own words for the step (`View rec.sh`), when no argument says what it acts on.
fn summary(p: &Value) -> Option<String> {
    let s = p.pointer("/toolCall/args/toolSummary")?.as_str()?;
    crate::filled(s).map(|s| crate::shorten(&s, 40))
}

/// Tool names are agy's step types, lowercased (`CORTEX_STEP_TYPE_VIEW_FILE` is `view_file`);
/// the model's own names for a few of them are here too.
fn activity(tool: &str) -> Activity {
    match tool {
        "view_file" | "view_file_outline" | "view_code_item" | "view_content_chunk" | "list_dir"
        | "list_directory" | "read_resource" | "list_resources" | "read_notebook" | "read_terminal"
        | "command_status" => Activity::Read,
        "grep_search"
        | "find"
        | "find_by_name"
        | "code_search"
        | "codebase_search"
        | "search_in_file"
        | "find_all_references"
        | "internal_search"
        | "tool_search" => Activity::Search,
        "code_action"
        | "file_change"
        | "propose_code"
        | "write_to_file"
        | "replace_file_content"
        | "multi_replace_file_content"
        | "write_blob"
        | "edit_notebook"
        | "move"
        | "delete_directory" => Activity::Edit,
        "run_command" | "shell_exec" | "send_command_input" | "execute_notebook" | "compile"
        | "restart_dev_server" | "git_commit" => Activity::Run,
        "search_web" | "read_url_content" | "read_browser_page" | "open_browser_url" | "mcp_tool" => {
            Activity::Web
        }
        "plan_input" | "task_boundary" => Activity::Plan,
        "invoke_subagent" | "browser_subagent" => Activity::Subagent,
        t if t.starts_with("browser_") => Activity::Web,
        _ => Activity::Work,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vultures_ai_protocol::Terminal;

    /// Hook calls recorded from a real agy 1.2.16 session (read a file, list the folder, search
    /// it), as `[event from the command line, stdin]`, with the home and project paths trimmed.
    const SESSION: &str = include_str!("../tests/fixtures/antigravity-session.jsonl");

    fn event(name: &str, payload: Value) -> Event {
        Event {
            v: vultures_ai_protocol::VERSION,
            id: "r1".into(),
            agent: AgentKind::Other,
            agent_name: Some(NAME.into()),
            event: name.into(),
            wants_reply: false,
            terminal: Terminal {
                cwd: Some("/home/me/.gemini/config".into()),
                ..Terminal::default()
            },
            payload,
        }
    }

    fn updates() -> Vec<AgentUpdate> {
        SESSION
            .lines()
            .map(|l| serde_json::from_str::<(String, Value)>(l).unwrap())
            .filter_map(|(name, stdin)| crate::parse(&event(&name, stdin)))
            .collect()
    }

    #[test]
    fn a_real_session_reads_as_steps() {
        let updates = updates();
        let first = &updates[0];
        assert_eq!(first.session.agent, AgentKind::Other);
        assert_eq!(
            first.session.session_id,
            "antigravity/847b8379-6531-45dc-8820-601246678e26"
        );
        // The workspace, not the folder the hook runs in.
        assert_eq!(first.cwd.as_deref(), Some("/home/me/proj"));

        let events: Vec<&AgentEvent> = updates.iter().map(|u| &u.event).collect();
        assert_eq!(events[0], &AgentEvent::PromptSubmitted);
        assert_eq!(
            events[1],
            &AgentEvent::ToolStarted(Step {
                activity: Activity::Read,
                detail: Some("rec.sh".into()),
                tool: "view_file".into(),
            })
        );
        assert_eq!(
            events[2],
            &AgentEvent::ToolFinished {
                failed: false,
                target: Some("view_file · /home/me/proj/rec.sh".into()),
                diff: None,
            }
        );
        assert_eq!(events.last(), Some(&&AgentEvent::Stopped { message: None }));
        // PostInvocation says nothing; every other call is one update.
        let posts = SESSION
            .lines()
            .filter(|l| l.starts_with("[\"PostInvocation\""))
            .count();
        assert_eq!(updates.len(), SESSION.lines().count() - posts);
    }

    #[test]
    fn a_command_shows_what_it_runs() {
        let u = Antigravity
            .parse(&event(
                "PreToolUse",
                json!({ "conversationId": "c1", "toolCall": { "name": "run_command",
                        "args": { "CommandLine": "echo hi", "Cwd": "/p", "toolSummary": "Run echo command" } } }),
            ))
            .unwrap();
        assert_eq!(
            u.event,
            AgentEvent::ToolStarted(Step {
                activity: Activity::Run,
                detail: Some("echo hi".into()),
                tool: "run_command".into(),
            })
        );
        // No workspace in the payload: the terminal's folder is all there is.
        assert_eq!(u.cwd.as_deref(), Some("/home/me/.gemini/config"));
    }

    #[test]
    fn failures_say_so() {
        let failed = Antigravity
            .parse(&event(
                "PostToolUse",
                json!({ "conversationId": "c1", "error": "exit status 1",
                        "toolCall": { "name": "run_command", "args": { "CommandLine": "false" } } }),
            ))
            .unwrap();
        assert!(matches!(
            failed.event,
            AgentEvent::ToolFinished { failed: true, .. }
        ));
        let stopped = Antigravity
            .parse(&event(
                "Stop",
                json!({ "conversationId": "c1", "terminationReason": "error", "error": "quota" }),
            ))
            .unwrap();
        assert_eq!(
            stopped.event,
            AgentEvent::StopFailed {
                error: Some("quota".into())
            }
        );
    }

    #[test]
    fn no_conversation_no_session() {
        assert!(Antigravity.parse(&event("Stop", json!({}))).is_none());
        assert!(
            Antigravity
                .parse(&event("PostInvocation", json!({ "conversationId": "c1" })))
                .is_none()
        );
    }

    #[test]
    fn the_hooks_file_gets_one_key_that_never_blocks() {
        let exe = Path::new("/opt/vultures-ai-hook");
        let file = json!({ "lint": { "PostToolUse": [ { "matcher": "run_command", "hooks": [ { "command": "./lint.sh" } ] } ] } });
        let installed = Antigravity.install(&file, exe);
        assert_eq!(installed["lint"], file["lint"]);
        assert_eq!(
            installed[KEY]["PreToolUse"],
            json!([{ "matcher": "*", "hooks": [{ "type": "command",
                "command": "'/opt/vultures-ai-hook' --agent antigravity PreToolUse || exit 0", "timeout": 5 }] }])
        );
        assert_eq!(
            installed[KEY]["Stop"],
            json!([{ "type": "command", "command": "'/opt/vultures-ai-hook' --agent antigravity Stop || exit 0", "timeout": 5 }])
        );
        assert!(Antigravity.installed(&installed));
        assert!(Antigravity.up_to_date(&installed, exe));
        assert!(!Antigravity.up_to_date(&installed, Path::new("/new/vultures-ai-hook")));
        assert_eq!(
            crate::other_hook(&Antigravity, &installed, Path::new("/new/vultures-ai-hook")),
            Some(exe.to_path_buf())
        );
        assert_eq!(Antigravity.uninstall(&installed), file);
    }

    #[test]
    fn its_name_is_a_tools_name() {
        assert!(vultures_ai_protocol::valid_agent_name(NAME));
        assert!(crate::installable(NAME).is_some());
        assert!(crate::installable("other").is_none());
        assert!(crate::installable("my-tool").is_none());
        assert_eq!(crate::installable("gemini").unwrap().kind(), AgentKind::Gemini);
    }

    /// Through the real preview and apply on a temp file: a dated backup of the exact bytes,
    /// and the other tools' keys come back with the same text, before and after uninstall.
    #[test]
    fn install_on_disk_backs_up_and_keeps_other_keys_bytes() {
        use vultures_ai_agent_config as config;
        let dir = std::env::temp_dir().join(format!("vultures-agy-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("hooks.json");
        let lint = "  \"lint\": {\n    \"PostToolUse\": [\n      {\n        \"matcher\": \"run_command\",\n        \"hooks\": [\n          {\n            \"command\": \"./lint.sh\"\n          }\n        ]\n      }\n    ]\n  }";
        let guard = "  \"guard\": {\n    \"enabled\": false,\n    \"Stop\": []\n  }";
        let original = format!("{{\n{lint},\n{guard}\n}}\n");
        std::fs::write(&path, &original).unwrap();
        let exe = Path::new("/opt/vultures-ai-hook");

        let plan = config::preview(&path, |v| Antigravity.install(v, exe)).unwrap();
        assert!(!plan.is_noop());
        let backup = config::apply(
            &path,
            &plan.fingerprint,
            |v| Antigravity.install(v, exe),
            std::time::SystemTime::now(),
        )
        .unwrap()
        .expect("a backup");
        assert_eq!(std::fs::read_to_string(&backup).unwrap(), original);
        let written = std::fs::read_to_string(&path).unwrap();
        assert!(written.contains(lint) && written.contains(guard), "{written}");
        assert!(Antigravity.installed(&config::read_json(&path).unwrap()));

        let plan = config::preview(&path, |v| Antigravity.uninstall(v)).unwrap();
        config::apply(
            &path,
            &plan.fingerprint,
            |v| Antigravity.uninstall(v),
            std::time::SystemTime::now(),
        )
        .unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_users_hook_with_our_name_blocks_the_install() {
        let file = json!({ KEY: { "Stop": [ { "command": "~/bin/wrap.sh" } ] } });
        assert!(Antigravity.install_blocked(&file).is_some());
        assert_eq!(Antigravity.install(&file, Path::new("/x/vultures-ai-hook")), file);
        let ours = Antigravity.install(&json!({}), Path::new("/x/vultures-ai-hook"));
        assert!(Antigravity.install_blocked(&ours).is_none());
        assert!(Antigravity.install_blocked(&json!({})).is_none());
    }

    /// A broken hook must not stop agy: run the command as agy does, with a binary that is gone.
    #[cfg(unix)]
    #[test]
    fn a_missing_hook_still_exits_0() {
        let command = &Antigravity.hook_entries(Path::new("/nonexistent/vultures-ai-hook"))[0].command;
        let out = std::process::Command::new("sh")
            .args(["-c", command])
            .output()
            .unwrap();
        assert!(out.status.success());
        assert!(out.stdout.is_empty());
    }
}
