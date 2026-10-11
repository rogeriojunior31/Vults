//! The network gate (ADR 0019; docs/safety.md): every request the app makes on its own, without a
//! click at that moment, asks here first. Each feature has the user's switch, a budget over the
//! last 24 hours, and a local trace of each request (a `request` span). Today: the connectors'
//! polls, the Codex usage read, Zeca's web tools, the phone's relay, the journal's daily
//! summary, the memory's suggestions and Zeca's sleep. Nothing of the trace leaves the machine. `scripts/check-network.sh` keeps any other
//! way out from appearing unlisted.

use std::collections::{BTreeMap, VecDeque};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use serde_json::json;
use tauri::AppHandle;
use vults_store::traces::Span;

/// Requests a connector may make in any 24 hours: the GitHub poll makes at most ~1,500 (every
/// minute while checks run), so only a runaway loop meets it.
pub const CONNECTOR_BUDGET: usize = 3000;
/// The Codex usage read, every 5 minutes at most: 288 a day.
pub const USAGE: &str = "codex-usage";
pub const USAGE_BUDGET: usize = 400;
/// Zeca's web tools (Z7): searches and page reads the model asks for in a chat turn.
pub const WEB: &str = "web";
/// A busy day of questions; only a model looping on its tools meets it.
pub const WEB_BUDGET: usize = 200;
/// The phone's relay: a long poll every 25 s per paired phone is about 3,500 a day, plus the
/// cards sent; this covers two phones with room for a busy day.
pub const PHONE: &str = "phone";
pub const PHONE_BUDGET: usize = 10_000;
/// Zeca's journal, each past day summed up by a small model (M4): one request a day, a few more
/// after days away.
pub const JOURNAL: &str = "journal-summary";
pub const JOURNAL_BUDGET: usize = 8;
/// Memory suggestions (M6): a small model reads a finished conversation; one per conversation.
pub const SUGGEST: &str = "memory-suggest";
pub const SUGGEST_BUDGET: usize = 20;
/// Zeca's sleep (M8): one night a day; a few more for a day the call failed.
pub const SLEEP: &str = "sleep";
pub const SLEEP_BUDGET: usize = 4;
const DAY_MS: i64 = 24 * 3600 * 1000;
const DAY: Duration = Duration::from_secs(24 * 3600);

/// Requests one feature may make in any 24 hours. A feature not named here has none: it must
/// choose its budget before its first request.
pub fn budget(feature: &str) -> usize {
    match feature {
        USAGE => USAGE_BUDGET,
        WEB => WEB_BUDGET,
        PHONE => PHONE_BUDGET,
        JOURNAL => JOURNAL_BUDGET,
        SUGGEST => SUGGEST_BUDGET,
        SLEEP => SLEEP_BUDGET,
        "github" => CONNECTOR_BUDGET,
        _ => 0,
    }
}

/// Is `feature` switched on in these settings? The gate asks before every request.
pub fn switched_on(s: &crate::settings::Settings, feature: &str) -> bool {
    if feature == USAGE {
        s.codex_usage.unwrap_or(false)
    } else if feature == WEB {
        s.web
    } else if feature == PHONE {
        s.phone
    } else if feature == JOURNAL {
        s.memory && s.journal_summary
    } else if feature == SUGGEST {
        s.memory && s.memory_suggest
    } else if feature == SLEEP {
        s.memory && s.sleep
    } else {
        s.connectors.get(feature).copied().unwrap_or(false)
    }
}

pub struct NetGate {
    /// Is this feature (a connector, or the usage read) switched on by the user?
    switch: Box<dyn Fn(&str) -> bool + Send + Sync>,
    budget: Box<dyn Fn(&str) -> usize + Send + Sync>,
    /// When each connector's recent requests went out, on the monotonic clock: a
    /// wall clock set back or forward never resets the budget or locks a connector out.
    sent: Mutex<BTreeMap<String, VecDeque<Instant>>>,
    /// Where each request's span goes.
    trace: Box<dyn Fn(Span) + Send + Sync>,
}

