//! The microphone, into memory. cpal's stream is not `Send` on every backend, so it lives on a
//! thread of its own and this handle only talks to that thread.

use std::path::PathBuf;
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

use crate::preview::Preview;
use crate::{EndOfSpeech, Vad, to_whisper};

/// A recording stops by itself after this long: a forgotten mic never keeps listening.
pub const MAX_SECONDS: u32 = 60;
/// How often the level goes to the UI (its waveform).
const LEVEL_EVERY: Duration = Duration::from_millis(50);
/// Tap-to-talk: how often the VAD looks (the stop lands up to this late), and how far back.
pub(crate) const WATCH_EVERY: Duration = Duration::from_millis(100);
/// Twice the pause: a window that starts mid-word makes Silero misplace the end of the clipped
/// words, and the stop comes late.
pub(crate) const WATCH_MS: u32 = 3_000;
const _: () = assert!(2 * crate::vad::PAUSE <= (crate::SAMPLE_RATE * WATCH_MS / 1000) as usize);

/// Tap-to-talk: the recording watches for the end of speech and calls `ended` once. It keeps
/// recording until it is stopped; stopping is the caller's call.
pub struct AutoStop {
    /// The Silero model (see [`crate::vad_path`]).
    pub vad: PathBuf,
    pub ended: Box<dyn Fn() + Send>,
}

impl std::fmt::Debug for AutoStop {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AutoStop")
            .field("vad", &self.vad)
            .finish_non_exhaustive()
    }
}

#[derive(Debug)]
pub struct Recorder {
    stop: mpsc::Sender<()>,
    thread: Option<JoinHandle<Result<Vec<f32>, String>>>,
    audio: Audio,
    preview: Option<Preview>,
}

impl Recorder {
    /// Starts recording the default input device. `level` gets the loudness (0..1) every 50 ms.
    pub fn start(level: impl Fn(f32) + Send + 'static, auto: Option<AutoStop>) -> Result<Self, String> {
        let (stop, stopped) = mpsc::channel();
        let (ready, started) = mpsc::channel();
        let thread = std::thread::Builder::new()
            .name("voice-recorder".into())
            .spawn(move || record(level, auto, stopped, ready))
            .map_err(|e| e.to_string())?;
        // The device opens (or fails) before start returns, so the UI only shows "Listening" when
        // it really listens.
        match started.recv() {
            Ok(Ok(audio)) => Ok(Self {
                stop,
                thread: Some(thread),
                audio,
                preview: None,
            }),
            Ok(Err(e)) => Err(e),
            Err(_) => Err(thread
                .join()
                .map_or_else(|_| "the recorder crashed".into(), |r| r.err().unwrap_or_default())),
        }
    }

    /// Shows the text so far while the user speaks: `show` gets it, about every 0.8 s while it
    /// changes. Only on a GPU (see [`crate::gpu`]): elsewhere nothing comes. `load` gives the
    /// model, on the preview's own thread.
    pub fn preview(
        &mut self,
        load: impl FnOnce() -> Result<Arc<crate::Transcriber>, String> + Send + 'static,
        language: Option<String>,
        show: impl Fn(String) + Send + 'static,
    ) {
        self.preview = Some(Preview::start(self.audio.clone(), load, language, show));
    }

    /// Stops and returns 16 kHz mono samples for whisper. The preview stops first: its pass under
    /// way aborts, and the model is free for the final transcription.
    pub fn finish(mut self) -> Result<Vec<f32>, String> {
        drop(self.preview.take());
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
    auto: Option<AutoStop>,
    stopped: mpsc::Receiver<()>,
    ready: mpsc::Sender<Result<Audio, String>>,
) -> Result<Vec<f32>, String> {
    let opened = open(level);
    let (stream, audio) = match opened {
        Ok(v) => v,
        Err(e) => {
            let _ = ready.send(Err(e.clone()));
            return Err(e);
        }
    };
    let _ = ready.send(Ok(audio.clone()));
    // Without the VAD, tap-to-talk is click to start, click to stop: as before.
    let mut watch = auto.and_then(|a| match Vad::load(&a.vad) {
        Ok(vad) => Some((vad, a.ended, EndOfSpeech::default())),
        Err(e) => {
            tracing::warn!("voice: {e}");
            None
        }
    });
    // Wait for stop, or for the time limit.
    let deadline = Instant::now() + Duration::from_secs(u64::from(MAX_SECONDS));
    loop {
        let left = deadline.saturating_duration_since(Instant::now());
        let wait = if watch.is_some() {
            left.min(WATCH_EVERY)
        } else {
            left
        };
        if left.is_zero() || stopped.recv_timeout(wait) != Err(RecvTimeoutError::Timeout) {
            break;
        }
        let Some((vad, ended, end)) = watch.as_mut() else {
            continue;
        };
        let pcm = audio.tail();
        match vad.speech(&pcm) {
            Ok(speech) if end.update(&speech, pcm.len()) => {
                ended();
                watch = None;
            }
            Ok(_) => {}
            Err(e) => {
                tracing::warn!("voice: {e}");
                watch = None;
            }
        }
    }
    drop(stream);
    let samples = std::mem::take(&mut *audio.samples.lock().unwrap_or_else(|e| e.into_inner()));
    Ok(to_whisper(&samples, audio.channels, audio.rate))
}

/// The recording as the microphone gives it, shared with its audio callback.
#[derive(Debug, Clone)]
pub(crate) struct Audio {
    samples: Arc<Mutex<Vec<f32>>>,
    channels: u16,
    rate: u32,
}

impl Audio {
    /// Copied out first, then converted: the audio callback waits on the same lock.
    fn from(&self, keep: impl FnOnce(usize) -> usize) -> Vec<f32> {
        let frame = usize::from(self.channels.max(1));
        let part = {
            let all = self.samples.lock().unwrap_or_else(|e| e.into_inner());
            let from = all.len().saturating_sub(keep(all.len()));
            all[from - from % frame..].to_vec()
        };
        to_whisper(&part, self.channels, self.rate)
    }

    /// The last [`WATCH_MS`], as whisper takes it.
    fn tail(&self) -> Vec<f32> {
        let n = (self.rate * WATCH_MS / 1000) as usize * usize::from(self.channels.max(1));
        self.from(|_| n)
    }

    /// All of it so far, as whisper takes it.
    pub fn all(&self) -> Vec<f32> {
        self.from(|len| len)
    }

    /// How long it is, in whisper's samples.
    pub fn len(&self) -> usize {
        let raw = self.samples.lock().unwrap_or_else(|e| e.into_inner()).len();
        let frames = raw / usize::from(self.channels.max(1));
        (frames as u64 * u64::from(crate::SAMPLE_RATE) / u64::from(self.rate.max(1))) as usize
    }
}

type Opened = (cpal::Stream, Audio);

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
    Ok((
        stream,
        Audio {
            samples,
            channels,
            rate,
        },
    ))
}
