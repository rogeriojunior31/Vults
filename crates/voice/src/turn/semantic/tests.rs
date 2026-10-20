use super::*;
use features::{FRAMES, Features, MELS, SAMPLES};

fn short() -> Vec<f32> {
    [
        0.0, 0.25, 0.5, 0.75, 1.0, 0.75, 0.5, 0.25, 0.0, -0.25, -0.5, -0.75, -1.0, -0.75, -0.5, -0.25,
    ]
    .repeat(200)
}
fn noise() -> Vec<f32> {
    let mut state = 17_u32;
    (0..128321)
        .map(|_| {
            state = state.wrapping_mul(1664525).wrapping_add(1013904223);
            (state >> 8) as f32 / 8388608.0 - 1.0
        })
        .collect()
}
fn speech() -> Vec<f32> {
    let wav = include_bytes!("../../../tests/fixtures/speech.wav");
    hound::WavReader::new(std::io::Cursor::new(wav))
        .unwrap()
        .samples::<i16>()
        .map(|s| f32::from(s.unwrap()) / 32768.0)
        .collect()
}
fn reference(bytes: &[u8]) -> Vec<f32> {
    bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|b| f32::from_le_bytes(*b))
        .collect()
}

#[test]
fn all_features_match_stored_primary_numpy_reference_arrays() {
    let extractor = Features::new();
    for (name, audio, bytes) in [
        (
            "short",
            short(),
            include_bytes!("../../../tests/fixtures/semantic-turn/short.f32le").as_slice(),
        ),
        (
            "noise",
            noise(),
            include_bytes!("../../../tests/fixtures/semantic-turn/noise.f32le").as_slice(),
        ),
        (
            "speech",
            speech(),
            include_bytes!("../../../tests/fixtures/semantic-turn/speech.f32le").as_slice(),
        ),
    ] {
        let actual = extractor
            .extract(&audio, &AtomicBool::new(false), None)
            .unwrap()
            .unwrap();
        let expected = reference(bytes);
        assert_eq!(expected.len(), MELS * FRAMES);
        assert_eq!(actual.len(), MELS * FRAMES);
        let max = actual
            .iter()
            .zip(&expected)
            .map(|(a, b)| (a - b).abs())
            .fold(0.0_f32, f32::max);
        eprintln!("{name}: maximum feature difference {max}");
        assert!(max < 1e-4, "{name}: maximum feature difference {max}");
    }
}
#[test]
fn left_padding_cropping_silence_and_invalid_audio_preserve_the_contract() {
    let extractor = Features::new();
    let stop = AtomicBool::new(false);
    let silent = extractor.extract(&[], &stop, None).unwrap().unwrap();
    assert!(
        extractor
            .extract(&[f32::NAN], &stop, Some(Instant::now()))
            .unwrap_err()
            .contains("budget expired")
    );
    assert!(silent.iter().all(|v| *v == -1.5));
    let audio = noise();
    assert_eq!(
        extractor.extract(&audio, &stop, None).unwrap(),
        extractor
            .extract(&audio[audio.len() - SAMPLES..], &stop, None)
            .unwrap()
    );
    let audio = short();
    let mut padded = vec![0.0; SAMPLES - audio.len()];
    padded.extend_from_slice(&audio);
    assert_eq!(
        extractor.extract(&audio, &stop, None).unwrap(),
        extractor.extract(&padded, &stop, None).unwrap()
    );
    for invalid in [f32::NAN, f32::INFINITY, f32::MAX] {
        assert!(extractor.extract(&[invalid], &stop, None).is_err());
    }
    assert!(
        extractor
            .extract(&[f32::NAN], &AtomicBool::new(true), None)
            .unwrap()
            .is_none()
    );
}
#[test]
fn model_admission_rejects_corruption_and_cancellation_before_native_loading() {
    let path = std::env::temp_dir().join(format!("vults-semantic-model-{}", std::process::id()));
    let file = std::fs::File::create_new(&path).unwrap();
    file.set_len(MODEL_SIZE as u64).unwrap();
    drop(file);
    assert!(crate::model_file::read_file(&path, MODEL_SIZE, MODEL_SHA256, &AtomicBool::new(false)).is_err());
    std::fs::remove_file(path).unwrap();
    assert!(
        crate::model_file::read_file(
            Path::new("/missing/model"),
            MODEL_SIZE,
            MODEL_SHA256,
            &AtomicBool::new(true)
        )
        .unwrap()
        .is_none()
    );
}
#[test]
fn native_control_cancellation_timeout_and_panics_do_not_publish_stale_work() {
    let stop = AtomicBool::new(false);
    let result = control::run(
        &stop,
        Instant::now() + Duration::from_millis(25),
        || Ok(AtomicBool::new(false)),
        |flag| {
            flag.store(true, Ordering::SeqCst);
            Ok(())
        },
        |flag| {
            while !flag.load(Ordering::SeqCst) {
                std::thread::park_timeout(Duration::from_millis(1));
            }
            Ok(42)
        },
    );
    assert!(result.is_err());
    let result = control::run(
        &stop,
        Instant::now() + Duration::from_secs(1),
        || Ok(AtomicBool::new(false)),
        |flag| {
            flag.store(true, Ordering::SeqCst);
            Ok(())
        },
        |flag| {
            stop.store(true, Ordering::SeqCst);
            while !flag.load(Ordering::SeqCst) {
                std::thread::park_timeout(Duration::from_millis(1));
            }
            Ok(42)
        },
    );
    assert!(result.unwrap().is_none());
    let stop = AtomicBool::new(false);
    let result = std::panic::catch_unwind(|| {
        control::run::<(), _>(
            &stop,
            Instant::now() + Duration::from_secs(1),
            || Ok(()),
            |_| panic!("finished watcher must not terminate"),
            |_| panic!("fake inference panic"),
        )
    });
    assert!(result.is_err());
}
#[test]
fn native_control_refuses_expired_work_and_preserves_real_errors() {
    let stop = AtomicBool::new(false);
    assert!(
        control::run::<(), ()>(
            &stop,
            Instant::now() + Duration::from_secs(1),
            || {
                stop.store(true, Ordering::SeqCst);
                Ok(())
            },
            |_| Ok(()),
            |_| panic!("cancelled preparation cannot run")
        )
        .unwrap()
        .is_none()
    );
    stop.store(false, Ordering::SeqCst);
    assert!(
        control::run::<(), ()>(
            &stop,
            Instant::now(),
            || panic!("expired work cannot initialize"),
            |_| Ok(()),
            |_| Ok(())
        )
        .is_err()
    );
    assert_eq!(
        control::run(
            &stop,
            Instant::now() + Duration::from_secs(1),
            || Ok(()),
            |_| Ok(()),
            |_| Ok(42)
        )
        .unwrap(),
        Some(42)
    );
    assert!(
        control::run::<(), ()>(
            &stop,
            Instant::now() + Duration::from_secs(1),
            || Ok(()),
            |_| Ok(()),
            |_| Err("real inference error".into())
        )
        .unwrap_err()
        .contains("real inference error")
    );
}
#[test]
#[ignore = "requires explicitly supplied pinned semantic model and trusted ONNX runtime"]
fn native_probabilities_match_the_reference_on_checked_artifacts() {
    let stop = AtomicBool::new(false);
    let runtime = Runtime::load(
        &std::path::PathBuf::from(
            std::env::var("VULTS_SEMANTIC_TEST_RUNTIME").expect("explicit trusted runtime"),
        ),
        &std::env::var("VULTS_SEMANTIC_TEST_RUNTIME_SHA256").expect("independent runtime hash"),
        &stop,
    )
    .unwrap()
    .unwrap();
    let model =
        std::path::PathBuf::from(std::env::var("VULTS_SEMANTIC_TEST_MODEL").expect("explicit pinned model"));
    let mut detector = Detector::load(&runtime, &model, &stop).unwrap().unwrap();
    let manifest: serde_json::Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/semantic-turn/reference.json"
    ))
    .unwrap();
    for (name, audio) in [
        ("short", short()),
        ("noise", noise()),
        ("speech", speech()),
        ("silence", vec![]),
    ] {
        let actual = detector
            .score(&audio, &stop, Duration::from_millis(1200))
            .unwrap()
            .unwrap();
        let expected = manifest[name]["probability"].as_f64().unwrap() as f32;
        if name == "short" {
            let _busy = crate::WHISPER.lock();
            let tensor = Tensor::from_array((
                [1, MELS, FRAMES],
                reference(include_bytes!(
                    "../../../tests/fixtures/semantic-turn/short.f32le"
                )),
            ))
            .unwrap();
            let outputs = detector
                .session
                .0
                .run(ort::inputs!["input_features" => tensor])
                .unwrap();
            eprintln!(
                "Exact reference tensor probability {:?}",
                outputs["logits"].try_extract_tensor::<f32>().unwrap().1
            );
        }
        eprintln!("{name}: probability {actual}, reference {expected}");
        assert!((actual - expected).abs() < 2e-4, "{name}: {actual} vs {expected}");
    }
    assert!(
        detector
            .score(&speech(), &AtomicBool::new(true), Duration::from_millis(1200))
            .unwrap()
            .is_none()
    );
    assert!(detector.score(&speech(), &stop, Duration::ZERO).is_err());
}
