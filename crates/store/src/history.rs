//! The local history of agent turns (docs/guide/activity.md): each finished turn for 12 weeks, and
//! each day's totals for a year, which the grid reads. Counts and folder names only.

use std::collections::BTreeMap;
use std::path::Path;

use rusqlite::{OptionalExtension, Row, params};
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
        tx.commit()
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
        rows.collect()
    }

    /// Drops the turns past 12 weeks and the days past a year, counted back from `today`.
    pub fn prune_history(&mut self, today: Date) -> Result<(), Error> {
        let tx = self.conn.transaction()?;
        tx.execute(
            "DELETE FROM turns WHERE day < ?1",
            [today.plus(-KEEP_TURNS_DAYS).iso()],
        )?;
        tx.execute("DELETE FROM days WHERE day < ?1", [today.plus(-KEEP_DAYS).iso()])?;
        tx.commit()
    }

    /// Removes the whole history, and the old files if any are left.
    pub fn clear_history(&mut self, dir: &Path) -> Result<(), Error> {
        let tx = self.conn.transaction()?;
        tx.execute_batch("DELETE FROM turns; DELETE FROM days;")?;
        tx.commit()?;
        // The freed pages would still hold the rows until reused.
        self.conn.execute_batch("VACUUM;")?;
        remove_old(dir)
    }

    /// Copies `history.jsonl` and `days.json` in, once, then removes them. A line that does not
    /// read is skipped: one bad line never costs the rest. Returns how many turns came in.
    pub fn import_old_history(&mut self, dir: &Path) -> Result<usize, Error> {
        let turns_file = dir.join(OLD_TURNS);
        let days_file = dir.join(OLD_DAYS);
        if !turns_file.exists() && !days_file.exists() {
            return Ok(0);
        }
        let done = self
            .conn
            .query_row("SELECT 1 FROM imports WHERE name = ?1", [OLD_TURNS], |_| Ok(()))
            .optional()?
            .is_some();
        let mut count = 0;
        if !done {
            let entries: Vec<Entry> = std::fs::read_to_string(&turns_file)
                .unwrap_or_default()
                .lines()
                .filter_map(|l| serde_json::from_str(l).ok())
                .collect();
            let mut days: BTreeMap<String, DayTotal> = std::fs::read_to_string(&days_file)
                .ok()
                .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
                .and_then(|v| serde_json::from_value(v.get("days")?.clone()).ok())
                .unwrap_or_default();
            // A day days.json lacks (the file missing or broken) is summed from its turns.
            let mut summed = BTreeMap::<String, DayTotal>::new();
            for e in &entries {
                let t = summed.entry(e.day.clone()).or_default();
                t.turns += 1;
                t.secs += e.secs;
            }
            for (day, total) in summed {
                days.entry(day).or_insert(total);
            }
            let tx = self.conn.transaction()?;
            for e in &entries {
                insert_turn(&tx, e)?;
            }
            // days.json already counts every turn above, and the year before them.
            for (day, total) in &days {
                add_to_day(&tx, day, total.turns, total.secs)?;
            }
            tx.execute("INSERT INTO imports (name) VALUES (?1)", [OLD_TURNS])?;
            tx.commit()?;
            count = entries.len();
        }
        remove_old(dir)?;
        Ok(count)
    }
}

fn insert_turn(tx: &rusqlite::Transaction, e: &Entry) -> Result<(), Error> {
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

fn add_to_day(tx: &rusqlite::Transaction, day: &str, turns: u32, secs: u64) -> Result<(), Error> {
    tx.execute(
        "INSERT INTO days (day, turns, secs) VALUES (?1, ?2, ?3)
         ON CONFLICT (day) DO UPDATE SET turns = turns + ?2, secs = secs + ?3",
        params![day, turns, i64::try_from(secs).unwrap_or(i64::MAX)],
    )?;
    Ok(())
}

/// A row as an entry; none when its agent is one this version does not know.
fn entry(r: &Row) -> Result<Option<Entry>, Error> {
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

fn remove_old(dir: &Path) -> Result<(), Error> {
    for name in [OLD_TURNS, OLD_DAYS] {
        match std::fs::remove_file(dir.join(name)) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => return Err(crate::io(e)),
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
}
