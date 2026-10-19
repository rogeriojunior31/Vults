//! Explicit text-only archive actions. Recording and summary completion never save implicitly.
use std::path::Path;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, Runtime, Webview};
use vults_store::{
    Store,
    meetings::{Entry, Meeting, Source},
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
pub async fn meeting_archive_rename<R: Runtime>(
    app: AppHandle<R>,
    window: Webview<R>,
    id: String,
    source: Source,
    speaker: u32,
    name: Option<String>,
) -> Result<bool, String> {
    access(&app, &window)?;
    let epoch = crate::dictation_history::archive_epoch();
    let dir = crate::history::dir();
    let worker = app.clone();
    let renamed = tauri::async_runtime::spawn_blocking(move || {
        crate::dictation_history::retaining_archive(epoch, || {
            if crate::lock::locked(&worker) {
                return Err(UNAVAILABLE.into());
            }
            Store::open(&dir)
                .and_then(|s| s.rename_meeting_speaker(&id, source, speaker, name.as_deref()))
                .map_err(|_| "Could not rename this voice. Use a name of at most 120 bytes.".into())
        })
    })
    .await
    .map_err(|_| UNAVAILABLE.to_owned())??;
    unchanged(&app, epoch)?;
    Ok(renamed)
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Format {
    Markdown,
    Srt,
    Vtt,
}
impl Format {
    fn extension(self) -> &'static str {
        match self {
            Self::Markdown => "md",
            Self::Srt => "srt",
            Self::Vtt => "vtt",
        }
    }
}

// One native chooser at a time, including calls from a recreated page.
static EXPORTING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
struct Exporting;
impl Drop for Exporting {
    fn drop(&mut self) {
        EXPORTING.store(false, std::sync::atomic::Ordering::SeqCst);
    }
}

#[tauri::command]
pub async fn meeting_archive_export<R: Runtime>(
    app: AppHandle<R>,
    window: Webview<R>,
    id: String,
    format: Format,
    source: Option<Source>,
    title: String,
) -> Result<bool, String> {
    use tauri_plugin_dialog::DialogExt;
    access(&app, &window)?;
    if !matches!(format, Format::Markdown) && source.is_none() {
        return Err("Choose one audio source for subtitle export.".into());
    }
    if EXPORTING.swap(true, std::sync::atomic::Ordering::SeqCst) {
        return Err("A meeting export dialog is already open.".into());
    }
    let guard = Exporting;
    let epoch = crate::dictation_history::archive_epoch();
    let dir = crate::history::dir();
    let language = app
        .state::<crate::settings::SettingsState>()
        .0
        .lock()
        .map_err(|_| UNAVAILABLE.to_owned())?
        .language
        .clone();
    let lang = crate::settings::resolve(language.as_deref());
    let title: String = title.chars().filter(|c| !c.is_control()).take(80).collect();
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = guard;
        let path = app
            .dialog()
            .file()
            .set_parent(&window.window())
            .set_title(title)
            .set_file_name(format!("vults-meeting.{}", format.extension()))
            .add_filter(format.extension().to_uppercase(), &[format.extension()])
            .blocking_save_file();
        let Some(path) = path else {
            return Ok(false);
        };
        let path = path
            .into_path()
            .map_err(|_| "Choose a local export file.".to_owned())?;
        crate::dictation_history::retaining_archive(epoch, || {
            access(&app, &window)?;
            if !window.window().is_visible().unwrap_or(false)
                || window.window().is_minimized().unwrap_or(true)
            {
                return Err(UNAVAILABLE.into());
            }
            export_to(&dir, &path, &id, format, source, lang)?;
            Ok(true)
        })
    })
    .await
    .map_err(|_| "Could not export this meeting.".to_owned())?
}

fn export_to(
    dir: &Path,
    path: &Path,
    id: &str,
    format: Format,
    source: Option<Source>,
    lang: vults_core::i18n::Lang,
) -> Result<(), String> {
    use vults_store::meetings::export::{self, Subtitle};
    let meeting = Store::open(dir)
        .and_then(|mut s| s.meeting(id))
        .map_err(|_| UNAVAILABLE.to_owned())?
        .ok_or(UNAVAILABLE)?;
    let text = match format {
        Format::Markdown => export::markdown(&meeting, lang),
        Format::Srt | Format::Vtt => export::subtitles(
            &meeting,
            source.ok_or("Choose one audio source for subtitle export.")?,
            if matches!(format, Format::Srt) {
                Subtitle::Srt
            } else {
                Subtitle::Vtt
            },
            lang,
        ),
    }
    .map_err(|_| "Could not prepare this meeting export.".to_owned())?;
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    use std::io::Write;
    options
        .open(path)
        .and_then(|mut file| file.write_all(text.as_bytes()))
        .map_err(|_| "Could not write this meeting export.".to_owned())
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
