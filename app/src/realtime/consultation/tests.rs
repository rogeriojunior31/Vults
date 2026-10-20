use super::*;
use std::sync::atomic::AtomicUsize;

#[tokio::test(start_paused = true)]
async fn reassurance_waits_1500_ms_and_never_repeats_or_claims_completion() {
    let (_stop, mut stopped) = watch::channel(false);
    let (cancel, _cancelled) = watch::channel(false);
    let (reply, receive) = oneshot::channel();
    let notices = Arc::new(AtomicUsize::new(0));
    let count = notices.clone();
    let worker = tokio::spawn(async move {
        owned(
            async { receive.await.unwrap() },
            &mut stopped,
            cancel,
            || true,
            || {
                count.fetch_add(1, Ordering::SeqCst);
                true
            },
            tokio::time::Instant::now(),
        )
        .await
    });
    tokio::task::yield_now().await;
    tokio::time::advance(Duration::from_millis(1499)).await;
    assert_eq!(notices.load(Ordering::SeqCst), 0);
    tokio::time::advance(Duration::from_millis(1)).await;
    tokio::task::yield_now().await;
    assert_eq!(notices.load(Ordering::SeqCst), 1);
    tokio::time::advance(Duration::from_secs(1)).await;
    assert_eq!(notices.load(Ordering::SeqCst), 1);
    reply.send(Ok("a textual answer".into())).unwrap();
    assert_eq!(worker.await.unwrap().unwrap(), "a textual answer");
    for lang in [
        vults_core::i18n::Lang::En,
        vults_core::i18n::Lang::PtBr,
        vults_core::i18n::Lang::Es,
        vults_core::i18n::Lang::Zh,
    ] {
        assert!(
            !vults_core::i18n::voice_feedback(lang, crate::voice_commands::Feedback::Consulting).is_empty()
        );
    }
}

#[tokio::test(start_paused = true)]
async fn revocation_waiter_loss_and_deadline_retain_the_owner_until_child_acknowledgement() {
    for mode in 0..4 {
        let owner = Arc::new(super::super::State::default());
        let (lease, _) = owner.begin().unwrap();
        let (stop, mut stopped) = watch::channel(false);
        let (cancel, mut cancellation) = watch::channel(false);
        let active = Arc::new(AtomicBool::new(true));
        let valid = active.clone();
        let (started, start) = oneshot::channel();
        let (seen, cancelled) = oneshot::channel();
        let (release, wait) = oneshot::channel();
        let busy = Arc::new(AtomicBool::new(false));
        let guard = Busy::acquire(&busy).unwrap();
        let worker = tokio::spawn(async move {
            let _busy = guard;
            let _lease = lease;
            owned(
                async {
                    started.send(()).unwrap();
                    cancellation.changed().await.unwrap();
                    assert!(*cancellation.borrow());
                    seen.send(()).unwrap();
                    wait.await.unwrap();
                    Ok("obsolete answer".into())
                },
                &mut stopped,
                cancel,
                || valid.load(Ordering::SeqCst),
                || true,
                tokio::time::Instant::now(),
            )
            .await
        });
        start.await.unwrap();
        match mode {
            0 => {
                stop.send_replace(true);
            }
            1 => {
                active.store(false, Ordering::SeqCst);
            }
            2 => drop(stop),
            _ => {
                tokio::time::advance(LIMIT + Duration::from_millis(25)).await;
            }
        }
        cancelled.await.unwrap();
        assert!(!worker.is_finished());
        assert!(
            Busy::acquire(&busy).is_err(),
            "the other provider shares this slot"
        );
        assert_eq!(
            owner.begin().err(),
            Some(vults_chat::realtime::transport::Failure::Busy)
        );
        release.send(()).unwrap();
        assert!(worker.await.unwrap().is_err());
        assert!(owner.begin().is_ok());
        assert!(Busy::acquire(&busy).is_ok());
    }
}

