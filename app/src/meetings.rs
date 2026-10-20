//! Explicit, page-owned recording and local transcription. Summaries require separate consent.
#![cfg_attr(not(target_os = "linux"), allow(dead_code))]

use serde::Serialize;
use std::sync::{
    Arc, Condvar, Mutex,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Manager};

pub(crate) mod archive;
pub(crate) mod summary;
#[cfg(target_os = "linux")]
mod transcription;

const SECONDS: u32 = 60;
const LEASE: Duration = Duration::from_secs(2);
const REVIEW: Duration = Duration::from_secs(60);

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    id: u64,
    phase: &'static str,
    elapsed_ms: u64,
    max_seconds: u32,
    microphone: Option<String>,
    monitor: Option<String>,
    fallback: bool,
    error: Option<&'static str>,
    model_id: Option<String>,
    language: Option<String>,
    has_transcript: bool,
    identification_available: bool,
    #[cfg(target_os = "linux")]
    progress: Option<transcription::Progress>,
}
impl Default for Snapshot {
    fn default() -> Self {
        Self {
            id: 0,
            phase: if cfg!(target_os = "linux") {
                "recovering"
            } else {
                "unsupported"
            },
            elapsed_ms: 0,
            max_seconds: SECONDS,
            microphone: None,
            monitor: None,
            fallback: false,
            error: None,
            model_id: None,
            language: None,
            has_transcript: false,
            identification_available: cfg!(all(target_os = "linux", feature = "meeting-speakers")),
            #[cfg(target_os = "linux")]
            progress: None,
        }
    }
}

#[derive(Debug)]
enum Action {
    Prepare(u64),
    Begin(u64, bool),
    End(u64, bool),
    Recover,
}
#[derive(Debug)]
struct Slot {
    view: Snapshot,
    action: Option<Action>,
    reviewed: Option<Instant>,
    touched: Instant,
    summary: summary::Slot,
    #[cfg(target_os = "linux")]
    transcript: Option<transcription::Transcript>,
    #[cfg(target_os = "linux")]
    archive: Option<archive::Capture>,
}
impl Default for Slot {
    fn default() -> Self {
        Self {
            view: Snapshot::default(),
            action: None,
            reviewed: None,
            touched: Instant::now(),
            summary: summary::Slot::default(),
            #[cfg(target_os = "linux")]
            transcript: None,
            #[cfg(target_os = "linux")]
            archive: None,
        }
    }
}
impl Slot {
    fn pending(&self) -> bool {
        matches!(
            self.view.phase,
            "recovering"
                | "preparing"
                | "starting"
                | "recording"
                | "stopping"
                | "transcribing"
                | "cancelling"
        )
    }
    fn end(&mut self, id: u64, cancel: bool) {
        if self.view.id != id {
            return;
        }
        if cancel {
            self.summary.discard();
            self.view.has_transcript = false;
            #[cfg(target_os = "linux")]
            {
                self.transcript = None;
                self.archive = None;
            }
            if matches!(self.view.phase, "ready" | "finished") {
                self.view.phase = "cancelled";
                self.reviewed = None;
                return;
            }
        }
        if !self.pending() || self.view.phase == "recovering" {
            return;
        }
        if self.view.phase == "transcribing" && !cancel {
            return;
        }
        let cancel = cancel || matches!(self.view.phase, "starting" | "preparing" | "cancelling");
        self.view.phase = if cancel { "cancelling" } else { "stopping" };
        self.reviewed = None;
        self.action = Some(Action::End(id, cancel));
    }
    #[cfg(target_os = "linux")]
    fn complete(
        &mut self,
        result: Result<vults_audio::meeting::Outcome, String>,
        completed: &mut transcription::Job,
    ) {
        use vults_audio::meeting::Outcome;
        let cancelled = self.view.phase == "cancelling";
        self.view.phase = match result {
            Ok(_) if cancelled => "cancelled",
            Ok(Outcome::Finished(_)) => "finished",
            Ok(Outcome::Cancelled) => "cancelled",
            Err(_) => "failed",
        };
        if self.view.phase == "finished" && completed.result.is_none() {
            self.view.phase = "failed";
            completed.failed = true;
        }
        self.view.error = (self.view.phase == "failed").then_some(if completed.failed {
            "transcription"
        } else {
            "capture"
        });
        let transcript = completed.result.take();
        if self.view.phase == "finished" {
            self.transcript = transcript;
        }
        self.view.has_transcript = self.transcript.is_some();
    }
    fn begin(&mut self, id: u64, consent: bool, identify: bool, now: Instant) -> Result<(), String> {
        if identify && !self.view.identification_available {
            return Err("Speaker identification is unavailable in this build.".into());
        }
        if !consent
            || self.view.model_id.is_none()
            || self.view.id != id
            || self.view.phase != "ready"
            || self
                .reviewed
                .is_none_or(|at| now.saturating_duration_since(at) > REVIEW)
        {
            return Err("Review the recording sources and confirm temporary audio capture first.".into());
        }
        self.reviewed = None;
        self.view.phase = "starting";
        #[cfg(target_os = "linux")]
        if let Some(capture) = self.archive.as_mut() {
            capture.started();
        }
        self.touched = now;
        self.action = Some(Action::Begin(id, identify));
        Ok(())
    }
}
#[derive(Debug, Default)]
pub struct State {
    slot: Mutex<Slot>,
    wake: Condvar,
    shutdown: AtomicBool,
    finished: AtomicBool,
}

