//! The update notice (FF4, ADR 0019): asked for, at most daily, never an install. Once the user
//! said yes, the app looks at its releases list through the network gate about 30 s after launch
//! and then once a day; what it found waits in `update.json` for the island to show. Distro
//! packages build without the `update-notice` feature: then only `update_status` exists, saying
//! the notice is not available, and nothing here can reach the network.
pub mod rules;

use std::sync::Mutex;
#[cfg(feature = "update-notice")]
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(feature = "update-notice")]
use std::time::{Duration, Instant};

#[cfg(feature = "update-notice")]
use tauri::Emitter;
use tauri::{AppHandle, Manager};
#[cfg(feature = "update-notice")]
use vults_connectors::Gate;

#[cfg(any(feature = "update-notice", test))]
use rules::Release;
use rules::{Snooze, Version};

/// How often the loop wakes to see whether a check is due; the check itself is daily.
#[cfg(feature = "update-notice")]
const POLL: Duration = Duration::from_secs(3600);
const FILE: &str = "update.json";
/// Release notes are shown, not stored whole: a long changelog stays on the release page.
#[cfg(any(feature = "update-notice", test))]
const MAX_NOTES: usize = 4000;
/// Only a release page of the app's own repository is ever opened.
#[cfg(any(feature = "update-notice", test))]
const PAGES: &str = "https://github.com/rogeriojunior31/Vults/releases/";
#[cfg(feature = "update-notice")]
static WAKE: tokio::sync::Notify = tokio::sync::Notify::const_new();

/// A newer release, as the notice shows it.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Found {
    pub version: Version,
    pub name: String,
    pub notes: String,
    pub url: String,
}

/// What survives a restart: the last check (rule 4 counts it across runs), what it found and *Later*.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Saved {
    #[serde(default)]
    pub last_check_ms: Option<i64>,
    /// The last look failed (no network, GitHub refused): shown in Settings, never as a notice.
    #[serde(default)]
    pub failed: bool,
    #[serde(default)]
    pub found: Option<Found>,
    #[serde(default)]
    pub snooze: Option<Snooze>,
}

#[derive(Debug, Default)]
pub struct UpdateState {
    saved: Mutex<Saved>,
    #[cfg(feature = "update-notice")]
    checking: AtomicBool,
}

/// What the renderer reads (`update_status`, the `update-notice` event).
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    /// False in distro packages: no question, no switch, no notice.
    available: bool,
    consent: rules::Consent,
    ask: bool,
    current: Option<Version>,
    checked_at_ms: Option<i64>,
    checking: bool,
    failed: bool,
    /// Only when it is newer than this build and not snoozed.
    notice: Option<Found>,
}

/// The newest release in `raw` above `current`, ready for the notice. `Err` on JSON that is not
/// a releases list; `Ok(None)` when nothing newer is published.
#[cfg(any(feature = "update-notice", test))]
pub fn read_releases(raw: &[u8], current: Version) -> Result<Option<Found>, String> {
    let list: Vec<Release> = serde_json::from_slice(raw).map_err(|_| "Invalid releases list")?;
    Ok(rules::newest(&list, current).and_then(|(version, r)| {
        let url = safe_page(&r.html_url).then(|| r.html_url.clone())?;
        Some(Found {
            version,
            name: r
                .name
                .clone()
                .filter(|n| !n.trim().is_empty())
                .unwrap_or_else(|| r.tag_name.clone()),
            notes: r
                .body
                .as_deref()
                .unwrap_or_default()
                .chars()
                .take(MAX_NOTES)
                .collect(),
            url,
        })
    }))
}

/// The link is opened on a click: only a plain path under the repository's releases.
#[cfg(any(feature = "update-notice", test))]
fn safe_page(url: &str) -> bool {
    url.strip_prefix(PAGES).is_some_and(|rest| {
        !rest.is_empty()
            && !rest.contains("..")
            && rest
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'/' | b'-' | b'_'))
    })
}

