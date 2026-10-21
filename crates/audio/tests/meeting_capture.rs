#![cfg(all(target_os = "linux", feature = "meeting-capture"))]

use std::path::PathBuf;
use std::time::{Duration, Instant};

use vults_audio::meeting::{self, End, Outcome, Phase, Recording};
use vults_audio::{Capture, CaptureOptions};

fn wait_for_recording(recording: &mut Recording) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while recording.status().phase != Phase::Recording {
        assert!(
            recording.take_result().is_none(),
            "fixture stopped during startup"
        );
        assert!(Instant::now() < deadline, "fixture startup timed out");
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn finish(recording: &mut Recording) -> Outcome {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(result) = recording.take_result() {
            return result.expect("fixture recording");
        }
        assert!(Instant::now() < deadline, "fixture teardown timed out");
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[test]
#[ignore = "requires an explicitly configured isolated server with synthetic sources only"]
fn stop_and_cancel_leave_an_independent_dictation_stream_running() {
    let server = std::env::var("VULTS_AUDIO_TEST_SERVER").expect("explicit fixture server");
    assert_eq!(std::env::var("PULSE_SERVER").expect("PulseAudio server"), server);
    let socket = PathBuf::from(server.strip_prefix("unix:").expect("local socket only"));
    let runtime = PathBuf::from(std::env::var("XDG_RUNTIME_DIR").expect("private runtime"));
    assert!(runtime.starts_with(std::env::temp_dir()) && runtime != std::env::temp_dir());
    assert!(socket.starts_with(&runtime));
    // Pin the normal ALSA microphone path to the same private synthetic server.
    let alsa = PathBuf::from(std::env::var("ALSA_CONFIG_PATH").expect("isolated ALSA configuration"));
    assert!(alsa.starts_with(&runtime));
    let config = std::fs::read_to_string(alsa).expect("ALSA fixture");
    assert!(config.contains(&server) && config.contains("vults_spike_mic"));
    let sources = meeting::sources().expect("fixture inventory");
    assert_eq!(sources.microphone, "vults_spike_mic");
    assert_eq!(sources.monitor, "vults_spike_output.monitor");
    let root = runtime.join("vults-meeting-controls-test");
    let dictation = Capture::start(
        CaptureOptions {
            max_seconds: 10,
            ..Default::default()
        },
        |_| {},
    )
    .expect("independent synthetic microphone");
    let audio = dictation.audio();
    let mut recording = Recording::start(&root, 60).expect("meeting start");
    wait_for_recording(&mut recording);
    std::thread::sleep(Duration::from_millis(200));
    assert!(
        Recording::start(&root, 60).is_err(),
        "duplicate start must retain ownership"
    );
    recording.stop();
    match finish(&mut recording) {
        Outcome::Finished(report) => {
            assert_eq!(report.end, End::Stopped);
            assert!(report.microphone.frames > 0 && report.system.frames > 0);
        }
        Outcome::Cancelled => panic!("Stop must finish an active recording"),
    }
    assert_eq!(recording.status().phase, Phase::Finished);
    let before = audio.len();
    let mut next = Recording::start(&root, 60).expect("restart after teardown");
    wait_for_recording(&mut next);
    std::thread::sleep(Duration::from_millis(200));
    next.cancel();
    assert!(matches!(finish(&mut next), Outcome::Cancelled));
    assert_eq!(next.status().phase, Phase::Cancelled);
    assert!(
        audio.len() > before,
        "meeting controls must not stop the dictation microphone"
    );
    let pcm = dictation.finish().expect("dictation continues independently");
    assert!(pcm.len() > 3_200 && pcm.iter().all(|sample| sample.is_finite()));
    assert!(!root.join("mic.wav").exists() && !root.join("system.wav").exists());
    meeting::recover(&root).expect("no active worker remains");
    println!(
        "Stopped and cancelled two meetings while dictation retained {} local samples.",
        pcm.len()
    );
}

#[test]
#[ignore = "requires an explicitly configured isolated server with synthetic sources; records 65 seconds"]
fn long_recording_passes_one_minute_with_bounded_pcm16_files_and_cleanup() {
    let server = std::env::var("VULTS_AUDIO_TEST_SERVER").expect("explicit fixture server");
    assert_eq!(std::env::var("PULSE_SERVER").unwrap(), server);
    let socket = PathBuf::from(server.strip_prefix("unix:").unwrap());
    let runtime = PathBuf::from(std::env::var("XDG_RUNTIME_DIR").unwrap());
    assert!(runtime.starts_with(std::env::temp_dir()) && runtime != std::env::temp_dir());
    assert!(socket.starts_with(&runtime));
    let sources = meeting::sources().unwrap();
    assert_eq!(sources.microphone, "vults_spike_mic");
    assert_eq!(sources.monitor, "vults_spike_output.monitor");
    let root = runtime.join("vults-long-meeting-test");
    let (send, receive) = std::sync::mpsc::channel();
    let mut recording = Recording::start_processed(&root, meeting::MAX_SECONDS, sources, move |tracks, _| {
        let mut durations = Vec::new();
        for input in [tracks.microphone, tracks.system] {
            let bytes = input.get_ref().metadata().unwrap().len();
            let wav = hound::WavReader::new(input).unwrap();
            assert_eq!(wav.spec().sample_rate, 16_000);
            assert_eq!(wav.spec().channels, 1);
            assert_eq!(wav.spec().bits_per_sample, 16);
            assert_eq!(wav.spec().sample_format, hound::SampleFormat::Int);
            assert!(wav.duration() > 60 * 16_000);
            assert!(bytes <= u64::from(wav.duration()) * 2 + 128);
            durations.push(wav.duration());
        }
        send.send(durations).unwrap();
        Ok(())
    })
    .unwrap();
    wait_for_recording(&mut recording);
    let started = Instant::now();
    while started.elapsed() < Duration::from_secs(65) {
        assert_eq!(recording.status().phase, Phase::Recording);
        assert!(recording.take_result().is_none());
        std::thread::sleep(Duration::from_millis(100));
    }
    recording.stop();
    assert!(matches!(finish(&mut recording), Outcome::Finished(_)));
    println!(
        "Synthetic capture past one minute: {:?} normalized frames",
        receive.recv().unwrap()
    );
    assert!(!root.join("mic.wav").exists() && !root.join("system.wav").exists());
    meeting::recover(&root).unwrap();
}
