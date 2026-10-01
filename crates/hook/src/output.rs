//! What each agent expects on stdout for a decision. Anything unknown prints nothing:
//! silence makes the agent ask in the terminal, which is always safe.

use vultures_ai_protocol::{AgentKind, Decision};

pub fn decision_json(agent: AgentKind, decision: Decision) -> Option<String> {
    match agent {
        AgentKind::Claude => Some(claude(decision)),
        // The Codex hook reply format is not confirmed yet (plan M3): stay silent.
        AgentKind::Codex => None,
    }
}

/// PermissionRequest output, https://code.claude.com/docs/en/hooks
fn claude(decision: Decision) -> String {
    let behavior = match decision {
        Decision::Allow => r#"{"behavior":"allow"}"#.to_string(),
        Decision::Deny => format!(
            r#"{{"behavior":"deny","message":"Denied from {}"}}"#,
            vultures_ai_brand::NAME
        ),
    };
    format!(r#"{{"hookSpecificOutput":{{"hookEventName":"PermissionRequest","decision":{behavior}}}}}"#)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn claude_shape() {
        assert_eq!(
            decision_json(AgentKind::Claude, Decision::Allow).unwrap(),
            r#"{"hookSpecificOutput":{"hookEventName":"PermissionRequest","decision":{"behavior":"allow"}}}"#
        );
        assert_eq!(
            decision_json(AgentKind::Claude, Decision::Deny).unwrap(),
            r#"{"hookSpecificOutput":{"hookEventName":"PermissionRequest","decision":{"behavior":"deny","message":"Denied from Vultures AI"}}}"#
        );
    }

    #[test]
    fn codex_stays_silent_until_its_format_is_confirmed() {
        assert!(decision_json(AgentKind::Codex, Decision::Allow).is_none());
    }
}
