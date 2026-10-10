//! What each agent expects on stdout for a decision. Anything unknown prints nothing:
//! silence makes the agent ask in the terminal, which is always safe.

use serde_json::{Map, Value, json};
use vults_protocol::{AgentKind, Answer, Decision};

/// How an agent takes an answer from its hook. An agent without one is never made to wait: its
/// own terminal asks the user.
#[derive(Debug)]
pub struct Replies {
    /// Answers its `AskUserQuestion` through `PreToolUse` (the `--ask` entry).
    pub questions: bool,
    /// What it reads on stdout for a decision.
    pub decision: fn(Decision) -> String,
    /// What it reads on stdout for the answers to its questions, from the tool's own input;
    /// `None` when they don't fit it.
    pub answers: fn(&Value, Vec<Answer>) -> Option<String>,
}

/// The one place that says which agents we answer, and how. The match is exhaustive: a new
/// [`AgentKind`] decides here, before anything waits for it.
pub fn replies(agent: AgentKind) -> Option<&'static Replies> {
    // Both agents read the same PermissionRequest output:
    // https://code.claude.com/docs/en/hooks and https://learn.chatgpt.com/docs/hooks
    static CLAUDE: Replies = Replies {
        questions: true,
        decision: permission_request,
        answers: answers_json,
    };
    static CODEX: Replies = Replies {
        questions: false,
        decision: permission_request,
        answers: |_, _| None,
    };
    static OPENCODE: Replies = Replies {
        questions: true,
        decision: opencode_reply,
        answers: opencode_answers,
    };
    match agent {
        AgentKind::Claude => Some(&CLAUDE),
        // Qwen Code, Claude Code's fork, reads the same output; its questions take no answer
        // from a hook.
        AgentKind::Codex | AgentKind::Qwen => Some(&CODEX),
        AgentKind::OpenCode => Some(&OPENCODE),
        // Their hooks can't take an answer.
        AgentKind::Gemini | AgentKind::Other => None,
    }
}

/// Whether the hook waits for the app on this event.
pub fn waits(agent: AgentKind, event: &str, ask: bool, tool: Option<&str>) -> bool {
    replies(agent).is_some_and(|r| vults_protocol::wants_reply(event, ask && r.questions, tool))
}

pub fn decision_json(agent: AgentKind, decision: Decision) -> Option<String> {
    replies(agent).map(|r| (r.decision)(decision))
}

pub fn answers_output(agent: AgentKind, tool_input: &Value, answers: Vec<Answer>) -> Option<String> {
    replies(agent).and_then(|r| (r.answers)(tool_input, answers))
}

