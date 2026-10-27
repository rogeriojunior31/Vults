//! Bind spoken project/agent labels to exact live session identities, never to a guessed id.
use std::collections::BTreeMap;
use vults_core::{AgentKind, SessionKey, State};
use vults_dictation::command::{
    names::{Name, NameKind, NameResolver},
    normalize,
};

pub(super) struct Catalog {
    pub resolver: NameResolver,
    entries: Vec<(Name, SessionKey)>,
    applications: Vec<(Name, vults_platform::applications::Application)>,
}

impl Catalog {
    pub fn new(state: &State, project: bool, replacements: &[(String, String)]) -> Self {
        Self::with_applications(state, project, replacements, &[])
    }

    pub fn with_applications(
        state: &State,
        project: bool,
        replacements: &[(String, String)],
        apps: &[vults_platform::applications::Application],
    ) -> Self {
        let mut groups: BTreeMap<String, Vec<_>> = BTreeMap::new();
        for session in state.sessions.values() {
            let id = if project {
                let Some(cwd) = &session.cwd else {
                    continue;
                };
                cwd.clone()
            } else {
                identity(&session.key)
            };
            groups.entry(id).or_default().push(session);
        }
        let mut entries = Vec::new();
        let mut labels = Vec::new();
        for (id, sessions) in groups {
            let session = sessions[0];
            let name = Name {
                id,
                kind: if project {
                    NameKind::Project
                } else {
                    NameKind::Agent
                },
                name: if project {
                    format!(
                        "{} / {}",
                        session.project,
                        session.cwd.as_deref().unwrap_or_default()
                    )
                } else {
                    format!(
                        "{} / {} / {}",
                        session.project,
                        agent_name(&session.key),
                        session.key.session_id
                    )
                },
            };
            for session in sessions {
                for label in [
                    session.project.clone(),
                    agent_name(&session.key),
                    session.key.agent.name().into(),
                ] {
                    labels.push((normalize(&label), name.name.clone()));
                }
            }
            entries.push((name, session.key.clone()));
        }
        let applications: Vec<_> = apps
            .iter()
            .map(|app| {
                let name = Name {
                    id: app.id.clone(),
                    name: format!("{} / {}", app.name, app.id),
                    kind: NameKind::Application,
                };
                for label in std::iter::once(&app.name).chain(app.aliases.iter()) {
                    labels.push((normalize(label), name.name.clone()));
                }
                (name, app.clone())
            })
            .collect();
        // Expand only full known labels, not another replacement or an arbitrary word in a name.
        let saved: Vec<_> = replacements
            .iter()
            .flat_map(|(from, to)| {
                let to = normalize(to);
                labels
                    .iter()
                    .filter(move |(label, _)| *label == to)
                    .map(move |(_, name)| (from.clone(), name.clone()))
            })
            .collect();
        labels.retain(|(label, _)| !saved.iter().any(|(from, _)| normalize(from) == *label));
        labels.extend(saved);
        let resolver = NameResolver::new(
            entries
                .iter()
                .map(|(name, _)| name.clone())
                .chain(applications.iter().map(|(name, _)| name.clone()))
                .collect(),
            &labels,
        );
        Self {
            resolver,
            entries,
            applications,
        }
    }

    pub fn application(&self, name: &Name) -> Option<vults_platform::applications::Application> {
        self.applications
            .iter()
            .find(|(current, _)| current == name)
            .map(|(_, app)| app.clone())
    }

    pub fn session(&self, name: &Name) -> Option<SessionKey> {
        self.entries
            .iter()
            .find(|(current, _)| {
                current.kind == name.kind
                    && current.id == name.id
                    && normalize(&current.name) == normalize(&name.name)
            })
            .map(|(_, key)| key.clone())
    }
}

fn identity(key: &SessionKey) -> String {
    // Serialization preserves separators in opaque ids and keeps same-id agents distinct.
    serde_json::to_string(key).expect("Session keys serialize")
}

fn agent_name(key: &SessionKey) -> String {
    match key.agent {
        AgentKind::Claude => "Claude Code",
        AgentKind::Codex => "Codex",
        AgentKind::OpenCode => "OpenCode",
        AgentKind::Qwen => "Qwen Code",
        AgentKind::Copilot => "Copilot CLI",
        AgentKind::Other => key.session_id.split_once('/').map_or("other", |(name, _)| name),
    }
    .into()
}

/// A clarification must not authorize a replacement session using the same visible name and key.
#[derive(Debug)]
pub(super) struct SessionBinding {
    key: SessionKey,
    started: std::time::Instant,
    cwd: Option<String>,
    terminal: Option<vults_protocol::Terminal>,
}

impl SessionBinding {
    pub fn capture(catalog: &Catalog, state: &State, name: &Name, terminal: bool) -> Option<Self> {
        let key = catalog.session(name)?;
        let session = state.sessions.get(&key)?;
        Some(Self {
            key,
            started: session.started,
            cwd: session.cwd.clone(),
            terminal: terminal.then(|| session.terminal.clone()),
        })
    }

    pub fn current(&self, state: &State) -> bool {
        state.sessions.get(&self.key).is_some_and(|session| {
            session.started == self.started
                && session.cwd == self.cwd
                && self
                    .terminal
                    .as_ref()
                    .is_none_or(|terminal| &session.terminal == terminal)
        })
    }
}
