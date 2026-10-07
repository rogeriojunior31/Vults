//! Kokoro's model and voices, downloaded from Hugging Face only after a click in Settings, from a
//! pinned revision. Each file is checked against its known size and SHA-256 before it is used.

use std::path::{Path, PathBuf};

use futures_util::StreamExt;
use serde::Serialize;
use sha2::{Digest, Sha256};
use tokio::io::AsyncWriteExt;

use crate::Lang;

/// onnx-community/Kokoro-82M-v1.0-ONNX (Apache-2.0), at a fixed commit: a later push changes
/// nothing here. Sizes and hashes from its LFS metadata, checked against a download (2026-10-06).
const BASE_URL: &str = "https://huggingface.co/onnx-community/Kokoro-82M-v1.0-ONNX/resolve/1939ad2a8e416c0acfeecc08a694d14ef25f2231/";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct File {
    /// Where it is in the repository.
    remote: &'static str,
    /// Its name on disk.
    local: &'static str,
    size: u64,
    sha256: &'static str,
}

/// fp32: on the CPU fp16 gave silence and int8 was 3.5 times slower (E15).
const MODEL: File = File {
    remote: "onnx/model.onnx",
    local: "kokoro-v1.0.onnx",
    size: 325_532_232,
    sha256: "8fbea51ea711f2af382e88c833d9e288c6dc82ce5e98421ea61c058ce21a34cb",
};

#[derive(Serialize, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Voice {
    pub id: &'static str,
    pub label: &'static str,
    pub lang: Lang,
    #[serde(skip)]
    file: File,
}

const fn voice(
    id: &'static str,
    label: &'static str,
    lang: Lang,
    remote: &'static str,
    local: &'static str,
    sha256: &'static str,
) -> Voice {
    Voice {
        id,
        label,
        lang,
        file: File {
            remote,
            local,
            size: 522_240,
            sha256,
        },
    }
}

/// The first of each language is its default.
pub const VOICES: &[Voice] = &[
    voice(
        "am_michael",
        "Michael",
        Lang::En,
        "voices/am_michael.bin",
        "am_michael.bin",
        "1d1f21dd8da39c30705cd4c75d039d265e9bc4a2a93ed09bc9e1b1225eb95ba1",
    ),
    voice(
        "af_heart",
        "Heart",
        Lang::En,
        "voices/af_heart.bin",
        "af_heart.bin",
        "d583ccff3cdca2f7fae535cb998ac07e9fcb90f09737b9a41fa2734ec44a8f0b",
    ),
    voice(
        "pm_alex",
        "Alex",
        Lang::Pt,
        "voices/pm_alex.bin",
        "pm_alex.bin",
        "0175c753f59c54e7fd5a995bedef0c5ff2fb67e0043dd3dcb2ae74ec2acbeb2a",
    ),
    voice(
        "pm_santa",
        "Santa",
        Lang::Pt,
        "voices/pm_santa.bin",
        "pm_santa.bin",
        "8b012db3185778afe2e45a62cbad69db73021774fe68dda634bcc748a982eede",
    ),
    voice(
        "pf_dora",
        "Dora",
        Lang::Pt,
        "voices/pf_dora.bin",
        "pf_dora.bin",
        "3da7b5b2d91847ebf5646f57631af6ececae3c29a89cd300f06edf9aa6cfe9ee",
    ),
];

/// The voice `id` for `lang`, else that language's default.
pub fn voice_for(lang: Lang, id: Option<&str>) -> &'static Voice {
    let of_lang = || VOICES.iter().filter(move |v| v.lang == lang);
    id.and_then(|id| of_lang().find(|v| v.id == id))
        .or_else(|| of_lang().next())
        .unwrap_or(&VOICES[0])
}

fn files() -> impl Iterator<Item = File> {
    std::iter::once(MODEL).chain(VOICES.iter().map(|v| v.file))
}

/// Bytes the download takes on disk.
pub fn size() -> u64 {
    files().map(|f| f.size).sum()
}

pub fn model_path(dir: &Path) -> PathBuf {
    dir.join(MODEL.local)
}

