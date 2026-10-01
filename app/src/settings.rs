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
}

fn yes() -> bool {
    true
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            version: VERSION,
            connectors: BTreeMap::new(),
            sounds: true,
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
}

#[tauri::command]
pub fn app_settings(state: tauri::State<'_, SettingsState>) -> Public {
    Public {
        sounds: state.0.lock().map(|s| s.sounds).unwrap_or(true),
    }
}

#[tauri::command]
pub fn set_sounds(app: AppHandle, on: bool) -> Result<(), String> {
    edit(&app, |s| s.sounds = on)?;
    let _ = app.emit("settings", Public { sounds: on });
    Ok(())
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