fn authorized(label: &str) -> Result<(), String> {
    if label == crate::NEST {
        Ok(())
    } else {
        Err("Only the Nest can control meeting recording.".into())
    }
}

pub fn init(app: &AppHandle) {
    let state = Arc::new(State::default());
    app.manage(state.clone());
    #[cfg(target_os = "linux")]
    {
        let handle = app.clone();
        let shared = state.clone();
        if std::thread::Builder::new()
            .name("meeting-owner".into())
            .spawn(move || run(handle, shared))
            .is_err()
            && let Ok(mut slot) = state.slot.lock()
        {
            slot.view.phase = "failed";
            slot.view.error = Some("recovery");
        }
    }
}

pub fn cancel<R: tauri::Runtime>(app: &AppHandle<R>) {
    if let Some(state) = app.try_state::<Arc<State>>()
        && let Ok(mut slot) = state.slot.lock()
    {
        let id = slot.view.id;
        slot.end(id, true);
        state.wake.notify_one();
    }
}
pub fn shutdown(app: &AppHandle) {
    if let Some(state) = app.try_state::<Arc<State>>() {
        cancel(app);
        state.shutdown.store(true, Ordering::SeqCst);
        state.wake.notify_one();
        #[cfg(target_os = "linux")]
        {
            let deadline = Instant::now() + Duration::from_secs(3);
            while !state.finished.load(Ordering::SeqCst) && Instant::now() < deadline {
                std::thread::sleep(Duration::from_millis(10));
            }
        }
    }
}

