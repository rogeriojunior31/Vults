//! `vultures-ai-hook [--agent claude|codex] [--ask] [EventName]`: the relay an agent runs on every hook event.
//!
//! Reads the hook JSON on stdin, wraps it in a protocol [`Event`] and hands it to the app.
//! Hard rule: **never block the agent.** Every failure (app closed, socket wedged, garbage
//! reply, unknown version) ends in exit 0 with an empty stdout, and the agent carries on as
//! if we were not installed. The deadlines are enforced by the main thread, so a peer that
//! accepts and then stops reading cannot hold us either.

mod output;

use std::io::{Read, Write};
use std::sync::mpsc;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Map, Value};
use vultures_ai_protocol::{self as protocol, AgentKind, Event, Reply, Terminal, limits};

/// Pointless to forward and possibly huge (a whole file, a full command output).
const DROPPED_FIELDS: &[&str] = &["tool_response", "transcript_path"];
/// Lines of a finished edit's patch kept for the island's diff: enough to read there, and far
/// from [`protocol::MAX_MESSAGE`] even with every line at the field cap.
const MAX_PATCH_LINES: usize = 400;
/// Codex's patch is `apply_patch`'s command: it keeps more than any other string.
const MAX_PATCH_LEN: usize = 64 * 1024;

/// Environment variables that tell which terminal or editor the session runs in.
const TERMINAL_VARS: &[&str] = &[
    "TERM_PROGRAM",
    "TERM_SESSION_ID",
    "WT_SESSION",
    "VSCODE_PID",
    // The editor's own binary in VS Code-family terminals: tells Cursor from VS Code.
    "VSCODE_GIT_ASKPASS_NODE",
    "CURSOR_TRACE_ID",
    "KITTY_WINDOW_ID",
    "WEZTERM_PANE",
    "KONSOLE_DBUS_SESSION",
    "TMUX",
    "TMUX_PANE",
    "HERDR_WORKSPACE_ID",
    "HERDR_TAB_ID",
    "HERDR_PANE_ID",
    "XDG_CURRENT_DESKTOP",
];

fn main() {
    let args = Args::parse(std::env::args().skip(1));
    if let Some((event, raw)) = read_event(&args) {
        let budget = if event.wants_reply {
            limits::DECISION_BUDGET
        } else {
            limits::FIRE_AND_FORGET_BUDGET
        };
        let agent = event.agent;

        // The worker owns every blocking call; if it overruns we stop listening and exit,
        // and the process dying takes the connection with it.
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(talk(&event));
        });
        let json = match rx.recv_timeout(budget) {
            Ok(Some(Outcome::Decision(decision))) => output::decision_json(agent, decision),
            // The answers go back inside the tool's own input, as the agent sent it: the copy
            // the app saw had its strings capped.
            Ok(Some(Outcome::Answers(answers))) => {
                original_input(&raw).and_then(|input| output::answers_json(&input, answers))
            }
            _ => None,
        };
        if let Some(json) = json {
            let mut out = std::io::stdout();
            let _ = writeln!(out, "{json}");
            let _ = out.flush();
        }
    }
    std::process::exit(0);
}

/// What the app said to an event that waited.
enum Outcome {
    Decision(protocol::Decision),
    Answers(Vec<protocol::Answer>),
}

struct Args {
    agent: AgentKind,
    /// Another tool's name, with [`AgentKind::Other`].
    agent_name: Option<String>,
    event: Option<String>,
    /// [`protocol::ASK_FLAG`]: this entry's timeout lets a question wait for the island.
    ask: bool,
}

