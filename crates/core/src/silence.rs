//! A quiet bird: a session *working* that has sent nothing for a while may be stuck (a command
//! waiting on input, a hung tool). After [`QUIET_AFTER`] its bird is flagged, after
//! [`LOUD_AFTER`] loudly. It only informs: nothing here acts on the agent. The user snoozes the
//! flag, says *keep going*, or dismisses it for the rest of that run.

use std::time::{Duration, Instant};

use serde::Serialize;

use crate::{AgentEvent, Session, Status};

pub const QUIET_AFTER: Duration = Duration::from_secs(5 * 60);
pub const LOUD_AFTER: Duration = Duration::from_secs(15 * 60);
/// *Snooze*: the flag goes for this long, then comes back if the bird is still quiet.
pub const SNOOZE: Duration = Duration::from_secs(15 * 60);
/// *Keep going*: no flag for this long; then the ladder starts again.
pub const KEEP_GOING: Duration = Duration::from_secs(30 * 60);

/// How long a working session has been quiet, as its bird shows it.
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub enum Silence {
    /// No news for [`QUIET_AFTER`].
    Quiet,
    /// No news for [`LOUD_AFTER`]: worth a look, and a notification.
    Loud,
}

/// What the user said about a quiet bird.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hush {
    Snooze,
    KeepGoing,
    /// Not again until the session's next run (a new prompt, or the turn ends).
    Dismiss,
}

/// The watch over one session's silence.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Watch {
    /// As of the last tick.
    pub level: Option<Silence>,
    /// *Keep going* moves the start of the silence to here.
    pub armed: Option<Instant>,
    pub snoozed_until: Option<Instant>,
    pub dismissed: bool,
    /// When the user last answered a flag: a bird they said to watch stays on the wire.
    pub touched: Option<Instant>,
}

/// The flag a session's silence earns now.
pub(crate) fn level(s: &Session, now: Instant) -> Option<Silence> {
    let w = &s.watch;
    if s.status != Status::Working || w.dismissed || w.snoozed_until.is_some_and(|t| now < t) {
        return None;
    }
    let from = w.armed.map_or(s.updated, |a| a.max(s.updated));
    let quiet = now.saturating_duration_since(from);
    if quiet >= LOUD_AFTER {
        Some(Silence::Loud)
    } else if quiet >= QUIET_AFTER {
        Some(Silence::Quiet)
    } else {
        None
    }
}

/// The user answered the flag.
pub(crate) fn hush(s: &mut Session, hush: Hush, now: Instant) {
    let w = &mut s.watch;
    match hush {
        Hush::Snooze => w.snoozed_until = Some(now + SNOOZE),
        Hush::KeepGoing => {
            w.armed = Some(now + KEEP_GOING - QUIET_AFTER);
            w.snoozed_until = None;
        }
        Hush::Dismiss => w.dismissed = true,
    }
    w.touched = Some(now);
    w.level = None;
}

/// The session spoke: no silence now. A new run forgets a dismissal.
pub(crate) fn heard(s: &mut Session, event: &AgentEvent) {
    let new_run = matches!(
        event,
        AgentEvent::SessionStarted
            | AgentEvent::PromptSubmitted
            | AgentEvent::Stopped { .. }
            | AgentEvent::StopFailed { .. }
    );
    let w = &mut s.watch;
    w.level = None;
    w.snoozed_until = None;
    w.armed = None;
    if new_run {
        w.dismissed = false;
    }
}
