//! Which watched session the user may hand words to (`docs/dev/plan-zeca.md` W7, `/continue`): one
//! that stopped, in the chat's folder, at most [`EXPIRES`] ago. The core keeps the sessions that
//! ended a little while, since an ended session is the one most often continued; whether its
//! process is still alive is the app's to check (a session whose terminal still runs it is never
//! continued from here: its terminal answers).

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use crate::{SessionKey, State, Status};

/// How long after it stopped a session may still be continued from here.
pub const EXPIRES: Duration = Duration::from_secs(10 * 60);
/// How many ended sessions are kept for it.
const KEPT: usize = 20;

/// A session that ended, as it was.
#[derive(Clone, Debug, PartialEq)]
pub struct Ended {
    pub key: SessionKey,
    pub cwd: Option<String>,
    pub pid: Option<u32>,
    pub at: Instant,
}

/// One the user could continue.
#[derive(Clone, Debug, PartialEq)]
pub struct Candidate {
    pub key: SessionKey,
    pub cwd: String,
    /// The agent's process when the session was last heard from, if the OS told.
    pub pid: Option<u32>,
    /// Its session ended (the agent said so): no process holds it.
    pub ended: bool,
    /// How long since it stopped.
    pub ago: Duration,
}

/// Keeps an ended session for [`EXPIRES`], at most [`KEPT`] of them.
pub(crate) fn ended(list: &mut VecDeque<Ended>, e: Ended) {
    list.retain(|o| o.key != e.key);
    list.push_front(e);
    list.truncate(KEPT);
}

impl State {
    /// The sessions in `cwd` that stopped, or ended, at most [`EXPIRES`] ago, the latest first:
    /// only Claude Code's and Codex's, which can be resumed, and never one with a card waiting.
    pub fn continuable(&self, cwd: &str, now: Instant) -> Vec<Candidate> {
        let fresh = |at: Instant| now.saturating_duration_since(at) <= EXPIRES;
        let resumable = |k: &SessionKey| {
            matches!(
                k.agent,
                vults_protocol::AgentKind::Claude | vults_protocol::AgentKind::Codex
            )
        };
        let mut out: Vec<Candidate> = self
            .sessions
            .iter()
            .filter(|(k, s)| {
                resumable(k)
                    && s.cwd.as_deref() == Some(cwd)
                    && matches!(s.status, Status::Finished | Status::Failed | Status::Idle)
                    && fresh(s.updated)
                    && !self.pending.iter().any(|p| &p.session == *k)
            })
            .map(|(k, s)| Candidate {
                key: k.clone(),
                cwd: cwd.to_string(),
                pid: s.terminal.pid,
                ended: false,
                ago: now.saturating_duration_since(s.updated),
            })
            .collect();
        out.extend(
            self.closed
                .iter()
                .filter(|e| resumable(&e.key) && e.cwd.as_deref() == Some(cwd) && fresh(e.at))
                .filter(|e| !self.sessions.contains_key(&e.key))
                .map(|e| Candidate {
                    key: e.key.clone(),
                    cwd: cwd.to_string(),
                    pid: e.pid,
                    ended: true,
                    ago: now.saturating_duration_since(e.at),
                }),
        );
        out.sort_by_key(|c| c.ago);
        out
    }
}
