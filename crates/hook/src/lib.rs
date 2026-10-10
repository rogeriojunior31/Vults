//! What the hook makes of an agent's hook JSON: the [`Event`] it sends, built without IO so it can
//! be tested and fuzzed (`fuzz/fuzz_targets/hook_event.rs`). std + serde_json only: it starts on
//! every agent event.

pub mod output;

use serde_json::{Map, Value};
use vults_protocol::{self as protocol, AgentKind, Event, Terminal};

/// Pointless to forward and possibly huge (a whole file, a full command output).
pub const DROPPED_FIELDS: &[&str] = &[
    "tool_response",
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

#[derive(Debug)]
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

/// Claude Code's statusLine input carries the whole session (paths, cost, model); only the
/// usage is forwarded. Before the session's first reply there is none: nothing to send.
pub fn status_line_payload(payload: &Map<String, Value>) -> Option<Map<String, Value>> {
    let limits = payload.get("rate_limits").filter(|v| v.is_object())?;
    let mut kept = Map::new();
    kept.insert("rate_limits".into(), limits.clone());
    if let Some(id) = payload.get("session_id") {
        kept.insert("session_id".into(), id.clone());
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

    // The JSON usually names the event; argv only fills the gap. `--statusline` always wins:
    // whatever the input calls itself, only its usage may leave.
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

    // A tool's error is kept, never its output: Gemini says a tool failed only in there. A
    // finished edit keeps its patch too (Claude Code's `structuredPatch`), for the island's diff.
    let response = payload.get("tool_response");
    let mut kept = Map::new();
    if let Some(error) = response.and_then(|r| r.get("error")).filter(|e| !e.is_null()) {
        kept.insert("error".into(), error.clone());
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
    let patch = finished && payload.get("tool_name").and_then(Value::as_str) == Some("apply_patch");
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

    // Only the agents in `output::replies` take an answer; any other tool's own terminal asks.
    let tool = payload.get("tool_name").and_then(Value::as_str);
    let wants_reply = output::waits(args.agent, &event, args.ask, tool);

    let mut event = Event {
        v: protocol::VERSION,
        id,
        agent: args.agent,
        agent_name: args.agent_name.clone(),
        wants_reply,
        event,
        terminal,
        payload,
    };
    fit(&mut event).then_some(event)
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
/// one unread, and its card would never show. Strings are cut shorter and shorter, then only the
/// essential fields stay; false when even that does not fit (nothing is sent, as on any failure).
fn fit(event: &mut Event) -> bool {
    let fits = |e: &Event| serde_json::to_vec(e).is_ok_and(|v| v.len() < protocol::MAX_MESSAGE);
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
