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
    /// Show what is playing on the island. Off until the user turns it on.
    #[serde(default)]
    pub now_playing: bool,
    /// The voice model the chat's mic uses; none keeps voice off.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub voice_model: Option<String>,
    /// What the user speaks: a code (`pt`), `auto` to detect it, absent to follow the system.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub voice_language: Option<String>,
    /// Zeca's species, a renderer id (`ui/src/character/flock/species.ts`); an unknown one draws
    /// the black vulture.
    #[serde(default = "zeca_species")]
    pub zeca_species: String,
    /// What Zeca wears: the calendar's look (`auto`), none, or one look for good.
    #[serde(default)]
    pub zeca_look: vultures_ai_core::looks::Outfit,
    /// Where the other sessions' birds are drawn from.
    #[serde(default)]
    pub flock: vultures_ai_core::flock::Flock,
    /// Now and then a vulture from outside the flock crosses the sky.
    #[serde(default = "yes")]
    pub visitors: bool,
}

fn zeca_species() -> String {
    "atratus".into()
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
            now_playing: false,
            voice_model: None,
            voice_language: None,
            zeca_species: zeca_species(),
            zeca_look: Default::default(),
            flock: Default::default(),
            visitors: true,
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
    #[serde(rename = "nowPlaying")]
    pub now_playing: bool,
    #[serde(rename = "zecaSpecies")]
    pub zeca_species: String,
    #[serde(rename = "zecaLook")]
    pub zeca_look: vultures_ai_core::looks::Outfit,
    pub flock: vultures_ai_core::flock::Flock,
    pub visitors: bool,
    /// Where the settings file and the app's data really are (XDG aware), `~` for $HOME.
    #[serde(rename = "settingsPath")]
    pub settings_path: String,
    #[serde(rename = "dataPath")]
    pub data_path: String,
}

