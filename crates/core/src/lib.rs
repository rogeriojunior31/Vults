//! The domain, with no IO, no async and no clock of its own: everything the app does flows
//! through [`reduce`], and the UI only renders [`State::view`].
//!
//! The rule that must never break: a permission is only answered because a human decided it
//! ([`Intent::Decide`]). Everything else can at most *release* a request, which leaves the
//! decision to the agent's terminal.

#![forbid(unsafe_code)]

pub mod i18n;
mod safe_url;
mod view;

use std::collections::{BTreeMap, VecDeque};
use std::time::{Duration, Instant};

pub use safe_url::SafeUrl;
use serde::{Deserialize, Serialize};
pub use view::{AlertView, ApprovalView, SessionView, ViewModel};
pub use vultures_ai_protocol::{AgentKind, Decision, Terminal};

/// Steps kept per session for the overview.
const MAX_STEPS: usize = 8;
/// Connector alerts kept on the island, newest first.
const MAX_ALERTS: usize = 5;
/// A pending card is dropped once the hook has surely given up.
const PENDING_TTL: Duration = vultures_ai_protocol::limits::SERVER_DECISION_TIMEOUT;
/// A session that sends nothing for this long has most likely died without a SessionEnd
/// (terminal closed, crash): its bird leaves the wire.
const SESSION_TTL: Duration = Duration::from_secs(30 * 60);
/// A finished session lingers this long before leaving.
const FINISHED_TTL: Duration = Duration::from_secs(10 * 60);

/// What an agent is doing right now; each one has its own animation clip.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum Activity {
    Read,
    Search,
    Edit,
    Run,
    Web,
    Plan,
    Subagent,
    Think,
    /// Anything we cannot classify.
    Work,
}

/// The session's state machine; approval, question, done… are states, not activities.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Idle,
    Thinking,
    Working,
    Approval,
    Question,
    Finished,
    Failed,
    RateLimited,
}

/// Sessions are keyed by agent too: Claude and Codex ids may collide.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SessionKey {
    pub agent: AgentKind,
    pub session_id: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RequestId(pub String);

/// One line of the session's log: what it did and on what.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Step {
    pub activity: Activity,
    /// The agent's tool name, shown when the activity is [`Activity::Work`].
    pub tool: String,
    /// The file, command or query, already shortened.
    pub detail: Option<String>,
}

/// An agent event, normalized by the `agents` crate.
#[derive(Clone, Debug, PartialEq)]
pub struct AgentUpdate {
    pub session: SessionKey,
    pub cwd: Option<String>,
    /// Where the agent runs, so a click can bring that terminal forward.
    pub terminal: Terminal,
    pub event: AgentEvent,
}

