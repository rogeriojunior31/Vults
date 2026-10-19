//! Read-only local diagnostics. Only an explicit Settings action opens the microphone.
pub(crate) mod microphone;
pub use microphone::stop_mic;

use serde::Serialize;
use std::sync::atomic::{AtomicU64, Ordering};
use tauri::{AppHandle, Manager};
use vults_store::dictations::TimingSummary;

static LOOPS: AtomicU64 = AtomicU64::new(0);
static RETRIES: AtomicU64 = AtomicU64::new(0);
static COPIED: AtomicU64 = AtomicU64::new(0);
#[cfg(target_os = "linux")]
pub fn loops_cut(count: u64) {
    LOOPS.fetch_add(count, Ordering::Relaxed);
}
#[cfg(target_os = "linux")]
pub fn retried() {
    RETRIES.fetch_add(1, Ordering::Relaxed);
}
#[cfg(target_os = "linux")]
pub fn copied() {
    COPIED.fetch_add(1, Ordering::Relaxed);
}

#[derive(Debug, Serialize)]
pub struct Counters {
    loops: u64,
    retries: u64,
    copied: u64,
}
#[derive(Debug, Serialize)]
pub struct System {
    host: Option<String>,
    device: Option<String>,
    rate: Option<u32>,
    session: &'static str,
    shortcuts: &'static str,
    keyboard: String,
    kwin: &'static str,
    gpu: bool,
    vad: bool,
}
#[derive(Serialize)]
pub struct Report {
    system: System,
    counters: Counters,
    timings: Option<TimingSummary>,
}

// Facts come from the current owners; a desktop name alone never proves permission or readiness.
fn system(
    audio: Result<vults_audio::InputInfo, String>,
    session: &str,
    shortcuts_bound: bool,
    keyboard: &str,
    focus_ready: bool,
    gpu: bool,
    vad: bool,
) -> System {
    let (host, device, rate) = match audio {
        Ok(info) => (Some(info.host), Some(info.device), Some(info.rate)),
        Err(_) => (None, None, None),
    };
    System {
        host,
        device,
        rate,
        session: match session {
            "wayland" => "Wayland",
            "x11" => "X11",
            _ => "unknown",
        },
        shortcuts: if shortcuts_bound { "ready" } else { "unavailable" },
        keyboard: if session == "wayland" {
            keyboard.into()
        } else if session == "x11" {
            "not-needed".into()
        } else {
            "unavailable".into()
        },
        kwin: if session == "x11" {
            "not-needed"
        } else if focus_ready {
            "ready"
        } else {
            "unverified"
        },
        gpu,
        vad,
    }
}

#[tauri::command]
pub async fn dictation_doctor(app: AppHandle, window: tauri::Webview) -> Result<Report, String> {
    crate::settings_page(&window)?;
    tauri::async_runtime::spawn_blocking(move || {
        let device = app
            .state::<crate::settings::SettingsState>()
            .0
            .lock()
            .map_err(|_| "Could not read dictation diagnostics.")?
            .dictation_capture_options()
            .device;
        let bound = app
            .state::<crate::ShortcutKeys>()
            .0
            .lock()
            .is_ok_and(|keys| keys.get("talk").is_some_and(|key| !key.is_empty()));
        #[cfg(target_os = "linux")]
        let (keyboard, focus) = (
            crate::dictation::permission::status(app.clone()),
            app.try_state::<crate::dictation::permission::Insertion>()
                .is_some_and(|s| s.portal.focus_ready()),
        );
        #[cfg(not(target_os = "linux"))]
        let (keyboard, focus) = (String::from("unavailable"), false);
        Ok(Report {
            system: system(
                vults_audio::input_info(device.as_deref()),
                &std::env::var("XDG_SESSION_TYPE").unwrap_or_default(),
                bound,
                &keyboard,
                focus,
                vults_voice::gpu(),
                vults_voice::vad_path(&crate::voice::models_dir()).is_some(),
            ),
            counters: Counters {
                loops: LOOPS.load(Ordering::Relaxed),
                retries: RETRIES.load(Ordering::Relaxed),
                copied: COPIED.load(Ordering::Relaxed),
            },
            timings: crate::dictation_history::timings(&app).ok(),
        })
    })
    .await
    .map_err(|_| "Could not read dictation diagnostics.".to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn probes_do_not_infer_readiness_from_desktop_or_missing_audio() {
        let report = system(
            Err("private device error".into()),
            "wayland",
            false,
            "off",
            false,
            false,
            false,
        );
        assert_eq!((report.host, report.rate, report.device), (None, None, None));
        assert_eq!(
            (report.shortcuts, report.keyboard.as_str(), report.kwin),
            ("unavailable", "off", "unverified")
        );
        assert!(!report.gpu && !report.vad);
        let report = system(
            Ok(vults_audio::InputInfo {
                host: "ALSA".into(),
                device: "Mic".into(),
                rate: 48000,
            }),
            "wayland",
            true,
            "temporary",
            true,
            true,
            true,
        );
        assert_eq!(report.rate, Some(48000));
        assert_eq!(
            (report.shortcuts, report.keyboard.as_str(), report.kwin),
            ("ready", "temporary", "ready")
        );
        let report = system(Err(String::new()), "x11", false, "ready", true, false, true);
        assert_eq!(
            (report.keyboard.as_str(), report.kwin),
            ("not-needed", "not-needed")
        );
        assert_eq!(
            system(Err(String::new()), "", false, "off", false, false, false).session,
            "unknown"
        );
    }
}
