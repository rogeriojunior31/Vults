use super::super::State;
use super::*;

pub(super) fn fixture() -> (
    CaptureControl,
    Worker,
    mpsc::Receiver<Event>,
    watch::Sender<bool>,
    Arc<State>,
) {
    let state = Arc::new(State::default());
    let (lease, epoch) = state.begin().unwrap();
    let current = state.clone();
    let (output, events) = mpsc::channel(16);
    let (cancel, cancelled) = watch::channel(false);
    let (control, request) = capture_control();
    let worker = Worker {
        end: request.0,
        owner: Owner {
            lease: Arc::new(lease),
            semantic: None,
            active: Arc::new(move || current.current(epoch)),
            cancel: cancelled,
            output,
        },
        stop: Arc::default(),
        record_for: Duration::from_secs(30),
        until: None,
        expired: Arc::default(),
    };
    (control, worker, events, cancel, state)
}

#[test]
fn controls_open_nothing_finish_is_sticky_and_cancel_cannot_be_restored() {
    let (control, request) = capture_control();
    let active = request.active();
    assert!(active());
    control.finish();
    control.finish();
    assert_eq!(*request.0.borrow(), End::Finished);
    assert!(active());
    assert!(control.restart());
    assert!(!control.restart());
    control.finish();
    control.cancel();
    control.finish();
    assert_eq!(*request.0.borrow(), End::Cancelled);
    assert!(!active());
    assert!(!control.restart());
    let (control, request) = capture_control();
    let active = request.active();
    drop(control);
    assert!(!active());
}

#[test]
fn explicit_finish_closes_audio_before_decode_and_retains_its_same_owner() {
    let (control, worker, mut events, _cancel, state) = fixture();
    let closed = Arc::new(AtomicBool::new(false));
    let finished = closed.clone();
    let text = "The exact finalized local turn.";
    let result = worker
        .record(
            |live, admit| {
                assert!(live() && admit());
                control.finish();
                Ok(())
            },
            |_| {
                finished.store(true, Ordering::SeqCst);
                Ok(vec![0.1; 3200])
            },
            |pcm, stop| {
                assert!(closed.load(Ordering::SeqCst));
                assert!(!stop.load(Ordering::SeqCst));
                assert_eq!(pcm.len(), 3200);
                assert_eq!(state.begin().err(), Some(Failure::Busy));
                Ok(text.into())
            },
        )
        .unwrap();
    assert!(format!("{result:?}").contains("frames: 4800"));
    assert!(!format!("{result:?}").contains(text));
    assert!(matches!(
        events.try_recv().unwrap(),
        Event::Capture(CapturePhase::Starting)
    ));
    assert!(matches!(
        events.try_recv().unwrap(),
        Event::Capture(CapturePhase::Recording)
    ));
    assert!(matches!(
        events.try_recv().unwrap(),
        Event::Capture(CapturePhase::Transcribing)
    ));
    assert!(events.try_recv().is_err());
}

#[test]
fn cancelled_or_departed_requests_never_open_a_device() {
    for mode in 0..4 {
        let (control, worker, _events, cancel, _state) = fixture();
        match mode {
            0 => control.cancel(),
            1 => control.finish(),
            2 => drop(control),
            _ => drop(cancel),
        }
        assert!(
            worker
                .record(
                    |_, _| -> Result<(), Failure> { panic!("Revoked capture cannot open a device") },
                    |_| panic!("No device exists"),
                    |_, _| panic!("No decode is allowed"),
                )
                .is_err()
        );
    }
}

