//! Local speech with an explicitly downloaded whisper.cpp model. Chat and dictation keep audio
//! in memory; the meeting API reads already-open WAVs owned by its caller. No audio is sent anywhere.

mod gate;
pub mod meeting;
mod model_file;
mod models;
#[cfg(any(feature = "diarization", feature = "semantic-turn"))]
pub mod onnx;
mod preview;
mod record;
mod scheduler;
pub mod turn;
mod vad;
pub mod vocabulary;

use std::mem::ManuallyDrop;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

pub use models::{MODELS, Model, VAD, download, installed, model, model_path, vad_path};
pub use preview::Preview;
pub use record::{AutoStop, Recorder};
use vad::{EndOfSpeech, Vad};

/// What whisper takes: 16 kHz, mono, f32.
pub use vults_audio::SAMPLE_RATE;

/// Whether Silero heard speech, for a caller deciding whether an empty transcript needs a retry.
pub fn has_speech(pcm: &[f32], model: &Path) -> Result<bool, String> {
    Vad::load(model)?.speech(pcm).map(|speech| !speech.is_empty())
}

/// A final decode's text and the number of repetitive runs collapsed by its filter.
#[derive(Debug, Default)]
pub struct Transcript {
    pub text: String,
    pub loops_cut: u64,
}

/// A loaded model, kept between recordings: loading takes longer than a short transcription.
pub struct Transcriber {
    /// Freed under [`WHISPER`] too (see `Drop`).
    ctx: ManuallyDrop<whisper_rs::WhisperContext>,
    /// Give whisper the coding vocabulary (see [`Model::prompt`]).
    prompt: bool,
}

/// Held while a model (or the VAD) loads, while a model decodes or is freed, and while the GPUs are
/// listed. Interactive work interrupts meeting decoding at its next abort check and runs first.
/// Optional CPU ONNX analysis shares this slot to keep meetings behind interactive speech.
/// Two contexts at work at once on Vulkan crash ggml (seen in this crate's tests), including
/// two models after a Settings change during a recording.
static WHISPER: scheduler::Scheduler = scheduler::Scheduler::new();

impl Drop for Transcriber {
    fn drop(&mut self) {
        let _busy = WHISPER.lock();
        // SAFETY: dropped once, here, and never used after.
        unsafe { ManuallyDrop::drop(&mut self.ctx) };
    }
}

impl std::fmt::Debug for Transcriber {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Transcriber")
    }
}

impl Transcriber {
    pub fn load(model: &Path, prompt: bool) -> Result<Self, String> {
        let _busy = WHISPER.lock();
        Ok(Self {
            ctx: ManuallyDrop::new(Self::context(model)?),
            prompt,
        })
    }

    /// Cold meeting loads wait behind interactive work. A native load already in progress
    /// cannot be aborted; cancellation discards its context before returning.
    pub fn load_background(model: &Path, stop: &AtomicBool) -> Result<Option<Self>, String> {
        let Some(_busy) = WHISPER.background(stop) else {
            return Ok(None);
        };
        let ctx = Self::context(model)?;
        if stop.load(Ordering::SeqCst) {
            return Ok(None);
        }
        Ok(Some(Self {
            ctx: ManuallyDrop::new(ctx),
            prompt: false,
        }))
    }

    fn context(model: &Path) -> Result<whisper_rs::WhisperContext, String> {
        // whisper.cpp logs every layer it loads to stderr; through `tracing` it stays quiet.
        whisper_rs::install_logging_hooks();
        let path = model.to_str().ok_or("the model path is not UTF-8")?;
        whisper_rs::WhisperContext::new_with_params(path, whisper_rs::WhisperContextParameters::default())
            .map_err(|e| format!("can't load the voice model: {e}"))
    }

    /// Blocking: seconds of CPU on a long recording. `language` is a code (`pt`, `en`) or None to
    /// detect it. `vad` is the Silero model, when it is on disk.
    pub fn transcribe(
        &self,
        pcm: &[f32],
        language: Option<&str>,
        vad: Option<&Path>,
    ) -> Result<String, String> {
        // Whisper invents words on silence (*Thank you.*, *Obrigado.*): only the speech goes in, and
        // a recording with none gives no text.
        let pcm = speech_only(pcm, vad);
        Ok(self
            .decode(pcm, language, None, true, None)?
            .unwrap_or_default()
            .text)
    }

    /// Final transcription with token-level cancellation. `cut_repeats` affects loops only;
    /// silence artifacts and sound annotations are always filtered. Keep this per call because
    /// the cached model also serves Chat.
    pub fn transcribe_cancellable(
        &self,
        pcm: &[f32],
        language: Option<&str>,
        vad: Option<&Path>,
        stop: &AtomicBool,
        cut_repeats: bool,
    ) -> Result<Option<String>, String> {
        if stop.load(Ordering::SeqCst) {
            return Ok(None);
        }
        self.decode(speech_only(pcm, vad), language, Some(stop), cut_repeats, None)
            .map(|text| text.map(|text| text.text))
    }

