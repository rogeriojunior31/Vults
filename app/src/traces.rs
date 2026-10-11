//! Traces of Zeca's work (docs/safety.md; `docs/dev/plan-zeca.md` S6): a span per chat turn and per
//! tool it asked to run, with OpenTelemetry's GenAI attribute names. Names, timings, tokens, cost
//! and a tool's target with secrets redacted: never a prompt or a reply. Kept 30 days.

use serde_json::{Map, Value, json};
use vults_chat::Usage;
use vults_store::Store;
use vults_store::traces::Span;

pub fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_millis()).unwrap_or(0))
}

/// A fresh random id for a trace or a span (W3C's 16 hex digits).
pub fn new_id() -> String {
    use std::hash::{BuildHasher, Hasher};
    // Each RandomState has its own random keys: no clock, so two ids made back to back differ.
    let mut h = std::hash::RandomState::new().build_hasher();
    h.write_u128(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos()),
    );
    format!("{:016x}", h.finish())
}

/// The OpenTelemetry name of a provider, where the conventions have one of their own.
fn provider_name(id: &str) -> &str {
    match id {
        "google" => "gcp.gemini",
        "mistral" => "mistral_ai",
        "xai" => "x_ai",
        other => other,
    }
}

/// A chat turn: who ran it, on which model, what it cost, how it ended.
#[derive(Debug)]
pub struct Turn {
    pub trace: String,
    pub span: String,
    pub start_ms: i64,
    pub provider: &'static str,
    pub model: Option<String>,
}

impl Turn {
    pub fn start(provider: &'static str, model: Option<String>) -> Self {
        Self {
            trace: new_id(),
            span: new_id(),
            start_ms: now_ms(),
            provider,
            model,
        }
    }

    /// Writes the turn's span; `status` is `ok`, `error` or `stopped`.
    pub fn end(&self, usage: Option<&Usage>, status: &str) {
        let span = chat_span(self, usage, status, now_ms());
        if let Some(u) = usage {
            tracing::info!(
                provider = self.provider,
                input = u.input,
                output = u.output,
                cache_read = u.cache_read,
                cache_write = u.cache_write,
                cost_usd = ?u.cost_usd,
                "chat turn"
            );
        }
        crate::writer::send(crate::writer::Job::Span(span));
    }
}

fn chat_span(turn: &Turn, usage: Option<&Usage>, status: &str, end_ms: i64) -> Span {
    let answered = usage.and_then(|u| u.model.clone());
    let model = answered.clone().or_else(|| turn.model.clone());
    let mut attrs = Map::new();
    attrs.insert("gen_ai.operation.name".into(), json!("chat"));
    attrs.insert("gen_ai.provider.name".into(), json!(provider_name(turn.provider)));
    if let Some(m) = &turn.model {
        attrs.insert("gen_ai.request.model".into(), json!(m));
    }
    if let Some(m) = &answered {
        attrs.insert("gen_ai.response.model".into(), json!(m));
    }
    if let Some(u) = usage {
        attrs.insert("gen_ai.usage.input_tokens".into(), json!(u.input));
        attrs.insert("gen_ai.usage.output_tokens".into(), json!(u.output));
        attrs.insert("gen_ai.usage.cache_read.input_tokens".into(), json!(u.cache_read));
        attrs.insert(
            "gen_ai.usage.cache_creation.input_tokens".into(),
            json!(u.cache_write),
        );
        if let Some(cost) = u.cost_usd {
            attrs.insert("vults.cost_usd".into(), json!(cost));
        }
    }
    Span {
        trace: turn.trace.clone(),
        span: turn.span.clone(),
        parent: None,
        name: format!("chat {}", model.as_deref().unwrap_or(turn.provider)),
        kind: "chat".into(),
        start_ms: turn.start_ms,
        end_ms,
        status: status.into(),
        attrs: Value::Object(attrs),
    }
}

/// A tool the chat asked to run, from its card to the answer (not the tool's own run, which the CLI
/// keeps to itself): `status` is `ok` (allowed), `denied`, or `stopped` (closed unanswered).
pub fn tool(parent: &Turn, tool: &str, target: &str, since_ms: i64, status: &str) {
    let span = tool_span(parent, tool, target, since_ms, now_ms(), status);
    crate::writer::send(crate::writer::Job::Span(span));
}

fn tool_span(parent: &Turn, tool: &str, target: &str, start_ms: i64, end_ms: i64, status: &str) -> Span {
    Span {
        trace: parent.trace.clone(),
        span: new_id(),
        parent: Some(parent.span.clone()),
        name: format!("execute_tool {tool}"),
        kind: "execute_tool".into(),
        start_ms,
        end_ms,
        status: status.into(),
        attrs: json!({
            "gen_ai.operation.name": "execute_tool",
            "gen_ai.tool.name": tool,
            "vults.target": vults_core::redact::secrets(target),
            "vults.allowed": status == "ok",
        }),
    }
}

/// Drops the spans past 30 days.
pub fn prune() -> Result<(), vults_store::Error> {
    Store::open(&crate::history::dir())?.prune_traces(now_ms())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_turn_span_names_its_model_and_says_its_tokens_and_cost() {
        let turn = Turn {
            trace: "t".into(),
            span: "s".into(),
            start_ms: 1000,
            provider: "claude-code",
            model: None,
        };
        let usage = Usage {
            model: Some("claude-sonnet-5-5".into()),
            input: 12,
            output: 45,
            cache_read: 9000,
            cache_write: 3000,
            cost_usd: Some(0.0123),
            session_cost_usd: None,
        };
        let s = chat_span(&turn, Some(&usage), "ok", 2500);
        assert_eq!(s.name, "chat claude-sonnet-5-5");
        assert_eq!(s.attrs["gen_ai.response.model"], "claude-sonnet-5-5");
        assert!(s.attrs.get("gen_ai.request.model").is_none(), "the CLI chose it");
        assert_eq!(s.attrs["gen_ai.provider.name"], "claude-code");
        assert_eq!(s.attrs["gen_ai.usage.input_tokens"], 12);
        assert_eq!(s.attrs["gen_ai.usage.cache_read.input_tokens"], 9000);
        assert_eq!(s.attrs["vults.cost_usd"], 0.0123);
        assert_eq!((s.start_ms, s.end_ms), (1000, 2500));
    }

    #[test]
    fn a_tool_span_hangs_from_its_turn_with_secrets_redacted() {
        let turn = Turn::start("codex", None);
        let s = tool_span(&turn, "Bash", "API_TOKEN=abc ls", 10, 20, "denied");
        assert_eq!(s.parent.as_deref(), Some(turn.span.as_str()));
        assert_eq!(s.trace, turn.trace);
        assert_eq!(s.status, "denied");
        assert_eq!(s.attrs["vults.target"], "API_TOKEN=[redacted] ls");
    }

    #[test]
    fn ids_are_random_and_never_repeat_back_to_back() {
        let a = new_id();
        let b = new_id();
        assert_ne!(a, b);
        assert_eq!(a.len(), 16);
        let turn = Turn::start("google", Some("gemini-x".into()));
        assert_ne!(turn.trace, turn.span);
        let s = chat_span(&turn, None, "error", turn.start_ms);
        assert_eq!(s.attrs["gen_ai.provider.name"], "gcp.gemini");
        assert_eq!(s.attrs["gen_ai.request.model"], "gemini-x");
    }
}
