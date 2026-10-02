//! `vultures-ai-hook [--agent claude|codex] [EventName]`: the relay an agent runs on every hook event.
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
    if let Some(event) = read_event(&args) {
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
        if let Ok(Some(decision)) = rx.recv_timeout(budget)
            && let Some(json) = output::decision_json(agent, decision)
        {
            let mut out = std::io::stdout();
            let _ = writeln!(out, "{json}");
            let _ = out.flush();
        }
    }
    std::process::exit(0);
}

struct Args {
    agent: AgentKind,
    /// Another tool's name, with [`AgentKind::Other`].
    agent_name: Option<String>,
    event: Option<String>,
}

impl Args {
    fn parse(mut args: impl Iterator<Item = String>) -> Self {
        let mut parsed = Args {
            agent: AgentKind::Claude,
            agent_name: None,
            event: None,
        };
        while let Some(arg) = args.next() {
            if arg == "--agent" {
                let Some(name) = args.next() else { continue };
                if let Some(agent) = AgentKind::parse(&name) {
                    parsed.agent = agent;
                } else if protocol::valid_agent_name(&name) {
                    parsed.agent = AgentKind::Other;
                    parsed.agent_name = Some(name);
                }
            } else {
                parsed.event = Some(arg);
            }
        }
        parsed
    }
}

fn read_event(args: &Args) -> Option<Event> {
    let mut raw = Vec::new();
    std::io::stdin().read_to_end(&mut raw).ok()?;
    build_event(args, &raw, std::env::current_dir().ok(), |var| {
        std::env::var(var).ok()
    })
}

/// Pure part of [`read_event`], so it can be tested without a process.
fn build_event(
    args: &Args,
    raw: &[u8],
    cwd: Option<std::path::PathBuf>,
    env: impl Fn(&str) -> Option<String>,
) -> Option<Event> {
    // Some shells hand us a UTF-8 BOM, which serde_json rejects.
    let raw = raw.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(raw);
    let mut payload: Map<String, Value> = serde_json::from_slice(raw).ok()?;

    // The JSON usually names the event; argv only fills the gap.
    let event = payload
        .get("hook_event_name")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .or_else(|| args.event.clone())?;

    // A tool's error is kept, never its output: Gemini says a tool failed only in there.
    let error = payload
        .get("tool_response")
        .and_then(|r| r.get("error"))
        .filter(|e| !e.is_null())
        .cloned();
    for field in DROPPED_FIELDS {
        payload.remove(*field);
    }
    if let Some(error) = error {
        payload.insert("tool_response".into(), serde_json::json!({ "error": error }));
    }
    let mut payload = Value::Object(payload);
    truncate_strings(&mut payload);

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

    Some(Event {
        v: protocol::VERSION,
        id: new_id(),
        agent: args.agent,
        agent_name: args.agent_name.clone(),
        // Only Claude Code and Codex take an answer from a hook; any other tool's own terminal
        // asks the user.
        wants_reply: protocol::wants_reply(&event)
            && matches!(args.agent, AgentKind::Claude | AgentKind::Codex),
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
fn truncate_strings(value: &mut Value) {
    match value {
        Value::String(s) if s.len() > protocol::MAX_FIELD_LEN => {
            let mut end = protocol::MAX_FIELD_LEN;
            while !s.is_char_boundary(end) {
                end -= 1;
            }
            s.truncate(end);
            s.push('…');
        }
        Value::Array(items) => items.iter_mut().for_each(truncate_strings),
        Value::Object(map) => map.values_mut().for_each(truncate_strings),
        _ => {}
    }
}

/// Connect, send, and, when the event wants a reply, wait for the app's decision.
fn talk(event: &Event) -> Option<protocol::Decision> {
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
        Reply::Decision { v, id, decision, .. } if v == protocol::VERSION && id == event.id => Some(decision),
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
        }
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
        // A name it may not use keeps the default instead of failing the agent's hook.
        for bad in ["My Tool", "other", ""] {
            let a = Args::parse(["--agent", bad].map(String::from).into_iter());
            assert_eq!((a.agent, a.agent_name), (AgentKind::Claude, None), "{bad}");
        }
        let a = Args::parse(["--agent"].map(String::from).into_iter());
        assert_eq!(a.agent, AgentKind::Claude);
    }

    #[test]
    fn another_tool_never_waits_for_an_answer() {
        let a = Args {
            agent: AgentKind::Other,
            agent_name: Some("my-tool".into()),
            event: None,
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
    fn no_event_name_or_bad_json_sends_nothing() {
        assert!(build_event(&args(AgentKind::Claude, None), b"{}", None, |_| None).is_none());
        assert!(build_event(&args(AgentKind::Claude, Some("Stop")), b"[]", None, |_| None).is_none());
    }

    #[test]
    fn long_strings_are_cut_on_a_char_boundary() {
        let mut v = json!({ "tool_input": { "content": "é".repeat(4000) } }); // check-english:allow (multi-byte test input)
        truncate_strings(&mut v);
        let s = v["tool_input"]["content"].as_str().unwrap();
        assert!(s.len() <= protocol::MAX_FIELD_LEN + '…'.len_utf8());
        assert!(s.ends_with('…'));
    }
}
