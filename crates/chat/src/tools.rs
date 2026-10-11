//! Tools for the API chat's loop (`docs/dev/plan-zeca.md` Z6): the model calls one, the harness
//! runs it and hands the result back, up to [`MAX_STEPS`] times a turn. A tool here only reads:
//! anything with an effect goes through `core::policy` and the ledger in `app`, never through this
//! loop. Z7's web tools and Z9's flock tools implement [`Tool`].

use std::sync::Arc;

use serde_json::Value;
use vults_agents::driver::BoxFuture;

/// Model calls in one turn: past this the turn ends, saying so.
pub const MAX_STEPS: usize = 8;
/// A result longer than this is cut, and says it was.
pub const MAX_RESULT: usize = 16 * 1024;
/// The history may fill at most this share of the model's window; past it, old tool results are
/// cleared, then the oldest exchanges go.
pub const WINDOW_SHARE: usize = 2;

/// One tool the model may call.
pub trait Tool: Send + Sync + std::fmt::Debug {
    fn name(&self) -> &str;
    /// For the model: what it does and when to use it.
    fn description(&self) -> &str;
    /// The input's JSON Schema.
    fn schema(&self) -> Value;
    fn call<'a>(&'a self, input: &'a Value) -> BoxFuture<'a, Result<String, String>>;
}

/// The tools a conversation offers.
pub type Tools = Vec<Arc<dyn Tool>>;

/// Runs the named tool; an unknown name or a failure comes back as an error result for the model,
/// never as the turn's end.
pub(crate) async fn run(tools: &Tools, name: &str, input: &Value) -> (String, bool) {
    let Some(tool) = tools.iter().find(|t| t.name() == name) else {
        return (format!("There is no tool named {name}."), true);
    };
    match tool.call(input).await {
        Ok(text) => (cut(text), false),
        Err(e) => (cut(e), true),
    }
}

fn cut(mut text: String) -> String {
    if text.len() > MAX_RESULT {
        let mut end = MAX_RESULT;
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        text.truncate(end);
        text.push_str("\n[cut: the result was longer]");
    }
    text
}

/// What stands in for a result cleared to make room.
pub(crate) const CLEARED: &str = "[cleared to make room]";

/// A rough token count: a quarter of the text, and a flat cost for an image or a document (their
/// base64 would count many times what the model is charged).
pub(crate) fn tokens(value: &Value) -> usize {
    match value {
        Value::String(s) => s.len() / 4 + 1,
        Value::Array(items) => items.iter().map(tokens).sum(),
        Value::Object(map) => {
            let kind = map.get("type").and_then(Value::as_str).unwrap_or_default();
            if matches!(kind, "image" | "document" | "image_url") {
                return 1_600;
            }
            map.values().map(tokens).sum::<usize>() + map.len()
        }
        _ => 1,
    }
}

/// Keeps `messages` under `window / WINDOW_SHARE` tokens: the oldest tool results are cleared
/// first (Anthropic's `tool_result` blocks, OpenAI's `tool` messages), then the oldest exchanges
/// go, a whole user turn at a time, so the history still starts with a user message. The newest
/// exchange always stays.
pub(crate) fn fit(messages: &mut Vec<Value>, window: usize) {
    let budget = window / WINDOW_SHARE;
    let over = |m: &[Value]| m.iter().map(tokens).sum::<usize>() > budget;
    if !over(messages) {
        return;
    }
    // Clearing a result leaves the call and its pairing intact: the wire stays valid.
    let last = messages.len().saturating_sub(1);
    for i in 0..last {
        if !over(messages) {
            return;
        }
        let m = &mut messages[i];
        if m["role"] == "tool" {
            m["content"] = Value::String(CLEARED.into());
        } else if let Some(blocks) = m["content"].as_array_mut() {
            for b in blocks.iter_mut().filter(|b| b["type"] == "tool_result") {
                b["content"] = Value::String(CLEARED.into());
            }
        }
    }
    while over(messages) {
        // The next user message that starts a turn (not one carrying tool results).
        let next = messages
            .iter()
            .enumerate()
            .skip(1)
            .find(|(_, m)| m["role"] == "user" && !carries_results(m))
            .map(|(i, _)| i);
        match next {
            Some(i) if i < messages.len() => {
                messages.drain(..i);
            }
            _ => return,
        }
    }
}

fn carries_results(m: &Value) -> bool {
    m["content"]
        .as_array()
        .is_some_and(|b| b.iter().any(|b| b["type"] == "tool_result"))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use serde_json::json;

    /// A tool that echoes its input's `text`, for the loop's tests.
    #[derive(Debug)]
    pub(crate) struct Echo;

    impl Tool for Echo {
        fn name(&self) -> &str {
            "echo"
        }
        fn description(&self) -> &str {
            "Says the text back."
        }
        fn schema(&self) -> Value {
            json!({ "type": "object", "properties": { "text": { "type": "string" } }, "required": ["text"] })
        }
        fn call<'a>(&'a self, input: &'a Value) -> BoxFuture<'a, Result<String, String>> {
            Box::pin(async move {
                input["text"]
                    .as_str()
                    .map(|t| format!("echo: {t}"))
                    .ok_or_else(|| "no text".to_string())
            })
        }
    }

    #[tokio::test]
    async fn a_tool_runs_and_a_failure_is_a_result() {
        let tools: Tools = vec![Arc::new(Echo)];
        assert_eq!(
            run(&tools, "echo", &json!({ "text": "hi" })).await,
            ("echo: hi".into(), false)
        );
        assert_eq!(run(&tools, "echo", &json!({})).await, ("no text".into(), true));
        assert!(run(&tools, "rm", &json!({})).await.1);
        let long = cut("x".repeat(MAX_RESULT + 10));
        assert!(long.len() < MAX_RESULT + 64 && long.ends_with("longer]"));
    }

    #[test]
    fn the_history_stays_under_half_the_window() {
        let big = "word ".repeat(4_000);
        let mut m = Vec::new();
        for i in 0..6 {
            m.push(json!({ "role": "user", "content": format!("question {i}") }));
            m.push(json!({ "role": "assistant", "content": [{ "type": "tool_use", "id": format!("t{i}"), "name": "echo", "input": {} }] }));
            m.push(json!({ "role": "user", "content": [{ "type": "tool_result", "tool_use_id": format!("t{i}"), "content": big }] }));
            m.push(json!({ "role": "assistant", "content": format!("answer {i}") }));
        }
        m.push(json!({ "role": "user", "content": "last question" }));
        let window = 20_000;
        fit(&mut m, window);
        assert!(m.iter().map(tokens).sum::<usize>() <= window / WINDOW_SHARE);
        // Old results go first; the conversation itself stays.
        assert_eq!(m[0]["content"], "question 0");
        assert_eq!(m[2]["content"][0]["content"], CLEARED);
        assert_eq!(m.last().unwrap()["content"], "last question");
        // Still too long once every result is gone: the oldest turns go, whole.
        fit(&mut m, 400);
        assert_eq!(m[0]["role"], "user");
        assert!(!carries_results(&m[0]));
        assert_eq!(m.last().unwrap()["content"], "last question");
    }

    #[test]
    fn an_image_counts_flat() {
        let image = json!({ "type": "image", "source": { "data": "A".repeat(4_000_000) } });
        assert_eq!(tokens(&image), 1_600);
    }
}
