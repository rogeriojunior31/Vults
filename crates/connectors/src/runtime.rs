//! Polls enabled connectors on their interval, backs off on errors, waits out rate limits,
//! and turns snapshot differences into events.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use serde::Serialize;
use tokio::sync::{Mutex, mpsc, watch};

use crate::{Connector, Error, Event, Snapshot};

const MAX_BACKOFF: Duration = Duration::from_secs(15 * 60);
/// Missing tool or login: the user has to act, so check again only now and then.
const NEEDS_USER: Duration = Duration::from_secs(10 * 60);

/// What the settings window shows per connector.
#[derive(Serialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub enabled: bool,
    /// Seconds since the epoch of the last successful poll.
    pub last_ok: Option<u64>,
    pub error: Option<String>,
    /// How many things it watches right now.
    pub watching: usize,
}

pub struct Runtime {
    state_dir: PathBuf,
    statuses: Arc<Mutex<BTreeMap<String, Status>>>,
    switches: BTreeMap<String, watch::Sender<bool>>,
}

impl std::fmt::Debug for Runtime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Runtime")
            .field("state_dir", &self.state_dir)
            .finish_non_exhaustive()
    }
}

impl Runtime {
    /// Starts one task per connector; each polls only while enabled. Must run inside Tokio.
    pub fn start(
        connectors: Vec<Box<dyn Connector>>,
        enabled: &BTreeMap<String, bool>,
        state_dir: PathBuf,
        events: mpsc::Sender<Event>,
    ) -> Self {
        let statuses = Arc::new(Mutex::new(BTreeMap::new()));
        let mut switches = BTreeMap::new();
        for c in connectors {
            let id = c.id().to_string();
            let on = enabled.get(&id).copied().unwrap_or(false);
            let (tx, rx) = watch::channel(on);
            switches.insert(id.clone(), tx);
            let task = Task {
                connector: Arc::from(c),
                path: state_dir.join(format!("{id}.json")),
                statuses: statuses.clone(),
                events: events.clone(),
            };
            tokio::spawn(task.run(rx));
        }
        Self {
            state_dir,
            statuses,
            switches,
        }
    }

    pub fn set_enabled(&self, id: &str, on: bool) {
        if let Some(tx) = self.switches.get(id) {
            let _ = tx.send(on);
        }
    }

    pub async fn statuses(&self) -> BTreeMap<String, Status> {
        let mut all = self.statuses.lock().await.clone();
        for (id, tx) in &self.switches {
            all.entry(id.clone()).or_default().enabled = *tx.borrow();
        }
        all
    }
}

struct Task {
    connector: Arc<dyn Connector>,
    path: PathBuf,
    statuses: Arc<Mutex<BTreeMap<String, Status>>>,
    events: mpsc::Sender<Event>,
}

impl Task {
    async fn run(self, mut enabled: watch::Receiver<bool>) {
        let id = self.connector.id().to_string();
        let mut backoff = self.connector.interval();
        loop {
            // Asleep while disabled: no polls, no timers.
            while !*enabled.borrow() {
                if enabled.changed().await.is_err() {
                    return;
                }
            }
            let wait = match self.connector.poll().await {
                Ok(snapshot) => {
                    let before = self.load();
                    if let Some(before) = &before {
                        for e in self.connector.diff(before, &snapshot) {
                            let _ = self.events.send(e).await;
                        }
                    }
                    self.save(&snapshot);
                    self.status(&id, |s| {
                        s.last_ok = Some(now_secs());
                        s.error = None;
                        s.watching = snapshot.len();
                    })
                    .await;
                    backoff = self.connector.interval();
                    backoff
                }
                Err(err) => {
                    self.status(&id, |s| s.error = Some(err.to_string())).await;
                    match err {
                        Error::RateLimited { retry_after } => retry_after,
                        Error::Unavailable(_) | Error::Auth(_) => NEEDS_USER,
                        Error::Other(_) => {
                            backoff = (backoff * 2).min(MAX_BACKOFF);
                            backoff
                        }
                    }
                }
            };
            // Sleep, but wake at once if the switch flips (off stops, on polls right away).
            tokio::select! {
                () = tokio::time::sleep(wait) => {}
                r = enabled.changed() => if r.is_err() { return },
            }
        }
    }

