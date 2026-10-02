//! Any other tool, named with `--agent <name>`. It sends Claude Code's hook JSON, the de facto
//! format, so it is read the same way. Two differences: the session id is prefixed with the
//! name (two tools may number sessions alike), and a permission is shown as a question, since
//! nothing here answers it: the tool's own terminal asks the user.

use vultures_ai_core::{AgentEvent, AgentUpdate};
use vultures_ai_protocol::{AgentKind, Event, valid_agent_name};

use crate::{Agent, Claude};

pub(crate) fn parse(e: &Event) -> Option<AgentUpdate> {
    let name = e.agent_name.as_deref().filter(|n| valid_agent_name(n))?;
    let mut update = Claude.parse(e)?;
    update.session.agent = AgentKind::Other;
    update.session.session_id = format!("{name}/{}", update.session.session_id);
    if let AgentEvent::PermissionRequested { target, .. } = update.event {
        update.event = AgentEvent::Question { message: target };
    }
    Some(update)
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use vultures_ai_core::AgentEvent;
    use vultures_ai_protocol::{AgentKind, Event, Terminal};

    fn event(name: Option<&str>, kind: &str, payload: serde_json::Value) -> Event {
        Event {
            v: vultures_ai_protocol::VERSION,
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

    #[test]
    fn no_valid_name_no_session() {
        let payload = json!({ "session_id": "s1" });
        assert!(super::parse(&event(None, "SessionStart", payload.clone())).is_none());
        assert!(super::parse(&event(Some("claude"), "SessionStart", payload)).is_none());
    }
}
