//! The network gate (ADR 0019; docs/safety.md): every request the app makes on its own, without a
//! click at that moment, asks here first. Each feature has the user's switch, a budget over the
//! last 24 hours, and a local trace of each request (a `request` span). Today: the connectors'
//! polls. Nothing of it leaves the machine.

use std::collections::{BTreeMap, VecDeque};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde_json::json;
use vults_store::traces::Span;

/// Requests a connector may make in any 24 hours: the GitHub poll makes at most ~1,500 (every
/// minute while checks run), so only a runaway loop meets it.
pub const CONNECTOR_BUDGET: usize = 3000;
const DAY_MS: i64 = 24 * 3600 * 1000;
const DAY: Duration = Duration::from_secs(24 * 3600);

pub struct NetGate {
    /// Is this connector switched on by the user?
    switch: Box<dyn Fn(&str) -> bool + Send + Sync>,
    budget: usize,
    /// When each connector's recent requests went out, on the monotonic clock: a
    /// wall clock set back or forward never resets the budget or locks a connector out.
    sent: Mutex<BTreeMap<String, VecDeque<Instant>>>,
    /// Where each request's span goes.
    trace: Box<dyn Fn(Span) + Send + Sync>,
}

impl NetGate {
    pub fn new(
        switch: impl Fn(&str) -> bool + Send + Sync + 'static,
        budget: usize,
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
            budget,
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
        if times.len() >= self.budget {
            tracing::warn!(
                connector,
                budget = self.budget,
                "connector's daily request budget spent"
            );
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
        attrs: json!({ "vults.feature": "connectors", "vults.target": connector }),
    }
}

/// Each connector's requests of the last 24 hours, read back from the traces, so a restart does not
/// give a fresh budget.
pub fn seed(connectors: &[&str]) -> BTreeMap<String, Vec<i64>> {
    let since = crate::traces::now_ms() - DAY_MS;
    let Ok(store) = vults_store::Store::open(&crate::history::dir()) else {
        return BTreeMap::new();
    };
    connectors
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
    use std::sync::Arc;

    fn gate(on: bool, budget: usize, seed: BTreeMap<String, Vec<i64>>) -> (NetGate, Arc<Mutex<Vec<Span>>>) {
        let spans = Arc::new(Mutex::new(Vec::new()));
        let sink = spans.clone();
        let g = NetGate::new(move |_| on, budget, seed, move |s| sink.lock().unwrap().push(s));
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

    /// The real gate in front of the real runtime: a connector the runtime has on, but whose switch
    /// the gate reads off, makes no request at all (ADR 0019's consequence).
    #[tokio::test]
    async fn the_runtime_behind_a_closed_gate_sends_nothing() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use vults_connectors::{Connector, Event, Poll, Runtime, Snapshot};

        struct Counting(Arc<AtomicUsize>);
        impl Connector for Counting {
            fn id(&self) -> &'static str {
                "counting"
            }
            fn interval(&self, _: &Snapshot) -> Duration {
                Duration::from_millis(10)
            }
            fn poll(&self) -> Poll<'_> {
                self.0.fetch_add(1, Ordering::SeqCst);
                Box::pin(async { Ok(Snapshot::new()) })
            }
            fn diff(&self, _: &Snapshot, _: &Snapshot) -> Vec<Event> {
                Vec::new()
            }
        }

        let polls = Arc::new(AtomicUsize::new(0));
        let (g, spans) = gate(false, CONNECTOR_BUDGET, BTreeMap::new());
        let (tx, _rx) = tokio::sync::mpsc::channel(8);
        let on = BTreeMap::from([("counting".to_string(), true)]);
        let dir = std::env::temp_dir().join(format!("vults-gate-{}", std::process::id()));
        let _rt = Runtime::start(vec![Box::new(Counting(polls.clone()))], &on, dir, tx, Arc::new(g));
        tokio::time::sleep(Duration::from_millis(150)).await;
        assert_eq!(polls.load(Ordering::SeqCst), 0);
        assert!(spans.lock().unwrap().is_empty());
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
