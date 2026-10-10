//! The local history of agent turns (docs/guide/activity.md), kept in the local database
//! (`vults-store`): each finished turn for 12 weeks and each day's totals for a year, which the grid
//! reads. Counts and folder names only, nothing leaves the machine; Settings → Activity stops it or
//! clears it.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Serialize;
use vults_core::looks::Date;
use vults_store::Store;

/// A finished turn, as the history keeps it.
pub use vults_core::recap::Entry as Record;

pub fn unix_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(0))
}

/// The desktop's UTC offset at that instant; UTC when it can't say (the day may then be off by one).
pub fn local_offset(unix: i64) -> i32 {
    vults_platform::utc_offset(unix).unwrap_or_else(|| {
        tracing::debug!("no local time zone: history days in UTC");
        0
    })
}

/// Today, in the user's own calendar.
pub fn today() -> Date {
    let now = unix_now();
    Date::of(now, local_offset(now))
}

pub fn dir() -> PathBuf {
    crate::paths::data_dir()
}

/// The database in `dir`, with 0.1.x's `history.jsonl` and `days.json` copied in on first use.
fn open(dir: &Path) -> std::io::Result<Store> {
    let mut store = Store::open(dir).map_err(std::io::Error::other)?;
    match store.import_old_history(dir) {
        Ok(0) => {}
        Ok(n) => tracing::info!("history: {n} turns moved into {}", vults_store::FILE),
        // The old files stay for the next try; the database still works. Said once per run.
        Err(e) => {
            static SAID: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
            if !SAID.swap(true, std::sync::atomic::Ordering::Relaxed) {
                tracing::warn!("history: the old files did not move in: {e}");
            }
        }
    }
    Ok(store)
}

/// Keeps a finished turn, and adds it to its day's totals.
pub fn append(dir: &Path, record: &Record) -> std::io::Result<()> {
    open(dir)?.append_turn(record).map_err(std::io::Error::other)
}

/// Every kept turn, oldest first; none when the database can't be read (logged).
pub fn turns(dir: &Path) -> Vec<Record> {
    open(dir)
        .and_then(|s| s.turns().map_err(std::io::Error::other))
        .unwrap_or_else(|e| {
            tracing::warn!("history: can't read the turns: {e}");
            Vec::new()
        })
}

/// Drops the turns past 12 weeks and the days past a year, counted back from `today`.
pub fn prune(dir: &Path, today: Date) -> std::io::Result<()> {
    open(dir)?.prune_history(today).map_err(std::io::Error::other)
}

/// Removes the whole history, and the audit log with it: nothing of either is left.
pub fn clear(dir: &Path) -> std::io::Result<()> {
    let mut store = open(dir)?;
    store.clear_history(dir).map_err(std::io::Error::other)?;
    store.clear_audit().map_err(std::io::Error::other)
}

#[cfg(test)]
mod tests {
    use super::*;
    use vults_core::recap::DayTotal;

    fn days(dir: &Path) -> BTreeMap<String, DayTotal> {
        open(dir).unwrap().days().unwrap()
    }
    use std::time::Duration;
    use vults_core::turns::Turn;
    use vults_protocol::AgentKind;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("vults-history-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    fn turn(secs: u64) -> Turn {
        Turn {
            agent: AgentKind::Claude,
            project: "site".into(),
            secs,
            ago: Duration::ZERO,
            steps: 3,
            commands: 1,
            files: 2,
            added: 10,
            removed: 4,
            allowed: 1,
            denied: 0,
            answered: 0,
            questions: 0,
            failed: false,
        }
    }

    /// 2026-10-09 12:00 UTC.
    const NOON: i64 = 1_791_547_200;

    #[test]
    fn a_turn_is_kept_and_read_back_with_its_day() {
        let dir = temp("append");
        let a = Record::new(&turn(60), NOON, 0);
        let b = Record::new(&turn(30), NOON + 3600, -3 * 3600);
        append(&dir, &a).unwrap();
        append(&dir, &b).unwrap();
        assert_eq!(turns(&dir), vec![a.clone(), b]);
        assert_eq!(a.day, "2026-10-09");
        assert_eq!(days(&dir)["2026-10-09"], DayTotal { turns: 2, secs: 90 });
    }

    #[test]
    fn a_silent_turn_ends_when_its_last_event_was() {
        let t = Turn {
            ago: Duration::from_secs(13 * 3600),
            ..turn(5)
        };
        let r = Record::new(&t, NOON, 0);
        assert_eq!(r.end, NOON - 13 * 3600);
        assert_eq!(r.day, "2026-10-08");
    }

    #[test]
    fn prune_keeps_twelve_weeks_of_turns_and_a_year_of_days() {
        let dir = temp("prune");
        let day = 24 * 3600;
        for ago in [0, 80, 90, 300, 400] {
            append(&dir, &Record::new(&turn(60), NOON - ago * day, 0)).unwrap();
        }
        prune(&dir, Date::new(2026, 10, 9)).unwrap();
        let kept: Vec<String> = turns(&dir).into_iter().map(|r| r.day).collect();
        assert_eq!(kept.len(), 2, "{kept:?}");
        assert_eq!(days(&dir).len(), 4, "a year of days");
    }

