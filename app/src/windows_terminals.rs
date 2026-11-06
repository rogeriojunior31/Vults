//! Process creation tokens are frozen on session observation; window discovery runs off the loop.
use std::collections::HashMap;
use std::time::Instant;
use vults_core::{SessionKey, State, ViewModel};
use vults_platform::jump::Anchor;
use vults_protocol::Terminal;

#[derive(Clone, Debug)]
pub(crate) struct Candidate {
    key: SessionKey,
    started: Instant,
    terminal: Terminal,
    anchor: Anchor,
}
pub(crate) type Report = Vec<(Candidate, bool)>;
#[derive(Debug)]
struct Entry {
    started: Instant,
    observed: Instant,
    terminal: Terminal,
    anchor: Option<Anchor>,
    available: bool,
}
#[derive(Default, Debug)]
pub(crate) struct Routes(HashMap<SessionKey, Entry>);
impl Routes {
    pub fn observe(&mut self, state: &State) {
        self.0
            .retain(|key, _| state.sessions.get(key).is_some_and(|s| !s.process_gone));
        for (key, session) in &state.sessions {
            if session.process_gone {
                continue;
            }
            let entry = self.0.entry(key.clone()).or_insert_with(|| Entry {
                started: session.started,
                observed: session.event_at,
                terminal: session.terminal.clone(),
                anchor: session.terminal.pid.and_then(Anchor::capture),
                available: false,
            });
            if entry.started != session.started || entry.terminal.pid != session.terminal.pid {
                *entry = Entry {
                    started: session.started,
                    observed: session.event_at,
                    terminal: session.terminal.clone(),
                    anchor: session.terminal.pid.and_then(Anchor::capture),
                    available: false,
                };
            } else {
                if entry.terminal != session.terminal {
                    entry.available = false;
                    entry.terminal = session.terminal.clone();
                }
                if entry.anchor.is_none() && entry.observed != session.event_at {
                    entry.anchor = session.terminal.pid.and_then(Anchor::capture);
                }
                entry.observed = session.event_at;
            }
        }
    }
    pub fn candidates(&self) -> Vec<Candidate> {
        self.0
            .iter()
            .filter_map(|(key, e)| {
                Some(Candidate {
                    key: key.clone(),
                    started: e.started,
                    terminal: e.terminal.clone(),
                    anchor: e.anchor?,
                })
            })
            .collect()
    }
    pub fn apply(&mut self, report: Report) {
        for (candidate, available) in report {
            if let Some(entry) = self.0.get_mut(&candidate.key)
                && entry.started == candidate.started
                && entry.terminal == candidate.terminal
                && entry.anchor == Some(candidate.anchor)
            {
                entry.available = available;
            }
        }
    }
    pub fn anchor(&self, terminal: &Terminal) -> Option<Anchor> {
        let mut own = self
            .0
            .values()
            .filter(|e| e.available && e.terminal == *terminal)
            .filter_map(|e| e.anchor);
        let first = own.next()?;
        own.all(|anchor| anchor == first).then_some(first)
    }
    pub fn decorate(&self, view: &mut ViewModel) {
        for row in &mut view.sessions {
            row.raise = self
                .0
                .get(&SessionKey {
                    agent: row.agent,
                    session_id: row.id.clone(),
                })
                .is_some_and(|entry| entry.available);
        }
    }
}
pub(crate) fn scan(candidates: Vec<Candidate>) -> Report {
    let targets: Vec<_> = candidates
        .iter()
        .map(|candidate| (candidate.anchor, candidate.terminal.clone()))
        .collect();
    candidates
        .into_iter()
        .zip(vults_platform::jump::availability(&targets))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn late_window_discovery_cannot_reenable_a_replaced_session_or_project() {
        let key = SessionKey {
            agent: vults_core::AgentKind::Codex,
            session_id: "window-fixture".into(),
        };
        let started = Instant::now();
        let terminal = Terminal {
            pid: Some(std::process::id()),
            cwd: Some("C:\\Alpha".into()),
            ..Default::default()
        };
        let anchor = Anchor::capture(std::process::id()).unwrap();
        let candidate = Candidate {
            key: key.clone(),
            started,
            terminal: terminal.clone(),
            anchor,
        };
        let mut routes = Routes(HashMap::from([(
            key.clone(),
            Entry {
                started,
                observed: started,
                terminal: terminal.clone(),
                anchor: Some(anchor),
                available: false,
            },
        )]));
        routes.apply(vec![(candidate.clone(), true)]);
        assert_eq!(routes.anchor(&terminal), Some(anchor));
        let entry = routes.0.get_mut(&key).unwrap();
        entry.started += std::time::Duration::from_secs(1);
        entry.available = false;
        routes.apply(vec![(candidate.clone(), true)]);
        assert!(routes.anchor(&terminal).is_none());
        let entry = routes.0.get_mut(&key).unwrap();
        entry.started = started;
        entry.terminal.cwd = Some("C:\\Beta".into());
        routes.apply(vec![(candidate, true)]);
        assert!(routes.anchor(&terminal).is_none());
    }
}
