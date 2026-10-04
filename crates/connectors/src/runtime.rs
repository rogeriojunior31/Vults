//! Polls enabled connectors at the pace they ask for, backs off on errors, waits out rate
//! limits, polls early on request when the news is old, and turns snapshot differences into
//! events.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use serde::Serialize;
use tokio::sync::{Mutex, Notify, mpsc, watch};
use tokio::time::Instant;

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

/// Where a connector's task stands, for [`Runtime::refresh_if_stale`].
#[derive(Debug, Default)]
struct Pace {
    /// Tokio's clock, not the wall's: tests drive it, and a wall clock change can't fake freshness.
    last_ok: Option<Instant>,
    /// The last poll failed: its wait (backoff, rate limit, user must act) is never cut short.
    failing: bool,
}

struct Handle {
    switch: watch::Sender<bool>,
    wake: Arc<Notify>,
    pace: Arc<std::sync::Mutex<Pace>>,
}

pub struct Runtime {
    state_dir: PathBuf,
    statuses: Arc<Mutex<BTreeMap<String, Status>>>,
    handles: BTreeMap<String, Handle>,
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
        let mut handles = BTreeMap::new();
        for c in connectors {
            let id = c.id().to_string();
            let on = enabled.get(&id).copied().unwrap_or(false);
            let (switch, rx) = watch::channel(on);
            let handle = Handle {
                switch,
                wake: Arc::new(Notify::new()),
                pace: Arc::default(),
            };
            let task = Task {
                connector: Arc::from(c),
                path: state_dir.join(format!("{id}.json")),
                statuses: statuses.clone(),
                events: events.clone(),
                wake: handle.wake.clone(),
                pace: handle.pace.clone(),
            };
            handles.insert(id, handle);
            tokio::spawn(task.run(rx));
        }
        Self {
            state_dir,
            statuses,
            handles,
        }
    }

    pub fn ids(&self) -> impl Iterator<Item = &str> {
        self.handles.keys().map(String::as_str)
    }

    pub fn set_enabled(&self, id: &str, on: bool) {
        if let Some(h) = self.handles.get(id) {
            let _ = h.switch.send(on);
        }
    }

    /// Polls `id` now if its last good poll is older than `max_age`. Does nothing while it is
    /// off, failing (the backoff wins) or polling already. Never blocks.
    pub fn refresh_if_stale(&self, id: &str, max_age: Duration) {
        let Some(h) = self.handles.get(id) else {
            return;
        };
        if !*h.switch.borrow() {
            return;
        }
        let stale = h
            .pace
            .lock()
            .is_ok_and(|p| !p.failing && p.last_ok.is_none_or(|t| t.elapsed() > max_age));
        if stale {
            // Wakes the task only while it sleeps between polls. No permit is stored, so a
            // request that lands mid-poll (or while off) never becomes a second poll.
            h.wake.notify_waiters();
        }
    }

    pub async fn statuses(&self) -> BTreeMap<String, Status> {
        let mut all = self.statuses.lock().await.clone();
        for (id, h) in &self.handles {
            all.entry(id.clone()).or_default().enabled = *h.switch.borrow();
        }
        all
    }
}

struct Task {
    connector: Arc<dyn Connector>,
    path: PathBuf,
    statuses: Arc<Mutex<BTreeMap<String, Status>>>,
    events: mpsc::Sender<Event>,
    wake: Arc<Notify>,
    pace: Arc<std::sync::Mutex<Pace>>,
}

