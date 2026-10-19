use super::*;
use std::io::Cursor;

const S: usize = SAMPLE_RATE as usize;

#[test]
fn outer_trim_keeps_continuous_speech_and_offsets_but_never_concatenates_pauses() {
    let continuous = vec![0.1; 28 * S];
    assert_eq!(speech_range(&continuous), 0..continuous.len());
    let mut pcm = vec![0.0; 28 * S];
    pcm[3 * S..4 * S].copy_from_slice(&continuous[..S]);
    pcm[20 * S..21 * S].copy_from_slice(&continuous[..S]);
    assert_eq!(speech_range(&pcm), 3 * S - S / 4..21 * S + S / 4);
    assert!(speech_range(&vec![0.0; 28 * S]).is_empty());
    assert!(speech_range(&vec![0.001; 28 * S]).is_empty());
}

#[test]
fn whisper_abort_callback_yields_to_foreground_and_distinguishes_cancellation() {
    use std::time::{Duration, Instant};
    let scheduler = crate::scheduler::Scheduler::new();
    let stop = AtomicBool::new(false);
    let request = Abort {
        stop: &stop,
        scheduler: &scheduler,
        yielded: AtomicBool::new(false),
    };
    let data = std::ptr::from_ref(&request).cast_mut().cast();
    // SAFETY: the request stays borrowed until every callback returns, just as in `full`.
    assert!(!unsafe { abort(data) });
    let background = scheduler.background(&stop).expect("background chunk");
    std::thread::scope(|scope| {
        let interactive = scope.spawn(|| {
            drop(scheduler.lock());
        });
        let deadline = Instant::now() + Duration::from_secs(2);
        while !scheduler.foreground_waiting() {
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(1));
        }
        // SAFETY: same live request as above.
        assert!(unsafe { abort(data) });
        assert!(request.yielded.load(Ordering::SeqCst));
        assert!(
            !stop.load(Ordering::SeqCst),
            "preemption must not cancel the meeting"
        );
        drop(background);
        interactive.join().expect("foreground work");
    });
    assert!(scheduler.background(&stop).is_some(), "the same chunk may retry");
    request.yielded.store(false, Ordering::SeqCst);
    stop.store(true, Ordering::SeqCst);
    // SAFETY: the request is still live.
    assert!(unsafe { abort(data) });
    assert!(
        !request.yielded.load(Ordering::SeqCst),
        "user cancellation is terminal"
    );
}

fn wav(samples: &[f32]) -> Cursor<Vec<u8>> {
    let mut buffer = Cursor::new(Vec::new());
    let mut writer = hound::WavWriter::new(
        &mut buffer,
        hound::WavSpec {
            channels: 1,
            sample_rate: SAMPLE_RATE,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        },
    )
    .expect("WAV");
    for &sample in samples {
        writer.write_sample(sample).expect("sample");
    }
    writer.finalize().expect("WAV");
    buffer.set_position(0);
    buffer
}

#[test]
fn transcription_rejects_a_zero_millisecond_source_before_progress_or_decode() {
    for length in [1, S / 1000 - 1] {
        let mut progress = 0;
        let mut decodes = 0;
        let result = run(
            wav(&vec![0.0; length]),
            &AtomicBool::new(false),
            |_| progress += 1,
            |_| {
                decodes += 1;
                Ok(Some(Vec::new()))
            },
        );
        assert!(result.is_err(), "length={length}");
        assert_eq!((progress, decodes), (0, 0));
    }
    let result = run(
        wav(&[0.0; S / 1000]),
        &AtomicBool::new(false),
        |_| {},
        |_| Ok(Some(Vec::new())),
    )
    .expect("one millisecond is representable")
    .expect("active");
    assert_eq!(result.duration_ms, 1);
    assert!(result.segments.is_empty());
}

#[test]
fn timestamps_keep_pauses_and_chunk_offsets_and_progress_reaches_eof() {
    let stop = AtomicBool::new(false);
    let mut progress = Vec::new();
    let mut decoded = Vec::new();
    let transcript = run(
        wav(&vec![0.0; S * 60 + 137]),
        &stop,
        |p| progress.push(p),
        |pcm| {
            decoded.push(pcm.len());
            Ok(Some(vec![Segment {
                start_ms: 1200,
                end_ms: 2400,
                text: "Keep these words.".into(),
            }]))
        },
    )
    .expect("transcript")
    .expect("not cancelled");
    assert_eq!(decoded.len(), 3);
    assert_eq!(decoded.iter().sum::<usize>(), S * 60 + 137);
    assert_eq!(transcript.duration_ms, 60_008);
    for (index, segment) in transcript.segments.iter().enumerate() {
        let offset = millis(decoded[..index].iter().sum::<usize>() as u64);
        assert_eq!(segment.start_ms, offset + 1200);
        assert_eq!(segment.end_ms, offset + 2400);
    }
    assert!(progress.windows(2).all(|p| p[0].completed_ms < p[1].completed_ms));
    assert_eq!(
        progress.last(),
        Some(&Progress {
            completed_ms: 60_008,
            total_ms: 60_008,
            segments: 3
        })
    );
}

