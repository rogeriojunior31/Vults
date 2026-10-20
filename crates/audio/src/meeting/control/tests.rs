use super::*;
use crate::meeting::tests::Directory;
use crate::meeting::{End, Sources, Track};
use std::sync::mpsc;

fn report() -> Outcome {
    Outcome::Finished(Report {
        sources: Sources {
            microphone: "fixture-mic".into(),
            monitor: "fixture.monitor".into(),
            monitor_fallback: false,
        },
        microphone: Track {
            rate: 16_000,
            channels: 1,
            frames: 100,
            peak: 0.1,
        },
        system: Track {
            rate: 48_000,
            channels: 2,
            frames: 300,
            peak: 0.2,
        },
        startup: Duration::ZERO,
        end: End::Stopped,
    })
}

fn result(recording: &mut Recording) -> Result<Outcome, String> {
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        if let Some(result) = recording.take_result() {
            return result;
        }
        assert!(Instant::now() < deadline, "worker failed to release its fixture");
        std::thread::sleep(Duration::from_millis(1));
    }
}

#[test]
fn stop_during_startup_is_nonblocking_and_keeps_ownership_until_cleanup() {
    let dir = Directory::new();
    let (entered, ready) = mpsc::channel();
    let (release, blocked) = mpsc::channel();
    let mut recording = Recording::spawn(&dir.0, 60, move |temporary, _, control| {
        let _file = temporary.file("mic.wav")?;
        entered.send(()).expect("startup reached");
        blocked.recv().expect("startup released");
        assert!(!control.continuing());
        assert!(!control.started(), "a late startup must never reopen recording");
        Ok(Outcome::Cancelled)
    })
    .expect("start");
    ready.recv_timeout(Duration::from_secs(3)).expect("startup");
    assert_eq!(recording.status().phase, Phase::Starting);
    recording.stop();
    assert_eq!(recording.status().phase, Phase::Cancelling);
    assert_eq!(recording.status().elapsed, Duration::ZERO);
    assert!(recording.take_result().is_none());
    assert!(recover(&dir.0).is_err());
    assert!(Recording::spawn(&dir.0, 1, |_, _, _| panic!("busy worker must not run")).is_err());
    assert!(dir.0.join("mic.wav").exists());
    release.send(()).expect("release");
    assert!(matches!(result(&mut recording), Ok(Outcome::Cancelled)));
    assert_eq!(recording.status().phase, Phase::Cancelled);
    assert!(!dir.0.join("mic.wav").exists());
    assert!(recording.take_result().is_none(), "result is consumed only once");
    recover(&dir.0).expect("lock released");
}

#[test]
fn stop_and_cancel_races_cannot_publish_an_old_success() {
    let dir = Directory::new();
    let (entered, ready) = mpsc::channel();
    let (release, blocked) = mpsc::channel();
    let mut recording = Recording::spawn(&dir.0, 60, move |temporary, _, control| {
        let _file = temporary.file("system.wav")?;
        assert!(control.started());
        entered.send(()).expect("recording reached");
        blocked.recv().expect("stop released");
        Ok(report())
    })
    .expect("start");
    ready.recv_timeout(Duration::from_secs(3)).expect("recording");
    recording.stop();
    recording.stop();
    assert_eq!(recording.status().phase, Phase::Stopping);
    recording.cancel();
    recording.stop();
    assert_eq!(recording.status().phase, Phase::Cancelling);
    assert!(recording.take_result().is_none());
    release.send(()).expect("release");
    assert!(matches!(result(&mut recording), Ok(Outcome::Cancelled)));
    assert!(!dir.0.join("system.wav").exists());
    let elapsed = recording.status().elapsed;
    recording.stop();
    recording.cancel();
    assert_eq!(recording.status().phase, Phase::Cancelled);
    assert_eq!(recording.status().elapsed, elapsed);
}

