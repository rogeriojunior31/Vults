//! Push-to-talk for the chat. Off until the user downloads a model in Settings; then the island's
//! mic records into memory and whisper.cpp transcribes it here. The text goes into the chat's
//! input for the user to read before sending.

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use vultures_ai_voice::{AutoStop, MODELS, Recorder, Transcriber, VAD};

use crate::{ISLAND, paths, settings};

#[derive(Debug, Default)]
pub struct VoiceState {
    recording: Mutex<Option<Recorder>>,
    /// The loaded model, by id: loading it again for every question would cost more than the
    /// transcription.
    loaded: Mutex<Option<(String, Arc<Transcriber>)>>,
    /// Held while a model loads (see `transcriber`).
    loading: Mutex<()>,
    /// Models downloading now: a second click on one must not write the same file twice.
    downloading: Mutex<HashSet<String>>,
}

/// What the user speaks, as whisper takes it: the setting, else the system's language.
/// None lets whisper detect it, which misses on short phrases.
fn language(app: &AppHandle) -> Option<String> {
    match settings::voice_language(app).as_deref() {
        Some("auto") => None,
        Some(code) => Some(code.to_string()),
        None => system_language(),
    }
}

fn system_language() -> Option<String> {
    ["LC_ALL", "LC_MESSAGES", "LANG"]
        .iter()
        .filter_map(|v| std::env::var(v).ok())
        .find(|v| !v.is_empty())
        .and_then(|v| language_of(&v))
}

/// `pt` from `pt_BR.UTF-8`; none for `C` / `POSIX`.
fn language_of(locale: &str) -> Option<String> {
    let code: String = locale.chars().take_while(char::is_ascii_alphabetic).collect();
    (code.len() == 2).then(|| code.to_lowercase())
}

#[cfg(test)]
mod tests {
    use super::language_of;

    #[test]
    fn a_locale_names_its_language() {
        assert_eq!(language_of("pt_BR.UTF-8").as_deref(), Some("pt"));
        assert_eq!(language_of("en_US").as_deref(), Some("en"));
        assert_eq!(language_of("C.UTF-8"), None);
        assert_eq!(language_of("POSIX"), None);
    }
}

fn models_dir() -> PathBuf {
    paths::data_dir().join("voice")
}

#[derive(Serialize)]
pub struct ModelView {
    id: &'static str,
    label: &'static str,
    size: u64,
    installed: bool,
}

#[derive(Serialize)]
pub struct VoiceStatus {
    models: Vec<ModelView>,
    selected: Option<String>,
    /// As chosen: a code, `auto`, or none (the system's).
    language: Option<String>,
    /// The system's language, to name the default.
    system: Option<String>,
    downloading: Vec<String>,
    /// A model is chosen and on disk: the island shows the mic.
    ready: bool,
}

pub fn ready(app: &AppHandle) -> bool {
    let selected = settings::voice_model(app);
    selected.is_some_and(|id| vultures_ai_voice::installed(&models_dir()).contains(&id.as_str()))
}

#[tauri::command]
pub fn voice_status(app: AppHandle) -> VoiceStatus {
    // The island asks this as it loads: voice already on from before the VAD gets it now.
    let ready = ready(&app);
    if ready {
        tauri::async_runtime::spawn(fetch_vad());
    }
    let installed = vultures_ai_voice::installed(&models_dir());
    VoiceStatus {
        models: MODELS
            .iter()
            .map(|m| ModelView {
                id: m.id,
                label: m.label,
                size: m.size,
                installed: installed.contains(&m.id),
            })
            .collect(),
        selected: settings::voice_model(&app),
        language: settings::voice_language(&app),
        system: system_language(),
        downloading: app
            .state::<VoiceState>()
            .downloading
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .cloned()
            .collect(),
        ready,
    }
}

#[tauri::command]
pub fn voice_language_set(app: AppHandle, language: Option<String>) -> Result<(), String> {
    settings::edit(&app, |s| s.voice_language = language)
}

/// Downloads a model (Settings' button). The first one is used at once; another waits for "Use".
#[tauri::command]
pub async fn voice_download(app: AppHandle, id: String) -> Result<(), String> {
    // The VAD downloads on its own (`fetch_vad`), never as a model to choose.
    if vultures_ai_voice::model(&id).is_none() {
        return Err(format!("unknown voice model {id}"));
    }
    let started = app
        .state::<VoiceState>()
        .downloading
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .insert(id.clone());
    if !started {
        return Err("This model is already downloading.".into());
    }
    let progress = app.clone();
    let model = id.clone();
    let result = vultures_ai_voice::download(&models_dir(), &id, move |done, total| {
        let _ = progress.emit(
            "voice-download",
            serde_json::json!({ "id": model, "done": done, "total": total }),
        );
    })
    .await;
    app.state::<VoiceState>()
        .downloading
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .remove(&id);
    result?;
    // No model ready yet: this one. Otherwise the user picks it with "Use".
    if !ready(&app) {
        return voice_select(app, id);
    }
    tauri::async_runtime::spawn(fetch_vad());
    Ok(())
}

