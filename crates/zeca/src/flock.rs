//! What Zeca may know of the flock, and nothing he could act with (ADR 0010, 0018). The app fills a
//! [`Flock`] from its view; Zeca reads it through a [`FlockReader`]. This crate does not depend on
//! `vults-core`, so no code in it can build an `Intent`, let alone answer a permission:
//!
//! ```compile_fail
//! // Zeca has no way to name the core's intents (vults-core is not a dependency).
//! use vults_core::Intent;
//! ```
//!
//! `scripts/check-layers.sh` checks the same for the whole dependency tree (a crate that re-exported
//! the core would pass the line above). When Zeca acts (W2, W4), his actions stay types of his own
//! that `app` maps to `core::policy` and runs through the ledger: this crate never needs the core.

use serde::Serialize;

/// One session, as Zeca may see it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Session {
    /// `claude`, `codex`…
    pub agent: String,
    /// The project's folder name.
    pub project: String,
    /// `working`, `approval`, `question`, `finished`…
    pub status: String,
    /// What it is doing now, in a few words (`Running cargo test`).
    pub doing: Option<String>,
    /// Its latest steps, newest last, as the island lists them.
    pub steps: Vec<String>,
    /// The card it waits on, if any: the tool and its target as the card shows them. `app` fills
    /// it with secrets already redacted; it is untrusted content (it may carry an injection).
    pub waiting: Option<(String, String)>,
}

/// The flock at one moment.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Flock {
    pub sessions: Vec<Session>,
}

/// Read-only access to a snapshot of the flock.
#[derive(Debug, Clone)]
pub struct FlockReader {
    flock: Flock,
}

impl FlockReader {
    pub fn new(flock: Flock) -> Self {
        Self { flock }
    }

    pub fn sessions(&self) -> &[Session] {
        &self.flock.sessions
    }

    /// The sessions waiting on a card: what Zeca may tell the user about, never answer.
    pub fn waiting(&self) -> impl Iterator<Item = &Session> {
        self.flock.sessions.iter().filter(|s| s.waiting.is_some())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_reader_shows_who_waits() {
        let flock = Flock {
            sessions: vec![
                Session {
                    agent: "claude".into(),
                    project: "vults".into(),
                    status: "approval".into(),
                    doing: None,
                    steps: Vec::new(),
                    waiting: Some(("Bash".into(), "Bash · cargo test".into())),
                },
                Session {
                    agent: "codex".into(),
                    project: "site".into(),
                    status: "working".into(),
                    ..Session::default()
                },
            ],
        };
        let r = FlockReader::new(flock);
        assert_eq!(r.sessions().len(), 2);
        let waiting: Vec<&str> = r.waiting().map(|s| s.project.as_str()).collect();
        assert_eq!(waiting, ["vults"]);
    }
}
