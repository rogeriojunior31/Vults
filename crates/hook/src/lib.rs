//! What the hook makes of an agent's hook JSON: the [`Event`] it sends, built without IO so it can
//! be tested and fuzzed (`fuzz/fuzz_targets/hook_event.rs`). std + serde_json only: it starts on
//! every agent event.

pub mod output;

use serde_json::{Map, Value};
use vults_protocol::{self as protocol, AgentKind, Event, Terminal};

/// Pointless to forward and possibly huge (a whole file, a full command output).
pub const DROPPED_FIELDS: &[&str] = &[
    "tool_response",
    // Copilot CLI's: the tool's whole output, for the model.
    "toolResult",
    // Cursor's: the tool's output, and the signed-in account.
    "tool_output",
    "user_email",
    "transcript_path",
    "transcriptPath",
    "artifactDirectoryPath",
];
/// Lines of a finished edit's patch kept for the island's diff: enough to read there, and far
/// from [`protocol::MAX_MESSAGE`] even with every line at the field cap.
pub const MAX_PATCH_LINES: usize = 400;
/// Codex's patch is `apply_patch`'s command: it keeps more than any other string.
pub const MAX_PATCH_LEN: usize = 64 * 1024;

/// Environment variables that tell which terminal or editor the session runs in.
pub const TERMINAL_VARS: &[&str] = &[
    "TERM_PROGRAM",
    "TERM_SESSION_ID",
    "WT_SESSION",
    "VSCODE_PID",
    // The editor's own binary in VS Code-family terminals: tells Cursor from VS Code.
    "VSCODE_GIT_ASKPASS_NODE",
    "CURSOR_TRACE_ID",
    "KITTY_WINDOW_ID",
    // kitty's remote control socket: `kitty @` from outside kitty needs it.
    "KITTY_LISTEN_ON",
    "WEZTERM_PANE",
    "WEZTERM_UNIX_SOCKET",
    "KONSOLE_DBUS_SESSION",
    "TMUX",
    "TMUX_PANE",
    "HERDR_WORKSPACE_ID",
    "HERDR_TAB_ID",
    "HERDR_PANE_ID",
    "XDG_CURRENT_DESKTOP",
    // Which window system the session's terminal is on: X11 windows can be raised anywhere.
    "XDG_SESSION_TYPE",
    "DISPLAY",
    "WAYLAND_DISPLAY",
];

#[derive(Debug, Clone)]
pub struct Args {
    pub agent: AgentKind,
    /// Another tool's name, with [`AgentKind::Other`].
    pub agent_name: Option<String>,
    pub event: Option<String>,
    /// [`protocol::ASK_FLAG`]: this entry's timeout lets a question wait for the island.
    pub ask: bool,
}

impl Args {
    pub fn parse(mut args: impl Iterator<Item = String>) -> Self {
        let mut parsed = Args {
            agent: AgentKind::Claude,
            agent_name: None,
            event: None,
            ask: false,
        };
        while let Some(arg) = args.next() {
            if arg == "--agent" {
                let Some(name) = args.next() else { continue };
                if let Some(agent) = AgentKind::parse(&name) {
                    parsed.agent = agent;
                } else {
                    // A name it may not use stays nameless, and a nameless tool sends nothing:
                    // taken for Claude Code it would wait on a card and read Claude's JSON.
                    parsed.agent = AgentKind::Other;
                    parsed.agent_name = protocol::valid_agent_name(&name).then_some(name);
                }
            } else if arg == protocol::ASK_FLAG {
                parsed.ask = true;
            } else if arg == "--statusline" {
                parsed.event = Some(protocol::STATUS_LINE_EVENT.to_string());
            } else {
                parsed.event = Some(arg);
            }
        }
        parsed
    }
}

