//! The action inbox shared by every surface. Opening an item never answers it.

use serde::Serialize;

use crate::{Attention, SessionRef, State, Status, board};

#[derive(Serialize, Clone, Debug, PartialEq)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct AttentionItem {
    pub level: Attention,
    pub source: AttentionSource,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(tag = "kind", rename_all = "kebab-case")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub enum AttentionSource {
    Request {
        request: String,
        session: SessionRef,
        tool: String,
        target: String,
        description: Option<String>,
        question: bool,
        skipped: bool,
    },
    Session {
        session: SessionRef,
    },
    Connection {
        connector: String,
        item: String,
    },
    Alert {
        key: String,
    },
    Chat {
        count: usize,
    },
    /// None means the proposal count could not be read; Memory owns the retry.
    Memory {
        count: Option<usize>,
    },
}

/// The same actionable rows belong in Connections' Inbox and Quick's Now.
pub fn connection_needs(r: &board::RowView) -> bool {
    r.group == board::Group::ToReview
        || (r.group == board::Group::Yours
            && (r.checks == Some(board::Checks::Failing)
                || r.review == Some(board::Verdict::Changes)
                || r.merge))
        || (r.group == board::Group::Branches && r.checks == Some(board::Checks::Failing))
}

pub(crate) fn view(state: &State, boards: &[board::BoardView]) -> Vec<AttentionItem> {
    let mut items = Vec::new();
    for p in &state.pending {
        if !state.sessions.get(&p.session).is_some_and(|s| crate::shows(s, p)) {
            continue;
        }
        items.push(AttentionItem {
            level: Attention::NeedsYou,
            source: AttentionSource::Request {
                request: p.request.0.clone(),
                session: SessionRef::from(&p.session),
                tool: p.tool.clone(),
                target: p.target.clone(),
                description: p
                    .ask
                    .description
                    .clone()
                    .or_else(|| p.questions.first().map(|q| q.question.clone())),
                question: !p.questions.is_empty(),
                skipped: p.skipped,
            },
        });
    }
    for s in state.ordered() {
        if state
            .pending
            .iter()
            .any(|p| p.session == s.key && crate::shows(s, p))
        {
            continue;
        }
        let level = s.status.attention();
        if matches!(level, Attention::NeedsYou | Attention::Failed)
            || (s.status == Status::Working && s.watch.level == Some(crate::silence::Silence::Loud))
        {
            items.push(AttentionItem {
                level: if level == Attention::Quiet {
                    Attention::Info
                } else {
                    level
                },
                source: AttentionSource::Session {
                    session: SessionRef::from(&s.key),
                },
            });
        }
    }
    for b in boards {
        for r in b.rows.iter().filter(|r| connection_needs(r)) {
            items.push(AttentionItem {
                level: if r.checks == Some(board::Checks::Failing)
                    || r.review == Some(board::Verdict::Changes)
                {
                    Attention::Failed
                } else {
                    Attention::Info
                },
                source: AttentionSource::Connection {
                    connector: b.connector.clone(),
                    item: r.item.clone(),
                },
            });
        }
    }
    for a in &state.alerts {
        if !matches!(a.level, crate::AlertLevel::Warn | crate::AlertLevel::Error)
            || boards.iter().any(|b| {
                b.connector == a.connector
                    && b.rows.iter().any(|r| a.key.starts_with(&format!("{}:", r.item)))
            })
        {
            continue;
        }
        items.push(AttentionItem {
            level: if a.level == crate::AlertLevel::Error {
                Attention::Failed
            } else {
                Attention::Info
            },
            source: AttentionSource::Alert { key: a.key.clone() },
        });
    }
    items.sort_by_key(|i| std::cmp::Reverse(i.level));
    items
}
