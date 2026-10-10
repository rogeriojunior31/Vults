//! The local history of agent turns (docs/guide/activity.md): each finished turn for 12 weeks, and
//! each day's totals for a year, which the grid reads. Counts and folder names only.

use std::collections::BTreeMap;
use std::path::Path;

use rusqlite::{OptionalExtension, Row, Transaction, TransactionBehavior, params};
use vults_core::looks::Date;
use vults_core::recap::{DayTotal, Entry};
use vults_protocol::AgentKind;

use crate::{Error, Store};

/// Turns older than this many days are dropped.
pub const KEEP_TURNS_DAYS: i64 = 12 * 7;
/// Days older than this are dropped.
pub const KEEP_DAYS: i64 = 366;

/// The files the history lived in before the database (0.1.x).
const OLD_TURNS: &str = "history.jsonl";
const OLD_DAYS: &str = "days.json";

impl Store {
    /// Keeps a finished turn, and adds it to its day's totals.
    pub fn append_turn(&mut self, e: &Entry) -> Result<(), Error> {
        let tx = self.conn.transaction()?;
        insert_turn(&tx, e)?;
        add_to_day(&tx, &e.day, 1, e.secs)?;
        Ok(tx.commit()?)
    }

    /// Every kept turn, oldest first.
    pub fn turns(&self) -> Result<Vec<Entry>, Error> {
        let mut stmt = self.conn.prepare(
            "SELECT v, end_unix, day, secs, agent, project, steps, commands, files, added, removed,
                    allowed, denied, answered, questions, failed
             FROM turns ORDER BY end_unix, id",
        )?;
        let rows = stmt.query_map([], entry)?;
        // A row whose agent this version does not know is skipped, never an error for the rest.
        Ok(rows.filter_map(Result::ok).flatten().collect())
    }

    /// Each kept day's totals.
    pub fn days(&self) -> Result<BTreeMap<String, DayTotal>, Error> {
        let mut stmt = self.conn.prepare("SELECT day, turns, secs FROM days")?;
        let rows = stmt.query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                DayTotal {
                    turns: r.get(1)?,
                    secs: u64::try_from(r.get::<_, i64>(2)?).unwrap_or(0),
                },
            ))
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    /// Drops the turns past 12 weeks and the days past a year, counted back from `today`.
    pub fn prune_history(&mut self, today: Date) -> Result<(), Error> {
        let tx = self.conn.transaction()?;
        tx.execute(
            "DELETE FROM turns WHERE day < ?1",
            [today.plus(-KEEP_TURNS_DAYS).iso()],
        )?;
        tx.execute("DELETE FROM days WHERE day < ?1", [today.plus(-KEEP_DAYS).iso()])?;
        Ok(tx.commit()?)
    }

    /// Removes the whole history, and the old files if any are left.
    pub fn clear_history(&mut self, dir: &Path) -> Result<(), Error> {
        let tx = self.conn.transaction()?;
        tx.execute_batch("DELETE FROM turns; DELETE FROM days;")?;
        tx.commit()?;
        self.compact()?;
        remove_old(dir)
    }

    /// Copies `history.jsonl` and `days.json` in, once, then removes them. A line that does not
    /// read is skipped: one bad line never costs the rest. A file that cannot be read at all stays
    /// where it is, and nothing is copied. Returns how many turns came in.
    ///
    /// Everything from checking the marker to writing it runs under one write lock taken first:
    /// two windows importing at once copy the history once (the second finds the marker).
    pub fn import_old_history(&mut self, dir: &Path) -> Result<usize, Error> {
        let turns_file = dir.join(OLD_TURNS);
        let days_file = dir.join(OLD_DAYS);
        if !turns_file.exists() && !days_file.exists() {
            return Ok(0);
        }
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let count = match read_if_there(&turns_file)? {
            Some(bytes) => import_turns(&tx, &bytes, modified(&turns_file), &days_file)?,
            // No turns file: a days.json alone is copied once (it holds days older than any turn).
            None => {
                import_days_only(&tx, &days_file)?;
                0
            }
        };
        tx.commit()?;
        remove_old(dir)?;
        Ok(count)
    }
}

