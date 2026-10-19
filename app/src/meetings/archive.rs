//! Explicit text-only archive actions. Recording and summary completion never save implicitly.
use std::path::Path;
use std::sync::Arc;

use serde::Serialize;
use tauri::{AppHandle, Manager, Runtime, Webview};
use vults_store::{
    Store,
    meetings::{Entry, Meeting},
};

use super::State;

const UNAVAILABLE: &str = "Saved meetings are unavailable.";
#[cfg(target_os = "linux")]
const CURRENT: &str = "Finish the current recording before saving this meeting.";

fn access<R: Runtime>(app: &AppHandle<R>, window: &Webview<R>) -> Result<(), String> {
    super::authorized(window.label())?;
    if crate::lock::locked(app) {
        return Err("Unlock the screen before accessing saved meetings.".into());
    }
    Ok(())
}

fn unchanged<R: Runtime>(app: &AppHandle<R>, epoch: u64) -> Result<(), String> {
    if crate::lock::locked(app) || epoch != crate::dictation_history::archive_epoch() {
        return Err("Saved meeting access changed. Reload to try again.".into());
    }
    Ok(())
}

#[derive(Debug, Serialize)]
pub struct List {
    epoch: u64,
    rows: Vec<Entry>,
}

#[derive(Debug, Serialize)]
pub struct Detail {
    epoch: u64,
    meeting: Option<Meeting>,
}

#[tauri::command]
pub async fn meeting_archive_list<R: Runtime>(
    app: AppHandle<R>,
    window: Webview<R>,
    limit: usize,
) -> Result<List, String> {
    access(&app, &window)?;
    let epoch = crate::dictation_history::archive_epoch();
    let dir = crate::history::dir();
    let rows = tauri::async_runtime::spawn_blocking(move || {
        crate::dictation_history::retaining_archive(epoch, || {
            Store::open(&dir)
                .and_then(|s| s.meetings(limit))
                .map_err(|_| UNAVAILABLE.to_owned())
        })
    })
    .await
    .map_err(|_| UNAVAILABLE.to_owned())??;
    unchanged(&app, epoch)?;
    Ok(List { epoch, rows })
}

#[tauri::command]
pub async fn meeting_archive_read<R: Runtime>(
    app: AppHandle<R>,
    window: Webview<R>,
    id: String,
) -> Result<Detail, String> {
    access(&app, &window)?;
    let epoch = crate::dictation_history::archive_epoch();
    let dir = crate::history::dir();
    let meeting = tauri::async_runtime::spawn_blocking(move || {
        crate::dictation_history::retaining_archive(epoch, || {
            Store::open(&dir)
                .and_then(|mut s| s.meeting(&id))
                .map_err(|_| UNAVAILABLE.to_owned())
        })
    })
    .await
    .map_err(|_| UNAVAILABLE.to_owned())??;
    unchanged(&app, epoch)?;
    Ok(Detail { epoch, meeting })
}

#[tauri::command]
pub async fn meeting_archive_delete<R: Runtime>(
    app: AppHandle<R>,
    window: Webview<R>,
    id: String,
) -> Result<bool, String> {
    access(&app, &window)?;
    let epoch = crate::dictation_history::archive_epoch();
    let state = app.state::<Arc<State>>().inner().clone();
    let dir = crate::history::dir();
    let worker = app.clone();
    let deleted = tauri::async_runtime::spawn_blocking(move || {
        crate::dictation_history::retaining_archive(epoch, || {
            if crate::lock::locked(&worker) {
                return Err(UNAVAILABLE.into());
            }
            let mut slot = state.slot.lock().map_err(|_| UNAVAILABLE.to_owned())?;
            delete_at(&dir, &mut slot, &id)
        })
    })
    .await
    .map_err(|_| UNAVAILABLE.to_owned())??;
    unchanged(&app, epoch)?;
    Ok(deleted)
}

fn delete_at(dir: &Path, slot: &mut super::Slot, id: &str) -> Result<bool, String> {
    #[cfg(target_os = "linux")]
    if slot.archive.as_ref().is_some_and(|capture| capture.id == id) {
        // Revoke before IO: compaction can fail after deletion already committed.
        slot.archive = None;
    }
    #[cfg(not(target_os = "linux"))]
    let _ = slot;
    Store::open(dir)
        .and_then(|mut s| s.delete_meeting(id))
        .map_err(|_| "Could not delete this meeting.".into())
}

