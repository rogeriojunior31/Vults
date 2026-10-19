#![allow(clippy::unwrap_used)]
use super::*;
use std::sync::{
    Mutex,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};
use vults_dictation::cleanup::Language;

struct Fake {
    on: AtomicBool,
    stopped: AtomicBool,
    changed: watch::Sender<u64>,
    prepared: AtomicUsize,
    reserved: AtomicUsize,
    calls: AtomicUsize,
    dropped: AtomicBool,
    key: bool,
    quota: bool,
    delay: Duration,
    prepare_delay: Duration,
    reply: Result<String, filter::Error>,
    revoke_at: &'static str,
    outcomes: Mutex<Vec<String>>,
}
impl Default for Fake {
    fn default() -> Self {
        Self {
            on: AtomicBool::new(true),
            stopped: AtomicBool::new(false),
            changed: watch::channel(0).0,
            prepared: AtomicUsize::new(0),
            reserved: AtomicUsize::new(0),
            calls: AtomicUsize::new(0),
            dropped: AtomicBool::new(false),
            key: true,
            quota: true,
            delay: Duration::ZERO,
            prepare_delay: Duration::ZERO,
            reply: Ok("Please review the branch.".into()),
            revoke_at: "",
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
impl Fake {
    fn revoke(&self) {
        self.on.store(false, Ordering::SeqCst);
        self.changed.send_modify(|n| *n += 1);
    }
}
impl Net for Fake {
    type Prepared = ();
    fn current(&self) -> bool {
        self.on.load(Ordering::SeqCst) && !self.stopped.load(Ordering::SeqCst) && *self.changed.borrow() == 0
    }
    fn changed(&self) -> BoxFuture<'_, ()> {
        let mut rx = self.changed.subscribe();
        Box::pin(async move {
            let _ = rx.changed().await;
        })
    }
    fn prepare(&self) -> BoxFuture<'_, Result<(), ()>> {
        Box::pin(async move {
            self.prepared.fetch_add(1, Ordering::SeqCst);
            tokio::time::sleep(self.prepare_delay).await;
            if self.key { Ok(()) } else { Err(()) }
        })
    }
    fn reserve(&self) -> BoxFuture<'_, Option<Span>> {
        Box::pin(async move {
            self.reserved.fetch_add(1, Ordering::SeqCst);
            if self.revoke_at == "reserve" {
                self.revoke();
            }
            self.quota.then(|| Span {
                trace: "t".into(),
                span: "s".into(),
                parent: None,
                name: "request dictation-cleanup".into(),
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
                self.revoke();
            }
            self.reply.clone().map(|text| Reply {
                text,
                input_tokens: None,
                output_tokens: None,
            })
        })
    }
    fn done(&self, _: Span, outcome: &str, _: Option<(Option<u64>, Option<u64>)>, _: Duration) {
        self.outcomes.lock().unwrap().push(outcome.into());
    }
}
fn options() -> Options {
    Options {
        style: Style::Natural,
        ..Options::default()
    }
}
fn request(options: &Options) -> Request {
    Request::build(
        &vults_dictation::text::Rules::default()
            .apply("please review the branch", false)
            .expect("Local text"),
        options,
    )
    .expect("Fixture")
    .expect("Enabled style")
}

#[tokio::test]
async fn off_stale_unavailable_and_spent_never_start_a_provider_call() {
    let opts = options();
    let req = request(&opts);
    for net in [
        Fake {
            on: AtomicBool::new(false),
            ..Fake::default()
        },
        Fake {
            key: false,
            ..Fake::default()
        },
        Fake {
            quota: false,
            ..Fake::default()
        },
    ] {
        assert_eq!(run(&net, &req, &opts).await, None);
        assert_eq!(net.calls.load(Ordering::SeqCst), 0);
    }
    let net = Fake::default();
    net.revoke();
    net.on.store(true, Ordering::SeqCst);
    assert_eq!(run(&net, &req, &opts).await, None);
    assert_eq!(
        net.prepared.load(Ordering::SeqCst),
        0,
        "A later grant cannot revive a captured choice"
    );
}

#[tokio::test]
async fn valid_text_applies_and_failure_or_invalid_text_keeps_the_whole_original() {
    let opts = options();
    let req = request(&opts);
    let net = Fake::default();
    assert_eq!(
        run(&net, &req, &opts).await.as_deref(),
        Some("Please review the branch.")
    );
    assert_eq!(*net.outcomes.lock().unwrap(), ["applied"]);
    for (reply, outcome) in [
        (Err(filter::Error::Transport), "provider-error"),
        (Ok(" ".into()), "invalid"),
    ] {
        let net = Fake {
            reply,
            ..Fake::default()
        };
        assert_eq!(
            run(&net, &req, &opts)
                .await
                .unwrap_or_else(|| req.original().to_string()),
            "please review the branch"
        );
        assert_eq!(*net.outcomes.lock().unwrap(), [outcome]);
    }
}

#[tokio::test(start_paused = true)]
async fn deadlines_cover_normal_translation_and_prompt_and_drop_slow_providers() {
    for (style, write_in, seconds) in [
        (Style::Natural, None, 4),
        (Style::Natural, Some(Language::Spanish), 8),
        (Style::Prompt, Some(Language::English), 12),
    ] {
        let opts = Options {
            style,
            write_in,
            ..Options::default()
        };
        let req = request(&opts);
        let net = Fake {
            delay: Duration::from_secs(60),
            ..Fake::default()
        };
        let before = tokio::time::Instant::now();
        assert_eq!(run(&net, &req, &opts).await, None);
        assert_eq!(tokio::time::Instant::now() - before, Duration::from_secs(seconds));
        assert!(net.dropped.load(Ordering::SeqCst));
        assert_eq!(*net.outcomes.lock().unwrap(), ["timeout"]);
    }
}

#[tokio::test(start_paused = true)]
async fn disable_and_ticket_cancellation_drop_inflight_work_and_never_apply_a_late_reply() {
    for notify in [true, false] {
        let net = Fake {
            delay: Duration::from_secs(60),
            ..Fake::default()
        };
        let opts = options();
        let req = request(&opts);
        let cancel = async {
            while net.calls.load(Ordering::SeqCst) == 0 {
                tokio::task::yield_now().await;
            }
            if notify {
                net.revoke();
            } else {
                net.stopped.store(true, Ordering::SeqCst);
            }
        };
        let (result, _) = tokio::join!(run(&net, &req, &opts), cancel);
        assert_eq!(result, None);
        assert!(net.dropped.load(Ordering::SeqCst));
        assert_eq!(*net.outcomes.lock().unwrap(), ["cancelled"]);
    }
    for revoke_at in ["reserve", "reply"] {
        let net = Fake {
            revoke_at,
            ..Fake::default()
        };
        let opts = options();
        let req = request(&opts);
        assert_eq!(run(&net, &req, &opts).await, None);
        assert_eq!(*net.outcomes.lock().unwrap(), ["cancelled"]);
        if revoke_at == "reserve" {
            assert_eq!(net.calls.load(Ordering::SeqCst), 0);
        }
    }
}

#[tokio::test(start_paused = true)]
async fn an_entire_day_switched_off_sends_nothing_and_slow_preflight_cannot_extend_the_deadline() {
    let opts = options();
    let req = request(&opts);
    let off = Fake {
        on: AtomicBool::new(false),
        ..Fake::default()
    };
    for _ in 0..(24 * 12) {
        assert_eq!(run(&off, &req, &opts).await, None);
        tokio::time::sleep(Duration::from_secs(300)).await;
    }
    assert_eq!(off.prepared.load(Ordering::SeqCst), 0);
    assert_eq!(off.reserved.load(Ordering::SeqCst), 0);
    assert_eq!(off.calls.load(Ordering::SeqCst), 0);
    assert!(off.outcomes.lock().unwrap().is_empty());
    let slow = Fake {
        prepare_delay: Duration::from_secs(60),
        ..Fake::default()
    };
    let before = tokio::time::Instant::now();
    assert_eq!(run(&slow, &req, &opts).await, None);
    assert_eq!(tokio::time::Instant::now() - before, Duration::from_secs(4));
    assert_eq!(slow.reserved.load(Ordering::SeqCst), 0);
    assert_eq!(slow.calls.load(Ordering::SeqCst), 0);
}

#[test]
fn the_key_down_choice_is_shared_once_and_later_consent_cannot_replace_it() {
    let choice = || Choice {
        provider: vults_chat::providers::find("ollama").unwrap(),
        model: "fixture".into(),
        revision: 7,
        budget: "fixture-budget".into(),
        app: "fixture-app".into(),
        resolved: vults_dictation::styles::Resolved {
            style: Style::Natural,
            write_in: None,
        },
    };
    for enabled in [false, true] {
        let ticket = Ticket::test_command();
        ticket.pin_cleanup(enabled.then(choice));
        let captured = ticket.clone();
        ticket.pin_cleanup((!enabled).then(choice));
        assert_eq!(captured.cleanup_choice().is_some(), enabled);
        assert_eq!(ticket.cleanup_choice().is_some(), enabled);
        ticket.test_cancel();
        assert!(!captured.valid());
    }
}

#[test]
fn options_use_the_frozen_identity_style_and_language_and_unreadable_preferences_never_guess() {
    let dir = std::env::temp_dir().join(format!("vults-cleanup-style-cache-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let state = State::default();
    assert!(state.styles.lock().unwrap().is_none());
    let store = vults_store::Store::open(&dir).unwrap();
    store
        .dictation_style_set("thunderbird", Some("polished"))
        .unwrap();
    state.reload_styles(&store);
    let prefs = vults_dictation::styles::Preferences {
        apps: state.styles.lock().unwrap().clone().unwrap(),
        write_in: Some(Language::English),
        ..Default::default()
    };
    let choice = Choice {
        provider: vults_chat::providers::find("ollama").unwrap(),
        model: "fixture".into(),
        revision: 0,
        budget: "fixture".into(),
        app: "Thunderbird".into(),
        resolved: vults_dictation::styles::resolve(true, Some("Thunderbird"), false, false, &prefs),
    };
    store.dictation_style_set("thunderbird", Some("prompt")).unwrap();
    state.reload_styles(&store);
    assert_eq!(
        state.styles.lock().unwrap().as_ref().unwrap()["thunderbird"],
        Style::Prompt
    );
    let opts = choice.options(&["Vults".into()]);
    assert_eq!(opts.style, Style::Polished);
    assert_eq!(opts.write_in, Some(Language::English));
    assert_eq!(opts.app, "Thunderbird");
    assert_eq!(opts.spellings, ["Vults"]);
    assert_eq!(timeout(&opts), Duration::from_secs(8));
    let req = request(&opts);
    let user = &req.messages().last().unwrap().text;
    assert!(user.contains("Thunderbird"));
    drop(store);
    let _ = std::fs::remove_dir_all(&dir);
}
