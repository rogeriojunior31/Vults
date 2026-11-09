//! What is playing, on the island. The watcher runs only while the setting is on; off, nothing
//! reads track metadata and the island forgets the song. Independent dictation has its own
//! temporary volume/playback control in `platform::music`.

use std::sync::Mutex;

use tauri::{AppHandle, Emitter, Manager};
use vults_media::{Control, NowPlaying};

#[cfg(windows)]
pub(crate) mod windows_audio;

#[derive(Clone, Debug, serde::Serialize)]
pub struct AudioChoice {
    token: String,
    app: String,
    output: Option<String>,
    pid: u32,
    percent: u8,
}

#[tauri::command]
pub fn media_audio_status(app: AppHandle, window: tauri::Webview) -> Result<Option<AudioChoice>, String> {
    crate::settings_page(&window)?;
    #[cfg(windows)]
    return Ok(windows_audio::status(&app));
    #[cfg(not(windows))]
    {
        let _ = app;
        Ok(None)
    }
}

#[tauri::command]
pub async fn media_audio_choices(app: AppHandle, window: tauri::Webview) -> Result<Vec<AudioChoice>, String> {
    crate::settings_page(&window)?;
    #[cfg(windows)]
    return windows_audio::choices(app).await;
    #[cfg(not(windows))]
    {
        let _ = app;
        Err("Application audio selection requires Windows.".into())
    }
}

#[tauri::command]
pub async fn media_audio_select(
    app: AppHandle,
    window: tauri::Webview,
    token: Option<String>,
) -> Result<(), String> {
    crate::settings_page(&window)?;
    #[cfg(windows)]
    return windows_audio::select(app, token).await;
    #[cfg(not(windows))]
    {
        let _ = (app, token);
        Err("Application audio selection requires Windows.".into())
    }
}

#[derive(Debug, Default)]
pub struct MediaState {
    watcher: Mutex<Option<tauri::async_runtime::JoinHandle<()>>>,
    /// The song on screen: the controls go to its player, and an island that loads late asks
    /// for it (the watcher sends a song once, when it starts, and then only on a change).
    shown: Mutex<Option<NowPlaying>>,
    #[cfg(windows)]
    audio: Mutex<windows_audio::State>,
}

/// Starts or stops the watcher, from the setting.
pub fn apply(app: &AppHandle, on: bool) {
    let state = app.state::<MediaState>();
    let mut watcher = state.watcher.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(old) = watcher.take() {
        old.abort();
    }
    if !on {
        show(app, None);
    }
    #[cfg(any(target_os = "linux", windows))]
    if on {
        let app = app.clone();
        *watcher = Some(tauri::async_runtime::spawn(async move {
            let shown = app.clone();
            if let Err(e) = vults_media::watch(move |now| show(&shown, now)).await {
                show(&app, None);
                tracing::warn!("now playing stopped: {e}");
            }
        }));
    }
}

fn show(app: &AppHandle, now: Option<NowPlaying>) {
    let state = app.state::<MediaState>();
    #[cfg(windows)]
    if state
        .shown
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .as_ref()
        .map(|n| &n.player)
        != now.as_ref().map(|n| &n.player)
    {
        windows_audio::clear(app);
        let _ = app.emit("media-audio-cleared", ());
    }
    *state.shown.lock().unwrap_or_else(|e| e.into_inner()) = now.clone();
    // Once, to every page: the island and the Nest's header both show the song.
    let _ = app.emit("media", now);
}

#[tauri::command]
pub async fn media_control(app: AppHandle, action: Control) -> Result<(), String> {
    let player = app
        .state::<MediaState>()
        .shown
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .as_ref()
        .map(|n| n.player.clone());
    let Some(player) = player else {
        return Ok(());
    };
    #[cfg(any(target_os = "linux", windows))]
    return vults_media::control(&player, action).await;
    #[cfg(not(any(target_os = "linux", windows)))]
    {
        let _ = (player, action);
        Ok(())
    }
}

/// The song on screen now, for an island that loads after the watcher sent it.
#[tauri::command]
pub fn media_now(app: AppHandle) -> Option<NowPlaying> {
    app.state::<MediaState>()
        .shown
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
}