/// history.jsonl's turns (and, on the first copy, days.json's totals), unless this file was
/// copied already. Runs inside the import's locked transaction.
fn import_turns(tx: &Transaction, bytes: &[u8], mtime: i64, days_file: &Path) -> Result<usize, Error> {
    let copied: Option<i64> = tx
        .query_row("SELECT mtime FROM imports WHERE name = ?1", [OLD_TURNS], |r| {
            r.get(0)
        })
        .optional()?;
    if copied.is_some_and(|at| mtime <= at) {
        // Copied before; the removal did not happen (the app stopped in between).
        return Ok(0);
    }
    let entries: Vec<Entry> = bytes
        .split(|b| *b == b'\n')
        .filter_map(|l| serde_json::from_slice(l).ok())
        .collect();
    // days.json counts only on the first copy: after a downgrade the older Vults wrote a new one
    // holding days already here, so only the new turns' days get their totals added.
    let mut days = if copied.is_none() {
        read_days(days_file)?
    } else {
        BTreeMap::new()
    };
    // A day days.json lacks (missing, broken, or a later copy) is summed from its turns.
    let mut summed = BTreeMap::<String, DayTotal>::new();
    for e in &entries {
        let t = summed.entry(e.day.clone()).or_default();
        t.turns += 1;
        t.secs += e.secs;
    }
    for (day, total) in summed {
        days.entry(day).or_insert(total);
    }
    for e in &entries {
        insert_turn(tx, e)?;
    }
    for (day, total) in &days {
        add_to_day(tx, day, total.turns, total.secs)?;
    }
    tx.execute(
        "INSERT INTO imports (name, mtime) VALUES (?1, ?2)
         ON CONFLICT (name) DO UPDATE SET mtime = ?2",
        params![OLD_TURNS, mtime],
    )?;
    Ok(entries.len())
}

fn import_days_only(tx: &Transaction, days_file: &Path) -> Result<(), Error> {
    let copied = tx
        .query_row("SELECT 1 FROM imports WHERE name = ?1", [OLD_DAYS], |_| Ok(()))
        .optional()?
        .is_some();
    if copied {
        return Ok(());
    }
    for (day, total) in &read_days(days_file)? {
        add_to_day(tx, day, total.turns, total.secs)?;
    }
    tx.execute(
        "INSERT INTO imports (name, mtime) VALUES (?1, ?2)",
        params![OLD_DAYS, modified(days_file)],
    )?;
    Ok(())
}

/// days.json's totals; none when it is missing or does not read as JSON. An I/O error is returned.
fn read_days(path: &Path) -> Result<BTreeMap<String, DayTotal>, Error> {
    Ok(read_if_there(path)?
        .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok())
        .and_then(|v| serde_json::from_value(v.get("days")?.clone()).ok())
        .unwrap_or_default())
}