#[test]
fn cancellation_limit_and_failed_publication_never_decode_or_publish_a_turn() {
    let (control, worker, events, _cancel, _state) = fixture();
    drop(events);
    assert!(matches!(
        worker.record(
            |_, _| -> Result<(), Failure> { panic!("No consumer means no capture") },
            |_| panic!("No capture"),
            |_, _| panic!("No decode"),
        ),
        Err(Failure::Output)
    ));
    drop(control);
    let (late_control, mut worker, _events, _cancel, _state) = fixture();
    worker.record_for = Duration::from_millis(1);
    assert!(matches!(
        worker.record(
            |_, _| Ok(()),
            |_| {
                late_control.finish();
                Ok(vec![0.0; 3200])
            },
            |_, _| panic!("A late Finish after the hard limit cannot decode/upload"),
        ),
        Err(Failure::Timeout)
    ));
    let (control, worker, _events, _cancel, state) = fixture();
    assert!(matches!(
        worker.record(
            |_, _| {
                control.finish();
                Ok(())
            },
            |_| Ok(vec![0.0; 3200]),
            |_, _| {
                state.key_changed("openai");
                Ok("late words".into())
            },
        ),
        Err(Failure::Off)
    ));
    let (control, worker, _events, _cancel, _state) = fixture();
    assert!(matches!(
        worker.record(
            |_, _| {
                control.finish();
                Ok(())
            },
            |_| Ok(vec![0.0; 3200]),
            |_, _| {
                control.cancel();
                Ok("cancelled words".into())
            },
        ),
        Err(Failure::Cancelled)
    ));
}

