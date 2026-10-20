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
            channels: 1,
            sample_rate: crate::SAMPLE_RATE,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        },
    )
    .expect("WAV header");
    (
        tx,
        Writer {
            wav,
            chunks,
            track: Track {
                channels: 1,
                rate: crate::SAMPLE_RATE,
                frames: 0,
                peak: 0.0,
            },
            limit,
            last_chunk: Instant::now(),
            input_channels: channels,
            converter: crate::resample::Converter::new(rate).unwrap(),
        },
    )
}

#[test]
fn separate_wavs_normalize_to_bounded_mono_pcm16() {
    let dir = Directory::new();
    let temporary = TemporaryAudio::open(&dir.0).expect("directory");
    let (mic_tx, mut mic) = writer(temporary.file("mic.wav").expect("mic"), 1, 16_000, 3);
    let (sys_tx, mut system) = writer(temporary.file("system.wav").expect("system"), 2, 48_000, 2);
    mic_tx.send(vec![0.1, 0.2, 0.3, 0.4]).expect("mic chunk");
    sys_tx.send(vec![0.5; 48]).expect("output chunk");
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
        (1, 16_000, 2)
    );
    assert_eq!(
        system
            .into_samples::<i16>()
            .collect::<Result<Vec<_>, _>>()
            .expect("samples"),
        crate::resample(&[0.5; 48], 2, 48_000).unwrap()[..2]
            .iter()
            .map(|sample| (sample * 32768.0).round() as i16)
            .collect::<Vec<_>>()
    );
    temporary.finish().expect("remove both");
    assert!(!dir.0.join("mic.wav").exists() && !dir.0.join("system.wav").exists());
}

#[test]
fn empty_stream_and_duration_errors_leave_no_audio() {
    let dir = Directory::new();
    for seconds in [0, MAX_SECONDS + 1, u32::MAX] {
        assert!(record_for(&dir.0, seconds).is_err());
        assert!(!dir.0.exists(), "reject before opening devices or touching disk");
    }
    let temporary = TemporaryAudio::open(&dir.0).expect("directory");
    let (_, mic) = writer(temporary.file("mic.wav").expect("mic"), 1, 16_000, 16_000);
    assert!(mic.finish().is_err());
    drop(temporary);
    assert!(!dir.0.join("mic.wav").exists());
}

#[test]
fn long_capture_streams_to_fixed_rate_pcm_without_retaining_the_meeting() {
    let dir = Directory::new();
    let temporary = TemporaryAudio::open(&dir.0).unwrap();
    let seconds = 75_u32;
    let frames = u64::from(seconds) * u64::from(crate::SAMPLE_RATE);
    let (tx, mut output) = writer(temporary.file("mic.wav").unwrap(), 2, 48_000, frames);
    for block in 0..seconds as usize * 100 {
        let mut chunk = Vec::with_capacity(960);
        for frame in 0..480 {
            let tone = ((block * 480 + frame) as f32 * std::f32::consts::TAU * 1000.0 / 48_000.0).sin();
            chunk.extend([tone * 0.5, tone * 0.25]);
        }
        tx.send(chunk).unwrap();
        output.drain().unwrap();
        assert!(output.track.frames <= frames);
    }
    let report = output.finish().unwrap();
    assert_eq!((report.channels, report.rate, report.frames), (1, 16_000, frames));
    assert!((report.peak - 0.375).abs() < 0.02);
    let file = temporary.reader("mic.wav").unwrap();
    let bytes = file.metadata().unwrap().len();
    let reader = hound::WavReader::new(file).unwrap();
    assert_eq!(reader.duration(), 75 * 16_000);
    assert_eq!(reader.spec().sample_format, hound::SampleFormat::Int);
    assert_eq!(reader.spec().bits_per_sample, 16);
    assert!(
        bytes <= frames * 2 + 128,
        "Storage grows only with normalized PCM16"
    );
    assert!(2 * (u64::from(MAX_SECONDS) * 16_000 * 2 + 128) < 1024 * 1024 * 1024);
    drop(reader);
    temporary.finish().unwrap();
    assert!(!dir.0.join("mic.wav").exists());
}

#[test]
fn clipped_audio_and_failed_disk_writes_never_wrap_or_report_success() {
    let dir = Directory::new();
    let temporary = TemporaryAudio::open(&dir.0).unwrap();
    let (tx, mut output) = writer(temporary.file("mic.wav").unwrap(), 1, 16_000, 2);
    tx.send(vec![2.0, -2.0]).unwrap();
    output.drain().unwrap();
    assert_eq!(output.finish().unwrap().peak, 1.0);
    let samples = hound::WavReader::new(temporary.reader("mic.wav").unwrap())
        .unwrap()
        .into_samples::<i16>()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(samples, [i16::MAX, i16::MIN]);
    drop(temporary.file("system.wav").unwrap());
    let (tx, mut output) = writer(temporary.reader("system.wav").unwrap(), 1, 16_000, 20_000);
    tx.send(vec![0.5; 20_000]).unwrap();
    assert!(
        output.drain().is_err(),
        "A write error cannot become a completed track"
    );
    drop(output);
    temporary.finish().unwrap();
    assert!(!dir.0.join("mic.wav").exists() && !dir.0.join("system.wav").exists());
}

#[test]
fn source_clocks_survive_packet_boundaries_at_low_fractional_and_high_rates() {
    for rate in [8_000, 44_100, 192_000] {
        let dir = Directory::new();
        let temporary = TemporaryAudio::open(&dir.0).unwrap();
        let (tx, mut output) = writer(temporary.file("mic.wav").unwrap(), 2, rate, u64::MAX);
        let frames = rate as usize + 137;
        for start in (0..frames).step_by(137) {
            tx.send(vec![0.1; (frames - start).min(137) * 2]).unwrap();
            output.drain().unwrap();
        }
        let report = output.finish().unwrap();
        assert_eq!(report.frames, frames as u64 * 16_000 / u64::from(rate));
        let wav = hound::WavReader::new(temporary.reader("mic.wav").unwrap()).unwrap();
        assert_eq!(u64::from(wav.duration()), report.frames);
        assert_eq!((wav.spec().channels, wav.spec().sample_rate), (1, 16_000));
        drop(wav);
        temporary.finish().unwrap();
    }
}
