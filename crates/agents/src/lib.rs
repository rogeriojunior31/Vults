//! What differs between agents: event names, tool names, where their config lives and what
//! the installer adds to it. Everything else in the app is agent-agnostic.

mod claude;
mod codex;

use std::path::{Path, PathBuf};

use vultures_ai_agent_config::HookEntry;
use vultures_ai_core::AgentUpdate;
use vultures_ai_protocol::{AgentKind, Event};

pub use claude::Claude;
pub use codex::{Codex, Trust as CodexTrust, trust as codex_trust};

pub trait Agent: Send + Sync {
    fn kind(&self) -> AgentKind;
    /// Normalizes a hook event; `None` for events the app ignores.
    fn parse(&self, event: &Event) -> Option<AgentUpdate>;
    /// The file the installer edits, under the user's home.
    fn config_file(&self, home: &Path) -> PathBuf;
    /// What the installer adds to that file.
    fn hook_entries(&self, hook_exe: &Path) -> Vec<HookEntry>;
}

pub fn agent(kind: AgentKind) -> Option<&'static dyn Agent> {
    match kind {
        AgentKind::Claude => Some(&Claude),
        AgentKind::Codex => Some(&Codex),
    }
}

/// Our entries are recognized by the hook binary's name in their command.
pub const MARKER: &str = vultures_ai_brand::HOOK_BIN;

/// `'<exe>' --agent <agent> <Event>`, safe for the POSIX shell the agents run hooks with
/// (Git Bash on Windows, hence forward slashes there).
pub fn hook_command(hook_exe: &Path, agent: &str, event: &str) -> String {
    let exe = hook_exe.to_string_lossy();
    let exe = if cfg!(windows) {
        exe.replace('\\', "/")
    } else {
        exe.into_owned()
    };
    format!("'{}' --agent {agent} {event}", exe.replace('\'', r"'\''"))
}

/// What a step shows next to its verb, most specific field first.
const DETAIL_FIELDS: &[&str] = &[
    "command",
    "file_path",
    "path",
    "notebook_path",
    "url",
    "query",
    "pattern",
];
/// What an approval card shows: the exact thing Allow authorizes.
const TARGET_FIELDS: &[&str] = &[
    "command",
    "file_path",
    "path",
    "notebook_path",
    "url",
    "query",
    "pattern",
    "prompt",
    "description",
];

fn first_field<'a>(input: &'a serde_json::Value, fields: &[&'static str]) -> Option<(&'static str, &'a str)> {
    fields.iter().find_map(|&f| {
        let v = input.get(f)?.as_str()?.trim();
        (!v.is_empty()).then_some((f, v))
    })
}

/// `main.rs` for a file, the first 40 characters of a command or query.
pub(crate) fn detail(input: &serde_json::Value) -> Option<String> {
    let (field, value) = first_field(input, DETAIL_FIELDS)?;
    Some(match field {
        "file_path" | "path" | "notebook_path" => file_name(value),
        _ => shorten(value, 40),
    })
}

/// `Bash · rm -rf build`: the tool and the exact thing it would act on.
pub(crate) fn target(tool: &str, input: &serde_json::Value) -> String {
    match first_field(input, TARGET_FIELDS) {
        Some((_, value)) => format!("{tool} · {}", shorten(value, 300)),
        None => tool.to_string(),
    }
}

/// Last path component, for step details: `Editing main.rs`, not the whole path.
pub(crate) fn file_name(path: &str) -> String {
    path.trim_end_matches(['/', '\\'])
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(path)
        .to_string()
}

/// First `max` characters, on a char boundary, with an ellipsis when cut.
pub(crate) fn shorten(text: &str, max: usize) -> String {
    let text = text.trim();
    let line = text.lines().next().unwrap_or("");
    match line.char_indices().nth(max) {
        Some((i, _)) => format!("{}…", &line[..i]),
        None if line.len() < text.len() => format!("{line}…"),
        None => line.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commands_are_quoted_for_the_shell() {
        assert_eq!(
            hook_command(Path::new("/home/me/bin/vultures-ai-hook"), "claude", "Stop"),
            "'/home/me/bin/vultures-ai-hook' --agent claude Stop"
        );
        assert_eq!(
            hook_command(Path::new("/it's/hook"), "claude", "Stop"),
            r"'/it'\''s/hook' --agent claude Stop"
        );
        assert!(hook_command(Path::new("/x/vultures-ai-hook"), "claude", "Stop").contains(MARKER));
    }

    #[test]
    fn shortening() {
        assert_eq!(shorten("cargo test", 40), "cargo test");
        assert_eq!(shorten("abcdef", 3), "abc…");
        assert_eq!(shorten("first\nsecond", 40), "first…");
        assert_eq!(shorten("ééé", 2), "éé…"); // check-english:allow (multi-byte test input)
        assert_eq!(file_name("/a/b/main.rs"), "main.rs");
        assert_eq!(file_name(r"C:\a\lib.rs"), "lib.rs");
    }
}
