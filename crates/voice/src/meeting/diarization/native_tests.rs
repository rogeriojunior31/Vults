//! Explicit opt-in checks; CI never reads native runtime/model paths or downloads them.

use super::*;
use ort::{
    session::{RunOptions, Session},
    value::Tensor,
};
use std::path::PathBuf;
use std::sync::{atomic::AtomicUsize, mpsc};
use std::time::{Duration, Instant};

fn runtime() -> Runtime {
    Runtime::load(
        &PathBuf::from(std::env::var("VULTS_DIARIZATION_TEST_RUNTIME").expect("explicit trusted runtime")),
        &std::env::var("VULTS_DIARIZATION_TEST_RUNTIME_SHA256").expect("independent runtime fingerprint"),
        &AtomicBool::new(false),
    )
    .expect("runtime")
    .expect("not cancelled")
}
fn artifacts() -> VerifiedModels {
    super::super::speaker_models::read_models(
        &PathBuf::from(std::env::var("VULTS_DIARIZATION_TEST_SEGMENTATION").expect("explicit segmentation")),
        &PathBuf::from(std::env::var("VULTS_DIARIZATION_TEST_EMBEDDING").expect("explicit embedding")),
        &AtomicBool::new(false),
    )
    .expect("pinned models")
    .expect("not cancelled")
}
fn segmentation(runtime: &Runtime) -> Session {
    let artifacts = artifacts();
    Session::builder(&runtime.environment().expect("admitted environment"))
        .expect("builder")
        .with_intra_threads(1)
        .expect("CPU thread")
        .with_inter_threads(1)
        .expect("inter-op thread")
        .commit_from_memory(artifacts.segmentation())
        .expect("segmentation")
}

#[test]
#[ignore = "requires an explicitly supplied trusted Silero model"]
fn native_speech_diagnostic_preserves_duration_and_anonymous_ordered_regions() {
    let model = PathBuf::from(std::env::var("VOICE_TEST_VAD").expect("explicit trusted VAD model"));
    let speech = include_bytes!("../../../tests/fixtures/speech.wav");
    for profile in [SpeechProfile::Dictation, SpeechProfile::MeetingFrames] {
        let timeline = speech_activity_wav(
            std::io::Cursor::new(speech),
            &model,
            &AtomicBool::new(false),
            profile,
        )
        .expect("native diagnostic")
        .expect("active");
        assert_eq!(timeline.duration_ms, 3400);
        assert!(!timeline.turns.is_empty());
        assert!(
            timeline
                .turns
                .iter()
                .all(|t| t.speaker.is_none() && t.start_ms < t.end_ms && t.end_ms <= timeline.duration_ms)
        );
        assert!(
            timeline
                .turns
                .windows(2)
                .all(|pair| pair[0].end_ms < pair[1].start_ms)
        );
        assert!(
            speech_activity_wav(
                std::io::Cursor::new(speech),
                &model,
                &AtomicBool::new(true),
                profile,
            )
            .expect("cancel before loading")
            .is_none()
        );
    }
}

#[test]
#[ignore = "requires explicitly supplied trusted native runtime and pinned public models"]
fn native_pipeline_completes_or_discards_late_cancellation() {
    let runtime = runtime();
    let mut progress = Vec::new();
    let speech = include_bytes!("../../../tests/fixtures/speech.wav");
    let result = analyze_wav(
        &runtime,
        artifacts(),
        std::io::Cursor::new(speech),
        &AtomicBool::new(false),
        |p| progress.push(p),
    )
    .expect("pipeline")
    .expect("result");
    assert_eq!(result.timeline.duration_ms, 3400);
    assert_eq!(result.speakers, 1);
    assert!(progress.iter().any(|p| p.phase == Phase::Reading));
    assert!(progress.iter().any(|p| p.phase == Phase::Loading));
    assert_eq!(progress.last().expect("final progress").phase, Phase::Analyzing);
    let stop = AtomicBool::new(false);
    assert!(
        analyze_wav(&runtime, artifacts(), std::io::Cursor::new(speech), &stop, |p| {
            if p.phase == Phase::Analyzing && p.completed_ms == p.total_ms {
                stop.store(true, Ordering::SeqCst);
            }
        })
        .expect("cancelled after final window")
        .is_none()
    );
    assert!(
        Runtime::load(
            &PathBuf::from("/missing"),
            &"0".repeat(64),
            &AtomicBool::new(true)
        )
        .expect("cancel")
        .is_none()
    );
}