#[test]
fn successful_stop_cleans_before_publishing_and_cannot_be_cancelled_later() {
    let dir = Directory::new();
    let mut recording = Recording::spawn(&dir.0, 60, |temporary, _, control| {
        let _file = temporary.file("mic.wav")?;
        assert!(control.started());
        control.request(false);
        Ok(report())
    })
    .expect("start");
    assert!(matches!(
        result(&mut recording),
        Ok(Outcome::Finished(Report {
            end: End::Stopped,
            ..
        }))
    ));
    assert_eq!(recording.status().phase, Phase::Finished);
    assert!(!dir.0.join("mic.wav").exists());
    recording.cancel();
    assert_eq!(recording.status().phase, Phase::Finished);
    let (entered, ready) = mpsc::channel();
    let (release, blocked) = mpsc::channel();
    let mut next = Recording::spawn(&dir.0, 1, move |_, _, control| {
        assert!(control.started());
        entered.send(()).expect("restarted");
        blocked.recv().expect("new capture released");
        assert_eq!(control.phase(), Phase::Recording);
        Ok(report())
    })
    .expect("restart");
    ready.recv_timeout(Duration::from_secs(3)).expect("new capture");
    recording.cancel();
    assert_eq!(next.status().phase, Phase::Recording);
    release.send(()).expect("release");
    assert!(matches!(result(&mut next), Ok(Outcome::Finished(_))));
}

#[test]
fn dropping_an_owner_cancels_without_releasing_live_files_early() {
    let dir = Directory::new();
    let (entered, ready) = mpsc::channel();
    let (release, blocked) = mpsc::channel();
    let recording = Recording::spawn(&dir.0, 60, move |temporary, _, control| {
        let _file = temporary.file("mic.wav")?;
        entered.send(()).expect("startup reached");
        blocked.recv().expect("startup released");
        assert_eq!(control.phase(), Phase::Cancelling);
        Ok(Outcome::Cancelled)
    })
    .expect("start");
    let worker_control = recording.control.clone();
    ready.recv_timeout(Duration::from_secs(3)).expect("startup");
    drop(recording);
    assert_eq!(worker_control.phase(), Phase::Cancelling);
    assert!(recover(&dir.0).is_err());
    release.send(()).expect("release");
    let deadline = Instant::now() + Duration::from_secs(3);
    while worker_control.phase() != Phase::Cancelled {
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(1));
    }
    assert!(!dir.0.join("mic.wav").exists());
}

#[test]
fn backend_errors_and_panics_cleanup_before_a_failed_result() {
    for panic in [false, true] {
        let dir = Directory::new();
        let mut recording = Recording::spawn(&dir.0, 60, move |temporary, _, _| {
            let _file = temporary.file("mic.wav")?;
            assert!(!panic, "fixture backend panic");
            Err("fixture backend failure".into())
        })
        .expect("start");
        assert!(result(&mut recording).is_err());
        assert_eq!(recording.status().phase, Phase::Failed);
        assert!(!dir.0.join("mic.wav").exists());
        recover(&dir.0).expect("worker released ownership");
    }
}

#[test]
fn cleanup_failure_never_reports_success_or_cancellation() {
    for cancel in [false, true] {
        let dir = Directory::new();
        let root = dir.0.clone();
        let mut recording = Recording::spawn(&dir.0, 60, move |temporary, _, control| {
            let _file = temporary.file("system.wav")?;
            std::fs::create_dir(root.join("mic.wav")).expect("cleanup failure");
            if cancel {
                control.request(true);
            }
            Ok(report())
        })
        .expect("start");
        assert!(
            result(&mut recording)
                .expect_err("cleanup failure")
                .contains("cleanup failed")
        );
        assert_eq!(recording.status().phase, Phase::Failed);
        assert!(!dir.0.join("system.wav").exists());
    }
}

#[test]
fn recovery_opens_no_devices_and_does_not_create_a_fresh_profile() {
    let dir = Directory::new();
    recover(&dir.0).expect("nothing to recover");
    assert!(!dir.0.exists());
    let temporary = TemporaryAudio::open(&dir.0).expect("old recording directory");
    drop(temporary);
    std::fs::write(dir.0.join("mic.wav"), b"interrupted recording").expect("leftover");
    recover(&dir.0).expect("recover without backend");
    assert!(!dir.0.join("mic.wav").exists());
}

