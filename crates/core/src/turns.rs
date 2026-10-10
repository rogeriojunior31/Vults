//! A turn: what one prompt set going, until the agent stopped. The core counts it as the events
//! come, and says when it ended ([`crate::Effect::Turn`]); the app keeps the history. Counts only:
//! no prompt, reply, command, file name or path ever leaves the session (docs/guide/activity.md).

use std::collections::BTreeSet;
use std::time::{Duration, Instant};

use vults_protocol::AgentKind;

use crate::{Activity, AgentEvent, Outcome, Session};

/// A turn with no event for this long has lost its agent (a crash, a closed terminal): it ends at
/// its last event.
pub const SILENT_END: Duration = Duration::from_secs(2 * 60 * 60);

/// The turn being counted, on its session.
#[derive(Clone, Debug, PartialEq)]
pub struct Draft {
    pub started: Instant,
    pub last: Instant,
    pub steps: u32,
    pub commands: u32,
    /// The files its edits changed, by path: counted, never kept past the turn.
    files: BTreeSet<String>,
    pub added: u32,
    pub removed: u32,
    pub allowed: u32,
    pub denied: u32,
    pub answered: u32,
    pub questions: u32,
}

impl Draft {
    fn new(now: Instant) -> Self {
        Self {
            started: now,
            last: now,
            steps: 0,
            commands: 0,
            files: BTreeSet::new(),
            added: 0,
            removed: 0,
            allowed: 0,
            denied: 0,
            answered: 0,
            questions: 0,
        }
    }
}

/// A finished turn, as the history keeps it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Turn {
    pub agent: AgentKind,
    /// The project's folder name, never its path; empty without a folder.
    pub project: String,
    /// From its first event to its last.
    pub secs: u64,
    /// How long ago its last event was: zero for one that just stopped, more for a silent one.
    pub ago: Duration,
    pub steps: u32,
    pub commands: u32,
    pub files: u32,
    pub added: u32,
    pub removed: u32,
    pub allowed: u32,
    pub denied: u32,
    pub answered: u32,
    pub questions: u32,
    pub failed: bool,
}

/// Counts an event into the session's turn, starting one on the events that set an agent going.
pub fn heard(session: &mut Session, event: &AgentEvent, now: Instant) {
    let starts = matches!(
        event,
        AgentEvent::PromptSubmitted
            | AgentEvent::ToolStarted(_)
            | AgentEvent::PermissionRequested { .. }
            | AgentEvent::QuestionAsked { .. }
            | AgentEvent::Question { .. }
    );
    if session.turn.is_none() && starts {
        session.turn = Some(Draft::new(now));
    }
    let Some(turn) = session.turn.as_mut() else {
        return;
    };
    turn.last = now;
    match event {
        AgentEvent::ToolStarted(step) => {
            turn.steps += 1;
            if step.activity == Activity::Run {
                turn.commands += 1;
            }
        }
        AgentEvent::ToolFinished {
            failed: false,
            diff: Some(diff),
            ..
        } => {
            for f in &diff.files {
                turn.files.insert(f.path.clone());
                turn.added = turn.added.saturating_add(f.added);
                turn.removed = turn.removed.saturating_add(f.removed);
            }
        }
        AgentEvent::QuestionAsked { .. } | AgentEvent::Question { .. } => turn.questions += 1,
        _ => {}
    }
}

/// A card of the session's left the line: the user's answers count, not what the terminal took.
pub fn decided(session: &mut Session, outcome: Outcome) {
    let Some(turn) = session.turn.as_mut() else {
        return;
    };
    match outcome {
        // A rule the user made answers as their Allow.
        Outcome::Allowed | Outcome::Rule => turn.allowed += 1,
        Outcome::Denied => turn.denied += 1,
        Outcome::Answered => turn.answered += 1,
        Outcome::Released | Outcome::Terminal | Outcome::Expired => {}
    }
}

/// Ends the session's turn, if one runs: at its last event.
pub fn end(session: &mut Session, now: Instant, failed: bool) -> Option<Turn> {
    let t = session.turn.take()?;
    Some(Turn {
        agent: session.key.agent,
        project: session.project.clone(),
        secs: t.last.saturating_duration_since(t.started).as_secs(),
        ago: now.saturating_duration_since(t.last),
        steps: t.steps,
        commands: t.commands,
        files: u32::try_from(t.files.len()).unwrap_or(u32::MAX),
        added: t.added,
        removed: t.removed,
        allowed: t.allowed,
        denied: t.denied,
        answered: t.answered,
        questions: t.questions,
        failed,
    })
}

/// Whether the session's turn went silent for good.
pub fn silent(session: &Session, now: Instant) -> bool {
    session
        .turn
        .as_ref()
        .is_some_and(|t| now.saturating_duration_since(t.last) >= SILENT_END)
}
