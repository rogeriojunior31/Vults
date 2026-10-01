//! The domain, with no IO, no async and no clock of its own: everything the app does flows
//! through [`reduce`], and the UI only renders what comes out of it.
//!
//! M0 holds the types and the one rule that must never break: a permission is only ever
//! answered because a human decided it ([`Intent::Decide`]).

#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::time::Instant;

use serde::{Deserialize, Serialize};
pub use vultures_ai_protocol::{AgentKind, Decision};

/// What an agent is doing right now; each one has its own animation clip.
/// Approval, question, done, failure, rate limit, idle and sleep are session states, not activities.
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

/// Sessions are keyed by agent too: Claude and Codex ids may collide.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SessionKey {
    pub agent: AgentKind,
    pub session_id: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RequestId(pub String);

/// Agent events, already normalized by the `agents` crate.
#[derive(Clone, Debug, PartialEq)]
pub enum AgentEvent {
    PermissionRequested {
        session: SessionKey,
        request: RequestId,
        summary: String,
    },
}

/// What a human did in the UI.
#[derive(Clone, Debug, PartialEq)]
pub enum Intent {
    Decide { request: RequestId, decision: Decision },
}

#[derive(Clone, Debug, PartialEq)]
pub enum Input {
    Agent(AgentEvent),
    User(Intent),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Effect {
    /// Only ever produced from [`Intent::Decide`].
    RespondPermission { request: RequestId, decision: Decision },
}

#[derive(Clone, Debug, PartialEq)]
pub struct PendingPermission {
    pub session: SessionKey,
    pub summary: String,
    pub since: Instant,
}

#[derive(Clone, Debug, Default)]
pub struct State {
    pub pending: BTreeMap<RequestId, PendingPermission>,
}

pub fn reduce(state: &mut State, input: Input, now: Instant) -> Vec<Effect> {
    match input {
        Input::Agent(AgentEvent::PermissionRequested {
            session,
            request,
            summary,
        }) => {
            state.pending.insert(
                request,
                PendingPermission {
                    session,
                    summary,
                    since: now,
                },
            );
            Vec::new()
        }
        Input::User(Intent::Decide { request, decision }) => match state.pending.remove(&request) {
            Some(_) => vec![Effect::RespondPermission { request, decision }],
            // A click on a card that is gone (answered in the terminal, timed out) does nothing.
            None => Vec::new(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn requested(id: &str) -> Input {
        Input::Agent(AgentEvent::PermissionRequested {
            session: SessionKey {
                agent: AgentKind::Claude,
                session_id: "s".into(),
            },
            request: RequestId(id.into()),
            summary: "Run cargo test".into(),
        })
    }

    fn decide(id: &str, decision: Decision) -> Input {
        Input::User(Intent::Decide {
            request: RequestId(id.into()),
            decision,
        })
    }

    #[test]
    fn a_decision_answers_its_request_once() {
        let mut s = State::default();
        let now = Instant::now();
        assert!(reduce(&mut s, requested("a"), now).is_empty());
        assert_eq!(
            reduce(&mut s, decide("a", Decision::Allow), now),
            vec![Effect::RespondPermission {
                request: RequestId("a".into()),
                decision: Decision::Allow
            }]
        );
        assert!(reduce(&mut s, decide("a", Decision::Deny), now).is_empty());
    }

    #[test]
    fn unknown_requests_are_never_answered() {
        let mut s = State::default();
        assert!(reduce(&mut s, decide("ghost", Decision::Allow), Instant::now()).is_empty());
    }
}
