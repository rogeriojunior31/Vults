use super::*;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use vults_chat::realtime::Settlement;

fn channels() -> (
    Channels,
    mpsc::Sender<Turn>,
    mpsc::Receiver<Event>,
    watch::Sender<bool>,
) {
    let (send, input) = mpsc::channel(1);
    let (output, receive) = mpsc::channel(8);
    let (stop, cancel) = watch::channel(false);
    (
        Channels {
            input,
            output,
            cancel,
        },
        send,
        receive,
        stop,
    )
}
fn lease() -> Arc<super::super::Lease> {
    let state = Arc::new(super::super::State::default());
    Arc::new(state.begin().expect("test voice owner").0)
}
fn turn(value: i16) -> Turn {
    Turn::new(vec![value; 4800]).expect("bounded synthetic turn")
}
async fn upload(io: &mut transport::Channels) -> Vec<i16> {
    let mut pcm = Vec::new();
    loop {
        match io.input.recv().await.expect("turn command") {
            Command::Audio(chunk) => {
                assert!(chunk.len() <= CHUNK);
                pcm.extend(chunk);
            }
            Command::Commit => return pcm,
            Command::Interrupt => panic!("unexpected interrupt"),
        }
    }
}
async fn completed(io: &mut transport::Channels) {
    io.output
        .send(Event::Finished(Settlement {
            attempt: "synthetic".into(),
            actual_microusd: Some(10),
            ok: true,
            within_bounds: true,
        }))
        .await
        .expect("receipt");
    io.output.send(Event::Ready).await.expect("ready");
}

#[tokio::test]
async fn admission_failure_retains_the_complete_turn_and_local_recovery_preserves_the_whole_text() {
    let (io, send, _receive, _stop) = channels();
    let cancel = io.cancel.clone();
    send.send(turn(31415)).await.unwrap();
    let (saved, reason) = drive(io, &|| true, |mut remote| async move {
        assert!(remote.input.try_recv().is_err());
        Err(Failure::Budget)
    })
    .await
    .unwrap()
    .unwrap();
    assert_eq!(saved.samples().as_ref(), vec![31415; 4800]);
    assert_eq!(reason, Failure::Budget);
    let full = "  A complete local sentence.\nAnd the last word.  ";
    let recovery = recover(
        saved,
        reason,
        Arc::new(|| true),
        cancel,
        lease(),
        move |pcm, flag| {
            assert_eq!(pcm.len(), 4800);
            assert!(!flag.load(Ordering::SeqCst));
            Ok(full.into())
        },
    )
    .await
    .unwrap();
    assert!(matches!(&recovery, Recovery::Text { reason: Failure::Budget, text } if text == full));
    assert!(!format!("{recovery:?}").contains("sentence"));
}

#[tokio::test]
async fn one_connection_retains_only_the_failed_turn_across_successful_receipts() {
    let (io, send, mut receive, _stop) = channels();
    let drain = tokio::spawn(async move { while receive.recv().await.is_some() {} });
    let first = turn(123);
    let first_weak = Arc::downgrade(&first.0);
    send.send(first).await.unwrap();
    let calls = Arc::new(AtomicUsize::new(0));
    let count = calls.clone();
    let worker = tokio::spawn(async move {
        drive(io, &|| true, move |mut remote| {
            count.fetch_add(1, Ordering::SeqCst);
            async move {
                remote.output.send(Event::Ready).await.unwrap();
                assert_eq!(upload(&mut remote).await, vec![123; 4800]);
                completed(&mut remote).await;
                assert_eq!(upload(&mut remote).await, vec![456; 4800]);
                assert!(first_weak.upgrade().is_none(), "successful PCM must be released");
                Err(Failure::Transport)
            }
        })
        .await
    });
    send.send(turn(456)).await.unwrap();
    let (saved, reason) = worker.await.unwrap().unwrap().unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(saved.samples().as_ref(), vec![456; 4800]);
    assert_eq!(reason, Failure::Transport);
    drain.await.unwrap();
}

#[tokio::test]
async fn an_idle_failure_never_recovers_a_previously_completed_turn() {
    let (io, send, _receive, _stop) = channels();
    send.send(turn(123)).await.unwrap();
    let result = drive(io, &|| true, |mut remote| async move {
        remote.output.send(Event::Ready).await.unwrap();
        upload(&mut remote).await;
        completed(&mut remote).await;
        Err(Failure::Timeout)
    })
    .await;
    assert!(matches!(result, Err(Failure::Timeout)));
}

#[tokio::test]
async fn cancellation_stops_the_provider_and_never_offers_recovery() {
    let (io, send, _receive, stop) = channels();
    send.send(turn(123)).await.unwrap();
    let (entered, seen) = tokio::sync::oneshot::channel();
    let ended = Arc::new(AtomicBool::new(false));
    let done = ended.clone();
    let worker = tokio::spawn(async move {
        drive(io, &|| true, |mut remote| async move {
            remote.output.send(Event::Ready).await.unwrap();
            upload(&mut remote).await;
            entered.send(()).unwrap();
            remote.cancel.changed().await.unwrap();
            done.store(true, Ordering::SeqCst);
            Err(Failure::Cancelled)
        })
        .await
    });
    seen.await.unwrap();
    stop.send(true).unwrap();
    assert!(matches!(worker.await.unwrap(), Err(Failure::Cancelled)));
    assert!(ended.load(Ordering::SeqCst));
}

