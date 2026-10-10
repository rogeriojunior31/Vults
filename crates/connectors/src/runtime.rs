//! Polls enabled connectors at the pace they ask for, backs off on errors, waits out rate
//! limits, polls early on request when the news is old, and turns snapshot differences into
//! events and, for a connector with a card, the card's rows.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use serde::Serialize;
use tokio::sync::{Mutex, Notify, mpsc, watch};
use tokio::time::Instant;

use crate::{Connector, Error, Snapshot, Update};

/// Errors double this (240 s, 480 s, …), whatever pace the connector keeps when all is well.
const BACKOFF_BASE: Duration = Duration::from_secs(120);
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
    /// When the last poll ended, good or not. Tokio's clock, not the wall's: tests drive it, and
    /// a wall clock change can't fake freshness.
    last_try: Option<Instant>,
    /// The service asked us to wait: a refresh never cuts that short.
    rate_limited: bool,
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
        events: mpsc::Sender<Update>,
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

    /// Polls `id` now if its last poll, good or failed, is older than `max_age`: fresh news, or
    /// an early retry after an error the user may have fixed (`gh auth login`, the network
    /// back). Does nothing while it is off, rate limited or polling already. Never blocks.
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
            .is_ok_and(|p| !p.rate_limited && p.last_try.is_none_or(|t| t.elapsed() > max_age));
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
    events: mpsc::Sender<Update>,
    wake: Arc<Notify>,
    pace: Arc<std::sync::Mutex<Pace>>,
}

impl Task {
    async fn run(self, mut enabled: watch::Receiver<bool>) {
        let id = self.connector.id().to_string();
        let mut backoff = BACKOFF_BASE;
        // A card was sent: switching off takes it away.
        let mut carded = false;
        loop {
            if carded && !*enabled.borrow() {
                carded = false;
                let _ = self
                    .events
                    .send(Update::Board {
                        connector: id.clone(),
                        rows: None,
                    })
                    .await;
            }
            // Asleep while disabled: no polls, no timers.
            while !*enabled.borrow() {
                if enabled.changed().await.is_err() {
                    return;
                }
            }
            let result = self.connector.poll().await;
            if let Ok(mut p) = self.pace.lock() {
                p.last_try = Some(Instant::now());
                p.rate_limited = matches!(result, Err(Error::RateLimited { .. }));
            }
            let wait = match result {
                Ok(snapshot) => {
                    // Switched off mid-poll: no news, but keep the snapshot as the baseline so
                    // switching back on does not replay it.
                    let still_on = *enabled.borrow();
                    if still_on && let Some(before) = &self.load() {
                        for e in self.connector.diff(before, &snapshot) {
                            let _ = self.events.send(Update::Event(e)).await;
                        }
                    }
                    // After the news: an item gone from the card takes its alerts with it.
                    if still_on && let Some(rows) = self.connector.board(&snapshot) {
                        carded = true;
                        let _ = self
                            .events
                            .send(Update::Board {
                                connector: id.clone(),
                                rows: Some(rows),
                            })
                            .await;
                    }
                    self.save(&snapshot);
                    self.status(&id, |s| {
                        s.last_ok = Some(now_secs());
                        s.error = None;
                        s.watching = snapshot.len();
                    })
                    .await;
                    backoff = BACKOFF_BASE;
                    self.connector.interval(&snapshot)
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
    use crate::{Event, Level, Poll, Row};
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
                topic: None,
                story: None,
            }]
        }
    }

    fn event(u: Option<Update>) -> Event {
        match u {
            Some(Update::Event(e)) => e,
            other => panic!("expected an event, got {other:?}"),
        }
    }

    /// A [`Counter`] with a card: one row, named after its poll count.
    struct Carded(Counter);

