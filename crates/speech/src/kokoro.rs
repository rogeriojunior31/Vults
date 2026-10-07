//! Kokoro-82M (Apache-2.0) through ONNX Runtime, on the CPU: phonemes and a voice in, 24 kHz audio
//! out. One stateless call per sentence.

use std::path::Path;
use std::sync::{Arc, Mutex};

use ort::session::{RunOptions, Session};
use ort::value::Tensor;

/// What Kokoro gives: mono, f32.
pub const SAMPLE_RATE: u32 = 24_000;
/// Phonemes it takes at once; the sentence cutter keeps well under it.
const MAX_TOKENS: usize = 510;
/// A voice file: one 256-wide style per phoneme count.
const STYLE: usize = 256;

pub struct Kokoro {
    session: Session,
    /// The run under way, so [`Kokoro::canceller`] can end it from another thread.
    current: Arc<Mutex<Option<Arc<RunOptions>>>>,
}

impl std::fmt::Debug for Kokoro {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Kokoro")
    }
}

/// Linux: loads ONNX Runtime from the library downloaded with the model, once per process.
#[cfg(target_os = "linux")]
pub fn runtime(lib: &Path) -> Result<(), String> {
    static LOADED: std::sync::OnceLock<Result<(), String>> = std::sync::OnceLock::new();
    LOADED
        .get_or_init(|| {
            ort::init_from(lib)
                .map_err(|e| format!("can't load ONNX Runtime: {e}"))?
                .commit();
            Ok(())
        })
        .clone()
}

impl Kokoro {
    /// 0.6 s from a warm disk, up to 12 s from a cold one: load it when speech is turned on.
    pub fn load(model: &Path, threads: usize) -> Result<Self, String> {
        let error = |e: &dyn std::fmt::Display| format!("can't load the speech model: {e}");
        let session = Session::builder()
            .map_err(|e| error(&e))?
            .with_intra_threads(threads)
            .map_err(|e| error(&e))?
            .commit_from_file(model)
            .map_err(|e| error(&e))?;
        Ok(Self {
            session,
            current: Arc::default(),
        })
    }

    /// Ends the run under way, if one is; the next one starts afresh.
    pub fn canceller(&self) -> impl Fn() + Send + Sync + 'static {
        let current = self.current.clone();
        move || {
            if let Some(run) = current.lock().unwrap_or_else(|e| e.into_inner()).as_ref() {
                let _ = run.terminate();
            }
        }
    }

    /// Audio for `phonemes` in `voice` (a voice file's floats). `stale` is asked once the run can
    /// be cancelled: a stop that came just before it ends it there.
    pub fn speak(
        &mut self,
        phonemes: &str,
        voice: &[f32],
        stale: &dyn Fn() -> bool,
    ) -> Result<Vec<f32>, String> {
        let mut ids = vec![0i64];
        ids.extend(tokens(phonemes));
        ids.push(0);
        let n = ids.len() - 2;
        if n == 0 {
            return Ok(Vec::new());
        }
        let row = n.min((voice.len() / STYLE).saturating_sub(1)) * STYLE;
        let style = voice
            .get(row..row + STYLE)
            .ok_or("the voice file is too short")?
            .to_vec();
        let run = Arc::new(RunOptions::new().map_err(|e| e.to_string())?);
        *self.current.lock().unwrap_or_else(|e| e.into_inner()) = Some(run.clone());
        if stale() {
            return Err("stopped".into());
        }
        let len = ids.len();
        let tensor = |e: ort::Error| e.to_string();
        let inputs = ort::inputs! {
            "input_ids" => Tensor::from_array(([1usize, len], ids)).map_err(tensor)?,
            "style" => Tensor::from_array(([1usize, STYLE], style)).map_err(tensor)?,
            "speed" => Tensor::from_array(([1usize], vec![1.0f32])).map_err(tensor)?,
        };
        let out = self.session.run_with_options(inputs, &*run);
        *self.current.lock().unwrap_or_else(|e| e.into_inner()) = None;
        let out = out.map_err(|e| format!("speech failed: {e}"))?;
        let (_, audio) = out[0].try_extract_tensor::<f32>().map_err(|e| e.to_string())?;
        Ok(audio.to_vec())
    }
}

/// A voice file's floats (little-endian f32).
pub fn voice(path: &Path) -> Result<Vec<f32>, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("can't read the voice: {e}"))?;
    if bytes.len() < STYLE * 4 * 2 {
        return Err("the voice file is too short".into());
    }
    let (floats, _) = bytes.as_chunks::<4>();
    Ok(floats.iter().map(|c| f32::from_le_bytes(*c)).collect())
}

