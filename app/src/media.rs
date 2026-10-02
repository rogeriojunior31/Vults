//! What is playing, on the island. The watcher runs only while the setting is on; off, nothing
//! reads the bus and the island forgets the song.

use std::sync::Mutex;

use tauri::{AppHandle, Emitter, Manager};
use vultures_ai_media::{Control, NowPlaying};

use crate::ISLAND;

#[derive(Debug, Default)]
pub struct MediaState {
    watcher: Mutex<Option<tauri::async_runtime::JoinHandle<()>>>,
    /// The player on screen: the controls go to it.
    shown: Mutex<Option<String>>,
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
    #[cfg(target_os = "linux")]
    if on {
        let app = app.clone();
        *watcher = Some(tauri::async_runtime::spawn(async move {
            let shown = app.clone();
            if let Err(e) = vultures_ai_media::watch(move |now| show(&shown, now)).await {
                tracing::warn!("now playing stopped: {e}");
            }
        }));
    }
}

fn show(app: &AppHandle, now: Option<NowPlaying>) {
    let state = app.state::<MediaState>();
    *state.shown.lock().unwrap_or_else(|e| e.into_inner()) = now.as_ref().map(|n| n.player.clone());
    let _ = app.emit_to(ISLAND, "media", now);
}

#[tauri::command]
pub async fn media_control(app: AppHandle, action: Control) -> Result<(), String> {
    let player = app
        .state::<MediaState>()
        .shown
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone();
    let Some(player) = player else {
        return Ok(());
    };
    #[cfg(target_os = "linux")]
    return vultures_ai_media::control(&player, action).await;
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (player, action);
        Ok(())
    }
}
