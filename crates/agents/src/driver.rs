//! How the app runs an agent itself (`docs/dev/plan-zeca.md` S8), whatever the agent: start it,
//! send it a turn, interrupt it. Today the chat's Claude Code and Codex; later the sessions Zeca
//! starts for a task (W4) and ACP agents (W10). No runtime here (this crate is in the Core layer):
//! futures are boxed, and whoever runs a driver brings the runtime and a [`Sink`].

use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;

use serde::Serialize;
use serde_json::Value;

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// One turn for an agent: the user's words and the files they dropped (already in our inbox).
#[derive(Debug, Clone, Default)]
pub struct Input {
    pub text: String,
    pub files: Vec<PathBuf>,
}

/// What a turn cost, as the agent reported it: a field it did not report stays 0 (or none).
#[derive(Serialize, Debug, Clone, PartialEq, Default)]
pub struct Usage {
    pub model: Option<String>,
    /// Every input token, the cached ones included (OpenTelemetry's `gen_ai.usage.input_tokens`).
    pub input: u64,
    pub output: u64,
    /// Input tokens read from the provider's prompt cache.
    pub cache_read: u64,
    /// Input tokens written to it.
    pub cache_write: u64,
    /// Only when the agent says (Claude Code does); never guessed from a price list.
    pub cost_usd: Option<f64>,
}

impl Usage {
    /// Anthropic's `usage` object (the Messages API and Claude Code's `result`). Its `input_tokens`
    /// leaves the cached ones out: they are added back, as every other engine counts them.
    pub fn anthropic(v: &Value) -> Self {
        let n = |k: &str| v[k].as_u64().unwrap_or(0);
        let (read, write) = (n("cache_read_input_tokens"), n("cache_creation_input_tokens"));
        Self {
            input: n("input_tokens") + read + write,
            output: n("output_tokens"),
            cache_read: read,
            cache_write: write,
            ..Self::default()
        }
    }

    /// What `self` has beyond `earlier` (tokens never go below zero).
    pub fn minus(&self, earlier: &Self) -> Self {
        Self {
            model: self.model.clone(),
            input: self.input.saturating_sub(earlier.input),
            output: self.output.saturating_sub(earlier.output),
            cache_read: self.cache_read.saturating_sub(earlier.cache_read),
            cache_write: self.cache_write.saturating_sub(earlier.cache_write),
            cost_usd: self.cost_usd,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.input == 0 && self.output == 0 && self.cost_usd.is_none()
    }
}

/// What an agent says while a turn runs.
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    /// More reply text.
    Text(String),
    /// What the turn cost; before the turn ends.
    Usage(Usage),
}

/// A tool the agent wants to run: the card's tool and target as the agent names them, and the
/// call's input as it sent it (for the card's detail).
#[derive(Debug, Clone, PartialEq)]
pub struct Ask {
    pub id: String,
    pub tool: String,
    pub target: String,
    pub input: Value,
}

/// Where a driver reports, and asks. Whoever runs the driver implements it.
pub trait Sink: Send + Sync {
    fn event(&self, event: Event) -> BoxFuture<'_, ()>;
    /// Resolves `true` only from a human's answer (rule 2); silence or a dropped card is a no.
    fn ask(&self, ask: Ask) -> BoxFuture<'_, bool>;
}

/// An agent the app runs. A driver keeps what lets the next turn go on (a session id, a live
/// process); `send` is one turn, ended when it resolves.
pub trait AgentDriver: Send + std::fmt::Debug {
    fn name(&self) -> &'static str;
    /// Gets ready for turns (a long-lived process starts here); a no-op for an agent that runs one
    /// process per turn.
    fn start(&mut self) -> BoxFuture<'_, Result<(), String>>;
    /// One turn: what the agent says goes to `sink`, which also answers its tool requests.
    fn send<'a>(&'a mut self, input: &'a Input, sink: &'a dyn Sink) -> BoxFuture<'a, Result<(), String>>;
    /// Stops what the agent is doing and leaves the driver usable: the next turn resumes the same
    /// conversation.
    fn interrupt(&mut self) -> BoxFuture<'_, Result<(), String>>;
    /// The agent's id for the conversation, once it has one.
    fn session(&self) -> Option<&str>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usage_from_anthropic_counts_the_cache_in() {
        let u = Usage::anthropic(&serde_json::json!({
            "input_tokens": 12, "cache_read_input_tokens": 9000,
            "cache_creation_input_tokens": 3000, "output_tokens": 45
        }));
        assert_eq!(
            (u.input, u.output, u.cache_read, u.cache_write),
            (12_012, 45, 9000, 3000)
        );
        assert_eq!(
            u.minus(&Usage {
                input: 12,
                ..Usage::default()
            })
            .input,
            12_000
        );
    }
}