#[test]
fn processing_keeps_both_files_locked_freezes_the_timer_and_honors_cancel() {
    use std::io::Read;
    let dir = Directory::new();
    let (entered, ready) = mpsc::channel();
    let (release, blocked) = mpsc::channel();
    let mut recording = Recording::spawn(&dir.0, 60, move |temporary, _, control| {
        temporary.file("mic.wav")?;
        temporary.file("system.wav")?;
        assert!(control.started());
        process_capture(temporary, control, report(), |mut tracks, stop| {
            entered.send(()).expect("processing");
            blocked.recv().expect("release");
            assert!(stop.load(Ordering::SeqCst));
            let mut bytes = Vec::new();
            tracks.microphone.read_to_end(&mut bytes).expect("still owned");
            Ok(())
        })
    })
    .expect("start");
    ready.recv_timeout(Duration::from_secs(3)).expect("processing");
    assert_eq!(recording.status().phase, Phase::Processing);
    let elapsed = recording.status().elapsed;
    recording.stop();
    assert_eq!(recording.status().phase, Phase::Processing);
    assert!(recover(&dir.0).is_err());
    assert!(dir.0.join("mic.wav").exists() && dir.0.join("system.wav").exists());
    recording.cancel();
    assert_eq!(recording.status().phase, Phase::Cancelling);
    assert_eq!(recording.status().elapsed, elapsed);
    assert!(recording.take_result().is_none());
    release.send(()).expect("release");
    assert!(matches!(result(&mut recording), Ok(Outcome::Cancelled)));
    assert!(!dir.0.join("mic.wav").exists() && !dir.0.join("system.wav").exists());
    recover(&dir.0).expect("cleanup lock released");
}

#[test]
fn processing_errors_and_panics_release_readers_before_cleanup() {
    for panic in [false, true] {
        let dir = Directory::new();
        let mut recording = Recording::spawn(&dir.0, 60, move |temporary, _, control| {
            temporary.file("mic.wav")?;
            temporary.file("system.wav")?;
            assert!(control.started());
            process_capture(temporary, control, report(), |_, _| {
                assert!(!panic, "synthetic processor panic");
                Err("synthetic processor failure".into())
            })
        })
        .expect("start");
        assert!(result(&mut recording).is_err());
        assert!(!dir.0.join("mic.wav").exists() && !dir.0.join("system.wav").exists());
        recover(&dir.0).expect("readers released");
    }
}

#[test]
fn processing_never_reads_a_replaced_symlink_hardlink_or_fifo() {
    use std::os::unix::fs::{PermissionsExt, symlink};
    for kind in ["symlink", "hardlink", "fifo", "mode"] {
        let dir = Directory::new();
        let root = dir.0.clone();
        let mut recording = Recording::spawn(&dir.0, 60, move |temporary, _, control| {
            temporary.file("mic.wav")?;
            temporary.file("system.wav")?;
            let mic = root.join("mic.wav");
            let system = root.join("system.wav");
            match kind {
                "symlink" => {
                    std::fs::remove_file(&mic).expect("replace");
                    symlink(&system, &mic).expect("link");
                }
                "hardlink" => {
                    std::fs::remove_file(&mic).expect("replace");
                    std::fs::hard_link(&system, &mic).expect("link");
                }
                "fifo" => {
                    std::fs::remove_file(&mic).expect("replace");
                    assert!(
                        std::process::Command::new("mkfifo")
                            .arg(&mic)
                            .status()
                            .expect("fixture FIFO")
                            .success()
                    );
                }
                _ => std::fs::set_permissions(&mic, std::fs::Permissions::from_mode(0o640)).expect("mode"),
            }
            assert!(control.started());
            process_capture(temporary, control, report(), |_, _| {
                panic!("unsafe audio must not reach processing")
            })
        })
        .expect("start");
        assert!(result(&mut recording).is_err());
        assert!(!dir.0.join("mic.wav").exists() && !dir.0.join("system.wav").exists());
    }
}

#[test]
fn a_four_hour_owner_keeps_recording_past_one_minute_and_still_stops_explicitly() {
    let dir = Directory::new();
    let (ready, started) = mpsc::channel();
    let mut recording = Recording::spawn(&dir.0, super::super::MAX_SECONDS, move |_, seconds, control| {
        assert_eq!(seconds, 4 * 60 * 60);
        assert!(control.started());
        control.state.lock().unwrap().started = Some(Instant::now() - Duration::from_secs(75));
        ready.send(()).unwrap();
        while control.continuing() {
            std::thread::yield_now();
        }
        Ok(report())
    })
    .unwrap();
    started.recv_timeout(Duration::from_secs(2)).unwrap();
    assert_eq!(recording.status().phase, Phase::Recording);
    assert!(recording.status().elapsed >= Duration::from_secs(75));
    recording.stop();
    assert!(matches!(result(&mut recording).unwrap(), Outcome::Finished(_)));
}
