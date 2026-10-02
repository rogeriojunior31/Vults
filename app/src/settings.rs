//! The app's own settings, `~/.config/vultures-ai/settings.json`. Versioned so a later release
//! can migrate an older file instead of guessing.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};

const VERSION: u32 = 1;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Settings {
    pub version: u32,
    /// Connector id → enabled. Off until the user turns one on.
    #[serde(default)]
    pub connectors: BTreeMap<String, bool>,
    #[serde(default = "yes")]
    pub sounds: bool,
    /// Seconds the open island waits, once the pointer leaves, before folding.
    #[serde(default = "fold_after")]
    pub fold_after: u32,
    /// Permissions the user chose to always allow (exact tool and target, per project).
    #[serde(default)]
    pub rules: Vec<vultures_ai_core::Rule>,
    /// The monitor the island sits on, by maker and model; `None` lets the compositor choose.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub monitor: Option<String>,
    /// The API chat's provider (an id from `vultures_ai_chat::providers`).
    #[serde(default = "api_provider")]
    pub api_provider: String,
    /// Provider id → the model chosen for it. Keys never live here: only in the keyring.
    #[serde(default)]
    pub api_models: BTreeMap<String, String>,
}

fn api_provider() -> String {
    "anthropic".into()
}

fn yes() -> bool {
    true
}

/// The choices the settings offer; anything else in the file is brought into this range.
const FOLD_AFTER: std::ops::RangeInclusive<u32> = 5..=120;

fn fold_after() -> u32 {
    15
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            version: VERSION,
            connectors: BTreeMap::new(),
            sounds: true,
            fold_after: fold_after(),
            rules: Vec::new(),
            monitor: None,
            api_provider: api_provider(),
            api_models: BTreeMap::new(),
        }
    }
}

/// The settings in memory; every change is written through to disk.
#[derive(Debug)]
pub struct SettingsState(pub Mutex<Settings>);

/// What the windows may read.
#[derive(Serialize, Clone)]
pub struct Public {
    pub sounds: bool,
    /// Starts with the desktop session (an XDG autostart entry on Linux).
    pub autostart: bool,
    #[serde(rename = "foldAfter")]
    pub fold_after: u32,
    pub monitor: Option<String>,
}

#[tauri::command]
pub fn app_settings(app: AppHandle, state: tauri::State<'_, SettingsState>) -> Public {
    use tauri_plugin_autostart::ManagerExt;
    let (sounds, fold, monitor) = state
        .0
        .lock()
        .map(|s| (s.sounds, s.fold_after, s.monitor.clone()))
        .unwrap_or((true, fold_after(), None));
    Public {
        sounds,
        monitor,
        fold_after: fold.clamp(*FOLD_AFTER.start(), *FOLD_AFTER.end()),
        // The OS is the source of truth: the user may remove the entry by hand.
        autostart: app.autolaunch().is_enabled().unwrap_or(false),
    }
}

#[tauri::command]
pub fn set_autostart(app: AppHandle, on: bool) -> Result<(), String> {
    use tauri_plugin_autostart::ManagerExt;
    let launcher = app.autolaunch();
    if on { launcher.enable() } else { launcher.disable() }
        .map_err(|e| format!("can't change autostart: {e}"))
}

#[tauri::command]
pub fn set_sounds(app: AppHandle, on: bool) -> Result<(), String> {
    edit(&app, |s| s.sounds = on)?;
    let _ = app.emit("settings", serde_json::json!({ "sounds": on }));
    Ok(())
}

#[tauri::command]
pub fn set_fold_after(app: AppHandle, seconds: u32) -> Result<(), String> {
    let seconds = seconds.clamp(*FOLD_AFTER.start(), *FOLD_AFTER.end());
    edit(&app, |s| s.fold_after = seconds)?;
    let _ = app.emit("settings", serde_json::json!({ "foldAfter": seconds }));
    Ok(())
}

#[derive(Serialize, Clone)]
pub struct Monitor {
    /// What `set_monitor` takes.
    pub name: String,
    pub label: String,
}

/// The connected monitors. GTK is read on its own thread.
#[tauri::command]
pub async fn monitors(app: AppHandle) -> Vec<Monitor> {
    #[cfg(target_os = "linux")]
    {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let _ = app.run_on_main_thread(move || {
            let list = vultures_ai_platform::linux::monitor_names();
            let _ = tx.send(
                list.into_iter()
                    .map(|(name, label)| Monitor { name, label })
                    .collect(),
            );
        });
        rx.await.unwrap_or_default()
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = app;
        Vec::new()
    }
}

#[tauri::command]
pub fn set_monitor(app: AppHandle, name: Option<String>) -> Result<(), String> {
    edit(&app, |s| s.monitor = name)?;
    crate::place_island(&app);
    Ok(())
}

#[cfg(target_os = "linux")]
pub fn monitor(app: &AppHandle) -> Option<String> {
    let state = app.state::<SettingsState>();
    state.0.lock().ok().and_then(|s| s.monitor.clone())
}

/// Changes the settings and saves them.
pub fn edit(app: &AppHandle, change: impl FnOnce(&mut Settings)) -> Result<(), String> {
    let state = app.state::<SettingsState>();
    let mut s = state.0.lock().map_err(|_| "settings are busy")?;
    change(&mut s);
    save(&s).map_err(|e| format!("can't save the settings: {e}"))
}

fn path() -> PathBuf {
    let base = if cfg!(windows) {
        std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(crate::paths::home)
    } else {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| crate::paths::home().join(".config"))
    };
    base.join(if cfg!(windows) {
        vultures_ai_brand::NAME
    } else {
        vultures_ai_brand::SLUG
    })
    .join("settings.json")
}

/// A missing or unreadable file means defaults; a newer version is read as far as we understand it.
pub fn load() -> Settings {
    std::fs::read_to_string(path())
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

pub fn save(settings: &Settings) -> std::io::Result<()> {
    let path = path();
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let temp = path.with_extension("json.tmp");
    std::fs::write(&temp, serde_json::to_string_pretty(settings).unwrap_or_default())?;
    std::fs::rename(temp, path)
}