/// The file's bytes; none when it is not there. Any other error is returned: a file we could not
/// read is never taken for an empty one (and then removed).
fn read_if_there(path: &Path) -> Result<Option<Vec<u8>>, Error> {
    match std::fs::read(path) {
        Ok(b) => Ok(Some(b)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// Modification time in Unix seconds; 0 when the system can't say.
fn modified(path: &Path) -> i64 {
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(0))
}

fn insert_turn(tx: &Transaction, e: &Entry) -> Result<(), Error> {
    tx.execute(
        "INSERT INTO turns (v, end_unix, day, secs, agent, project, steps, commands, files, added,
                            removed, allowed, denied, answered, questions, failed)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
        params![
            e.v,
            e.end,
            e.day,
            i64::try_from(e.secs).unwrap_or(i64::MAX),
            agent_name(e.agent),
            e.project,
            e.steps,
            e.commands,
            e.files,
            e.added,
            e.removed,
            e.allowed,
            e.denied,
            e.answered,
            e.questions,
            e.failed,
        ],
    )?;
    Ok(())
}

fn add_to_day(tx: &Transaction, day: &str, turns: u32, secs: u64) -> Result<(), Error> {
    tx.execute(
        "INSERT INTO days (day, turns, secs) VALUES (?1, ?2, ?3)
         ON CONFLICT (day) DO UPDATE SET turns = turns + ?2, secs = secs + ?3",
        params![day, turns, i64::try_from(secs).unwrap_or(i64::MAX)],
    )?;
    Ok(())
}

/// A row as an entry; none when its agent is one this version does not know.
fn entry(r: &Row) -> rusqlite::Result<Option<Entry>> {
    let Some(agent) = agent_kind(&r.get::<_, String>(4)?) else {
        return Ok(None);
    };
    Ok(Some(Entry {
        v: r.get(0)?,
        end: r.get(1)?,
        day: r.get(2)?,
        secs: u64::try_from(r.get::<_, i64>(3)?).unwrap_or(0),
        agent,
        project: r.get(5)?,
        steps: r.get(6)?,
        commands: r.get(7)?,
        files: r.get(8)?,
        added: r.get(9)?,
        removed: r.get(10)?,
        allowed: r.get(11)?,
        denied: r.get(12)?,
        answered: r.get(13)?,
        questions: r.get(14)?,
        failed: r.get(15)?,
    }))
}

/// The agent as `history.jsonl` wrote it (its serde name), so the import and the table agree.
fn agent_name(agent: AgentKind) -> String {
    serde_json::to_value(agent)
        .ok()
        .and_then(|v| v.as_str().map(str::to_owned))
        .unwrap_or_default()
}

fn agent_kind(name: &str) -> Option<AgentKind> {
    serde_json::from_value(serde_json::Value::String(name.to_owned())).ok()
}

pub(crate) fn remove_old(dir: &Path) -> Result<(), Error> {
    for name in [OLD_TURNS, OLD_DAYS] {
        match std::fs::remove_file(dir.join(name)) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => return Err(e.into()),
            _ => {}
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use crate::temp;
    use std::time::Duration;
    use vults_core::turns::Turn;

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
    const DAY: i64 = 24 * 3600;

    #[test]
    fn a_turn_is_kept_and_read_back_with_its_day() {
        let mut s = Store::open(&temp("h-append")).unwrap();
        let a = Entry::new(&turn(60), NOON, 0);
        let b = Entry::new(&turn(30), NOON + 3600, -3 * 3600);
        s.append_turn(&a).unwrap();
        s.append_turn(&b).unwrap();
        assert_eq!(s.turns().unwrap(), vec![a, b]);
        assert_eq!(s.days().unwrap()["2026-10-09"], DayTotal { turns: 2, secs: 90 });
    }

    #[test]
    fn every_agent_reads_back_as_itself() {
        for agent in [
            AgentKind::Claude,
            AgentKind::Codex,
            AgentKind::Gemini,
            AgentKind::OpenCode,
            AgentKind::Qwen,
            AgentKind::Other,
        ] {
            assert_eq!(agent_kind(&agent_name(agent)), Some(agent), "{agent:?}");
        }
        assert_eq!(agent_kind("an-agent-from-the-future"), None);
    }

    #[test]
    fn prune_keeps_twelve_weeks_of_turns_and_a_year_of_days() {
        let mut s = Store::open(&temp("h-prune")).unwrap();
        for ago in [0, 80, 90, 300, 400] {
            s.append_turn(&Entry::new(&turn(60), NOON - ago * DAY, 0))
                .unwrap();
        }
        s.prune_history(Date::new(2026, 10, 9)).unwrap();
        assert_eq!(s.turns().unwrap().len(), 2);
        assert_eq!(s.days().unwrap().len(), 4, "a year of days");
    }

    #[test]
    fn clear_leaves_nothing() {
        let dir = temp("h-clear");
        let mut s = Store::open(&dir).unwrap();
        s.append_turn(&Entry::new(&turn(60), NOON, 0)).unwrap();
        std::fs::write(dir.join(OLD_DAYS), "{}").unwrap();
        s.clear_history(&dir).unwrap();
        assert!(s.turns().unwrap().is_empty() && s.days().unwrap().is_empty());
        assert!(!dir.join(OLD_DAYS).exists());
    }

    /// Writes the 0.1.x files as app/src/history.rs did.
    fn old_files(dir: &Path, entries: &[Entry], days: &str) {
        std::fs::create_dir_all(dir).unwrap();
        let mut text = String::new();
        for e in entries {
            text.push_str(&serde_json::to_string(e).unwrap());
            text.push('\n');
        }
        text.push_str("not json\n");
        std::fs::write(dir.join(OLD_TURNS), text).unwrap();
        std::fs::write(dir.join(OLD_DAYS), days).unwrap();
    }

    #[test]
    fn the_old_files_come_in_once_and_go() {
        let dir = temp("h-import");
        let a = Entry::new(&turn(60), NOON, 0);
        let b = Entry::new(&turn(30), NOON - DAY, 0);
        // A day older than the turns kept: days.json holds a year, history.jsonl 12 weeks.
        old_files(
            &dir,
            &[b.clone(), a.clone()],
            r#"{"v":1,"days":{"2026-01-02":{"turns":5,"secs":900},"2026-10-08":{"turns":1,"secs":30},"2026-10-09":{"turns":1,"secs":60}}}"#,
        );
        let mut s = Store::open(&dir).unwrap();
        assert_eq!(s.import_old_history(&dir).unwrap(), 2);
        assert_eq!(s.turns().unwrap(), vec![b, a]);
        let days = s.days().unwrap();
        assert_eq!(days.len(), 3);
        assert_eq!(days["2026-01-02"], DayTotal { turns: 5, secs: 900 });
        assert!(!dir.join(OLD_TURNS).exists() && !dir.join(OLD_DAYS).exists());
        // Nothing left to import: nothing doubles.
        assert_eq!(s.import_old_history(&dir).unwrap(), 0);
        assert_eq!(s.turns().unwrap().len(), 2);
    }

    #[test]
    fn files_left_after_an_import_are_removed_not_copied_again() {
        let dir = temp("h-reimport");
        let a = Entry::new(&turn(60), NOON, 0);
        old_files(
            &dir,
            std::slice::from_ref(&a),
            r#"{"v":1,"days":{"2026-10-09":{"turns":1,"secs":60}}}"#,
        );
        let mut s = Store::open(&dir).unwrap();
        s.import_old_history(&dir).unwrap();
        // As if the app died after the copy, before the removal.
        old_files(
            &dir,
            &[a],
            r#"{"v":1,"days":{"2026-10-09":{"turns":1,"secs":60}}}"#,
        );
        assert_eq!(s.import_old_history(&dir).unwrap(), 0);
        assert_eq!(s.turns().unwrap().len(), 1);
        assert_eq!(s.days().unwrap()["2026-10-09"].turns, 1);
        assert!(!dir.join(OLD_TURNS).exists());
    }

    #[test]
    fn a_missing_or_broken_days_file_still_brings_the_turns() {
        let dir = temp("h-nodays");
        std::fs::create_dir_all(&dir).unwrap();
        let a = Entry::new(&turn(60), NOON, 0);
        std::fs::write(dir.join(OLD_TURNS), serde_json::to_string(&a).unwrap() + "\n").unwrap();
        let mut s = Store::open(&dir).unwrap();
        assert_eq!(s.import_old_history(&dir).unwrap(), 1);
        assert_eq!(s.turns().unwrap(), vec![a]);
        assert_eq!(s.days().unwrap()["2026-10-09"], DayTotal { turns: 1, secs: 60 });
    }

    #[test]
    fn a_cut_last_line_costs_only_itself() {
        let dir = temp("h-cut");
        std::fs::create_dir_all(&dir).unwrap();
        let a = Entry::new(&turn(60), NOON, 0);
        let mut bytes = (serde_json::to_string(&a).unwrap() + "\n").repeat(2).into_bytes();
        // A power cut mid-character: invalid UTF-8 at the end.
        bytes.extend_from_slice(b"{\"v\":1,\"project\":\"caf\xc3");
        std::fs::write(dir.join(OLD_TURNS), bytes).unwrap();
        let mut s = Store::open(&dir).unwrap();
        assert_eq!(s.import_old_history(&dir).unwrap(), 2);
        assert_eq!(s.turns().unwrap().len(), 2);
    }

    #[cfg(unix)]
    #[test]
    fn a_file_that_cannot_be_read_stays_and_nothing_is_copied() {
        use std::os::unix::fs::PermissionsExt;
        let dir = temp("h-unreadable");
        let a = Entry::new(&turn(60), NOON, 0);
        old_files(&dir, &[a], r#"{"v":1,"days":{}}"#);
        let file = dir.join(OLD_TURNS);
        std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o000)).unwrap();
        if std::fs::read(&file).is_ok() {
            return; // Running as root: nothing to show.
        }
        let mut s = Store::open(&dir).unwrap();
        assert!(s.import_old_history(&dir).is_err());
        assert!(file.exists());
        assert!(s.turns().unwrap().is_empty());
        std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o600)).unwrap();
        assert_eq!(
            s.import_old_history(&dir).unwrap(),
            1,
            "it comes in once it reads"
        );
    }

    #[test]
    fn a_newer_file_after_a_downgrade_comes_in_too() {
        let dir = temp("h-downgrade");
        let a = Entry::new(&turn(60), NOON - DAY, 0);
        old_files(
            &dir,
            std::slice::from_ref(&a),
            r#"{"v":1,"days":{"2026-10-08":{"turns":1,"secs":60}}}"#,
        );
        let mut s = Store::open(&dir).unwrap();
        s.import_old_history(&dir).unwrap();
        // The older Vults starts new files: only the turn kept meanwhile, and its day.
        std::thread::sleep(std::time::Duration::from_millis(1100));
        let b = Entry::new(&turn(30), NOON, 0);
        old_files(
            &dir,
            std::slice::from_ref(&b),
            r#"{"v":1,"days":{"2026-10-09":{"turns":1,"secs":30}}}"#,
        );
        assert_eq!(s.import_old_history(&dir).unwrap(), 1);
        assert_eq!(s.turns().unwrap(), vec![a, b]);
        let days = s.days().unwrap();
        assert_eq!(days["2026-10-08"], DayTotal { turns: 1, secs: 60 });
        assert_eq!(days["2026-10-09"], DayTotal { turns: 1, secs: 30 });
    }

    #[test]
    fn windows_importing_at_once_copy_the_history_once() {
        for round in 0..10 {
            let dir = temp(&format!("h-race-{round}"));
            let entries: Vec<Entry> = (0..300)
                .map(|i| Entry::new(&turn(10), NOON - i * 60, 0))
                .collect();
            old_files(
                &dir,
                &entries,
                r#"{"v":1,"days":{"2026-10-09":{"turns":300,"secs":3000}}}"#,
            );
            let importers: Vec<_> = (0..4)
                .map(|_| {
                    let dir = dir.clone();
                    std::thread::spawn(move || {
                        let mut s = Store::open(&dir).unwrap();
                        s.import_old_history(&dir).unwrap()
                    })
                })
                .collect();
            let copied: usize = importers.into_iter().map(|t| t.join().unwrap()).sum();
            assert_eq!(copied, 300, "round {round}");
            let s = Store::open(&dir).unwrap();
            assert_eq!(s.turns().unwrap().len(), 300);
            assert_eq!(
                s.days().unwrap()["2026-10-09"],
                DayTotal {
                    turns: 300,
                    secs: 3000
                }
            );
        }
    }
}
