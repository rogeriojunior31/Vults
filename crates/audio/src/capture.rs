use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

use crate::meter::Metering;
use crate::resample::Converter;
use crate::{Meter, SAMPLE_RATE};

#[derive(Debug, Clone)]
pub struct CaptureOptions {
    pub device: Option<String>,
    /// Hard limit: 1..600 seconds. Chat keeps its existing 60-second limit.
    pub max_seconds: u32,
}

impl Default for CaptureOptions {
    fn default() -> Self {
        Self {
            device: None,
            max_seconds: 60,
        }
    }
}

/// Already resampled audio; readers never hold the microphone's callback lock.
#[derive(Debug, Clone)]
pub struct Audio(Arc<Mutex<Vec<f32>>>);

impl Audio {
    pub fn all(&self) -> Vec<f32> {
        self.tail(u32::MAX)
    }

    pub fn tail(&self, milliseconds: u32) -> Vec<f32> {
        let keep = u64::from(milliseconds) * u64::from(SAMPLE_RATE) / 1000;
        let all = self.0.lock().unwrap_or_else(|e| e.into_inner());
        all[all
            .len()
            .saturating_sub(usize::try_from(keep).unwrap_or(usize::MAX))..]
            .to_vec()
    }

    pub fn len(&self) -> usize {
        self.0.lock().unwrap_or_else(|e| e.into_inner()).len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[derive(Debug)]
pub struct Capture {
    stop: mpsc::Sender<()>,
    thread: Option<JoinHandle<Result<Vec<f32>, String>>>,
    audio: Audio,
}

impl Capture {
    /// Opens the requested microphone, or the default if it is not listed. Returns only once
    /// the stream is playing; metering runs on the capture thread, about 30 times a second.
    pub fn start(options: CaptureOptions, meter: impl Fn(Meter) + Send + 'static) -> Result<Self, String> {
        if !(1..=600).contains(&options.max_seconds) {
            return Err("microphone capture must be limited to 1-600 seconds".into());
        }
        let audio = Audio(Arc::new(Mutex::new(Vec::with_capacity(
            options.max_seconds as usize * SAMPLE_RATE as usize,
        ))));
        let shared = audio.clone();
        let (stop, stopped) = mpsc::channel();
        let (ready, started) = mpsc::channel();
        let thread = std::thread::Builder::new()
            .name("audio-capture".into())
            .spawn(move || capture(options, shared, meter, stopped, ready))
            .map_err(|e| e.to_string())?;
        match started.recv() {
            Ok(Ok(())) => Ok(Self {
                stop,
                thread: Some(thread),
                audio,
            }),
            Ok(Err(error)) => {
                let _ = thread.join();
                Err(error)
            }
            Err(_) => Err(thread.join().map_or_else(
                |_| "the recorder crashed".into(),
                |r| r.err().unwrap_or_else(|| "the recorder did not start".into()),
            )),
        }
    }

    pub fn audio(&self) -> Audio {
        self.audio.clone()
    }

    pub fn finish(mut self) -> Result<Vec<f32>, String> {
        let _ = self.stop.send(());
        match self.thread.take().map(JoinHandle::join) {
            Some(Ok(result)) => result,
            _ => Err("the recorder crashed".into()),
        }
    }
}

impl Drop for Capture {
    fn drop(&mut self) {
        let _ = self.stop.send(());
    }
}

fn monitor(name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    name == "monitor" || name.ends_with(".monitor") || name.starts_with("monitor of ")
}

/// Lists input names without starting a recording; monitor inputs are never offered.
pub fn microphones() -> Result<Vec<String>, String> {
    let host = cpal::default_host();
    let mut names: Vec<String> = host
        .input_devices()
        .map_err(|_| "Could not list microphones.")?
        .map(|device| device.to_string())
        .collect();
    if let Some(device) = host.default_input_device() {
        names.push(device.to_string());
    }
    Ok(microphone_names(names))
}

/// The selected input's native format, without opening a stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputInfo {
    pub host: String,
    pub device: String,
    pub rate: u32,
}

pub fn input_info(preferred: Option<&str>) -> Result<InputInfo, String> {
    let device = input(preferred)?;
    let config = device
        .default_input_config()
        .map_err(|_| "Could not read microphone format.")?;
    Ok(InputInfo {
        host: cpal::default_host().id().name().into(),
        device: device.to_string(),
        rate: config.sample_rate(),
    })
}

fn microphone_names(mut names: Vec<String>) -> Vec<String> {
    names.retain(|name| !name.trim().is_empty() && !monitor(name));
    names.sort();
    names.dedup();
    names
}

fn input_index(names: &[String], preferred: Option<&str>, default: Option<usize>) -> Option<usize> {
    preferred
        .and_then(|name| names.iter().position(|n| n == name && !monitor(n)))
        .or_else(|| default.filter(|&i| names.get(i).is_some_and(|n| !monitor(n))))
        .or_else(|| names.iter().position(|n| !monitor(n)))
}

fn input(preferred: Option<&str>) -> Result<cpal::Device, String> {
    let host = cpal::default_host();
    let mut devices: Vec<_> = host.input_devices().map(|d| d.collect()).unwrap_or_default();
    let default = host.default_input_device().map(|device| {
        devices.push(device);
        devices.len() - 1
    });
    let names: Vec<_> = devices.iter().map(ToString::to_string).collect();
    let index = input_index(&names, preferred, default).ok_or("no microphone found")?;
    Ok(devices.swap_remove(index))
}

type Chunks = mpsc::Receiver<Vec<f32>>;

fn open(
    preferred: Option<&str>,
    failed: Arc<Mutex<Option<String>>>,
) -> Result<(cpal::Stream, Converter, Chunks), String> {
    let device = input(preferred)?;
    let config = device
        .default_input_config()
        .map_err(|e| format!("can't read the microphone's format: {e}"))?;
    let (channels, rate) = (config.channels(), config.sample_rate());
    if !(1..=32).contains(&channels) {
        return Err("unsupported microphone channel count".into());
    }
    // Bound callback backlog independently of the ten-minute transcription buffer.
    let (tx, rx) = mpsc::sync_channel(16);
    let stream = match config.sample_format() {
        cpal::SampleFormat::F32 => stream::<f32>(&device, config.into(), tx, failed),
        cpal::SampleFormat::I16 => stream::<i16>(&device, config.into(), tx, failed),
        cpal::SampleFormat::U16 => stream::<u16>(&device, config.into(), tx, failed),
        cpal::SampleFormat::I32 => stream::<i32>(&device, config.into(), tx, failed),
        cpal::SampleFormat::F64 => stream::<f64>(&device, config.into(), tx, failed),
        other => return Err(format!("the microphone's format ({other}) is not supported")),
    }?;
    let converter = Converter::new(rate)?;
    stream
        .play()
        .map_err(|e| format!("can't start the microphone: {e}"))?;
    Ok((stream, converter, rx))
}

fn stream<T>(
    device: &cpal::Device,
    config: cpal::StreamConfig,
    tx: mpsc::SyncSender<Vec<f32>>,
    failed: Arc<Mutex<Option<String>>>,
) -> Result<cpal::Stream, String>
where
    T: cpal::SizedSample,
    f32: cpal::FromSample<T>,
{
    let channels = usize::from(config.channels);
    let errors = failed.clone();
    device
        .build_input_stream::<T, _, _>(
            config,
            move |data, _| {
                if !data.len().is_multiple_of(channels) {
                    *failed.lock().unwrap_or_else(|e| e.into_inner()) =
                        Some("incomplete microphone frame".into());
                    return;
                }
                let mono: Vec<f32> = data
                    .chunks_exact(channels)
                    .map(|frame| {
                        frame
                            .iter()
                            .map(|s| (*s).to_sample::<f32>() / channels as f32)
                            .sum()
                    })
                    .collect();
                let error = if mono.iter().any(|s| !s.is_finite()) {
                    Some("the microphone returned a non-finite sample")
                } else if matches!(tx.try_send(mono), Err(mpsc::TrySendError::Full(_))) {
                    Some("microphone capture could not keep up")
                } else {
                    None
                };
                if let Some(error) = error {
                    *failed.lock().unwrap_or_else(|e| e.into_inner()) = Some(error.into());
                }
            },
            move |error| {
                *errors.lock().unwrap_or_else(|e| e.into_inner()) = Some(format!("microphone: {error}"));
            },
            None,
        )
        .map_err(|e| format!("can't open the microphone: {e}"))
}

fn capture(
    options: CaptureOptions,
    audio: Audio,
    notify: impl Fn(Meter),
    stop: mpsc::Receiver<()>,
    ready: mpsc::Sender<Result<(), String>>,
) -> Result<Vec<f32>, String> {
    let failed = Arc::new(Mutex::new(None));
    let opened = open(options.device.as_deref(), failed.clone());
    let (stream, mut converter, chunks) = match opened {
        Ok(opened) => opened,
        Err(error) => {
            let _ = ready.send(Err(error.clone()));
            return Err(error);
        }
    };
    let mut meter = Metering::new();
    let cap = options.max_seconds as usize * SAMPLE_RATE as usize;
    let deadline = Instant::now() + Duration::from_secs(u64::from(options.max_seconds));
    let _ = ready.send(Ok(()));
    let append = |pcm: &[f32], meter: &mut Metering| {
        let mut all = audio.0.lock().unwrap_or_else(|e| e.into_inner());
        let take = pcm.len().min(cap.saturating_sub(all.len()));
        all.extend_from_slice(&pcm[..take]);
        drop(all);
        for &sample in &pcm[..take] {
            if let Some(update) = meter.push(sample) {
                notify(update);
            }
        }
    };
    loop {
        if Instant::now() >= deadline
            || audio.len() >= cap
            || stop.try_recv() != Err(mpsc::TryRecvError::Empty)
        {
            break;
        }
        if failed.lock().unwrap_or_else(|e| e.into_inner()).is_some() {
            break;
        }
        match chunks.recv_timeout(Duration::from_millis(20)) {
            Ok(chunk) => append(&converter.push(&chunk)?, &mut meter),
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }
    drop(stream);
    if let Some(error) = failed.lock().unwrap_or_else(|e| e.into_inner()).take() {
        return Err(error);
    }
    for chunk in chunks.try_iter() {
        append(&converter.push(&chunk)?, &mut meter);
    }
    append(&converter.finish()?, &mut meter);
    Ok(std::mem::take(
        &mut *audio.0.lock().unwrap_or_else(|e| e.into_inner()),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_devices_fall_back_and_monitors_are_never_selected() {
        let names: Vec<String> = ["speakers.monitor", "USB mic", "default", "Monitor of Speakers"]
            .map(str::to_owned)
            .into();
        assert_eq!(input_index(&names, Some("missing"), Some(2)), Some(2));
        assert_eq!(input_index(&names, Some("USB mic"), Some(2)), Some(1));
        assert_eq!(input_index(&names, Some("speakers.monitor"), Some(2)), Some(2));
        assert_eq!(input_index(&names, None, Some(0)), Some(1));
        assert_eq!(input_index(&["monitor".into()], None, None), None);
        assert_eq!(microphone_names(names), ["USB mic", "default"]);
        assert!(microphone_names(vec![" ".into(), "monitor".into()]).is_empty());
        assert_eq!(
            microphone_names(vec!["USB mic".into(), "USB mic".into()]),
            ["USB mic"]
        );
    }

    #[test]
    fn invalid_limits_never_open_a_microphone() {
        for max_seconds in [0, 601, u32::MAX] {
            assert!(
                Capture::start(
                    CaptureOptions {
                        max_seconds,
                        ..Default::default()
                    },
                    |_| {}
                )
                .is_err()
            );
        }
        assert_eq!(CaptureOptions::default().max_seconds, 60);
        let audio = Audio(Arc::new(Mutex::new(vec![0.1; 32_000])));
        assert_eq!(audio.tail(500).len(), 8_000);
        assert_eq!(audio.all().len(), 32_000);
    }
}
