use super::*;
use std::io::Cursor;

fn wav(rate: u32, channels: u16, frames: usize) -> Vec<u8> {
    let mut bytes = Cursor::new(Vec::new());
    let mut writer = hound::WavWriter::new(
        &mut bytes,
        hound::WavSpec {
            channels,
            sample_rate: rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        },
    )
    .expect("header");
    for _ in 0..frames * usize::from(channels) {
        writer.write_sample(100_i16).expect("sample");
    }
    writer.finalize().expect("wav");
    bytes.into_inner()
}

#[test]
fn normalization_preserves_duration_and_mono_budget_for_capture_formats() {
    for (rate, channels) in [(16_000, 1), (48_000, 2), (44_100, 1)] {
        let stop = AtomicBool::new(false);
        let mut progress = Vec::new();
        let pcm = read_audio(Cursor::new(wav(rate, channels, rate as usize)), &stop, &mut |p| {
            progress.push(p)
        })
        .expect("format")
        .expect("audio");
        assert_eq!(pcm.len(), RATE);
        assert!(pcm.iter().all(|s| s.is_finite()));
        assert_eq!(progress.first().expect("initial").completed_ms, 0);
        assert_eq!(progress.last().expect("last").completed_ms, 1000);
        assert!(
            progress
                .iter()
                .all(|p| p.phase == Phase::Reading && p.total_ms == 1000)
        );
    }
}

#[test]
fn normalized_source_must_fit_at_least_one_millisecond_before_progress_or_models() {
    for (rate, channels) in [(8_000, 1), (16_000, 1), (48_000, 2), (192_000, 1)] {
        let frames_per_ms = rate as usize / 1000;
        for frames in [1, frames_per_ms - 1] {
            let result = read_audio(
                Cursor::new(wav(rate, channels, frames)),
                &AtomicBool::new(false),
                &mut |_| panic!("submillisecond audio must fail before progress"),
            );
            assert!(result.is_err(), "rate={rate}, frames={frames}");
        }
        let pcm = read_audio(
            Cursor::new(wav(rate, channels, frames_per_ms)),
            &AtomicBool::new(false),
            &mut |_| {},
        )
        .expect("one millisecond is representable")
        .expect("active");
        assert_eq!(pcm.len(), RATE / 1000);
    }
}

#[test]
fn reading_cancellation_publishes_no_partial_pcm() {
    let stop = AtomicBool::new(false);
    let result = read_audio(Cursor::new(wav(16_000, 1, RATE)), &stop, &mut |p| {
        if p.completed_ms > 0 {
            stop.store(true, Ordering::SeqCst);
        }
    })
    .expect("cancel");
    assert!(result.is_none());
    assert!(
        read_audio(Cursor::new(Vec::new()), &AtomicBool::new(true), &mut |_| panic!(
            "progress"
        ))
        .expect("no IO")
        .is_none()
    );
}

#[test]
fn oversized_or_truncated_audio_is_rejected_before_model_loading() {
    assert!(
        read_audio(
            Cursor::new(wav(8000, 1, 8000 * 181)),
            &AtomicBool::new(false),
            &mut |_| {}
        )
        .is_err()
    );
    let mut truncated = wav(16_000, 1, RATE);
    truncated.truncate(truncated.len() - 2);
    assert!(read_audio(Cursor::new(truncated), &AtomicBool::new(false), &mut |_| {}).is_err());
}

#[test]
fn runtime_admission_rejects_bad_requests_before_native_loading() {
    assert!(
        Runtime::load(
            std::path::Path::new("relative"),
            &"0".repeat(64),
            &AtomicBool::new(false)
        )
        .is_err()
    );
    assert!(
        Runtime::load(
            std::path::Path::new("/missing/runtime"),
            "bad hash",
            &AtomicBool::new(false)
        )
        .is_err()
    );
    assert!(
        Runtime::load(
            std::path::Path::new("/missing/runtime"),
            &"0".repeat(64),
            &AtomicBool::new(true)
        )
        .expect("cancel before IO")
        .is_none()
    );
}

#[test]
fn cancelled_clustering_never_returns_partial_labels() {
    assert!(
        cluster::assign(
            &[vec![1.0, 0.0]],
            0.7,
            &std::collections::BTreeSet::new(),
            &AtomicBool::new(true)
        )
        .expect("cancel")
        .is_none()
    );
}

#[test]
fn a_second_analysis_is_rejected_and_cancelled_admission_returns_immediately() {
    let stop = AtomicBool::new(false);
    let first = admit(&stop).expect("first analysis").expect("admitted");
    assert!(admit(&stop).is_err());
    for profile in [SpeechProfile::Dictation, SpeechProfile::MeetingFrames] {
        assert!(
            speech_activity_wav(
                Cursor::new(Vec::new()),
                std::path::Path::new("/missing/vad"),
                &stop,
                profile,
            )
            .is_err()
        );
        assert!(
            speech_activity_wav(
                Cursor::new(Vec::new()),
                std::path::Path::new("/missing/vad"),
                &AtomicBool::new(true),
                profile,
            )
            .expect("cancel before IO")
            .is_none()
        );
    }
    assert!(admit(&AtomicBool::new(true)).expect("cancel").is_none());
    drop(first);
    assert!(admit(&stop).expect("released slot").is_some());
}

#[test]
fn runtime_requires_startup_privacy_before_touching_a_library() {
    if std::env::var("ORT_DISABLE_TELEMETRY").as_deref() == Ok("1") {
        return;
    }
    let error = Runtime::load(
        std::path::Path::new("/missing/native-library"),
        &"0".repeat(64),
        &AtomicBool::new(false),
    )
    .expect_err("privacy must fail before IO");
    assert!(error.contains("telemetry must be disabled at process startup"));
}

#[cfg(feature = "meeting-assets")]
#[test]
fn composed_handoff_preserves_text_on_missing_or_failed_analysis_and_discards_cancelled_results() {
    use crate::meeting::{Segment, Transcript, speakers::Turn};
    let transcript = || Transcript {
        duration_ms: 1000,
        segments: vec![Segment {
            start_ms: 100,
            end_ms: 900,
            text: "Keep every word".into(),
        }],
    };
    let analysis = || Analysis {
        timeline: Timeline {
            duration_ms: 1000,
            turns: vec![Turn {
                start_ms: 0,
                end_ms: 1000,
                speaker: Some(7),
            }],
        },
        speakers: 1,
        observations: 1,
    };
    let mut mismatched = analysis();
    mismatched.timeline.duration_ms = 1500;
    let stop = AtomicBool::new(false);
    for (result, status, speaker) in [
        (Ok(Some(mismatched)), Attribution::Unavailable, None),
        (Ok(None), Attribution::Unavailable, None),
        (Err("native failure".into()), Attribution::Unavailable, None),
        (Ok(Some(analysis())), Attribution::Analyzed, Some(7)),
    ] {
        let result = finish_transcript(transcript(), result, &stop)
            .expect("handoff")
            .expect("active");
        assert_eq!(result.attribution, status);
        assert_eq!(result.transcript.duration_ms, 1000);
        assert_eq!(result.transcript.segments[0].segment, transcript().segments[0]);
        assert_eq!(result.transcript.segments[0].speaker, speaker);
    }
    assert!(
        finish_transcript(transcript(), Ok(Some(analysis())), &AtomicBool::new(true))
            .expect("cancel")
            .is_none()
    );
}
