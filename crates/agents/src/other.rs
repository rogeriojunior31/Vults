//! Any other tool, named with `--agent <name>`. It sends Claude Code's hook JSON, the de facto
//! format, so it is read the same way. Two differences: the session id is prefixed with the
//! name (two tools may number sessions alike), and a permission is shown as a question, since
//! nothing here answers it: the tool's own terminal asks the user.
//!
//! Tools that copied the format loosely are met halfway: Cursor's camelCase event names
//! (`preToolUse`, `stop`), `conversation_id` for the session, `workspace_roots` for the folder,
//! `error_message` for a failure.

use serde_json::Value;
use vults_core::{AgentEvent, AgentUpdate};
use vults_protocol::{AgentKind, Event, valid_agent_name};

use crate::{Agent, Claude};

/// Session ids under the names tools give them, Claude Code's first.
const SESSION_FIELDS: &[&str] = &["session_id", "conversation_id", "sessionId", "conversationId"];

pub(crate) fn parse(e: &Event) -> Option<AgentUpdate> {
    let name = e.agent_name.as_deref().filter(|n| valid_agent_name(n))?;
    // A tool whose hooks have their own shape.
    if name == crate::antigravity::NAME {
        return crate::Antigravity.parse(e);
    }
    let mut update = Claude.parse(&normalized(e)?)?;
    update.session.agent = AgentKind::Other;
    update.session.session_id = format!("{name}/{}", update.session.session_id);
    if let AgentEvent::PermissionRequested { target, .. } = update.event {
        update.event = AgentEvent::Question { message: target };
    }
    Some(update)
}

/// The event in Claude Code's words; `None` without a session id, or every session of the tool
/// would land on the same bird.
pub(crate) fn normalized(e: &Event) -> Option<Event> {
    let p = &e.payload;
    let text = |k: &str| p.get(k).and_then(Value::as_str).filter(|s| !s.is_empty());
    let session = SESSION_FIELDS.iter().find_map(|k| text(k))?.to_string();
    let event = match e.event.as_str() {
        "sessionStart" => "SessionStart",
        "sessionEnd" => "SessionEnd",
        "beforeSubmitPrompt" => "UserPromptSubmit",
        "preToolUse" => "PreToolUse",
        "postToolUse" => "PostToolUse",
        "postToolUseFailure" => "PostToolUseFailure",
        "subagentStart" => "SubagentStart",
        "subagentStop" => "SubagentStop",
        "stop" if text("status") == Some("error") => "StopFailure",
        "stop" => "Stop",
        other => other,
    }
    .to_string();
    let mut e = e.clone();
    e.event = event;
    let payload = e.payload.as_object_mut()?;
    payload.insert("session_id".into(), session.into());
    if !payload.contains_key("error")
        && let Some(error) = payload.get("error_message").cloned()
    {
        payload.insert("error".into(), error);
    }
    if !payload.contains_key("cwd")
        && let Some(root) = payload.get("workspace_roots").and_then(|r| r.get(0)).cloned()
    {
        payload.insert("cwd".into(), root);
    }
    Some(e)
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use vults_core::AgentEvent;
    use vults_protocol::{AgentKind, Event, Terminal};

    fn event(name: Option<&str>, kind: &str, payload: serde_json::Value) -> Event {
        Event {
            v: vults_protocol::VERSION,
            id: "r1".into(),
            agent: AgentKind::Other,
            agent_name: name.map(str::to_string),
            event: kind.into(),
            wants_reply: false,
            terminal: Terminal::default(),
            payload,
        }
    }

    #[test]
    fn sessions_go_by_the_tool_name() {
        let u = super::parse(&event(
            Some("my-tool"),
            "SessionStart",
            json!({ "session_id": "s1" }),
        ))
        .unwrap();
        assert_eq!(u.session.agent, AgentKind::Other);
        assert_eq!(u.session.session_id, "my-tool/s1");
    }

    #[test]
    fn a_permission_is_a_question_for_the_terminal() {
        let payload = json!({ "session_id": "s1", "tool_name": "Bash", "tool_input": { "command": "make" } });
        let u = super::parse(&event(Some("my-tool"), "PermissionRequest", payload)).unwrap();
        assert!(
            matches!(u.event, AgentEvent::Question { ref message } if message.contains("make")),
            "{:?}",
            u.event
        );
    }

    /// Cursor's hook JSON, as its docs describe it (not recorded from a real Cursor).
    #[test]
    fn cursor_names_are_read_too() {
        let u = super::parse(&event(
            Some("cursor"),
            "preToolUse",
            json!({ "conversation_id": "c1", "workspace_roots": ["/w"], "tool_name": "Shell",
                    "tool_input": { "command": "make", "working_directory": "/w" } }),
        ))
        .unwrap();
        assert_eq!(u.session.session_id, "cursor/c1");
        assert_eq!(u.cwd.as_deref(), Some("/w"));
        assert!(matches!(u.event, AgentEvent::ToolStarted(ref s) if s.detail.as_deref() == Some("make")));
        let failed = super::parse(&event(
            Some("cursor"),
            "stop",
            json!({ "conversation_id": "c1", "status": "error", "error_message": "quota" }),
        ))
        .unwrap();
        assert_eq!(
            failed.event,
            AgentEvent::StopFailed {
                error: Some("quota".into())
            }
        );
        let done = super::parse(&event(
            Some("cursor"),
            "stop",
            json!({ "conversation_id": "c1", "status": "completed" }),
        ));
        assert!(matches!(done.unwrap().event, AgentEvent::Stopped { .. }));
    }

    #[test]
    fn no_valid_name_or_session_id_no_session() {
        let payload = json!({ "session_id": "s1" });
        assert!(super::parse(&event(None, "SessionStart", payload.clone())).is_none());
        assert!(super::parse(&event(Some("claude"), "SessionStart", payload)).is_none());
        let nameless = json!({ "cwd": "/p" });
        assert!(super::parse(&event(Some("my-tool"), "SessionStart", nameless)).is_none());
    }
}
