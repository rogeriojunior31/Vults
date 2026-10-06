//! The domain, with no IO, no async and no clock of its own: everything the app does flows
//! through [`reduce`], and the UI only renders [`State::view`].
//!
//! The rule that must never break: a permission is only answered because a human decided it
//! ([`Intent::Decide`]). Everything else can at most *release* a request, which leaves the
//! decision to the agent's terminal.

#![forbid(unsafe_code)]

pub mod board;
pub mod flock;
pub mod i18n;
pub mod looks;
pub mod notify;
mod safe_url;
mod view;

use std::collections::{BTreeMap, VecDeque};
use std::time::{Duration, Instant};

pub use safe_url::SafeUrl;
use serde::{Deserialize, Serialize};
pub use view::{AlertView, ApprovalView, DiffSummary, EndedView, SessionRef, SessionView, ViewModel};
pub use vultures_ai_protocol::{AgentKind, Answer, Decision, Terminal};

/// Longest reply the user may type to a question.
const MAX_ANSWER_LEN: usize = 2_000;
/// Steps kept per session for the overview.
const MAX_STEPS: usize = 8;
/// Ended cards kept for the view, newest first.
const MAX_ENDED: usize = 8;
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
#[cfg_attr(test, derive(ts_rs::TS))]
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
#[cfg_attr(test, derive(ts_rs::TS))]
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

/// How much a session wants the user, in order: the higher, the more it does. Surfaces choose the
/// sound, the icon and the color; the level is the same for all of them (ADR 0008).
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "kebab-case")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub enum Attention {
    /// Working, thinking, idle: nothing to tell.
    Quiet,
    /// Worth a glance, not news (a rate limit it waits out by itself).
    Info,
    /// The turn ended well.
    Done,
    /// The turn ended on an error.
    Failed,
    /// A permission or a question waits for the user, here or in the terminal.
    NeedsYou,
}

impl Status {
    pub fn attention(self) -> Attention {
        match self {
            Status::Idle | Status::Thinking | Status::Working => Attention::Quiet,
            Status::RateLimited => Attention::Info,
            Status::Finished => Attention::Done,
            Status::Failed => Attention::Failed,
            Status::Approval | Status::Question => Attention::NeedsYou,
        }
    }
}

/// How a card left the line (ADR 0008). Information only: it answers nothing, it says what an
/// answer already did.
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub enum Outcome {
    /// Allow or Always, here.
    Allowed,
    /// Deny, here.
    Denied,
    /// A question card answered here.
    Answered,
    /// Sent to the terminal from here ("Reply in the terminal").
    Released,
    /// The agent moved on, or its session ended: the user answered in the terminal.
    Terminal,
    /// Nobody answered in time: the terminal asks now.
    Expired,
    /// A rule the user had just made (an Always on an identical card) answered it.
    Rule,
}

/// A card that left the line, and how.
#[derive(Clone, Debug, PartialEq)]
pub struct Ended {
    pub request: RequestId,
    pub session: SessionKey,
    pub outcome: Outcome,
}

/// Sessions are keyed by agent too: Claude and Codex ids may collide. Another tool's session id
/// starts with its name (`my-tool/<id>`), so two tools never share one.
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
    /// The subagent that sent it; `None` for the session's main agent. Each agent works one tool
    /// at a time, so only its own events say its permission was settled elsewhere.
    pub agent_id: Option<String>,
    pub event: AgentEvent,
}

/// What a permission card shows besides the target: the agent's own words for the action, the
/// whole command when the target had to cut it, and for an edit the lines it adds and removes.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Ask {
    pub description: Option<String>,
    pub full: Option<String>,
    pub added: u32,
    pub removed: u32,
}

