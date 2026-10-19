//! Settings owns explicit installation. Preparing files never activates speaker analysis.
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::{AppHandle, Manager};
use vults_voice::meeting::assets::{self, Cancel};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
enum Phase {
    Idle,
    Checking,
    Review,
    Downloading,
    Cancelling,
    Installed,
    Cancelled,
    Failed,
    Unsupported,
}
impl Phase {
    fn busy(self) -> bool {
        matches!(self, Self::Checking | Self::Downloading | Self::Cancelling)
    }
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    id: String,
    phase: Phase,
    artifact: String,
    downloaded: u64,
    total: u64,
    error: Option<&'static str>,
    bundle: &'static str,
    download_bytes: u64,
}
struct Review {
    token: String,
    manifest: String,
    ledger: String,
    expires: Instant,
}
struct Inner {
    view: Snapshot,
    review: Option<Review>,
    cancel: Cancel,
}
#[derive(Clone)]
pub struct AssetsState(Arc<Mutex<Inner>>);
impl Default for AssetsState {
    fn default() -> Self {
        Self(Arc::new(Mutex::new(Inner {
            view: Snapshot {
                id: String::new(),
                phase: if cfg!(all(
                    target_os = "linux",
                    target_arch = "x86_64",
                    target_env = "gnu"
                )) {
                    Phase::Idle
                } else {
                    Phase::Unsupported
                },
                artifact: String::new(),
                downloaded: 0,
                total: 0,
                error: None,
                bundle: assets::ID,
                download_bytes: assets::download_bytes(),
            },
            review: None,
            cancel: Cancel::default(),
        })))
    }
}
impl AssetsState {
    fn snapshot(&self) -> Snapshot {
        self.0.lock().unwrap_or_else(|e| e.into_inner()).view.clone()
    }
    fn progress(&self, id: &str, progress: assets::Progress) {
        let mut inner = self.0.lock().unwrap_or_else(|e| e.into_inner());
        if inner.view.id == id && inner.view.phase == Phase::Downloading && !inner.cancel.is_cancelled() {
            inner.view.artifact = progress.artifact.into();
            inner.view.downloaded = progress.downloaded;
            inner.view.total = progress.total;
        }
    }
    fn admit(&self, id: &str, consent: bool) -> Result<(String, String, Cancel), String> {
        let mut state = self.0.lock().map_err(|_| "meeting installation is busy")?;
        if state.view.phase != Phase::Review || state.view.id != id || !consent {
            return Err("review the meeting installation first".into());
        }
        let review = state
            .review
            .take()
            .ok_or("review the meeting installation again")?;
        if review.token != id || review.manifest != assets::review_id() || Instant::now() >= review.expires {
            state.view.phase = Phase::Idle;
            return Err("meeting installation review expired".into());
        }
        state.view.phase = Phase::Downloading;
        Ok((review.manifest, review.ledger, state.cancel.clone()))
    }
}

#[tauri::command]
pub fn meeting_assets_status(
    window: tauri::Webview,
    state: tauri::State<'_, AssetsState>,
) -> Result<Snapshot, String> {
    crate::settings_page(&window)?;
    Ok(state.snapshot())
}

#[tauri::command]
pub async fn meeting_assets_review(window: tauri::Webview, app: AppHandle) -> Result<Snapshot, String> {
    crate::settings_page(&window)?;
    let state = app.state::<AssetsState>().inner().clone();
    let id = crate::traces::new_id();
    let cancel = Cancel::default();
    {
        let mut inner = state.0.lock().map_err(|_| "meeting installation is busy")?;
        if inner.view.phase == Phase::Unsupported || inner.view.phase.busy() {
            return Ok(inner.view.clone());
        }
        inner.review = None;
        inner.cancel = cancel.clone();
        inner.view.id = id.clone();
        inner.view.phase = Phase::Checking;
        inner.view.error = None;
        inner.view.artifact.clear();
        inner.view.downloaded = 0;
        inner.view.total = 0;
    }
    let stop = cancel.clone();
    let checked = tauri::async_runtime::spawn_blocking(move || {
        let installed = assets::installed(&crate::voice::models_dir().join("meetings"), &stop)?.is_some();
        let ledger = vults_store::Store::open(&crate::history::dir())
            .and_then(|s| s.request_budget_id())
            .map_err(|_| "meeting installation ledger unavailable")?;
        Ok::<_, String>((installed, ledger))
    })
    .await;
    let mut inner = state.0.lock().map_err(|_| "meeting installation is busy")?;
    if inner.view.id == id {
        if cancel.is_cancelled() {
            inner.view.phase = Phase::Cancelled;
        } else if let Ok(Ok((installed, ledger))) = checked {
            inner.view.phase = if installed {
                Phase::Installed
            } else {
                Phase::Review
            };
            if !installed {
                inner.review = Some(Review {
                    token: id,
                    manifest: assets::review_id(),
                    ledger,
                    expires: Instant::now() + Duration::from_secs(60),
                });
            }
        } else {
            inner.view.phase = Phase::Failed;
            inner.view.error = Some("verification");
        }
    }
    Ok(inner.view.clone())
}

