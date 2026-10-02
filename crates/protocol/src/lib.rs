//! Wire protocol between `vultures-ai-hook` and the app: one JSON object per line.
//! Spec: docs/reference/protocol.md. Any change to a message bumps [`VERSION`].

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const VERSION: u32 = 2;

/// Largest line either side accepts, newline included.
pub const MAX_MESSAGE: usize = 1 << 20;
/// Longest string the hook forwards in a payload; the island shows far less.
pub const MAX_FIELD_LEN: usize = 2_000;

/// Deadlines. The hook's are hard limits so an agent is never blocked by us.
pub mod limits {
    use super::Duration;
    /// Hook: time to reach the app. Beyond it the agent carries on alone.
    pub const CONNECT_TIMEOUT: Duration = Duration::from_millis(300);
    /// Hook: whole run for an event nobody answers.
    pub const FIRE_AND_FORGET_BUDGET: Duration = Duration::from_secs(2);
    /// Hook: how long a permission card may wait for a human.
    pub const DECISION_BUDGET: Duration = Duration::from_secs(110);
    /// App: slightly under the hook's budget, so the app always answers first.
    pub const SERVER_DECISION_TIMEOUT: Duration = Duration::from_secs(108);
    /// App: time for the UI to confirm the card is on screen before a human is awaited.
    pub const ACK_TIMEOUT: Duration = Duration::from_millis(800);
    /// App: time to read one message from a connection.
    pub const READ_TIMEOUT: Duration = Duration::from_secs(5);
    /// App: simultaneous hook connections.
    pub const MAX_CONNECTIONS: usize = 32;
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "lowercase")]
pub enum AgentKind {
    Claude,
    Codex,
    /// Any other tool that sends Claude Code-style hook JSON, named by [`Event::agent_name`].
    Other,
}

impl AgentKind {
    /// A built-in agent; `other` is not one (a tool calling itself that names nothing).
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "claude" => Some(Self::Claude),
            "codex" => Some(Self::Codex),
            _ => None,
        }
    }
}

/// A name another tool may go by: short, lowercase, and never a built-in agent's, so nothing
/// can pass itself off as Claude Code or Codex.
pub fn valid_agent_name(name: &str) -> bool {
    (1..=24).contains(&name.len())
        && name
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        && !matches!(name, "claude" | "codex" | "other")
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Decision {
    Allow,
    Deny,
}

/// Where the agent runs, so the app can jump back to it. Context only, never a filter.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
pub struct Terminal {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    /// The agent process (the hook's parent), when the OS tells us.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pid: Option<u32>,
    /// Terminal-identifying environment variables that were set (`TERM_PROGRAM`, `WT_SESSION`…).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub env: BTreeMap<String, String>,
}

/// hook → app.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "kind", rename = "event")]
pub struct Event {
    pub v: u32,
    pub id: String,
    pub agent: AgentKind,
    /// Only with [`AgentKind::Other`]: the tool's name, checked by [`valid_agent_name`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_name: Option<String>,
    /// The agent's own event name, e.g. `PreToolUse`.
    pub event: String,
    /// The hook waits on this connection for a [`Reply`].
    pub wants_reply: bool,
    #[serde(default)]
    pub terminal: Terminal,
    /// The agent's hook JSON, with large fields dropped and strings capped.
    pub payload: Value,
}

/// app → hook, only on connections whose event `wants_reply`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Reply {
    Decision {
        v: u32,
        id: String,
        decision: Decision,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        reason: Option<String>,
    },
    /// The app does not speak the event's version: the hook stays silent.
    Unsupported { v: u32, id: String },
}

/// Events that hold the connection open for a human. Everything else is fire and forget.
pub fn wants_reply(event: &str) -> bool {
    event == "PermissionRequest"
}

#[derive(Debug, PartialEq, Eq)]
pub enum DecodeError {
    /// Not a protocol message; drop the connection without a word.
    Malformed,
    /// A message from another protocol version; answer [`Reply::Unsupported`].
    Unsupported { id: String },
}

/// Decodes one hook → app line (without the newline).
pub fn decode_event(line: &[u8]) -> Result<Event, DecodeError> {
    let value: Value = serde_json::from_slice(line).map_err(|_| DecodeError::Malformed)?;
    let v = value.get("v").and_then(Value::as_u64);
    if v != Some(u64::from(VERSION)) {
        let id = value
            .get("id")
            .and_then(Value::as_str)
            .ok_or(DecodeError::Malformed)?;
        return Err(DecodeError::Unsupported { id: id.to_string() });
    }
    serde_json::from_value(value).map_err(|_| DecodeError::Malformed)
}

/// Serializes a message as one line, newline included.
pub fn encode<T: Serialize>(msg: &T) -> Vec<u8> {
    // Our own types always serialize; an empty line is dropped by the other side anyway.
    let mut line = serde_json::to_vec(msg).unwrap_or_default();
    line.push(b'\n');
    line
}