impl NetGate {
    pub fn new(
        switch: impl Fn(&str) -> bool + Send + Sync + 'static,
        budget: impl Fn(&str) -> usize + Send + Sync + 'static,
        seed: BTreeMap<String, Vec<i64>>,
        trace: impl Fn(Span) + Send + Sync + 'static,
    ) -> Self {
        // The traces' wall-clock times, once, as instants on this run's clock.
        let (now, now_ms) = (Instant::now(), crate::traces::now_ms());
        let at = |t: i64| now.checked_sub(Duration::from_millis(u64::try_from(now_ms - t).unwrap_or(0)));
        let sent = seed
            .into_iter()
            .map(|(k, v)| (k, v.into_iter().filter_map(at).collect()))
            .collect();
        Self {
            switch: Box::new(switch),
            budget: Box::new(budget),
            sent: Mutex::new(sent),
            trace: Box::new(trace),
        }
    }

    fn allow_at(&self, connector: &str, now: Instant) -> bool {
        if !(self.switch)(connector) {
            return false;
        }
        let Ok(mut sent) = self.sent.lock() else {
            return false;
        };
        let times = sent.entry(connector.to_owned()).or_default();
        // Whatever their order (a seed, a clock): only the last 24 hours count.
        times.retain(|t| now.duration_since(*t) < DAY);
        let budget = (self.budget)(connector);
        if times.len() >= budget {
            tracing::warn!(connector, budget, "daily request budget spent");
            return false;
        }
        times.push_back(now);
        true
    }
}

impl vults_connectors::Gate for NetGate {
    fn allow(&self, connector: &str) -> bool {
        self.allow_at(connector, Instant::now())
    }

    fn done(&self, connector: &str, ok: bool, took: Duration) {
        let end_ms = crate::traces::now_ms();
        let took_ms = i64::try_from(took.as_millis()).unwrap_or(0);
        (self.trace)(request_span(connector, ok, end_ms - took_ms, end_ms));
    }
}

/// The app's one gate, built on first use with the user's switches and the last day's requests.
pub fn shared(app: &AppHandle) -> Arc<NetGate> {
    static GATE: OnceLock<Arc<NetGate>> = OnceLock::new();
    GATE.get_or_init(|| {
        let mut features: Vec<&str> = vults_connectors::all().iter().map(|c| c.id()).collect();
        features.push(USAGE);
        features.push(WEB);
        features.push(PHONE);
        features.push(JOURNAL);
        features.push(SUGGEST);
        features.push(SLEEP);
        let handle = app.clone();
        Arc::new(NetGate::new(
            move |feature| {
                use tauri::Manager;
                handle
                    .state::<crate::settings::SettingsState>()
                    .0
                    .lock()
                    .is_ok_and(|s| switched_on(&s, feature))
            },
            budget,
            seed(&features),
            |span| crate::writer::send(crate::writer::Job::Span(span)),
        ))
    })
    .clone()
}

/// The name of a connector's request spans; also how the budget is counted back at start.
pub fn span_name(connector: &str) -> String {
    format!("request {connector}")
}

fn request_span(connector: &str, ok: bool, start_ms: i64, end_ms: i64) -> Span {
    Span {
        trace: crate::traces::new_id(),
        span: crate::traces::new_id(),
        parent: None,
        name: span_name(connector),
        kind: "request".into(),
        start_ms,
        end_ms,
        status: if ok { "ok" } else { "error" }.into(),
        attrs: json!({ "vults.feature": feature(connector), "vults.target": connector }),
    }
}

fn feature(key: &str) -> &'static str {
    match key {
        USAGE => "usage",
        WEB => "web",
        PHONE => "phone",
        JOURNAL => "journal",
        SUGGEST => "memory",
        SLEEP => "sleep",
        _ => "connectors",
    }
}