    impl Connector for Carded {
        fn id(&self) -> &'static str {
            "counter"
        }
        fn interval(&self, last: &Snapshot) -> Duration {
            self.0.interval(last)
        }
        fn poll(&self) -> Poll<'_> {
            self.0.poll()
        }
        fn diff(&self, before: &Snapshot, after: &Snapshot) -> Vec<Event> {
            self.0.diff(before, after)
        }
        fn board(&self, snapshot: &Snapshot) -> Option<Vec<Row>> {
            Some(vec![Row {
                item: "n".into(),
                group: crate::Group::Yours,
                name: snapshot["n"].to_string(),
                title: String::new(),
                checks: None,
                review: None,
                url: None,
            }])
        }
    }

    #[tokio::test]
    async fn a_card_follows_its_news_and_goes_when_switched_off() {
        let (tx, mut rx) = mpsc::channel(16);
        let on = BTreeMap::from([("counter".to_string(), true)]);
        let rt = Runtime::start(
            vec![Box::new(Carded(Counter(Arc::new(AtomicUsize::new(0)))))],
            &on,
            dir("carded"),
            tx,
        );
        let wait = Duration::from_secs(2);
        // The baseline poll has no news, only its card.
        let Some(Update::Board { rows: Some(rows), .. }) =
            tokio::time::timeout(wait, rx.recv()).await.unwrap()
        else {
            panic!("the first poll sends its card");
        };
        assert_eq!(rows[0].name, "0");
        let news = event(tokio::time::timeout(wait, rx.recv()).await.unwrap());
        assert_eq!(news.key, "n:1", "the news comes before its card");
        assert!(matches!(
            tokio::time::timeout(wait, rx.recv()).await.unwrap(),
            Some(Update::Board { rows: Some(_), .. })
        ));
        rt.set_enabled("counter", false);
        loop {
            match tokio::time::timeout(wait, rx.recv()).await.unwrap() {
                Some(Update::Board {
                    rows: None,
                    connector,
                }) => {
                    assert_eq!(connector, "counter");
                    break;
                }
                Some(_) => {}
                None => panic!("switching off takes the card away"),
            }
        }
    }

    fn dir(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("vults-connectors-{}-{name}", std::process::id()));
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
        let first = event(
            tokio::time::timeout(Duration::from_secs(2), rx.recv())
                .await
                .unwrap(),
        );
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
        let first = event(
            tokio::time::timeout(Duration::from_secs(2), rx.recv())
                .await
                .unwrap(),
        );
        assert_eq!(
            first.key, "n:1",
            "poll 0 matched the saved state, so it was not news"
        );
    }

    /// One pull request whose poll `n` answers `script[n]` (the last one repeats): its checks'
    /// state, or an error. GitHub's own pacing.
    struct Paced {
        polls: Polls,
        script: Vec<Result<&'static str, Error>>,
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
                let ci = self.script[n.min(self.script.len() - 1)].clone()?;
                Ok(Snapshot::from([(
                    "pr:me/app#1".to_string(),
                    serde_json::json!({ "ci": ci }),
                )]))
            })
        }
        fn diff(&self, before: &Snapshot, after: &Snapshot) -> Vec<Event> {
            if before == after {
                return vec![];
            }
            vec![Event {
                connector: "paced".into(),
                key: "changed".into(),
                level: Level::Info,
                title: "changed".into(),
                detail: String::new(),
                url: None,
                topic: None,
                story: None,
            }]
        }
    }

    type Polls = Arc<std::sync::Mutex<Vec<Instant>>>;

    fn paced(
        name: &str,
        on: bool,
        script: Vec<Result<&'static str, Error>>,
        takes: Duration,
    ) -> (Runtime, Polls, mpsc::Receiver<Update>) {
        let polls = Polls::default();
        let c = Paced {
            polls: polls.clone(),
            script,
            takes,
        };
        let (tx, rx) = mpsc::channel(16);
        let enabled = BTreeMap::from([("paced".to_string(), on)]);
        let rt = Runtime::start(vec![Box::new(c)], &enabled, dir(name), tx);
        (rt, polls, rx)
    }

    fn count(polls: &Polls) -> usize {
        polls.lock().unwrap().len()
    }

    /// Seconds between one poll and the next.
    fn gaps(polls: &Polls) -> Vec<u64> {
        let times = polls.lock().unwrap().clone();
        times.windows(2).map(|w| (w[1] - w[0]).as_secs()).collect()
    }

    async fn at(start: Instant, secs: u64) {
        tokio::time::sleep_until(start + Duration::from_secs(secs)).await;
    }

    const MAX_AGE: Duration = Duration::from_secs(60);
    const RUNNING: Result<&str, Error> = Ok("PENDING");
    const DONE: Result<&str, Error> = Ok("SUCCESS");

    fn down() -> Result<&'static str, Error> {
        Err(Error::Other("down".into()))
    }

    #[tokio::test(start_paused = true)]
    async fn sooner_while_checks_run_slower_when_idle() {
        let (_rt, polls, _) = paced("pace", true, vec![RUNNING, RUNNING, DONE], Duration::ZERO);
        at(Instant::now(), 800).await;
        // Polls 0 and 1 see checks running; poll 2 sees them done.
        assert_eq!(gaps(&polls), [60, 60, 300, 300]);
    }

    #[tokio::test(start_paused = true)]
    async fn error_backoff_is_the_same_whatever_the_pace() {
        for (name, first, last_gap) in [("backoff-running", RUNNING, 60), ("backoff-idle", DONE, 300)] {
            let start = Instant::now();
            let (_rt, polls, _) = paced(name, true, vec![first, down()], Duration::ZERO);
            at(start, last_gap + 240 + 480 + 1).await;
            assert_eq!(gaps(&polls), [last_gap, 240, 480], "{name}");
        }
    }

    #[tokio::test(start_paused = true)]
    async fn a_good_poll_resets_the_backoff() {
        let start = Instant::now();
        let (_rt, polls, _) = paced("backoff-reset", true, vec![down(), DONE, down()], Duration::ZERO);
        at(start, 240 + 300 + 240 + 1).await;
        assert_eq!(gaps(&polls), [240, 300, 240]);
    }

    #[tokio::test(start_paused = true)]
    async fn refresh_only_when_older_than_max_age() {
        let start = Instant::now();
        let (rt, polls, _) = paced("stale", true, vec![DONE], Duration::ZERO);
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
    async fn opening_retries_an_error_at_most_once_a_minute() {
        let auth = Err(Error::Auth("not logged in".into()));
        let missing = Err(Error::Unavailable("no gh".into()));
        for (name, error) in [
            ("retry-auth", auth),
            ("retry-unavailable", missing),
            ("retry-other", down()),
        ] {
            let start = Instant::now();
            let (rt, polls, _) = paced(name, true, vec![error, DONE], Duration::ZERO);
            at(start, 30).await;
            rt.refresh_if_stale("paced", MAX_AGE);
            at(start, 31).await;
            assert_eq!(count(&polls), 1, "{name}: tried 30 s ago");
            at(start, 61).await;
            rt.refresh_if_stale("paced", MAX_AGE);
            at(start, 62).await;
            assert_eq!(count(&polls), 2, "{name}: an open after 61 s tries again");
        }
    }

    #[tokio::test(start_paused = true)]
    async fn a_failed_retry_is_not_retried_within_a_minute() {
        let start = Instant::now();
        let (rt, polls, _) = paced("retry-fails", true, vec![down()], Duration::ZERO);
        at(start, 61).await;
        rt.refresh_if_stale("paced", MAX_AGE);
        at(start, 62).await;
        assert_eq!(count(&polls), 2);
        at(start, 100).await;
        rt.refresh_if_stale("paced", MAX_AGE);
        at(start, 101).await;
        assert_eq!(count(&polls), 2, "the retry failed 39 s ago");
        // The failed retry doubled the backoff: 240 s, then 480 s after the retry.
        at(start, 61 + 480 + 1).await;
        assert_eq!(count(&polls), 3);
        assert_eq!((polls.lock().unwrap()[2] - start).as_secs(), 541);
    }

    #[tokio::test(start_paused = true)]
    async fn a_rate_limit_is_never_cut_short() {
        let start = Instant::now();
        let limited = Err(Error::RateLimited {
            retry_after: Duration::from_secs(900),
        });
        let (rt, polls, _) = paced("rate-limited", true, vec![limited, DONE], Duration::ZERO);
        for secs in [61, 300, 899] {
            at(start, secs).await;
            rt.refresh_if_stale("paced", MAX_AGE);
        }
        at(start, 899).await;
        assert_eq!(count(&polls), 1);
        at(start, 901).await;
        assert_eq!(count(&polls), 2, "polled when the service said");
    }

    #[tokio::test(start_paused = true)]
    async fn no_refresh_while_off() {
        let start = Instant::now();
        let (rt, polls, _) = paced("refresh-off", false, vec![DONE], Duration::ZERO);
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
        let (rt, polls, _) = paced("in-flight", true, vec![DONE], Duration::from_secs(10));
        at(start, 5).await;
        rt.refresh_if_stale("paced", MAX_AGE);
        at(start, 20).await;
        assert_eq!(count(&polls), 1);
    }

    #[tokio::test(start_paused = true)]
    async fn switched_off_mid_poll_saves_but_stays_quiet() {
        let start = Instant::now();
        let state = dir("off-mid-poll");
        let failed = Snapshot::from([("pr:me/app#1".to_string(), serde_json::json!({ "ci": "FAILURE" }))]);
        std::fs::create_dir_all(&state).unwrap();
        std::fs::write(state.join("paced.json"), serde_json::to_string(&failed).unwrap()).unwrap();
        let polls = Polls::default();
        let c = Paced {
            polls: polls.clone(),
            script: vec![DONE],
            takes: Duration::from_secs(10),
        };
        let (tx, mut rx) = mpsc::channel(16);
        let on = BTreeMap::from([("paced".to_string(), true)]);
        let rt = Runtime::start(vec![Box::new(c)], &on, state.clone(), tx);
        at(start, 5).await;
        rt.set_enabled("paced", false);
        at(start, 20).await;
        assert_eq!(count(&polls), 1);
        assert!(
            rx.try_recv().is_err(),
            "no news from a connector just switched off"
        );
        let saved: Snapshot =
            serde_json::from_str(&std::fs::read_to_string(state.join("paced.json")).unwrap()).unwrap();
        assert_eq!(
            saved["pr:me/app#1"]["ci"], "SUCCESS",
            "but its snapshot is the new baseline"
        );
        // On again: the same SUCCESS is no news against that baseline.
        rt.set_enabled("paced", true);
        at(start, 45).await;
        assert_eq!(count(&polls), 2);
        assert!(rx.try_recv().is_err(), "nothing replayed after switching back on");
    }
}