#[test]
fn invalid_and_overlapping_model_timestamps_cannot_escape_the_chunk() {
    let segment = |start_ms, end_ms| Segment {
        start_ms,
        end_ms,
        text: "Speech".into(),
    };
    let previous = segment(100, 1200);
    assert_eq!(
        place(segment(0, 4000), 1000, 2000, Some(&previous)),
        Some(segment(1200, 2000))
    );
    assert!(place(segment(2000, 1000), 1000, 2000, None).is_none());
    assert!(place(segment(u64::MAX, u64::MAX), 1000, 2000, None).is_none());
    assert!(
        place(
            Segment {
                text: String::new(),
                ..segment(0, 500)
            },
            1000,
            2000,
            None
        )
        .is_none()
    );
}

#[test]
fn cancellation_during_decode_or_progress_never_returns_partial_text() {
    let stop = AtomicBool::new(false);
    let result = run(
        wav(&vec![0.0; S * 60]),
        &stop,
        |_| {},
        |_| {
            stop.store(true, Ordering::SeqCst);
            Ok(Some(vec![Segment {
                start_ms: 0,
                end_ms: 500,
                text: "Discard this".into(),
            }]))
        },
    )
    .expect("cancelled decode");
    assert!(result.is_none());
    stop.store(false, Ordering::SeqCst);
    let mut calls = 0;
    let result = run(
        wav(&vec![0.0; S * 60]),
        &stop,
        |progress| {
            if progress.completed_ms > 0 {
                stop.store(true, Ordering::SeqCst);
            }
        },
        |_| {
            calls += 1;
            Ok(Some(Vec::new()))
        },
    )
    .expect("cancelled progress");
    assert!(result.is_none());
    assert_eq!(calls, 1);
    assert!(
        run(
            Cursor::new(b"not even read"),
            &stop,
            |_| panic!("no progress"),
            |_| panic!("no decode")
        )
        .expect("already cancelled")
        .is_none()
    );
}

#[test]
fn decode_and_read_failure_discard_the_accumulated_transcript() {
    let stop = AtomicBool::new(false);
    let mut calls = 0;
    let result = run(
        wav(&vec![0.0; S * 60]),
        &stop,
        |_| {},
        |_| {
            calls += 1;
            if calls == 2 {
                return Err("synthetic decode failure".into());
            }
            Ok(Some(vec![Segment {
                start_ms: 0,
                end_ms: 500,
                text: "Partial text".into(),
            }]))
        },
    );
    assert_eq!(result.expect_err("failure"), "synthetic decode failure");
    let mut input = wav(&vec![0.0; S * 60]).into_inner();
    input.truncate(input.len() - S * 4);
    assert!(run(Cursor::new(input), &stop, |_| {}, |_| Ok(Some(Vec::new()))).is_err());
}

#[test]
fn a_model_cannot_exceed_the_output_budget() {
    let stop = AtomicBool::new(false);
    assert!(
        run(
            wav(&vec![0.0; S]),
            &stop,
            |_| {},
            |_| Ok(Some(vec![Segment {
                start_ms: 0,
                end_ms: 1000,
                text: "x".repeat(16 * 1024 * 1024 + 1),
            }]))
        )
        .is_err()
    );
}

