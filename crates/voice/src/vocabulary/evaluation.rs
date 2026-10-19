//! Opt-in, offline WER comparison. Never records a microphone or downloads a model.

use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

use serde::Deserialize;

#[derive(Deserialize)]
struct Clip {
    id: String,
    language: String,
    reference: String,
    words: Vec<String>,
}

fn normalized(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|s| !s.is_empty())
        .map(str::to_lowercase)
        .collect()
}

fn distance(reference: &[String], heard: &[String]) -> usize {
    let mut row: Vec<usize> = (0..=heard.len()).collect();
    for (i, expected) in reference.iter().enumerate() {
        let mut diagonal = row[0];
        row[0] = i + 1;
        for (j, actual) in heard.iter().enumerate() {
            let above = row[j + 1];
            row[j + 1] = (row[j] + 1)
                .min(above + 1)
                .min(diagonal + usize::from(expected != actual));
            diagonal = above;
        }
    }
    row[heard.len()]
}

fn audio(path: &Path) -> Vec<f32> {
    let bytes = std::fs::read(path).expect("local WAV fixture");
    assert_eq!(&bytes[..4], b"RIFF");
    assert_eq!(&bytes[8..12], b"WAVE");
    let mut at = 12;
    let mut format = false;
    while at + 8 <= bytes.len() {
        let size = u32::from_le_bytes(bytes[at + 4..at + 8].try_into().unwrap()) as usize;
        let chunk = &bytes[at + 8..at + 8 + size];
        match &bytes[at..at + 4] {
            b"fmt " => {
                assert_eq!(u16::from_le_bytes(chunk[..2].try_into().unwrap()), 1);
                assert_eq!(u16::from_le_bytes(chunk[2..4].try_into().unwrap()), 1);
                assert_eq!(u32::from_le_bytes(chunk[4..8].try_into().unwrap()), 16_000);
                assert_eq!(u16::from_le_bytes(chunk[14..16].try_into().unwrap()), 16);
                format = true;
            }
            b"data" => {
                assert!(format);
                return chunk
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .map(|b| f32::from(i16::from_le_bytes([b[0], b[1]])) / 32768.0)
                    .collect();
            }
            _ => {}
        }
        at += 8 + size + size % 2;
    }
    panic!("WAV data chunk missing");
}

fn synthesize(dir: &Path, clip: &Clip) -> PathBuf {
    use std::io::Write;
    use std::process::{Command, Stdio};
    let path = dir.join(format!("{}.wav", clip.id));
    if path.exists() {
        return path;
    }
    assert!(
        std::env::var("VOICE_WER_SYNTHESIZE").as_deref() == Ok("1"),
        "Provide corpus WAV files or explicitly enable local synthetic fixtures"
    );
    std::fs::create_dir_all(dir).unwrap();
    let raw = path.with_extension("raw.wav");
    let mut child = Command::new("espeak-ng")
        .args([
            "-v",
            if clip.language == "pt" { "pt-br" } else { "en-us" },
            "-s",
            "155",
            "-w",
        ])
        .arg(&raw)
        .arg("--stdin")
        .stdin(Stdio::piped())
        .spawn()
        .expect("local espeak-ng");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(clip.reference.as_bytes())
        .unwrap();
    assert!(child.wait().unwrap().success());
    assert!(
        Command::new("ffmpeg")
            .args(["-v", "error", "-y", "-i"])
            .arg(&raw)
            .args(["-ar", "16000", "-ac", "1", "-c:a", "pcm_s16le"])
            .arg(&path)
            .status()
            .unwrap()
            .success()
    );
    std::fs::remove_file(raw).unwrap();
    path
}

#[test]
fn word_error_distance_counts_substitution_insertion_deletion_and_empty_audio() {
    let reference = normalized("Hello, local world!");
    assert_eq!(distance(&reference, &normalized("hello LOCAL world")), 0);
    assert_eq!(distance(&reference, &normalized("hello brave world")), 1);
    assert_eq!(distance(&reference, &normalized("hello local new world")), 1);
    assert_eq!(distance(&reference, &normalized("hello world")), 1);
    assert_eq!(distance(&[], &normalized("invented")), 1);
}

