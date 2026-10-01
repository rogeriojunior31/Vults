//! What each agent expects on stdout for a decision. Anything unknown prints nothing:
//! silence makes the agent ask in the terminal, which is always safe.

use vultures_ai_protocol::{AgentKind, Decision};

pub fn decision_json(agent: AgentKind, decision: Decision) -> Option<String> {
    match agent {
        // Both agents read the same PermissionRequest output:
        // https://code.claude.com/docs/en/hooks and https://learn.chatgpt.com/docs/hooks
        AgentKind::Claude | AgentKind::Codex => Some(permission_request(decision)),
    }
}

fn permission_request(decision: Decision) -> String {
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
    fn codex_reads_the_same_shape() {
        assert_eq!(
            decision_json(AgentKind::Codex, Decision::Deny),
            decision_json(AgentKind::Claude, Decision::Deny)
        );
    }
}
