use super::*;
use std::sync::atomic::AtomicUsize;
use std::time::Duration;

fn receipt(value: bool) -> oneshot::Receiver<Receipt> {
    let (send, receive) = oneshot::channel();
    send.send(value.into()).unwrap();
    receive
}
#[tokio::test]
async fn completion_requires_every_ack_and_distinguishes_failure_from_timeout() {
    let deadline = Instant::now() + Duration::from_secs(1);
    assert_eq!(
        outcome(vec![receipt(true), receipt(true)], deadline).await,
        Feedback::Done
    );
    assert_eq!(
        outcome(vec![receipt(true), receipt(false)], deadline).await,
        Feedback::ExecutionFailed
    );
    assert_eq!(outcome(vec![], deadline).await, Feedback::ExecutionFailed);
    let (send, receive) = oneshot::channel();
    send.send(Receipt::Submitted).unwrap();
    assert_eq!(
        outcome(vec![receipt(true), receive], deadline).await,
        Feedback::Requested
    );
    let (send, receive) = oneshot::channel();
    drop(send);
    assert_eq!(outcome(vec![receive], deadline).await, Feedback::ExecutionFailed);
    let (_send, receive) = oneshot::channel();
    assert_eq!(
        outcome(vec![receipt(true), receive], Instant::now()).await,
        Feedback::Requested
    );
    // A known failure wins even when another result is unknown.
    let (_send, receive) = oneshot::channel();
    assert_eq!(
        outcome(vec![receive, receipt(false)], Instant::now()).await,
        Feedback::ExecutionFailed
    );
}

#[tokio::test]
async fn mixed_reads_and_actions_wait_for_ack_without_losing_the_read_answer() {
    for result in [Receipt::Completed, Receipt::Submitted, Receipt::Failed] {
        let now = Instant::now();
        let mut state = super::super::tests::fixture(now);
        let mut commands = Commands::new(now);
        let report = route(
            &mut commands,
            &mut state,
            "mute api and then who is waiting",
            true,
            now,
            serde_json::json!([]),
        )
        .unwrap();
        assert_eq!(report.feedback, Feedback::Answer);
        assert_eq!(report.effects.len(), 1);
        let answer = report.text.clone();
        let owner = Arc::new(crate::realtime::State::default());
        let (lease, _) = owner.begin().unwrap();
        let router = Router::new(Arc::new(lease), Arc::new(|_| true));
        let (reply, mut receive) = oneshot::channel();
        let mut completion = Completion::new(
            reply,
            report,
            router,
            now + Duration::from_secs(1),
            Arc::new(AtomicBool::new(true)),
        );
        let work = completion.work();
        completion.finish(Lang::En);
        tokio::time::sleep(Duration::from_millis(10)).await;
        assert!(matches!(
            receive.try_recv(),
            Err(oneshot::error::TryRecvError::Empty)
        ));
        assert!(work.live.load(Ordering::SeqCst));
        work.reply.send(result).unwrap();
        let report = receive.await.unwrap().unwrap().unwrap();
        match result {
            Receipt::Completed => {
                assert_eq!(report.feedback, Feedback::Answer);
                assert_eq!(report.text, answer);
            }
            Receipt::Submitted => assert_eq!(report.feedback, Feedback::Requested),
            Receipt::Failed => assert_eq!(report.feedback, Feedback::ExecutionFailed),
        }
    }
}
#[tokio::test]
async fn queued_work_rechecks_authority_deadline_and_waiter_while_retaining_its_owner() {
    let app = tauri::test::mock_builder()
        .build(tauri::test::mock_context(tauri::test::noop_assets()))
        .unwrap();
    let settings = Settings {
        dictation_commands: true,
        voice_realtime: crate::realtime::Consent {
            on: true,
            ledger: Some("test".into()),
            ..Default::default()
        },
        ..Default::default()
    };
    app.manage(crate::settings::SettingsState(Mutex::new(settings)));
    for mode in 0..8 {
        {
            let settings = app.state::<crate::settings::SettingsState>();
            let mut s = settings.0.lock().unwrap();
            s.dictation_commands = mode != 5;
            s.voice_realtime.on = mode != 6;
        }
        let state = Arc::new(crate::realtime::State::default());
        let (lease, _) = state.begin().unwrap();
        let router = Router::new(Arc::new(lease), Arc::new(move |_| mode != 1));
        let (reply, receive) = oneshot::channel();
        let report = Report {
            feedback: Feedback::Requested,
            text: String::new(),
            choices: vec![],
            effects: vec![],
            media: vec![],
        };
        let deadline = if mode == 2 {
            Instant::now()
        } else {
            Instant::now() + Duration::from_secs(1)
        };
        let waiting = Arc::new(AtomicBool::new(true));
        let mut completion = Some(Completion::new(reply, report, router, deadline, waiting.clone()));
        let work = completion.as_mut().unwrap().work();
        if mode == 3 {
            completion.take();
        }
        if mode == 4 {
            // A timed-out receiver closes admission even before Live drops.
            let receipts = std::mem::take(&mut completion.as_mut().unwrap().receipts);
            assert_eq!(outcome(receipts, Instant::now()).await, Feedback::Requested);
        }
        if mode == 7 {
            waiting.store(false, Ordering::SeqCst);
        }
        assert_eq!(state.begin().err(), Some(Failure::Busy));
        let calls = AtomicUsize::new(0);
        let executed = work.run(app.handle(), || {
            calls.fetch_add(1, Ordering::SeqCst);
            Receipt::Completed
        });
        assert_eq!(
            executed,
            if mode == 0 {
                Receipt::Completed
            } else {
                Receipt::Failed
            }
        );
        assert_eq!(calls.load(Ordering::SeqCst), usize::from(mode == 0));
        if mode == 0 {
            let completion = completion.take().unwrap();
            completion.finish(Lang::En);
            let report = receive.await.unwrap().unwrap().unwrap();
            assert_eq!(report.feedback, Feedback::Done);
            assert_eq!(
                report.text,
                vults_core::i18n::voice_feedback(Lang::En, Feedback::Done)
            );
        } else {
            drop(receive);
        }
        drop(completion);
        assert!(state.begin().is_ok());
    }
}
