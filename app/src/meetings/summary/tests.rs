#![allow(clippy::unwrap_used)]
use super::*;
use std::sync::{
    Mutex,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};

struct Fake {
    stop: watch::Sender<bool>,
    calls: AtomicUsize,
    reservations: AtomicUsize,
    dropped: AtomicBool,
    key: bool,
    quota: bool,
    prepare_delay: Duration,
    delay: Duration,
    revoke_at: &'static str,
    reply: Result<String, filter::Error>,
    outcomes: Mutex<Vec<String>>,
}
fn notes() -> String {
    r#"{"title":"Review","summary":"No decision was made.","key_points":[],"todos":[]}"#.into()
}
impl Default for Fake {
    fn default() -> Self {
        Self {
            stop: watch::channel(false).0,
            calls: AtomicUsize::new(0),
            reservations: AtomicUsize::new(0),
            dropped: AtomicBool::new(false),
            key: true,
            quota: true,
            prepare_delay: Duration::ZERO,
            delay: Duration::ZERO,
            revoke_at: "",
            reply: Ok(notes()),
            outcomes: Mutex::new(Vec::new()),
        }
    }
}
struct Dropped<'a>(&'a AtomicBool);
impl Drop for Dropped<'_> {
    fn drop(&mut self) {
        self.0.store(true, Ordering::SeqCst);
    }
}
impl Net for Fake {
    type Prepared = ();
    fn current(&self) -> bool {
        !*self.stop.borrow()
    }
    fn cancelled(&self) -> BoxFuture<'_, ()> {
        let mut receiver = self.stop.subscribe();
        Box::pin(async move {
            if !*receiver.borrow_and_update() {
                let _ = receiver.changed().await;
            }
        })
    }
    fn prepare(&self) -> BoxFuture<'_, Result<(), ()>> {
        Box::pin(async move {
            tokio::time::sleep(self.prepare_delay).await;
            if self.key { Ok(()) } else { Err(()) }
        })
    }
    fn reserve(&self) -> BoxFuture<'_, Option<Span>> {
        Box::pin(async move {
            self.reservations.fetch_add(1, Ordering::SeqCst);
            if self.revoke_at == "reserve" {
                self.stop.send_replace(true);
            }
            self.quota.then(|| Span {
                trace: "t".into(),
                span: "s".into(),
                parent: None,
                name: "request meeting-summary".into(),
                kind: "request".into(),
                start_ms: 0,
                end_ms: 0,
                status: "started".into(),
                attrs: serde_json::json!({}),
            })
        })
    }
    fn call<'a>(&'a self, _: &'a (), _: &'a Request) -> BoxFuture<'a, Result<Reply, filter::Error>> {
        Box::pin(async move {
            self.calls.fetch_add(1, Ordering::SeqCst);
            let _drop = Dropped(&self.dropped);
            tokio::time::sleep(self.delay).await;
            if self.revoke_at == "reply" {
                self.stop.send_replace(true);
            }
            self.reply.clone().map(|text| Reply {
                text,
                input_tokens: Some(10),
                output_tokens: Some(20),
            })
        })
    }
    fn done(&self, _: Span, outcome: &str, _: Option<&Reply>, _: Duration) {
        self.outcomes.lock().unwrap().push(outcome.into());
    }
}

#[tokio::test]
async fn off_missing_key_and_spent_quota_never_call_a_model() {
    let request = Request::build("Should we approve? No decision yet.", "").unwrap();
    for (net, expected) in [
        (
            Fake {
                key: false,
                ..Fake::default()
            },
            "configuration",
        ),
        (
            Fake {
                quota: false,
                ..Fake::default()
            },
            "quota",
        ),
        (
            {
                let net = Fake::default();
                net.stop.send_replace(true);
                net
            },
            "cancelled",
        ),
    ] {
        assert_eq!(run(&net, &request).await, Err(expected));
        assert_eq!(net.calls.load(Ordering::SeqCst), 0);
        assert!(net.outcomes.lock().unwrap().is_empty());
    }
}