/// Opt-in local model check; the tracked speech fixture is public-domain speech, not user audio.
#[test]
fn local_model_keeps_two_speech_offsets_and_silence_empty() {
    let Some(model) = std::env::var_os("VOICE_TEST_MODEL") else {
        return;
    };
    let transcriber = Transcriber::load(std::path::Path::new(&model), false).expect("installed model");
    let stop = AtomicBool::new(false);
    let mut reader = hound::WavReader::new(Cursor::new(include_bytes!("../../tests/fixtures/speech.wav")))
        .expect("public speech fixture");
    let speech: Vec<f32> = reader
        .samples::<i16>()
        .map(|s| f32::from(s.expect("sample")) / 32768.0)
        .collect();
    let mut samples = vec![0.0; 40 * S];
    samples[3 * S..3 * S + speech.len()].copy_from_slice(&speech);
    samples[33 * S..33 * S + speech.len()].copy_from_slice(&speech);
    let transcript = transcribe_wav(&transcriber, wav(&samples), Some("en"), &stop, |_| {})
        .expect("local model")
        .expect("not cancelled");
    assert_eq!(transcript.duration_ms, 40_000);
    assert!(
        transcript
            .segments
            .iter()
            .any(|s| s.start_ms >= 2500 && s.end_ms < 10_000),
        "{transcript:?}"
    );
    assert!(
        transcript
            .segments
            .iter()
            .any(|s| s.start_ms >= 32_500 && s.end_ms <= 40_000),
        "{transcript:?}"
    );
    assert!(
        transcript
            .segments
            .windows(2)
            .all(|s| s[0].end_ms <= s[1].start_ms)
    );
    let text = transcript
        .segments
        .iter()
        .map(|s| s.text.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    assert!(text.to_lowercase().contains("americans"), "{text}");
    let silence = transcribe_wav(&transcriber, wav(&vec![0.0; 30 * S]), Some("en"), &stop, |_| {})
        .expect("silence")
        .expect("not cancelled");
    assert!(silence.segments.is_empty());
    let continuous: Vec<f32> = speech.iter().copied().cycle().take(28 * S).collect();
    let cancelled = std::thread::scope(|scope| {
        scope.spawn(|| {
            std::thread::sleep(std::time::Duration::from_millis(30));
            stop.store(true, Ordering::SeqCst);
        });
        decode(&transcriber, &continuous, Some("en"), &stop, false).expect("cancellable model")
    });
    assert!(
        cancelled.is_none(),
        "token cancellation discards the active decode"
    );
    stop.store(false, Ordering::SeqCst);
    let recovered = transcribe_wav(&transcriber, wav(&speech), Some("en"), &stop, |_| {})
        .expect("model reusable after abort")
        .expect("not cancelled");
    assert!(!recovered.segments.is_empty());
    stop.store(true, Ordering::SeqCst);
    assert!(
        transcribe_wav(&transcriber, wav(&samples), Some("en"), &stop, |_| panic!(
            "cancelled progress"
        ))
        .expect("cancelled")
        .is_none()
    );
}

#[test]
fn model_word_times_split_only_when_the_complete_filtered_text_and_clocks_survive() {
    let segment = Segment {
        start_ms: 100,
        end_ms: 900,
        text: "Hello world!".into(),
    };
    let tokens = vec![
        (b" Hello".to_vec(), 0, 20),
        (b" wor".to_vec(), 20, 40),
        (b"ld".to_vec(), 40, 60),
        (b"!".to_vec(), 60, 70),
    ];
    let words = timed_words(segment.clone(), 100, tokens.clone().into_iter());
    assert_eq!(
        words,
        vec![
            Segment {
                start_ms: 100,
                end_ms: 300,
                text: "Hello".into()
            },
            Segment {
                start_ms: 300,
                end_ms: 800,
                text: "world!".into()
            }
        ]
    );
    for bad in [
        vec![],
        vec![(b" Hello".to_vec(), -1, 20)],
        vec![(b" Hello".to_vec(), 0, 0)],
        vec![(b" Hello".to_vec(), 0, 80), (b" world!".to_vec(), 20, 60)],
        vec![(b" Different text".to_vec(), 0, 70)],
    ] {
        assert_eq!(
            timed_words(segment.clone(), 100, bad.into_iter()),
            vec![segment.clone()]
        );
    }
    let utf = Segment {
        start_ms: 0,
        end_ms: 1000,
        text: "Hi \u{4e16}\u{754c}".into(),
    };
    let parts = vec![
        (b" Hi".to_vec(), 0, 20),
        (vec![32, 0xe4, 0xb8], 20, 40),
        (vec![0x96, 0xe7, 0x95, 0x8c], 40, 60),
    ];
    let words = timed_words(utf.clone(), 0, parts.into_iter());
    assert_eq!(
        words
            .iter()
            .map(|w| w.text.as_str())
            .collect::<Vec<_>>()
            .join(" "),
        utf.text
    );
    assert_eq!(words.len(), 2);
}

#[test]
fn punctuation_is_retained_with_model_words_instead_of_becoming_an_identified_voice_row() {
    let segment = Segment {
        start_ms: 0,
        end_ms: 1000,
        text: "- Hello - world.".into(),
    };
    let tokens = vec![
        (b" -".to_vec(), 0, 10),
        (b" Hello".to_vec(), 10, 40),
        (b" -".to_vec(), 40, 50),
        (b" world".to_vec(), 50, 80),
        (b".".to_vec(), 80, 90),
    ];
    let words = timed_words(segment.clone(), 0, tokens.into_iter());
    assert_eq!(
        words
            .iter()
            .map(|w| w.text.as_str())
            .collect::<Vec<_>>()
            .join(" "),
        segment.text
    );
    assert_eq!(words.len(), 2);
    assert!(words.iter().all(|w| w.text.chars().any(char::is_alphanumeric)));
}
