//! Transcript retention is serialized with erase; an in-flight old recording cannot undo it.
use std::sync::{
    Mutex,
    atomic::{AtomicU64, Ordering},
};

use tauri::{AppHandle, Emitter, Manager};
#[cfg(any(target_os = "linux", test))]
use vults_store::dictations::Entry;
use vults_store::{Store, dictations::Retention};

static HISTORY: Mutex<()> = Mutex::new(());
static EPOCH: AtomicU64 = AtomicU64::new(0);
// Explicit meeting saves are independent of dictation retention changes and dictation-only erase.
static ARCHIVE_EPOCH: AtomicU64 = AtomicU64::new(0);

#[cfg(any(target_os = "linux", test))]
pub fn epoch() -> u64 {
    EPOCH.load(Ordering::SeqCst)
}

fn retention(value: &str) -> Option<Retention> {
    Some(match value {
        "0" => Retention::NotKept,
        "7" => Retention::Days7,
        "30" => Retention::Days30,
        "365" => Retention::Days365,
        "all" => Retention::Always,
        _ => return None,
    })
}

fn choice(app: &AppHandle) -> String {
    app.state::<crate::settings::SettingsState>()
        .0
        .lock()
        .map(|s| s.dictation_keep.clone())
        .unwrap_or_else(|_| "0".into())
}

pub fn clearing<T>(clear: impl FnOnce() -> std::io::Result<T>) -> std::io::Result<T> {
    let _guard = HISTORY
        .lock()
        .map_err(|_| std::io::Error::other("Dictation history is unavailable."))?;
    EPOCH.fetch_add(1, Ordering::SeqCst);
    ARCHIVE_EPOCH.fetch_add(1, Ordering::SeqCst);
    clear()
}

pub(crate) fn archive_epoch() -> u64 {
    ARCHIVE_EPOCH.load(Ordering::SeqCst)
}

/// A pending save cannot race global history erasure and restore its old text.
pub(crate) fn retaining_archive<T>(
    expected: u64,
    write: impl FnOnce() -> Result<T, String>,
) -> Result<T, String> {
    let _guard = HISTORY.lock().map_err(|_| "History is unavailable.")?;
    if archive_epoch() != expected {
        return Err("History changed. Review a new recording before saving.".into());
    }
    write()
}

#[cfg(target_os = "linux")]
pub fn start(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let Ok(_guard) = HISTORY.lock() else {
            return;
        };
        let keep = retention(&choice(&app)).unwrap_or(Retention::NotKept);
        if Store::open_dictations(&crate::history::dir(), keep, crate::history::unix_now()).is_err() {
            tracing::warn!("dictation history could not be pruned");
        }
    });
}

#[cfg(target_os = "linux")]
pub fn record(app: &AppHandle, entry: Entry, epoch: u64) {
    let Ok(_guard) = HISTORY.lock() else {
        return;
    };
    let keep = retention(&choice(app)).unwrap_or(Retention::NotKept);
    let result = save(&crate::history::dir(), &entry, keep, epoch);
    if result.is_err() {
        // Database errors can contain user-supplied values; never log the transcript.
        tracing::warn!("dictation history could not be saved");
    } else {
        let _ = app.emit("dictation-history", false);
    }
}

#[cfg(any(target_os = "linux", test))]
fn save(
    dir: &std::path::Path,
    entry: &Entry,
    keep: Retention,
    epoch: u64,
) -> Result<Option<i64>, vults_store::Error> {
    if epoch != self::epoch() {
        return Ok(None);
    }
    let now = crate::history::unix_now();
    let day = vults_core::looks::Date::of(entry.at, crate::history::local_offset(entry.at));
    Store::open_dictations(dir, keep, now)?.dictation_add(entry, day, keep, now)
}

#[tauri::command]
pub async fn dictation_history(app: AppHandle) -> Result<serde_json::Value, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = HISTORY.lock().map_err(|_| "Dictation history is unavailable.")?;
        let value = choice(&app);
        let keep = retention(&value).unwrap_or(Retention::NotKept);
        load(
            &crate::history::dir(),
            keep,
            &value,
            crate::history::unix_now(),
            &crate::history::today().iso(),
        )
        .map_err(|_| "Could not read dictation history.".to_string())
    })
    .await
    .map_err(|_| "Could not read dictation history.".to_string())?
}

