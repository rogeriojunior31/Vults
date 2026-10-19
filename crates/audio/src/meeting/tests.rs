use super::*;
use std::fs;
use std::os::unix::fs::{MetadataExt, PermissionsExt, symlink};
use std::path::PathBuf;
use std::sync::atomic::AtomicU64;

pub(super) struct Directory(pub(super) PathBuf);

impl Directory {
    pub(super) fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let unique = NEXT.fetch_add(1, Ordering::SeqCst);
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        Self(std::env::temp_dir().join(format!(
            "vults-meeting-test-{}-{stamp}-{unique}",
            std::process::id()
        )))
    }
}

impl Drop for Directory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn monitor_selection_uses_server_ids_and_never_falls_back_to_a_microphone() {
    let names = ["a.monitor", "mic", "speakers.monitor"].map(str::to_owned);
    for (sink, expected) in [
        (Some("speakers"), Some(2)),
        (Some("gone"), Some(0)),
        (None, Some(0)),
    ] {
        assert_eq!(monitor_index(&names, sink), expected);
    }
    assert_eq!(monitor_index(&["mic".into()], Some("speakers")), None);
    assert_eq!(monitor_index(&[], None), None);
    assert_eq!(microphone_index(&names, Some("mic")), Some(1));
    assert_eq!(microphone_index(&names, Some("speakers.monitor")), Some(1));
    assert_eq!(microphone_index(&["sink.monitor".into()], None), None);
    assert_eq!(microphone_index(&[String::new()], None), None);
}

#[test]
fn monitor_metadata_selects_custom_names_and_excludes_them_from_microphones() {
    let server = protocol::ServerInfo {
        default_sink_name: Some(c"speakers".to_owned()),
        default_source_name: Some(c"custom_loopback".to_owned()),
        ..Default::default()
    };
    let monitor = SourceInfo {
        name: c"custom_loopback".to_owned(),
        monitor_of_sink_index: Some(1),
        monitor_of_sink_name: Some(c"speakers".to_owned()),
        ..Default::default()
    };
    let mic = SourceInfo {
        name: c"usb_microphone".to_owned(),
        ..Default::default()
    };
    let (sources, _, _) =
        select_devices(server.clone(), vec![monitor.clone(), mic.clone()]).expect("source metadata");
    assert_eq!(sources.microphone, "usb_microphone");
    assert_eq!(sources.monitor, "custom_loopback");
    assert!(!sources.monitor_fallback);
    assert!(select_devices(server.clone(), vec![monitor]).is_err());
    assert!(select_devices(server, vec![mic]).is_err());
    assert!(select_devices(protocol::ServerInfo::default(), vec![SourceInfo::default()]).is_err());
}

#[test]
fn missing_default_sink_uses_a_deterministic_reported_monitor_fallback() {
    let custom = |name: &str| SourceInfo {
        name: std::ffi::CString::new(name).expect("fixture ID"),
        monitor_of_sink_index: Some(1),
        ..Default::default()
    };
    let mic = SourceInfo {
        name: c"mic".to_owned(),
        ..Default::default()
    };
    let (sources, _, _) = select_devices(
        protocol::ServerInfo::default(),
        vec![custom("z"), mic, custom("a")],
    )
    .expect("fallback");
    assert_eq!(sources.monitor, "a");
    assert!(sources.monitor_fallback);
}

#[test]
fn temporary_files_are_private_removed_on_drop_and_recovered_after_a_crash() {
    let dir = Directory::new();
    let temporary = TemporaryAudio::open(&dir.0).expect("temporary directory");
    let mic = temporary.file("mic.wav").expect("microphone file");
    assert_eq!(fs::metadata(&dir.0).expect("directory").mode() & 0o777, 0o700);
    assert_eq!(mic.metadata().expect("file").mode() & 0o777, 0o600);
    assert!(temporary.file("../outside.wav").is_err());
    assert!(temporary.file("mic.wav").is_err());
    assert!(
        TemporaryAudio::open(&dir.0).is_err(),
        "a second recorder must not remove live audio"
    );
    assert!(dir.0.join("mic.wav").exists());
    drop(mic);
    drop(temporary);
    assert!(!dir.0.join("mic.wav").exists());
    fs::write(dir.0.join("mic.wav"), b"interrupted mic").expect("crash fixture");
    fs::write(dir.0.join("system.wav"), b"interrupted output").expect("crash fixture");
    fs::write(dir.0.join("keep.txt"), b"unrelated").expect("unrelated file");
    let recovered = TemporaryAudio::open(&dir.0).expect("startup recovery");
    assert!(!dir.0.join("mic.wav").exists());
    assert!(!dir.0.join("system.wav").exists());
    assert!(dir.0.join("keep.txt").exists());
    recovered.finish().expect("cleanup");
}