#[tokio::test(start_paused = true)]
async fn short_answers_skip_reassurance_and_invalid_or_obsolete_answers_never_publish() {
    for text in [
        Ok("answer".to_string()),
        Ok("  ".to_string()),
        Err("private provider detail".to_string()),
    ] {
        let (_stop, mut stopped) = watch::channel(false);
        let (cancel, _receive) = watch::channel(false);
        let result = owned(
            std::future::ready(text.clone()),
            &mut stopped,
            cancel,
            || true,
            || panic!("no notice for a fast answer"),
            tokio::time::Instant::now(),
        )
        .await;
        assert_eq!(result.is_ok(), text.is_ok_and(|t| !t.trim().is_empty()));
        if let Err(error) = result {
            assert!(!error.contains("private provider detail"));
        }
    }
    for input in [
        json!({}),
        json!({"question":""}),
        json!({"question":"\0"}),
        json!({"question":"hello", "tool":"run"}),
        json!({"question":"x".repeat(257)}),
    ] {
        assert!(question(&input).is_err());
    }
    assert_eq!(
        question(&json!({"question":"Explain this error."})).unwrap(),
        "Explain this error."
    );
    let dir = Directory::new().unwrap();
    assert!(vults_peer::is_private_dir(&dir.0));
    assert!(std::fs::read_dir(&dir.0).unwrap().next().is_none());
    let path = dir.0.clone();
    drop(dir);
    assert!(!path.exists());
}

#[tokio::test(start_paused = true)]
async fn slow_admission_uses_the_original_clock_and_cannot_start_a_child_after_cancellation() {
    let owner = Arc::new(super::super::State::default());
    let (lease, _) = owner.begin().unwrap();
    let started = tokio::time::Instant::now();
    let (_stop, mut stopped) = watch::channel(false);
    let (cancel, cancellation) = watch::channel(false);
    let (admit, admission) = oneshot::channel();
    let notices = Arc::new(AtomicUsize::new(0));
    let count = notices.clone();
    let calls = Arc::new(AtomicUsize::new(0));
    let invoked = calls.clone();
    tokio::time::advance(Duration::from_secs(1)).await;
    let worker = tokio::spawn(async move {
        let _lease = lease;
        owned(
            async {
                admission.await.unwrap();
                if *cancellation.borrow() {
                    return Err("cancelled before CLI construction".into());
                }
                invoked.fetch_add(1, Ordering::SeqCst);
                Ok("answer".into())
            },
            &mut stopped,
            cancel,
            || true,
            || {
                count.fetch_add(1, Ordering::SeqCst);
                true
            },
            started,
        )
        .await
    });
    tokio::task::yield_now().await;
    tokio::time::advance(Duration::from_millis(500)).await;
    tokio::task::yield_now().await;
    assert_eq!(
        notices.load(Ordering::SeqCst),
        1,
        "notice includes admission and scheduling time"
    );
    tokio::time::advance(LIMIT).await;
    tokio::task::yield_now().await;
    assert!(
        !worker.is_finished(),
        "cancellation retains uninterruptible admission ownership"
    );
    assert_eq!(
        owner.begin().err(),
        Some(vults_chat::realtime::transport::Failure::Busy)
    );
    admit.send(()).unwrap();
    assert!(worker.await.unwrap().is_err());
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert!(owner.begin().is_ok());
}

#[test]
fn provider_consent_and_ledger_are_independent() {
    let mut settings = crate::settings::Settings {
        voice_consultation_budget: Some("claude-ledger".into()),
        voice_codex_consultation_budget: Some("codex-ledger".into()),
        ..Default::default()
    };
    for claude in [false, true] {
        for codex in [false, true] {
            settings.voice_consultation = claude;
            settings.voice_codex_consultation = codex;
            assert_eq!(
                Provider::Claude.ledger(&settings).map(String::as_str),
                claude.then_some("claude-ledger")
            );
            assert_eq!(
                Provider::Codex.ledger(&settings).map(String::as_str),
                codex.then_some("codex-ledger")
            );
        }
    }
    settings.voice_codex_consultation_budget = None;
    assert!(Provider::Codex.ledger(&settings).is_none());
    assert_eq!(Provider::Claude.name(), "consult_claude");
    assert_eq!(Provider::Codex.name(), "consult_codex");
}