#[test]
#[ignore = "requires explicitly supplied trusted native runtime and pinned public models"]
fn native_termination_discards_an_active_operation() {
    let runtime = runtime();
    let mut session = segmentation(&runtime);
    let stop = AtomicBool::new(false);
    let terminated = AtomicBool::new(false);
    let saw_native_abort = AtomicBool::new(false);
    let (enter, entered) = mpsc::channel();
    std::thread::scope(|scope| {
        let stop_ref = &stop;
        let cancel = scope.spawn(move || {
            entered
                .recv_timeout(Duration::from_secs(5))
                .expect("native operation entered");
            stop_ref.store(true, Ordering::SeqCst);
        });
        let result = control::run(
            &crate::WHISPER,
            &stop,
            || RunOptions::new().map_err(|e| e.to_string()),
            |options| {
                options.terminate().map_err(|e| e.to_string())?;
                terminated.store(true, Ordering::SeqCst);
                Ok(())
            },
            |options| {
                let input = Tensor::from_array(([1, 1, RATE * 10], vec![0.01_f32; RATE * 10]))
                    .map_err(|e| e.to_string())?;
                enter.send(()).expect("entered");
                let deadline = Instant::now() + Duration::from_secs(5);
                loop {
                    match session.run_with_options(ort::inputs![&input], options) {
                        Err(error) => {
                            saw_native_abort.store(
                                error.to_string().to_ascii_lowercase().contains("terminate"),
                                Ordering::SeqCst,
                            );
                            return Err::<(), _>(error.to_string());
                        }
                        Ok(_) if Instant::now() < deadline => {}
                        Ok(_) => return Err("native termination did not arrive".into()),
                    }
                }
            },
        )
        .expect("cancellation");
        assert!(result.is_none());
        cancel.join().expect("cancellation worker");
    });
    assert!(terminated.load(Ordering::SeqCst));
    assert!(saw_native_abort.load(Ordering::SeqCst));
}

#[test]
#[ignore = "requires explicitly supplied trusted native runtime and pinned public models"]
fn native_preemption_runs_interactive_first_and_retries_cleanly() {
    let runtime = runtime();
    let mut session = segmentation(&runtime);
    let stop = AtomicBool::new(false);
    let attempts = AtomicUsize::new(0);
    let (enter, entered) = mpsc::channel();
    let (order, ordered) = mpsc::channel();
    let (release, held) = mpsc::channel();
    std::thread::scope(|scope| {
        let interactive = scope.spawn(move || {
            entered
                .recv_timeout(Duration::from_secs(5))
                .expect("native operation entered");
            let _busy = crate::WHISPER.lock();
            order.send(()).expect("foreground order");
            held.recv_timeout(Duration::from_secs(5)).expect("release");
        });
        let attempts_ref = &attempts;
        let coordinator = scope.spawn(move || {
            ordered
                .recv_timeout(Duration::from_secs(5))
                .expect("foreground entered");
            assert_eq!(attempts_ref.load(Ordering::SeqCst), 1);
            release.send(()).expect("release");
        });
        let result = control::native(&stop, |options| {
            let input = Tensor::from_array(([1, 1, RATE * 10], vec![0.01_f32; RATE * 10]))
                .map_err(|e| e.to_string())?;
            if attempts.fetch_add(1, Ordering::SeqCst) == 0 {
                enter.send(()).expect("enter");
                let deadline = Instant::now() + Duration::from_secs(5);
                loop {
                    match session.run_with_options(ort::inputs![&input], options) {
                        Err(error) => return Err(error.to_string()),
                        Ok(_) if Instant::now() < deadline => {}
                        Ok(_) => return Err("native preemption did not arrive".into()),
                    }
                }
            }
            let outputs = session
                .run_with_options(ort::inputs![input], options)
                .map_err(|e| e.to_string())?;
            let (shape, _) = outputs[0]
                .try_extract_tensor::<f32>()
                .map_err(|e| e.to_string())?;
            Ok(shape.to_vec())
        })
        .expect("native retry")
        .expect("complete operation");
        assert_eq!(result, [1, 589, 7]);
        assert_eq!(attempts.load(Ordering::SeqCst), 2);
        interactive.join().expect("foreground");
        coordinator.join().expect("coordinator");
    });
}

#[test]
#[ignore = "requires explicitly supplied trusted native runtime/models and a consented long WAV"]
fn native_long_source_cancellation_discards_global_work_after_the_old_limit() {
    let runtime = runtime();
    let path =
        PathBuf::from(std::env::var("VULTS_DIARIZATION_TEST_LONG_WAV").expect("explicit consented long WAV"));
    let input = std::fs::File::open(path).expect("read source");
    let stop = AtomicBool::new(false);
    let mut completed = 0;
    let started = Instant::now();
    let result = analyze_wav(&runtime, artifacts(), input, &stop, |p| {
        assert_eq!(p.total_ms, 1_800_000);
        if p.phase == Phase::Analyzing {
            completed = p.completed_ms;
            if completed > 180_000 {
                stop.store(true, Ordering::SeqCst);
            }
        }
    })
    .expect("cooperative cancellation");
    assert!(completed > 180_000 && completed < 1_800_000);
    assert!(result.is_none());
    assert!(started.elapsed() < Duration::from_secs(120));
}