pub fn voice_path(dir: &Path, v: &Voice) -> PathBuf {
    dir.join(v.file.local)
}

/// The model and every voice are on disk (checked by size; the hash was checked as they came in).
pub fn installed(dir: &Path) -> bool {
    files().all(|f| present(dir, &f))
}

fn present(dir: &Path, f: &File) -> bool {
    std::fs::metadata(dir.join(f.local)).is_ok_and(|md| md.len() == f.size)
}

/// Downloads whatever is missing into `dir`; `progress` gets the bytes so far and the total. A
/// file only takes its final name once its size and SHA-256 match, so a cut download is never
/// loaded.
pub async fn download(dir: &Path, progress: impl Fn(u64, u64)) -> Result<(), String> {
    tokio::fs::create_dir_all(dir).await.map_err(|e| e.to_string())?;
    let total = size();
    let mut before = 0;
    for f in files() {
        if !present(dir, &f) {
            fetch(dir, &f, |done| progress(before + done, total)).await?;
        }
        before += f.size;
        progress(before, total);
    }
    Ok(())
}

async fn fetch(dir: &Path, f: &File, progress: impl Fn(u64)) -> Result<(), String> {
    let target = dir.join(f.local);
    let part = target.with_extension("part");
    // A stalled connection ends in an error, not a download that never finishes.
    let client = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(30))
        .read_timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| e.to_string())?;
    let response = client
        .get(format!("{BASE_URL}{}", f.remote))
        .send()
        .await
        .and_then(reqwest::Response::error_for_status)
        .map_err(|e| format!("can't download the speech model: {e}"))?;
    let mut file = tokio::fs::File::create(&part).await.map_err(|e| e.to_string())?;
    let mut hash = Sha256::new();
    let mut done = 0u64;
    let mut body = response.bytes_stream();
    while let Some(chunk) = body.next().await {
        let chunk = chunk.map_err(|e| format!("the download stopped: {e}"))?;
        done += chunk.len() as u64;
        if done > f.size {
            break;
        }
        hash.update(&chunk);
        file.write_all(&chunk).await.map_err(|e| e.to_string())?;
        progress(done);
    }
    file.flush().await.map_err(|e| e.to_string())?;
    drop(file);
    if done != f.size || !matches(&hash.finalize(), f.sha256) {
        let _ = tokio::fs::remove_file(&part).await;
        return Err("the speech model did not download correctly; try again".into());
    }
    tokio::fs::rename(&part, &target).await.map_err(|e| e.to_string())
}

fn matches(digest: &[u8], hex: &str) -> bool {
    digest.iter().map(|b| format!("{b:02x}")).collect::<String>() == hex
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_file_is_pinned() {
        for f in files() {
            assert_eq!(f.sha256.len(), 64, "{}", f.local);
            assert!(f.sha256.chars().all(|c| c.is_ascii_hexdigit()));
            assert!(f.remote.ends_with(f.local) || f.remote == MODEL.remote);
        }
        assert!(BASE_URL.contains("/resolve/") && !BASE_URL.contains("/main/"));
        assert_eq!(size(), 325_532_232 + 5 * 522_240);
        assert!(matches(
            &Sha256::digest(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        ));
    }

    #[test]
    fn each_language_has_a_default_voice() {
        assert_eq!(voice_for(Lang::En, None).id, "am_michael");
        assert_eq!(voice_for(Lang::Pt, Some("pf_dora")).id, "pf_dora");
        // A voice of the other language (or none we know) is that language's default.
        assert_eq!(voice_for(Lang::Pt, Some("af_heart")).id, "pm_alex");
        assert_eq!(voice_for(Lang::En, Some("gone")).id, "am_michael");
    }

    #[test]
    fn a_file_of_the_wrong_size_is_not_installed() {
        let dir = std::env::temp_dir().join(format!("speech-models-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("temp dir");
        assert!(!installed(&dir));
        std::fs::write(model_path(&dir), b"cut short").expect("write");
        assert!(!installed(&dir));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
