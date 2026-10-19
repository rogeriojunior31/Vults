//! The local database (`docs/dev/plan-zeca.md`, S1): one `vults.sqlite` in the data folder, the
//! user's alone, upgraded by numbered migrations. Each table arrives with the step that needs it;
//! the history of turns ([`history`]), the audit log ([`audit`]), the user's consents
//! ([`consents`]), traces ([`traces`]), Zeca's conversations ([`conversations`]) and the
//! text-only dictation history API ([`dictations`]).

pub mod audit;
pub mod consents;
pub mod conversations;
pub mod devices;
pub mod dictations;
pub mod history;
pub mod journal;
pub mod meetings;
pub mod proposals;
pub mod recall;
pub mod requests;
mod retired_tasks;
pub mod sleep;
pub mod spend;
pub mod styles;
pub mod traces;
pub mod words;

use std::path::{Path, PathBuf};
use std::time::Duration;

use rusqlite::{Connection, ErrorCode, TransactionBehavior};

/// Why the database could not do what was asked.
#[derive(Debug)]
pub enum Error {
    Sqlite(rusqlite::Error),
    Io(std::io::Error),
    /// The file was written by a newer Vults (its schema version); never written with an older one.
    Newer(i64),
    /// A device's counter not saved: no such device, or it already used one this high.
    Counter,
    /// Opened without migrating (Zeca's MCP server), and the file is at another schema.
    Schema(i64),
    /// A value refused before it was written.
    Invalid(&'static str),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sqlite(e) => write!(f, "{FILE}: {e}"),
            Self::Io(e) => write!(f, "{FILE}: {e}"),
            Self::Newer(v) => write!(f, "{FILE} is from a newer Vults (schema {v})"),
            Self::Schema(v) => write!(f, "{FILE} is at schema {v}, not this Vults' own: restart the app"),
            Self::Invalid(why) => write!(f, "{why}"),
            Self::Counter => write!(
                f,
                "{FILE}: the device is unknown, or its counter is past this one"
            ),
        }
    }
}

impl std::error::Error for Error {}

impl From<rusqlite::Error> for Error {
    fn from(e: rusqlite::Error) -> Self {
        Self::Sqlite(e)
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

pub const FILE: &str = "vults.sqlite";

/// One entry per schema version, applied in order; `PRAGMA user_version` says how many ran.
/// Never edit one that shipped: add the next.
const MIGRATIONS: &[&str] = &[
    // 1: the history of turns (was history.jsonl) and each day's totals (was days.json).
    "CREATE TABLE turns (
        id INTEGER PRIMARY KEY,
        v INTEGER NOT NULL,
        end_unix INTEGER NOT NULL,
        day TEXT NOT NULL,
        secs INTEGER NOT NULL,
        agent TEXT NOT NULL,
        project TEXT NOT NULL,
        steps INTEGER NOT NULL,
        commands INTEGER NOT NULL,
        files INTEGER NOT NULL,
        added INTEGER NOT NULL,
        removed INTEGER NOT NULL,
        allowed INTEGER NOT NULL,
        denied INTEGER NOT NULL,
        answered INTEGER NOT NULL,
        questions INTEGER NOT NULL,
        failed INTEGER NOT NULL
    );
    CREATE INDEX turns_day ON turns (day);
    CREATE TABLE days (
        day TEXT PRIMARY KEY,
        turns INTEGER NOT NULL,
        secs INTEGER NOT NULL
    );
    -- Old files already copied in, and the file's modification time then: a crash between the
    -- copy and their removal never copies twice, while a newer file (written by an older Vults
    -- after a downgrade) still comes in.
    CREATE TABLE imports (name TEXT PRIMARY KEY, mtime INTEGER NOT NULL);",
    // 2: the audit log (ADR 0004): who answered each card, how, and on what.
    audit::TABLE,
    // 3: traces, one span per model call, tool and action (S6).
    traces::TABLE,
    // 4: consents, apart from the audit (S13), and the audit's guard against INSERT OR REPLACE.
    consents::TABLE,
    // 5: Zeca's conversations and what was said in them (Z4).
    conversations::TABLE,
    // 6: paired phones and the highest counter each signed with (L1).
    devices::TABLE,
    // 7: what a conversation read from outside (Z8).
    conversations::OUTSIDE,
    // 8: the frozen context each conversation began with (M2).
    conversations::FROZEN,
    // 9: Zeca's journal and each day's summary (M4).
    journal::TABLE,
    // 10: recall over it: full-text indexes and the summaries' vectors (M5).
    recall::TABLE,
    // 11: what Zeca would remember, waiting for the user's yes (M6).
    proposals::TABLE,
    // 12: Zeca's nights: what each sleep did, and its debrief (M8).
    sleep::TABLE,
    // 13: the consent of the policy that answered, beside each audit line (W2).
    audit::CONSENT,
    // 14: the user's tasks and their criteria (W3).
    retired_tasks::TABLE,
    // 15: what a task's last session said, and its id (W4).
    retired_tasks::RUNS,
    // 16: the context pack a task's last handoff left (W5).
    retired_tasks::PACK,
    // 17: a task's last review: its words, verdict and round (W6).
    retired_tasks::REVIEW,
    // 18: why the router sent a task where it went (W8).
    retired_tasks::ROUTE,
    // 19: what the agents cost, by day, source and roost (W9).
    spend::TABLE,
    // 20: the tokens a task's runs used (W9).
    retired_tasks::TOKENS,
    // 21: text-only dictation history and independent usage totals (N3 V1.13).
    dictations::TABLE,
    // 22: user-maintained dictation dictionary, replacements and snippets (N3 V2.1).
    words::TABLE,
    // 23: nullable stage timings on retained delivered dictations (N3 V4.1).
    dictations::TIMINGS,
    // 24: content-free daily application word totals, independent of transcript retention.
    dictations::statistics::TABLE,
    // 25: durable model-attempt quota, independent of erasable history/traces.
    requests::TABLE,
    // 26: per-app style preferences and retained output language.
    styles::TABLE,
    // 27: explicitly saved, text-only meetings and source-local speakers.
    meetings::TABLE,
];

/// An open database. Short-lived: open, do one thing, drop; SQLite's own locking keeps two
/// writers apart.
#[derive(Debug)]
pub struct Store {
    conn: Connection,
}

impl Store {
    /// Opens (or creates) `vults.sqlite` in `dir` and brings it to the latest schema. A database
    /// from a newer version is refused rather than written with an older schema.
    pub fn open(dir: &Path) -> Result<Self, Error> {
        let mut store = Self::connect(dir)?;
        store.migrate()?;
        Ok(store)
    }

