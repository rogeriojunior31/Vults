//! Two independent disk-backed meeting streams; bounded queues and mono PCM16 storage.

mod control;
pub(crate) mod pulse;
mod temporary;

pub use control::{Outcome, Phase, Recording, Status, Tracks, recover};

use std::fs::File;
use std::io::BufWriter;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

use pulse::Connection;
use pulseaudio::protocol::{self, SourceInfo};
use temporary::TemporaryAudio;

/// Four hours of two mono 16 kHz PCM16 tracks stays below 1 GiB, including WAV headers.
pub const MAX_SECONDS: u32 = 4 * 60 * 60;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sources {
    pub microphone: String,
    pub monitor: String,
    pub monitor_fallback: bool,
}

/// Server identifiers, not translated descriptions. Never substitute a microphone for a monitor.
fn monitor_index(names: &[String], sink: Option<&str>) -> Option<usize> {
    sink.and_then(|sink| names.iter().position(|name| name == &format!("{sink}.monitor")))
        .or_else(|| names.iter().position(|name| name.ends_with(".monitor")))
}

fn microphone_index(names: &[String], default: Option<&str>) -> Option<usize> {
    default
        .and_then(|default| names.iter().position(|name| name == default && !is_monitor(name)))
        .or_else(|| names.iter().position(|name| !is_monitor(name)))
}

fn is_monitor(name: &str) -> bool {
    name.is_empty() || name.ends_with(".monitor")
}

fn devices(connection: &Connection) -> Result<(Sources, SourceInfo, SourceInfo), String> {
    let server = connection.wait(connection.client.server_info())?;
    let inputs = connection.wait(connection.client.list_sources())?;
    select_devices(server, inputs)
}

fn select_devices(
    server: protocol::ServerInfo,
    mut inputs: Vec<SourceInfo>,
) -> Result<(Sources, SourceInfo, SourceInfo), String> {
    inputs.retain(|source| !source.name.as_bytes().is_empty());
    inputs.sort_by(|a, b| a.name.cmp(&b.name));
    let name = |source: &SourceInfo| source.name.to_string_lossy().into_owned();
    let monitor = |source: &&SourceInfo| source.monitor_of_sink_index.is_some() || is_monitor(&name(source));
    let microphones: Vec<_> = inputs.iter().filter(|source| !monitor(source)).collect();
    let monitors: Vec<_> = inputs.iter().filter(monitor).collect();
    let mic_names: Vec<_> = microphones.iter().map(|source| name(source)).collect();
    let monitor_names: Vec<_> = monitors.iter().map(|source| name(source)).collect();
    let default = server
        .default_source_name
        .as_ref()
        .map(|name| name.to_string_lossy());
    let sink = server
        .default_sink_name
        .as_ref()
        .map(|name| name.to_string_lossy());
    let mic = microphone_index(&mic_names, default.as_deref()).ok_or("no non-monitor microphone found")?;
    let preferred = monitors.iter().position(|source| {
        source
            .monitor_of_sink_name
            .as_ref()
            .zip(server.default_sink_name.as_ref())
            .is_some_and(|(monitor, sink)| monitor == sink)
    });
    let system = preferred
        .or_else(|| monitor_index(&monitor_names, sink.as_deref()))
        .or_else(|| (!monitors.is_empty()).then_some(0))
        .ok_or("no system-audio monitor found")?;
    let sources = Sources {
        microphone: mic_names[mic].clone(),
        monitor: monitor_names[system].clone(),
        monitor_fallback: preferred.is_none()
            && sink.is_none_or(|sink| monitor_names[system] != format!("{sink}.monitor")),
    };
    Ok((sources, microphones[mic].clone(), monitors[system].clone()))
}

/// Lists selected sources using bounded local requests, without opening an audio stream.
pub fn sources() -> Result<Sources, String> {
    let connection = Connection::open(None, 0)?;
    Ok(devices(&connection)?.0)
}

