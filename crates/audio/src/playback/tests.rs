use super::*;

fn playback(rate: u32) -> Playback {
    let mut queue = Queue::new(rate);
    queue.phase = Phase::Playing;
    Playback {
        shared: Arc::new(Shared {
            queue: Mutex::new(queue),
            producer: Mutex::new(Producer {
                converter: Some(Converter::at_rate(RATE, rate).expect("test resampler")),
            }),
            cancel: Arc::default(),
            stopped: AtomicBool::new(false),
            created: Instant::now(),
        }),
    }
}

#[test]
fn device_latency_counts_only_played_frames_and_stop_freezes_before_clearing() {
    let output = playback(48_000);
    output.push(&vec![16384; 2400]).unwrap();
    output.finish().unwrap();
    let now = Instant::now();
    let mut data = vec![0_f32; 960];
    render(
        &mut data,
        2,
        48_000,
        &output.shared,
        now,
        Duration::from_millis(50),
    );
    assert!(data.iter().all(|s| s.is_finite()));
    assert!(data.iter().any(|s| s.abs() > 0.1));
    assert!(data.as_chunks::<2>().0.iter().all(|frame| frame[0] == frame[1]));
    let early = output.shared.status(now + Duration::from_millis(49));
    assert_eq!(
        (early.total_frames, early.submitted_frames, early.played_frames),
        (2400, 240, 0)
    );
    assert!(early.first_played.is_none());
    let partial = output.shared.status(now + Duration::from_millis(55));
    assert_eq!(partial.played_frames, 120);
    assert_eq!(partial.first_played, Some(now + Duration::from_millis(50)));
    let mut queue = output.shared.queue.lock().unwrap();
    queue.halt(Phase::Stopped, None, now + Duration::from_millis(55));
    assert!(queue.pcm.is_empty() && queue.marks.is_empty());
    queue.advance(now + Duration::from_secs(10));
    assert_eq!(queue.cursor().played_frames, 120);
}

#[test]
fn empty_callback_is_not_drain_and_producer_end_waits_for_the_device() {
    let output = playback(RATE);
    output.push(&vec![0; 2400]).unwrap();
    let now = Instant::now();
    let mut data = vec![1_f32; 2400];
    render(&mut data, 1, RATE, &output.shared, now, Duration::ZERO);
    let consumed = output.shared.status(now + Duration::from_millis(100));
    assert_eq!(consumed.phase, Phase::Playing);
    assert_eq!(consumed.played_frames, 2400);
    render(
        &mut data,
        1,
        RATE,
        &output.shared,
        now + Duration::from_millis(100),
        Duration::ZERO,
    );
    assert!(data.iter().all(|s| *s == 0.0));
    assert_eq!(
        output
            .shared
            .status(now + Duration::from_millis(200))
            .played_frames,
        2400
    );
    output.finish().unwrap();
    assert_eq!(
        output.shared.status(now + Duration::from_millis(201)).phase,
        Phase::Finished
    );

    let second = playback(RATE);
    second.push(&vec![1; 2400]).unwrap();
    second.finish().unwrap();
    render(&mut data, 1, RATE, &second.shared, now, Duration::from_millis(30));
    assert_eq!(
        second.shared.status(now + Duration::from_millis(129)).phase,
        Phase::Draining
    );
    let done = second.shared.status(now + Duration::from_millis(130));
    assert_eq!(done.phase, Phase::Finished);
    assert_eq!(done.played_frames, 2400);
}

#[test]
fn callbacks_insert_silence_for_contention_and_revocation_never_consumes_more() {
    let output = playback(RATE);
    output.push(&[8192; 2400]).unwrap();
    let now = Instant::now();
    let mut data = [1_f32; 24];
    let held = output.shared.queue.lock().unwrap();
    render(&mut data, 1, RATE, &output.shared, now, Duration::ZERO);
    assert_eq!(data, [0_f32; 24]);
    drop(held);
    assert_eq!(output.cursor().submitted_frames, 0);
    output.shared.cancel.store(true, Ordering::Release);
    render(&mut data, 1, RATE, &output.shared, now, Duration::ZERO);
    assert_eq!(data, [0_f32; 24]);
    assert_eq!(output.cursor().phase, Phase::Stopped);
    assert_eq!(output.cursor().played_frames, 0);
    assert_eq!(output.push(&[1]), Err(Failure::Cancelled));
    assert!(matches!(
        Playback::start(Arc::new(AtomicBool::new(true))),
        Err(Failure::Cancelled)
    ));
}