    /// Opens a database the running app already brought to this schema, never migrating it: a
    /// second process (Zeca's MCP server, maybe a newer binary after an update) must not move the
    /// schema under the app.
    pub fn open_current(dir: &Path) -> Result<Self, Error> {
        if !path(dir).exists() {
            return Err(Error::Schema(0));
        }
        let store = Self::connect(dir)?;
        let have = store.schema_version()?;
        if have != i64::try_from(MIGRATIONS.len()).unwrap_or(i64::MAX) {
            return Err(Error::Schema(have));
        }
        Ok(store)
    }

    fn connect(dir: &Path) -> Result<Self, Error> {
        std::fs::create_dir_all(dir)?;
        let path = path(dir);
        // Created 0600 before SQLite opens it: it names the user's projects.
        #[cfg(unix)]
        if !path.exists() {
            use std::os::unix::fs::OpenOptionsExt;
            std::fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(false)
                .mode(0o600)
                .open(&path)?;
        }
        let conn = Connection::open(&path)?;
        conn.busy_timeout(Duration::from_secs(2))?;
        // A REPLACE deletes the row it replaces: with this, that fires the delete guards too.
        conn.pragma_update(None, "recursive_triggers", true)?;
        conn.pragma_update(None, "foreign_keys", true)?;
        // WAL survives a crash mid-write and lets a reader run beside the writer. Switching a new
        // file needs it to itself, which the busy handler does not always wait for: retry.
        let mut tries = 0;
        loop {
            match conn.pragma_update(None, "journal_mode", "WAL") {
                Err(e) if busy(&e) && tries < 40 => {
                    tries += 1;
                    std::thread::sleep(Duration::from_millis(50));
                }
                other => break other?,
            }
        }
        Ok(Self { conn })
    }

    /// Every migration not yet run, under one write lock taken up front: two windows opening a new
    /// file at once never both run the same one (the second sees the version the first wrote).
    fn migrate(&mut self) -> Result<(), Error> {
        let latest = i64::try_from(MIGRATIONS.len()).unwrap_or(i64::MAX);
        if self.schema_version()? == latest {
            return Ok(());
        }
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let have: i64 = tx.pragma_query_value(None, "user_version", |r| r.get(0))?;
        if have > latest {
            return Err(Error::Newer(have));
        }
        for (i, sql) in MIGRATIONS
            .iter()
            .enumerate()
            .skip(usize::try_from(have).unwrap_or(0))
        {
            tx.execute_batch(sql)?;
            tx.pragma_update(None, "user_version", i64::try_from(i + 1).unwrap_or(i64::MAX))?;
        }
        tx.commit()?;
        Ok(())
    }

    /// After removing rows the user asked to remove: the freed pages are rewritten and the
    /// write-ahead log emptied, so nothing of them lingers in either file.
    pub(crate) fn compact(&mut self) -> Result<(), Error> {
        self.conn
            .execute_batch("VACUUM; PRAGMA wal_checkpoint(TRUNCATE);")?;
        Ok(())
    }