    fn load(&self) -> Option<Snapshot> {
        let text = std::fs::read_to_string(&self.path).ok()?;
        serde_json::from_str(&text).ok()
    }

    fn save(&self, snapshot: &Snapshot) {
        if let Some(dir) = self.path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Ok(text) = serde_json::to_string(snapshot) {
            let temp = self.path.with_extension("json.tmp");
            if std::fs::write(&temp, text).is_ok() {
                let _ = std::fs::rename(&temp, &self.path);
            }
        }
    }

    async fn status(&self, id: &str, edit: impl FnOnce(&mut Status)) {
        edit(self.statuses.lock().await.entry(id.to_string()).or_default());
    }
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::{Level, Poll};
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// Reports `n` (its poll count) as its only item; news when it changes.
    struct Counter(Arc<AtomicUsize>);

    impl Connector for Counter {
        fn id(&self) -> &'static str {
            "counter"
        }
        fn interval(&self) -> Duration {
            Duration::from_millis(20)
        }
        fn poll(&self) -> Poll<'_> {
            let n = self.0.fetch_add(1, Ordering::SeqCst);
            Box::pin(async move { Ok(Snapshot::from([("n".to_string(), serde_json::json!(n))])) })
        }
        fn diff(&self, before: &Snapshot, after: &Snapshot) -> Vec<Event> {
            if before == after {
                return vec![];
            }
            vec![Event {
                connector: "counter".into(),
                key: format!("n:{}", after["n"]),
                level: Level::Info,
                title: "changed".into(),
                detail: String::new(),
                url: None,
            }]
        }
    }

    fn dir(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("vultures-ai-connectors-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    #[tokio::test]
    async fn baseline_first_then_news_and_nothing_while_off() {
        let polls = Arc::new(AtomicUsize::new(0));
        let (tx, mut rx) = mpsc::channel(16);
        let off = BTreeMap::from([("counter".to_string(), false)]);
        let rt = Runtime::start(vec![Box::new(Counter(polls.clone()))], &off, dir("off"), tx);

        tokio::time::sleep(Duration::from_millis(80)).await;
        assert_eq!(
            polls.load(Ordering::SeqCst),
            0,
            "a disabled connector never polls"
        );

        rt.set_enabled("counter", true);
        // Poll 0 is the baseline: no event. Poll 1 differs: the first event is n:1.
        let first = tokio::time::timeout(Duration::from_secs(2), rx.recv())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(first.key, "n:1");
        let status = &rt.statuses().await["counter"];
        assert!(status.enabled && status.last_ok.is_some() && status.error.is_none());

        rt.set_enabled("counter", false);
        tokio::time::sleep(Duration::from_millis(60)).await;
        let stopped_at = polls.load(Ordering::SeqCst);
        tokio::time::sleep(Duration::from_millis(80)).await;
        assert_eq!(
            polls.load(Ordering::SeqCst),
            stopped_at,
            "switching off stops polling"
        );
    }

    #[tokio::test]
    async fn a_restart_does_not_replay_news() {
        let state = dir("restart");
        let snap = |n: u64| Snapshot::from([("n".to_string(), serde_json::json!(n))]);
        std::fs::create_dir_all(&state).unwrap();
        // What a previous run saw last: n = 0, which is also what the first poll returns.
        std::fs::write(
            state.join("counter.json"),
            serde_json::to_string(&snap(0)).unwrap(),
        )
        .unwrap();
        let (tx, mut rx) = mpsc::channel(16);
        let on = BTreeMap::from([("counter".to_string(), true)]);
        let _rt = Runtime::start(
            vec![Box::new(Counter(Arc::new(AtomicUsize::new(0))))],
            &on,
            state,
            tx,
        );
        let first = tokio::time::timeout(Duration::from_secs(2), rx.recv())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            first.key, "n:1",
            "poll 0 matched the saved state, so it was not news"
        );
    }
}