fn status_of(saved: &Saved, consent: rules::Consent, checking: bool, now_ms: i64) -> Status {
    let current = Version::current();
    let available = cfg!(feature = "update-notice");
    Status {
        available,
        consent,
        ask: available && consent.should_ask(now_ms),
        current,
        checked_at_ms: saved.last_check_ms,
        checking,
        failed: saved.failed,
        notice: saved
            .found
            .clone()
            .filter(|_| available)
            .filter(|f| current.is_some_and(|c| f.version > c))
            .filter(|f| rules::should_show(f.version, saved.snooze, now_ms)),
    }
}

fn saved(app: &AppHandle) -> Saved {
    app.try_state::<UpdateState>()
        .and_then(|s| s.saved.lock().ok().map(|s| s.clone()))
        .unwrap_or_default()
}

fn checking(app: &AppHandle) -> bool {
    #[cfg(feature = "update-notice")]
    {
        app.try_state::<UpdateState>()
            .is_some_and(|s| s.checking.load(Ordering::Acquire))
    }
    #[cfg(not(feature = "update-notice"))]
    {
        let _ = app;
        false
    }
}

#[tauri::command]
pub fn update_status(app: AppHandle) -> Status {
    status_of(
        &saved(&app),
        crate::settings::update_consent(&app),
        checking(&app),
        crate::traces::now_ms(),
    )
}

#[cfg(feature = "update-notice")]
fn publish(app: &AppHandle) {
    let _ = app.emit("update-notice", update_status(app.clone()));
}

#[cfg(feature = "update-notice")]
fn store(app: &AppHandle, change: impl FnOnce(&mut Saved)) {
    let state = app.state::<UpdateState>();
    let mut saved = state.saved.lock().unwrap_or_else(|e| e.into_inner());
    change(&mut saved);
    let path = crate::paths::data_dir().join(FILE);
    let temp = path.with_extension("json.new");
    if let Ok(raw) = serde_json::to_vec(&*saved)
        && std::fs::write(&temp, raw)
            .and_then(|_| std::fs::rename(&temp, &path))
            .is_err()
    {
        tracing::warn!("could not save the update notice state");
    }
}

/// Only the island (its question and notice) and Settings answer for the update notice.
#[cfg(feature = "update-notice")]
fn from_island_or_settings(window: &tauri::Webview) -> Result<(), String> {
    if window.label() == crate::ISLAND || window.label() == crate::SETTINGS {
        Ok(())
    } else {
        Err("only the island or Settings can do this".into())
    }
}

/// The island's question answered, or Settings' switch: yes lets the daily look start.
#[cfg(feature = "update-notice")]
#[tauri::command]
pub fn set_update_checks(app: AppHandle, window: tauri::Webview, on: bool) -> Result<Status, String> {
    from_island_or_settings(&window)?;
    let consent = if on {
        rules::Consent::Yes
    } else {
        rules::Consent::No
    };
    crate::settings::edit(&app, |s| s.update_consent = consent)?;
    if on {
        WAKE.notify_one();
    }
    publish(&app);
    Ok(update_status(app))
}

/// The question was on screen: left unanswered, it comes back after a day, never sooner.
#[cfg(feature = "update-notice")]
#[tauri::command]
pub fn update_question_shown(app: AppHandle, window: tauri::Webview) -> Result<(), String> {
    from_island_or_settings(&window)?;
    let now = crate::traces::now_ms();
    if crate::settings::update_consent(&app).should_ask(now) {
        crate::settings::edit(&app, |s| s.update_consent = rules::Consent::Asked { at_ms: now })?;
        publish(&app);
    }
    Ok(())
}

/// *Later*: this version stays out of sight for a day; a newer one shows at once.
#[cfg(feature = "update-notice")]
#[tauri::command]
pub fn update_later(app: AppHandle, window: tauri::Webview) -> Result<(), String> {
    from_island_or_settings(&window)?;
    let now = crate::traces::now_ms();
    store(&app, |s| {
        if let Some(found) = &s.found {
            s.snooze = Some(Snooze::later(found.version, now));
        }
    });
    publish(&app);
    Ok(())
}