#[tauri::command]
pub fn meeting_status(app: AppHandle, window: tauri::Webview, id: Option<u64>) -> Result<Snapshot, String> {
    authorized(window.label())?;
    let state = app.state::<Arc<State>>();
    let mut slot = state
        .slot
        .lock()
        .map_err(|_| "Meeting recording is unavailable.")?;
    if id == Some(slot.view.id) {
        slot.touched = Instant::now();
    }
    Ok(slot.view.clone())
}
#[tauri::command]
pub fn meeting_prepare(app: AppHandle, window: tauri::Webview) -> Result<Snapshot, String> {
    authorized(window.label())?;
    if crate::lock::locked(&app) {
        return Err("Unlock the screen before recording.".into());
    }
    let state = app.state::<Arc<State>>();
    let mut slot = state
        .slot
        .lock()
        .map_err(|_| "Meeting recording is unavailable.")?;
    if slot.pending() || slot.view.error == Some("recovery") || slot.view.phase == "unsupported" {
        return Err("Finish the current recording action first.".into());
    }
    slot.view.id += 1;
    slot.summary.discard();
    slot.view.phase = "preparing";
    slot.view.error = None;
    slot.view.elapsed_ms = 0;
    slot.view.microphone = None;
    slot.view.monitor = None;
    slot.view.model_id = None;
    slot.view.language = None;
    slot.view.has_transcript = false;
    #[cfg(target_os = "linux")]
    {
        slot.transcript = None;
        slot.archive = Some(archive::Capture::new());
        slot.view.progress = None;
    }
    slot.reviewed = None;
    slot.touched = Instant::now();
    slot.action = Some(Action::Prepare(slot.view.id));
    state.wake.notify_one();
    Ok(slot.view.clone())
}
#[tauri::command]
pub fn meeting_begin(
    app: AppHandle,
    window: tauri::Webview,
    id: u64,
    consent: bool,
    identify_speakers: Option<bool>,
) -> Result<Snapshot, String> {
    authorized(window.label())?;
    if crate::lock::locked(&app) {
        return Err("Unlock the screen before recording.".into());
    }
    let state = app.state::<Arc<State>>();
    let mut slot = state
        .slot
        .lock()
        .map_err(|_| "Meeting recording is unavailable.")?;
    if slot.view.model_id != crate::settings::voice_model(&app)
        || slot.view.language != crate::voice::language(&app)
    {
        return Err("The voice settings changed; review the recording again.".into());
    }
    let identify = identify_speakers.unwrap_or(false);
    slot.begin(id, consent, identify, Instant::now())?;
    state.wake.notify_one();
    Ok(slot.view.clone())
}
#[tauri::command]
pub fn meeting_end(
    app: AppHandle,
    window: tauri::Webview,
    id: u64,
    cancel: bool,
) -> Result<Snapshot, String> {
    authorized(window.label())?;
    let state = app.state::<Arc<State>>();
    let mut slot = state
        .slot
        .lock()
        .map_err(|_| "Meeting recording is unavailable.")?;
    slot.end(id, cancel);
    state.wake.notify_one();
    Ok(slot.view.clone())
}
#[tauri::command]
pub fn meeting_recover(app: AppHandle, window: tauri::Webview) -> Result<Snapshot, String> {
    authorized(window.label())?;
    let state = app.state::<Arc<State>>();
    let mut slot = state
        .slot
        .lock()
        .map_err(|_| "Meeting recording is unavailable.")?;
    if slot.pending() || slot.view.phase == "unsupported" {
        return Err("Finish the current recording action first.".into());
    }
    slot.view.phase = "recovering";
    slot.action = Some(Action::Recover);
    state.wake.notify_one();
    Ok(slot.view.clone())
}

#[cfg(target_os = "linux")]
#[tauri::command]
pub fn meeting_transcript(
    app: AppHandle,
    window: tauri::Webview,
    id: u64,
) -> Result<Option<transcription::Transcript>, String> {
    authorized(window.label())?;
    if crate::lock::locked(&app) {
        return Err("Unlock the screen before reading a transcript.".into());
    }
    let state = app.state::<Arc<State>>();
    let slot = state
        .slot
        .lock()
        .map_err(|_| "Meeting recording is unavailable.")?;
    Ok(slot
        .transcript
        .as_ref()
        .filter(|t| t.id == id && slot.view.phase == "finished")
        .cloned())
}
#[cfg(not(target_os = "linux"))]
#[tauri::command]
pub fn meeting_transcript(window: tauri::Webview, id: u64) -> Result<Option<()>, String> {
    authorized(window.label())?;
    let _ = id;
    Ok(None)
}

