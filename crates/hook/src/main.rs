//! `vults-hook [--agent claude|codex|gemini|opencode|qwen|<tool>] [--ask] [EventName]`: the relay an agent runs on every hook event.
//!
//! Reads the hook JSON on stdin, wraps it in a protocol [`Event`] and hands it to the app.
//! Hard rule: **never block the agent.** Every failure (app closed, socket wedged, garbage
//! reply, unknown version) ends in exit 0 with an empty stdout, and the agent carries on as
//! if we were not installed. The deadlines are enforced by the main thread, so a peer that
//! accepts and then stops reading cannot hold us either.
//!
//! `--statusline` also runs the user's own Claude Code status line, if the installer saved one,
//! and prints only what that prints ([`chain`]).

mod chain;

use std::io::{Read, Write};
use std::sync::mpsc;
use std::time::{SystemTime, UNIX_EPOCH};

use vults_hook::{Args, original_input, output};
use vults_protocol::{self as protocol, Event, Reply, limits};

fn main() {
    // A panic anywhere, on any thread, is one more failure: exit 0 with nothing printed (rule 1),
    // never a non-zero status or a backtrace the agent might show. Release builds abort on panic;
    // the hook runs before the abort.
    std::panic::set_hook(Box::new(|_| std::process::exit(0)));
    // Debug builds only: the round-trip test forces a panic to check the line above.
    #[cfg(debug_assertions)]
    if std::env::var_os("VULTS_HOOK_TEST_PANIC").is_some() {
        panic!("forced by VULTS_HOOK_TEST_PANIC");
    }
    let args = Args::parse(std::env::args().skip(1));
    let mut raw = Vec::new();
    let read = std::io::stdin().read_to_end(&mut raw).is_ok();
    // The user's own status line starts first and runs while we relay the usage.
    let previous = (args.event.as_deref() == Some(protocol::STATUS_LINE_EVENT))
        .then(chain::previous_file)
        .flatten()
        .and_then(|file| chain::start(&file, &raw));
    let event = read
        .then(|| {
            build_event(&args, &raw, std::env::current_dir().ok(), |var| {
                std::env::var(var).ok()
            })
        })
        .flatten();
    let mut out = std::io::stdout();
    if let Some(json) = event.and_then(|event| relay(event, &raw)) {
        let _ = writeln!(out, "{json}");
    }
    if let Some(previous) = previous {
        let _ = out.write_all(&previous.finish(chain::TIMEOUT));
    }
    let _ = out.flush();
    std::process::exit(0);
}

/// Hands the event to the app; what to print when it answers.
fn relay(event: Event, raw: &[u8]) -> Option<String> {
    let budget = if event.wants_reply {
        limits::DECISION_BUDGET
    } else {
        limits::FIRE_AND_FORGET_BUDGET
    };
    let agent = event.agent;

    // The worker owns every blocking call; if it overruns we stop listening and exit,
    // and the process dying takes the connection with it.
    let wants_reply = event.wants_reply;
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(Step::Done(talk(&event, &tx)));
    });
    let start = std::time::Instant::now();
    // A frozen app still accepts the connection (the kernel does): until it says a human is
    // being asked, only a short wait.
    let first = if wants_reply {
        limits::WAITING_TIMEOUT.min(budget)
    } else {
        budget
    };
    let outcome = match rx.recv_timeout(first) {
        Ok(Step::Waiting) => match rx.recv_timeout(budget.saturating_sub(start.elapsed())) {
            Ok(Step::Done(outcome)) => outcome,
            _ => None,
        },
        Ok(Step::Done(outcome)) => outcome,
        Err(_) => None,
    };
    match outcome {
        Some(Outcome::Decision(decision)) => output::decision_json(agent, decision),
        // The answers go back inside the tool's own input, as the agent sent it: the copy
        // the app saw had its strings capped.
        Some(Outcome::Answers(answers)) => {
            original_input(raw).and_then(|input| output::answers_output(agent, &input, answers))
        }
        _ => None,
    }
}

/// What the worker tells `relay` as the connection goes.
enum Step {
    /// [`Reply::Waiting`]: the card is up, a human may take a while.
    Waiting,
    Done(Option<Outcome>),
}

/// What the app said to an event that waited.
enum Outcome {
    Decision(protocol::Decision),
    Answers(Vec<protocol::Answer>),
}

/// [`vults_hook::build_event`] with this process's parent and a fresh id.
fn build_event(
    args: &Args,
    raw: &[u8],
    cwd: Option<std::path::PathBuf>,
    env: impl Fn(&str) -> Option<String>,
) -> Option<Event> {
    vults_hook::build_event(args, raw, cwd, env, parent_pid(), new_id())
}