/// Each feature's requests of the last 24 hours, read back from the traces, so a restart does not
/// give a fresh budget.
pub fn seed(features: &[&str]) -> BTreeMap<String, Vec<i64>> {
    let since = crate::traces::now_ms() - DAY_MS;
    let Ok(store) = vults_store::Store::open(&crate::history::dir()) else {
        return BTreeMap::new();
    };
    features
        .iter()
        .map(|c| {
            (
                c.to_string(),
                store
                    .span_starts("request", &span_name(c), since)
                    .unwrap_or_default(),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gate(on: bool, budget: usize, seed: BTreeMap<String, Vec<i64>>) -> (NetGate, Arc<Mutex<Vec<Span>>>) {
        let spans = Arc::new(Mutex::new(Vec::new()));
        let sink = spans.clone();
        let g = NetGate::new(
            move |_| on,
            move |_| budget,
            seed,
            move |s| sink.lock().unwrap().push(s),
        );
        (g, spans)
    }

    fn at(start: Instant, ms: u64) -> Instant {
        start + Duration::from_millis(ms)
    }

    #[test]
    fn with_every_feature_off_nothing_goes_out() {
        let (g, spans) = gate(false, CONNECTOR_BUDGET, BTreeMap::new());
        let start = Instant::now();
        for t in 0..100 {
            assert!(!g.allow_at("github", at(start, t)));
        }
        assert!(spans.lock().unwrap().is_empty());
    }

    #[test]
    fn the_budget_holds_over_any_24_hours() {
        let (g, _) = gate(true, 3, BTreeMap::new());
        let s = Instant::now();
        assert!(g.allow_at("github", at(s, 0)));
        assert!(g.allow_at("github", at(s, 10)));
        assert!(g.allow_at("github", at(s, 20)));
        assert!(!g.allow_at("github", at(s, 30)), "the fourth within a day");
        assert!(
            g.allow_at("vercel", at(s, 30)),
            "each connector counts on its own"
        );
        let day = DAY_MS as u64;
        assert!(g.allow_at("github", at(s, day + 1)), "the first one is a day old");
        assert!(!g.allow_at("github", at(s, day + 2)));
    }

    #[test]
    fn a_restart_counts_the_last_days_requests() {
        let now_ms = crate::traces::now_ms();
        let seed = BTreeMap::from([(
            "github".to_string(),
            vec![now_ms - 2000, now_ms - 1000, now_ms - DAY_MS - 5],
        )]);
        let (g, _) = gate(true, 3, seed);
        let s = Instant::now();
        assert!(g.allow_at("github", s), "the one older than a day does not count");
        assert!(!g.allow_at("github", at(s, 1)));
    }

    /// The real gate in front of the real connector runtime and the real usage loop, for a whole
    /// simulated day: with every switch off, neither a poll nor a read starts (ADR 0019's
    /// consequence: the machine's count, not only the gate's).
    #[tokio::test(start_paused = true)]
    async fn a_day_with_every_feature_off_makes_no_request() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use vults_connectors::{Connector, Event, Poll, Runtime, Snapshot};

        struct Counting(Arc<AtomicUsize>);
        impl Connector for Counting {
            fn id(&self) -> &'static str {
                "counting"
            }
            fn interval(&self, _: &Snapshot) -> Duration {
                Duration::from_secs(60)
            }
            fn poll(&self) -> Poll<'_> {
                self.0.fetch_add(1, Ordering::SeqCst);
                Box::pin(async { Ok(Snapshot::new()) })
            }
            fn diff(&self, _: &Snapshot, _: &Snapshot) -> Vec<Event> {
                Vec::new()
            }
        }

        let (polls, reads) = (Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(0)));
        let (g, spans) = gate(false, CONNECTOR_BUDGET, BTreeMap::new());
        let g = Arc::new(g);
        let (tx, _rx) = tokio::sync::mpsc::channel(8);
        let on = BTreeMap::from([("counting".to_string(), true)]);
        let dir = std::env::temp_dir().join(format!("vults-gate-{}", std::process::id()));
        let _rt = Runtime::start(vec![Box::new(Counting(polls.clone()))], &on, dir, tx, g.clone());
        let counted = reads.clone();
        let read = move || {
            counted.fetch_add(1, Ordering::SeqCst);
            async { Ok(Vec::new()) }
        };
        let wake = tokio::sync::Notify::new();
        let usage = crate::usage::run(&*g, || false, read, |_| {}, &wake);
        assert!(
            tokio::time::timeout(DAY, usage).await.is_err(),
            "the loop runs all day"
        );
        assert_eq!(
            (polls.load(Ordering::SeqCst), reads.load(Ordering::SeqCst)),
            (0, 0)
        );
        assert!(spans.lock().unwrap().is_empty());
    }

    /// Switched on, the usage loop reads every 5 minutes, each read traced, within its budget.
    #[tokio::test(start_paused = true)]
    async fn the_usage_read_goes_through_the_gate() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let spans = Arc::new(Mutex::new(Vec::new()));
        let sink = spans.clone();
        let g = NetGate::new(
            |f| f == USAGE,
            budget,
            BTreeMap::new(),
            move |s| sink.lock().unwrap().push(s),
        );
        let reads = Arc::new(AtomicUsize::new(0));
        let counted = reads.clone();
        let read = move || {
            counted.fetch_add(1, Ordering::SeqCst);
            async { Ok(Vec::new()) }
        };
        let wake = tokio::sync::Notify::new();
        let _ = tokio::time::timeout(DAY, crate::usage::run(&g, || false, read, |_| {}, &wake)).await;
        let n = reads.load(Ordering::SeqCst);
        assert_eq!(n, 24 * 12 + 1, "one at the start, then one every 5 minutes");
        assert!(n <= USAGE_BUDGET);
        let spans = spans.lock().unwrap();
        assert_eq!(spans.len(), n);
        assert_eq!(spans[0].name, "request codex-usage");
        assert_eq!(spans[0].attrs["vults.feature"], "usage");
    }

    /// Off, the usage read never starts; switched on, a wake reads at once, not after the wait.
    #[tokio::test(start_paused = true)]
    async fn the_usage_switch_is_heard_at_once() {
        use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
        let on = Arc::new(AtomicBool::new(false));
        let switch = on.clone();
        let g = NetGate::new(
            move |_| switch.load(Ordering::SeqCst),
            budget,
            BTreeMap::new(),
            |_| {},
        );
        let reads = Arc::new(AtomicUsize::new(0));
        let counted = reads.clone();
        let read = move || {
            counted.fetch_add(1, Ordering::SeqCst);
            async { Ok(Vec::new()) }
        };
        let wake = tokio::sync::Notify::new();
        let run = crate::usage::run(&g, || false, read, |_| {}, &wake);
        tokio::pin!(run);
        let _ = tokio::time::timeout(Duration::from_secs(3600), &mut run).await;
        assert_eq!(reads.load(Ordering::SeqCst), 0, "off: no read in an hour");
        on.store(true, Ordering::SeqCst);
        wake.notify_one();
        let _ = tokio::time::timeout(Duration::from_secs(1), &mut run).await;
        assert_eq!(reads.load(Ordering::SeqCst), 1, "on: read within a second");
    }

    #[test]
    fn each_switch_opens_only_its_feature() {
        let mut s = crate::settings::Settings::default();
        assert!(!switched_on(&s, USAGE) && !switched_on(&s, "github"));
        s.codex_usage = Some(true);
        assert!(switched_on(&s, USAGE) && !switched_on(&s, "github"));
        s.codex_usage = Some(false);
        s.connectors.insert("github".into(), true);
        assert!(!switched_on(&s, USAGE) && switched_on(&s, "github"));
        // A connector named like the usage read never opens it.
        s.connectors.insert(USAGE.into(), true);
        assert!(!switched_on(&s, USAGE));
        // The phone only by its own switch, and it opens nothing else.
        assert!(!switched_on(&s, PHONE));
        s.connectors.insert(PHONE.into(), true);
        assert!(!switched_on(&s, PHONE));
        s.phone = true;
        assert!(switched_on(&s, PHONE) && !switched_on(&s, WEB));
    }

    #[test]
    fn every_feature_has_a_budget() {
        for c in vults_connectors::all() {
            assert!(budget(c.id()) > 0, "{}", c.id());
        }
        assert_eq!(budget(USAGE), USAGE_BUDGET);
        assert_eq!(budget(PHONE), PHONE_BUDGET);
        assert_eq!(budget("unknown"), 0);
    }

    #[test]
    fn the_usage_budget_holds() {
        let g = NetGate::new(|_| true, budget, BTreeMap::new(), |_| {});
        let s = Instant::now();
        for t in 0..USAGE_BUDGET as u64 {
            assert!(g.allow_at(USAGE, at(s, t)));
        }
        assert!(!g.allow_at(USAGE, at(s, USAGE_BUDGET as u64)));
        assert!(g.allow_at("github", at(s, 0)), "connectors keep their own budget");
    }

    #[test]
    fn each_request_is_traced() {
        use vults_connectors::Gate as _;
        let (g, spans) = gate(true, 3, BTreeMap::new());
        g.done("github", false, Duration::from_millis(250));
        let spans = spans.lock().unwrap();
        assert_eq!(spans.len(), 1);
        let s = &spans[0];
        assert_eq!(
            (s.kind.as_str(), s.name.as_str(), s.status.as_str()),
            ("request", "request github", "error")
        );
        assert_eq!(s.end_ms - s.start_ms, 250);
        assert_eq!(s.attrs["vults.target"], "github");
    }
}