#[tauri::command]
pub fn meeting_assets_begin(
    window: tauri::Webview,
    app: AppHandle,
    id: String,
    consent: bool,
) -> Result<Snapshot, String> {
    crate::settings_page(&window)?;
    let state = app.state::<AssetsState>().inner().clone();
    let (manifest, ledger, cancel) = state.admit(&id, consent)?;
    let snapshot = state.snapshot();
    tauri::async_runtime::spawn(async move {
        let progress = state.clone();
        let progress_id = id.clone();
        let result = tokio::spawn(async move {
            let gate = crate::gate::MeetingAssetsGate::new(crate::history::dir(), ledger, cancel.clone());
            let result = assets::install(
                crate::voice::models_dir().join("meetings"),
                &manifest,
                &gate,
                cancel,
                move |p| progress.progress(&progress_id, p),
            )
            .await;
            (result, gate.denied())
        })
        .await;
        let mut inner = state.0.lock().unwrap_or_else(|e| e.into_inner());
        if inner.view.id != id {
            return;
        }
        match result {
            Ok((Ok(Some(_)), _)) => inner.view.phase = Phase::Installed,
            Ok((Ok(None), _)) => inner.view.phase = Phase::Cancelled,
            other => {
                inner.view.phase = Phase::Failed;
                inner.view.error = Some(if matches!(other, Ok((_, true))) {
                    "admission"
                } else {
                    "installation"
                });
            }
        }
    });
    Ok(snapshot)
}

#[tauri::command]
pub fn meeting_assets_cancel(
    window: tauri::Webview,
    state: tauri::State<'_, AssetsState>,
    id: String,
) -> Result<Snapshot, String> {
    crate::settings_page(&window)?;
    let mut inner = state.0.lock().map_err(|_| "meeting installation is busy")?;
    if inner.view.id == id {
        inner.cancel.cancel();
        inner.review = None;
        inner.view.phase = if inner.view.phase.busy() {
            Phase::Cancelling
        } else if inner.view.phase == Phase::Review {
            Phase::Cancelled
        } else {
            inner.view.phase
        };
    }
    Ok(inner.view.clone())
}

pub fn shutdown(app: &AppHandle) {
    if let Some(state) = app.try_state::<AssetsState>() {
        state.0.lock().unwrap_or_else(|e| e.into_inner()).cancel.cancel();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn reviewed() -> AssetsState {
        let state = AssetsState::default();
        {
            let mut inner = state.0.lock().unwrap();
            inner.view.phase = Phase::Review;
            inner.view.id = "review".into();
            inner.review = Some(Review {
                token: "review".into(),
                manifest: assets::review_id(),
                ledger: "ledger".into(),
                expires: Instant::now() + Duration::from_secs(60),
            });
        }
        state
    }
    #[test]
    fn late_progress_cannot_change_a_cancelled_or_finished_installation() {
        let update = || assets::Progress {
            artifact: "runtime",
            downloaded: 50,
            total: 100,
        };
        for phase in [
            Phase::Idle,
            Phase::Checking,
            Phase::Review,
            Phase::Cancelling,
            Phase::Installed,
            Phase::Cancelled,
            Phase::Failed,
            Phase::Unsupported,
        ] {
            let state = reviewed();
            state.0.lock().unwrap().view.phase = phase;
            state.progress("review", update());
            let view = state.snapshot();
            assert_eq!(view.phase, phase);
            assert_eq!((view.artifact.as_str(), view.downloaded, view.total), ("", 0, 0));
        }
        let state = reviewed();
        state.admit("review", true).expect("current review");
        state.progress("stale", update());
        assert_eq!(state.snapshot().downloaded, 0);
        state.progress("review", update());
        assert_eq!(state.snapshot().downloaded, 50);
        state.0.lock().unwrap().cancel.cancel();
        state.progress(
            "review",
            assets::Progress {
                downloaded: 99,
                ..update()
            },
        );
        assert_eq!(state.snapshot().downloaded, 50);
    }

    #[test]
    fn installation_needs_current_explicit_single_use_review() {
        let state = reviewed();
        assert!(state.admit("review", false).is_err());
        assert!(state.admit("stale", true).is_err());
        assert!(state.admit("review", true).is_ok());
        assert!(state.admit("review", true).is_err());
        let expired = reviewed();
        expired.0.lock().unwrap().review.as_mut().unwrap().expires = Instant::now();
        assert!(expired.admit("review", true).is_err());
        let stale = reviewed();
        stale.0.lock().unwrap().review.as_mut().unwrap().manifest.clear();
        assert!(stale.admit("review", true).is_err());
        assert_eq!(
            AssetsState::default().snapshot().phase,
            if cfg!(all(
                target_os = "linux",
                target_arch = "x86_64",
                target_env = "gnu"
            )) {
                Phase::Idle
            } else {
                Phase::Unsupported
            }
        );
    }
}
