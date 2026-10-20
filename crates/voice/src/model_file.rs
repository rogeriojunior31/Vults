//! Exact-size/hash model admission shared by offline speech consumers.
use sha2::{Digest, Sha256};
use std::{
    fs::OpenOptions,
    io::Read,
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
};

pub(crate) fn read_file(
    path: &Path,
    size: usize,
    hash: &str,
    stop: &AtomicBool,
) -> Result<Option<Vec<u8>>, String> {
    if stop.load(Ordering::SeqCst) {
        return Ok(None);
    }
    if !std::fs::symlink_metadata(path)
        .map_err(|e| e.to_string())?
        .is_file()
    {
        return Err("speech models must be regular files".into());
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
        return Err("speech model type or size does not match the pinned export".into());
    }
    read_checked(file, size, hash, stop)
}

pub(crate) fn read_checked(
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
            .map_err(|e| format!("could not read speech model: {e}"))?;
        hash.update(&chunk[..count]);
        bytes.extend_from_slice(&chunk[..count]);
    }
    if stop.load(Ordering::SeqCst) {
        return Ok(None);
    }
    if input.read(&mut chunk[..1]).map_err(|e| e.to_string())? != 0
        || format!("{:x}", hash.finalize()) != expected
    {
        return Err("speech model fingerprint or size does not match the pinned export".into());
    }
    Ok((!stop.load(Ordering::SeqCst)).then_some(bytes))
}
