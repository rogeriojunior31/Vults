//! Push-to-talk for the chat. Off until the user downloads a model in Settings; then the island's
//! mic records into memory and whisper.cpp transcribes it here. The text goes into the chat's
//! input for the user to read before sending.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use vultures_ai_voice::{Language, MODELS, Recorder, Transcriber};

use crate::{ISLAND, paths, settings};

#[derive(Debug, Default)]
pub struct VoiceState {
    recording: Mutex<Option<Recorder>>,
    /// The loaded model, by id: loading it again for every question would cost more than the
    /// transcription.
    loaded: Mutex<Option<(String, Arc<Transcriber>)>>,
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
    /// A model is chosen and on disk: the island shows the mic.
    ready: bool,
    language: Language,
}

pub fn ready(app: &AppHandle) -> bool {
    let selected = settings::voice_model(app);
    selected.is_some_and(|id| vultures_ai_voice::installed(&models_dir()).contains(&id.as_str()))
}

#[tauri::command]
pub fn voice_status(app: AppHandle) -> VoiceStatus {
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
        ready: ready(&app),
        language: settings::voice_language(&app),
    }
}

#[tauri::command]
pub fn voice_language(app: AppHandle, language: Language) -> Result<(), String> {
    settings::edit(&app, |s| s.voice_language = Some(language))
}

/// Downloads a model (Settings' button), then uses it.
#[tauri::command]
pub async fn voice_download(app: AppHandle, id: String) -> Result<(), String> {
    let progress = app.clone();
    let model = id.clone();
    vultures_ai_voice::download(&models_dir(), &id, move |done, total| {
        let _ = progress.emit(
            "voice-download",
            serde_json::json!({ "id": model, "done": done, "total": total }),
        );
    })
    .await?;
    voice_select(app, id)
}

#[tauri::command]
pub fn voice_select(app: AppHandle, id: String) -> Result<(), String> {
    settings::edit(&app, |s| s.voice_model = Some(id))?;
    announce(&app);
    Ok(())
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

#[tauri::command]
pub fn voice_start(app: AppHandle) -> Result<(), String> {
    if !ready(&app) {
        return Err("Choose a voice model in Settings → Chat first.".into());
    }
    let levels = app.clone();
    let recorder = Recorder::start(move |level| {
        let _ = levels.emit_to(ISLAND, "voice-level", level);
    })?;
    // A second start replaces the first recording, which is dropped (and stops).
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
    let language = settings::voice_language(&app);
    // Seconds of CPU: off the async workers.
    tauri::async_runtime::spawn_blocking(move || {
        let pcm = recorder.finish()?;
        transcriber.transcribe(&pcm, language)
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
    let mut loaded = state.loaded.lock().unwrap_or_else(|e| e.into_inner());
    if let Some((have, t)) = loaded.as_ref()
        && have == id
    {
        return Ok(t.clone());
    }
    let model = vultures_ai_voice::model(id).ok_or("unknown voice model")?;
    let t = Arc::new(Transcriber::load(&vultures_ai_voice::model_path(
        &models_dir(),
        model,
    ))?);
    *loaded = Some((id.to_string(), t.clone()));
    Ok(t)
}
