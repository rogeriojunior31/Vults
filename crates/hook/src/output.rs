//! What each agent expects on stdout for a decision. Anything unknown prints nothing:
//! silence makes the agent ask in the terminal, which is always safe.

use serde_json::{Map, Value, json};
use vults_protocol::{AgentKind, Answer, Decision, Then};

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
    // Copilot CLI reads the same `behavior` bare, checked with Copilot CLI 1.0.96; it has no
    // question tool.
    static COPILOT: Replies = Replies {
        questions: false,
        decision: behavior,
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
        AgentKind::Copilot => Some(&COPILOT),
        // Their hooks can't take an answer.
        AgentKind::Other => None,
    }
}

/// Whether the hook waits for the app on this event.
pub fn waits(agent: AgentKind, event: &str, ask: bool, tool: Option<&str>) -> bool {
    replies(agent).is_some_and(|r| vults_protocol::wants_reply(event, ask && r.questions, tool))
}

pub fn decision_json(agent: AgentKind, decision: Decision) -> Option<String> {
    replies(agent).map(|r| (r.decision)(decision))
}

/// A click's answer as the app sent it.
#[derive(Debug)]
pub struct Click {
    pub decision: Decision,
    pub then: Option<Then>,
    /// The user's words for the agent with a no (a plan's "what to change").
    pub reason: Option<String>,
}

/// [`decision_json`], with the tool's input sent back where the agent needs it: Claude Code
/// ignores a bare allow of `ExitPlanMode` ("allow alone is not sufficient for these tools",
/// https://code.claude.com/docs/en/hooks), and then stays in plan mode.
pub fn decision_for(
    agent: AgentKind,
    tool: Option<&str>,
    tool_input: Option<Value>,
    Click {
        decision,
        then,
        reason,
    }: Click,
) -> Option<String> {
    // A plan sent back with what to change: Claude Code gives the message to the model, which
    // stays in plan mode and plans again.
    if let Some(message) = reason
        .filter(|_| agent == AgentKind::Claude && tool == Some("ExitPlanMode") && decision == Decision::Deny)
    {
        let deny = json!({"behavior": "deny", "message": message});
        return Some(
            json!({"hookSpecificOutput":{"hookEventName":"PermissionRequest","decision":deny}}).to_string(),
        );
    }
    let echoed = agent == AgentKind::Claude && tool == Some("ExitPlanMode") && decision == Decision::Allow;
    match tool_input.filter(|_| echoed) {
        Some(input) => {
            let mut allow = json!({"behavior": "allow", "updatedInput": input});
            // The plan's own choice in Claude Code's terminal: this session's edits go ahead
            // without asking (https://code.claude.com/docs/en/hooks, `updatedPermissions`).
            if then == Some(Then::AcceptEdits) {
                allow["updatedPermissions"] =
                    json!([{"type": "setMode", "mode": "acceptEdits", "destination": "session"}]);
            }
            Some(
                json!({"hookSpecificOutput":{"hookEventName":"PermissionRequest","decision":allow}})
                    .to_string(),
            )
        }
        None => decision_json(agent, decision),
    }
}

/// What Antigravity reads from its PreToolUse hook. Only a click gets here: no answer prints
/// nothing, and agy asks in its own prompt.
pub fn antigravity_decision(decision: Decision) -> String {
    match decision {
        Decision::Allow => json!({"decision": "allow"}).to_string(),
        Decision::Deny => {
            json!({"decision": "deny", "reason": format!("Denied from {}", vults_brand::NAME)}).to_string()
        }
    }
}

pub fn answers_output(agent: AgentKind, tool_input: &Value, answers: Vec<Answer>) -> Option<String> {
    replies(agent).and_then(|r| (r.answers)(tool_input, answers))
}

fn behavior(decision: Decision) -> String {
    match decision {
        Decision::Allow => r#"{"behavior":"allow"}"#.to_string(),
        Decision::Deny => format!(
            r#"{{"behavior":"deny","message":"Denied from {}"}}"#,
            vults_brand::NAME
        ),
    }
}