#[test]
fn invalid_chunks_late_writes_and_bad_clock_never_change_the_cursor() {
    let output = playback(RATE);
    assert_eq!(output.push(&[]), Err(Failure::Input));
    assert_eq!(output.push(&vec![0; RATE as usize + 1]), Err(Failure::Input));
    output.shared.queue.lock().unwrap().total = MAX_FRAMES;
    assert_eq!(output.push(&[1]), Err(Failure::Input));
    assert_eq!(output.cursor().total_frames, MAX_FRAMES);
    output.shared.queue.lock().unwrap().total = 0;
    output.push(&[1; 2400]).unwrap();
    output.finish().unwrap();
    assert_eq!(output.push(&[1]), Err(Failure::State));
    assert_eq!(output.finish(), Err(Failure::State));
    let mut data = [1_f32; 16];
    render(
        &mut data,
        2,
        RATE,
        &output.shared,
        Instant::now(),
        MAX_LATENCY + Duration::from_millis(1),
    );
    assert_eq!(output.cursor().failure, Some(Failure::Clock));
    assert_eq!(output.cursor().played_frames, 0);
    assert_eq!(data, [0_f32; 16]);
}

#[test]
fn fractional_resampling_and_callback_jitter_keep_an_ordered_bounded_timeline() {
    for rate in [8_000, 16_000, 24_000, 44_100, 48_000, 96_000] {
        let output = playback(rate);
        output.push(&[16384; 2401]).unwrap();
        output.finish().unwrap();
        let wanted = 2401_u64 * u64::from(rate) / u64::from(RATE);
        let now = Instant::now();
        let delay = Duration::from_millis(50);
        let mut data = vec![0_f32; wanted as usize];
        render(&mut data, 1, rate, &output.shared, now, delay);
        let done = output.shared.status(now + delay + frame_time(wanted, rate));
        assert_eq!(done.phase, Phase::Finished, "rate {rate}");
        assert_eq!(done.played_frames, 2401);
    }
    let output = playback(RATE);
    output.push(&[1; 2400]).unwrap();
    let now = Instant::now();
    let mut data = [0_f32; 240];
    render(&mut data, 1, RATE, &output.shared, now, Duration::from_millis(50));
    render(
        &mut data,
        1,
        RATE,
        &output.shared,
        now + Duration::from_micros(9990),
        Duration::from_millis(50),
    );
    let queue = output.shared.queue.lock().unwrap();
    assert_eq!(queue.marks[1].at, now + Duration::from_millis(60));
    assert!(queue.failure.is_none());
}

#[test]
fn unsupported_configs_and_start_deadlines_fail_closed() {
    use cpal::{SampleFormat as F, SupportedBufferSize as B, SupportedStreamConfigRange as C};
    let range = |channels, rate, format| C::new(channels, rate, rate, B::Unknown, format);
    assert!(
        output_config(
            [
                range(0, 48_000, F::F32),
                range(2, 192_000, F::F32),
                range(2, 48_000, F::F64)
            ]
            .into_iter()
        )
        .is_none()
    );
    let config = output_config([range(1, 24_000, F::I16), range(2, 48_000, F::F32)].into_iter()).unwrap();
    assert_eq!(
        (config.sample_rate(), config.channels(), config.sample_format()),
        (48_000, 2, F::F32)
    );
    let output = playback(RATE);
    output.shared.queue.lock().unwrap().phase = Phase::Starting;
    let expired = output.shared.status(output.shared.created + START_TIMEOUT);
    assert_eq!(expired.failure, Some(Failure::Timeout));
    assert_eq!(output.cursor().phase, Phase::Failed);
}

/// Run in a child with an explicit isolated ALSA null configuration, never the desktop output.
#[cfg(target_os = "linux")]
#[test]
#[ignore = "requires VULTS_PLAYBACK_NULL_TEST and a private ALSA null configuration"]
fn native_null_output() {
    assert_eq!(std::env::var("VULTS_PLAYBACK_NULL_TEST").as_deref(), Ok("1"));
    let config = std::env::var_os("ALSA_CONFIG_PATH").expect("isolated ALSA configuration");
    assert!(std::fs::read_to_string(config).unwrap().contains("type null"));
    let output = Playback::start(Arc::default()).unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while output.cursor().phase == Phase::Starting && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(output.cursor().phase, Phase::Playing, "{:?}", output.cursor());
    output.push(&vec![8192; 2400]).unwrap();
    output.finish().unwrap();
    while output.cursor().phase == Phase::Draining && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(5));
    }
    let cursor = output.cursor();
    assert_eq!(cursor.phase, Phase::Finished, "{cursor:?}");
    assert_eq!(cursor.played_frames, 2400);
    assert!(cursor.first_played.is_some());
    drop(output);
    while BUSY.load(Ordering::Acquire) && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(
        !BUSY.load(Ordering::Acquire),
        "native teardown retained ownership"
    );
}