/// Only numeric session metrics and plan windows leave the status line; paths and model data do not.
pub fn status_line_payload(payload: &Map<String, Value>) -> Option<Map<String, Value>> {
    let limits = payload.get("rate_limits").filter(|v| v.is_object());
    let mut cost = Map::new();
    for field in ["total_cost_usd", "total_usd", "total_duration_ms"] {
        if let Some(value) = payload.get("cost").and_then(|c| c.get(field)).filter(|v| {
            if field == "total_duration_ms" {
                v.as_u64().is_some()
            } else {
                v.as_f64().is_some_and(|n| n.is_finite() && n >= 0.0)
            }
        }) {
            cost.insert(field.into(), value.clone());
        }
    }
    let context = payload
        .get("context_window")
        .and_then(|c| c.get("used_percentage"))
        .filter(|v| v.as_f64().is_some_and(|n| (0.0..=100.0).contains(&n)));
    let mut kept = Map::new();
    if let Some(limits) = limits {
        kept.insert("rate_limits".into(), limits.clone());
    }
    if !cost.is_empty() {
        kept.insert("cost".into(), Value::Object(cost));
    }
    if let Some(used) = context {
        kept.insert(
            "context_window".into(),
            serde_json::json!({ "used_percentage": used }),
        );
    }
    if let Some(id) = payload
        .get("model")
        .and_then(|m| m.get("id"))
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty() && s.len() <= 256)
    {
        kept.insert("model".into(), serde_json::json!({"id":id}));
    }
    // Only numeric usage and a model identifier, never account or transcript contents.
    let mut current = Map::new();
    for field in [
        "input_tokens",
        "cache_creation_input_tokens",
        "cache_read_input_tokens",
    ] {
        if let Some(value) = payload
            .get("context_window")
            .and_then(|c| c.get("current_usage"))
            .and_then(|u| u.get(field))
            .filter(|v| v.as_u64().is_some())
        {
            current.insert(field.into(), value.clone());
        }
    }
    if payload
        .get("context_window")
        .and_then(|c| c.get("current_usage"))
        .is_some_and(Value::is_null)
    {
        kept.entry("context_window")
            .or_insert_with(|| serde_json::json!({}))["current_usage"] = Value::Null;
    }
    if !current.is_empty() {
        let context = kept
            .entry("context_window")
            .or_insert_with(|| serde_json::json!({}));
        context["current_usage"] = Value::Object(current);
    }
    if kept.is_empty() {
        return None;
    }
    if let Some(id) = payload
        .get("session_id")
        .or_else(|| payload.get("conversation_id"))
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty() && s.len() <= 256)
    {
        kept.insert("session_id".into(), id.into());
    }
    Some(kept)
}

/// The tool's input exactly as the agent sent it.
pub fn original_input(raw: &[u8]) -> Option<Value> {
    let raw = raw.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(raw);
    let mut payload: Map<String, Value> = serde_json::from_slice(raw).ok()?;
    payload.remove("tool_input")
}

