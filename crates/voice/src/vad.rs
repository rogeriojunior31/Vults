//! Silero VAD through whisper.cpp: where the speech is in a recording, and when the user has
//! finished talking.

use std::ops::Range;
use std::path::Path;

use whisper_rs::{WhisperVadContext, WhisperVadContextParams, WhisperVadParams};

use crate::SAMPLE_RATE;

/// Tap-to-talk stops after this much silence following speech. Dictation breathes between phrases
/// for up to about a second (jfk.wav pauses 0.95 s twice); 1.5 s is past any of those and still
/// soon enough to feel like it stopped when the user did. It must fit in the recorder's 2 s window.
const PAUSE: usize = SAMPLE_RATE as usize * 3 / 2; // 1.5 s
/// Kept around the speech: the VAD's edges are tight, and a clipped first syllable is misheard.
const MARGIN: usize = SAMPLE_RATE as usize / 5; // 200 ms
/// Shorter than one Silero window (32 ms) is nothing to look at.
const SHORTEST: usize = 512;

#[derive(Debug)]
pub struct Vad(WhisperVadContext);

impl Vad {
    /// Quick (the model is under 1 MB), so it is loaded for each recording.
    pub fn load(model: &Path) -> Result<Self, String> {
        whisper_rs::install_logging_hooks();
        let path = model.to_str().ok_or("the VAD model path is not UTF-8")?;
        let mut params = WhisperVadContextParams::default();
        // Small work on short audio; the transcription wants the cores.
        params.set_n_threads(1);
        WhisperVadContext::new(path, params)
            .map(Self)
            .map_err(|e| format!("can't load the VAD model: {e}"))
    }

    /// The stretches of speech in 16 kHz mono audio, as sample ranges, in order.
    pub fn speech(&mut self, pcm: &[f32]) -> Result<Vec<Range<usize>>, String> {
        if pcm.len() < SHORTEST {
            return Ok(Vec::new());
        }
        let segments = self
            .0
            .segments_from_samples(WhisperVadParams::default(), pcm)
            .map_err(|e| format!("the VAD failed: {e}"))?;
        // whisper.cpp gives centiseconds.
        let at = |cs: f32| ((cs.max(0.0) * SAMPLE_RATE as f32 / 100.0) as usize).min(pcm.len());
        Ok(segments
            .map(|s| at(s.start)..at(s.end))
            .filter(|r| !r.is_empty())
            .collect())
    }
}

/// The recording from its first speech to its last, with a margin; nothing when there is none.
pub(crate) fn around<'a>(pcm: &'a [f32], speech: &[Range<usize>]) -> &'a [f32] {
    let (Some(first), Some(last)) = (speech.first(), speech.last()) else {
        return &pcm[..0];
    };
    let start = first.start.saturating_sub(MARGIN);
    let end = (last.end + MARGIN).min(pcm.len());
    pcm.get(start..end).unwrap_or(&pcm[..0])
}

/// Tap-to-talk: decides, from the VAD over the last seconds of a recording, that the user has
/// finished. Silence before any speech never ends it: the user may still be finding the words.
#[derive(Debug, Default)]
pub struct EndOfSpeech {
    heard: bool,
}

impl EndOfSpeech {
    /// `speech` is what the VAD found in the last `window` samples. True once speech was heard and
    /// the window ends in [`PAUSE`] of silence.
    pub fn update(&mut self, speech: &[Range<usize>], window: usize) -> bool {
        match speech.last() {
            Some(last) => {
                self.heard = true;
                window.saturating_sub(last.end) >= PAUSE
            }
            None => self.heard && window >= PAUSE,
        }
    }
}

#[cfg(test)]
// One stretch of speech is a one-range list, not a list of samples.
#[allow(clippy::single_range_in_vec_init)]
mod tests {
    use super::*;

    const S: usize = SAMPLE_RATE as usize;

    #[test]
    fn the_speech_and_a_margin_are_kept() {
        let pcm = vec![0.0; 5 * S];
        assert_eq!(around(&pcm, &[2 * S..3 * S]).len(), S + 2 * MARGIN);
        assert_eq!(around(&pcm, &[S..2 * S, 3 * S..4 * S]).len(), 3 * S + 2 * MARGIN);
        // At the very edges the margin is cut, not run past the recording.
        assert_eq!(around(&pcm, &[0..5 * S]).len(), 5 * S);
        assert!(around(&pcm, &[]).is_empty(), "no speech, no audio");
    }

    #[test]
    fn a_pause_after_speech_ends_the_turn() {
        let window = 2 * S;
        let mut end = EndOfSpeech::default();
        // Silence before speaking: keep listening, however long.
        assert!(!end.update(&[], window));
        assert!(!end.update(&[], window));
        // Talking up to the end of the window.
        assert!(!end.update(&[S / 2..window], window));
        // A short breath between words.
        assert!(!end.update(&[0..window - S / 5], window));
        // A pause of a second between phrases.
        assert!(!end.update(&[0..window - S], window));
        // 1.5 s of quiet after the last word.
        assert!(end.update(&[0..window - PAUSE], window));
    }