#[tauri::command]
pub async fn meeting_save<R: Runtime>(
    app: AppHandle<R>,
    window: Webview<R>,
    id: u64,
    title: String,
) -> Result<String, String> {
    access(&app, &window)?;
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (id, title);
        Err("Saving a recording is available on Linux only.".into())
    }
    #[cfg(target_os = "linux")]
    {
        let state = app.state::<Arc<State>>().inner().clone();
        let epoch = {
            let slot = state.slot.lock().map_err(|_| UNAVAILABLE.to_owned())?;
            if slot.view.id != id || slot.view.phase != "finished" {
                return Err(CURRENT.into());
            }
            slot.archive.as_ref().ok_or(CURRENT)?.epoch
        };
        let dir = crate::history::dir();
        let worker = app.clone();
        let saved = tauri::async_runtime::spawn_blocking(move || {
            crate::dictation_history::retaining_archive(epoch, || {
                if crate::lock::locked(&worker) {
                    return Err(UNAVAILABLE.into());
                }
                let mut slot = state.slot.lock().map_err(|_| UNAVAILABLE.to_owned())?;
                let settings = worker.state::<crate::settings::SettingsState>();
                let notes_enabled = settings
                    .0
                    .lock()
                    .map_err(|_| UNAVAILABLE.to_owned())?
                    .meeting_summary;
                save_at(&dir, &mut slot, id, &title, epoch, notes_enabled)
            })
        })
        .await
        .map_err(|_| "Could not save this meeting.".to_owned())??;
        unchanged(&app, epoch)?;
        Ok(saved)
    }
}

#[cfg(target_os = "linux")]
#[derive(Debug)]
pub(super) struct Capture {
    id: String,
    recorded_ms: i64,
    epoch: u64,
    saved: Option<Meeting>,
}

#[cfg(target_os = "linux")]
impl Capture {
    pub(super) fn new() -> Self {
        Self {
            id: crate::traces::new_id(),
            recorded_ms: crate::traces::now_ms(),
            epoch: crate::dictation_history::archive_epoch(),
            saved: None,
        }
    }

    pub(super) fn started(&mut self) {
        self.recorded_ms = crate::traces::now_ms();
    }

    fn freeze(
        &mut self,
        transcript: &super::transcription::Transcript,
        title: &str,
        notes: Option<vults_chat::meeting_summary::Summary>,
    ) -> Result<Meeting, String> {
        if let Some(saved) = &self.saved {
            return Ok(saved.clone());
        }
        use vults_store::meetings::{Notes, Source, Speaker, Todo};
        let microphone = transcript.microphone.archived()?;
        let system = transcript.system.archived()?;
        let mut speakers = Vec::new();
        for (source, track) in [(Source::Microphone, &microphone), (Source::System, &system)] {
            let ids = track
                .segments
                .iter()
                .filter_map(|s| s.speaker)
                .collect::<std::collections::BTreeSet<_>>();
            speakers.extend(ids.into_iter().map(|id| Speaker {
                source,
                id,
                name: None,
            }));
        }
        let notes = notes.map(|n| Notes {
            title: n.title,
            summary: n.summary,
            key_points: n.key_points,
            todos: n
                .todos
                .into_iter()
                .map(|t| Todo {
                    text: t.text,
                    owner: t.owner,
                    due: t.due,
                })
                .collect(),
        });
        let saved = Meeting {
            id: self.id.clone(),
            recorded_ms: self.recorded_ms,
            title: title.into(),
            microphone,
            system,
            speakers,
            notes,
        };
        saved
            .validate()
            .map_err(|_| "The meeting text or title could not be saved.".to_owned())?;
        self.saved = Some(saved.clone());
        Ok(saved)
    }
}

#[cfg(target_os = "linux")]
// The caller holds both the erase guard and recording slot until the snapshot commits.
fn save_at(
    dir: &Path,
    slot: &mut super::Slot,
    id: u64,
    title: &str,
    epoch: u64,
    notes_enabled: bool,
) -> Result<String, String> {
    if slot.view.id != id || slot.view.phase != "finished" || !slot.view.has_transcript {
        return Err(CURRENT.into());
    }
    let capture = slot
        .archive
        .as_mut()
        .filter(|c| c.epoch == epoch)
        .ok_or("This recording was discarded or its history was erased.")?;
    let transcript = slot.transcript.as_ref().filter(|t| t.id == id).ok_or(CURRENT)?;
    let notes = if capture.saved.is_some() {
        None
    } else {
        let notes = slot.summary.archive_notes()?;
        notes_enabled.then_some(notes).flatten()
    };
    let snapshot = capture.freeze(transcript, title, notes)?;
    Store::open(dir)
        .and_then(|mut s| s.save_meeting(&snapshot))
        .map_err(|_| "Could not save this meeting.".to_owned())?;
    slot.touched = std::time::Instant::now();
    Ok(snapshot.id)
}

#[cfg(all(test, target_os = "linux"))]
mod tests;