/// Opens the found release's page; the renderer never names the link.
#[cfg(feature = "update-notice")]
#[tauri::command]
pub fn open_update_page(app: AppHandle, window: tauri::Webview) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    from_island_or_settings(&window)?;
    let url = saved(&app)
        .found
        .map(|f| f.url)
        .filter(|u| safe_page(u))
        .ok_or("No release page to open")?;
    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|_| "Could not open the release page".to_string())
}

/// Settings' *Check now*: a click, so it works with the automatic look off, through its own
/// small budget and trace.
#[cfg(feature = "update-notice")]
#[tauri::command]
pub async fn check_update_now(app: AppHandle, window: tauri::Webview) -> Result<Status, String> {
    crate::settings_page(&window)?;
    let _ = check(&app, crate::gate::UPDATE_NOW).await;
    Ok(update_status(app))
}

pub fn start(app: &AppHandle) {
    let loaded = std::fs::read(crate::paths::data_dir().join(FILE))
        .ok()
        .and_then(|raw| serde_json::from_slice::<Saved>(&raw).ok())
        .unwrap_or_default();
    app.manage(UpdateState {
        saved: Mutex::new(loaded),
        #[cfg(feature = "update-notice")]
        checking: AtomicBool::new(false),
    });
    #[cfg(feature = "update-notice")]
    {
        let app = app.clone();
        tauri::async_runtime::spawn(async move {
            publish(&app);
            tokio::time::sleep(Duration::from_millis(rules::FIRST_LOOK_MS as u64)).await;
            loop {
                let due = rules::should_check(crate::traces::now_ms(), saved(&app).last_check_ms);
                if due && crate::settings::update_consent(&app).allows_checks() && !crate::lock::locked(&app)
                {
                    let _ = check(&app, crate::gate::UPDATE).await;
                }
                tokio::select! {
                    () = tokio::time::sleep(POLL) => {}
                    () = WAKE.notified() => {}
                }
            }
        });
    }
}

/// One look at a time, whoever asked.
#[cfg(feature = "update-notice")]
async fn check(app: &AppHandle, feature: &str) -> Result<(), String> {
    let state = app.state::<UpdateState>();
    if state.checking.swap(true, Ordering::AcqRel) {
        return Err("Already checking".into());
    }
    publish(app);
    let result = look(app, feature).await;
    state.checking.store(false, Ordering::Release);
    publish(app);
    result
}

