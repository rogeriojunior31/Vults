use super::*;
use std::sync::atomic::{AtomicBool, Ordering};
fn lease() -> Arc<super::super::Lease> {
    let state = Arc::new(super::super::State::default());
    Arc::new(state.begin().expect("test voice owner").0)
}
fn turn(value: i16) -> Turn {
    Turn::new(vec![value; 4800]).expect("bounded synthetic turn")
}

#[test]
fn completed_capture_keeps_its_clock_signal_and_original_local_words() {
    let rate = vults_audio::SAMPLE_RATE as usize;
    for frames in [rate / 10, rate / 10 + 1, rate - 1, rate, rate * 30 - 1, rate * 30] {
        let native = Turn::from_capture(&vec![0.0; frames], "A complete local sentence.".into()).unwrap();
        assert_eq!(
            native.samples.len(),
            (frames as u64 * RATE as u64 / rate as u64) as usize
        );
        assert!(native.samples.iter().all(|sample| *sample == 0));
    }
    let source: Vec<f32> = (0..rate)
        .map(|i| (i as f32 * std::f32::consts::TAU * 1000.0 / rate as f32).sin() * 0.25)
        .collect();
    let original = source.clone();
    let text = "  Private captured words.\nThe complete last sentence.  ";
    let native = Turn::from_capture(&source, text.into()).unwrap();
    let error: f32 = (2400..RATE - 2400)
        .map(|i| {
            let expected = (i as f32 * std::f32::consts::TAU * 1000.0 / RATE as f32).sin() * 8192.0;
            (f32::from(native.samples[i]) - expected).abs()
        })
        .sum::<f32>()
        / (RATE - 4800) as f32;
    assert!(
        error < 2.0,
        "Resampling must preserve pitch, phase and level: {error}"
    );
    assert_eq!(source, original);
    assert_eq!(native.transcript.as_deref(), Some(text));
    assert!(!format!("{native:?}").contains("Private captured words"));
}

#[test]
fn capture_assembly_refuses_invalid_audio_and_text_and_saturates_filter_peaks() {
    for frames in [0, 1599, 480001] {
        assert!(matches!(
            Turn::from_capture(&vec![0.0; frames], "Words.".into()),
            Err(Failure::Input)
        ));
    }
    for sample in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        let mut source = vec![0.0; 1600];
        source[800] = sample;
        assert!(matches!(
            Turn::from_capture(&source, "Words.".into()),
            Err(Failure::Input)
        ));
    }
    for text in [
        String::new(),
        " \n\t".into(),
        "bad\0text".into(),
        "x".repeat(4097),
        "\u{00e9}".repeat(2049),
    ] {
        assert!(matches!(
            Turn::from_capture(&vec![0.0; 1600], text),
            Err(Failure::Input)
        ));
    }
    for (sample, expected) in [
        (0.5, 16384_i16),
        (-0.5, -16384),
        (1.0, i16::MAX),
        (-1.0, i16::MIN),
        (1.1, i16::MAX),
        (-1.1, i16::MIN),
    ] {
        let native = Turn::from_capture(&vec![sample; 4800], "Words.".into()).unwrap();
        assert!(
            native.samples[2400..4800]
                .iter()
                .all(|s| (i32::from(*s) - i32::from(expected)).abs() <= 1)
        );
    }
}

#[tokio::test]
async fn captured_transcript_is_recovered_without_a_second_decode_and_never_after_revocation() {
    let text = "  An already finalized capture.\nKeep these exact words.  ";
    let native = Turn::from_capture(&vec![0.1; 3200], text.into()).unwrap();
    for reason in [
        Failure::Budget,
        Failure::Key,
        Failure::Transport,
        Failure::Timeout,
    ] {
        let (_sender, cancel) = watch::channel(false);
        let result = recover(
            native.clone(),
            reason,
            Arc::new(|| true),
            cancel,
            lease(),
            |_, _| panic!("A finalized native transcript must not load or decode a model again"),
        )
        .await
        .unwrap();
        assert!(
            matches!(result, Recovery::Text { text: returned, reason: returned_reason } if returned == text && returned_reason == reason)
        );
    }
    for (active, cancelled, handled) in [(false, false, false), (true, true, false), (true, false, true)] {
        let (_sender, cancel) = watch::channel(cancelled);
        let native = native.clone();
        if handled {
            native.consume();
        }
        assert!(
            recover(
                native,
                Failure::Key,
                Arc::new(move || active),
                cancel,
                lease(),
                |_, _| { panic!("Revoked or consumed captures must not reach decoding") }
            )
            .await
            .is_err()
        );
    }
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

#[tokio::test]
async fn handled_capture_clones_cannot_decode_or_return_as_reviewable_recovery() {
    let capture = turn(99);
    let original = capture.clone();
    capture.consume();
    assert!(original.handled());
    let (_send, cancel) = watch::channel(false);
    let result = recover(
        original,
        Failure::Timeout,
        Arc::new(|| true),
        cancel,
        lease(),
        |_, _| panic!("an accepted command must never reach recovery transcription"),
    )
    .await;
    assert!(matches!(result, Err(Failure::Input)));
}