/// Kokoro's ids for the phonemes it knows; the rest is dropped, and so is anything past its limit.
fn tokens(phonemes: &str) -> impl Iterator<Item = i64> + '_ {
    phonemes
        .chars()
        .filter_map(|c| VOCAB.iter().find(|(v, _)| *v == c).map(|(_, id)| *id))
        .take(MAX_TOKENS)
}

/// Kokoro's vocabulary, from `tokenizer.json` of onnx-community/Kokoro-82M-v1.0-ONNX (Apache-2.0)
/// at the revision the models download from.
#[rustfmt::skip]
const VOCAB: [(char, i64); 115] = [
    ('$', 0), (';', 1), (':', 2), (',', 3), ('.', 4), ('!', 5), ('?', 6), ('\u{2014}', 9),
    ('\u{2026}', 10), ('"', 11), ('(', 12), (')', 13), ('\u{201c}', 14), ('\u{201d}', 15),
    (' ', 16), ('\u{303}', 17), ('\u{2a3}', 18), ('\u{2a5}', 19), ('\u{2a6}', 20), ('\u{2a8}', 21),
    ('\u{1d5d}', 22), ('\u{ab67}', 23), ('A', 24), ('I', 25), ('O', 31), ('Q', 33), ('S', 35),
    ('T', 36), ('W', 39), ('Y', 41), ('\u{1d4a}', 42), ('a', 43), ('b', 44), ('c', 45), ('d', 46),
    ('e', 47), ('f', 48), ('h', 50), ('i', 51), ('j', 52), ('k', 53), ('l', 54), ('m', 55),
    ('n', 56), ('o', 57), ('p', 58), ('q', 59), ('r', 60), ('s', 61), ('t', 62), ('u', 63),
    ('v', 64), ('w', 65), ('x', 66), ('y', 67), ('z', 68), ('\u{251}', 69), ('\u{250}', 70),
    ('\u{252}', 71), ('\u{e6}', 72), ('\u{3b2}', 75), ('\u{254}', 76), ('\u{255}', 77),
    ('\u{e7}', 78), ('\u{256}', 80), ('\u{f0}', 81), ('\u{2a4}', 82), ('\u{259}', 83),
    ('\u{25a}', 85), ('\u{25b}', 86), ('\u{25c}', 87), ('\u{25f}', 90), ('\u{261}', 92),
    ('\u{265}', 99), ('\u{268}', 101), ('\u{26a}', 102), ('\u{29d}', 103), ('\u{26f}', 110),
    ('\u{270}', 111), ('\u{14b}', 112), ('\u{273}', 113), ('\u{272}', 114), ('\u{274}', 115),
    ('\u{f8}', 116), ('\u{278}', 118), ('\u{3b8}', 119), ('\u{153}', 120), ('\u{279}', 123),
    ('\u{27e}', 125), ('\u{27b}', 126), ('\u{281}', 128), ('\u{27d}', 129), ('\u{282}', 130),
    ('\u{283}', 131), ('\u{288}', 132), ('\u{2a7}', 133), ('\u{28a}', 135), ('\u{28b}', 136),
    ('\u{28c}', 138), ('\u{263}', 139), ('\u{264}', 140), ('\u{3c7}', 142), ('\u{28e}', 143),
    ('\u{292}', 147), ('\u{294}', 148), ('\u{2c8}', 156), ('\u{2cc}', 157), ('\u{2d0}', 158),
    ('\u{2b0}', 162), ('\u{2b2}', 164), ('\u{2193}', 169), ('\u{2192}', 171), ('\u{2197}', 172),
    ('\u{2198}', 173), ('\u{1d7b}', 177),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phonemes_become_kokoros_ids() {
        assert_eq!(
            tokens("h\u{259}l\u{2c8}O.").collect::<Vec<_>>(),
            [50, 83, 54, 156, 31, 4]
        );
        // Unknown symbols (a zero-width joiner, an emoji) are dropped.
        assert_eq!(tokens("a\u{200d}\u{1f600}").collect::<Vec<_>>(), [43]);
        assert_eq!(tokens(&"a".repeat(600)).count(), MAX_TOKENS);
    }

    #[test]
    fn a_cut_voice_file_is_refused() {
        let path = std::env::temp_dir().join(format!("speech-voice-{}.bin", std::process::id()));
        std::fs::write(&path, [0u8; 16]).expect("write");
        assert!(voice(&path).is_err());
        std::fs::write(&path, vec![0u8; 510 * 256 * 4]).expect("write");
        assert_eq!(voice(&path).map(|v| v.len()), Ok(510 * 256));
        let _ = std::fs::remove_file(&path);
    }
}