/// What a finished edit changed, file by file, for the island's diff card.
#[derive(Serialize, Clone, Debug, PartialEq, Eq, Default)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct Diff {
    pub files: Vec<FileDiff>,
    /// The patch was longer than what reached us: the card says it stops short.
    pub cut: bool,
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct FileDiff {
    pub path: String,
    pub added: u32,
    pub removed: u32,
    pub hunks: Vec<Hunk>,
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct Hunk {
    /// The hunk's first line in the old and the new file, when the agent said.
    pub old_start: Option<u32>,
    pub new_start: Option<u32>,
    /// Each line behind its mark: `+`, `-` or a space.
    pub lines: Vec<String>,
}

impl Diff {
    pub fn added(&self) -> u32 {
        self.files.iter().map(|f| f.added).sum()
    }

    pub fn removed(&self) -> u32 {
        self.files.iter().map(|f| f.removed).sum()
    }
}

/// One question the agent asks the user, with its choices. The user may also answer in their own
/// words.
#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct Question {
    pub question: String,
    /// A short tag for it ("Color").
    pub header: String,
    pub options: Vec<Choice>,
    /// Several choices may be picked.
    pub multi: bool,
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct Choice {
    pub label: String,
    pub description: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum AgentEvent {
    SessionStarted,
    PromptSubmitted,
    ToolStarted(Step),
    ToolFinished {
        failed: bool,
        /// Same form as `PermissionRequested::target`, when the agent says which call finished.
        target: Option<String>,
        /// What an edit changed, when the agent sent its patch.
        diff: Option<Diff>,
    },
    /// `target` is what Allow actually authorizes: `Bash · rm -rf build`, not just `Bash`.
    PermissionRequested {
        request: RequestId,
        tool: String,
        target: String,
        ask: Ask,
    },
    /// The agent asks something its terminal would otherwise ask; the island may answer.
    QuestionAsked {
        request: RequestId,
        /// Same form as `PermissionRequested::target`, so the call finishing settles the card.
        target: String,
        questions: Vec<Question>,
    },
    /// The agent asks something in its terminal; only the terminal can answer.
    Question {
        message: String,
    },
    RateLimited,
    /// The turn ended. `message` is the first paragraph of the agent's last reply as one plain
    /// line, when the agent sends it.
    Stopped {
        message: Option<String>,
    },
    /// The turn ended on an error (an API failure, not a failed tool).
    StopFailed {
        error: Option<String>,
    },
    SessionEnded,
    SubagentStarted,
    SubagentStopped,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
#[cfg_attr(test, derive(ts_rs::TS))]
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
    /// The story it belongs to (`pr:owner/repo#12:ci`): a newer alert of the same topic retires
    /// the older ones, so a pass does not sit next to the failure it fixed.
    pub topic: Option<String>,
    /// Set by the core on arrival, new each time: news that comes again (a review requested
    /// again) is news again, even under a key the island has shown.
    pub seq: u64,
    pub connector: String,
    pub level: AlertLevel,
    pub title: String,
    pub detail: String,
    /// Already checked: a link the app may open. Unsafe links are dropped, not shown.
    pub url: Option<SafeUrl>,
}

/// A permission the user chose to always allow: this exact tool and target, for this agent, in this
/// project. Nothing broader: `cargo test` does not allow `cargo test && rm -rf build`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Rule {
    pub agent: AgentKind,
    pub cwd: String,
    pub tool: String,
    pub target: String,
}

/// What a human did in the UI.
#[derive(Clone, Debug, PartialEq)]
pub enum Intent {
    /// A click on Allow / Deny, or Y / N with the card visible.
    Decide {
        request: RequestId,
        decision: Decision,
    },
    /// A click on Always: allow this request and every identical one in this project.
    DecideAlways {
        request: RequestId,
    },
    /// The user's replies to a question card, one per question.
    Answer {
        request: RequestId,
        answers: Vec<Answer>,
    },
    /// "Reply in the terminal": the card goes and the agent's terminal asks.
    Release {
        request: RequestId,
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
    /// A click on a row of a connector's card.
    OpenRow {
        connector: String,
        item: String,
    },
    /// Put this session in front (a click on its bird or row); `None` gives the choice back to
    /// [`State::front`]'s rule. A waiting card still comes first.
    Focus {
        session: Option<SessionKey>,
    },
    /// The next session after the one in front, in the view's order, wrapping (a shortcut).
    FocusNext,
    /// The one before it, wrapping.
    FocusPrevious,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Input {
    Agent(AgentUpdate),
    Connector(Alert),
    /// A connector's card after a good poll; `None` once it is switched off.
    Board {
        connector: String,
        rows: Option<Vec<board::Row>>,
    },
    User(Intent),
    /// The saved rules: at start-up, and after the user removes one in the settings.
    SetRules(Vec<Rule>),
    /// The user chose another pool for the flock: sessions draw their birds again from it.
    SetFlock(flock::Flock),
    /// Zeca's look, as the user chose it in the settings.
    SetOutfit(looks::Outfit),
    /// The user's date, at start-up and on every tick: the seasonal looks follow it.
    Today(looks::Date),
    Tick,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Effect {
    /// The card for this request is on screen: the hook may wait for a human.
    AckPermission(RequestId),
    /// Only ever produced from [`Intent::Decide`], [`Intent::DecideAlways`], or a saved rule a
    /// human made that matches the request exactly: agent, folder, tool and target (ADR 0004).
    RespondPermission {
        request: RequestId,
        decision: Decision,
    },
    /// Only ever produced from [`Intent::Answer`].
    AnswerQuestion {
        request: RequestId,
        answers: Vec<Answer>,
    },
    /// Nobody will decide this one here: the agent asks in its terminal.
    ReleasePermission(RequestId),
    OpenUrl(SafeUrl),
    JumpToTerminal(Terminal),
    /// The rules changed (a new Always): write them to the settings.
    SaveRules(Vec<Rule>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Session {
    pub key: SessionKey,
    pub project: String,
    pub cwd: Option<String>,
    pub status: Status,
    pub activity: Option<Activity>,
    pub steps: VecDeque<Step>,
    /// Every step so far, not just the ones kept.
    pub step_count: u32,
    pub subagents: u32,
    pub updated: Instant,
    /// When the session first showed up: the oldest of a project is its king.
    pub started: Instant,
    pub terminal: Terminal,
    /// What the state is about, in the agent's words: the question asked, the last reply, the
    /// error. Shown on the island, never logged.
    pub note: Option<String>,
    /// Numbers (counted like `step_count`) of the latest steps one of the user's rules allowed.
    pub ruled: VecDeque<u32>,
    /// The diffs of kept steps, by step number (counted like `step_count`).
    pub diffs: VecDeque<(u32, Diff)>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Pending {
    pub request: RequestId,
    pub session: SessionKey,
    /// The subagent that asked, if any.
    pub agent_id: Option<String>,
    pub tool: String,
    pub target: String,
    pub ask: Ask,
    /// A question card's questions; empty for a permission.
    pub questions: Vec<Question>,
    pub since: Instant,
}

#[derive(Clone, Debug, Default)]
pub struct State {
    pub sessions: BTreeMap<SessionKey, Session>,
    /// Every permission or question waiting for a human, oldest first. The island shows the first; each one
    /// is answered only by its own click (or a rule), and each keeps its own deadline.
    pub pending: VecDeque<Pending>,
    /// The cards that left the line most recently, newest first.
    pub ended: VecDeque<Ended>,
    pub alerts: VecDeque<Alert>,
    /// The last [`Alert::seq`] given.
    pub alert_seq: u64,
    /// Each connector's card, by connector id.
    pub boards: BTreeMap<String, Vec<board::Row>>,
    pub rules: Vec<Rule>,
    pub lang: i18n::Lang,
    /// Picked by the app at start-up (the core draws nothing itself): each season, a new flock.
    pub season: u64,
    /// The pool the flock draws from, as the user chose it.
    pub flock: flock::Flock,
    /// What Zeca wears, as the user chose it.
    pub outfit: looks::Outfit,
    /// The user's date, from the app; none until it says.
    pub today: Option<looks::Date>,
    /// The session the user put in front; forgotten when it leaves.
    pub focus: Option<SessionKey>,
}

pub fn reduce(state: &mut State, input: Input, now: Instant) -> Vec<Effect> {
    let effects = apply(state, input, now);
    if state
        .focus
        .as_ref()
        .is_some_and(|k| !state.sessions.contains_key(k))
    {
        state.focus = None;
    }
    effects
}

fn apply(state: &mut State, input: Input, now: Instant) -> Vec<Effect> {
    match input {
        Input::Agent(update) => on_agent(state, update, now),
        Input::Connector(mut alert) => {
            state
                .alerts
                .retain(|a| a.key != alert.key && (alert.topic.is_none() || a.topic != alert.topic));
            state.alert_seq += 1;
            alert.seq = state.alert_seq;
            state.alerts.push_front(alert);
            state.alerts.truncate(MAX_ALERTS);
            Vec::new()
        }
        Input::Board { connector, rows } => {
            board::set(state, connector, rows);
            Vec::new()
        }
        Input::User(Intent::OpenRow { connector, item }) => board::url(state, &connector, &item)
            .map(Effect::OpenUrl)
            .into_iter()
            .collect(),
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
        Input::User(Intent::DecideAlways { request }) => {
            let Some(p) = take_pending(state, |p| p.request == request && p.questions.is_empty()) else {
                return Vec::new();
            };
            record_end(state, &p, Outcome::Allowed);
            let mut effects = vec![Effect::RespondPermission {
                request,
                decision: Decision::Allow,
            }];
            let cwd = state.sessions.get(&p.session).and_then(|s| s.cwd.clone());
            settle(state, &p.session, now);
            // Without a folder there is nothing to scope the rule to: it is a plain Allow.
            if let Some(cwd) = cwd {
                let rule = Rule {
                    agent: p.session.agent,
                    cwd,
                    tool: p.tool,
                    target: p.target,
                };
                // The same request may already wait again (an agent retrying): the new rule
                // answers it too, as it will answer every identical one from now on.
                let same: Vec<RequestId> = state
                    .pending
                    .iter()
                    .filter(|q| {
                        let cwd = state.sessions.get(&q.session).and_then(|s| s.cwd.as_ref());
                        q.session.agent == rule.agent
                            && q.tool == rule.tool
                            && q.target == rule.target
                            && cwd == Some(&rule.cwd)
                    })
                    .map(|q| q.request.clone())
                    .collect();
                for id in same {
                    if let Some(q) = take_pending(state, |p| p.request == id) {
                        record_end(state, &q, Outcome::Rule);
                        mark_ruled(state, &q.session);
                        settle(state, &q.session, now);
                        effects.push(Effect::RespondPermission {
                            request: q.request,
                            decision: Decision::Allow,
                        });
                    }
                }
                if !state.rules.contains(&rule) {
                    state.rules.push(rule);
                    effects.push(Effect::SaveRules(state.rules.clone()));
                }
            }
            effects
        }
        Input::SetRules(rules) => {
            state.rules = rules;
            Vec::new()
        }
        Input::SetFlock(flock) => {
            state.flock = flock;
            Vec::new()
        }
        Input::SetOutfit(outfit) => {
            state.outfit = outfit;
            Vec::new()
        }
        Input::Today(date) => {
            state.today = Some(date);
            Vec::new()
        }
        Input::User(Intent::Jump { session }) => state
            .sessions
            .get(&session)
            .map(|s| vec![Effect::JumpToTerminal(s.terminal.clone())])
            .unwrap_or_default(),
        Input::User(Intent::Focus { session }) => {
            if session.as_ref().is_none_or(|k| state.sessions.contains_key(k)) {
                state.focus = session;
            }
            Vec::new()
        }
        Input::User(Intent::FocusNext) => {
            step_focus(state, true);
            Vec::new()
        }
        Input::User(Intent::FocusPrevious) => {
            step_focus(state, false);
            Vec::new()
        }
        Input::User(Intent::DismissAlert { key }) => {
            state.alerts.retain(|a| a.key != key);
            Vec::new()
        }
        Input::User(Intent::Decide { request, decision }) => {
            // A click on a card that is gone (answered elsewhere, timed out) does nothing. Allow
            // would run a question with no answers: only Answer settles one.
            let Some(p) = take_pending(state, |p| p.request == request && p.questions.is_empty()) else {
                return Vec::new();
            };
            let outcome = match decision {
                Decision::Allow => Outcome::Allowed,
                Decision::Deny => Outcome::Denied,
            };
            record_end(state, &p, outcome);
            settle(state, &p.session, now);
            vec![Effect::RespondPermission { request, decision }]
        }
        Input::User(Intent::Answer { request, answers }) => {
            let Some(p) = take_pending(state, |p| p.request == request && fits(&p.questions, &answers))
            else {
                return Vec::new();
            };
            record_end(state, &p, Outcome::Answered);
            settle(state, &p.session, now);
            vec![Effect::AnswerQuestion { request, answers }]
        }
        Input::User(Intent::Release { request }) => {
            let Some(p) = take_pending(state, |p| p.request == request) else {
                return Vec::new();
            };
            record_end(state, &p, Outcome::Released);
            settle(state, &p.session, now);
            vec![Effect::ReleasePermission(request)]
        }
        Input::Tick => {
            let mut effects = Vec::new();
            while let Some(p) = take_pending(state, |p| now.duration_since(p.since) >= PENDING_TTL) {
                record_end(state, &p, Outcome::Expired);
                settle(state, &p.session, now);
                effects.push(Effect::ReleasePermission(p.request));
            }
            let waiting: Vec<SessionKey> = state.pending.iter().map(|p| p.session.clone()).collect();
            state.sessions.retain(|key, s| {
                let quiet = now.duration_since(s.updated);
                let ttl = if s.status == Status::Finished {
                    FINISHED_TTL
                } else {
                    SESSION_TTL
                };
                waiting.contains(key) || quiet < ttl
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
        agent_id,
        event,
    } = update;
    let mut effects = Vec::new();

    // A later event from the agent that asked means its terminal moved on (the user answered
    // there). Only that agent's: a subagent working in parallel says nothing about it. A tool
    // that finished settles only its own card: a parallel call may still wait for its answer.
    // The session ending settles every permission it still has.
    if !matches!(
        event,
        AgentEvent::PermissionRequested { .. } | AgentEvent::QuestionAsked { .. }
    ) {
        let ended = matches!(event, AgentEvent::SessionEnded);
        let finished = match &event {
            AgentEvent::ToolFinished { target: Some(t), .. } => Some(t.clone()),
            _ => None,
        };
        let settles = |p: &Pending| {
            p.session == key
                && (ended || p.agent_id == agent_id && finished.as_ref().is_none_or(|t| &p.target == t))
        };
        while let Some(p) = take_pending(state, settles) {
            record_end(state, &p, Outcome::Terminal);
            effects.push(Effect::ReleasePermission(p.request));
        }
    }

    let session = state.sessions.entry(key.clone()).or_insert_with(|| Session {
        key: key.clone(),
        project: String::new(),
        cwd: None,
        status: Status::Idle,
        activity: None,
        steps: VecDeque::new(),
        step_count: 0,
        subagents: 0,
        updated: now,
        started: now,
        terminal: Terminal::default(),
        note: None,
        ruled: VecDeque::new(),
        diffs: VecDeque::new(),
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
    // A note belongs to the state it explains; anything that changes the state drops it.
    if !matches!(event, AgentEvent::SubagentStarted | AgentEvent::SubagentStopped) {
        session.note = None;
    }

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
            session.step_count += 1;
            let first = session.step_count + 1 - session.steps.len() as u32;
            session.diffs.retain(|(n, _)| *n >= first);
        }
        // A parallel call may still wait for its answer: its card stays only while the session
        // says it needs approval.
        AgentEvent::ToolFinished { failed, diff, .. } => {
            if !state.pending.iter().any(|p| p.session == key) {
                session.status = Status::Working;
            }
            if let Some(diff) = diff.filter(|_| !failed) {
                attach(session, diff);
            }
        }
        AgentEvent::PermissionRequested {
            request,
            tool,
            target,
            ask,
        } => {
            let ruled = session.cwd.as_ref().is_some_and(|cwd| {
                state
                    .rules
                    .iter()
                    .any(|r| r.agent == key.agent && &r.cwd == cwd && r.tool == tool && r.target == target)
            });
            if ruled {
                // The user already said always: answer at once, no card; the step says so.
                effects.push(Effect::AckPermission(request.clone()));
                effects.push(Effect::RespondPermission {
                    request,
                    decision: Decision::Allow,
                });
                mark_ruled(state, &key);
            } else {
                session.status = Status::Approval;
                effects.push(Effect::AckPermission(request.clone()));
                state.pending.push_back(Pending {
                    request,
                    session: key,
                    agent_id,
                    tool,
                    target,
                    ask,
                    questions: Vec::new(),
                    since: now,
                });
            }
        }
        AgentEvent::QuestionAsked {
            request,
            target,
            questions,
        } => {
            session.status = Status::Question;
            session.note = questions.first().and_then(|q| note(q.question.clone()));
            effects.push(Effect::AckPermission(request.clone()));
            state.pending.push_back(Pending {
                request,
                session: key,
                agent_id,
                tool: vultures_ai_protocol::QUESTION_TOOL.to_string(),
                target,
                ask: Ask::default(),
                questions,
                since: now,
            });
        }
        AgentEvent::Question { message } => {
            session.status = Status::Question;
            session.note = note(message);
        }
        AgentEvent::RateLimited => session.status = Status::RateLimited,
        AgentEvent::Stopped { message } => {
            session.status = Status::Finished;
            session.activity = None;
            session.subagents = 0;
            session.note = message.and_then(note);
        }
        AgentEvent::StopFailed { error } => {
            session.status = Status::Failed;
            session.activity = None;
            session.note = error.and_then(note);
        }
        AgentEvent::SessionEnded => {
            state.sessions.remove(&key);
        }
        AgentEvent::SubagentStarted => session.subagents += 1,
        AgentEvent::SubagentStopped => session.subagents = session.subagents.saturating_sub(1),
    }
    effects
}

/// The diff goes to the latest edit of its file that has none yet: calls may finish out of order.
fn attach(session: &mut Session, diff: Diff) {
    let Some(file) = diff.files.first().map(|f| file_name(&f.path)) else {
        return;
    };
    let first = session.step_count + 1 - session.steps.len() as u32;
    let step = session.steps.iter().enumerate().rev().find_map(|(i, st)| {
        let n = first + i as u32;
        let named = st
            .detail
            .as_deref()
            .is_some_and(|d| d == file || d.starts_with(&format!("{file} +")));
        (st.activity == Activity::Edit && named && session.diffs.iter().all(|(m, _)| *m != n)).then_some(n)
    });
    if let Some(n) = step {
        session.diffs.push_back((n, diff));
    }
}

fn file_name(path: &str) -> &str {
    path.trim_end_matches(['/', '\\'])
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(path)
}

/// One reply per question: a choice or the user's own words, several only where several may be
/// picked, never blank.
fn fits(questions: &[Question], answers: &[Answer]) -> bool {
    let filled = |t: &String| !t.trim().is_empty() && t.len() <= MAX_ANSWER_LEN;
    !questions.is_empty()
        && questions.len() == answers.len()
        && questions.iter().zip(answers).all(|(q, a)| match a {
            Answer::One(t) => filled(t),
            Answer::Many(ts) => q.multi && !ts.is_empty() && ts.iter().all(filled),
        })
}

/// Moves the focus one session along from the one in front. A waiting card stays in front: the
/// focus walks behind it, from where it is, and takes over once the card is gone.
fn step_focus(state: &mut State, forward: bool) {
    let keys: Vec<SessionKey> = state.ordered().iter().map(|s| s.key.clone()).collect();
    let n = keys.len();
    if n == 0 {
        return;
    }
    let from = match state.card_session() {
        Some(card) => Some(state.focus.as_ref().unwrap_or(card)),
        None => state.front(),
    };
    let at = from.and_then(|f| keys.iter().position(|k| k == f));
    let i = match (at, forward) {
        (Some(i), true) => (i + 1) % n,
        (Some(i), false) => (i + n - 1) % n,
        // Only when the front is not on the wire, which `reduce` never leaves behind.
        (None, true) => 0,
        (None, false) => n - 1,
    };
    state.focus = Some(keys[i].clone());
}

/// Takes the first waiting permission that matches.
fn take_pending(state: &mut State, matches: impl Fn(&Pending) -> bool) -> Option<Pending> {
    let i = state.pending.iter().position(matches)?;
    state.pending.remove(i)
}

/// Notes how a card left the line, for the view.
fn record_end(state: &mut State, p: &Pending, outcome: Outcome) {
    state.ended.push_front(Ended {
        request: p.request.clone(),
        session: p.session.clone(),
        outcome,
    });
    state.ended.truncate(MAX_ENDED);
}

/// A permission of this session was settled: it works again, unless another one still waits.
fn settle(state: &mut State, key: &SessionKey, now: Instant) {
    if !state.pending.iter().any(|p| &p.session == key) {
        set_status(state, key, Status::Working, now);
    }
}

/// The session's latest step ran because of one of the user's rules.
fn mark_ruled(state: &mut State, key: &SessionKey) {
    if let Some(s) = state.sessions.get_mut(key)
        && s.step_count > 0
        && s.ruled.back() != Some(&s.step_count)
    {
        s.ruled.push_back(s.step_count);
        if s.ruled.len() > MAX_STEPS {
            s.ruled.pop_front();
        }
    }
}

/// Blank text is no note.
fn note(text: String) -> Option<String> {
    let text = text.trim();
    (!text.is_empty()).then(|| text.to_string())
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