#[tauri::command]
pub async fn dictation_statistics(
    period: vults_store::dictations::Period,
) -> Result<vults_store::dictations::Statistics, String> {
    tauri::async_runtime::spawn_blocking(move || {
        Store::open(&crate::history::dir())?.dictation_statistics(period, crate::history::today())
    })
    .await
    .map_err(|_| "Could not read dictation statistics.".to_string())?
    .map_err(|_| "Could not read dictation statistics.".to_string())
}

pub fn timings(app: &AppHandle) -> Result<vults_store::dictations::TimingSummary, String> {
    let _guard = HISTORY.lock().map_err(|_| "Dictation timings are unavailable.")?;
    let keep = retention(&choice(app)).unwrap_or(Retention::NotKept);
    Store::open_dictations(&crate::history::dir(), keep, crate::history::unix_now())
        .and_then(|store| store.dictation_timings())
        .map_err(|_| "Dictation timings are unavailable.".into())
}

fn load(
    dir: &std::path::Path,
    keep: Retention,
    value: &str,
    now: i64,
    today: &str,
) -> Result<serde_json::Value, vults_store::Error> {
    let store = Store::open_dictations(dir, keep, now)?;
    let rows = store
        .dictations(500)?
        .into_iter()
        .map(|(id, e)| {
            serde_json::json!({
                "id": id, "at": e.at, "app": e.app, "session": e.session, "heard": e.heard,
                "text": e.text, "style": e.style, "language": e.language, "outputLanguage": e.output_language, "secs": e.secs,
                "words": e.words, "fixes": e.fixes, "outcome": e.outcome,
                "timings": e.timings,
            })
        })
        .collect::<Vec<_>>();
    let day = store
        .dictation_days()?
        .remove(today)
        .map(|d| serde_json::json!({ "words": d.words, "secs": d.secs, "count": d.count }));
    Ok(
        serde_json::json!({ "retention": if retention(value).is_some() { value } else { "0" }, "rows": rows, "today": day, "timing_summary": store.dictation_timings()? }),
    )
}

#[tauri::command]
pub async fn set_dictation_keep(app: AppHandle, window: tauri::Webview, value: String) -> Result<(), String> {
    if window.label() != crate::SETTINGS {
        return Err("Dictation retention is changed in Settings.".into());
    }
    let keep = retention(&value).ok_or("Invalid dictation retention.")?;
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = HISTORY.lock().map_err(|_| "Dictation history is unavailable.")?;
        // Invalidate even if erasure fails; no earlier recording may repopulate the old choice.
        EPOCH.fetch_add(1, Ordering::SeqCst);
        let pruned = Store::open_dictations(&crate::history::dir(), keep, crate::history::unix_now());
        let _ = app.emit("dictation-history", true);
        pruned.map_err(|_| "Could not erase old dictation text. Retention was not changed.")?;
        {
            let state = app.state::<crate::settings::SettingsState>();
            let mut saved = state.0.lock().map_err(|_| "Settings are unavailable.")?;
            let mut next = saved.clone();
            next.dictation_keep = value.clone();
            crate::settings::save(&next).map_err(|_| "Could not save dictation retention.")?;
            *saved = next;
        }
        let _ = app.emit("settings", serde_json::json!({ "dictationKeep": value }));
        Ok(())
    })
    .await
    .map_err(|_| "Could not change dictation retention.".to_string())?
}

#[tauri::command]
pub async fn dictation_delete(app: AppHandle, id: i64) -> Result<(), String> {
    erase(app, Some(id)).await
}

#[tauri::command]
pub async fn dictation_clear(app: AppHandle) -> Result<(), String> {
    erase(app, None).await
}

async fn erase(app: AppHandle, id: Option<i64>) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = HISTORY.lock().map_err(|_| "Dictation history is unavailable.")?;
        if id.is_none() {
            EPOCH.fetch_add(1, Ordering::SeqCst);
        }
        let mut store =
            Store::open(&crate::history::dir()).map_err(|_| "Could not erase dictation history.")?;
        let result = match id {
            Some(id) => store.dictation_delete(id),
            None => store.dictations_clear(),
        };
        let _ = app.emit("dictation-history", true);
        result.map_err(|_| "Could not erase dictation history.")?;
        Ok(())
    })
    .await
    .map_err(|_| "Could not erase dictation history.".to_string())?
}

