use std::fs::{File, OpenOptions};
use std::io::Read;
use std::mem::ManuallyDrop;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, TryLockError};
use std::time::Duration;

use sha2::{Digest, Sha256};

struct Loaded {
    path: PathBuf,
    fingerprint: String,
    environment: ort::environment::Environment,
}

static LOADED: Mutex<Option<Loaded>> = Mutex::new(None);

/// Explicit native runtime admission for the optional backend. Not an app distribution policy.
/// Only this module may initialize ort in a process. A loaded runtime cannot be switched/unloaded.
#[derive(Debug)]
pub struct Runtime {
    fingerprint: String,
}

impl Runtime {
    /// The bytes admitted by the process-wide native owner, for backend-specific qualification.
    pub fn fingerprint(&self) -> &str {
        &self.fingerprint
    }
    pub(crate) fn environment(&self) -> Result<ort::environment::Environment, String> {
        let state = LOADED.lock().map_err(|_| "speech runtime ownership failed")?;
        state
            .as_ref()
            .filter(|loaded| loaded.fingerprint == self.fingerprint)
            .map(|loaded| loaded.environment.clone())
            .ok_or_else(|| "speech runtime was not admitted".into())
    }
    /// Blocking. The caller must supply a trusted, immutable native library and independent hash.
    /// A matching hash identifies bytes, not whether executing a caller-supplied library is safe.
    /// Cancellation is checked during hashing/admission; native initialization itself cannot abort.
    pub fn load(path: &Path, expected: &str, stop: &AtomicBool) -> Result<Option<Self>, String> {
        if stop.load(Ordering::SeqCst) {
            return Ok(None);
        }
        if std::env::var("ORT_DISABLE_TELEMETRY").as_deref() != Ok("1") {
            return Err(
                "native telemetry must be disabled at process startup before loading the speech runtime"
                    .into(),
            );
        }
        if !path.is_absolute() || expected.len() != 64 || !expected.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err("speech runtime requires an absolute path and SHA-256 fingerprint".into());
        }
        let Some(hash) = fingerprint(path, stop)? else {
            return Ok(None);
        };
        if hash != expected.to_ascii_lowercase() {
            return Err("speech runtime fingerprint mismatch; no runtime was loaded".into());
        }
        let admitted = hash.clone();
        let path = path.canonicalize().map_err(|e| e.to_string())?;
        let mut state = loop {
            if stop.load(Ordering::SeqCst) {
                return Ok(None);
            }
            match LOADED.try_lock() {
                Ok(state) => break state,
                Err(TryLockError::WouldBlock) => std::thread::park_timeout(Duration::from_millis(10)),
                Err(TryLockError::Poisoned(_)) => return Err("speech runtime ownership failed".into()),
            }
        };
        if let Some(previous) = state.as_ref() {
            if previous.path != path || previous.fingerprint != hash {
                return Err("speech runtime is already pinned for this process".into());
            }
        } else {
            let environment = ort::init_from(&path)
                .map_err(|e| e.to_string())?
                .with_telemetry(false)
                .build()
                .map_err(|e| e.to_string())?;
            *state = Some(Loaded {
                path,
                fingerprint: hash,
                environment,
            });
        }
        Ok((!stop.load(Ordering::SeqCst)).then_some(Self {
            fingerprint: admitted,
        }))
    }
}

fn fingerprint(path: &Path, stop: &AtomicBool) -> Result<Option<String>, String> {
    if !std::fs::symlink_metadata(path)
        .map_err(|e| e.to_string())?
        .is_file()
    {
        return Err("speech runtime must be a regular file".into());
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
        return Err("speech runtime must be a nonempty regular library at most 128 MiB".into());
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
            return Err("speech runtime changed while reading".into());
        }
        hash.update(&bytes[..len]);
    }
    if read != metadata.len() {
        return Err("speech runtime changed while reading".into());
    }
    Ok(Some(format!("{:x}", hash.finalize())))
}

/// Keep native session destruction under the same process-wide speech slot as inference.
pub(crate) struct NativeSession(pub(crate) ManuallyDrop<ort::session::Session>);
impl Drop for NativeSession {
    fn drop(&mut self) {
        let _busy = crate::WHISPER.lock();
        // SAFETY: dropped once, here, under the native-work slot.
        unsafe { ManuallyDrop::drop(&mut self.0) };
    }
}

#[test]
#[cfg(feature = "semantic-turn")]
fn semantic_backend_refuses_an_unqualified_runtime_before_opening_a_model() {
    let runtime = Runtime {
        fingerprint: "unqualified".into(),
    };
    assert!(runtime.environment().is_err());
    let path = Path::new("/missing/model");
    let error = crate::turn::semantic::Detector::load(&runtime, path, &AtomicBool::new(false)).unwrap_err();
    assert!(error.contains("qualified ONNX runtime"));
    assert!(
        crate::turn::semantic::Detector::load(&runtime, path, &AtomicBool::new(true))
            .unwrap()
            .is_none()
    );
}