    #[test]
    fn the_page_pages_through_this_week_and_every_kept_one() {
        let dir = temp("activity");
        let day = 24 * 3600;
        // 2026-10-09 is a Friday; turns on it and two weeks before.
        append(&dir, &Record::new(&turn(600), NOON, 0)).unwrap();
        append(&dir, &Record::new(&turn(300), NOON - 14 * day, 0)).unwrap();
        let today = Date::new(2026, 10, 9);
        let a = activity(&dir, true, today, None, vults_core::i18n::Lang::En);
        assert_eq!(a.weeks, ["2026-10-05", "2026-09-21"]);
        assert_eq!((a.week.monday.as_str(), a.week.turns), ("2026-10-05", 1));
        assert_eq!(a.grid.last().map(|d| d.day.as_str()), Some("2026-10-09"));
        let older = activity(&dir, true, today, Some("2026-09-23"), vults_core::i18n::Lang::En);
        assert_eq!(
            (older.week.monday.as_str(), older.week.active_secs),
            ("2026-09-21", 300)
        );
        // A quiet week still shows this week first.
        let later = activity(
            &dir,
            true,
            Date::new(2026, 10, 20),
            None,
            vults_core::i18n::Lang::En,
        );
        assert_eq!(later.weeks[0], "2026-10-19");
        assert_eq!(later.week.turns, 0);
    }

    #[test]
    fn clear_leaves_nothing() {
        let dir = temp("clear");
        append(&dir, &Record::new(&turn(60), NOON, 0)).unwrap();
        clear(&dir).unwrap();
        assert!(turns(&dir).is_empty() && days(&dir).is_empty());
        clear(&dir).expect("clearing nothing is fine");
    }

    #[test]
    fn clear_takes_the_audit_log_too() {
        use vults_core::audit::{Act, Actor, Audit};
        let dir = temp("clear-audit");
        let line = Audit::new(Actor::Human, Act::Allow, "claude", "site", "Bash", "ls");
        open(&dir).unwrap().append_audit(unix_now(), &line).unwrap();
        clear(&dir).unwrap();
        assert!(open(&dir).unwrap().audit(10).unwrap().is_empty());
    }
}

/// What Settings → Activity shows: one week's recap, the weeks there are, and the grid.
#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Activity {
    pub history: bool,
    /// Today (`2026-10-09`), in the user's calendar.
    pub today: String,
    /// The Mondays to page through, newest first: this week's, and every kept week with a turn.
    pub weeks: Vec<String>,
    pub week: vults_core::recap::WeekView,
    pub grid: Vec<vults_core::recap::GridDay>,
}

/// The page's data for the week of `monday` (this week's when none, or one that does not read).
pub fn activity(
    dir: &Path,
    history: bool,
    today: Date,
    monday: Option<&str>,
    lang: vults_core::i18n::Lang,
) -> Activity {
    let (entries, days) = match open(dir) {
        Ok(s) => (
            s.turns().unwrap_or_else(|e| {
                tracing::warn!("history: can't read the turns: {e}");
                Vec::new()
            }),
            s.days().unwrap_or_else(|e| {
                tracing::warn!("history: can't read the days: {e}");
                BTreeMap::new()
            }),
        ),
        Err(e) => {
            tracing::warn!("history: can't open: {e}");
            (Vec::new(), BTreeMap::new())
        }
    };
    let this_week = today.monday();
    let mut weeks = vults_core::recap::weeks_with_turns(&entries);
    if !weeks.contains(&this_week) {
        weeks.push(this_week);
    }
    weeks.sort_unstable_by(|a, b| b.cmp(a));
    let monday = monday.and_then(Date::parse).map_or(this_week, Date::monday);
    Activity {
        history,
        today: today.iso(),
        weeks: weeks.iter().map(|d| d.iso()).collect(),
        week: vults_core::recap::week(lang, &entries, monday),
        grid: vults_core::recap::grid(&days, today),
    }
}

/// Bigger than any recap image: anything past it is not one.
const MAX_IMAGE: usize = 16 * 1024 * 1024;
const PNG_MAGIC: &[u8] = b"\x89PNG\r\n\x1a\n";

/// A file name for the image: letters, digits, `-`, `_` and `.` only, ending in `.png`.
fn image_name(name: &str) -> String {
    let stem: String = name
        .trim_end_matches(".png")
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.') {
                c
            } else {
                '-'
            }
        })
        .take(80)
        .collect();
    let stem = stem.trim_matches(['-', '.']);
    format!("{}.png", if stem.is_empty() { "vults-week" } else { stem })
}

/// Settings → Activity's *Save as image…*: the PNG the page drew, written only where the user
/// picks in the desktop's own dialog (on Linux; elsewhere into the Pictures folder). The path it
/// went to, `~` for home; none when the user cancelled.
#[tauri::command]
pub async fn save_recap_image(name: String, png: Vec<u8>) -> Result<Option<String>, String> {
    if png.len() > MAX_IMAGE || !png.starts_with(PNG_MAGIC) {
        return Err("not a PNG image".into());
    }
    let name = image_name(&name);
    #[cfg(target_os = "linux")]
    let path = match vults_platform::save::png("Save the week as an image", &name).await {
        Some(path) => path,
        None => return Ok(None),
    };
    #[cfg(not(target_os = "linux"))]
    let path = {
        let dir = crate::paths::home().join("Pictures");
        std::fs::create_dir_all(&dir).map_err(|e| format!("can't make {}: {e}", dir.display()))?;
        dir.join(&name)
    };
    let shown = crate::paths::shown(&path);
    tauri::async_runtime::spawn_blocking(move || std::fs::write(&path, png))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| format!("can't save {shown}: {e}"))?;
    Ok(Some(shown))
}

#[cfg(test)]
mod image_tests {
    use super::image_name;

    #[test]
    fn the_image_name_is_tame() {
        assert_eq!(
            image_name("vults-week-2026-10-05.png"),
            "vults-week-2026-10-05.png"
        );
        assert_eq!(image_name("../../etc/passwd"), "etc-passwd.png");
        assert_eq!(image_name("my week \u{e9}"), "my-week.png");
        assert_eq!(image_name(""), "vults-week.png");
    }
}