#[derive(Clone, Debug, PartialEq)]
pub enum AgentEvent {
    SessionStarted,
    PromptSubmitted,
    ToolStarted(Step),
    ToolFinished {
        failed: bool,
    },
    /// `target` is what Allow actually authorizes: `Bash · rm -rf build`, not just `Bash`.
    PermissionRequested {
        request: RequestId,
        tool: String,
        target: String,
    },
    Question {
        message: String,
    },
    RateLimited,
    Stopped,
    StopFailed,
    SessionEnded,
    SubagentStarted,
    SubagentStopped,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AlertLevel {
    Info,
    Ok,
    Warn,
    Error,
}

/// News from a connector (a pull request's checks failed, a review was requested…).
#[derive(Clone, Debug, PartialEq)]
pub struct Alert {
    /// Stable per news; the same key replaces the older alert.
    pub key: String,
    pub connector: String,
    pub level: AlertLevel,
    pub title: String,
    pub detail: String,
    /// Already checked: a link the app may open. Unsafe links are dropped, not shown.
    pub url: Option<SafeUrl>,
}

/// What a human did in the UI.
#[derive(Clone, Debug, PartialEq)]
pub enum Intent {
    /// A click on Allow / Deny, or Y / N with the card visible.
    Decide {
        request: RequestId,
        decision: Decision,
    },
    OpenAlert {
        key: String,
    },
    /// A click on a session: bring its terminal forward.
    Jump {
        session: SessionKey,
    },
    DismissAlert {
        key: String,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub enum Input {
    Agent(AgentUpdate),
    Connector(Alert),
    User(Intent),
    Tick,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Effect {
    /// The card for this request is on screen: the hook may wait for a human.
    AckPermission(RequestId),
    /// Only ever produced from [`Intent::Decide`].
    RespondPermission {
        request: RequestId,
        decision: Decision,
    },
    /// Nobody will decide this one here: the agent asks in its terminal.
    ReleasePermission(RequestId),
    OpenUrl(SafeUrl),
    JumpToTerminal(Terminal),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Session {
    pub key: SessionKey,
    pub project: String,
    pub cwd: Option<String>,
    pub status: Status,
    pub activity: Option<Activity>,
    pub steps: VecDeque<Step>,
    pub subagents: u32,
    pub updated: Instant,
    pub terminal: Terminal,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Pending {
    pub request: RequestId,
    pub session: SessionKey,
    pub tool: String,
    pub target: String,
    pub since: Instant,
}

#[derive(Clone, Debug, Default)]
pub struct State {
    pub sessions: BTreeMap<SessionKey, Session>,
    /// One card at a time: a second request would silently replace the first while the first
    /// still waits, so it is released to the terminal instead.
    pub pending: Option<Pending>,
    pub alerts: VecDeque<Alert>,
    pub lang: i18n::Lang,
}

pub fn reduce(state: &mut State, input: Input, now: Instant) -> Vec<Effect> {
    match input {
        Input::Agent(update) => on_agent(state, update, now),
        Input::Connector(alert) => {
            state.alerts.retain(|a| a.key != alert.key);
            state.alerts.push_front(alert);
            state.alerts.truncate(MAX_ALERTS);
            Vec::new()
        }
        Input::User(Intent::OpenAlert { key }) => {
            let Some(i) = state.alerts.iter().position(|a| a.key == key) else {
                return Vec::new();
            };
            // Opening is reading: the alert is done.
            let alert = state.alerts.remove(i);
            alert
                .and_then(|a| a.url)
                .map(Effect::OpenUrl)
                .into_iter()
                .collect()
        }
        Input::User(Intent::Jump { session }) => state
            .sessions
            .get(&session)
            .map(|s| vec![Effect::JumpToTerminal(s.terminal.clone())])
            .unwrap_or_default(),
        Input::User(Intent::DismissAlert { key }) => {
            state.alerts.retain(|a| a.key != key);
            Vec::new()
        }
        Input::User(Intent::Decide { request, decision }) => {
            // A click on a card that is gone (answered elsewhere, timed out) does nothing.
            let Some(p) = state.pending.take_if(|p| p.request == request) else {
                return Vec::new();
            };
            set_status(state, &p.session, Status::Working, now);
            vec![Effect::RespondPermission { request, decision }]
        }
        Input::Tick => {
            let mut effects = Vec::new();
            if let Some(p) = state
                .pending
                .take_if(|p| now.duration_since(p.since) >= PENDING_TTL)
            {
                set_status(state, &p.session, Status::Working, now);
                effects.push(Effect::ReleasePermission(p.request));
            }
            let waiting = state.pending.as_ref().map(|p| p.session.clone());
            state.sessions.retain(|key, s| {
                let quiet = now.duration_since(s.updated);
                let ttl = if s.status == Status::Finished {
                    FINISHED_TTL
                } else {
                    SESSION_TTL
                };
                Some(key) == waiting.as_ref() || quiet < ttl
            });
            effects
        }
    }
}

fn on_agent(state: &mut State, update: AgentUpdate, now: Instant) -> Vec<Effect> {
    let AgentUpdate {
        session: key,
        cwd,
        terminal,
        event,
    } = update;
    let mut effects = Vec::new();

    // Any later event from the session that holds the card means its terminal moved on.
    if !matches!(event, AgentEvent::PermissionRequested { .. })
        && let Some(p) = state.pending.take_if(|p| p.session == key)
    {
        effects.push(Effect::ReleasePermission(p.request));
    }

    let session = state.sessions.entry(key.clone()).or_insert_with(|| Session {
        key: key.clone(),
        project: String::new(),
        cwd: None,
        status: Status::Idle,
        activity: None,
        steps: VecDeque::new(),
        subagents: 0,
        updated: now,
        terminal: Terminal::default(),
    });
    // The latest event knows best where the agent runs (it may have moved to another pane).
    if terminal != Terminal::default() {
        session.terminal = terminal;
    }
    if let Some(name) = cwd.as_deref().and_then(project_name) {
        session.project = name;
        session.cwd = cwd;
    }
    session.updated = now;

    match event {
        AgentEvent::SessionStarted => session.status = Status::Idle,
        AgentEvent::PromptSubmitted => {
            session.status = Status::Thinking;
            session.activity = Some(Activity::Think);
        }
        AgentEvent::ToolStarted(step) => {
            session.status = Status::Working;
            session.activity = Some(step.activity);
            if session.steps.len() == MAX_STEPS {
                session.steps.pop_front();
            }
            session.steps.push_back(step);
        }
        AgentEvent::ToolFinished { .. } => session.status = Status::Working,
        AgentEvent::PermissionRequested {
            request,
            tool,
            target,
        } => {
            if state.pending.is_some() {
                effects.push(Effect::ReleasePermission(request));
            } else {
                session.status = Status::Approval;
                effects.push(Effect::AckPermission(request.clone()));
                state.pending = Some(Pending {
                    request,
                    session: key,
                    tool,
                    target,
                    since: now,
                });
            }
        }
        AgentEvent::Question { .. } => session.status = Status::Question,
        AgentEvent::RateLimited => session.status = Status::RateLimited,
        AgentEvent::Stopped => {
            session.status = Status::Finished;
            session.activity = None;
            session.subagents = 0;
        }
        AgentEvent::StopFailed => {
            session.status = Status::Failed;
            session.activity = None;
        }
        AgentEvent::SessionEnded => {
            state.sessions.remove(&key);
        }
        AgentEvent::SubagentStarted => session.subagents += 1,
        AgentEvent::SubagentStopped => session.subagents = session.subagents.saturating_sub(1),
    }
    effects
}

fn set_status(state: &mut State, key: &SessionKey, status: Status, now: Instant) {
    if let Some(s) = state.sessions.get_mut(key) {
        s.status = status;
        s.updated = now;
    }
}

fn project_name(cwd: &str) -> Option<String> {
    let name = cwd.trim_end_matches(['/', '\\']).rsplit(['/', '\\']).next()?;
    (!name.is_empty()).then(|| name.to_string())
}

#[cfg(test)]
mod tests;
