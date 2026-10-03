//! Push-to-talk for the chat. The microphone is recorded into memory (never to disk), then
//! transcribed on this computer by whisper.cpp with a model the user downloaded on purpose.
//! Nothing here sends audio anywhere.

mod language;
mod models;
mod record;

use std::path::Path;

pub use language::Language;
use language::Spoken;
pub use models::{MODELS, Model, download, installed, model, model_path};
pub use record::Recorder;

/// What whisper takes: 16 kHz, mono, f32.
pub const SAMPLE_RATE: u32 = 16_000;

/// A loaded model, kept between recordings: loading takes longer than a short transcription.
pub struct Transcriber {
    ctx: whisper_rs::WhisperContext,
}

impl std::fmt::Debug for Transcriber {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Transcriber")
    }
}

impl Transcriber {
    pub fn load(model: &Path) -> Result<Self, String> {
        // whisper.cpp logs every layer it loads to stderr; through `tracing` it stays quiet.
        whisper_rs::install_logging_hooks();
        let path = model.to_str().ok_or("the model path is not UTF-8")?;
        let ctx = whisper_rs::WhisperContext::new_with_params(
            path,
            whisper_rs::WhisperContextParameters::default(),
        )
        .map_err(|e| format!("can't load the voice model: {e}"))?;
        Ok(Self { ctx })
    }

    /// Blocking: seconds of CPU on a long recording.
    pub fn transcribe(&self, pcm: &[f32], language: Language) -> Result<String, String> {
        // Under half a second there is nothing to hear, and whisper invents words on silence.
        if pcm.len() < SAMPLE_RATE as usize / 2 {
            return Ok(String::new());
        }
        let mut state = self.ctx.create_state().map_err(|e| e.to_string())?;
        let spoken = match language.settled() {
            Some(spoken) => spoken,
            None => detect(&mut state, pcm)?,
        };
        let mut params = whisper_rs::FullParams::new(whisper_rs::SamplingStrategy::Greedy { best_of: 1 });
        params.set_language(Some(spoken.code()));
        params.set_n_threads(threads());
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);
        params.set_no_context(true);
        params.set_suppress_blank(true);
        state
            .full(params, pcm)
            .map_err(|e| format!("transcription failed: {e}"))?;
        let text: Vec<String> = state
            .as_iter()
            .map(|s| s.to_string().trim().to_string())
            .collect();
        Ok(clean(&text.join(" ")))
    }
}

/// Portuguese or English, whichever the start of the recording sounds more like.
fn detect(state: &mut whisper_rs::WhisperState, pcm: &[f32]) -> Result<Spoken, String> {
    let threads = threads() as usize;
    state
        .pcm_to_mel(pcm, threads)
        .map_err(|e| format!("can't read the recording: {e}"))?;
    let (_, probs) = state
        .lang_detect(0, threads)
        .map_err(|e| format!("can't tell the language: {e}"))?;
    let prob = |code: &str| {
        whisper_rs::get_lang_id(code)
            .and_then(|id| probs.get(id as usize).copied())
            .unwrap_or(0.0)
    };
    Ok(Spoken::likelier(prob("pt"), prob("en")))
}

fn threads() -> i32 {
    let n = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4);
    // Half the cores: the agents and the desktop keep running meanwhile.
    (n / 2).clamp(1, 8) as i32
}

/// Whisper marks sounds it heard but did not understand: `[BLANK_AUDIO]`, `(music)`. Those are
/// not words the user said.
fn clean(text: &str) -> String {
    let mut out = String::new();
    let mut depth = 0u32;
    for c in text.chars() {
        match c {
            '[' | '(' => depth += 1,
            ']' | ')' => depth = depth.saturating_sub(1),
            _ if depth == 0 => out.push(c),
            _ => {}
        }
    }
    let mut text = out.split_whitespace().collect::<Vec<_>>().join(" ");
    for p in [".", ",", "?", "!"] {
        text = text.replace(&format!(" {p}"), p);
    }
    text
}

/// Linear resampling and down-mixing to what whisper takes. Speech needs nothing finer.
pub fn to_whisper(samples: &[f32], channels: u16, rate: u32) -> Vec<f32> {
    let channels = usize::from(channels.max(1));
    let mono: Vec<f32> = samples
        .chunks(channels)
        .map(|frame| frame.iter().sum::<f32>() / frame.len() as f32)
        .collect();
    if rate == SAMPLE_RATE || mono.is_empty() {
        return mono;
    }
    let step = f64::from(rate) / f64::from(SAMPLE_RATE);
    let len = (mono.len() as f64 / step) as usize;
    (0..len)
        .map(|i| {
            let at = i as f64 * step;
            let j = at as usize;
            let frac = (at - j as f64) as f32;
            let a = mono[j];
            let b = mono.get(j + 1).copied().unwrap_or(a);
            a + (b - a) * frac
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audio_becomes_16_khz_mono() {
        // One second of stereo at 48 kHz.
        let stereo: Vec<f32> = (0..96_000).map(|i| if i % 2 == 0 { 0.5 } else { -0.5 }).collect();
        let out = to_whisper(&stereo, 2, 48_000);
        assert_eq!(out.len(), 16_000);
        assert!(out.iter().all(|s| s.abs() < 1e-6), "left and right cancel out");
        assert_eq!(to_whisper(&[0.1, 0.2], 1, 16_000), vec![0.1, 0.2]);
    }

    #[test]
    fn sounds_whisper_could_not_read_are_dropped() {
        assert_eq!(clean(" [BLANK_AUDIO] "), "");
        assert_eq!(clean("Hello (music) there [laughs]."), "Hello there.");
        assert_eq!(clean("Why is the build failing?"), "Why is the build failing?");
    }
}