/// The releases list, through the gate (switch, budget, trace).
#[cfg(feature = "update-notice")]
async fn look(app: &AppHandle, feature: &str) -> Result<(), String> {
    let current = Version::current().ok_or("Unknown app version")?;
    let gate = crate::gate::shared(app);
    if !gate.allow(feature) {
        return Err("Update check unavailable".into());
    }
    let at = Instant::now();
    let result = vults_chat::releases::fetch().await;
    let found = result
        .as_deref()
        .map_err(Clone::clone)
        .and_then(|raw| read_releases(raw, current));
    gate.done(feature, found.is_ok(), at.elapsed());
    // A failed look still counts as the day's: retrying sooner would break "at most daily".
    let now = crate::traces::now_ms();
    store(app, |s| {
        s.last_check_ms = Some(now);
        s.failed = found.is_err();
        if let Ok(found) = &found {
            s.found = found.clone();
        }
    });
    found.map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(text: &str) -> Version {
        Version::parse(text).unwrap()
    }
    fn page(tag: &str) -> String {
        format!("{PAGES}tag/{tag}")
    }

    #[test]
    fn reads_the_newest_published_release() {
        let raw = serde_json::json!([
            {"tag_name":"v9.0.1","name":"Vults v9.0.1","body":"Fixes","html_url":page("v9.0.1"),"draft":false,"prerelease":false},
            {"tag_name":"v9.1.0","name":"","body":null,"html_url":page("v9.1.0"),"prerelease":true},
            {"tag_name":"v0.0.1","html_url":page("v0.0.1")}
        ])
        .to_string();
        let found = read_releases(raw.as_bytes(), v("0.1.8")).unwrap().unwrap();
        assert_eq!(found.version, v("9.0.1"));
        assert_eq!(found.name, "Vults v9.0.1");
        assert_eq!(found.notes, "Fixes");
        assert_eq!(read_releases(raw.as_bytes(), v("9.0.1")).unwrap(), None);
        assert!(read_releases(b"{\"message\":\"rate limited\"}", v("0.1.8")).is_err());
    }

    #[test]
    fn only_the_repositorys_own_release_pages_are_offered() {
        for bad in [
            "https://evil.example/x",
            "https://github.com/someone/else/releases/tag/v9.0.0",
            "http://github.com/rogeriojunior31/Vults/releases/tag/v9.0.0",
            "https://github.com/rogeriojunior31/Vults/releases/../../evil",
            "https://github.com/rogeriojunior31/Vults/releases/tag/v9?x=javascript:1",
            "https://github.com/rogeriojunior31/Vults/releases/",
        ] {
            let raw = serde_json::json!([{"tag_name":"v9.0.0","html_url":bad}]).to_string();
            assert_eq!(read_releases(raw.as_bytes(), v("0.1.8")).unwrap(), None, "{bad}");
        }
        assert!(safe_page(&page("v9.0.0")));
    }

    #[test]
    fn long_notes_are_cut_and_a_missing_name_falls_back_to_the_tag() {
        let body = "x".repeat(MAX_NOTES + 50);
        let raw =
            serde_json::json!([{"tag_name":"v9.0.0","body":body,"html_url":page("v9.0.0")}]).to_string();
        let found = read_releases(raw.as_bytes(), v("0.1.8")).unwrap().unwrap();
        assert_eq!(found.notes.chars().count(), MAX_NOTES);
        assert_eq!(found.name, "v9.0.0");
    }

    #[test]
    fn the_notice_hides_when_snoozed_or_not_newer() {
        let t = 1_000 * rules::DAY_MS;
        let found = Found {
            version: v("99.0.0"),
            name: "n".into(),
            notes: String::new(),
            url: page("v99.0.0"),
        };
        let mut saved = Saved {
            found: Some(found.clone()),
            ..Saved::default()
        };
        let notice = |saved: &Saved, now| status_of(saved, rules::Consent::Yes, false, now).notice;
        let on = cfg!(feature = "update-notice");
        assert_eq!(notice(&saved, t), on.then(|| found.clone()));
        saved.snooze = Some(Snooze::later(found.version, t));
        assert_eq!(notice(&saved, t + 1), None);
        assert_eq!(notice(&saved, t + rules::DAY_MS).is_some(), on);
        saved.snooze = None;
        saved.found = Some(Found {
            version: v("0.0.1"),
            ..found
        });
        assert_eq!(notice(&saved, t), None);
        assert_eq!(status_of(&saved, rules::Consent::Unasked, false, t).ask, on);
        assert_eq!(status_of(&saved, rules::Consent::Unasked, false, t).available, on);
    }

    #[test]
    fn saved_state_tolerates_an_old_or_partial_file() {
        assert_eq!(serde_json::from_str::<Saved>("{}").unwrap(), Saved::default());
        let old = r#"{"lastCheckMs":5,"found":null,"snooze":null}"#;
        assert_eq!(serde_json::from_str::<Saved>(old).unwrap().last_check_ms, Some(5));
        let s = Saved {
            last_check_ms: Some(5),
            failed: true,
            ..Saved::default()
        };
        let raw = serde_json::to_string(&s).unwrap();
        assert_eq!(serde_json::from_str::<Saved>(&raw).unwrap(), s);
    }
}
