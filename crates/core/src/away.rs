//! "While you were away": what happened while the screen was locked or the app paused, told once
//! when the user is back. Deterministic: counts from the state, a sentence from `i18n`, no model
//! (ADR 0012).

use std::collections::BTreeSet;
use std::time::{Duration, Instant};

use serde::Serialize;

use crate::{AgentEvent, SessionKey, State, Status, i18n};

/// What is being kept while the user is away.
#[derive(Clone, Debug, PartialEq)]
pub struct Away {
    pub since: Instant,
    pub finished: BTreeSet<SessionKey>,
    pub failed: BTreeSet<SessionKey>,
}

/// The story told on return.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Digest {
    /// New for each digest: a surface shows each one once.
    pub seq: u64,
    pub finished: u32,
    pub failed: u32,
    /// Cards still waiting for the user, and the longest wait among them.
    pub waiting: u32,
    pub waited: Duration,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct DigestView {
    pub seq: u64,
    /// The whole sentence ("While you were away: 2 finished, 1 failed.").
    pub text: String,
    pub finished: u32,
    pub failed: u32,
    pub waiting: u32,
}

/// The user left: keep what happens from now (an earlier start is kept).
pub(crate) fn leave(state: &mut State, now: Instant) {
    state.away.get_or_insert_with(|| Away {
        since: now,
        finished: BTreeSet::new(),
        failed: BTreeSet::new(),
    });
}

/// A turn that ended while away is news for the digest; a later turn's end replaces it.
pub(crate) fn heard(state: &mut State, key: &SessionKey, event: &AgentEvent) {
    let Some(a) = state.away.as_mut() else { return };
    match event {
        AgentEvent::Stopped { .. } => {
            a.failed.remove(key);
            a.finished.insert(key.clone());
        }
        AgentEvent::StopFailed { .. } => {
            a.finished.remove(key);
            a.failed.insert(key.clone());
        }
        _ => {}
    }
}

/// The user is back: the digest of what happened, plus `missed` (sessions whose news the
/// notifications held back: paused, off, do not disturb). Nothing to tell, no digest.
pub(crate) fn back(state: &mut State, missed: &[SessionKey], now: Instant) {
    let mut away = state.away.take().unwrap_or(Away {
        since: now,
        finished: BTreeSet::new(),
        failed: BTreeSet::new(),
    });
    // Held-back news counts only when it came while away: older news was there to be seen.
    for key in missed {
        let Some(s) = state.sessions.get(key).filter(|s| s.updated >= away.since) else {
            continue;
        };
        match s.status {
            Status::Finished => {
                away.finished.insert(key.clone());
            }
            Status::Failed => {
                away.failed.insert(key.clone());
            }
            _ => {}
        }
    }
    // Only what tells at rest: not a hidden project, not a muted one (as `notify`).
    let tells = |k: &SessionKey| {
        state
            .sessions
            .get(k)
            .is_some_and(|s| state.visible(s) && !state.prefs(s).mute)
    };
    let finished = away.finished.iter().filter(|k| tells(k)).count() as u32;
    let failed = away.failed.iter().filter(|k| tells(k)).count() as u32;
    // The cards that came while away and still wait: one already waiting before is not news.
    let cards: Vec<&crate::Pending> = state
        .pending
        .iter()
        .filter(|p| p.since >= away.since)
        .filter(|p| state.sessions.get(&p.session).is_some_and(|s| crate::shows(s, p)))
        .collect();
    let waiting = cards.len() as u32;
    let waited = cards
        .iter()
        .map(|p| now.saturating_duration_since(p.since))
        .max()
        .unwrap_or_default();
    if finished + failed + waiting == 0 {
        return;
    }
    state.digest_seq += 1;
    state.digest = Some(Digest {
        seq: state.digest_seq,
        finished,
        failed,
        waiting,
        waited,
    });
}

pub(crate) fn view(state: &State) -> Option<DigestView> {
    let d = state.digest.as_ref()?;
    Some(DigestView {
        seq: d.seq,
        text: i18n::digest(state.lang, d),
        finished: d.finished,
        failed: d.failed,
        waiting: d.waiting,
    })
}
