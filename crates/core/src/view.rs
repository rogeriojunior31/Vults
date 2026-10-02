//! What the UI renders. Plain data, serialized to the webview as-is.

use serde::Serialize;

use crate::{Activity, AgentKind, AlertLevel, State, Status, Terminal, i18n};

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct ViewModel {
    pub sessions: Vec<SessionView>,
    pub approval: Option<ApprovalView>,
    pub alerts: Vec<AlertView>,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct AlertView {
    pub key: String,
    pub connector: String,
    pub level: AlertLevel,
    pub title: String,
    pub detail: String,
    /// The alert opens something when clicked.
    pub link: bool,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct SessionView {
    pub id: String,
    pub agent: AgentKind,
    /// Another tool's name (`AgentKind::Other`), taken from its session id: `<name>/<id>`.
    pub agent_name: Option<String>,
    pub project: String,
    /// The project folder, where the chat works when this session is in front.
    pub cwd: Option<String>,
    pub status: Status,
    pub activity: Option<Activity>,
    pub step: Option<String>,
    /// The latest steps, oldest first, for the island's step ticker.
    pub steps: Vec<String>,
    /// How many steps the session has taken so far.
    pub step_count: u32,
    pub subagents: u32,
    /// The question, the last reply or the error that goes with the status.
    pub note: Option<String>,
    /// The editor whose terminal the session runs in ("Cursor", "VS Code").
    pub editor: Option<&'static str>,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct ApprovalView {
    pub request: String,
    pub agent: AgentKind,
    /// The session that asked, to put it in front.
    pub session: String,
    pub project: String,
    pub tool: String,
    pub target: String,
    /// The agent's own words for the action ("Run the test suite").
    pub description: Option<String>,
    /// The whole command, when `target` had to cut it.
    pub full: Option<String>,
    /// Lines an edit adds and removes; both 0 when it is not an edit.
    pub added: u32,
    pub removed: u32,
    /// How many permissions wait, this one included.
    pub queue: usize,
}

impl State {
    pub fn view(&self) -> ViewModel {
        // Most recently active first.
        let mut sessions: Vec<_> = self.sessions.values().collect();
        sessions.sort_by_key(|s| std::cmp::Reverse(s.updated));
        ViewModel {
            sessions: sessions
                .into_iter()
                .map(|s| SessionView {
                    id: s.key.session_id.clone(),
                    agent: s.key.agent,
                    agent_name: (s.key.agent == AgentKind::Other)
                        .then(|| s.key.session_id.split_once('/').map(|(name, _)| name.to_string()))
                        .flatten(),
                    project: s.project.clone(),
                    cwd: s.cwd.clone(),
                    status: s.status,
                    activity: s.activity,
                    step: steps(self, s).pop(),
                    steps: steps(self, s),
                    step_count: s.step_count,
                    subagents: s.subagents,
                    note: s.note.clone(),
                    editor: editor(&s.terminal),
                })
                .collect(),
            approval: self.pending.front().map(|p| ApprovalView {
                request: p.request.0.clone(),
                agent: p.session.agent,
                session: p.session.session_id.clone(),
                project: self
                    .sessions
                    .get(&p.session)
                    .map(|s| s.project.clone())
                    .unwrap_or_default(),
                tool: p.tool.clone(),
                target: p.target.clone(),
                description: p.ask.description.clone(),
                full: p.ask.full.clone(),
                added: p.ask.added,
                removed: p.ask.removed,
                queue: self.pending.len(),
            }),
            alerts: self
                .alerts
                .iter()
                .map(|a| AlertView {
                    key: a.key.clone(),
                    connector: a.connector.clone(),
                    level: a.level,
                    title: a.title.clone(),
                    detail: a.detail.clone(),
                    link: a.url.is_some(),
                })
                .collect(),
        }
    }
}

/// VS Code and its forks all set `TERM_PROGRAM=vscode`; the binary behind git's askpass names the fork.
pub(crate) fn editor(t: &Terminal) -> Option<&'static str> {
    let env = |k: &str| t.env.get(k).map(String::as_str);
    if env("TERM_PROGRAM") != Some("vscode") && env("VSCODE_PID").is_none() {
        return None;
    }
    let binary = env("VSCODE_GIT_ASKPASS_NODE")
        .and_then(|p| p.rsplit('/').next())
        .unwrap_or_default()
        .to_ascii_lowercase();
    Some(if binary.contains("cursor") || env("CURSOR_TRACE_ID").is_some() {
        "Cursor"
    } else {
        "VS Code"
    })
}

/// The session's kept steps as text, oldest first; those a rule allowed say so.
fn steps(state: &State, s: &crate::Session) -> Vec<String> {
    // Step numbers count from 1; the kept ones are the last `steps.len()`.
    let first = s.step_count + 1 - s.steps.len() as u32;
    s.steps
        .iter()
        .enumerate()
        .map(|(i, st)| {
            let text = i18n::step(state.lang, st.activity, &st.tool, st.detail.as_deref());
            if s.ruled.contains(&(first + i as u32)) {
                i18n::ruled_step(state.lang, &text)
            } else {
                text
            }
        })
        .collect()
}
