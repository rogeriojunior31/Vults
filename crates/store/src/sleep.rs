//! Zeca's sleep (`docs/dev/plan-zeca.md` M8): one run a day at most, what it did, and the debrief
//! it left for the user. The debrief is a model's reading of the journal's summaries: shown to the
//! user as such, never fed back to a model as instructions.

use rusqlite::{OptionalExtension, params};

use crate::{Error, Store};

/// Migration 12. Shipped: never edit it.
pub(crate) const TABLE: &str = "
    CREATE TABLE sleep_runs (
        day TEXT PRIMARY KEY,
        ms INTEGER NOT NULL,
        dropped INTEGER NOT NULL,
        proposed INTEGER NOT NULL,
        debrief TEXT NOT NULL
    );";

/// Longest debrief kept, in characters.
pub const MAX_DEBRIEF: usize = 800;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Run {
    pub day: String,
    pub ms: i64,
    /// Repeated facts dropped.
    pub dropped: i64,
    /// Merges proposed, waiting for the user.
    pub proposed: i64,
    pub debrief: String,
}

impl Store {
    /// Has the sleep already run on `day`?
    pub fn sleep_ran(&self, day: &str) -> Result<bool, Error> {
        Ok(self.conn.query_row(
            "SELECT EXISTS (SELECT 1 FROM sleep_runs WHERE day = ?1)",
            [day],
            |r| r.get(0),
        )?)
    }

    /// A run, redacted and cut; a second one the same day replaces it.
    pub fn sleep_record(&mut self, run: &Run) -> Result<(), Error> {
        let debrief: String = vults_core::redact::secrets(&run.debrief)
            .chars()
            .take(MAX_DEBRIEF)
            .collect();
        self.conn.execute(
            "INSERT INTO sleep_runs (day, ms, dropped, proposed, debrief) VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT (day) DO UPDATE SET ms = excluded.ms, dropped = excluded.dropped,
             proposed = excluded.proposed, debrief = excluded.debrief",
            params![run.day, run.ms, run.dropped, run.proposed, debrief],
        )?;
        // A quarter of a year of nights is plenty for the history of a few numbers.
        self.conn.execute(
            "DELETE FROM sleep_runs WHERE day NOT IN (SELECT day FROM sleep_runs ORDER BY day DESC LIMIT 90)",
            [],
        )?;
        Ok(())
    }

    /// The latest run.
    pub fn sleep_last(&self) -> Result<Option<Run>, Error> {
        Ok(self
            .conn
            .query_row(
                "SELECT day, ms, dropped, proposed, debrief FROM sleep_runs ORDER BY day DESC LIMIT 1",
                [],
                |r| {
                    Ok(Run {
                        day: r.get(0)?,
                        ms: r.get(1)?,
                        dropped: r.get(2)?,
                        proposed: r.get(3)?,
                        debrief: r.get(4)?,
                    })
                },
            )
            .optional()?)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn a_night_is_kept_once_redacted_and_the_latest_is_read() {
        let mut s = Store::open(&crate::temp("sleep")).unwrap();
        assert!(!s.sleep_ran("2026-10-11").unwrap());
        let run = |day: &str, debrief: &str| Run {
            day: day.into(),
            ms: 1,
            dropped: 2,
            proposed: 1,
            debrief: debrief.into(),
        };
        s.sleep_record(&run("2026-10-10", "old")).unwrap();
        s.sleep_record(&run(
            "2026-10-11",
            "Pushed with ghp_abcdefghijklmnopqrstuvwxyz0123456789",
        ))
        .unwrap();
        assert!(s.sleep_ran("2026-10-11").unwrap());
        let last = s.sleep_last().unwrap().unwrap();
        assert_eq!(last.day, "2026-10-11");
        assert!(!last.debrief.contains("ghp_abc"), "{}", last.debrief);
    }
}