    #[test]
    fn speech_out_of_the_window_still_counts() {
        let mut end = EndOfSpeech::default();
        assert!(!end.update(&[S / 2..2 * S], 2 * S));
        // The words have slid out of the last two seconds: only silence is left.
        assert!(end.update(&[], 2 * S));
        // A recording still shorter than the pause is not over.
        let mut short = EndOfSpeech::default();
        assert!(!short.update(&[0..S / 10], S / 2));
    }

    /// The real model, from `VOICE_TEST_VAD` (a path to ggml-silero-v6.2.0.bin); skipped without.
    fn real_vad() -> Option<Vad> {
        let path = std::env::var_os("VOICE_TEST_VAD")?;
        Some(Vad::load(Path::new(&path)).expect("the VAD model loads"))
    }

    /// whisper.cpp's samples/jfk.wav (16 kHz mono 16-bit), from `VOICE_TEST_WAV`.
    fn jfk() -> Option<Vec<f32>> {
        let bytes = std::fs::read(std::env::var_os("VOICE_TEST_WAV")?).ok()?;
        let at = bytes.windows(4).position(|w| w == b"data")? + 8;
        Some(
            bytes[at..]
                .as_chunks::<2>()
                .0
                .iter()
                .map(|b| f32::from(i16::from_le_bytes(*b)) / f32::from(i16::MAX))
                .collect(),
        )
    }

    #[test]
    fn silero_hears_no_speech_in_silence_or_noise() {
        let Some(mut vad) = real_vad() else { return };
        assert!(vad.speech(&vec![0.0; 3 * S]).expect("vad").is_empty());
        // A cheap, fixed white noise.
        let mut x = 1u32;
        let noise: Vec<f32> = (0..3 * S)
            .map(|_| {
                x ^= x << 13;
                x ^= x >> 17;
                x ^= x << 5;
                (x as f32 / u32::MAX as f32 - 0.5) * 0.1
            })
            .collect();
        assert!(vad.speech(&noise).expect("vad").is_empty(), "noise is not speech");
    }

    #[test]
    fn silero_finds_the_speech_and_the_end_of_it() {
        let (Some(mut vad), Some(words)) = (real_vad(), jfk()) else {
            return;
        };
        // A second of silence before, two after: room for the pause that ends the turn.
        let mut pcm = vec![0.0; S];
        pcm.extend(&words);
        pcm.extend(vec![0.0; 2 * S]);
        let speech = vad.speech(&pcm).expect("vad");
        let kept = around(&pcm, &speech);
        assert!(
            kept.len() < pcm.len() - S,
            "the silence is trimmed: {}",
            kept.len()
        );
        assert!(
            kept.len() > words.len() * 3 / 4,
            "the speech is kept: {}",
            kept.len()
        );

        // Tap-to-talk as the recorder runs it: the last 2 s, every 200 ms.
        let stop_at = |vad: &mut Vad, pcm: &[f32]| {
            let mut end = EndOfSpeech::default();
            (1..)
                .map(|tick| (tick * S / 5).min(pcm.len()))
                .find(|&now| {
                    let window = &pcm[now.saturating_sub(2 * S)..now];
                    end.update(&vad.speech(window).expect("vad"), window.len()) || now == pcm.len()
                })
                .expect("a tick")
        };
        // The speech pauses for almost a second after "my fellow Americans", and again later: a
        // breath between phrases, which must not end the turn. It ends after the last word.
        assert!(
            speech.len() >= 2 && speech.windows(2).any(|w| w[1].start - w[0].end > S * 8 / 10),
            "{speech:?}"
        );
        let last_word = speech.last().expect("speech").end;
        let stopped = stop_at(&mut vad, &pcm);
        assert!(
            last_word < stopped && stopped < pcm.len(),
            "after the last word: {stopped}, {speech:?}"
        );
        // Its last sentence alone: the same, within the trailing two seconds.
        let mut last = vec![0.0; S];
        last.extend(&words[8 * S..]);
        last.extend(vec![0.0; 2 * S]);
        let last_word = vad.speech(&last).expect("vad").last().expect("speech").end;
        let stopped = stop_at(&mut vad, &last);
        assert!(
            last_word < stopped && stopped < last.len(),
            "{stopped} after {last_word}"
        );
    }

    /// End to end with a whisper model from `VOICE_TEST_MODEL`: silence around the words adds none.
    #[test]
    fn whisper_hears_only_the_words() {
        let (Some(model), Some(vad), Some(words)) = (
            std::env::var_os("VOICE_TEST_MODEL"),
            std::env::var_os("VOICE_TEST_VAD"),
            jfk(),
        ) else {
            return;
        };
        let mut pcm = vec![0.0; 3 * S];
        pcm.extend(&words);
        pcm.extend(vec![0.0; 3 * S]);
        let t = crate::Transcriber::load(Path::new(&model), false).expect("model");
        let text = t
            .transcribe(&pcm, Some("en"), Some(Path::new(&vad)))
            .expect("text");
        assert!(text.contains("ask what you can do for your country"), "{text}");
    }
}
