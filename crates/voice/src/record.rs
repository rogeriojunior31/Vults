//! Chat's transcription watchers stay here; device capture belongs to vults-audio.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use vults_audio::{Capture, CaptureOptions};

use crate::preview::Preview;
use crate::{EndOfSpeech, Vad};

pub(crate) use vults_audio::Audio;

pub const MAX_SECONDS: u32 = 60;
pub(crate) const WATCH_EVERY: Duration = Duration::from_millis(100);
pub(crate) const WATCH_MS: u32 = 3_000;
const _: () = assert!(2 * crate::vad::PAUSE <= (crate::SAMPLE_RATE * WATCH_MS / 1000) as usize);

/// Tap-to-talk watches for end of speech but leaves stopping to its caller.
pub struct AutoStop {
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
    capture: Option<Capture>,
    preview: Option<Preview>,
    watching: Arc<AtomicBool>,
}

impl Recorder {
    /// Chat retains its one-minute cap and scalar waveform; other callers can use audio's bands.
    pub fn start(level: impl Fn(f32) + Send + 'static, auto: Option<AutoStop>) -> Result<Self, String> {
        Self::start_guarded(level, auto, || true)
    }

    /// The owner may revoke admission while native device setup is still in progress.
    pub fn start_guarded(
        level: impl Fn(f32) + Send + 'static,
        auto: Option<AutoStop>,
        live: impl Fn() -> bool + Send + 'static,
    ) -> Result<Self, String> {
        let capture = Capture::start_guarded(
            CaptureOptions {
                max_seconds: MAX_SECONDS,
                ..Default::default()
            },
            move |meter| level(meter.level),
            live,
            || true,
        )?;
        let watching = Arc::new(AtomicBool::new(true));
        if let Some(auto) = auto {
            let audio = capture.audio();
            let flag = watching.clone();
            std::thread::Builder::new()
                .name("voice-end-of-speech".into())
                .spawn(move || {
                    if let Err(error) = watch(audio, auto, &flag) {
                        tracing::warn!("voice: {error}");
                    }
                })
                .map_err(|e| e.to_string())?;
        }
        Ok(Self {
            capture: Some(capture),
            preview: None,
            watching,
        })
    }

    pub fn preview(
        &mut self,
        load: impl FnOnce() -> Result<Arc<crate::Transcriber>, String> + Send + 'static,
        language: Option<String>,
        show: impl Fn(String) + Send + 'static,
    ) {
        if let Some(capture) = &self.capture {
            self.preview = Some(Preview::start(
                capture.audio(),
                load,
                language,
                true,
                Vec::new,
                show,
            ));
        }
    }

    pub fn finish(mut self) -> Result<Vec<f32>, String> {
        self.watching.store(false, Ordering::SeqCst);
        drop(self.preview.take());
        self.capture
            .take()
            .ok_or("the recorder already stopped")?
            .finish()
    }
}

impl Drop for Recorder {
    fn drop(&mut self) {
        self.watching.store(false, Ordering::SeqCst);
        drop(self.preview.take());
        drop(self.capture.take());
    }
}

fn watch(audio: Audio, auto: AutoStop, running: &AtomicBool) -> Result<(), String> {
    let mut vad = Vad::load(&auto.vad)?;
    let mut end = EndOfSpeech::default();
    let deadline = Instant::now() + Duration::from_secs(u64::from(MAX_SECONDS));
    while running.load(Ordering::SeqCst) && Instant::now() < deadline {
        std::thread::sleep(WATCH_EVERY);
        if !running.load(Ordering::SeqCst) {
            break;
        }
        let pcm = audio.tail(WATCH_MS);
        if end.update(&vad.speech(&pcm)?, pcm.len()) {
            if running.load(Ordering::SeqCst) {
                (auto.ended)();
            }
            break;
        }
    }
    Ok(())
}