#[tokio::test]
async fn one_admitted_attempt_validates_notes_and_never_changes_the_transcript() {
    let request = Request::build("Should we approve? No decision yet.", "Other source.").unwrap();
    for (reply, expected) in [
        (Ok(notes()), "generated"),
        (Ok("not JSON".into()), "invalid"),
        (Err(filter::Error::Tools), "provider"),
        (Err(filter::Error::Truncated), "provider"),
        (Err(filter::Error::Transport), "provider"),
    ] {
        let net = Fake {
            reply,
            ..Fake::default()
        };
        let result = run(&net, &request).await;
        if expected == "generated" {
            assert!(result.unwrap().todos.is_empty());
        } else {
            assert_eq!(result, Err(expected));
        }
        assert_eq!(net.calls.load(Ordering::SeqCst), 1);
        assert_eq!(net.reservations.load(Ordering::SeqCst), 1);
        assert_eq!(*net.outcomes.lock().unwrap(), [expected]);
        assert_eq!(
            request.sources(),
            ["Should we approve? No decision yet.", "Other source."]
        );
    }
}

#[tokio::test(start_paused = true)]
async fn a_single_120_second_deadline_includes_preparation_and_drops_the_call() {
    let request = Request::build("No decision yet.", "").unwrap();
    for prepare_delay in [Duration::ZERO, Duration::from_secs(40), Duration::from_secs(121)] {
        let net = Fake {
            prepare_delay,
            delay: Duration::from_secs(121),
            ..Fake::default()
        };
        let start = tokio::time::Instant::now();
        assert_eq!(run(&net, &request).await, Err("timeout"));
        assert_eq!(tokio::time::Instant::now() - start, TIMEOUT);
        if prepare_delay < TIMEOUT {
            assert!(net.dropped.load(Ordering::SeqCst));
            assert_eq!(*net.outcomes.lock().unwrap(), ["timeout"]);
        } else {
            assert_eq!(net.reservations.load(Ordering::SeqCst), 0);
        }
    }
}

#[tokio::test(start_paused = true)]
async fn revocation_cancels_preparation_and_inflight_work_and_rejects_boundary_replies() {
    let request = Request::build("No decision yet.", "").unwrap();
    for preparing in [false, true] {
        let net = Fake {
            prepare_delay: if preparing { TIMEOUT } else { Duration::ZERO },
            delay: TIMEOUT,
            ..Fake::default()
        };
        let cancel = async {
            tokio::time::sleep(Duration::from_secs(1)).await;
            net.stop.send_replace(true);
        };
        let (result, _) = tokio::join!(run(&net, &request), cancel);
        assert_eq!(result, Err("cancelled"));
        assert_eq!(net.calls.load(Ordering::SeqCst), usize::from(!preparing));
        if !preparing {
            assert!(net.dropped.load(Ordering::SeqCst));
        }
    }
    for revoke_at in ["reserve", "reply"] {
        let net = Fake {
            revoke_at,
            ..Fake::default()
        };
        assert_eq!(run(&net, &request).await, Err("cancelled"));
        assert_eq!(
            net.calls.load(Ordering::SeqCst),
            usize::from(revoke_at == "reply")
        );
        assert_eq!(*net.outcomes.lock().unwrap(), ["cancelled"]);
    }
}

#[test]
fn cancelled_discarded_and_replaced_attempts_cannot_publish_or_block_a_retry() {
    let summary: Summary = serde_json::from_str(&notes()).unwrap();
    let mut slot = Slot::default();
    let (first, stop) = slot.begin().unwrap();
    assert!(slot.begin().is_err());
    slot.cancel();
    assert!(*stop.borrow());
    slot.complete(first, Ok(summary.clone()));
    assert_eq!(slot.phase, "cancelled");
    assert!(slot.notes.is_none());
    let (second, _) = slot.begin().unwrap();
    slot.complete(first, Ok(summary.clone()));
    assert_eq!(slot.phase, "generating");
    slot.complete(second, Err("provider"));
    assert_eq!(slot.error, Some("provider"));
    let (third, _) = slot.begin().unwrap();
    slot.complete(third, Ok(summary));
    assert_eq!(slot.phase, "finished");
    slot.discard();
    assert!(slot.notes.is_none());
    assert_eq!(slot.phase, "idle");
}

#[test]
fn choice_requires_its_own_consent_ledger_and_valid_api_model() {
    let mut settings = crate::settings::Settings {
        dictation_cleanup: true,
        dictation_cleanup_budget: Some("cleanup".into()),
        ..Default::default()
    };
    assert!(Choice::read(&settings).is_none());
    settings.meeting_summary = true;
    assert!(Choice::read(&settings).is_none());
    settings.meeting_summary_budget = Some("summary".into());
    settings.api_provider = "ollama".into();
    assert!(Choice::read(&settings).is_none(), "No silent model substitution");
    settings.api_models.insert("ollama".into(), "fixture".into());
    let choice = Choice::read(&settings).unwrap();
    assert!(choice.provider.local);
    assert_eq!(choice.ledger, "summary");
    settings
        .api_models
        .insert("ollama".into(), "invalid\nmodel".into());
    assert!(Choice::read(&settings).is_none());
}