#[test]
fn cleanup_attempts_both_files_when_one_cannot_be_removed() {
    let dir = Directory::new();
    let temporary = TemporaryAudio::open(&dir.0).expect("directory");
    fs::create_dir(dir.0.join("mic.wav")).expect("blocked cleanup fixture");
    drop(temporary.file("system.wav").expect("system audio"));
    assert!(temporary.finish().is_err());
    assert!(!dir.0.join("system.wav").exists());
}

#[test]
fn unsafe_directories_and_lock_symlinks_are_rejected() {
    let dir = Directory::new();
    fs::create_dir(&dir.0).expect("directory");
    fs::set_permissions(&dir.0, fs::Permissions::from_mode(0o755)).expect("public mode");
    assert!(TemporaryAudio::open(&dir.0).is_err());
    fs::set_permissions(&dir.0, fs::Permissions::from_mode(0o700)).expect("private mode");
    let alias = Directory::new();
    symlink(&dir.0, &alias.0).expect("directory alias");
    assert!(TemporaryAudio::open(&alias.0).is_err());
    fs::write(dir.0.join("keep.txt"), b"unchanged").expect("fixture");
    symlink(dir.0.join("keep.txt"), dir.0.join(".lock")).expect("lock alias");
    assert!(TemporaryAudio::open(&dir.0).is_err());
    assert_eq!(fs::read(dir.0.join("keep.txt")).expect("fixture"), b"unchanged");
}

#[test]
fn stale_audio_symlinks_are_unlinked_without_touching_their_targets() {
    let dir = Directory::new();
    let temporary = TemporaryAudio::open(&dir.0).expect("directory");
    drop(temporary);
    let outside = dir.0.join("outside.wav");
    fs::write(&outside, b"keep").expect("outside file");
    symlink(&outside, dir.0.join("mic.wav")).expect("stale alias");
    TemporaryAudio::open(&dir.0)
        .expect("recovery")
        .finish()
        .expect("cleanup");
    assert_eq!(fs::read(outside).expect("outside file"), b"keep");
}

fn writer(file: File, channels: u16, rate: u32, limit: u64) -> (mpsc::SyncSender<Vec<f32>>, Writer) {
    let (tx, chunks) = mpsc::sync_channel(16);
    let wav = hound::WavWriter::new(
        BufWriter::new(file),
        hound::WavSpec {
            channels,
            sample_rate: rate,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        },
    )
    .expect("WAV header");
    (
        tx,
        Writer {
            wav,
            chunks,
            track: Track {
                channels,
                rate,
                frames: 0,
                peak: 0.0,
            },
            limit,
            last_chunk: Instant::now(),
        },
    )
}

#[test]
fn separate_wavs_retain_native_formats_and_bound_frame_counts() {
    let dir = Directory::new();
    let temporary = TemporaryAudio::open(&dir.0).expect("directory");
    let (mic_tx, mut mic) = writer(temporary.file("mic.wav").expect("mic"), 1, 16_000, 3);
    let (sys_tx, mut system) = writer(temporary.file("system.wav").expect("system"), 2, 48_000, 2);
    mic_tx.send(vec![0.1, 0.2, 0.3, 0.4]).expect("mic chunk");
    sys_tx
        .send(vec![0.5, -0.5, 0.25, -0.25, 0.9, -0.9])
        .expect("output chunk");
    mic.drain().expect("write mic");
    system.drain().expect("write output");
    assert_eq!(mic.finish().expect("finalize mic").frames, 3);
    assert_eq!(system.finish().expect("finalize output").frames, 2);
    let mic = hound::WavReader::open(dir.0.join("mic.wav")).expect("read mic");
    let system = hound::WavReader::open(dir.0.join("system.wav")).expect("read output");
    assert_eq!(
        (mic.spec().channels, mic.spec().sample_rate, mic.duration()),
        (1, 16_000, 3)
    );
    assert_eq!(
        (
            system.spec().channels,
            system.spec().sample_rate,
            system.duration()
        ),
        (2, 48_000, 2)
    );
    assert_eq!(
        system
            .into_samples::<f32>()
            .collect::<Result<Vec<_>, _>>()
            .expect("samples"),
        [0.5, -0.5, 0.25, -0.25]
    );
    temporary.finish().expect("remove both");
    assert!(!dir.0.join("mic.wav").exists() && !dir.0.join("system.wav").exists());
}

#[test]
fn empty_stream_and_duration_errors_leave_no_audio() {
    let dir = Directory::new();
    for seconds in [0, 61, u32::MAX] {
        assert!(record_for(&dir.0, seconds).is_err());
        assert!(!dir.0.exists(), "reject before opening devices or touching disk");
    }
    let temporary = TemporaryAudio::open(&dir.0).expect("directory");
    let (_, mic) = writer(temporary.file("mic.wav").expect("mic"), 1, 16_000, 16_000);
    assert!(mic.finish().is_err());
    drop(temporary);
    assert!(!dir.0.join("mic.wav").exists());
}
