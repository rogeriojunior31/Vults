//! The update notice (FF4, ADR 0019): asked for, at most daily, never an install. Once the user
//! said yes, the app looks at its releases list through the network gate about 30 s after launch
//! and then once a day; what it found waits in `update.json` for the island to show.
pub mod rules;

use std::sync::Mutex;
use std::time::{Duration, Instant};

use tauri::{AppHandle, Emitter, Manager};
use vults_connectors::Gate;

use rules::{Release, Snooze, Version};

/// How often the loop wakes to see whether a check is due; the check itself is daily.
const POLL: Duration = Duration::from_secs(3600);
const FILE: &str = "update.json";
/// Release notes are shown, not stored whole: a long changelog stays on the release page.
const MAX_NOTES: usize = 4000;

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
    #[serde(default)]
    pub found: Option<Found>,
    #[serde(default)]
    pub snooze: Option<Snooze>,
}

#[derive(Debug, Default)]
pub struct UpdateState(Mutex<Saved>);

/// What the renderer reads (`update_status`, the `update-notice` event).
#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    consent: rules::Consent,
    ask: bool,
    current: Option<Version>,
    checked_at_ms: Option<i64>,
    /// Only when it is newer than this build and not snoozed.
    notice: Option<Found>,
}

/// The newest release in `raw` above `current`, ready for the notice. `Err` on JSON that is not
/// a releases list; `Ok(None)` when nothing newer is published.
pub fn read_releases(raw: &[u8], current: Version) -> Result<Option<Found>, String> {
    let list: Vec<Release> = serde_json::from_slice(raw).map_err(|_| "Invalid releases list")?;
    Ok(rules::newest(&list, current).and_then(|(version, r)| {
        // Only a link to the repository's own release page is opened later.
        let url = r
            .html_url
            .starts_with("https://github.com/")
            .then(|| r.html_url.clone())?;
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

fn status_of(saved: &Saved, consent: rules::Consent, now_ms: i64) -> Status {
    let current = Version::current();
    Status {
        consent,
        ask: consent.should_ask(now_ms),
        current,
        checked_at_ms: saved.last_check_ms,
        notice: saved
            .found
            .clone()
            .filter(|f| current.is_some_and(|c| f.version > c))
            .filter(|f| rules::should_show(f.version, saved.snooze, now_ms)),
    }
}

fn saved(app: &AppHandle) -> Saved {
    app.state::<UpdateState>()
        .0
        .lock()
        .map(|s| s.clone())
        .unwrap_or_default()
}

#[tauri::command]
pub fn update_status(app: AppHandle) -> Status {
    status_of(
        &saved(&app),
        crate::settings::update_consent(&app),
        crate::traces::now_ms(),
    )
}

fn publish(app: &AppHandle) {
    let _ = app.emit("update-notice", update_status(app.clone()));
}

fn store(app: &AppHandle, change: impl FnOnce(&mut Saved)) {
    let state = app.state::<UpdateState>();
    let mut saved = state.0.lock().unwrap_or_else(|e| e.into_inner());
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

pub fn start(app: &AppHandle) {
    let loaded = std::fs::read(crate::paths::data_dir().join(FILE))
        .ok()
        .and_then(|raw| serde_json::from_slice::<Saved>(&raw).ok())
        .unwrap_or_default();
    app.manage(UpdateState(Mutex::new(loaded)));
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        publish(&app);
        tokio::time::sleep(Duration::from_millis(rules::FIRST_LOOK_MS as u64)).await;
        loop {
            let due = rules::should_check(crate::traces::now_ms(), saved(&app).last_check_ms);
            if due && crate::settings::update_consent(&app).allows_checks() && !crate::lock::locked(&app) {
                let _ = check(&app).await;
            }
            tokio::time::sleep(POLL).await;
        }
    });
}

/// One look at the releases list, through the gate (switch, budget, trace).
async fn check(app: &AppHandle) -> Result<(), String> {
    let current = Version::current().ok_or("Unknown app version")?;
    let gate = crate::gate::shared(app);
    if !gate.allow(crate::gate::UPDATE) {
        return Err("Update check unavailable".into());
    }
    let at = Instant::now();
    let result = vults_chat::releases::fetch().await;
    let found = result
        .as_deref()
        .map_err(Clone::clone)
        .and_then(|raw| read_releases(raw, current));
    gate.done(crate::gate::UPDATE, found.is_ok(), at.elapsed());
    // A failed look still counts as the day's: retrying sooner would break "at most daily".
    let now = crate::traces::now_ms();
    store(app, |s| {
        s.last_check_ms = Some(now);
        if let Ok(found) = &found {
            s.found = found.clone();
        }
    });
    publish(app);
    found.map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(text: &str) -> Version {
        Version::parse(text).unwrap()
    }
    const LIST: &str = r#"[
        {"tag_name":"v9.0.1","name":"Vults v9.0.1","body":"Fixes","html_url":"https://github.com/o/r/releases/tag/v9.0.1","draft":false,"prerelease":false},
        {"tag_name":"v9.1.0","name":"","body":null,"html_url":"https://github.com/o/r/releases/tag/v9.1.0","prerelease":true},
        {"tag_name":"v0.0.1","html_url":"https://github.com/o/r/releases/tag/v0.0.1"}
    ]"#;

    #[test]
    fn reads_the_newest_published_release() {
        let found = read_releases(LIST.as_bytes(), v("0.1.8")).unwrap().unwrap();
        assert_eq!(found.version, v("9.0.1"));
        assert_eq!(found.name, "Vults v9.0.1");
        assert_eq!(found.notes, "Fixes");
        assert_eq!(read_releases(LIST.as_bytes(), v("9.0.1")).unwrap(), None);
        assert!(read_releases(b"{\"message\":\"rate limited\"}", v("0.1.8")).is_err());
    }

    #[test]
    fn a_link_off_github_is_never_offered() {
        let raw = r#"[{"tag_name":"v9.0.0","html_url":"https://evil.example/x"}]"#;
        assert_eq!(read_releases(raw.as_bytes(), v("0.1.8")).unwrap(), None);
    }

    #[test]
    fn long_notes_are_cut_and_a_missing_name_falls_back_to_the_tag() {
        let body = "x".repeat(MAX_NOTES + 50);
        let raw = serde_json::json!([{"tag_name":"v9.0.0","body":body,"html_url":"https://github.com/o/r"}])
            .to_string();
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
            url: "https://github.com/o/r".into(),
        };
        let mut saved = Saved {
            found: Some(found.clone()),
            ..Saved::default()
        };
        assert_eq!(
            status_of(&saved, rules::Consent::Yes, t).notice,
            Some(found.clone())
        );
        saved.snooze = Some(Snooze::later(found.version, t));
        assert_eq!(status_of(&saved, rules::Consent::Yes, t + 1).notice, None);
        assert!(
            status_of(&saved, rules::Consent::Yes, t + rules::DAY_MS)
                .notice
                .is_some()
        );
        saved.snooze = None;
        saved.found = Some(Found {
            version: v("0.0.1"),
            ..found
        });
        assert_eq!(status_of(&saved, rules::Consent::Yes, t).notice, None);
        assert!(status_of(&saved, rules::Consent::Unasked, t).ask);
    }

    #[test]
    fn saved_state_tolerates_an_old_or_partial_file() {
        assert_eq!(serde_json::from_str::<Saved>("{}").unwrap(), Saved::default());
        let s = Saved {
            last_check_ms: Some(5),
            ..Saved::default()
        };
        let raw = serde_json::to_string(&s).unwrap();
        assert_eq!(serde_json::from_str::<Saved>(&raw).unwrap(), s);
    }
}
