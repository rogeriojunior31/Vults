use super::super::State;
use super::*;

fn fixture() -> (
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
            active: Arc::new(move || current.current(epoch)),
            cancel: cancelled,
            output,
        },
        stop: Arc::default(),
        record_for: Duration::from_secs(30),
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
