//! The local database (`docs/dev/plan-zeca.md`, S1): one `vults.sqlite` in the data folder, the
//! user's alone, upgraded by numbered migrations. Each table arrives with the step that needs it;
//! today the history of turns ([`history`]).

pub mod history;

use std::path::{Path, PathBuf};
use std::time::Duration;

use rusqlite::Connection;

pub use rusqlite::Error;

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
    -- Old files already copied in: a crash between the copy and their removal never copies twice.
    CREATE TABLE imports (name TEXT PRIMARY KEY);",
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
        std::fs::create_dir_all(dir).map_err(io)?;
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
                .open(&path)
                .map_err(io)?;
        }
        let conn = Connection::open(&path)?;
        conn.busy_timeout(Duration::from_secs(2))?;
        // WAL survives a crash mid-write and lets a reader run beside the writer.
        conn.pragma_update(None, "journal_mode", "WAL")?;
        let mut store = Self { conn };
        store.migrate()?;
        Ok(store)
    }

    fn migrate(&mut self) -> Result<(), Error> {
        let have: usize = self
            .conn
            .pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))?
            .try_into()
            .unwrap_or(0);
        if have > MIGRATIONS.len() {
            return Err(Error::InvalidQuery);
        }
        for (i, sql) in MIGRATIONS.iter().enumerate().skip(have) {
            let tx = self.conn.transaction()?;
            tx.execute_batch(sql)?;
            tx.pragma_update(None, "user_version", i64::try_from(i + 1).unwrap_or(i64::MAX))?;
            tx.commit()?;
        }
        Ok(())
    }

    pub fn schema_version(&self) -> Result<i64, Error> {
        self.conn.pragma_query_value(None, "user_version", |r| r.get(0))
    }
}

pub fn path(dir: &Path) -> PathBuf {
    dir.join(FILE)
}

fn io(e: std::io::Error) -> Error {
    Error::ToSqlConversionFailure(Box::new(e))
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
    fn a_newer_database_is_refused() {
        let dir = temp("newer");
        let store = Store::open(&dir).unwrap();
        store.conn.pragma_update(None, "user_version", 999).unwrap();
        drop(store);
        assert!(Store::open(&dir).is_err());
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
