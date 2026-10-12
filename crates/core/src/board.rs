//! A connector's card on the island: what is open there right now (pull requests and their
//! checks, reviews waiting, default branches), as opposed to alerts, which are news.

use serde::Serialize;

use crate::{SafeUrl, State};

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub enum Group {
    /// The user's own open pull requests.
    Yours,
    /// Pull requests waiting for the user's review.
    ToReview,
    /// The default branch of the user's recent repositories.
    Branches,
    /// The latest release of the user's recent repositories: Room's Releases tab, never news.
    Releases,
}

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub enum Checks {
    Passing,
    Failing,
    Running,
}

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
#[cfg_attr(test, derive(ts_rs::TS))]
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
    /// A pull request's head commit as the last poll saw it: the merge takes that one only, never
    /// a commit pushed since, which the user has not seen green.
    pub head: Option<String>,
    pub facts: Facts,
}

/// What a row's detail in Room shows beyond its line, each part only when the connector said it.
#[derive(Serialize, Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct Facts {
    /// `owner/repo`, for the repository filter.
    pub repo: Option<String>,
    pub branch: Option<String>,
    pub diff: Option<RowDiff>,
    pub author: Option<String>,
    pub reviewers: Vec<Reviewer>,
    /// When it last changed, in Unix seconds.
    pub updated: Option<u64>,
    pub runs: Vec<CheckRun>,
    pub release: Option<Release>,
    /// Text the user can copy into a terminal; Vults never runs it.
    pub command: Option<String>,
}

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct RowDiff {
    pub added: u32,
    pub removed: u32,
    pub files: u32,
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct Reviewer {
    pub login: String,
    pub said: Said,
}

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub enum Said {
    Approved,
    Changes,
    Commented,
    Waiting,
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct CheckRun {
    pub name: String,
    pub state: CheckState,
}

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub enum CheckState {
    Passed,
    Failed,
    Running,
    Waiting,
    Skipped,
}

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub enum Release {
    Latest,
    Prerelease,
    Draft,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct BoardView {
    pub connector: String,
    pub rows: Vec<RowView>,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct RowView {
    pub item: String,
    pub group: Group,
    pub name: String,
    pub title: String,
    pub checks: Option<Checks>,
    pub review: Option<Verdict>,
    /// The row opens something when clicked.
    pub link: bool,
    /// The user's own pull request, approved and green: the card offers to merge it.
    pub merge: bool,
    pub facts: Facts,
}

/// Only the user's own pull request, approved, with its checks passing, is offered a merge, and
/// only such a row is merged (`Intent::MergeRow`): a page can't ask for any other.
fn mergeable(r: &Row) -> bool {
    r.group == Group::Yours
        && r.review == Some(Verdict::Approved)
        && r.checks == Some(Checks::Passing)
        && r.head.is_some()
}

/// The row to merge and its head commit, if it is one the card offers.
pub(crate) fn merge(state: &State, connector: &str, item: &str) -> Option<(String, String)> {
    let r = state
        .boards
        .get(connector)?
        .iter()
        .find(|r| r.item == item && mergeable(r))?;
    Some((r.item.clone(), r.head.clone()?))
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
                // A branch without checks has nothing to say; it stays in the state so its
                // alerts are not taken for gone.
                .filter(|r| r.group != Group::Branches || r.checks.is_some())
                .map(|r| RowView {
                    item: r.item.clone(),
                    group: r.group,
                    name: r.name.clone(),
                    title: r.title.clone(),
                    checks: r.checks,
                    review: r.review,
                    link: r.url.is_some(),
                    merge: mergeable(r),
                    facts: r.facts.clone(),
                })
                .collect(),
        })
        .collect()
}
