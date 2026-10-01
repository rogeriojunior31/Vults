//! What the UI renders. Plain data, serialized to the webview as-is.

use serde::Serialize;

use crate::{Activity, AgentKind, AlertLevel, State, Status, i18n};

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
}

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct ApprovalView {
    pub request: String,
    pub agent: AgentKind,
    pub project: String,
    pub tool: String,
    pub target: String,
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
                    project: s.project.clone(),
                    cwd: s.cwd.clone(),
                    status: s.status,
                    activity: s.activity,
                    step: s
                        .steps
                        .back()
                        .map(|st| i18n::step(self.lang, st.activity, &st.tool, st.detail.as_deref())),
                    steps: s
                        .steps
                        .iter()
                        .map(|st| i18n::step(self.lang, st.activity, &st.tool, st.detail.as_deref()))
                        .collect(),
                    step_count: s.step_count,
                    subagents: s.subagents,
                })
                .collect(),
            approval: self.pending.as_ref().map(|p| ApprovalView {
                request: p.request.0.clone(),
                agent: p.session.agent,
                project: self
                    .sessions
                    .get(&p.session)
                    .map(|s| s.project.clone())
                    .unwrap_or_default(),
                tool: p.tool.clone(),
                target: p.target.clone(),
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