    /// Per-call local vocabulary; callers gate measured model/language pairs and user choice.
    /// This does not update the cached model or apply any post-transcription replacements.
    pub fn transcribe_with_vocabulary(
        &self,
        pcm: &[f32],
        language: Option<&str>,
        vad: Option<&Path>,
        stop: &AtomicBool,
        cut_repeats: bool,
        words: &[String],
    ) -> Result<Option<String>, String> {
        self.transcribe_measured(pcm, language, vad, stop, cut_repeats, words)
            .map(|text| text.map(|text| text.text))
    }

    /// Final-only measurements: preview passes never contribute to diagnostic counters.
    pub fn transcribe_measured(
        &self,
        pcm: &[f32],
        language: Option<&str>,
        vad: Option<&Path>,
        stop: &AtomicBool,
        cut_repeats: bool,
        words: &[String],
    ) -> Result<Option<Transcript>, String> {
        if stop.load(Ordering::SeqCst) {
            return Ok(None);
        }
        self.decode(
            speech_only(pcm, vad),
            language,
            Some(stop),
            cut_repeats,
            Some(words),
        )
    }

    /// The live preview while the user still speaks: the whole recording so far, trimmed by the
    /// loudness gate (Silero over all of it on every pass would keep a core busy). Setting `stop`
    /// aborts it between tokens; then it gives None.
    pub(crate) fn preview(
        &self,
        pcm: &[f32],
        language: Option<&str>,
        stop: &AtomicBool,
        cut_repeats: bool,
        words: &[String],
    ) -> Result<Option<String>, String> {
        self.decode(trim_silence(pcm), language, Some(stop), cut_repeats, Some(words))
            .map(|text| text.map(|text| text.text))
    }

    fn decode(
        &self,
        pcm: &[f32],
        language: Option<&str>,
        stop: Option<&AtomicBool>,
        cut_repeats: bool,
        words: Option<&[String]>,
    ) -> Result<Option<Transcript>, String> {
        let stopped = || stop.is_some_and(|s| s.load(Ordering::SeqCst));
        if pcm.len() < SAMPLE_RATE as usize / 2 {
            return Ok(Some(Transcript::default()));
        }
        let _busy = WHISPER.lock();
        if stopped() {
            return Ok(None);
        }
        let mut state = self.ctx.create_state().map_err(|e| e.to_string())?;
        let mut params = whisper_rs::FullParams::new(whisper_rs::SamplingStrategy::Greedy { best_of: 1 });
        params.set_language(Some(language.unwrap_or("auto")));
        // The words a coding chat uses, so they come out as written, not as they sound.
        let seed = language.filter(|_| self.prompt).and_then(vocabulary);
        let prompt = if let Some(words) = words.filter(|words| !words.is_empty()) {
            vocabulary::build(language.and_then(vocabulary), words, |text| {
                // Byte-level BPE needs no more tokens than input bytes plus its leading space.
                self.ctx
                    .tokenize(text, text.len() + 16)
                    .map_err(|e| e.to_string())
            })?
        } else {
            None
        };
        if let Some(prompt) = &prompt {
            params.set_tokens(prompt);
        } else if let Some(seed) = seed {
            params.set_initial_prompt(seed);
        }
        params.set_no_speech_thold(NO_SPEECH);
        params.set_n_threads(threads());
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);
        params.set_no_context(true);
        params.set_suppress_blank(true);
        if let Some(stop) = stop {
            // whisper-rs' safe setter hands whisper.cpp a pointer of another type than the one its
            // trampoline reads, so a plain function over the flag instead.
            unsafe extern "C" fn aborted(flag: *mut std::ffi::c_void) -> bool {
                // SAFETY: the `&AtomicBool` set below, borrowed until `full` returns.
                unsafe { &*flag.cast::<AtomicBool>() }.load(Ordering::SeqCst)
            }
            // SAFETY: whisper.cpp calls it only inside `full`, while `stop` is borrowed here.
            unsafe {
                params.set_abort_callback(Some(aborted));
                params.set_abort_callback_user_data(std::ptr::from_ref(stop).cast_mut().cast());
            }
        }
        let result = state.full(params, pcm);
        if stopped() {
            return Ok(None);
        }
        result.map_err(|e| format!("transcription failed: {e}"))?;
        let text: Vec<String> = state
            .as_iter()
            .filter(|s| s.no_speech_probability() < NO_SPEECH)
            .map(|s| s.to_string().trim().to_string())
            .collect();
        Ok(Some(gate::measured(&text.join(" "), cut_repeats)))
    }
}

/// Whether whisper runs on a GPU here. Only then is decoding the recording again while the user
/// speaks cheap enough: on the CPU a pass of Base takes about 0.6 s on 8 threads, however short.
pub fn gpu() -> bool {
    #[cfg(feature = "vulkan")]
    {
        static GPU: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
        // Listing the devices starts ggml's Vulkan state, which guards itself with no lock.
        *GPU.get_or_init(|| {
            let _busy = WHISPER.lock();
            !whisper_rs::vulkan::list_devices().is_empty()
        })
    }
    #[cfg(not(feature = "vulkan"))]
    false
}

