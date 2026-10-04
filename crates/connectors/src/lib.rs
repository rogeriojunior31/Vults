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
    pub level: Level,
    pub title: String,
    pub detail: String,
    pub url: Option<String>,
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
}

/// Every connector the app knows, by id.
pub fn all() -> Vec<Box<dyn Connector>> {
    vec![Box::new(github::GitHub)]
}
