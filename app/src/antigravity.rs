//! Approving Antigravity's shell commands from the island (Settings → Agents → Antigravity, off by
//! default). agy has no permission request: it runs its `PreToolUse` hook before its own
//! permission check and takes the hook's decision. The gate here only decides whether a card shows;
//! it never allows anything. No card means the hook prints nothing and agy asks as it always does.

use std::path::Path;

use serde_json::Value;
use tauri::AppHandle;
use vults_protocol::Event;

/// agy stops waiting for its hook after the hook's timeout; a card needs this many seconds of it,
/// and the hook answers by 110 s (`limits::DECISION_BUDGET`). Hooks written before had 5 s.
pub const HOOK_TIMEOUT: u64 = 120;

/// Whether this agy `PreToolUse` is a shell command to put on a card.
pub(crate) fn gate(app: &AppHandle, event: &Event) -> bool {
    if !crate::settings::antigravity_approvals(app) {
        return false;
    }
    let home = crate::paths::agent_dirs().home().to_path_buf();
    gate_in(event, &home)
}

fn gate_in(event: &Event, home: &Path) -> bool {
    if event.agent_name.as_deref() != Some("antigravity") {
        return false;
    }
    let Some(command) = vults_agents::antigravity_command(event) else {
        return false;
    };
    // The hook cuts long strings: a command not seen whole is never one to approve.
    if command.ends_with('…') && command.len() >= vults_protocol::MAX_FIELD_LEN {
        return false;
    }
    if pre_tool_timeout(home).is_none_or(|t| t < HOOK_TIMEOUT) {
        return false;
    }
    let cwd = event
        .payload
        .pointer("/workspacePaths/0")
        .and_then(Value::as_str)
        .or(event.terminal.cwd.as_deref())
        .unwrap_or_default();
    vults_agents::AgyRules::load(home, cwd).verdict(&command) == vults_agents::AgyVerdict::Unknown
}

/// The timeout, in seconds, of our `PreToolUse` hook in agy's hooks file; None when it is not there.
fn pre_tool_timeout(home: &Path) -> Option<u64> {
    let path = home.join(".gemini").join("config").join("hooks.json");
    if !std::fs::metadata(&path).is_ok_and(|m| m.is_file()) {
        return None;
    }
    let bytes = std::fs::read(path).ok()?;
    let bytes = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(&bytes);
    let root: Value = serde_json::from_slice(bytes).ok()?;
    root.get(vults_brand::SLUG)?
        .get("PreToolUse")?
        .as_array()?
        .iter()
        .filter_map(|group| group.get("hooks")?.as_array())
        .flatten()
        .filter_map(|hook| hook.get("timeout")?.as_u64())
        .min()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn event(tool: &str, command: &str) -> Event {
        Event {
            v: vults_protocol::VERSION,
            id: "r1".into(),
            agent: vults_protocol::AgentKind::Other,
            agent_name: Some("antigravity".into()),
            event: "PreToolUse".into(),
            wants_reply: true,
            zeca: false,
            terminal: Default::default(),
            payload: json!({"conversationId": "c1", "workspacePaths": ["/work/site"],
                "toolCall": {"name": tool, "args": {"CommandLine": command}}}),
        }
    }

    fn home(timeout: u64, allow: &[&str]) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "vults-agy-gate-{}-{timeout}-{}",
            std::process::id(),
            allow.len()
        ));
        let config = dir.join(".gemini").join("config");
        std::fs::create_dir_all(&config).unwrap();
        let hook = json!({"type": "command", "command": "vults-hook --agent antigravity PreToolUse || exit 0", "timeout": timeout});
        std::fs::write(
            config.join("hooks.json"),
            json!({ vults_brand::SLUG: {"PreToolUse": [{"matcher": "*", "hooks": [hook]}]} }).to_string(),
        )
        .unwrap();
        let grants: Vec<String> = allow.iter().map(|c| format!("command({c})")).collect();
        std::fs::create_dir_all(dir.join(".gemini").join("antigravity-cli")).unwrap();
        std::fs::write(
            dir.join(".gemini").join("antigravity-cli").join("settings.json"),
            json!({"permissions": {"allow": grants}}).to_string(),
        )
        .unwrap();
        dir
    }

    #[test]
    fn only_a_shell_command_agy_has_no_rule_for_goes_on_a_card() {
        let ready = home(120, &["git status"]);
        assert!(gate_in(&event("run_command", "cargo test"), &ready));
        // agy's own rule covers it: agy decides, no card.
        assert!(!gate_in(&event("run_command", "git status"), &ready));
        // Not a shell command, or another tool's event.
        assert!(!gate_in(&event("view_file", "x"), &ready));
        let mut other = event("run_command", "cargo test");
        other.agent_name = Some("cursor".into());
        assert!(!gate_in(&other, &ready));
        // A command the hook cut is never approved unseen.
        let cut = format!("{}…", "x".repeat(vults_protocol::MAX_FIELD_LEN));
        assert!(!gate_in(&event("run_command", &cut), &ready));
        // Hooks written with the old 5 s timeout: agy would kill the hook before a click.
        let old = home(5, &[]);
        assert!(!gate_in(&event("run_command", "cargo test"), &old));
        let _ = std::fs::remove_dir_all(ready);
        let _ = std::fs::remove_dir_all(old);
    }
}