#[tauri::command]
pub async fn dictation_paste_again(app: AppHandle, id: i64) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = HISTORY.lock().map_err(|_| "Dictation history is unavailable.")?;
        let keep = retention(&choice(&app)).unwrap_or(Retention::NotKept);
        let store = Store::open_dictations(&crate::history::dir(), keep, crate::history::unix_now())
            .map_err(|_| "Could not read dictation history.")?;
        let entry = store
            .dictation(id)
            .map_err(|_| "Could not read dictation history.")?
            .ok_or("This dictation is no longer in history.")?;
        let text = replay_text(entry)?;
        #[cfg(target_os = "linux")]
        {
            if crate::lock::locked(&app) {
                return Err("Unlock the screen before copying dictation.".into());
            }
            app.try_state::<crate::dictation::State>()
                .ok_or("The dictation worker is unavailable.")?
                .copy_again(Some(text))
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = text;
            Err("Dictation is available on Linux.".into())
        }
    })
    .await
    .map_err(|_| "Could not copy dictation again.".to_string())?
}

fn replay_text(entry: vults_store::dictations::Entry) -> Result<String, String> {
    if entry.outcome == "command" {
        Err("Voice commands cannot be pasted from history.".into())
    } else {
        Ok(entry.text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_history_is_rejected_before_any_replay_worker_or_clipboard_call() {
        let mut entry = Entry {
            at: 0,
            app: None,
            session: None,
            heard: "mute api".into(),
            text: "Command completed.".into(),
            style: "command".into(),
            language: None,
            output_language: None,
            secs: 1.0,
            words: 2,
            fixes: None,
            outcome: "command".into(),
            timings: Default::default(),
        };
        assert!(replay_text(entry.clone()).is_err());
        entry.outcome = "copied".into();
        assert_eq!(
            replay_text(entry).expect("Regular dictation"),
            "Command completed."
        );
    }
    #[test]
    fn reads_real_totals_hides_expired_text_and_an_erase_invalidates_old_capture() {
        let dir = std::env::temp_dir().join(format!("vults-dictation-history-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let at = crate::history::unix_now();
        let today = crate::history::today().iso();
        let entry = Entry {
            at,
            app: Some("Editor".into()),
            session: None,
            heard: "heard words".into(),
            text: "written words".into(),
            style: "verbatim".into(),
            language: None,
            output_language: None,
            secs: 2.0,
            words: 2,
            fixes: None,
            outcome: "copied".into(),
            timings: Default::default(),
        };
        let before = {
            let _guard = HISTORY.lock().expect("history lock");
            let before = epoch();
            save(&dir, &entry, Retention::Days30, before).expect("record");
            before
        };
        let loaded = load(&dir, Retention::Days30, "30", at, &today).expect("load");
        assert_eq!(loaded["rows"][0]["text"], "written words");
        assert_eq!(loaded["rows"][0]["fixes"], serde_json::Value::Null);
        assert_eq!(loaded["rows"][0]["timings"]["total_ms"], serde_json::Value::Null);
        assert_eq!(loaded["timing_summary"]["total"]["count"], 0);
        assert_eq!(loaded["today"]["words"], 2);
        clearing(|| {
            Store::open(&dir)
                .and_then(|mut s| s.dictations_clear())
                .map_err(std::io::Error::other)
        })
        .expect("clear");
        let _guard = HISTORY.lock().expect("history lock");
        assert!(
            save(&dir, &entry, Retention::Always, before)
                .expect("stale record")
                .is_none()
        );
        let cleared = load(&dir, Retention::Days30, "30", at, &today).expect("read cleared");
        assert_eq!(cleared["rows"], serde_json::json!([]));
        assert_eq!(cleared["today"]["words"], 2);
        save(&dir, &entry, Retention::NotKept, epoch()).expect("not kept");
        let private = load(&dir, Retention::NotKept, "0", at, &today).expect("read private");
        assert_eq!(private["rows"], serde_json::json!([]));
        assert_eq!(private["today"]["words"], 4);
        assert!(retention("bad").is_none());
        std::fs::remove_dir_all(dir).expect("test cleanup");
    }
}

#[cfg(test)]
mod archive_tests {
    #[test]
    fn global_erase_rejects_old_archive_writes_before_they_touch_storage() {
        let before = super::archive_epoch();
        super::clearing(|| Ok(())).unwrap();
        let mut called = false;
        let result = super::retaining_archive(before, || {
            called = true;
            Ok(())
        });
        assert!(result.is_err());
        assert!(!called);
    }
}