fn permission_request(decision: Decision) -> String {
    let behavior = match decision {
        Decision::Allow => r#"{"behavior":"allow"}"#.to_string(),
        Decision::Deny => format!(
            r#"{{"behavior":"deny","message":"Denied from {}"}}"#,
            vults_brand::NAME
        ),
    };
    format!(r#"{{"hookSpecificOutput":{{"hookEventName":"PermissionRequest","decision":{behavior}}}}}"#)
}

/// What our OpenCode plugin reads: the `response` it hands to OpenCode's permission endpoint.
/// `once`, never `always`: an *Always* is our rule, and OpenCode's would change its own config.
fn opencode_reply(decision: Decision) -> String {
    match decision {
        Decision::Allow => r#"{"reply":"once"}"#.to_string(),
        Decision::Deny => r#"{"reply":"reject"}"#.to_string(),
    }
}

/// What our OpenCode plugin hands to OpenCode's question endpoint: one list of labels (or the
/// user's own words) per question, in order. One answer per question, or nothing.
fn opencode_answers(tool_input: &Value, answers: Vec<Answer>) -> Option<String> {
    let questions = tool_input.get("questions")?.as_array()?;
    if questions.is_empty() || questions.len() != answers.len() {
        return None;
    }
    let lists: Vec<Vec<String>> = answers
        .into_iter()
        .map(|a| match a {
            Answer::One(label) => vec![label],
            Answer::Many(labels) => labels,
        })
        .collect();
    Some(json!({ "answers": lists }).to_string())
}

/// Claude Code's `PreToolUse` output that answers an `AskUserQuestion`: the tool runs with the
/// replies added to its input, keyed by each question's text. One answer per question, or nothing.
fn answers_json(tool_input: &Value, answers: Vec<Answer>) -> Option<String> {
    let questions = tool_input.get("questions")?.as_array()?;
    if questions.is_empty() || questions.len() != answers.len() {
        return None;
    }
    let mut keyed = Map::new();
    for (question, answer) in questions.iter().zip(answers) {
        let text = question.get("question")?.as_str()?;
        let value = match answer {
            Answer::One(label) => Value::String(label),
            Answer::Many(labels) => json!(labels),
        };
        keyed.insert(text.to_string(), value);
    }
    let mut input = tool_input.as_object()?.clone();
    input.insert("answers".into(), Value::Object(keyed));
    let out = json!({
        "hookSpecificOutput": {
            "hookEventName": "PreToolUse",
            "permissionDecision": "allow",
            "updatedInput": input,
        }
    });
    Some(out.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn answers_are_keyed_by_the_questions_text() {
        // The shape Claude Code 2.1.286 accepted in a real session.
        let input = json!({ "questions": [
            { "question": "Which color?", "header": "Color", "multiSelect": false, "options": [ { "label": "Red" }, { "label": "Blue" } ] },
            { "question": "Which sizes?", "header": "Sizes", "multiSelect": true, "options": [ { "label": "S" }, { "label": "M" } ] },
        ] });
        let out = answers_json(
            &input,
            vec![
                Answer::One("Blue".into()),
                Answer::Many(vec!["S".into(), "M".into()]),
            ],
        )
        .unwrap();
        let out: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(out["hookSpecificOutput"]["hookEventName"], "PreToolUse");
        assert_eq!(out["hookSpecificOutput"]["permissionDecision"], "allow");
        let updated = &out["hookSpecificOutput"]["updatedInput"];
        assert_eq!(updated["questions"], input["questions"]);
        assert_eq!(
            updated["answers"],
            json!({ "Which color?": "Blue", "Which sizes?": ["S", "M"] })
        );
    }

    #[test]
    fn a_wrong_count_of_answers_prints_nothing() {
        let input = json!({ "questions": [ { "question": "A?" }, { "question": "B?" } ] });
        assert_eq!(answers_json(&input, vec![Answer::One("x".into())]), None);
        assert_eq!(answers_json(&json!({ "questions": [] }), vec![]), None);
        assert_eq!(answers_json(&json!({}), vec![Answer::One("x".into())]), None);
    }

    #[test]
    fn claude_shape() {
        assert_eq!(
            decision_json(AgentKind::Claude, Decision::Allow).unwrap(),
            r#"{"hookSpecificOutput":{"hookEventName":"PermissionRequest","decision":{"behavior":"allow"}}}"#
        );
        assert_eq!(
            decision_json(AgentKind::Claude, Decision::Deny).unwrap(),
            r#"{"hookSpecificOutput":{"hookEventName":"PermissionRequest","decision":{"behavior":"deny","message":"Denied from Vults"}}}"#
        );
    }

    const KINDS: [AgentKind; 6] = [
        AgentKind::Claude,
        AgentKind::Codex,
        AgentKind::Gemini,
        AgentKind::OpenCode,
        AgentKind::Qwen,
        AgentKind::Other,
    ];

    #[test]
    fn qwen_gets_claude_codes_decision() {
        for decision in [Decision::Allow, Decision::Deny] {
            assert_eq!(
                decision_json(AgentKind::Qwen, decision),
                decision_json(AgentKind::Claude, decision)
            );
        }
    }

    #[test]
    fn only_an_agent_with_replies_ever_waits_or_gets_a_decision() {
        for agent in KINDS {
            let answered = replies(agent).is_some();
            assert_eq!(
                waits(agent, "PermissionRequest", false, None),
                answered,
                "{agent:?}"
            );
            assert_eq!(
                decision_json(agent, Decision::Allow).is_some(),
                answered,
                "{agent:?}"
            );
            assert_eq!(
                decision_json(agent, Decision::Deny).is_some(),
                answered,
                "{agent:?}"
            );
            assert!(!waits(agent, "PreToolUse", false, Some("Bash")), "{agent:?}");
        }
    }

    #[test]
    fn only_claude_and_opencode_wait_on_a_question_and_only_from_the_ask_entry() {
        for agent in KINDS {
            let q = Some(vults_protocol::QUESTION_TOOL);
            let asks = matches!(agent, AgentKind::Claude | AgentKind::OpenCode);
            assert_eq!(waits(agent, "PreToolUse", true, q), asks, "{agent:?}");
            assert!(!waits(agent, "PreToolUse", false, q), "{agent:?}");
        }
    }

    #[test]
    fn opencode_gets_a_list_of_labels_per_question() {
        let input =
            json!({ "questions": [ { "question": "Which color?" }, { "question": "Which sizes?" } ] });
        let out = answers_output(
            AgentKind::OpenCode,
            &input,
            vec![
                Answer::One("Blue".into()),
                Answer::Many(vec!["S".into(), "M".into()]),
            ],
        );
        assert_eq!(out.as_deref(), Some(r#"{"answers":[["Blue"],["S","M"]]}"#));
        assert_eq!(
            answers_output(AgentKind::OpenCode, &input, vec![Answer::One("x".into())]),
            None
        );
        assert_eq!(
            answers_output(AgentKind::Codex, &input, vec![Answer::One("x".into())]),
            None
        );
    }

    #[test]
    fn opencode_gets_once_or_reject() {
        assert_eq!(
            decision_json(AgentKind::OpenCode, Decision::Allow).unwrap(),
            r#"{"reply":"once"}"#
        );
        assert_eq!(
            decision_json(AgentKind::OpenCode, Decision::Deny).unwrap(),
            r#"{"reply":"reject"}"#
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