    /// Settings → Activity → Clear history: the history, the audit log, the traces, Zeca's
    /// conversations and his journal, in one transaction (all go, or none), then compacted; the
    /// 0.1.x files too if any are left. Dictation text goes too; usage totals and quota admissions stay.
    pub fn clear_all(&mut self, dir: &Path) -> Result<(), Error> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        tx.execute_batch(
            "DELETE FROM turns; DELETE FROM days; DELETE FROM traces;
             DELETE FROM chat_messages; DELETE FROM conversations;
             DELETE FROM journal; DELETE FROM journal_days; DELETE FROM recall_vectors;
             DELETE FROM meetings; DELETE FROM memory_proposals; DELETE FROM sleep_runs; DELETE FROM dictations;",
        )?;
        audit::clear_audit_in(&tx)?;
        tx.commit()?;
        self.compact()?;
        history::remove_old(dir)
    }

    pub fn schema_version(&self) -> Result<i64, Error> {
        Ok(self.conn.pragma_query_value(None, "user_version", |r| r.get(0))?)
    }
}

pub fn path(dir: &Path) -> PathBuf {
    dir.join(FILE)
}

fn busy(e: &rusqlite::Error) -> bool {
    matches!(
        e.sqlite_error_code(),
        Some(ErrorCode::DatabaseBusy | ErrorCode::DatabaseLocked)
    )
}

#[cfg(test)]
pub(crate) fn temp(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("vults-store-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn a_new_database_reaches_the_latest_schema_once() {
        let dir = temp("migrate");
        let store = Store::open(&dir).unwrap();
        assert_eq!(store.schema_version().unwrap(), MIGRATIONS.len() as i64);
        drop(store);
        // Opening again runs nothing twice.
        assert_eq!(
            Store::open(&dir).unwrap().schema_version().unwrap(),
            MIGRATIONS.len() as i64
        );
    }

    #[test]
    fn retired_rows_survive_an_upgrade_and_legacy_spend_still_counts() {
        let dir = temp("retired-upgrade");
        std::fs::create_dir_all(&dir).unwrap();
        let conn = Connection::open(path(&dir)).unwrap();
        for sql in &MIGRATIONS[..14] {
            conn.execute_batch(sql).unwrap();
        }
        conn.execute(
            "INSERT INTO tasks (created_ms, updated_ms, goal) VALUES (1, 1, 'Kept work')",
            [],
        )
        .unwrap();
        conn.pragma_update(None, "user_version", 14).unwrap();
        drop(conn);
        let mut store = Store::open(&dir).unwrap();
        assert_eq!(store.schema_version().unwrap(), MIGRATIONS.len() as i64);
        assert_eq!(
            store
                .conn
                .query_row("SELECT goal FROM tasks", [], |r| r.get::<_, String>(0))
                .unwrap(),
            "Kept work"
        );
        store
            .spend_add(
                &spend::Source {
                    day: "2026-10-16",
                    source: "task:1",
                    roost: "project",
                    plan: "subscription",
                },
                1.25,
                100,
            )
            .unwrap();
        drop(store);
        let store = Store::open(&dir).unwrap();
        assert_eq!(store.schema_version().unwrap(), MIGRATIONS.len() as i64);
        assert_eq!(store.spend_day("2026-10-16").unwrap().usd, 1.25);
        assert_eq!(store.spend_roosts("2026-10-16").unwrap()[0].today, 1.25);
    }

    #[test]
    fn a_newer_database_is_refused() {
        let dir = temp("newer");
        let store = Store::open(&dir).unwrap();
        store.conn.pragma_update(None, "user_version", 999).unwrap();
        drop(store);
        assert!(matches!(Store::open(&dir), Err(Error::Newer(999))));
    }

    #[test]
    fn windows_opening_a_new_file_at_once_all_succeed() {
        for round in 0..20 {
            let dir = temp(&format!("race-{round}"));
            let openers: Vec<_> = (0..6)
                .map(|_| {
                    let dir = dir.clone();
                    std::thread::spawn(move || Store::open(&dir).map(|s| s.schema_version().unwrap()))
                })
                .collect();
            for o in openers {
                assert_eq!(o.join().unwrap().unwrap(), MIGRATIONS.len() as i64);
            }
        }
    }

    #[cfg(unix)]
    #[test]
    fn the_file_is_the_users_alone() {
        use std::os::unix::fs::PermissionsExt;
        let dir = temp("mode");
        Store::open(&dir).unwrap();
        let mode = std::fs::metadata(path(&dir)).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
    }

    #[test]
    fn full_text_search_is_built_in() {
        let store = Store::open(&temp("fts")).unwrap();
        store
            .conn
            .execute_batch(
                "CREATE VIRTUAL TABLE t USING fts5(body); INSERT INTO t VALUES ('a roost of vultures');",
            )
            .unwrap();
        let hits: i64 = store
            .conn
            .query_row("SELECT count(*) FROM t WHERE t MATCH 'vultures'", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(hits, 1);
    }
}