#[tokio::test]
async fn local_failure_retains_audio_and_revocation_discards_late_decode_results() {
    for (decoded, expected) in [
        (Ok(String::new()), LocalFailure::Empty),
        (Err(LocalFailure::Unavailable), LocalFailure::Unavailable),
        (Err(LocalFailure::Decode), LocalFailure::Decode),
        (Ok("x".repeat(65537)), LocalFailure::Decode),
    ] {
        let (_send, cancel) = watch::channel(false);
        let result = recover(
            turn(99),
            Failure::Key,
            Arc::new(|| true),
            cancel,
            lease(),
            move |_, _| decoded,
        )
        .await
        .unwrap();
        assert!(
            matches!(result, Recovery::Audio { turn, failure, .. } if failure == expected && turn.0.len() == 4800)
        );
    }
    let (_send, cancel) = watch::channel(false);
    let active = Arc::new(AtomicBool::new(true));
    let live = active.clone();
    let result = recover(
        turn(99),
        Failure::Timeout,
        Arc::new(move || live.load(Ordering::SeqCst)),
        cancel,
        lease(),
        move |_, _| {
            active.store(false, Ordering::SeqCst);
            Ok("obsolete local text".into())
        },
    )
    .await;
    assert!(matches!(result, Err(Failure::Off)));
}

#[test]
fn turn_limits_and_debug_never_expose_audio() {
    assert!(Turn::new(vec![0; CHUNK - 1]).is_err());
    assert!(Turn::new(vec![0; MAX_FRAMES + 1]).is_err());
    assert!(Turn::new(vec![0; MAX_FRAMES]).is_ok());
    assert!(!format!("{:?}", turn(31415)).contains("31415"));
}

#[tokio::test]
async fn successful_decode_keeps_the_owner_until_the_result_is_checked() {
    let state = Arc::new(super::super::State::default());
    let (lease, epoch) = state.begin().unwrap();
    let live = state.clone();
    let (_send, cancel) = watch::channel(false);
    let result = recover(
        turn(99),
        Failure::Key,
        Arc::new(move || live.current(epoch)),
        cancel,
        Arc::new(lease),
        |_, _| Ok("complete text".into()),
    )
    .await
    .unwrap();
    assert!(matches!(result, Recovery::Text { .. }));
    assert!(!state.current(epoch));
}

#[tokio::test]
async fn timed_out_native_work_keeps_its_lease_and_original_audio_until_it_exits() {
    let state = Arc::new(super::super::State::default());
    let (lease, epoch) = state.begin().unwrap();
    let live = state.clone();
    let (_send, cancel) = watch::channel(false);
    let release = Arc::new((std::sync::Mutex::new(false), std::sync::Condvar::new()));
    let wait = release.clone();
    let (entered, seen) = tokio::sync::oneshot::channel();
    let worker = tokio::spawn(recover(
        turn(99),
        Failure::Transport,
        Arc::new(move || live.current(epoch)),
        cancel,
        Arc::new(lease),
        move |_, stop| {
            entered.send(stop).unwrap();
            let _wait = wait
                .1
                .wait_timeout_while(wait.0.lock().unwrap(), Duration::from_secs(3), |done| !*done)
                .unwrap();
            Ok("late text".into())
        },
    ));
    let flag = seen.await.unwrap();
    tokio::time::pause();
    tokio::time::advance(Duration::from_secs(31)).await;
    tokio::time::resume();
    let result = worker.await.unwrap().unwrap();
    assert!(
        matches!(result, Recovery::Audio { failure: LocalFailure::Timeout, turn, .. } if turn.0.len() == 4800)
    );
    assert!(flag.load(Ordering::SeqCst));
    assert_eq!(state.begin().err(), Some(Failure::Busy));
    *release.0.lock().unwrap() = true;
    release.1.notify_all();
    tokio::time::timeout(Duration::from_secs(2), async {
        while state.current(epoch) {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert!(state.begin().is_ok());
}

#[tokio::test]
async fn failed_receipts_and_output_saturation_keep_the_pending_original() {
    for saturation in [false, true] {
        let (mut io, send, _receive, _stop) = channels();
        let (_tiny, tiny_receive) = mpsc::channel(1);
        let _keep = if saturation {
            io.output = _tiny;
            Some(tiny_receive)
        } else {
            None
        };
        send.send(turn(42)).await.unwrap();
        let (saved, reason) = drive(io, &|| true, move |mut remote| async move {
            remote.output.send(Event::Ready).await.unwrap();
            upload(&mut remote).await;
            if saturation {
                remote
                    .output
                    .send(Event::Audio {
                        attempt: "synthetic".into(),
                        pcm: vec![1],
                    })
                    .await
                    .unwrap();
                remote.cancel.changed().await.unwrap();
            } else {
                remote
                    .output
                    .send(Event::Finished(Settlement {
                        attempt: "synthetic".into(),
                        actual_microusd: Some(5),
                        ok: false,
                        within_bounds: true,
                    }))
                    .await
                    .unwrap();
            }
            Err(Failure::Transport)
        })
        .await
        .unwrap()
        .unwrap();
        assert_eq!(saved.samples().as_ref(), vec![42; 4800]);
        assert_eq!(
            reason,
            if saturation {
                Failure::Output
            } else {
                Failure::Response
            }
        );
    }
}
