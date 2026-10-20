//! The network gate (ADR 0019; docs/safety.md): every request the app makes on its own, without a
//! click at that moment, asks here first. Each feature has the user's switch, a budget over the
//! last 24 hours, and a local trace of each request (a `request` span). Today: the connectors'
//! polls, the Codex usage read, Zeca's web tools, the phone's relay, the journal's daily
//! summary, the memory's suggestions, Zeca's sleep, opted-in dictation clean-up, meeting summaries
//! and tool-free voice consultations.
//! Nothing of the trace leaves the machine. `scripts/check-network.sh` keeps any other
//! way out from appearing unlisted. Explicit meeting installation uses its own one-use review
//! adapter below, with durable admission for every HTTP attempt.

mod meeting_assets;
pub(crate) mod realtime;
pub use meeting_assets::MeetingAssetsGate;

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
/// Zeca's sleep (M8): one call a day; a few more for a day whose call failed (it is tried again on
/// a later pass).
pub const SLEEP: &str = "sleep";
pub const SLEEP_BUDGET: usize = 4;
pub const CLEANUP: &str = "dictation-cleanup";
pub const CLEANUP_BUDGET: usize = 2000;
pub const MEETING_SUMMARY: &str = "meeting-summary";
pub const MEETING_SUMMARY_BUDGET: usize = 20;
pub const VOICE_REALTIME: &str = "voice-realtime";
pub const VOICE_CONSULTATION: &str = "voice-consultation";
pub const VOICE_CONSULTATION_BUDGET: usize = 20;
pub const VOICE_CODEX_CONSULTATION: &str = "voice-codex-consultation";
pub const VOICE_CODEX_CONSULTATION_BUDGET: usize = 20;
const DAY_MS: i64 = 24 * 3600 * 1000;
const DAY: Duration = Duration::from_secs(24 * 3600);

/// Own a durable admission until its async caller receives it, including detached blocking work.
pub(crate) struct Reservation {
    span: Option<Span>,
    directory: std::path::PathBuf,
    started: Instant,
}
impl Reservation {
    pub(crate) fn new(span: Span, directory: std::path::PathBuf) -> Self {
        Self {
            span: Some(span),
            directory,
            started: Instant::now(),
        }
    }
    pub(crate) fn take(mut self) -> Option<Span> {
        self.span.take()
    }
}
impl Drop for Reservation {
    fn drop(&mut self) {
        if let Some(mut span) = self.span.take() {
            span.end_ms = span
                .start_ms
                .saturating_add(i64::try_from(self.started.elapsed().as_millis()).unwrap_or(i64::MAX));
            span.status = "error".into();
            span.attrs["vults.outcome"] = json!("cancelled");
            if let Ok(mut store) = vults_store::Store::open_current(&self.directory) {
                let _ = store.finish_request(&span);
            }
        }
    }
}

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
        CLEANUP => CLEANUP_BUDGET,
        MEETING_SUMMARY => MEETING_SUMMARY_BUDGET,
        VOICE_CONSULTATION => VOICE_CONSULTATION_BUDGET,
        VOICE_CODEX_CONSULTATION => VOICE_CODEX_CONSULTATION_BUDGET,
        meeting_assets::FEATURE => meeting_assets::BUDGET,
        "github" => CONNECTOR_BUDGET,
        _ => 0,
    }
}

/// Is `feature` switched on in these settings? The gate asks before every request.
pub fn switched_on(s: &crate::settings::Settings, feature: &str) -> bool {
    if feature == VOICE_CODEX_CONSULTATION {
        vults_chat::consult_codex::SUBSCRIPTION_AUTH_QUALIFIED
            && s.voice_codex_consultation
            && s.voice_codex_consultation_budget.is_some()
    } else if feature == VOICE_CONSULTATION {
        s.voice_consultation && s.voice_consultation_budget.is_some()
    } else if feature == VOICE_REALTIME {
        s.voice_realtime.enabled()
    } else if feature == meeting_assets::FEATURE {
        false
    } else if feature == MEETING_SUMMARY {
        s.meeting_summary && s.meeting_summary_budget.is_some()
    } else if feature == CLEANUP {
        s.dictation_cleanup && s.dictation_cleanup_budget.is_some()
    } else if feature == USAGE {
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
        connector != CLEANUP
            && connector != MEETING_SUMMARY
            && connector != VOICE_REALTIME
            && connector != VOICE_CONSULTATION
            && connector != VOICE_CODEX_CONSULTATION
            && connector != meeting_assets::FEATURE
            && self.allow_at(connector, Instant::now())
    }

    fn done(&self, connector: &str, ok: bool, took: Duration) {
        let end_ms = crate::traces::now_ms();
        let took_ms = i64::try_from(took.as_millis()).unwrap_or(0);
        (self.trace)(request_span(connector, ok, end_ms - took_ms, end_ms));
    }
}