#[cfg(unix)]
fn parent_pid() -> Option<u32> {
    Some(vults_peer::parent_pid())
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

/// Connect, send, and, when the event wants a reply, wait for the app's decision; a
/// [`Reply::Waiting`] on the way goes to `steps`.
fn talk(event: &Event, steps: &mpsc::Sender<Step>) -> Option<Outcome> {
    let mut conn = connect()?;
    conn.write_all(&protocol::encode(event)).ok()?;
    conn.flush().ok()?;
    if !event.wants_reply {
        return None;
    }

    let mut line = Vec::new();
    let mut chunk = [0u8; 1024];
    loop {
        while !line.contains(&b'\n') && line.len() <= protocol::MAX_MESSAGE {
            match conn.read(&mut chunk) {
                Ok(0) | Err(_) => return None,
                Ok(n) => line.extend_from_slice(&chunk[..n]),
            }
        }
        let end = line.iter().position(|b| *b == b'\n')?;
        let reply = serde_json::from_slice(&line[..end]).ok()?;
        line.drain(..=end);
        match reply {
            Reply::Waiting { v, id } if v == protocol::VERSION && id == event.id => {
                let _ = steps.send(Step::Waiting);
            }
            Reply::Decision { v, id, decision, .. } if v == protocol::VERSION && id == event.id => {
                return Some(Outcome::Decision(decision));
            }
            Reply::Answer { v, id, answers } if v == protocol::VERSION && id == event.id => {
                return Some(Outcome::Answers(answers));
            }
            _ => return None,
        }
    }
}

/// A missing or refused socket means the app is closed: give up at once.
#[cfg(unix)]
fn connect() -> Option<std::os::unix::net::UnixStream> {
    let runtime = std::env::var_os("XDG_RUNTIME_DIR").map(std::path::PathBuf::from);
    let path = protocol::socket_path(
        runtime.as_deref(),
        &std::env::temp_dir(),
        vults_peer::current_uid(),
    );
    // A socket in a folder someone else controls could be anyone's.
    if !path.parent().is_some_and(vults_peer::is_private_dir) {
        return None;
    }
    let stream = std::os::unix::net::UnixStream::connect(path).ok()?;
    if !vults_peer::peer_is_same_user(&stream) {
        return None;
    }
    let _ = stream.set_write_timeout(Some(limits::CONNECT_TIMEOUT));
    Some(stream)
}

/// Retries only while every pipe instance is busy: the server exists and a slot will free up.
#[cfg(windows)]
fn connect() -> Option<std::fs::File> {
    const ERROR_PIPE_BUSY: i32 = 231;
    let name = protocol::pipe_name(&vults_peer::current_user_sid()?);
    let deadline = std::time::Instant::now() + limits::CONNECT_TIMEOUT;
    loop {
        match std::fs::OpenOptions::new().read(true).write(true).open(&name) {
            Ok(file) => return vults_peer::pipe_server_is_same_user(&file).then_some(file),
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
    use vults_hook::{MAX_PATCH_LINES, cap_patch, truncate_strings};
    use vults_protocol::AgentKind;

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

    /// Antigravity: the event only on the command line, the workspace instead of a cwd, and the
    /// hook run from the hooks file's folder.
    #[test]
    fn antigravity_names_its_workspace() {
        let a = Args::parse(
            ["--agent", "antigravity", "PreToolUse"]
                .map(String::from)
                .into_iter(),
        );
        let raw = br#"{"conversationId":"c1","workspacePaths":["/home/me/proj"],"transcriptPath":"/t","artifactDirectoryPath":"/a",
            "toolCall":{"name":"run_command","args":{"CommandLine":"echo hi"}}}"#;
        let e = build_event(&a, raw, Some("/home/me/.gemini/config".into()), |_| None).unwrap();
        assert_eq!(
            (e.agent, e.agent_name.as_deref()),
            (AgentKind::Other, Some("antigravity"))
        );
        assert_eq!(e.event, "PreToolUse");
        assert!(!e.wants_reply);
        assert_eq!(e.terminal.cwd.as_deref(), Some("/home/me/proj"));
        assert_eq!(e.payload.get("transcriptPath"), None);
        assert_eq!(e.payload.get("artifactDirectoryPath"), None);
        assert_eq!(e.payload["toolCall"]["args"]["CommandLine"], "echo hi");
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
