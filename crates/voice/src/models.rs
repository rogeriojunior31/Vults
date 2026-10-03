//! The whisper models the user can download, from the whisper.cpp repository on Hugging Face.
//! Only after a click in Settings; each file is checked against its known size and SHA-256
//! before it is used.

use std::path::{Path, PathBuf};

use futures_util::StreamExt;
use serde::Serialize;
use sha2::{Digest, Sha256};
use tokio::io::AsyncWriteExt;

const BASE_URL: &str = "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/";

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct Model {
    pub id: &'static str,
    pub label: &'static str,
    /// Bytes on disk.
    pub size: u64,
    #[serde(skip)]
    file: &'static str,
    #[serde(skip)]
    sha256: &'static str,
}

/// Multilingual models only: the user speaks whatever they speak. Sizes and hashes from the
/// repository's LFS metadata.
pub const MODELS: &[Model] = &[
    Model {
        id: "base",
        label: "Base: fast, but weak outside English",
        size: 59_707_625,
        file: "ggml-base-q5_1.bin",
        sha256: "422f1ae452ade6f30a004d7e5c6a43195e4433bc370bf23fac9cc591f01a8898",
    },
    Model {
        id: "small",
        label: "Small: good in Portuguese and other languages",
        size: 190_085_487,
        file: "ggml-small-q5_1.bin",
        sha256: "ae85e4a935d7a567bd102fe55afc16bb595bdb618e11b2fc7591bc08120411bb",
    },
    Model {
        id: "turbo",
        label: "Large v3 Turbo: the most accurate; best with a GPU",
        size: 574_041_195,
        file: "ggml-large-v3-turbo-q5_0.bin",
        sha256: "394221709cd5ad1f40c46e6031ca61bce88931e6e088c188294c6d5a55ffa7e2",
    },
];

pub fn model(id: &str) -> Option<&'static Model> {
    MODELS.iter().find(|m| m.id == id)
}

pub fn model_path(dir: &Path, m: &Model) -> PathBuf {
    dir.join(m.file)
}

/// The models already downloaded (checked by size; the hash was checked when it came in).
pub fn installed(dir: &Path) -> Vec<&'static str> {
    MODELS
        .iter()
        .filter(|m| std::fs::metadata(model_path(dir, m)).is_ok_and(|md| md.len() == m.size))
        .map(|m| m.id)
        .collect()
}

/// Downloads a model into `dir`; `progress` gets the bytes so far. The file only takes its final
/// name once its size and SHA-256 match, so a cut download is never loaded.
pub async fn download(dir: &Path, id: &str, progress: impl Fn(u64, u64)) -> Result<PathBuf, String> {
    let m = model(id).ok_or_else(|| format!("unknown voice model {id}"))?;
    tokio::fs::create_dir_all(dir).await.map_err(|e| e.to_string())?;
    let target = model_path(dir, m);
    let part = target.with_extension("part");
    let response = reqwest::get(format!("{BASE_URL}{}", m.file))
        .await
        .and_then(reqwest::Response::error_for_status)
        .map_err(|e| format!("can't download the voice model: {e}"))?;
    let mut file = tokio::fs::File::create(&part).await.map_err(|e| e.to_string())?;
    let mut hash = Sha256::new();
    let mut done = 0u64;
    let mut body = response.bytes_stream();
    while let Some(chunk) = body.next().await {
        let chunk = chunk.map_err(|e| format!("the download stopped: {e}"))?;
        done += chunk.len() as u64;
        if done > m.size {
            break;
        }
        hash.update(&chunk);
        file.write_all(&chunk).await.map_err(|e| e.to_string())?;
        progress(done, m.size);
    }
    file.flush().await.map_err(|e| e.to_string())?;
    drop(file);
    let digest: String = hash.finalize().iter().map(|b| format!("{b:02x}")).collect();
    if done != m.size || digest != m.sha256 {
        let _ = tokio::fs::remove_file(&part).await;
        return Err("the voice model did not download correctly; try again".into());
    }
    tokio::fs::rename(&part, &target)
        .await
        .map_err(|e| e.to_string())?;
    Ok(target)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_model_is_pinned() {
        for m in MODELS {
            assert_eq!(m.sha256.len(), 64, "{}", m.id);
            assert!(m.file.starts_with("ggml-") && m.file.ends_with(".bin"));
        }
        assert!(model("base").is_some() && model("nope").is_none());
    }

    #[test]
    fn a_file_of_the_wrong_size_is_not_installed() {
        let dir = std::env::temp_dir().join(format!("voice-models-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("temp dir");
        let base = model("base").expect("base");
        std::fs::write(model_path(&dir, base), b"cut short").expect("write");
        assert!(installed(&dir).is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