#[tauri::command]
pub fn voice_select(app: AppHandle, id: String) -> Result<(), String> {
    settings::edit(&app, |s| s.voice_model = Some(id))?;
    announce(&app);
    tauri::async_runtime::spawn(fetch_vad());
    Ok(())
}

/// The Silero VAD (under 1 MB) comes with the whisper models: whenever voice is on and it is
/// missing (a model from before it, a fetch that failed offline), in the background. Until it lands
/// voice works as before: a loudness gate trims, and only a click stops the mic.
async fn fetch_vad() {
    static FETCHING: AtomicBool = AtomicBool::new(false);
    let dir = models_dir();
    if vultures_ai_voice::vad_path(&dir).is_some() || FETCHING.swap(true, Ordering::SeqCst) {
        return;
    }
    if let Err(e) = vultures_ai_voice::download(&dir, VAD.id, |_, _| {}).await {
        tracing::warn!("voice: the VAD model: {e}");
    }
    FETCHING.store(false, Ordering::SeqCst);
}

/// Turns voice off: the mic goes from the island; downloaded models stay on disk.
#[tauri::command]
pub fn voice_off(app: AppHandle) -> Result<(), String> {
    settings::edit(&app, |s| s.voice_model = None)?;
    *app.state::<VoiceState>()
        .loaded
        .lock()
        .unwrap_or_else(|e| e.into_inner()) = None;
    announce(&app);
    Ok(())
}

fn announce(app: &AppHandle) {
    let _ = app.emit("settings", serde_json::json!({ "voice": ready(app) }));
}

/// `tap`: started by a click, so it ends by itself when the user stops talking (`voice-silence`
/// tells the island, which stops it as a click would). Held down (the shortcut): only the key ends it.
#[tauri::command]
pub fn voice_start(app: AppHandle, tap: Option<bool>) -> Result<(), String> {
    // The mic serves Zeca's chat only (ADR 0010).
    if !crate::settings::zeca(&app) {
        return Err("Zeca is off (Settings → Flock).".into());
    }
    if !ready(&app) {
        return Err("Choose a voice model in Settings → Chat first.".into());
    }
    // Never waited on: this recording goes without it, the next one has it.
    tauri::async_runtime::spawn(fetch_vad());
    let auto = vultures_ai_voice::vad_path(&models_dir())
        .filter(|_| tap == Some(true))
        .map(|vad| {
            let island = app.clone();
            AutoStop {
                vad,
                ended: Box::new(move || {
                    let _ = island.emit_to(ISLAND, "voice-silence", ());
                }),
            }
        });
    // A recording still going is dropped first, so its preview never shows in this one.
    drop(
        app.state::<VoiceState>()
            .recording
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take(),
    );
    let levels = app.clone();
    let mut recorder = Recorder::start(
        move |level| {
            let _ = levels.emit_to(ISLAND, "voice-level", level);
        },
        auto,
    )?;
    // The text so far, dimmed, while the user speaks (on a GPU only); the final text replaces it.
    if let Some(id) = settings::voice_model(&app) {
        let (loader, island) = (app.clone(), app.clone());
        recorder.preview(
            move || transcriber(&loader, &id),
            language(&app),
            move |text| {
                let _ = island.emit_to(ISLAND, "voice-partial", text);
            },
        );
    }
    *app.state::<VoiceState>()
        .recording
        .lock()
        .unwrap_or_else(|e| e.into_inner()) = Some(recorder);
    Ok(())
}

/// Stops recording and returns what was said.
#[tauri::command]
pub async fn voice_stop(app: AppHandle) -> Result<String, String> {
    let recorder = app
        .state::<VoiceState>()
        .recording
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .take();
    let Some(recorder) = recorder else {
        return Ok(String::new());
    };
    let id = settings::voice_model(&app).ok_or("No voice model is chosen.")?;
    let transcriber = transcriber(&app, &id)?;
    let language = language(&app);
    let vad = vultures_ai_voice::vad_path(&models_dir());
    // Seconds of CPU: off the async workers.
    tauri::async_runtime::spawn_blocking(move || {
        let pcm = recorder.finish()?;
        transcriber.transcribe(&pcm, language.as_deref(), vad.as_deref())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn voice_cancel(app: AppHandle) {
    // Dropping the recorder stops it and throws the audio away.
    drop(
        app.state::<VoiceState>()
            .recording
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take(),
    );
}

fn transcriber(app: &AppHandle, id: &str) -> Result<Arc<Transcriber>, String> {
    let state = app.state::<VoiceState>();
    let cached = || {
        state
            .loaded
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
            .filter(|(have, _)| have == id)
            .map(|(_, t)| t.clone())
    };
    if let Some(t) = cached() {
        return Ok(t);
    }
    // One load at a time (the preview and the stop both ask), but never under `loaded`: voice_off
    // runs on the main thread and would wait seconds for a big model.
    let _loading = state.loading.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(t) = cached() {
        return Ok(t);
    }
    let model = vultures_ai_voice::model(id).ok_or("unknown voice model")?;
    let t = Arc::new(Transcriber::load(
        &vultures_ai_voice::model_path(&models_dir(), model),
        model.prompt,
    )?);
    *state.loaded.lock().unwrap_or_else(|e| e.into_inner()) = Some((id.to_string(), t.clone()));
    Ok(t)
}