#[tokio::test]
async fn dropped_blocking_reservations_finish_their_trace_and_remain_charged() {
    for (reason, feature) in [
        ("cancel", crate::gate::CLEANUP),
        ("timeout", crate::gate::MEETING_SUMMARY),
    ] {
        let dir = std::env::temp_dir().join(format!("vults-summary-reserve-{reason}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut store = vults_store::Store::open(&dir).unwrap();
        let identity = store.request_budget_id().unwrap();
        let span = Span {
            trace: reason.into(),
            span: reason.into(),
            parent: None,
            name: format!("request {feature}"),
            kind: "request".into(),
            start_ms: crate::traces::now_ms(),
            end_ms: crate::traces::now_ms(),
            status: "started".into(),
            attrs: serde_json::json!({"vults.feature":feature}),
        };
        assert!(store.reserve_request(&identity, feature, 20, &span).unwrap());
        let (release, waiting) = std::sync::mpsc::channel();
        let path = dir.clone();
        let job = tokio::task::spawn_blocking(move || {
            waiting.recv().unwrap();
            crate::gate::Reservation::new(span, path)
        });
        // Both select branches detach a blocking worker; its eventual output still owns the trace.
        drop(job);
        release.send(()).unwrap();
        tokio::time::timeout(Duration::from_secs(3), async {
            loop {
                let finished = store.spans(1).unwrap().pop().unwrap();
                if finished.status != "started" {
                    assert_eq!(finished.attrs["vults.outcome"], "cancelled");
                    break;
                }
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
        })
        .await
        .unwrap();
        assert_eq!(store.request_attempt_starts(feature, 0).unwrap().len(), 1);
        drop(store);
        let _ = std::fs::remove_dir_all(dir);
    }
}

#[cfg(target_os = "linux")]
#[test]
fn publication_rechecks_consent_and_releases_a_job_claimed_after_revocation() {
    let app = tauri::test::mock_app();
    let state = Arc::new(super::super::State::default());
    let mut settings = crate::settings::Settings {
        meeting_summary: true,
        meeting_summary_budget: Some("fixture".into()),
        api_provider: "ollama".into(),
        ..Default::default()
    };
    settings.api_models.insert("ollama".into(), "fixture".into());
    let choice = Choice::read(&settings).unwrap();
    app.manage(crate::settings::SettingsState(Mutex::new(settings)));
    app.manage(state.clone());
    for reason in ["off", "model", "lease", "shutdown", "current"] {
        let (attempt, stop) = {
            let mut slot = state.slot.lock().unwrap();
            slot.view.id = 7;
            slot.view.phase = "finished";
            slot.touched = Instant::now();
            slot.summary.begin().unwrap()
        };
        {
            let saved = app.state::<crate::settings::SettingsState>();
            let mut saved = saved.0.lock().unwrap();
            saved.meeting_summary = reason != "off";
            saved.api_models.insert(
                "ollama".into(),
                if reason == "model" { "other" } else { "fixture" }.into(),
            );
        }
        if reason == "lease" {
            state.slot.lock().unwrap().touched = Instant::now() - Duration::from_secs(3);
        }
        state.shutdown.store(reason == "shutdown", Ordering::SeqCst);
        publish(
            app.handle(),
            7,
            attempt,
            &stop,
            &choice,
            Ok(serde_json::from_str(&notes()).unwrap()),
        );
        let mut slot = state.slot.lock().unwrap();
        assert_eq!(
            slot.summary.phase,
            if reason == "current" {
                "finished"
            } else {
                "cancelled"
            }
        );
        assert_eq!(slot.summary.notes.is_some(), reason == "current");
        slot.summary.discard();
    }
}

#[cfg(target_os = "linux")]
#[test]
fn summary_reads_and_cancellation_reject_every_other_webview() {
    let app = tauri::test::mock_app();
    for label in [crate::ISLAND, crate::SETTINGS, crate::NEST_WINDOW, "other"] {
        let view = tauri::WebviewWindowBuilder::new(&app, label, tauri::WebviewUrl::default())
            .build()
            .unwrap();
        assert!(meeting_summary_status(app.handle().clone(), view.as_ref().clone(), 1).is_err());
        assert!(meeting_summary_cancel(app.handle().clone(), view.as_ref().clone(), 1).is_err());
    }
}
