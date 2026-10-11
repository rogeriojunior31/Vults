//! How the app runs an agent itself (`docs/dev/plan-zeca.md` S8, S11), whatever the agent: what
//! to run it with (a [`Spec`]), a turn, and a way to interrupt it while the turn runs. Today the
//! chat's Claude Code and Codex; later the sessions Zeca starts for a task (W4) and ACP agents
//! (W10). No runtime here (this crate is in the Core layer): futures are boxed, and whoever runs a
//! driver brings the runtime and a [`Sink`].

use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;

use serde::Serialize;
use serde_json::Value;

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// One turn for an agent: the user's words and the files they dropped (already in our inbox).
#[derive(Debug, Clone, Default)]
pub struct Input {
    pub text: String,
    pub files: Vec<PathBuf>,
    /// What the app tells the model before the words, each turn (M2's `<today>`, `<flock>`,
    /// `<memories>`): never shown as the user's.
    pub context: Option<String>,
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
    /// Two calls of one turn together (a tool loop's steps); the model is the later one's.
    pub fn plus(&self, other: &Usage) -> Usage {
        Usage {
            model: other.model.clone().or_else(|| self.model.clone()),
            input: self.input + other.input,
            output: self.output + other.output,
            cache_read: self.cache_read + other.cache_read,
            cache_write: self.cache_write + other.cache_write,
            cost_usd: match (self.cost_usd, other.cost_usd) {
                (None, None) => None,
                (a, b) => Some(a.unwrap_or(0.0) + b.unwrap_or(0.0)),
            },
        }
    }

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

/// How hard the agent thinks, in the words Claude Code and Codex share.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Effort {
    Low,
    Medium,
    High,
    XHigh,
    Max,
}

impl Effort {
    pub const ALL: [Effort; 5] = [Self::Low, Self::Medium, Self::High, Self::XHigh, Self::Max];

    pub fn name(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::XHigh => "xhigh",
            Self::Max => "max",
        }
    }

    pub fn parse(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|e| e.name() == name)
    }
}

/// Which of the user's own settings the agent reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Settings {
    /// None: no hooks, no permission rules that skip asking, no MCP servers but ours.
    #[default]
    Isolated,
    /// The user's own (Z1's native mode): their `CLAUDE.md`, skills, MCPs, hooks and allow rules.
    /// Not the project's: `-p` skips Claude Code's folder trust prompt, so a cloned project's
    /// allow rules and hooks would apply unasked.
    User,
    /// The user's and the project's, for a folder the user already trusted in Claude Code
    /// ([`crate::claude_trusts`]): what `claude` reads there in a terminal.
    Project,
}

/// Where the agent's commands run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Sandbox {
    /// As the CLI itself sets it (Codex: writes inside the folder only).
    #[default]
    None,
    /// The CLI's own sandbox for every command (W4).
    Native,
}

/// What an agent runs with: given when the driver is made, fixed for its conversation.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Spec {
    pub dir: PathBuf,
    /// More folders it may read and write besides `dir`.
    pub add_dirs: Vec<PathBuf>,
    /// None: the CLI's own default.
    pub model: Option<String>,
    pub effort: Option<Effort>,
    pub settings: Settings,
    /// Added to the process's environment (Z1: `VULTS_ZECA=1`, so the hook knows the chat).
    pub env: Vec<(String, String)>,
    /// An MCP config file for this process alone (M7); never a write to the agent's config.
    /// Claude Code only, today: Codex takes its MCP servers from its own config.
    pub mcp_config: Option<PathBuf>,
    /// Appended to the system prompt after the persona (M2's frozen context).
    pub system_append: Option<String>,
    pub sandbox: Sandbox,
}

impl Spec {
    pub fn in_dir(dir: PathBuf) -> Self {
        Self {
            dir,
            ..Self::default()
        }
    }
}

/// How a turn ended.
#[derive(Debug, Clone, PartialEq)]
pub enum EndReason {
    Done,
    /// Interrupted on purpose ([`Interrupter`]); the conversation goes on next turn.
    Interrupted,
    Failed(String),
}

/// What an agent says while a turn runs.
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    /// More reply text.
    Text(String),
    /// What the turn cost; before the turn ends.
    Usage(Usage),
    /// A tool began; `id` pairs it with its [`Event::ToolEnd`]. `target` is what a card shows (cut),
    /// `detail` the whole command, path or query it acts on.
    ToolStart {
        id: String,
        tool: String,
        target: String,
        detail: String,
    },
    ToolEnd {
        id: String,
        ok: bool,
    },
    /// The turn is over: the last event of every turn that started.
    TurnEnd(EndReason),
}

/// A tool the agent wants to run: the card's tool and target as the agent names them, the whole
/// command when the target had to cut it, and the call's input as it sent it (for the card's
/// detail).
#[derive(Debug, Clone, PartialEq)]
pub struct Ask {
    pub id: String,
    pub tool: String,
    /// What the card shows.
    pub target: String,
    /// The whole command or path when `target` is cut: what an answer is bound to.
    pub full: Option<String>,
    pub input: Value,
}

/// A human's answer to an [`Ask`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Answer {
    Deny,
    Allow,
    /// Allow, and the same whole command again in this conversation (Z2): bound to `full` (or
    /// `target` when nothing was cut), gone when the conversation ends.
    AllowInConversation,
}

impl Answer {
    pub fn allows(self) -> bool {
        !matches!(self, Self::Deny)
    }
}

/// Where a driver reports, and asks. Whoever runs the driver implements it.
pub trait Sink: Send + Sync {
    fn event(&self, event: Event) -> BoxFuture<'_, ()>;
    /// Only a human's answer (rule 2); silence or a dropped card is [`Answer::Deny`].
    fn ask(&self, ask: Ask) -> BoxFuture<'_, Answer>;
}

/// Interrupts a driver's turn from outside it.
pub trait Interrupter: Send + Sync {
    /// Safe to call while a turn runs, and when none does; the driver stays usable, and the next
    /// turn resumes the same conversation. The turn ends with [`EndReason::Interrupted`].
    fn interrupt(&self) -> BoxFuture<'_, Result<(), String>>;
}

/// An agent the app runs. A driver keeps what lets the next turn go on (a session id, a live
/// process); `send` is one turn, ended when it resolves.
pub trait AgentDriver: Send + std::fmt::Debug {
    fn spec(&self) -> &Spec;
    /// A handle that can stop this driver's turns while `send` holds the driver.
    fn interrupter(&self) -> Arc<dyn Interrupter>;
    /// Gets ready for turns (a long-lived process starts here); a no-op for an agent that runs one
    /// process per turn.
    fn start(&mut self) -> BoxFuture<'_, Result<(), String>>;
    /// One turn: what the agent says goes to `sink`, which also answers its tool requests.
    fn send<'a>(&'a mut self, input: &'a Input, sink: &'a dyn Sink) -> BoxFuture<'a, Result<(), String>>;
    /// The agent's id for the conversation, once it has one.
    fn session(&self) -> Option<&str>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_effort_reads_back_its_name() {
        for e in Effort::ALL {
            assert_eq!(Effort::parse(e.name()), Some(e));
        }
        assert_eq!(Effort::parse("ultra"), None);
    }

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