#[test]
#[ignore = "offline vocabulary WER; requires VOICE_WER_MODELS and VOICE_WER_CORPUS, no downloads"]
fn vocabulary_wer_on_twenty_sentences_per_language() {
    let models = PathBuf::from(std::env::var_os("VOICE_WER_MODELS").expect("installed model directory"));
    let corpus = PathBuf::from(std::env::var_os("VOICE_WER_CORPUS").expect("local corpus directory"));
    let manifest = std::env::var_os("VOICE_WER_MANIFEST")
        .map(PathBuf::from)
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/vocabulary.jsonl"));
    let clips: Vec<Clip> = std::fs::read_to_string(manifest)
        .unwrap()
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    for language in ["pt", "en"] {
        assert_eq!(clips.iter().filter(|c| c.language == language).count(), 20);
    }
    for clip in &clips {
        assert!(
            !clip.id.is_empty() && clip.id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-'),
            "safe fixture id"
        );
        assert!(!normalized(&clip.reference).is_empty(), "nonempty reference");
    }
    let selected = std::env::var("VOICE_WER_MODEL").ok();
    if let Some(id) = &selected {
        assert!(
            crate::MODELS.iter().any(|m| m.id == id),
            "unknown benchmark model"
        );
    }
    assert_eq!(clips.len(), 40);
    let stop = AtomicBool::new(false);
    for model in crate::MODELS
        .iter()
        .filter(|m| selected.as_deref().is_none_or(|id| id == m.id))
    {
        let transcriber = crate::Transcriber::load(&crate::model_path(&models, model), model.prompt).unwrap();
        {
            let _busy = crate::WHISPER.lock();
            let many: Vec<String> = (0..1000).map(|i| format!("Project{i}")).collect();
            let tokens = super::build(crate::vocabulary("pt"), &many, |text| {
                transcriber
                    .ctx
                    .tokenize(text, text.len() + 16)
                    .map_err(|e| e.to_string())
            })
            .unwrap()
            .unwrap();
            assert!(tokens.len() <= super::TOKENS);
        }
        for language in ["pt", "en"] {
            let (mut before, mut after, mut total) = (0, 0, 0);
            let dictionary: Vec<String> = clips
                .iter()
                .filter(|c| c.language == language)
                .flat_map(|c| c.words.clone())
                .collect();
            for (index, clip) in clips.iter().filter(|c| c.language == language).enumerate() {
                let path = synthesize(&corpus, clip);
                let pcm = audio(&path);
                let baseline = transcriber
                    .transcribe_cancellable(&pcm, Some(language), None, &stop, true)
                    .unwrap()
                    .unwrap();
                let primed = transcriber
                    .transcribe_with_vocabulary(&pcm, Some(language), None, &stop, true, &dictionary)
                    .unwrap()
                    .unwrap();
                if index == 0 && super::enabled(model.id, Some(language)) {
                    assert_eq!(
                        transcriber
                            .preview(&pcm, Some(language), &stop, true, &dictionary)
                            .unwrap(),
                        Some(primed.clone()),
                        "preview shares the recording context"
                    );
                    assert_eq!(
                        transcriber
                            .transcribe_with_vocabulary(&pcm, Some(language), None, &stop, true, &[])
                            .unwrap(),
                        Some(baseline.clone()),
                        "disabling context on a cached model restores the baseline"
                    );
                    assert_eq!(
                        transcriber
                            .transcribe_cancellable(&pcm, Some(language), None, &stop, true)
                            .unwrap(),
                        Some(baseline.clone()),
                        "Chat never inherits dictation context"
                    );
                    assert!(
                        transcriber
                            .transcribe_with_vocabulary(
                                &pcm,
                                Some(language),
                                None,
                                &AtomicBool::new(true),
                                true,
                                &dictionary
                            )
                            .unwrap()
                            .is_none()
                    );
                }
                let expected = normalized(&clip.reference);
                before += distance(&expected, &normalized(&baseline));
                after += distance(&expected, &normalized(&primed));
                total += expected.len();

                eprintln!("{} {} baseline={baseline:?} primed={primed:?}", model.id, clip.id);
            }
            let mut silent = true;
            for fixture in ["silence.wav", "breath-noise.wav"] {
                let pcm = audio(
                    &Path::new(env!("CARGO_MANIFEST_DIR"))
                        .join("tests/fixtures")
                        .join(fixture),
                );
                let output = transcriber
                    .transcribe_with_vocabulary(&pcm, Some(language), None, &stop, true, &dictionary)
                    .unwrap()
                    .unwrap();
                silent &= output.is_empty();
                let raw = transcriber
                    .decode(&pcm, Some(language), Some(&stop), true, Some(&dictionary))
                    .unwrap()
                    .unwrap();
                if !output.is_empty() || !raw.text.is_empty() {
                    eprintln!(
                        "SILENCE model={} lang={language} fixture={fixture} public={output:?} raw={raw:?}",
                        model.id
                    );
                }
                silent &= raw.text.is_empty();
            }
            eprintln!(
                "WER model={} lang={language} words={total} baseline={before} primed={after} silence={silent} improved={}",
                model.id,
                after < before && silent
            );
            if super::enabled(model.id, Some(language)) {
                assert!(after < before && silent);
            }
        }
    }
}