/// The event for the hook JSON; pure, so it can be tested without a process. `pid` is the agent's
/// (the hook's parent), `id` the message's.
pub fn build_event(
    args: &Args,
    raw: &[u8],
    cwd: Option<std::path::PathBuf>,
    env: impl Fn(&str) -> Option<String>,
    pid: Option<u32>,
    id: String,
) -> Option<Event> {
    if args.agent == AgentKind::Other && args.agent_name.is_none() {
        return None;
    }
    // Some shells hand us a UTF-8 BOM, which serde_json rejects.
    let raw = raw.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(raw);
    let mut payload: Map<String, Value> = serde_json::from_slice(raw).ok()?;
    // Cursor runs Claude Code's hooks too (its third-party hooks): its sessions come through its
    // own hooks file, and would otherwise show twice, once as Claude Code.
    if args.agent == AgentKind::Claude && payload.contains_key("cursor_version") {
        return None;
    }

    // The JSON usually names the event; argv only fills the gap. `--statusline` always wins:
    // whatever the input calls itself, only its metrics may leave.
    let status_line = args.event.as_deref() == Some(protocol::STATUS_LINE_EVENT);
    let event = payload
        .get("hook_event_name")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty() && !status_line)
        .map(str::to_string)
        .or_else(|| args.event.clone())?;
    if event == protocol::STATUS_LINE_EVENT {
        payload = status_line_payload(&payload)?;
    }

    // A tool's error is kept, never its output: some agents say a tool failed only in there. A
    // finished edit keeps its patch too (Claude Code's `structuredPatch`), for the island's diff.
    let response = payload.get("tool_response");
    let mut kept = Map::new();
    if let Some(error) = response.and_then(|r| r.get("error")).filter(|e| !e.is_null()) {
        kept.insert("error".into(), error.clone());
    }
    // Copilot CLI says a tool failed in its result's type (the text is then the error), and a
    // command that exited non-zero only at the end of its output: just that is kept.
    if let Some(result) = payload.get("toolResult") {
        let text = result
            .get("textResultForLlm")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if result
            .get("resultType")
            .and_then(Value::as_str)
            .is_some_and(|t| t != "success")
        {
            kept.insert("error".into(), Value::String(text.to_string()));
        } else if let Some(code) = exit_code(text).filter(|c| *c != 0) {
            kept.insert("error".into(), Value::String(format!("exit code {code}")));
        }
    }
    let finished = event == "PostToolUse";
    if let Some(hunks) = response
        .and_then(|r| r.get("structuredPatch"))
        .and_then(Value::as_array)
        .filter(|_| finished)
    {
        let (hunks, cut) = cap_patch(hunks);
        kept.insert("structuredPatch".into(), Value::Array(hunks));
        if cut {
            kept.insert("cut".into(), Value::Bool(true));
        }
    }
    // Every event keeps Codex's patch whole up to its cap, not just a finished one: the approval
    // card names its files from it, and a file past the field cap would go unnamed.
    let patch = payload.get("tool_name").and_then(Value::as_str) == Some("apply_patch");
    let mut patch = patch
        .then(|| payload.get_mut("tool_input")?.get_mut("command").map(Value::take))
        .flatten();
    for field in DROPPED_FIELDS {
        payload.remove(*field);
    }
    if !kept.is_empty() {
        payload.insert("tool_response".into(), Value::Object(kept));
    }
    let mut payload = Value::Object(payload);
    truncate_strings(&mut payload, protocol::MAX_FIELD_LEN);
    if let Some(mut command) = patch.take() {
        truncate_strings(&mut command, MAX_PATCH_LEN);
        payload["tool_input"]["command"] = command;
    }

    // Antigravity and Cursor name their workspace, not always a cwd, and run their hooks from
    // the hooks file's folder.
    let cwd = ["/cwd", "/workspacePaths/0", "/workspace_roots/0"]
        .iter()
        .find_map(|at| payload.pointer(at)?.as_str().filter(|s| !s.is_empty()))
        .map(str::to_string)
        .or_else(|| cwd.map(|p| p.to_string_lossy().into_owned()));
    let terminal = Terminal {
        cwd,
        pid,
        env: TERMINAL_VARS
            .iter()
            .filter_map(|var| env(var).filter(|v| !v.is_empty()).map(|v| (var.to_string(), v)))
            .collect(),
    };

    // The chat's own CLI asks its cards in the chat: a card here would ask twice, and an answer
    // here would skip the chat's. Anyone may set the variable, but all it buys is no card: the
    // agent's own terminal (or the chat) still asks.
    let zeca = env(protocol::ZECA_ENV).as_deref() == Some("1");
    // Only the agents in `output::replies` take an answer; any other tool's own terminal asks.
    let tool = payload.get("tool_name").and_then(Value::as_str);
    let wants_reply = !zeca && output::waits(args.agent, &event, args.ask, tool);
    // A session Vults runs asks on its driver's channel, where the app makes the card: its hook
    // asking too would show two. Nothing is relayed, and the agent asks on that channel. Anyone
    // may set the variable, but all it buys is the agent's own question instead of the hook's.
    let run = env(protocol::RUN_ENV).as_deref() == Some("1");
    // Only its permissions: a question (the ask flag) still waits for the island.
    // The app ends a controlled turn; a short-lived CLI exiting must not remove its bird.
    if run && (wants_reply && event == "PermissionRequest" || event == "SessionEnd") {
        return None;
    }

    let mut event = Event {
        v: protocol::VERSION,
        id,
        agent: args.agent,
        agent_name: args.agent_name.clone(),
        wants_reply,
        zeca,
        event,
        terminal,
        payload,
    };
    fit(&mut event).then_some(event)
}