#[derive(Debug)]
pub struct Report {
    pub sources: Sources,
    pub microphone: Track,
    pub system: Track,
    /// Time spent starting both streams, not sample-clock synchronization.
    pub startup: Duration,
    pub end: End,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum End {
    Stopped,
    Limit,
}

#[derive(Debug)]
pub struct Track {
    pub rate: u32,
    pub channels: u16,
    pub frames: u64,
    pub peak: f32,
}

struct Writer {
    wav: hound::WavWriter<BufWriter<File>>,
    chunks: Receiver<Vec<f32>>,
    track: Track,
    limit: u64,
    last_chunk: Instant,
    input_channels: u16,
    converter: crate::resample::Converter,
}

impl Writer {
    fn drain(&mut self) -> Result<(), String> {
        for chunk in self.chunks.try_iter().take(16) {
            if !chunk.is_empty() {
                self.last_chunk = Instant::now();
            }
            if !chunk.len().is_multiple_of(usize::from(self.input_channels)) {
                return Err("meeting audio contains an incomplete frame".into());
            }
            let mono: Vec<f32> = chunk
                .chunks_exact(usize::from(self.input_channels))
                .map(|frame| {
                    frame
                        .iter()
                        .map(|sample| sample / f32::from(self.input_channels))
                        .sum()
                })
                .collect();
            let samples = self.converter.push(&mono)?;
            write_pcm(&mut self.wav, &mut self.track, self.limit, &samples)?;
        }
        Ok(())
    }

