use super::*;
use std::sync::atomic::{AtomicBool, Ordering};
fn lease() -> Arc<super::super::Lease> {
    let state = Arc::new(super::super::State::default());
    Arc::new(state.begin().expect("test voice owner").0)
}
fn turn(value: i16) -> Turn {
    Turn::new(vec![value; 4800]).expect("bounded synthetic turn")
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
            matches!(result, Recovery::Audio { turn, failure, .. } if failure == expected && turn.samples.len() == 4800)
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
        matches!(result, Recovery::Audio { failure: LocalFailure::Timeout, turn, .. } if turn.samples.len() == 4800)
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
async fn successful_recovery_preserves_the_whole_text_without_sending_it() {
    let (_stop, cancel) = watch::channel(false);
    let full = "  A complete local sentence.\nAnd the last word.  ";
    let recovery = recover(
        turn(31415),
        Failure::Budget,
        Arc::new(|| true),
        cancel,
        lease(),
        move |pcm, stop| {
            assert_eq!(pcm.as_ref(), vec![31415; 4800]);
            assert!(!stop.load(Ordering::SeqCst));
            Ok(full.into())
        },
    )
    .await
    .unwrap();
    assert!(matches!(&recovery, Recovery::Text { text, .. } if text == full));
    assert!(!format!("{recovery:?}").contains("sentence"));
}