#[tauri::command]
pub fn app_settings(app: AppHandle, state: tauri::State<'_, SettingsState>) -> Public {
    use tauri_plugin_autostart::ManagerExt;
    let s = state.0.lock().map(|s| s.clone()).unwrap_or_default();
    Public {
        sounds: s.sounds,
        monitor: s.monitor,
        now_playing: s.now_playing,
        fold_after: s.fold_after.clamp(*FOLD_AFTER.start(), *FOLD_AFTER.end()),
        zeca_species: s.zeca_species,
        zeca_look: s.zeca_look,
        flock: s.flock,
        visitors: s.visitors,
        settings_path: crate::paths::shown(&path()),
        data_path: crate::paths::shown(&crate::paths::data_dir()),
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
pub fn set_visitors(app: AppHandle, on: bool) -> Result<(), String> {
    edit(&app, |s| s.visitors = on)?;
    let _ = app.emit("settings", serde_json::json!({ "visitors": on }));
    Ok(())
}

/// Zeca's species: the island draws him as it, at once.
#[tauri::command]
pub fn set_zeca_species(app: AppHandle, id: String) -> Result<(), String> {
    edit(&app, |s| s.zeca_species = id.clone())?;
    let _ = app.emit("settings", serde_json::json!({ "zecaSpecies": id }));
    Ok(())
}

#[tauri::command]
pub fn set_now_playing(app: AppHandle, on: bool) -> Result<(), String> {
    edit(&app, |s| s.now_playing = on)?;
    crate::media::apply(&app, on);
    Ok(())
}

/// The language as chosen: a code, `auto`, or none to follow the system.
pub fn voice_language(app: &AppHandle) -> Option<String> {
    let state = app.state::<SettingsState>();
    state.0.lock().ok().and_then(|s| s.voice_language.clone())
}

pub fn voice_model(app: &AppHandle) -> Option<String> {
    let state = app.state::<SettingsState>();
    state.0.lock().ok().and_then(|s| s.voice_model.clone())
}

pub fn now_playing(app: &AppHandle) -> bool {
    let state = app.state::<SettingsState>();
    state.0.lock().map(|s| s.now_playing).unwrap_or(false)
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

/// A missing file means defaults; a newer version is read as far as we understand it. A file
/// we can't fully read is copied aside first: the next save writes only what was understood.
pub fn load() -> Settings {
    let path = path();
    let Ok(text) = std::fs::read_to_string(&path) else {
        return Settings::default();
    };
    let (settings, clean) = parse(&text);
    if !clean {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or_default();
        let aside = path.with_extension(format!("json.bad-{stamp}"));
        match std::fs::copy(&path, &aside) {
            Ok(_) => tracing::warn!(
                "some settings could not be read; the file was kept as {}",
                aside.display()
            ),
            Err(e) => tracing::warn!("some settings could not be read, and keeping a copy failed: {e}"),
        }
    }
    settings
}

/// The settings in `text`, and whether all of it was understood. A field that doesn't fit
/// (a wrong type, a value from another version) falls back to its default; the rest is kept.
fn parse(text: &str) -> (Settings, bool) {
    if let Ok(s) = serde_json::from_str(text) {
        return (s, true);
    }
    let Ok(serde_json::Value::Object(fields)) = serde_json::from_str(text) else {
        return (Settings::default(), false);
    };
    let mut kept = serde_json::to_value(Settings::default()).unwrap_or_default();
    for (key, value) in fields {
        let before = kept.get(&key).cloned();
        kept[&key] = value;
        if serde_json::from_value::<Settings>(kept.clone()).is_err() {
            match before {
                Some(v) => kept[&key] = v,
                None => {
                    kept.as_object_mut().map(|m| m.shift_remove(&key));
                }
            }
        }
    }
    (serde_json::from_value(kept).unwrap_or_default(), false)
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

#[cfg(test)]
mod tests {
    use super::*;

    const RULE: &str =
        r#"{ "agent": "claude", "cwd": "/home/me/p", "tool": "Bash", "target": "Bash · cargo test" }"#;

    #[test]
    fn one_bad_field_keeps_the_rest() {
        let text =
            format!(r#"{{ "version": 1, "rules": [{RULE}], "now_playing": "yes", "fold_after": 30 }}"#);
        let (s, clean) = parse(&text);
        assert!(!clean);
        assert_eq!(s.rules.len(), 1, "the always-allow rules survive a bad field");
        assert_eq!(s.fold_after, 30);
        assert!(!s.now_playing, "the bad field falls back to its default");
    }

    #[test]
    fn a_good_file_is_read_whole_and_garbage_is_defaults() {
        let (s, clean) = parse(&format!(
            r#"{{ "version": 1, "rules": [{RULE}], "now_playing": true }}"#
        ));
        assert!(clean);
        assert!(s.now_playing && s.rules.len() == 1);
        let (s, clean) = parse("{ not json");
        assert!(!clean);
        assert_eq!(s, Settings::default());
    }

    #[test]
    fn the_flock_and_zecas_species_are_read_and_default() {
        use vultures_ai_core::looks::Outfit;
        let (s, clean) = parse(r#"{ "version": 1, "zeca_species": "papa", "flock": "world" }"#);
        assert!(clean);
        assert_eq!(
            (s.zeca_species.as_str(), s.flock),
            ("papa", vultures_ai_core::flock::Flock::World)
        );
        let (s, _) = parse(r#"{ "version": 1, "flock": "mars" }"#);
        assert_eq!(
            (s.zeca_species.as_str(), s.flock),
            ("atratus", vultures_ai_core::flock::Flock::Brazil)
        );
        assert!(s.visitors, "rare visitors are on until the user turns them off");
        let (s, _) = parse(r#"{ "version": 1, "visitors": false }"#);
        assert!(!s.visitors);
        assert_eq!(s.zeca_look, Outfit::Auto, "the calendar's look by default");
        let (s, _) = parse(r#"{ "version": 1, "zeca_look": "witch-hat" }"#);
        assert_eq!(s.zeca_look, Outfit::WitchHat);
        // A look this version does not draw falls back to the calendar, and only that field does.
        let (s, _) = parse(r#"{ "version": 1, "zeca_look": "top-hat", "visitors": false }"#);
        assert_eq!((s.zeca_look, s.visitors), (Outfit::Auto, false));
    }
}