#[tokio::test]
async fn abandoning_or_timing_out_native_work_keeps_the_lease_until_it_really_exits() {
    for timeout in [false, true] {
        let (_control, worker, _events, _cancel, state) = fixture();
        let stop = worker.stop.clone();
        let release = Arc::new((std::sync::Mutex::new(false), std::sync::Condvar::new()));
        let waiting = release.clone();
        let (started, entered) = tokio::sync::oneshot::channel();
        let task = tokio::spawn(worker.execute(move |_| {
            started.send(()).unwrap();
            let mut unlocked = waiting.0.lock().unwrap();
            while !*unlocked {
                unlocked = waiting.1.wait(unlocked).unwrap();
            }
            Turn::from_capture(&vec![0.0; 3200], "late local words".into())
        }));
        entered.await.unwrap();
        if timeout {
            tokio::time::pause();
            tokio::time::advance(Duration::from_secs(61)).await;
            assert!(matches!(task.await.unwrap(), Err(Failure::Timeout)));
            tokio::time::resume();
        } else {
            task.abort();
            assert!(task.await.unwrap_err().is_cancelled());
        }
        assert!(stop.load(Ordering::SeqCst));
        assert_eq!(state.begin().err(), Some(Failure::Busy));
        *release.0.lock().unwrap() = true;
        release.1.notify_all();
        tokio::time::timeout(Duration::from_secs(2), async {
            while state.running.load(Ordering::SeqCst) {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        assert!(state.begin().is_ok());
    }
}

#[test]
fn automatic_completion_closes_capture_before_decode_but_never_overrides_cancel_or_the_cap() {
    for mode in 0..3 {
        let (control, mut worker, _events, _cancel, _state) = fixture();
        if mode == 2 {
            worker.record_for = Duration::from_millis(1);
        }
        let closed = AtomicBool::new(false);
        let result = worker.record_watched(
            |_, _| Ok(()),
            |_| {
                closed.store(true, Ordering::SeqCst);
                Ok(vec![0.1; 3200])
            },
            |_, _| {
                assert!(closed.load(Ordering::SeqCst));
                assert_eq!(mode, 0, "Cancelled/expired automatic completion cannot decode");
                Ok("Final local words".into())
            },
            |_, _| {
                if mode == 1 {
                    control.cancel();
                }
                if mode == 2 {
                    std::thread::sleep(Duration::from_millis(2));
                }
                true
            },
        );
        match mode {
            0 => assert!(result.is_ok()),
            1 => assert!(matches!(result, Err(Failure::Cancelled))),
            _ => assert!(matches!(result, Err(Failure::Timeout))),
        }
        assert!(closed.load(Ordering::SeqCst));
    }
}

#[test]
fn bounded_capture_closes_and_discards_at_the_original_deadline_without_decoding() {
    let (_control, mut worker, _events, _cancel, _state) = fixture();
    worker.until = Some(Instant::now() + Duration::from_millis(35));
    let until = worker.until.unwrap();
    let closed = Arc::new(AtomicBool::new(false));
    let finished = closed.clone();
    let result = worker.record(
        |live, admit| {
            assert!(live() && admit());
            Ok(())
        },
        move |_| {
            finished.store(true, Ordering::SeqCst);
            Ok(vec![0.1; 3200])
        },
        |_, _| panic!("Expiry must not decode or create an uploadable turn"),
    );
    assert!(matches!(result, Err(Failure::Timeout)));
    assert!(closed.load(Ordering::SeqCst));
    assert!(worker.expired.load(Ordering::SeqCst));
    assert!(Instant::now() >= until);
    let (_control, mut worker, _events, _cancel, _state) = fixture();
    worker.until = Some(Instant::now());
    assert!(matches!(
        worker.record(
            |_, _| -> Result<(), Failure> { panic!("Late admission cannot open a microphone") },
            |_| unreachable!(),
            |_, _| unreachable!()
        ),
        Err(Failure::Timeout)
    ));
    assert!(worker.expired.load(Ordering::SeqCst));
}

#[test]
fn bounded_microphone_guard_expires_even_if_native_initialization_is_delayed() {
    let (_control, mut worker, _events, _cancel, _state) = fixture();
    worker.until = Some(Instant::now() + Duration::from_millis(20));
    let result = worker.record(
        |live, admit| -> Result<(), Failure> {
            std::thread::sleep(Duration::from_millis(30));
            assert!(
                !live() && !admit(),
                "Both native admission checks retain the original deadline"
            );
            Err(Failure::Input)
        },
        |_| unreachable!(),
        |_, _| unreachable!(),
    );
    assert!(matches!(result, Err(Failure::Timeout)));
    assert!(worker.expired.load(Ordering::SeqCst));
}

#[test]
fn cancellation_at_expiry_wins_and_valid_finish_can_decode_after_microphone_closure() {
    let (control, mut worker, _events, _cancel, _state) = fixture();
    worker.until = Some(Instant::now());
    control.cancel();
    assert!(matches!(
        worker.record(
            |_, _| -> Result<(), Failure> { panic!("Cancelled capture cannot open") },
            |_| unreachable!(),
            |_, _| unreachable!()
        ),
        Err(Failure::Cancelled)
    ));
    assert!(!worker.expired.load(Ordering::SeqCst));
    let (control, mut worker, _events, _cancel, _state) = fixture();
    worker.until = Some(Instant::now() + Duration::from_millis(100));
    let closed = Arc::new(AtomicBool::new(false));
    let finished = closed.clone();
    let result = worker.record(
        |_, _| {
            control.finish();
            Ok(())
        },
        move |_| {
            finished.store(true, Ordering::SeqCst);
            Ok(vec![0.1; 3200])
        },
        |_, _| {
            assert!(closed.load(Ordering::SeqCst));
            Ok("An explicitly finalized answer".into())
        },
    );
    assert!(result.is_ok());
    assert!(!worker.expired.load(Ordering::SeqCst));
}

#[tokio::test]
async fn expired_continuation_keeps_the_driver_and_cannot_send_an_audio_turn() {
    let (next, receive) = mpsc::channel(2);
    let (send, mut output) = mpsc::channel(2);
    let generation = vults_voice::turn::Turn::default()
        .start(Duration::ZERO)
        .unwrap()
        .generation;
    next.send(Request::FollowUp {
        generation,
        until: Instant::now(),
    })
    .await
    .unwrap();
    next.send(Request::Capture(generation)).await.unwrap();
    let (finish, done) = tokio::sync::oneshot::channel();
    let mut finish = Some(finish);
    let result = continue_turns(
        async {
            done.await.unwrap();
            Ok(None)
        },
        receive,
        send,
        move |value, until| {
            assert_eq!(value, generation);
            if until.is_some() {
                std::future::ready(Ok(None))
            } else {
                finish.take().unwrap().send(()).unwrap();
                std::future::ready(
                    Turn::from_capture(&vec![0.1; 3200], "A new explicit answer".into()).map(Some),
                )
            }
        },
    )
    .await;
    assert!(result.unwrap().is_none());
    assert!(
        output.try_recv().is_err(),
        "Expiry cannot publish a turn; the driver wins over unfinished following work"
    );
}
