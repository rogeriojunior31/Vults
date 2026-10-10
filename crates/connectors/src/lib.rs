//! Connectors watch outside services and report what changed. Adding one is a file here that
//! implements [`Connector`], one line in [`all`], and one entry in `ui/src/connectors.ts`
//! (docs/CONNECTORS.md).
//!
//! A connector turns the service's state into a [`Snapshot`]; the runtime diffs it against the
//! last one it saw (saved to disk, so a restart does not replay old news) and only the
//! differences become events. The first poll ever only records a baseline.

pub mod github;
mod runtime;

use std::collections::BTreeMap;
use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

use serde::{Deserialize, Serialize};

pub use runtime::{Runtime, Status};

/// How loud an event is; the island colors it.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Level {
    Info,
    Ok,
    Warn,
    Error,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Event {
    pub connector: String,
    /// Stable per thing *and* per news, e.g. `pr:owner/repo#12:ci:FAILURE`.
    pub key: String,
    /// One story the news belongs to, e.g. `pr:owner/repo#12:ci`: a newer event of the same topic
    /// retires the older ones (checks passed after they failed on the same pull request).
    pub topic: Option<String>,
    pub level: Level,
    pub title: String,
    pub detail: String,
    pub url: Option<String>,
    /// What the news is, for the app to say it in the user's language; `title` is its English.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub story: Option<Story>,
}

/// A connector's news, by kind, with the names it is about.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Story {
    pub kind: StoryKind,
    /// The repository (`owner/repo#12`, `owner/repo`).
    pub name: String,
    /// The branch, for news about a default branch.
    pub branch: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum StoryKind {
    ReviewRequested,
    ChecksFailed,
    ChecksPassed,
    Approved,
    ChangesRequested,
}

/// What the runtime tells the app.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Update {
    Event(Event),
    /// The connector's card after a good poll, for connectors that have one; `None` once it is
    /// switched off.
    Board {
        connector: String,
        rows: Option<Vec<Row>>,
    },
}

/// One line of a connector's card on the island: something open the user may want to look at.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    /// The snapshot key it comes from (`pr:owner/repo#12`). Alerts keyed under it go with it.
    pub item: String,
    pub group: Group,
    /// Short and stable: `app#12`, `app`.
    pub name: String,
    pub title: String,
    pub checks: Option<Checks>,
    pub review: Option<Review>,
    pub url: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Group {
    /// The user's own open pull requests.
    Yours,
    /// Pull requests waiting for the user's review.
    ToReview,
    /// The default branch of the user's recent repositories.
    Branches,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Checks {
    Passing,
    Failing,
    Running,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Review {
    Approved,
    Changes,
}

/// What a connector last saw, item by item. Values are opaque to the runtime.
pub type Snapshot = BTreeMap<String, serde_json::Value>;

#[derive(Debug, Clone, PartialEq)]
pub enum Error {
    /// The tool it relies on is missing (e.g. `gh` not installed).
    Unavailable(String),
    /// Not logged in, or not allowed.
    Auth(String),
    /// The service asked us to slow down.
    RateLimited {
        retry_after: Duration,
    },
    Other(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Unavailable(m) | Error::Auth(m) | Error::Other(m) => f.write_str(m),
            Error::RateLimited { retry_after } => {
                write!(f, "rate limited, retrying in {}s", retry_after.as_secs())
            }
        }
    }
}

pub type Poll<'a> = Pin<Box<dyn Future<Output = Result<Snapshot, Error>> + Send + 'a>>;

pub trait Connector: Send + Sync {
    fn id(&self) -> &'static str;
    /// How long to wait after a good poll, from what it saw (`last`): sooner while something runs.
    fn interval(&self, last: &Snapshot) -> Duration;
    /// The service's current state.
    fn poll(&self) -> Poll<'_>;
    /// News between two snapshots of this connector.
    fn diff(&self, before: &Snapshot, after: &Snapshot) -> Vec<Event>;
    /// Its card on the island, from the latest snapshot; `None` when it has no card.
    fn board(&self, _snapshot: &Snapshot) -> Option<Vec<Row>> {
        None
    }
}

/// Every connector the app knows, by id.
pub fn all() -> Vec<Box<dyn Connector>> {
    vec![Box::new(github::GitHub)]
}
