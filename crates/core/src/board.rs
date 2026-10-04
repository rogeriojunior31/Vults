//! A connector's card on the island: what is open there right now (pull requests and their
//! checks, reviews waiting, default branches), as opposed to alerts, which are news.

use serde::Serialize;

use crate::{SafeUrl, State};

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Group {
    /// The user's own open pull requests.
    Yours,
    /// Pull requests waiting for the user's review.
    ToReview,
    /// The default branch of the user's recent repositories.
    Branches,
}

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Checks {
    Passing,
    Failing,
    Running,
}

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Verdict {
    Approved,
    Changes,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    /// What it is (`pr:owner/repo#12`); the alerts about it are keyed under it.
    pub item: String,
    pub group: Group,
    pub name: String,
    pub title: String,
    pub checks: Option<Checks>,
    pub review: Option<Verdict>,
    /// Already checked, as an alert's: the UI can only ask to open a row, never a URL.
    pub url: Option<SafeUrl>,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct BoardView {
    pub connector: String,
    pub rows: Vec<RowView>,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct RowView {
    pub item: String,
    pub group: Group,
    pub name: String,
    pub title: String,
    pub checks: Option<Checks>,
    pub review: Option<Verdict>,
    /// The row opens something when clicked.
    pub link: bool,
}

/// A connector's new card, or `None` once it is switched off. An item that left the card (a pull
/// request merged or closed, a review request withdrawn) takes its alerts with it.
pub(crate) fn set(state: &mut State, connector: String, rows: Option<Vec<Row>>) {
    let Some(rows) = rows else {
        state.boards.remove(&connector);
        return;
    };
    let gone: Vec<String> = state
        .boards
        .get(&connector)
        .into_iter()
        .flatten()
        .filter(|old| !rows.iter().any(|r| r.item == old.item))
        .map(|old| format!("{}:", old.item))
        .collect();
    state
        .alerts
        .retain(|a| a.connector != connector || !gone.iter().any(|item| a.key.starts_with(item)));
    state.boards.insert(connector, rows);
}

pub(crate) fn url(state: &State, connector: &str, item: &str) -> Option<SafeUrl> {
    state
        .boards
        .get(connector)?
        .iter()
        .find(|r| r.item == item)?
        .url
        .clone()
}

pub(crate) fn view(state: &State) -> Vec<BoardView> {
    state
        .boards
        .iter()
        .map(|(connector, rows)| BoardView {
            connector: connector.clone(),
            rows: rows
                .iter()
                .map(|r| RowView {
                    item: r.item.clone(),
                    group: r.group,
                    name: r.name.clone(),
                    title: r.title.clone(),
                    checks: r.checks,
                    review: r.review,
                    link: r.url.is_some(),
                })
                .collect(),
        })
        .collect()
}