impl Args {
    fn parse(mut args: impl Iterator<Item = String>) -> Self {
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
fn status_line_payload(payload: &Map<String, Value>) -> Option<Map<String, Value>> {
    let limits = payload.get("rate_limits").filter(|v| v.is_object())?;
    let mut kept = Map::new();
    kept.insert("rate_limits".into(), limits.clone());
    if let Some(id) = payload.get("session_id") {
        kept.insert("session_id".into(), id.clone());
    }
    Some(kept)
}

/// The event, and the hook JSON it came from.
fn read_event(args: &Args) -> Option<(Event, Vec<u8>)> {
    let mut raw = Vec::new();
    std::io::stdin().read_to_end(&mut raw).ok()?;
    let event = build_event(args, &raw, std::env::current_dir().ok(), |var| {
        std::env::var(var).ok()
    })?;
    Some((event, raw))
}

/// The tool's input exactly as the agent sent it.
fn original_input(raw: &[u8]) -> Option<Value> {
    let raw = raw.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(raw);
    let mut payload: Map<String, Value> = serde_json::from_slice(raw).ok()?;
    payload.remove("tool_input")
}

/// Pure part of [`read_event`], so it can be tested without a process.
fn build_event(
    args: &Args,
    raw: &[u8],
    cwd: Option<std::path::PathBuf>,
    env: impl Fn(&str) -> Option<String>,
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

    let cwd = payload
        .get("cwd")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .or_else(|| cwd.map(|p| p.to_string_lossy().into_owned()));
    let terminal = Terminal {
        cwd,
        pid: parent_pid(),
        env: TERMINAL_VARS
            .iter()
            .filter_map(|var| env(var).filter(|v| !v.is_empty()).map(|v| (var.to_string(), v)))
            .collect(),
    };

    // Only Claude Code and Codex take an answer from a hook, and only Claude Code asks questions;
    // any other tool's own terminal asks the user.
    let tool = payload.get("tool_name").and_then(Value::as_str);
    let wants_reply = match args.agent {
        AgentKind::Claude => protocol::wants_reply(&event, args.ask, tool),
        AgentKind::Codex => protocol::wants_reply(&event, false, tool),
        AgentKind::Gemini | AgentKind::Other => false,
    };

    Some(Event {
        v: protocol::VERSION,
        id: new_id(),
        agent: args.agent,
        agent_name: args.agent_name.clone(),
        wants_reply,
        event,
        terminal,
        payload,
    })
}

#[cfg(unix)]
fn parent_pid() -> Option<u32> {
    Some(vultures_ai_peer::parent_pid())
}

#[cfg(not(unix))]
fn parent_pid() -> Option<u32> {
    None
}

/// Unique per message: nanoseconds since the epoch plus our pid.
fn new_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{nanos:x}-{:x}", std::process::id())
}

/// Caps every string, cutting on a char boundary. A single `Write` can carry a whole file.
fn truncate_strings(value: &mut Value, max: usize) {
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
fn cap_patch(hunks: &[Value]) -> (Vec<Value>, bool) {
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

/// Connect, send, and, when the event wants a reply, wait for the app's decision.
fn talk(event: &Event) -> Option<Outcome> {
    let mut conn = connect()?;
    conn.write_all(&protocol::encode(event)).ok()?;
    conn.flush().ok()?;
    if !event.wants_reply {
        return None;
    }

    let mut line = Vec::new();
    let mut chunk = [0u8; 1024];
    while !line.contains(&b'\n') && line.len() <= protocol::MAX_MESSAGE {
        match conn.read(&mut chunk) {
            Ok(0) | Err(_) => break,
            Ok(n) => line.extend_from_slice(&chunk[..n]),
        }
    }
    let end = line.iter().position(|b| *b == b'\n')?;
    match serde_json::from_slice(&line[..end]).ok()? {
        Reply::Decision { v, id, decision, .. } if v == protocol::VERSION && id == event.id => {
            Some(Outcome::Decision(decision))
        }
        Reply::Answer { v, id, answers } if v == protocol::VERSION && id == event.id => {
            Some(Outcome::Answers(answers))
        }
        _ => None,
    }
}

/// A missing or refused socket means the app is closed: give up at once.
#[cfg(unix)]
fn connect() -> Option<std::os::unix::net::UnixStream> {
    let runtime = std::env::var_os("XDG_RUNTIME_DIR").map(std::path::PathBuf::from);
    let path = protocol::socket_path(
        runtime.as_deref(),
        &std::env::temp_dir(),
        vultures_ai_peer::current_uid(),
    );
    // A socket in a folder someone else controls could be anyone's.
    if !path.parent().is_some_and(vultures_ai_peer::is_private_dir) {
        return None;
    }
    let stream = std::os::unix::net::UnixStream::connect(path).ok()?;
    if !vultures_ai_peer::peer_is_same_user(&stream) {
        return None;
    }
    let _ = stream.set_write_timeout(Some(limits::CONNECT_TIMEOUT));
    Some(stream)
}

/// Retries only while every pipe instance is busy: the server exists and a slot will free up.
#[cfg(windows)]
fn connect() -> Option<std::fs::File> {
    const ERROR_PIPE_BUSY: i32 = 231;
    let name = protocol::pipe_name(&vultures_ai_peer::current_user_sid()?);
    let deadline = std::time::Instant::now() + limits::CONNECT_TIMEOUT;
    loop {
        match std::fs::OpenOptions::new().read(true).write(true).open(&name) {
            Ok(file) => return vultures_ai_peer::pipe_server_is_same_user(&file).then_some(file),
            Err(err)
                if err.raw_os_error() == Some(ERROR_PIPE_BUSY) && std::time::Instant::now() < deadline =>
            {
                std::thread::sleep(std::time::Duration::from_millis(15));
            }
            Err(_) => return None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn args(agent: AgentKind, event: Option<&str>) -> Args {
        Args {
            agent,
            agent_name: None,
            event: event.map(str::to_string),
            ask: false,
        }
    }

    #[test]
    fn a_status_line_forwards_only_the_usage() {
        let a = Args::parse(
            ["--agent", "claude", "--statusline"]
                .map(String::from)
                .into_iter(),
        );
        assert_eq!(a.event.as_deref(), Some(protocol::STATUS_LINE_EVENT));
        // Recorded from Claude Code 2.1.286: before the first reply, then after it.
        let mut lines = include_str!("../../agents/tests/fixtures/claude-statusline.jsonl").lines();
        let before = lines.next().unwrap().as_bytes();
        assert!(build_event(&a, before, None, |_| None).is_none());
        let raw = br#"{"session_id":"s1","cwd":"/home/me/secret","transcript_path":"/t.jsonl","cost":{"total_cost_usd":1.5},"rate_limits":{"five_hour":{"used_percentage":23,"resets_at":1790993400}}}"#;
        let e = build_event(&a, raw, None, |_| None).unwrap();
        assert_eq!(e.event, protocol::STATUS_LINE_EVENT);
        assert!(!e.wants_reply);
        assert_eq!(
            e.payload,
            serde_json::json!({ "session_id": "s1", "rate_limits": { "five_hour": { "used_percentage": 23, "resets_at": 1790993400 } } })
        );
        let after = lines.next().unwrap().as_bytes();
        assert!(build_event(&a, after, None, |_| None).is_some());
    }

    #[test]
    fn parses_agent_and_event() {
        let a = Args::parse(["--agent", "codex", "Stop"].map(String::from).into_iter());
        assert_eq!(a.agent, AgentKind::Codex);
        assert_eq!(a.event.as_deref(), Some("Stop"));
        // Any other tool goes by its own name.
        let a = Args::parse(["--agent", "my-tool"].map(String::from).into_iter());
        assert_eq!(a.agent, AgentKind::Other);
        assert_eq!(a.agent_name.as_deref(), Some("my-tool"));
        // A name it may not use is no agent at all: taken for Claude Code, a third-party tool
        // would get a card, wait for it and read Claude's answer JSON. Nothing is sent.
        let ask = br#"{"hook_event_name":"PermissionRequest","session_id":"s","tool_name":"Bash"}"#;
        for bad in ["My Tool", "my_tool", "other", "", &"x".repeat(40)] {
            let a = Args::parse(
                ["--agent", bad, "PermissionRequest"]
                    .map(String::from)
                    .into_iter(),
            );
            assert_eq!(
                (a.agent, a.agent_name.as_deref()),
                (AgentKind::Other, None),
                "{bad}"
            );
            assert!(build_event(&a, ask, None, |_| None).is_none(), "{bad}");
        }
        // No --agent at all is Claude Code, as installed before the flag existed.
        let a = Args::parse(["PermissionRequest"].map(String::from).into_iter());
        assert_eq!(a.agent, AgentKind::Claude);
        assert!(build_event(&a, ask, None, |_| None).unwrap().wants_reply);
    }

    #[test]
    fn a_status_line_stays_a_status_line_whatever_the_input_calls_itself() {
        let a = Args::parse(
            ["--agent", "claude", "--statusline"]
                .map(String::from)
                .into_iter(),
        );
        // An older Claude Code named its statusLine input "Status".
        let raw = br#"{"hook_event_name":"Status","cwd":"/home/me/secret","rate_limits":{"seven_day":{"used_percentage":12}}}"#;
        let e = build_event(&a, raw, None, |_| None).unwrap();
        assert_eq!(e.event, protocol::STATUS_LINE_EVENT);
        assert_eq!(
            e.payload,
            serde_json::json!({ "rate_limits": { "seven_day": { "used_percentage": 12 } } })
        );
    }

    #[test]
    fn another_tool_never_waits_for_an_answer() {
        let a = Args {
            agent: AgentKind::Other,
            agent_name: Some("my-tool".into()),
            event: None,
            ask: true,
        };
        let raw = br#"{"hook_event_name":"PermissionRequest","tool_name":"Bash"}"#;
        let e = build_event(&a, raw, None, |_| None).unwrap();
        assert!(!e.wants_reply);
        assert_eq!(e.agent_name.as_deref(), Some("my-tool"));
        // Gemini is built in, but its hooks can't approve either.
        let a = Args::parse(
            ["--agent", "gemini", "PermissionRequest"]
                .map(String::from)
                .into_iter(),
        );
        assert_eq!((a.agent, a.agent_name.as_deref()), (AgentKind::Gemini, None));
        assert!(!build_event(&a, raw, None, |_| None).unwrap().wants_reply);
    }

    #[test]
    fn builds_an_event_from_the_hook_json() {
        let raw = br#"{"hook_event_name":"PermissionRequest","tool_name":"Bash","tool_response":"huge","cwd":"/w"}"#;
        let env = |var: &str| (var == "TERM_PROGRAM").then(|| "kitty".to_string());
        let e = build_event(&args(AgentKind::Claude, None), raw, None, env).unwrap();
        assert_eq!(e.event, "PermissionRequest");
        assert!(e.wants_reply);
        assert_eq!(
            e.payload,
            json!({ "hook_event_name": "PermissionRequest", "tool_name": "Bash", "cwd": "/w" })
        );
        assert_eq!(e.terminal.cwd.as_deref(), Some("/w"));
        assert_eq!(
            e.terminal.env.get("TERM_PROGRAM").map(String::as_str),
            Some("kitty")
        );
        // A failed tool keeps its error, and only that.
        let raw = br#"{"hook_event_name":"AfterTool","tool_response":{"llmContent":"huge","error":{"message":"no such file"}}}"#;
        let e = build_event(&args(AgentKind::Gemini, None), raw, None, |_| None).unwrap();
        assert_eq!(
            e.payload["tool_response"],
            json!({ "error": { "message": "no such file" } })
        );
    }

    #[test]
    fn a_question_waits_only_from_the_ask_entry() {
        let raw = br#"{"hook_event_name":"PreToolUse","tool_name":"AskUserQuestion","tool_input":{"questions":[]}}"#;
        let ask = Args::parse(
            ["--agent", "claude", "--ask", "PreToolUse"]
                .map(String::from)
                .into_iter(),
        );
        assert!(ask.ask);
        assert!(build_event(&ask, raw, None, |_| None).unwrap().wants_reply);
        // An entry installed before the flag has a short timeout: it must not wait.
        assert!(
            !build_event(&args(AgentKind::Claude, None), raw, None, |_| None)
                .unwrap()
                .wants_reply
        );
        let other_tool = br#"{"hook_event_name":"PreToolUse","tool_name":"Bash"}"#;
        assert!(!build_event(&ask, other_tool, None, |_| None).unwrap().wants_reply);
        let codex = Args {
            agent: AgentKind::Codex,
            ..ask
        };
        assert!(!build_event(&codex, raw, None, |_| None).unwrap().wants_reply);
    }

    #[test]
    fn the_answers_go_back_in_the_uncut_input() {
        let long = "x".repeat(protocol::MAX_FIELD_LEN + 10);
        let raw = format!(
            r#"{{"hook_event_name":"PreToolUse","tool_name":"AskUserQuestion","tool_input":{{"questions":[{{"question":"{long}"}}]}}}}"#
        );
        let e = build_event(&args(AgentKind::Claude, None), raw.as_bytes(), None, |_| None).unwrap();
        assert_ne!(e.payload["tool_input"]["questions"][0]["question"], json!(long));
        assert_eq!(
            original_input(raw.as_bytes()).unwrap()["questions"][0]["question"],
            json!(long)
        );
    }

    #[test]
    fn falls_back_to_argv_and_process_cwd() {
        let raw = "\u{feff}{}".as_bytes();
        let e = build_event(
            &args(AgentKind::Codex, Some("Stop")),
            raw,
            Some("/p".into()),
            |_| None,
        )
        .unwrap();
        assert_eq!(e.event, "Stop");
        assert!(!e.wants_reply);
        assert_eq!(e.terminal.cwd.as_deref(), Some("/p"));
    }

    #[test]
    fn a_finished_edit_keeps_its_patch_and_nothing_else_of_the_output() {
        let raw = br#"{"hook_event_name":"PostToolUse","tool_name":"Edit","tool_input":{"file_path":"/w/a.rs"},
            "tool_response":{"originalFile":"huge","structuredPatch":[{"oldStart":1,"oldLines":1,"newStart":1,"newLines":1,"lines":["-a","+b"]}]}}"#;
        let e = build_event(&args(AgentKind::Claude, None), raw, None, |_| None).unwrap();
        assert_eq!(
            e.payload["tool_response"],
            json!({ "structuredPatch": [ { "oldStart": 1, "oldLines": 1, "newStart": 1, "newLines": 1, "lines": ["-a", "+b"] } ] })
        );
        // Only once the edit is done: before, it may still be refused.
        let raw =
            br#"{"hook_event_name":"PreToolUse","tool_name":"Edit","tool_response":{"structuredPatch":[]}}"#;
        let e = build_event(&args(AgentKind::Claude, None), raw, None, |_| None).unwrap();
        assert!(e.payload.get("tool_response").is_none());
    }

    #[test]
    fn a_long_patch_is_cut_to_its_first_lines() {
        let hunk = |n: usize| json!({ "oldStart": 1, "newStart": 1, "lines": vec!["+x"; n] });
        let (kept, cut) = cap_patch(&[hunk(300), hunk(300), hunk(5)]);
        assert!(cut);
        assert_eq!(kept.len(), 2);
        assert_eq!(kept[1]["lines"].as_array().unwrap().len(), MAX_PATCH_LINES - 300);
        let (kept, cut) = cap_patch(&[hunk(3)]);
        assert!(!cut);
        assert_eq!(kept, [hunk(3)]);
    }

    #[test]
    fn codex_keeps_more_of_a_finished_patch() {
        let patch = format!(
            "*** Begin Patch\n*** Add File: a\n{}*** End Patch",
            "+x\n".repeat(2_000)
        );
        let raw = |event: &str| {
            serde_json::to_vec(&json!({ "hook_event_name": event, "tool_name": "apply_patch", "tool_input": { "command": patch } }))
                .unwrap()
        };
        let e = build_event(&args(AgentKind::Codex, None), &raw("PostToolUse"), None, |_| None).unwrap();
        assert_eq!(e.payload["tool_input"]["command"], patch);
        // Before it runs, the approval card needs only its files.
        let e = build_event(&args(AgentKind::Codex, None), &raw("PreToolUse"), None, |_| None).unwrap();
        assert!(
            e.payload["tool_input"]["command"].as_str().unwrap().len()
                <= protocol::MAX_FIELD_LEN + '…'.len_utf8()
        );
    }

    #[test]
    fn no_event_name_or_bad_json_sends_nothing() {
        assert!(build_event(&args(AgentKind::Claude, None), b"{}", None, |_| None).is_none());
        assert!(build_event(&args(AgentKind::Claude, Some("Stop")), b"[]", None, |_| None).is_none());
    }

    #[test]
    fn long_strings_are_cut_on_a_char_boundary() {
        let mut v = json!({ "tool_input": { "content": "é".repeat(4000) } }); // check-english:allow (multi-byte test input)
        truncate_strings(&mut v, protocol::MAX_FIELD_LEN);
        let s = v["tool_input"]["content"].as_str().unwrap();
        assert!(s.len() <= protocol::MAX_FIELD_LEN + '…'.len_utf8());
        assert!(s.ends_with('…'));
    }
}