/// Whisper's own guess that a stretch held no speech; above it the stretch is dropped.
const NO_SPEECH: f32 = 0.6;

/// A prompt for whisper in the language spoken, so it has to be in Portuguese.
const VOCABULARY_PT: &str = "Conversa com um assistente de programação sobre o código: commit, branch, merge, pull request, deploy, build, testes, terminal, Rust, TypeScript, Claude Code, Codex."; // check-english:allow

fn vocabulary(language: &str) -> Option<&'static str> {
    match language {
        "pt" => Some(VOCABULARY_PT),
        "en" => Some(
            "A chat with a coding assistant about the code: commit, branch, merge, pull request, deploy, \
             build, tests, terminal, Rust, TypeScript, Claude Code, Codex.",
        ),
        _ => None,
    }
}

/// The recording from its first speech to its last: the pause before the key is let go is where
/// whisper makes words up. Silero tells speech from noise; without its model (or if it fails), a
/// loudness gate does what it can.
fn speech_only<'a>(pcm: &'a [f32], vad: Option<&Path>) -> &'a [f32] {
    let Some(model) = vad else {
        return trim_silence(pcm);
    };
    match Vad::load(model).and_then(|mut v| v.speech(pcm)) {
        Ok(speech) => vad::around(pcm, &speech),
        Err(e) => {
            tracing::warn!("voice: {e}");
            trim_silence(pcm)
        }
    }
}

/// The loudness gate: from the first sound to the last, with a little margin. Nothing when
/// nothing stands out.
fn trim_silence(pcm: &[f32]) -> &[f32] {
    const FRAME: usize = SAMPLE_RATE as usize / 50; // 20 ms
    const MARGIN: usize = SAMPLE_RATE as usize / 4; // 250 ms
    let rms: Vec<f32> = pcm
        .chunks(FRAME)
        .map(|f| (f.iter().map(|s| s * s).sum::<f32>() / f.len() as f32).sqrt())
        .collect();
    // Relative to this recording: a quiet mic still speaks well above its own room noise, and a
    // voice reaches a fair share of its loudest moment.
    let mut sorted = rms.clone();
    sorted.sort_by(f32::total_cmp);
    let floor = sorted.get(sorted.len() / 5).copied().unwrap_or(0.0);
    let peak = sorted.last().copied().unwrap_or(0.0);
    let speech = (floor * 4.0).max(peak * 0.1).max(0.002);
    let (Some(first), Some(last)) = (
        rms.iter().position(|&r| r > speech),
        rms.iter().rposition(|&r| r > speech),
    ) else {
        return &pcm[..0];
    };
    let start = (first * FRAME).saturating_sub(MARGIN);
    let end = ((last + 1) * FRAME + MARGIN).min(pcm.len());
    &pcm[start..end]
}

fn threads() -> i32 {
    let n = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4);
    // Half the cores: the agents and the desktop keep running meanwhile.
    (n / 2).clamp(1, 8) as i32
}

/// Compatibility helper; capture itself propagates resampling failures to its caller.
pub fn to_whisper(samples: &[f32], channels: u16, rate: u32) -> Vec<f32> {
    vults_audio::resample(samples, channels, rate).unwrap_or_else(|error| {
        tracing::warn!(%error, "audio resampling failed");
        Vec::new()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_cancelled_background_load_never_opens_the_model() {
        let stop = AtomicBool::new(true);
        assert!(
            Transcriber::load_background(Path::new("missing-unopened-meeting-model.bin"), &stop)
                .expect("cancelled before model admission")
                .is_none()
        );
    }

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
    fn only_the_speech_is_kept() {
        let rate = SAMPLE_RATE as usize;
        let tone = |n: usize| {
            (0..n)
                .map(|i| (i as f32 * 0.05).sin() * 0.2)
                .collect::<Vec<f32>>()
        };
        // 2 s of silence, 1 s of voice, 2 s of silence.
        let mut pcm = vec![0.0; 2 * rate];
        pcm.extend(tone(rate));
        pcm.extend(vec![0.0; 2 * rate]);
        let kept = trim_silence(&pcm);
        assert!(
            kept.len() >= rate && kept.len() <= rate + rate / 2 + rate / 50,
            "{}",
            kept.len()
        );
        assert!(
            trim_silence(&vec![0.001; 3 * rate]).is_empty(),
            "a quiet room is no speech"
        );
        // A quiet mic: the voice is soft, but far above its own noise.
        let mut soft = vec![0.0005; 2 * rate];
        soft.extend(tone(rate).iter().map(|s| s * 0.1));
        assert!(trim_silence(&soft).len() >= rate);
        assert!(vocabulary("pt").is_some() && vocabulary("ja").is_none());
    }
}