/// The exit code Copilot CLI ends a command's output with (`<shellId: 4 completed with exit code 1>`).
fn exit_code(text: &str) -> Option<i64> {
    let tail = text.trim_end().strip_suffix('>')?;
    let (_, code) = tail.rsplit_once("with exit code ")?;
    code.trim().parse().ok()
}

/// The fields that say what happened, kept when nothing else fits.
const ESSENTIAL: &[&str] = &[
    "hook_event_name",
    "session_id",
    "sessionId",
    "conversation_id",
    "turn_id",
    "cwd",
    "tool_name",
    "agent_id",
];

/// Makes the event fit in one line the app reads ([`protocol::MAX_MESSAGE`]): the app drops a longer
/// one unread. Strings are cut shorter and shorter, then only the essential fields stay; false when
/// even that does not fit (nothing is sent, as on any failure). An event cut down this way never
/// waits for a card: the card could not show the whole of what Allow would authorize, so the agent's
/// own terminal asks (rule 2).
fn fit(event: &mut Event) -> bool {
    let fits = |e: &Event| serde_json::to_vec(e).is_ok_and(|v| v.len() < protocol::MAX_MESSAGE);
    if fits(event) {
        return true;
    }
    event.wants_reply = false;
    let mut max = protocol::MAX_FIELD_LEN;
    while !fits(event) {
        if max <= 32 {
            let Value::Object(map) = &mut event.payload else {
                return false;
            };
            map.retain(|k, v| ESSENTIAL.contains(&k.as_str()) && v.is_string());
            truncate_strings(&mut event.payload, 200);
            event.terminal.env.clear();
            return fits(event);
        }
        max /= 4;
        truncate_strings(&mut event.payload, max);
    }
    true
}

/// Caps every string, cutting on a char boundary. A single `Write` can carry a whole file.
pub fn truncate_strings(value: &mut Value, max: usize) {
    match value {
        Value::String(s) if s.len() > max => {
            let mut end = max;
            while !s.is_char_boundary(end) {
                end -= 1;
            }
            s.truncate(end);
            s.push('…');
        }
        Value::Array(items) => items.iter_mut().for_each(|v| truncate_strings(v, max)),
        Value::Object(map) => map.values_mut().for_each(|v| truncate_strings(v, max)),
        _ => {}
    }
}

