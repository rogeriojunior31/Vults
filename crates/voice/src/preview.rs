//! The live preview: while the user speaks, the recording so far is decoded again now and then, and
//! its text shown dimmed until the final transcription replaces it. Only on a GPU (see
//! [`crate::gpu`]); the final transcription always wins.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use crate::record::Audio;
use crate::{SAMPLE_RATE, Transcriber};

/// How often the text is decoded again: often enough to follow the voice, a word or two at a time.
pub(crate) const EVERY: Duration = Duration::from_millis(800);
/// How often the thread looks whether a pass is due.
const TICK: Duration = Duration::from_millis(100);
/// Less audio than one pass apart is not worth a first pass.
const LEAST: usize = SAMPLE_RATE as usize * 4 / 5;
/// New audio a pass needs over the last one: less brings no new word.
const GROWN: usize = SAMPLE_RATE as usize / 4;

/// When to decode again. A pass starts [`EVERY`] after the last one started, but never sooner than
/// twice what that one took: the decoder is busy at most half the time, however long the
/// recording grows (a longer one takes longer to decode).
#[derive(Debug, Default)]
pub(crate) struct Pace {
    /// The last pass: when it started, how long it took, and how much audio it read.
    last: Option<(Instant, Duration, usize)>,
}

impl Pace {
    /// Whether a pass over `len` samples (16 kHz) should start `now`.
    pub fn due(&self, now: Instant, len: usize) -> bool {
        match self.last {
            _ if len < LEAST => false,
            None => true,
            Some((started, took, read)) => len >= read + GROWN && now >= started + EVERY.max(took * 2),
        }
    }

    pub fn done(&mut self, started: Instant, took: Duration, read: usize) {
        self.last = Some((started, took, read));
    }
}

/// The preview running beside a recording. Dropping it stops it: a pass under way is aborted, and
/// its text, if it comes anyway, is never shown.
#[derive(Debug)]
pub(crate) struct Preview {
    stop: Arc<AtomicBool>,
}

impl Preview {
    /// `load` gives the model (loading it here, not in the click, also readies it for the final
    /// transcription); `show` gets each new text.
    pub fn start(
        audio: Audio,
        load: impl FnOnce() -> Result<Arc<Transcriber>, String> + Send + 'static,
        language: Option<String>,
        show: impl Fn(String) + Send + 'static,
    ) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let flag = stop.clone();
        let spawned = std::thread::Builder::new()
            .name("voice-preview".into())
            .spawn(move || {
                if let Err(e) = run(&audio, load, language.as_deref(), &show, &flag) {
                    tracing::warn!("voice preview: {e}");
                }
            });
        if let Err(e) = spawned {
            tracing::warn!("voice preview: {e}");
        }
        Self { stop }
    }
}

impl Drop for Preview {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
    }
}

fn run(
    audio: &Audio,
    load: impl FnOnce() -> Result<Arc<Transcriber>, String>,
    language: Option<&str>,
    show: &dyn Fn(String),
    stop: &AtomicBool,
) -> Result<(), String> {
    // Asked here, not in the click: the first answer starts Vulkan.
    if !crate::gpu() {
        return Ok(());
    }
    let transcriber = load()?;
    let mut pace = Pace::default();
    let mut shown = String::new();
    // One pass at a time, on this thread: a newer pass never races an older one.
    while !stop.load(Ordering::SeqCst) {
        std::thread::sleep(TICK);
        let now = Instant::now();
        if !pace.due(now, audio.len()) {
            continue;
        }
        let pcm = audio.all();
        let text = transcriber.preview(&pcm, language, stop)?;
        pace.done(now, now.elapsed(), pcm.len());
        match text {
            // Stopped meanwhile: the final transcription is on its way.
            _ if stop.load(Ordering::SeqCst) => break,
            Some(text) if !text.is_empty() && text != shown => {
                shown.clone_from(&text);
                show(text);
            }
            _ => {}
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const S: usize = SAMPLE_RATE as usize;
    const MS: fn(u64) -> Duration = Duration::from_millis;

    #[test]
    fn the_first_pass_waits_for_some_speech() {
        let pace = Pace::default();
        let now = Instant::now();
        assert!(!pace.due(now, S / 2), "half a second is too little");
        assert!(pace.due(now, S));
    }

    #[test]
    fn passes_come_every_800_ms_on_new_audio() {
        let t0 = Instant::now();
        let mut pace = Pace::default();
        pace.done(t0, MS(100), S);
        assert!(!pace.due(t0 + MS(500), 2 * S), "too soon");
        assert!(pace.due(t0 + MS(800), 2 * S));
        // Paused input (a held key, nothing new): no pass reads the same audio again.
        assert!(!pace.due(t0 + MS(3000), S + S / 10));
    }

    #[test]
    fn a_slow_pass_spaces_the_next() {
        // A long recording on a slow GPU: 700 ms a pass, so the next waits 1.4 s, not 0.8.
        let t0 = Instant::now();
        let mut pace = Pace::default();
        pace.done(t0, MS(700), 20 * S);
        assert!(!pace.due(t0 + MS(1000), 22 * S));
        assert!(pace.due(t0 + MS(1400), 22 * S));
    }

    /// The whole loop on a model from `VOICE_TEST_MODEL` and `VOICE_TEST_WAV` (see vad.rs):
    /// a pass aborted by `stop` gives None and leaves the model free for the final.
    #[test]
    fn a_stop_aborts_the_pass_and_the_final_runs() {
        let (Some(model), Some(wav)) = (
            std::env::var_os("VOICE_TEST_MODEL"),
            std::env::var_os("VOICE_TEST_WAV"),
        ) else {
            return;
        };
        let Ok(bytes) = std::fs::read(wav) else { return };
        let Some(at) = bytes.windows(4).position(|w| w == b"data") else {
            return;
        };
        let words: Vec<f32> = bytes[at + 8..]
            .as_chunks::<2>()
            .0
            .iter()
            .map(|b| f32::from(i16::from_le_bytes(*b)) / f32::from(i16::MAX))
            .collect();
        let t = Transcriber::load(std::path::Path::new(&model), false).expect("model");
        let stop = AtomicBool::new(false);
        let text = t.preview(&words, Some("en"), &stop).expect("preview");
        assert!(text.is_some_and(|t| t.contains("fellow Americans")));
        // Stopped while it decodes: it gives up instead of finishing.
        // Three times the speech, so a whole pass is long enough to see it cut short.
        let long: Vec<f32> = [&words[..], &words[..], &words[..]].concat();
        let started = Instant::now();
        assert!(t.preview(&long, Some("en"), &stop).expect("preview").is_some());
        let whole = started.elapsed();
        let started = Instant::now();
        let aborted = std::thread::scope(|s| {
            s.spawn(|| {
                std::thread::sleep(MS(30));
                stop.store(true, Ordering::SeqCst);
            });
            t.preview(&long, Some("en"), &stop).expect("preview")
        });
        assert_eq!(aborted, None);
        assert!(
            started.elapsed() < whole * 3 / 4,
            "aborted after {:?}, a whole pass takes {whole:?}",
            started.elapsed()
        );
        let final_text = t.transcribe(&words, Some("en"), None).expect("final");
        assert!(final_text.contains("your country"), "{final_text}");
    }
}