    fn finish(mut self) -> Result<Track, String> {
        self.drain()?;
        let tail = self.converter.finish()?;
        write_pcm(&mut self.wav, &mut self.track, self.limit, &tail)?;
        self.wav.finalize().map_err(|e| e.to_string())?;
        if self.track.frames == 0 {
            return Err("an audio source delivered no frames".into());
        }
        Ok(self.track)
    }
}

fn write_pcm(
    wav: &mut hound::WavWriter<BufWriter<File>>,
    track: &mut Track,
    limit: u64,
    samples: &[f32],
) -> Result<(), String> {
    if samples.iter().any(|sample| !sample.is_finite()) {
        return Err("meeting audio contains a non-finite sample".into());
    }
    for &sample in samples.iter().take(limit.saturating_sub(track.frames) as usize) {
        // Resampling may overshoot full scale; the integer cast saturates instead of wrapping.
        let stored = (sample * 32768.0).round() as i16;
        wav.write_sample(stored).map_err(|e| e.to_string())?;
        track.peak = track.peak.max(f32::from(stored).abs() / 32768.0);
        track.frames += 1;
    }
    Ok(())
}

fn open(
    connection: &Connection,
    source: &SourceInfo,
    file: File,
    seconds: u32,
    failed: Arc<AtomicBool>,
    live: Arc<AtomicBool>,
) -> Result<(pulseaudio::RecordStream, Writer), String> {
    let (channels, rate) = (source.sample_spec.channels, source.sample_spec.sample_rate);
    if !(1..=32).contains(&channels) || !(8_000..=192_000).contains(&rate) {
        return Err("unsupported meeting audio format".into());
    }
    let wav = hound::WavWriter::new(
        BufWriter::new(file),
        hound::WavSpec {
            channels: 1,
            sample_rate: crate::SAMPLE_RATE,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        },
    )
    .map_err(|e| e.to_string())?;
    let spec = protocol::SampleSpec {
        format: protocol::SampleFormat::Float32Le,
        channels,
        sample_rate: rate,
    };
    let params = protocol::RecordStreamParams {
        sample_spec: spec,
        channel_map: source.channel_map,
        source_name: Some(source.name.clone()),
        buffer_attr: protocol::stream::BufferAttr {
            fragment_size: rate / 50 * u32::from(channels) * 4,
            ..Default::default()
        },
        flags: protocol::stream::StreamFlags {
            start_corked: true,
            adjust_latency: true,
            no_move: true,
            ..Default::default()
        },
        ..Default::default()
    };
    let (tx, chunks) = mpsc::sync_channel(16);
    let callback = move |data: &[u8]| {
        if !live.load(Ordering::SeqCst) {
            return;
        }
        if data.len() > 1_048_576 || !data.len().is_multiple_of(usize::from(channels) * 4) {
            failed.store(true, Ordering::SeqCst);
            return;
        }
        let samples: Vec<f32> = data
            .as_chunks::<4>()
            .0
            .iter()
            .map(|sample| f32::from_le_bytes([sample[0], sample[1], sample[2], sample[3]]))
            .collect();
        if samples.iter().any(|sample| !sample.is_finite())
            || (live.load(Ordering::SeqCst) && tx.try_send(samples).is_err())
        {
            failed.store(true, Ordering::SeqCst);
        }
    };
    let stream = connection.wait(connection.client.create_record_stream(params, callback))?;
    if stream.sample_spec() != &spec {
        return Err("audio server changed the requested meeting format".into());
    }
    Ok((
        stream,
        Writer {
            wav,
            chunks,
            track: Track {
                rate: crate::SAMPLE_RATE,
                channels: 1,
                frames: 0,
                peak: 0.0,
            },
            limit: u64::from(seconds) * u64::from(crate::SAMPLE_RATE),
            last_chunk: Instant::now(),
            input_channels: u16::from(channels),
            converter: crate::resample::Converter::new(rate)?,
        },
    ))
}

/// Explicit recording, capped at four hours. WAVs are removed before returning even on failure.
/// SIGKILL/power loss leaves only private files, removed by the next call under the exclusive lock.
pub fn record_for(root: &Path, seconds: u32) -> Result<Report, String> {
    let mut recording = Recording::start(root, seconds)?;
    loop {
        if let Some(result) = recording.take_result() {
            return match result? {
                Outcome::Finished(report) => Ok(report),
                Outcome::Cancelled => Err("meeting capture was cancelled".into()),
            };
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn capture(temporary: &TemporaryAudio, seconds: u32, control: &control::Control) -> Result<Outcome, String> {
    capture_checked(temporary, seconds, control, None)
}

fn capture_checked(
    temporary: &TemporaryAudio,
    seconds: u32,
    control: &control::Control,
    expected: Option<&Sources>,
) -> Result<Outcome, String> {
    if !control.continuing() {
        return Ok(Outcome::Cancelled);
    }
    let result = capture_connected(temporary, seconds, control, expected);
    if control.phase() == Phase::Cancelling {
        Ok(Outcome::Cancelled)
    } else {
        result
    }
}

fn capture_connected(
    temporary: &TemporaryAudio,
    seconds: u32,
    control: &control::Control,
    expected: Option<&Sources>,
) -> Result<Outcome, String> {
    let connection = Connection::open(Some(control.clone()), seconds)?;
    let (sources, mic_source, system_source) = devices(&connection)?;
    if expected.is_some_and(|expected| expected != &sources) {
        return Err("meeting audio sources changed; review them before recording".into());
    }
    let failed = Arc::new(AtomicBool::new(false));
    let live = Arc::new(AtomicBool::new(true));
    let callbacks = Callbacks(live.clone());
    let (mic, mut microphone) = open(
        &connection,
        &mic_source,
        temporary.file("mic.wav")?,
        seconds,
        failed.clone(),
        live.clone(),
    )?;
    let (system, mut output) = open(
        &connection,
        &system_source,
        temporary.file("system.wav")?,
        seconds,
        failed.clone(),
        live,
    )?;
    let start = Instant::now();
    connection.wait(mic.uncork())?;
    connection.wait(system.uncork())?;
    connection.wait(mic.started())?;
    connection.wait(system.started())?;
    let startup = start.elapsed();
    if !control.started() {
        return Ok(Outcome::Cancelled);
    }
    let start = Instant::now();
    let mut checked = start;
    microphone.last_chunk = start;
    output.last_chunk = start;
    let end = loop {
        match control.phase() {
            Phase::Cancelling => return Ok(Outcome::Cancelled),
            Phase::Stopping => break End::Stopped,
            _ => {}
        }
        if start.elapsed() >= Duration::from_secs(u64::from(seconds)) {
            break End::Limit;
        }
        if failed.load(Ordering::SeqCst) {
            return Err("meeting audio overflowed or returned invalid samples".into());
        }
        microphone.drain()?;
        output.drain()?;
        if microphone.last_chunk.elapsed() >= pulse::TIMEOUT || output.last_chunk.elapsed() >= pulse::TIMEOUT
        {
            return Err("a meeting audio source stopped delivering frames".into());
        }
        if checked.elapsed() >= Duration::from_millis(500) {
            if let Err(error) = connection.wait(connection.client.server_info()) {
                match control.phase() {
                    Phase::Cancelling => return Ok(Outcome::Cancelled),
                    Phase::Stopping => break End::Stopped,
                    _ if start.elapsed() >= Duration::from_secs(u64::from(seconds)) => break End::Limit,
                    _ => return Err(error),
                }
            }
            checked = Instant::now();
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    drop(callbacks);
    connection.close();
    drop((mic, system));
    if failed.load(Ordering::SeqCst) {
        return Err("meeting audio overflowed or returned invalid samples".into());
    }
    Ok(Outcome::Finished(Report {
        sources,
        microphone: microphone.finish()?,
        system: output.finish()?,
        startup,
        end,
    }))
}

struct Callbacks(Arc<AtomicBool>);

impl Drop for Callbacks {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}

#[cfg(test)]
mod tests;
