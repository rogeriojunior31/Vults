//! The local history of agent turns (docs/dev/plan-activity.md): `history.jsonl`, one finished turn
//! per line for 12 weeks, and `days.json`, each day's totals for a year, which the grid reads. Counts
//! and folder names only, nothing leaves the machine; Settings → Activity stops it or clears it.

use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use vults_core::looks::Date;

const VERSION: u32 = 1;
/// Turns older than this many days leave `history.jsonl`.
const KEEP_TURNS_DAYS: i64 = 12 * 7;
/// Days older than this leave `days.json`.
const KEEP_DAYS: i64 = 366;

/// One file writer at a time: turns end on the core's loop, a clear comes from Settings.
static WRITING: Mutex<()> = Mutex::new(());

/// A finished turn, as a line of `history.jsonl`; and a day's totals, for the grid.
pub use vults_core::recap::{DayTotal, Entry as Record};

#[derive(Serialize, Deserialize, Debug, Default, PartialEq, Eq)]
struct Days {
    v: u32,
    days: BTreeMap<String, DayTotal>,
}

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

fn turns_file(dir: &Path) -> PathBuf {
    dir.join("history.jsonl")
}

fn days_file(dir: &Path) -> PathBuf {
    dir.join("days.json")
}

/// Keeps a finished turn: one more line, and its day's totals.
pub fn append(dir: &Path, record: &Record) -> std::io::Result<()> {
    let _writing = WRITING.lock().unwrap_or_else(|e| e.into_inner());
    std::fs::create_dir_all(dir)?;
    let mut line = serde_json::to_string(record).map_err(std::io::Error::other)?;
    line.push('\n');
    let mut file = private(std::fs::OpenOptions::new().create(true).append(true)).open(turns_file(dir))?;
    file.write_all(line.as_bytes())?;
    let mut days = read_days(dir);
    let total = days.days.entry(record.day.clone()).or_default();
    total.turns += 1;
    total.secs += record.secs;
    write_days(dir, &days)
}

/// Every kept turn, oldest first. A line that does not read is skipped: one bad line never
/// costs the rest.
pub fn turns(dir: &Path) -> Vec<Record> {
    let Ok(text) = std::fs::read_to_string(turns_file(dir)) else {
        return Vec::new();
    };
    text.lines()
        .filter_map(|l| serde_json::from_str::<Record>(l).ok())
        .collect()
}

/// Each kept day's totals.
pub fn days(dir: &Path) -> BTreeMap<String, DayTotal> {
    read_days(dir).days
}

/// Drops the turns past 12 weeks and the days past a year, counted back from `today`.
pub fn prune(dir: &Path, today: Date) -> std::io::Result<()> {
    let _writing = WRITING.lock().unwrap_or_else(|e| e.into_inner());
    let oldest_turn = today.plus(-KEEP_TURNS_DAYS);
    let kept: Vec<Record> = turns(dir)
        .into_iter()
        .filter(|r| Date::parse(&r.day).is_some_and(|d| d >= oldest_turn))
        .collect();
    if turns_file(dir).exists() {
        let mut text = String::new();
        for r in &kept {
            text.push_str(&serde_json::to_string(r).map_err(std::io::Error::other)?);
            text.push('\n');
        }
        replace(&turns_file(dir), text.as_bytes())?;
    }
    let oldest_day = today.plus(-KEEP_DAYS);
    let mut days = read_days(dir);
    let before = days.days.len();
    days.days
        .retain(|day, _| Date::parse(day).is_some_and(|d| d >= oldest_day));
    if days.days.len() != before {
        write_days(dir, &days)?;
    }
    Ok(())
}

/// Removes both files: nothing of the history is left.
pub fn clear(dir: &Path) -> std::io::Result<()> {
    let _writing = WRITING.lock().unwrap_or_else(|e| e.into_inner());
    for path in [turns_file(dir), days_file(dir)] {
        match std::fs::remove_file(&path) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => return Err(e),
            _ => {}
        }
    }
    Ok(())
}

fn read_days(dir: &Path) -> Days {
    std::fs::read_to_string(days_file(dir))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or(Days {
            v: VERSION,
            days: BTreeMap::new(),
        })
}

fn write_days(dir: &Path, days: &Days) -> std::io::Result<()> {
    let text = serde_json::to_string(days).map_err(std::io::Error::other)?;
    replace(&days_file(dir), text.as_bytes())
}

/// The user's alone: it names their projects.
fn private(options: &mut std::fs::OpenOptions) -> &mut std::fs::OpenOptions {
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(options, 0o600);
    options
}

/// Written beside, synced, then renamed over: a power cut leaves the old file or the new one.
fn replace(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let temp = path.with_extension("tmp");
    let mut file = private(
        std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true),
    )
    .open(&temp)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    std::fs::rename(temp, path)
}

#[cfg(test)]
mod tests {
    use super::*;
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
    fn a_bad_line_costs_only_itself() {
        let dir = temp("bad");
        let a = Record::new(&turn(60), NOON, 0);
        append(&dir, &a).unwrap();
        let mut f = std::fs::OpenOptions::new()
            .append(true)
            .open(turns_file(&dir))
            .unwrap();
        f.write_all(b"{\"v\":1,\"end\":\"soon\"}\nnot json\n").unwrap();
        append(&dir, &a).unwrap();
        assert_eq!(turns(&dir).len(), 2);
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
        let a = activity(&dir, true, today, None);
        assert_eq!(a.weeks, ["2026-10-05", "2026-09-21"]);
        assert_eq!((a.week.monday.as_str(), a.week.turns), ("2026-10-05", 1));
        assert_eq!(a.grid.last().map(|d| d.day.as_str()), Some("2026-10-09"));
        let older = activity(&dir, true, today, Some("2026-09-23"));
        assert_eq!(
            (older.week.monday.as_str(), older.week.active_secs),
            ("2026-09-21", 300)
        );
        // A quiet week still shows this week first.
        let later = activity(&dir, true, Date::new(2026, 10, 20), None);
        assert_eq!(later.weeks[0], "2026-10-19");
        assert_eq!(later.week.turns, 0);
    }

    #[test]
    fn clear_leaves_nothing() {
        let dir = temp("clear");
        append(&dir, &Record::new(&turn(60), NOON, 0)).unwrap();
        clear(&dir).unwrap();
        assert!(!turns_file(&dir).exists() && !days_file(&dir).exists());
        assert!(turns(&dir).is_empty() && days(&dir).is_empty());
        clear(&dir).expect("clearing nothing is fine");
    }

    #[test]
    fn the_line_names_no_path_prompt_or_command() {
        let r = Record::new(&turn(60), NOON, 0);
        let line = serde_json::to_string(&r).unwrap();
        assert_eq!(
            line,
            r#"{"v":1,"end":1791547200,"day":"2026-10-09","secs":60,"agent":"claude","project":"site","steps":3,"commands":1,"files":2,"added":10,"removed":4,"allowed":1,"denied":0,"answered":0,"questions":0,"failed":false}"#
        );
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
pub fn activity(dir: &Path, history: bool, today: Date, monday: Option<&str>) -> Activity {
    let entries = turns(dir);
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
        week: vults_core::recap::week(vults_core::i18n::Lang::En, &entries, monday),
        grid: vults_core::recap::grid(&days(dir), today),
    }
}