impl Task {
    async fn run(self, mut enabled: watch::Receiver<bool>) {
        let id = self.connector.id().to_string();
        let mut backoff = self.connector.interval(&self.load().unwrap_or_default());
        loop {
            // Asleep while disabled: no polls, no timers.
            while !*enabled.borrow() {
                if enabled.changed().await.is_err() {
                    return;
                }
            }
            let result = self.connector.poll().await;
            if let Ok(mut p) = self.pace.lock() {
                p.failing = result.is_err();
                if result.is_ok() {
                    p.last_ok = Some(Instant::now());
                }
            }
            let wait = match result {
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
                    backoff = self.connector.interval(&snapshot);
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
            // Sleep, but wake at once if the switch flips (off stops, on polls right away) or
            // fresh news is wanted (`refresh_if_stale` checked it is due).
            tokio::select! {
                () = tokio::time::sleep(wait) => {}
                () = self.wake.notified() => {}
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
        fn interval(&self, _: &Snapshot) -> Duration {
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

    /// One pull request, its checks running for the first `pending` polls; GitHub's own pacing.
    struct Paced {
        polls: Arc<std::sync::Mutex<Vec<Instant>>>,
        pending: usize,
        fails: bool,
        /// How long a poll takes.
        takes: Duration,
    }

    impl Connector for Paced {
        fn id(&self) -> &'static str {
            "paced"
        }
        fn interval(&self, last: &Snapshot) -> Duration {
            crate::github::GitHub.interval(last)
        }
        fn poll(&self) -> Poll<'_> {
            Box::pin(async move {
                let n = {
                    let mut polls = self.polls.lock().unwrap();
                    polls.push(Instant::now());
                    polls.len() - 1
                };
                tokio::time::sleep(self.takes).await;
                if self.fails {
                    return Err(Error::Other("down".into()));
                }
                let ci = if n < self.pending { "PENDING" } else { "SUCCESS" };
                Ok(Snapshot::from([(
                    "pr:me/app#1".to_string(),
                    serde_json::json!({ "ci": ci }),
                )]))
            })
        }
        fn diff(&self, _: &Snapshot, _: &Snapshot) -> Vec<Event> {
            vec![]
        }
    }

    type Polls = Arc<std::sync::Mutex<Vec<Instant>>>;

    fn paced(name: &str, on: bool, pending: usize, fails: bool, takes: Duration) -> (Runtime, Polls) {
        let polls = Polls::default();
        let c = Paced {
            polls: polls.clone(),
            pending,
            fails,
            takes,
        };
        let (tx, _) = mpsc::channel(16);
        let enabled = BTreeMap::from([("paced".to_string(), on)]);
        (Runtime::start(vec![Box::new(c)], &enabled, dir(name), tx), polls)
    }

    fn count(polls: &Polls) -> usize {
        polls.lock().unwrap().len()
    }

    async fn at(start: Instant, secs: u64) {
        tokio::time::sleep_until(start + Duration::from_secs(secs)).await;
    }

    const MAX_AGE: Duration = Duration::from_secs(60);

    #[tokio::test(start_paused = true)]
    async fn sooner_while_checks_run_slower_when_idle() {
        let (_rt, polls) = paced("pace", true, 2, false, Duration::ZERO);
        at(Instant::now(), 800).await;
        let times = polls.lock().unwrap().clone();
        let gaps: Vec<u64> = times.windows(2).map(|w| (w[1] - w[0]).as_secs()).collect();
        // Polls 0 and 1 see checks running; poll 2 sees them done.
        assert_eq!(gaps, [60, 60, 300, 300]);
    }

    #[tokio::test(start_paused = true)]
    async fn refresh_only_when_older_than_max_age() {
        let start = Instant::now();
        let (rt, polls) = paced("stale", true, 0, false, Duration::ZERO);
        at(start, 30).await;
        assert_eq!(count(&polls), 1);
        rt.refresh_if_stale("paced", MAX_AGE);
        at(start, 31).await;
        assert_eq!(count(&polls), 1, "30 s old is fresh enough");

        at(start, 61).await;
        rt.refresh_if_stale("paced", MAX_AGE);
        rt.refresh_if_stale("paced", MAX_AGE);
        at(start, 62).await;
        assert_eq!(count(&polls), 2, "61 s old: one poll now, not two");
        assert_eq!((polls.lock().unwrap()[1] - start).as_secs(), 61);
        rt.refresh_if_stale("paced", MAX_AGE);
        at(start, 63).await;
        assert_eq!(count(&polls), 2, "just polled");
        // The refresh restarted the wait: the next poll is a full interval after it.
        at(start, 61 + 300 + 1).await;
        assert_eq!(count(&polls), 3);
        assert_eq!((polls.lock().unwrap()[2] - start).as_secs(), 361);
    }

    #[tokio::test(start_paused = true)]
    async fn no_refresh_while_backing_off() {
        let start = Instant::now();
        let (rt, polls) = paced("failing", true, 0, true, Duration::ZERO);
        at(start, 100).await;
        rt.refresh_if_stale("paced", MAX_AGE);
        at(start, 101).await;
        assert_eq!(count(&polls), 1, "the backoff wins over a refresh");
        // An error doubles the idle wait: 600 s.
        at(start, 601).await;
        assert_eq!(count(&polls), 2);
    }

    #[tokio::test(start_paused = true)]
    async fn no_refresh_while_off() {
        let start = Instant::now();
        let (rt, polls) = paced("refresh-off", false, 0, false, Duration::ZERO);
        rt.refresh_if_stale("paced", MAX_AGE);
        at(start, 10).await;
        assert_eq!(count(&polls), 0);
        // Nothing was kept for later either: switching on polls once, then waits.
        rt.set_enabled("paced", true);
        at(start, 20).await;
        assert_eq!(count(&polls), 1);
    }

    #[tokio::test(start_paused = true)]
    async fn no_second_poll_while_one_is_in_flight() {
        let start = Instant::now();
        let (rt, polls) = paced("in-flight", true, 0, false, Duration::from_secs(10));
        at(start, 5).await;
        rt.refresh_if_stale("paced", MAX_AGE);
        at(start, 20).await;
        assert_eq!(count(&polls), 1);
    }
}
