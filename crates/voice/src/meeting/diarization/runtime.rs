use std::fs::{File, OpenOptions};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, TryLockError};
use std::time::Duration;

use sha2::{Digest, Sha256};

static LOADED: Mutex<Option<(PathBuf, String)>> = Mutex::new(None);

/// Explicit native runtime admission for the optional backend. Not an app distribution policy.
/// Only this module may initialize ort in a process. A loaded runtime cannot be switched/unloaded.
#[derive(Debug)]
pub struct Runtime {
    _private: (),
}

impl Runtime {
    /// Blocking. The caller must supply a trusted, immutable native library and independent hash.
    /// A matching hash identifies bytes, not whether executing a caller-supplied library is safe.
    /// Cancellation is checked during hashing/admission; native initialization itself cannot abort.
    pub fn load(path: &Path, expected: &str, stop: &AtomicBool) -> Result<Option<Self>, String> {
        if stop.load(Ordering::SeqCst) {
            return Ok(None);
        }
        if std::env::var("ORT_DISABLE_TELEMETRY").as_deref() != Ok("1") {
            return Err(
                "native telemetry must be disabled at process startup before loading the speaker runtime"
                    .into(),
            );
        }
        if !path.is_absolute() || expected.len() != 64 || !expected.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err("speaker runtime requires an absolute path and SHA-256 fingerprint".into());
        }
        let Some(hash) = fingerprint(path, stop)? else {
            return Ok(None);
        };
        if hash != expected.to_ascii_lowercase() {
            return Err("speaker runtime fingerprint mismatch; no runtime was loaded".into());
        }
        let path = path.canonicalize().map_err(|e| e.to_string())?;
        let mut state = loop {
            if stop.load(Ordering::SeqCst) {
                return Ok(None);
            }
            match LOADED.try_lock() {
                Ok(state) => break state,
                Err(TryLockError::WouldBlock) => std::thread::park_timeout(Duration::from_millis(10)),
                Err(TryLockError::Poisoned(_)) => return Err("speaker runtime ownership failed".into()),
            }
        };
        if let Some(previous) = state.as_ref() {
            if previous != &(path, hash) {
                return Err("speaker runtime is already pinned for this process".into());
            }
        } else {
            if !ort::init_from(&path)
                .map_err(|e| e.to_string())?
                .with_telemetry(false)
                .commit()
            {
                return Err("speaker runtime was initialized outside its owner".into());
            }
            *state = Some((path, hash));
        }
        Ok((!stop.load(Ordering::SeqCst)).then_some(Self { _private: () }))
    }
}

fn fingerprint(path: &Path, stop: &AtomicBool) -> Result<Option<String>, String> {
    if !std::fs::symlink_metadata(path)
        .map_err(|e| e.to_string())?
        .is_file()
    {
        return Err("speaker runtime must be a regular file".into());
    }
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    let mut file: File = options.open(path).map_err(|e| e.to_string())?;
    let metadata = file.metadata().map_err(|e| e.to_string())?;
    const MAX: u64 = 128 * 1024 * 1024;
    if !metadata.is_file() || metadata.len() == 0 || metadata.len() > MAX {
        return Err("speaker runtime must be a nonempty regular library at most 128 MiB".into());
    }
    let mut read = 0;
    let mut hash = Sha256::new();
    let mut bytes = [0; 64 * 1024];
    loop {
        if stop.load(Ordering::SeqCst) {
            return Ok(None);
        }
        let len = file.read(&mut bytes).map_err(|e| e.to_string())?;
        if len == 0 {
            break;
        }
        read += len as u64;
        if read > metadata.len() {
            return Err("speaker runtime changed while reading".into());
        }
        hash.update(&bytes[..len]);
    }
    if read != metadata.len() {
        return Err("speaker runtime changed while reading".into());
    }
    Ok(Some(format!("{:x}", hash.finalize())))
}