fn permission_request(decision: Decision) -> String {
    let behavior = behavior(decision);
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

    fn a(decision: Decision, then: Option<Then>) -> Click {
        Click {
            decision,
            then,
            reason: None,
        }
    }

    #[test]
    fn a_plan_sent_back_carries_the_users_words_and_nothing_else_does() {
        let plan = json!({ "plan": "1. Read" });
        let revise = |agent, tool, decision| {
            let answer = Click {
                decision,
                then: None,
                reason: Some("Keep the old API.".into()),
            };
            decision_for(agent, tool, Some(plan.clone()), answer)
        };
        let out: Value =
            serde_json::from_str(&revise(AgentKind::Claude, Some("ExitPlanMode"), Decision::Deny).unwrap())
                .unwrap();
        assert_eq!(
            out["hookSpecificOutput"]["decision"],
            json!({"behavior": "deny", "message": "Keep the old API."})
        );
        // Another tool, another agent or an allow: the words never travel.
        assert_eq!(
            revise(AgentKind::Claude, Some("Bash"), Decision::Deny),
            decision_json(AgentKind::Claude, Decision::Deny)
        );
        assert_eq!(
            revise(AgentKind::Codex, Some("ExitPlanMode"), Decision::Deny),
            decision_json(AgentKind::Codex, Decision::Deny)
        );
        assert!(
            !revise(AgentKind::Claude, Some("ExitPlanMode"), Decision::Allow)
                .unwrap()
                .contains("old API")
        );
    }

    #[test]
    fn an_allowed_plan_goes_back_with_its_input_and_nothing_else_does() {
        let plan = json!({ "plan": "1. Read\n2. Write", "planFilePath": "/tmp/plan.md" });
        let allow =
            |tool, decision| decision_for(AgentKind::Claude, tool, Some(plan.clone()), a(decision, None));
        let out: Value =
            serde_json::from_str(&allow(Some("ExitPlanMode"), Decision::Allow).unwrap()).unwrap();
        assert_eq!(out["hookSpecificOutput"]["decision"]["behavior"], "allow");
        assert_eq!(out["hookSpecificOutput"]["decision"]["updatedInput"], plan);
        // A deny, another tool or another agent keeps the plain answer: no input is rewritten.
        assert_eq!(
            allow(Some("ExitPlanMode"), Decision::Deny),
            decision_json(AgentKind::Claude, Decision::Deny)
        );
        assert_eq!(
            allow(Some("Bash"), Decision::Allow),
            decision_json(AgentKind::Claude, Decision::Allow)
        );
        assert_eq!(
            decision_for(
                AgentKind::Codex,
                Some("ExitPlanMode"),
                Some(plan.clone()),
                a(Decision::Allow, None)
            ),
            decision_json(AgentKind::Codex, Decision::Allow)
        );
        // Allowed accepting edits: the session's mode goes with the plan, and only with a plan.
        let accepting = decision_for(
            AgentKind::Claude,
            Some("ExitPlanMode"),
            Some(plan.clone()),
            a(Decision::Allow, Some(Then::AcceptEdits)),
        )
        .unwrap();
        let accepting: Value = serde_json::from_str(&accepting).unwrap();
        assert_eq!(
            accepting["hookSpecificOutput"]["decision"]["updatedPermissions"],
            json!([{"type": "setMode", "mode": "acceptEdits", "destination": "session"}])
        );
        assert_eq!(accepting["hookSpecificOutput"]["decision"]["updatedInput"], plan);
        assert_eq!(
            decision_for(
                AgentKind::Claude,
                Some("Bash"),
                Some(plan.clone()),
                a(Decision::Allow, Some(Then::AcceptEdits))
            ),
            decision_json(AgentKind::Claude, Decision::Allow)
        );
        // No input to send back: the plain allow, as before.
        assert_eq!(
            decision_for(
                AgentKind::Claude,
                Some("ExitPlanMode"),
                None,
                a(Decision::Allow, None)
            ),
            decision_json(AgentKind::Claude, Decision::Allow)
        );
    }

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
        AgentKind::OpenCode,
        AgentKind::Qwen,
        AgentKind::Copilot,
        AgentKind::Other,
    ];

    #[test]
    fn copilot_gets_the_bare_behavior() {
        assert_eq!(
            decision_json(AgentKind::Copilot, Decision::Allow).unwrap(),
            r#"{"behavior":"allow"}"#
        );
        assert_eq!(
            decision_json(AgentKind::Copilot, Decision::Deny).unwrap(),
            r#"{"behavior":"deny","message":"Denied from Vults"}"#
        );
        // Its permissions wait for a card; nothing else does.
        assert!(waits(
            AgentKind::Copilot,
            "PermissionRequest",
            false,
            Some("bash")
        ));
        assert!(!waits(AgentKind::Copilot, "PreToolUse", true, Some("bash")));
    }

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
