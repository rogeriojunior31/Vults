//! The microphone, into memory. cpal's stream is not `Send` on every backend, so it lives on a
//! thread of its own and this handle only talks to that thread.

use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

use crate::to_whisper;

/// A recording stops by itself after this long: a forgotten mic never keeps listening.
pub const MAX_SECONDS: u32 = 60;
/// How often the level goes to the UI (its waveform).
const LEVEL_EVERY: Duration = Duration::from_millis(50);

#[derive(Debug)]
pub struct Recorder {
    stop: mpsc::Sender<()>,
    thread: Option<JoinHandle<Result<Vec<f32>, String>>>,
}

impl Recorder {
    /// Starts recording the default input device. `level` gets the loudness (0..1) every 50 ms.
    pub fn start(level: impl Fn(f32) + Send + 'static) -> Result<Self, String> {
        let (stop, stopped) = mpsc::channel();
        let (ready, started) = mpsc::channel();
        let thread = std::thread::Builder::new()
            .name("voice-recorder".into())
            .spawn(move || record(level, stopped, ready))
            .map_err(|e| e.to_string())?;
        // The device opens (or fails) before start returns, so the UI only shows "Listening" when
        // it really listens.
        match started.recv() {
            Ok(Ok(())) => Ok(Self {
                stop,
                thread: Some(thread),
            }),
            Ok(Err(e)) => Err(e),
            Err(_) => Err(thread
                .join()
                .map_or_else(|_| "the recorder crashed".into(), |r| r.err().unwrap_or_default())),
        }
    }

    /// Stops and returns 16 kHz mono samples for whisper.
    pub fn finish(mut self) -> Result<Vec<f32>, String> {
        let _ = self.stop.send(());
        match self.thread.take().map(JoinHandle::join) {
            Some(Ok(result)) => result,
            _ => Err("the recorder crashed".into()),
        }
    }
}

impl Drop for Recorder {
    /// Dropped without `finish` (cancelled): the thread stops and the audio is thrown away.
    fn drop(&mut self) {
        let _ = self.stop.send(());
    }
}

fn record(
    level: impl Fn(f32) + Send + 'static,
    stopped: mpsc::Receiver<()>,
    ready: mpsc::Sender<Result<(), String>>,
) -> Result<Vec<f32>, String> {
    let opened = open(level);
    let (stream, samples, channels, rate) = match opened {
        Ok(v) => v,
        Err(e) => {
            let _ = ready.send(Err(e.clone()));
            return Err(e);
        }
    };
    let _ = ready.send(Ok(()));
    // Wait for stop, or for the time limit.
    let _ = stopped.recv_timeout(Duration::from_secs(u64::from(MAX_SECONDS)));
    drop(stream);
    let samples = std::mem::take(&mut *samples.lock().unwrap_or_else(|e| e.into_inner()));
    Ok(to_whisper(&samples, channels, rate))
}

type Opened = (cpal::Stream, Arc<Mutex<Vec<f32>>>, u16, u32);

fn open(level: impl Fn(f32) + Send + 'static) -> Result<Opened, String> {
    let device = cpal::default_host()
        .default_input_device()
        .ok_or("no microphone found")?;
    let config = device
        .default_input_config()
        .map_err(|e| format!("can't read the microphone's format: {e}"))?;
    let (channels, rate) = (config.channels(), config.sample_rate());
    let samples = Arc::new(Mutex::new(Vec::<f32>::with_capacity((rate * 10) as usize)));
    let cap = (rate * MAX_SECONDS) as usize * usize::from(channels);
    let sink = samples.clone();
    let every = (rate as f32 * LEVEL_EVERY.as_secs_f32()) as usize * usize::from(channels);
    let mut window = Vec::<f32>::with_capacity(every.max(1));
    // A stream error mid-recording (the device unplugged) ends in a short recording, not a crash.
    let mut push = move |data: &mut dyn Iterator<Item = f32>| {
        let mut all = sink.lock().unwrap_or_else(|e| e.into_inner());
        for s in data {
            if all.len() < cap {
                all.push(s);
            }
            window.push(s);
            if window.len() >= every.max(1) {
                let rms = (window.iter().map(|s| s * s).sum::<f32>() / window.len() as f32).sqrt();
                // Speech sits around 0.02 to 0.2 RMS; scaled so a normal voice fills the bars.
                level((rms * 6.0).min(1.0));
                window.clear();
            }
        }
    };
    let err = |e: cpal::Error| tracing::warn!("microphone: {e}");
    let stream = match config.sample_format() {
        cpal::SampleFormat::F32 => device.build_input_stream::<f32, _, _>(
            config.into(),
            move |d, _| push(&mut d.iter().copied()),
            err,
            None,
        ),
        cpal::SampleFormat::I16 => device.build_input_stream::<i16, _, _>(
            config.into(),
            move |d, _| push(&mut d.iter().map(|s| f32::from(*s) / f32::from(i16::MAX))),
            err,
            None,
        ),
        other => return Err(format!("the microphone's format ({other}) is not supported")),
    }
    .map_err(|e| format!("can't open the microphone: {e}"))?;
    stream
        .play()
        .map_err(|e| format!("can't start the microphone: {e}"))?;
    Ok((stream, samples, channels, rate))
}
