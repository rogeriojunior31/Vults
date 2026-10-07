//! Zeca speaks his chat replies, when the user turns it on (Settings → Chat). Off by default; the
//! model downloads only after a click. Only a reply's text is spoken: never a permission card, an
//! error, or the code in it. The island stops him on a key, a click or closing the chat; a new
//! message, the talk shortcut and Zeca switched off stop him here.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use vultures_ai_chat::Delta;
use vultures_ai_speech::{Engine, Lang, Player, Speaker, Synth, VOICES, Voice};

use crate::settings::{self, SettingsState};
use crate::{ISLAND, paths};

#[derive(Debug, Default)]
pub struct SpeechState {
    /// Loaded while speech is on; dropping it frees the model.
    speaker: Mutex<Option<Arc<Speaker>>>,
    downloading: AtomicBool,
}

fn dir() -> PathBuf {
    paths::data_dir().join("speech")
}

fn installed() -> bool {
    vultures_ai_speech::installed(&dir())
}

/// Whether the user turned it on, and the voice chosen per language.
fn chosen(app: &AppHandle) -> (bool, BTreeMap<String, String>) {
    let state = app.state::<SettingsState>();
    state
        .0
        .lock()
        .map(|s| (s.speak, s.speak_voices.clone()))
        .unwrap_or_default()
}

#[derive(Serialize)]
pub struct SpeechStatus {
    /// The model and voices are on disk.
    installed: bool,
    downloading: bool,
    /// Bytes to download.
    size: u64,
    on: bool,
    voices: &'static [Voice],
    /// Language code → the voice id in use for it.
    chosen: BTreeMap<&'static str, &'static str>,
    /// espeak-ng is installed: Portuguese can be spoken.
    espeak: bool,
}

#[tauri::command]
pub fn speech_status(app: AppHandle) -> SpeechStatus {
    let (on, voices) = chosen(&app);
    SpeechStatus {
        installed: installed(),
        downloading: app.state::<SpeechState>().downloading.load(Ordering::SeqCst),
        size: vultures_ai_speech::size(),
        on: on && installed(),
        voices: VOICES,
        chosen: [Lang::En, Lang::Pt]
            .into_iter()
            .map(|l| {
                (
                    l.code(),
                    vultures_ai_speech::voice_for(l, voices.get(l.code()).map(String::as_str)).id,
                )
            })
            .collect(),
        espeak: vultures_ai_speech::espeak().is_some(),
    }
}

/// Downloads the model and voices (Settings' button), then turns speech on.
#[tauri::command]
pub async fn speech_download(app: AppHandle) -> Result<(), String> {
    if app
        .state::<SpeechState>()
        .downloading
        .swap(true, Ordering::SeqCst)
    {
        return Err("The speech model is already downloading.".into());
    }
    let progress = app.clone();
    let result = vultures_ai_speech::download(&dir(), move |done, total| {
        let _ = progress.emit(
            "speech-download",
            serde_json::json!({ "done": done, "total": total }),
        );
    })
    .await;
    app.state::<SpeechState>()
        .downloading
        .store(false, Ordering::SeqCst);
    result?;
    speech_set(app, true)
}

/// "Zeca speaks" on or off. On loads the model now, so the first reply is not kept waiting.
#[tauri::command]
pub fn speech_set(app: AppHandle, on: bool) -> Result<(), String> {
    if on && !installed() {
        return Err("Download the speech model first.".into());
    }
    settings::edit(&app, |s| s.speak = on)?;
    apply(&app);
    Ok(())
}

/// The voice for a language (`en`, `pt`).
#[tauri::command]
pub fn speech_voice_set(app: AppHandle, lang: String, voice: String) -> Result<(), String> {
    let lang = Lang::from_code(&lang);
    if !VOICES.iter().any(|v| v.id == voice && v.lang == lang) {
        return Err(format!("Unknown voice {voice}."));
    }
    settings::edit(&app, |s| {
        s.speak_voices.insert(lang.code().to_string(), voice.clone());
    })?;
    if let Some(speaker) = speaker(&app) {
        speaker.set_voice(lang, &voice);
    }
    Ok(())
}

/// The island: a key, a click, the chat closed.
#[tauri::command]
pub fn speech_stop(app: AppHandle) {
    stop(&app);
}

pub fn stop(app: &AppHandle) {
    if let Some(speaker) = speaker(app) {
        speaker.stop();
    }
}

fn speaker(app: &AppHandle) -> Option<Arc<Speaker>> {
    app.state::<SpeechState>()
        .speaker
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
}

/// Starts or drops the speaker to match the settings (at start-up too). Speech serves Zeca's chat
/// only (ADR 0010): with Zeca off it stays unloaded.
pub fn apply(app: &AppHandle) {
    let (on, voices) = chosen(app);
    let want = on && settings::zeca(app) && installed();
    let state = app.state::<SpeechState>();
    let mut slot = state.speaker.lock().unwrap_or_else(|e| e.into_inner());
    if !want {
        // Freeing the model may wait for a sentence being made: not on the caller's thread.
        if let Some(old) = slot.take() {
            old.stop();
            std::thread::spawn(move || drop(old));
        }
        return;
    }
    if slot.is_some() {
        return;
    }
    let island = app.clone();
    let speaker = Speaker::start(
        || {
            let engine = Engine::load(&dir(), None)?;
            let cancel = engine.canceller();
            Ok((
                Box::new(engine) as Box<dyn Synth>,
                Box::new(cancel) as Box<dyn Fn() + Send + Sync>,
            ))
        },
        Arc::new(Player::new()),
        move |speaking| {
            let _ = island.emit_to(ISLAND, "speech", speaking);
        },
    );
    for (lang, voice) in &voices {
        speaker.set_voice(Lang::from_code(lang), voice);
    }
    *slot = Some(Arc::new(speaker));
}

/// A chat turn starts: the last reply stops, and this one is spoken in the user's language
/// unless its words say otherwise.
pub fn begin(app: &AppHandle) {
    if let Some(speaker) = speaker(app) {
        let fallback = crate::voice::language(app).or_else(crate::voice::system_language);
        speaker.begin(fallback.map_or(Lang::En, |code| Lang::from_code(&code)));
    }
}

/// What the chat streams: only the reply's text is spoken.
pub fn hear(app: &AppHandle, delta: &Delta) {
    let Some(speaker) = speaker(app) else {
        return;
    };
    match delta {
        Delta::Text { text } => speaker.hear(text),
        Delta::Done => speaker.finish(),
        Delta::Stopped | Delta::Error { .. } => speaker.stop(),
        Delta::Permission { .. } => {}
    }
}