#[cfg(target_os = "linux")]
fn run(app: AppHandle, state: Arc<State>) {
    use vults_audio::meeting::{self, Phase, Recording};
    let root = crate::paths::data_dir().join("meeting-audio");
    let recover = || {
        let result = meeting::recover(&root);
        if let Ok(mut slot) = state.slot.lock() {
            slot.view.phase = if result.is_ok() { "idle" } else { "failed" };
            slot.view.error = result.err().map(|_| "recovery");
            slot.reviewed = None;
        }
    };
    recover();
    let mut recording: Option<Recording> = None;
    let mut job = Arc::new(Mutex::new(transcription::Job::default()));
    loop {
        let quitting = state.shutdown.load(Ordering::SeqCst);
        let action = {
            let Ok(mut slot) = state.slot.lock() else {
                break;
            };
            let changed = slot.pending()
                && slot.view.model_id.is_some()
                && (slot.view.model_id != crate::settings::voice_model(&app)
                    || slot.view.language != crate::voice::language(&app));
            if quitting || crate::lock::locked(&app) || slot.touched.elapsed() > LEASE || changed {
                let id = slot.view.id;
                slot.end(id, true);
            }
            slot.action.take()
        };
        match action {
            Some(Action::Prepare(id)) => {
                let model_id = crate::settings::voice_model(&app);
                let language = crate::voice::language(&app);
                let installed = vults_voice::installed(&crate::voice::models_dir());
                let available = model_id.as_deref().is_some_and(|id| installed.contains(&id));
                let result = if available {
                    meeting::sources()
                } else {
                    Err("meeting model unavailable".into())
                };
                if let Ok(mut slot) = state.slot.lock()
                    && slot.view.id == id
                    && slot.view.phase == "preparing"
                {
                    match result {
                        Ok(sources) => {
                            slot.view.microphone = Some(sources.microphone);
                            slot.view.monitor = Some(sources.monitor);
                            slot.view.fallback = sources.monitor_fallback;
                            slot.view.model_id = model_id;
                            slot.view.language = language;
                            slot.view.phase = "ready";
                            slot.reviewed = Some(Instant::now());
                        }
                        Err(_) => {
                            slot.view.phase = "failed";
                            slot.view.error = Some(if available { "sources" } else { "model" });
                        }
                    }
                }
            }
            Some(Action::Begin(id, identify_speakers)) => {
                let expected = state.slot.lock().ok().and_then(|slot| {
                    if slot.view.id != id || slot.view.phase != "starting" || crate::lock::locked(&app) {
                        return None;
                    }
                    Some((
                        meeting::Sources {
                            microphone: slot.view.microphone.clone()?,
                            monitor: slot.view.monitor.clone()?,
                            monitor_fallback: slot.view.fallback,
                        },
                        slot.view.model_id.clone()?,
                        slot.view.language.clone(),
                    ))
                });
                if let Some((sources, model, language)) = expected {
                    job = Arc::new(Mutex::new(transcription::Job::default()));
                    let shared = job.clone();
                    let handle = app.clone();
                    match Recording::start_processed(&root, SECONDS, sources, move |tracks, stop| {
                        transcription::run(
                            &handle,
                            id,
                            transcription::Options {
                                model_id: model,
                                language,
                                identify_speakers,
                            },
                            tracks,
                            stop,
                            &shared,
                        )
                    }) {
                        Ok(started) => recording = Some(started),
                        Err(_) => {
                            if let Ok(mut slot) = state.slot.lock() {
                                slot.view.phase = "failed";
                                slot.view.error = Some("capture");
                            }
                        }
                    }
                }
            }
            Some(Action::End(id, cancel)) => {
                if let Ok(mut slot) = state.slot.lock()
                    && slot.view.id == id
                {
                    if let Some(recording) = &recording {
                        if cancel {
                            recording.cancel();
                        } else {
                            recording.stop();
                        }
                    } else {
                        slot.view.phase = "cancelled";
                    }
                }
            }
            Some(Action::Recover) => recover(),
            None => {}
        }
        if let Some(current) = recording.as_mut() {
            let status = current.status();
            if let Ok(mut slot) = state.slot.lock() {
                slot.view.elapsed_ms = status.elapsed.as_millis().min(u128::from(u64::MAX)) as u64;
                let mut completed = job.lock().unwrap_or_else(|e| e.into_inner());
                slot.view.progress = completed.progress.clone();
                if matches!(slot.view.phase, "starting" | "recording" | "stopping") {
                    slot.view.phase = match status.phase {
                        Phase::Recording => "recording",
                        Phase::Processing => "transcribing",
                        Phase::Stopping => "stopping",
                        _ => "starting",
                    };
                }
                if let Some(result) = current.take_result() {
                    slot.complete(result, &mut completed);
                    recording = None;
                }
            }
        }
        if quitting && recording.is_none() {
            break;
        }
        if let Ok(slot) = state.slot.lock() {
            let delay = if recording.is_some() || slot.view.has_transcript || slot.view.phase == "ready" {
                Duration::from_millis(50)
            } else {
                Duration::from_secs(3600)
            };
            drop(state.wake.wait_timeout_while(slot, delay, |slot| {
                slot.action.is_none() && !state.shutdown.load(Ordering::SeqCst)
            }));
        }
    }
    state.finished.store(true, Ordering::SeqCst);
}

#[cfg(test)]
mod tests;