/// `$XDG_RUNTIME_DIR/vultures-ai.sock`, or a per-user folder in `tmp` without a runtime dir.
/// Both sides must call this with the same inputs.
pub fn socket_path(runtime_dir: Option<&Path>, tmp: &Path, uid: u32) -> PathBuf {
    let file = format!("{}.sock", vultures_ai_brand::SLUG);
    match runtime_dir {
        Some(dir) => dir.join(file),
        None => tmp.join(format!("{}-{uid}", vultures_ai_brand::SLUG)).join(file),
    }
}

/// `\\.\pipe\vultures-ai-<SID>`: the SID keeps two accounts apart in the machine-wide namespace.
pub fn pipe_name(sid: &str) -> String {
    format!(r"\\.\pipe\{}-{sid}", vultures_ai_brand::SLUG)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn event() -> Event {
        Event {
            v: VERSION,
            id: "abc".into(),
            agent: AgentKind::Codex,
            agent_name: None,
            event: "PermissionRequest".into(),
            wants_reply: true,
            terminal: Terminal {
                cwd: Some("/w".into()),
                pid: Some(7),
                env: BTreeMap::new(),
            },
            payload: json!({ "tool_name": "Bash" }),
        }
    }

    #[test]
    fn event_wire_shape() {
        let wire: Value = serde_json::from_slice(&encode(&event())).unwrap();
        assert_eq!(
            wire,
            json!({ "kind": "event", "v": 2, "id": "abc", "agent": "codex", "event": "PermissionRequest",
                    "wants_reply": true, "terminal": { "cwd": "/w", "pid": 7 },
                    "payload": { "tool_name": "Bash" } })
        );
    }

    #[test]
    fn event_round_trips() {
        let line = encode(&event());
        assert_eq!(decode_event(&line[..line.len() - 1]), Ok(event()));
    }

    #[test]
    fn another_tool_carries_its_name() {
        let e = Event {
            agent: AgentKind::Other,
            agent_name: Some("my-tool".into()),
            ..event()
        };
        let wire: Value = serde_json::from_slice(&encode(&e)).unwrap();
        assert_eq!(wire["agent"], "other");
        assert_eq!(wire["agent_name"], "my-tool");
        let line = encode(&e);
        assert_eq!(decode_event(&line[..line.len() - 1]), Ok(e));
    }

    #[test]
    fn agent_names() {
        for ok in ["my-tool", "aider", "x", "gemini2", "a-b-c"] {
            assert!(valid_agent_name(ok), "{ok}");
        }
        let long = "a".repeat(25);
        for bad in [
            "",
            "claude",
            "codex",
            "other",
            "My-Tool",
            "my tool",
            "my_tool",
            "caf\u{e9}",
            long.as_str(),
        ] {
            assert!(!valid_agent_name(bad), "{bad}");
        }
    }

    #[test]
    fn reply_wire_shape() {
        let r = Reply::Decision {
            v: 1,
            id: "abc".into(),
            decision: Decision::Deny,
            reason: None,
        };
        assert_eq!(
            serde_json::to_value(&r).unwrap(),
            json!({ "kind": "decision", "v": 1, "id": "abc", "decision": "deny" })
        );
        let u = Reply::Unsupported {
            v: 1,
            id: "abc".into(),
        };
        assert_eq!(
            serde_json::to_value(&u).unwrap(),
            json!({ "kind": "unsupported", "v": 1, "id": "abc" })
        );
    }

    #[test]
    fn other_versions_are_unsupported_not_malformed() {
        let line = br#"{"kind":"event","v":1,"id":"x","something":"old"}"#;
        assert_eq!(
            decode_event(line),
            Err(DecodeError::Unsupported { id: "x".into() })
        );
    }

    #[test]
    fn garbage_is_malformed() {
        assert_eq!(decode_event(b"not json"), Err(DecodeError::Malformed));
        assert_eq!(decode_event(br#"{"v":2,"id":"x"}"#), Err(DecodeError::Malformed));
        assert_eq!(decode_event(br#"{"v":3}"#), Err(DecodeError::Malformed));
    }

    #[test]
    fn endpoints() {
        assert_eq!(
            socket_path(Some(Path::new("/run/user/1000")), Path::new("/tmp"), 1000),
            PathBuf::from("/run/user/1000/vultures-ai.sock")
        );
        assert_eq!(
            socket_path(None, Path::new("/tmp"), 1000),
            PathBuf::from("/tmp/vultures-ai-1000/vultures-ai.sock")
        );
        assert_eq!(pipe_name("S-1-5-21-1"), r"\\.\pipe\vultures-ai-S-1-5-21-1");
    }
}