/// The first [`MAX_PATCH_LINES`] lines of a patch's hunks; true when some were left out.
pub fn cap_patch(hunks: &[Value]) -> (Vec<Value>, bool) {
    let mut left = MAX_PATCH_LINES;
    let mut kept = Vec::new();
    for hunk in hunks {
        let Some(lines) = hunk.get("lines").and_then(Value::as_array) else {
            continue;
        };
        if left == 0 {
            return (kept, true);
        }
        let mut hunk = hunk.clone();
        let cut = lines.len() > left;
        hunk["lines"] = Value::Array(lines.iter().take(left).cloned().collect());
        left = left.saturating_sub(lines.len());
        kept.push(hunk);
        if cut {
            return (kept, true);
        }
    }
    (kept, false)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    fn build(raw: &[u8]) -> Option<Event> {
        let args = Args::parse(["--agent", "claude"].map(String::from).into_iter());
        build_event(&args, raw, None, |_| None, None, "x".into())
    }

    #[test]
    fn retired_execution_metadata_is_not_forwarded() {
        let args = Args::parse(["--agent", "claude"].map(String::from).into_iter());
        let raw = br#"{"hook_event_name":"PermissionRequest","session_id":"s","tool_name":"Bash","tool_input":{"command":"ls"}}"#;
        let env = |name: &str| match name {
            "VULTS_TASK" => Some("12".into()),
            "TERM_PROGRAM" => Some("kitty".into()),
            _ => None,
        };
        let event = build_event(&args, raw, None, env, None, "x".into()).unwrap();
        assert_eq!(event.terminal.env.len(), 1);
        assert_eq!(event.terminal.env["TERM_PROGRAM"], "kitty");
        assert!(event.wants_reply && !event.zeca);
    }

    #[test]
    fn wezterm_context_keeps_the_instance_without_forwarding_unrelated_environment() {
        let args = Args::parse(["--agent", "claude"].map(String::from).into_iter());
        let raw = br#"{"hook_event_name":"SessionStart","session_id":"s"}"#;
        let env = |name: &str| match name {
            "WEZTERM_PANE" => Some("17".into()),
            "WEZTERM_UNIX_SOCKET" => Some("/run/user/1000/wezterm/gui-sock-42".into()),
            "UNRELATED_SECRET" => Some("must not leave the environment".into()),
            _ => None,
        };
        let event = build_event(&args, raw, None, env, Some(42), "x".into()).unwrap();
        assert_eq!(event.terminal.env.len(), 2);
        assert_eq!(event.terminal.env["WEZTERM_PANE"], "17");
        assert_eq!(
            event.terminal.env["WEZTERM_UNIX_SOCKET"],
            "/run/user/1000/wezterm/gui-sock-42"
        );
    }

    #[test]
    fn a_huge_payload_still_fits_in_one_line() {
        // 900 long strings: each under the field cap, together past the message cap.
        let edits: Vec<Value> = (0..900).map(|_| Value::String("x".repeat(5000))).collect();
        let raw = serde_json::json!({
            "hook_event_name": "PermissionRequest",
            "session_id": "s",
            "tool_name": "MultiEdit",
            "tool_input": { "edits": edits },
        });
        let e = build(raw.to_string().as_bytes()).unwrap();
        assert!(protocol::encode(&e).len() <= protocol::MAX_MESSAGE);
        assert_eq!(e.event, "PermissionRequest");
        assert_eq!(e.payload["tool_name"], "MultiEdit");
        // Cut down, it can't show what Allow would authorize: no card, the terminal asks.
        assert!(!e.wants_reply);
    }

    #[test]
    fn the_chats_own_events_are_marked_and_never_wait() {
        let args = Args::parse(["--agent", "claude"].map(String::from).into_iter());
        let raw = br#"{"hook_event_name":"PermissionRequest","session_id":"s","tool_name":"Bash","tool_input":{"command":"ls"}}"#;
        let zeca = |var: &str| (var == protocol::ZECA_ENV).then(|| "1".to_string());
        let e = build_event(&args, raw, None, zeca, None, "x".into()).unwrap();
        assert!(e.zeca);
        assert!(!e.wants_reply);
        // Any other value is not the chat.
        let other = |var: &str| (var == protocol::ZECA_ENV).then(|| "0".to_string());
        let e = build_event(&args, raw, None, other, None, "x".into()).unwrap();
        assert!(!e.zeca);
        assert!(e.wants_reply);
    }

    #[test]
    fn a_permission_that_fits_still_waits_for_its_card() {
        let raw = br#"{"hook_event_name":"PermissionRequest","session_id":"s","tool_name":"Bash","tool_input":{"command":"ls"}}"#;
        assert!(build(raw).unwrap().wants_reply);
    }

    #[test]
    fn a_payload_of_countless_keys_keeps_only_what_says_what_happened() {
        let mut map = serde_json::Map::new();
        map.insert("hook_event_name".into(), "PreToolUse".into());
        map.insert("session_id".into(), "s".into());
        for i in 0..200_000 {
            map.insert(format!("k{i}"), Value::Bool(true));
        }
        let e = build(Value::Object(map).to_string().as_bytes()).unwrap();
        assert!(protocol::encode(&e).len() <= protocol::MAX_MESSAGE);
        assert_eq!(e.payload["session_id"], "s");
        assert!(e.payload.get("k1").is_none());
    }
}
