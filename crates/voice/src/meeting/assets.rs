//! Explicit, transactional installation; app consent, quota and traces belong to its gate.

mod catalog;
mod download;
mod files;

use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use tokio::sync::Notify;

pub use catalog::{FILES, FileSpec, ID, RUNTIME_SHA256};

#[derive(Debug, Clone, Default)]
pub struct Cancel(Arc<Signal>);
#[derive(Debug, Default)]
struct Signal {
    stop: AtomicBool,
    wake: Notify,
}
impl Cancel {
    pub fn cancel(&self) {
        self.0.stop.store(true, Ordering::SeqCst);
        self.0.wake.notify_waiters();
    }
    pub fn is_cancelled(&self) -> bool {
        self.0.stop.load(Ordering::SeqCst)
    }
    fn check(&self) -> Result<(), String> {
        if self.is_cancelled() {
            Err("meeting asset installation cancelled".into())
        } else {
            Ok(())
        }
    }
    async fn cancelled(&self) {
        loop {
            let notified = self.0.wake.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();
            if self.is_cancelled() {
                return;
            }
            notified.await;
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Complete,
    Failed,
    Cancelled,
}
/// App callers use its explicit-action network gate, quota and content-free trace.
/// Called before every HTTP attempt, redirects included. There is no permissive default gate.
pub trait Gate: Send + Sync {
    fn allow(&self, artifact: &'static str) -> bool;
    fn done(&self, artifact: &'static str, outcome: Outcome, elapsed_ms: u64);
}
#[derive(Debug, Clone, Copy)]
pub struct Progress {
    pub artifact: &'static str,
    pub downloaded: u64,
    pub total: u64,
}
#[derive(Debug, Clone)]
pub struct Bundle {
    path: PathBuf,
}
impl Bundle {
    pub fn directory(&self) -> &Path {
        &self.path
    }
    pub fn segmentation(&self) -> PathBuf {
        self.path.join("segmentation.onnx")
    }
    pub fn embedding(&self) -> PathBuf {
        self.path.join("embedding.onnx")
    }
    pub fn runtime(&self) -> PathBuf {
        self.path.join("libonnxruntime.so.1.31.0")
    }
}
/// This manifest fingerprint must match the caller's reviewed version, not an arbitrary URL.
pub fn review_id() -> String {
    let mut hash = Sha256::new();
    hash.update(ID);
    for d in catalog::DOWNLOADS {
        hash.update(d.url);
        hash.update(d.sha256);
        hash.update(d.size.to_le_bytes());
    }
    for f in FILES {
        hash.update(f.name);
        hash.update(f.sha256);
        hash.update(f.size.to_le_bytes());
    }
    for &(name, bytes) in catalog::NOTICES {
        hash.update(name);
        hash.update(bytes);
    }
    format!("{:x}", hash.finalize())
}
fn directory_name() -> String {
    format!("{}-{}", ID, &review_id()[..12])
}

pub fn download_bytes() -> u64 {
    catalog::DOWNLOADS.iter().map(|d| d.size).sum()
}
/// Read-only integrity check; a cached directory or length alone never proves readiness.
pub fn installed(root: &Path, cancel: &Cancel) -> Result<Option<Bundle>, String> {
    files::installed(root, &|| cancel.check())
}

/// Read-only preview readiness uses the recording owner's cancellation, including file hashes.
#[cfg(feature = "diarization")]
pub fn installed_for_analysis(root: &Path, stop: &AtomicBool) -> Result<Option<Bundle>, String> {
    files::installed(root, &|| {
        if stop.load(Ordering::SeqCst) {
            Err("meeting analysis cancelled".into())
        } else {
            Ok(())
        }
    })
}

struct CancelOnDrop(Option<Cancel>);
impl Drop for CancelOnDrop {
    fn drop(&mut self) {
        if let Some(cancel) = self.0.take() {
            cancel.cancel();
        }
    }
}

/// No automatic start/retry/update. Caller supplies explicit review, a rejecting gate and ownership.
/// A failed/cancelled attempt removes its private staging data and preserves installed versions.
/// Cancellation racing the final atomic rename may finish after the valid bundle is committed.
pub async fn install(
    root: PathBuf,
    review: &str,
    gate: &dyn Gate,
    cancel: Cancel,
    progress: impl Fn(Progress) + Send + Sync,
) -> Result<Option<Bundle>, String> {
    if !cfg!(all(
        target_os = "linux",
        target_arch = "x86_64",
        target_env = "gnu"
    )) {
        return Err("meeting assets are supported on Linux x86-64 only".into());
    }
    if review != review_id() {
        return Err("meeting asset review is stale or missing".into());
    }
    if cancel.is_cancelled() {
        return Ok(None);
    }
    let mut cancel_on_drop = CancelOnDrop(Some(cancel.clone()));
    let result = install_inner(root, gate, &cancel, &progress).await;
    cancel_on_drop.0 = None;
    match result {
        Ok(bundle) => Ok(Some(bundle)),
        Err(_) if cancel.is_cancelled() => Ok(None),
        Err(error) => Err(error),
    }
}
async fn install_inner(
    root: PathBuf,
    gate: &dyn Gate,
    cancel: &Cancel,
    progress: &(impl Fn(Progress) + Send + Sync),
) -> Result<Bundle, String> {
    let stop = cancel.clone();
    let prepared = tokio::task::spawn_blocking(move || files::Stage::begin(root, &stop))
        .await
        .map_err(|e| e.to_string())??;
    let mut stage = match prepared {
        files::Prepared::Ready(bundle) => return Ok(bundle),
        files::Prepared::Stage(stage) => stage,
    };
    for &artifact in catalog::DOWNLOADS {
        download::fetch(artifact, &stage.path, gate, cancel, progress).await?;
        let stop = cancel.clone();
        stage = tokio::task::spawn_blocking(move || {
            stage.unpack(artifact, &stop)?;
            Ok::<_, String>(stage)
        })
        .await
        .map_err(|e| e.to_string())??;
    }
    let stop = cancel.clone();
    tokio::task::spawn_blocking(move || stage.commit(&stop))
        .await
        .map_err(|e| e.to_string())?
}
