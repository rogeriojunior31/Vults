//! Offline model admission only; this does not approve downloads or initialize a native runtime.

use std::fs::OpenOptions;
use std::io::Read;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use sha2::{Digest, Sha256};

pub const SEGMENTATION_SHA256: &str = "220ad67ca923bef2fa91f2390c786097bf305bceb5e261d4af67b38e938e1079";
pub const EMBEDDING_SHA256: &str = "f682b514c05d947ee3fa91cd6ec6c5c7543479a128373fa29b1faedccd21fd11";
#[cfg(feature = "diarization")]
pub const ADVANCED_EMBEDDING_SHA256: &str =
    "aa3cfc16963a10586a9393f5035d6d6b57e98d358b347f80c2a30bf4f00ceba2";
const SEGMENTATION_SIZE: usize = 5_992_913;
const EMBEDDING_SIZE: usize = 28_281_138;

/// The exact checked bytes, not mutable paths to reopen during native model loading.
/// Dropping this value releases both buffers; it retains no file handles or filesystem state.
pub struct VerifiedModels {
    segmentation: Vec<u8>,
    embedding: Vec<u8>,
}

impl VerifiedModels {
    pub fn segmentation(&self) -> &[u8] {
        &self.segmentation
    }

    pub fn embedding(&self) -> &[u8] {
        &self.embedding
    }
}

impl std::fmt::Debug for VerifiedModels {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("VerifiedModels")
    }
}

/// Blocking: call on the caller's recording worker, with its consent/cleanup ownership retained.
/// Both artifacts must match the V5.4 export pins. Failure/cancellation exposes no partial pair.
/// The runtime and production download/distribution policy remain separate release requirements.
pub fn read_models(
    segmentation: &Path,
    embedding: &Path,
    stop: &AtomicBool,
) -> Result<Option<VerifiedModels>, String> {
    read_pair(segmentation, embedding, EMBEDDING_SIZE, EMBEDDING_SHA256, stop)
}

/// Explicit offline bilingual-model comparison. This candidate is not in the app's catalog.
#[cfg(feature = "diarization")]
pub fn read_advanced_models(
    segmentation: &Path,
    embedding: &Path,
    stop: &AtomicBool,
) -> Result<Option<VerifiedModels>, String> {
    read_pair(
        segmentation,
        embedding,
        28_281_164,
        ADVANCED_EMBEDDING_SHA256,
        stop,
    )
}

fn read_pair(
    segmentation: &Path,
    embedding: &Path,
    embedding_size: usize,
    embedding_hash: &str,
    stop: &AtomicBool,
) -> Result<Option<VerifiedModels>, String> {
    if stop.load(Ordering::SeqCst) {
        return Ok(None);
    }
    let Some(segmentation) = read_file(segmentation, SEGMENTATION_SIZE, SEGMENTATION_SHA256, stop)? else {
        return Ok(None);
    };
    let Some(embedding) = read_file(embedding, embedding_size, embedding_hash, stop)? else {
        return Ok(None);
    };
    if stop.load(Ordering::SeqCst) {
        return Ok(None);
    }
    Ok(Some(VerifiedModels {
        segmentation,
        embedding,
    }))
}

fn read_file(path: &Path, size: usize, hash: &str, stop: &AtomicBool) -> Result<Option<Vec<u8>>, String> {
    if stop.load(Ordering::SeqCst) {
        return Ok(None);
    }
    if !std::fs::symlink_metadata(path)
        .map_err(|e| e.to_string())?
        .is_file()
    {
        return Err("speaker models must be regular files".into());
    }
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        // A replaced FIFO must not block, and a replaced symlink must not be followed.
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    let file = options.open(path).map_err(|e| e.to_string())?;
    let metadata = file.metadata().map_err(|e| e.to_string())?;
    if !metadata.is_file() || metadata.len() != size as u64 {
        return Err("speaker model type or size does not match the pinned export".into());
    }
    read_checked(file, size, hash, stop)
}

fn read_checked(
    mut input: impl Read,
    size: usize,
    expected: &str,
    stop: &AtomicBool,
) -> Result<Option<Vec<u8>>, String> {
    let mut bytes = Vec::with_capacity(size);
    let mut hash = Sha256::new();
    let mut chunk = [0; 64 * 1024];
    while bytes.len() < size {
        if stop.load(Ordering::SeqCst) {
            return Ok(None);
        }
        let count = (size - bytes.len()).min(chunk.len());
        input
            .read_exact(&mut chunk[..count])
            .map_err(|e| format!("could not read speaker model: {e}"))?;
        hash.update(&chunk[..count]);
        bytes.extend_from_slice(&chunk[..count]);
    }
    if stop.load(Ordering::SeqCst) {
        return Ok(None);
    }
    if input.read(&mut chunk[..1]).map_err(|e| e.to_string())? != 0
        || format!("{:x}", hash.finalize()) != expected
    {
        return Err("speaker model fingerprint or size does not match the pinned export".into());
    }
    Ok((!stop.load(Ordering::SeqCst)).then_some(bytes))
}

#[cfg(test)]
mod tests;