impl NetGate {
    /// N3 needs a durable admission before sending, including local calls and interrupted attempts.
    #[cfg(any(target_os = "linux", test))]
    pub fn reserve_cleanup(
        &self,
        store: &mut vults_store::Store,
        identity: &str,
        provider: &str,
        model: &str,
        local: bool,
    ) -> Option<Span> {
        self.reserve_text(store, identity, CLEANUP, provider, model, local)
    }

    pub fn reserve_meeting_summary(
        &self,
        store: &mut vults_store::Store,
        identity: &str,
        provider: &str,
        model: &str,
        local: bool,
    ) -> Option<Span> {
        self.reserve_text(store, identity, MEETING_SUMMARY, provider, model, local)
    }
    pub fn reserve_voice_consultation(&self, store: &mut vults_store::Store, identity: &str) -> Option<Span> {
        self.reserve_text(
            store,
            identity,
            VOICE_CONSULTATION,
            "claude",
            vults_chat::summary::MODEL,
            false,
        )
    }

    pub fn reserve_codex_consultation(&self, store: &mut vults_store::Store, identity: &str) -> Option<Span> {
        self.reserve_text(
            store,
            identity,
            VOICE_CODEX_CONSULTATION,
            "codex",
            vults_chat::consult_codex::MODEL,
            false,
        )
    }

    fn reserve_text(
        &self,
        store: &mut vults_store::Store,
        identity: &str,
        feature: &str,
        provider: &str,
        model: &str,
        local: bool,
    ) -> Option<Span> {
        if !self.allow_at(feature, Instant::now()) {
            return None;
        }
        let now = crate::traces::now_ms();
        let mut span = request_span(feature, false, now, now);
        span.status = "started".into();
        span.attrs["vults.provider"] = json!(provider);
        span.attrs["gen_ai.request.model"] = json!(model);
        span.attrs["vults.local"] = json!(local);
        store
            .reserve_request(identity, feature, budget(feature), &span)
            .ok()
            .filter(|ok| *ok)
            .map(|_| span)
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
        features.push(CLEANUP);
        features.push(MEETING_SUMMARY);
        features.push(VOICE_CONSULTATION);
        features.push(VOICE_CODEX_CONSULTATION);
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
        CLEANUP => "dictation-cleanup",
        MEETING_SUMMARY => "meeting-summary",
        VOICE_CONSULTATION => "voice-consultation",
        VOICE_CODEX_CONSULTATION => "voice-codex-consultation",
        meeting_assets::FEATURE => "meeting-assets",
        _ => "connectors",
    }
}

/// Each feature's requests of the last 24 hours, read back from the traces, so a restart does not
/// give a fresh budget.
pub fn seed(features: &[&str]) -> BTreeMap<String, Vec<i64>> {
    let since = crate::traces::now_ms() - DAY_MS;
    let store = vults_store::Store::open(&crate::history::dir());
    features
        .iter()
        .map(|c| {
            let times = if matches!(
                *c,
                CLEANUP | MEETING_SUMMARY | VOICE_CONSULTATION | VOICE_CODEX_CONSULTATION
            ) {
                store
                    .as_ref()
                    .ok()
                    .and_then(|s| s.request_attempt_starts(c, since).ok())
                    .unwrap_or_else(|| vec![crate::traces::now_ms(); budget(c)])
            } else {
                store
                    .as_ref()
                    .ok()
                    .and_then(|s| s.span_starts("request", &span_name(c), since).ok())
                    .unwrap_or_default()
            };
            (c.to_string(), times)
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
    fn realtime_cannot_borrow_a_connector_switch_or_a_count_only_allowance() {
        let mut settings = crate::settings::Settings::default();
        settings.connectors.insert(VOICE_REALTIME.into(), true);
        assert!(!switched_on(&settings, VOICE_REALTIME));
        assert_eq!(budget(VOICE_REALTIME), 0);
        let (gate, _) = gate(true, usize::MAX, BTreeMap::new());
        assert!(!vults_connectors::Gate::allow(&gate, VOICE_REALTIME));
    }

    #[test]
    fn realtime_receipts_settle_the_reserved_attempt_and_unknown_usage_keeps_its_ceiling() {
        use vults_chat::realtime::{Plan, Session};
        const NOW: i64 = 1_792_368_000_000;
        let dir = std::env::temp_dir().join(format!("vults-realtime-budget-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut store = vults_store::Store::open(&dir).unwrap();
        let identity = store.request_budget_id().unwrap();
        let plan = Plan::new("gpt-realtime-2.1-mini", 8192, 1024).unwrap();
        let instructions = plan.session_update()["session"]["instructions"].clone();
        let mut session = Session::new(plan);
        let span = |attempt: &str| {
            let mut span = request_span(VOICE_REALTIME, false, NOW, NOW);
            span.status = "started".into();
            span.span = attempt.into();
            span
        };
        let mut receipts = Vec::new();
        for line in include_str!("../../crates/chat/tests/fixtures/realtime-session.jsonl").lines() {
            let mut event: serde_json::Value = serde_json::from_str(line).unwrap();
            if event["type"] == "session.updated" {
                event["session"]["instructions"] = instructions.clone();
            } else if event["type"] == "response.created" {
                let attempt = event["response"]["metadata"]["vults_attempt"].as_str().unwrap();
                let prepared = session.prepare(attempt, NOW).unwrap();
                assert!(
                    store
                        .reserve_cost(
                            &identity,
                            VOICE_REALTIME,
                            1_000_000,
                            Some(prepared.ceiling_microusd),
                            &span(attempt)
                        )
                        .unwrap()
                );
                assert!(
                    !store
                        .reserve_cost(
                            &identity,
                            VOICE_REALTIME,
                            1_000_000,
                            Some(820_801),
                            &span("parallel")
                        )
                        .unwrap()
                );
            } else if event["type"] == "response.done" && event["response"]["id"] == "resp_b" {
                event["response"]["usage"] = serde_json::Value::Null;
            }
            if let Some(receipt) = session.receive(&serde_json::to_vec(&event).unwrap()).unwrap() {
                assert!(
                    store
                        .settle_cost(
                            &identity,
                            &receipt.attempt,
                            receipt.actual_microusd,
                            NOW,
                            receipt.ok
                        )
                        .unwrap()
                );
                assert!(
                    !store
                        .settle_cost(&identity, &receipt.attempt, Some(0), NOW, true)
                        .unwrap()
                );
                receipts.push(receipt);
            }
        }
        assert_eq!(receipts[0].actual_microusd, Some(2059));
        assert_eq!(receipts[1].actual_microusd, None);
        assert!(!receipts[1].ok);
        assert!(!receipts[1].within_bounds);
        assert!(session.prepare("cannot_reopen", NOW).is_err());
        assert!(
            !store
                .reserve_cost(&identity, VOICE_REALTIME, 1_000_000, Some(818_742), &span("over"))
                .unwrap()
        );
        assert!(
            store
                .reserve_cost(
                    &identity,
                    VOICE_REALTIME,
                    1_000_000,
                    Some(818_741),
                    &span("exact_remaining")
                )
                .unwrap()
        );
        drop(store);
        std::fs::remove_dir_all(dir).unwrap();
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
    fn cleanup_has_an_independent_opt_in_and_durable_non_bypassable_admission() {
        use vults_connectors::Gate;
        let mut settings = crate::settings::Settings::default();
        settings.connectors.insert(CLEANUP.into(), true);
        assert!(!switched_on(&settings, CLEANUP));
        settings.dictation_cleanup = true;
        assert!(!switched_on(&settings, CLEANUP));
        settings.dictation_cleanup_budget = Some("fixture".into());
        assert!(switched_on(&settings, CLEANUP));
        assert_eq!(budget(CLEANUP), 2000);
        let dir = std::env::temp_dir().join(format!("vults-cleanup-gate-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut store = vults_store::Store::open(&dir).unwrap();
        let identity = store.request_budget_id().unwrap();
        let (off, _) = gate(false, 2, BTreeMap::new());
        assert!(
            off.reserve_cleanup(&mut store, &identity, "ollama", "fixture", true)
                .is_none()
        );
        let (on, _) = gate(true, 2, BTreeMap::new());
        assert!(
            !on.allow(CLEANUP),
            "Generic admission cannot bypass durable reservation"
        );
        let span = on
            .reserve_cleanup(&mut store, &identity, "ollama", "fixture", true)
            .unwrap();
        assert_eq!(
            span.attrs,
            json!({"vults.feature":"dictation-cleanup","vults.target":"dictation-cleanup","vults.provider":"ollama","gen_ai.request.model":"fixture","vults.local":true})
        );
        assert!(
            on.reserve_cleanup(&mut store, &identity, "ollama", "fixture", true)
                .is_some()
        );
        assert!(
            on.reserve_cleanup(&mut store, &identity, "ollama", "fixture", true)
                .is_none()
        );
        assert_eq!(store.request_attempt_starts(CLEANUP, 0).unwrap().len(), 2);
        assert_eq!(store.spans(10).unwrap().len(), 2);
        drop(store);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn meeting_summary_requires_separate_consent_and_twenty_durable_attempts_even_locally() {
        use vults_connectors::Gate;
        let mut settings = crate::settings::Settings::default();
        settings.connectors.insert(MEETING_SUMMARY.into(), true);
        settings.dictation_cleanup = true;
        settings.dictation_cleanup_budget = Some("cleanup".into());
        assert!(!switched_on(&settings, MEETING_SUMMARY));
        settings.meeting_summary = true;
        assert!(!switched_on(&settings, MEETING_SUMMARY));
        settings.meeting_summary_budget = Some("summary".into());
        assert!(switched_on(&settings, MEETING_SUMMARY));
        let dir = std::env::temp_dir().join(format!("vults-summary-gate-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut store = vults_store::Store::open(&dir).unwrap();
        let identity = store.request_budget_id().unwrap();
        let off = NetGate::new(|_| false, budget, BTreeMap::new(), |_| {});
        assert!(
            off.reserve_meeting_summary(&mut store, &identity, "ollama", "fixture", true)
                .is_none()
        );
        assert_eq!(budget(MEETING_SUMMARY), 20);
        for index in 0..20 {
            // Restarting the gate and alternating destinations cannot refresh this feature's quota.
            let gate = NetGate::new(|_| true, budget, BTreeMap::new(), |_| {});
            assert!(!gate.allow(MEETING_SUMMARY));
            assert!(
                gate.reserve_meeting_summary(
                    &mut store,
                    &identity,
                    if index % 2 == 0 { "ollama" } else { "anthropic" },
                    "fixture",
                    index % 2 == 0
                )
                .is_some()
            );
        }
        store.clear_all(&dir).unwrap();
        drop(store);
        let mut store = vults_store::Store::open_current(&dir).unwrap();
        let gate = NetGate::new(|_| true, budget, BTreeMap::new(), |_| {});
        assert!(
            gate.reserve_meeting_summary(&mut store, &identity, "ollama", "other-model", true)
                .is_none()
        );
        assert_eq!(
            store.request_attempt_starts(MEETING_SUMMARY, 0).unwrap().len(),
            20
        );
        assert!(
            gate.reserve_cleanup(&mut store, &identity, "ollama", "fixture", true)
                .is_some()
        );
        drop(store);
        let _ = std::fs::remove_dir_all(dir);
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
    fn voice_consultation_has_independent_durable_admission_and_content_free_traces() {
        use vults_connectors::Gate;
        let mut settings = crate::settings::Settings::default();
        settings.voice_realtime.on = true;
        settings.voice_realtime.ledger = Some("voice".into());
        settings.connectors.insert(VOICE_CONSULTATION.into(), true);
        assert!(!switched_on(&settings, VOICE_CONSULTATION));
        settings.voice_consultation = true;
        assert!(!switched_on(&settings, VOICE_CONSULTATION));
        settings.voice_consultation_budget = Some("consultation".into());
        assert!(switched_on(&settings, VOICE_CONSULTATION));
        let directory = std::env::temp_dir().join(format!("vults-consult-gate-{}", crate::traces::new_id()));
        let mut store = vults_store::Store::open(&directory).unwrap();
        let identity = store.request_budget_id().unwrap();
        let off = NetGate::new(|_| false, budget, BTreeMap::new(), |_| {});
        assert!(off.reserve_voice_consultation(&mut store, &identity).is_none());
        for _ in 0..VOICE_CONSULTATION_BUDGET {
            let gate = NetGate::new(|_| true, budget, BTreeMap::new(), |_| {});
            assert!(!gate.allow(VOICE_CONSULTATION));
            let span = gate.reserve_voice_consultation(&mut store, &identity).unwrap();
            assert_eq!(
                span.attrs,
                json!({"vults.feature":"voice-consultation","vults.target":"voice-consultation","vults.provider":"claude","gen_ai.request.model":vults_chat::summary::MODEL,"vults.local":false})
            );
        }
        store.clear_all(&directory).unwrap();
        drop(store);
        let mut store = vults_store::Store::open_current(&directory).unwrap();
        let gate = NetGate::new(|_| true, budget, BTreeMap::new(), |_| {});
        assert!(gate.reserve_voice_consultation(&mut store, &identity).is_none());
        assert!(
            gate.reserve_meeting_summary(&mut store, &identity, "fixture", "fixture", true)
                .is_some()
        );
        let starts = store.request_attempt_starts(VOICE_CONSULTATION, 0).unwrap();
        assert_eq!(starts.len(), VOICE_CONSULTATION_BUDGET);
        drop(store);
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn codex_consultation_cannot_borrow_consent_or_reset_its_own_quota() {
        use vults_connectors::Gate;
        let mut settings = crate::settings::Settings {
            voice_consultation: true,
            voice_consultation_budget: Some("claude".into()),
            ..Default::default()
        };
        settings.connectors.insert(VOICE_CODEX_CONSULTATION.into(), true);
        assert!(!switched_on(&settings, VOICE_CODEX_CONSULTATION));
        settings.voice_codex_consultation = true;
        assert!(!switched_on(&settings, VOICE_CODEX_CONSULTATION));
        settings.voice_codex_consultation_budget = Some("codex".into());
        assert!(!switched_on(&settings, VOICE_CODEX_CONSULTATION));
        settings.voice_consultation = false;
        assert!(!switched_on(&settings, VOICE_CODEX_CONSULTATION));
        let directory = std::env::temp_dir().join(format!("vults-codex-gate-{}", crate::traces::new_id()));
        let mut store = vults_store::Store::open(&directory).unwrap();
        let identity = store.request_budget_id().unwrap();
        let off = NetGate::new(|_| false, budget, BTreeMap::new(), |_| {});
        assert!(off.reserve_codex_consultation(&mut store, &identity).is_none());
        for _ in 0..VOICE_CODEX_CONSULTATION_BUDGET {
            let gate = NetGate::new(|_| true, budget, BTreeMap::new(), |_| {});
            assert!(!gate.allow(VOICE_CODEX_CONSULTATION));
            assert!(
                gate.reserve_codex_consultation(&mut store, "wrong-ledger")
                    .is_none()
            );
            let span = gate.reserve_codex_consultation(&mut store, &identity).unwrap();
            assert_eq!(
                span.attrs,
                json!({"vults.feature":"voice-codex-consultation","vults.target":"voice-codex-consultation","vults.provider":"codex","gen_ai.request.model":vults_chat::consult_codex::MODEL,"vults.local":false})
            );
        }
        store.clear_all(&directory).unwrap();
        drop(store);
        let mut store = vults_store::Store::open_current(&directory).unwrap();
        let gate = NetGate::new(|_| true, budget, BTreeMap::new(), |_| {});
        assert!(gate.reserve_codex_consultation(&mut store, &identity).is_none());
        assert!(gate.reserve_voice_consultation(&mut store, &identity).is_some());
        assert_eq!(
            store
                .request_attempt_starts(VOICE_CODEX_CONSULTATION, 0)
                .unwrap()
                .len(),
            VOICE_CODEX_CONSULTATION_BUDGET
        );
        drop(store);
        std::fs::remove_dir_all(directory).unwrap();
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
