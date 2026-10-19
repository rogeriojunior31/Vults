//! Text-only dictation history. Retention never removes the independent usage totals.

use std::{collections::BTreeMap, path::Path};

use rusqlite::{OptionalExtension, Row, params};
use vults_core::looks::Date;

use crate::{Error, Store};

pub(crate) mod statistics;
mod timings;
pub use statistics::{Period, Statistics, Usage};
pub use timings::{Distribution, TimingSummary, Timings};

pub(crate) const TIMINGS: &str = "
    ALTER TABLE dictations ADD COLUMN total_ms INTEGER CHECK(total_ms >= 0);
    ALTER TABLE dictations ADD COLUMN stt_ms INTEGER CHECK(stt_ms >= 0);
    ALTER TABLE dictations ADD COLUMN pipeline_ms INTEGER CHECK(pipeline_ms >= 0);
    ALTER TABLE dictations ADD COLUMN cleanup_ms INTEGER CHECK(cleanup_ms >= 0);
    ALTER TABLE dictations ADD COLUMN insert_ms INTEGER CHECK(insert_ms >= 0);";

pub(crate) const TABLE: &str = "
    CREATE TABLE dictations (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        at INTEGER NOT NULL,
        app TEXT,
        session TEXT,
        heard TEXT NOT NULL,
        text TEXT NOT NULL,
        style TEXT NOT NULL,
        language TEXT,
        secs REAL NOT NULL CHECK(secs >= 0),
        words INTEGER NOT NULL CHECK(words > 0),
        fixes INTEGER CHECK(fixes >= 0),
        outcome TEXT NOT NULL
    );
    CREATE INDEX dictations_at ON dictations (at, id);
    CREATE TABLE dictation_days (
        day TEXT PRIMARY KEY,
        words INTEGER NOT NULL,
        secs REAL NOT NULL,
        count INTEGER NOT NULL
    );
    CREATE TABLE dictation_apps (app TEXT PRIMARY KEY, words INTEGER NOT NULL);";

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Retention {
    NotKept,
    Days7,
    #[default]
    Days30,
    Days365,
    Always,
}

impl Retention {
    fn cutoff(self, now: i64) -> Option<i64> {
        let days = match self {
            Self::NotKept => return Some(i64::MAX),
            Self::Days7 => 7,
            Self::Days30 => 30,
            Self::Days365 => 365,
            Self::Always => return None,
        };
        Some(now.saturating_sub(days * 86_400))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    /// Unix seconds. The caller supplies the local calendar day separately.
    pub at: i64,
    pub app: Option<String>,
    pub session: Option<String>,
    pub heard: String,
    pub text: String,
    pub style: String,
    pub language: Option<String>,
    pub output_language: Option<String>,
    pub secs: f64,
    pub words: i64,
    /// Unknown until a processing stage measures corrections; never an invented zero.
    pub fixes: Option<i64>,
    pub outcome: String,
    pub timings: Timings,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct Day {
    pub words: i64,
    pub secs: f64,
    pub count: i64,
}

fn entry(r: &Row<'_>) -> rusqlite::Result<(i64, Entry)> {
    Ok((
        r.get(0)?,
        Entry {
            at: r.get(1)?,
            app: r.get(2)?,
            session: r.get(3)?,
            heard: r.get(4)?,
            text: r.get(5)?,
            style: r.get(6)?,
            language: r.get(7)?,
            output_language: r.get(17)?,
            secs: r.get(8)?,
            words: r.get(9)?,
            fixes: r.get(10)?,
            outcome: r.get(11)?,
            timings: Timings::read(r, 12)?,
        },
    ))
}

impl Store {
    /// Use this entry point when opening dictation history, including before reading it.
    pub fn open_dictations(dir: &Path, retention: Retention, now: i64) -> Result<Self, Error> {
        let mut store = Self::open(dir)?;
        store.dictations_prune(retention, now)?;
        Ok(store)
    }

    /// A zero-word result is not a dictation. Positive totals survive Not kept and expiry.
    /// The caller retains Copy again's text in memory; this store never persists it in that mode.
    pub fn dictation_add(
        &mut self,
        e: &Entry,
        day: Date,
        retention: Retention,
        now: i64,
    ) -> Result<Option<i64>, Error> {
        if e.words < 0
            || !e.timings.valid()
            || e.at < 0
            || !e.secs.is_finite()
            || e.secs < 0.0
            || e.fixes.is_some_and(|n| n < 0)
            || !matches!(
                e.outcome.as_str(),
                "copied" | "pasted" | "session" | "held" | "command"
            )
        {
            return Err(Error::Invalid("Invalid dictation measurements or outcome."));
        }
        if e.heard.len() > 256 * 1024
            || e.text.len() > 256 * 1024
            || [&e.app, &e.session, &e.language, &e.output_language]
                .iter()
                .any(|s| s.as_ref().is_some_and(|s| s.len() > 1024))
            || e.style.len() > 128
        {
            return Err(Error::Invalid("Dictation history fields are too large."));
        }
        // Finish erasure before committing a new row: a compaction error must not look like
        // a failed new write that a caller could retry and count twice.
        self.dictations_prune(retention, now)?;
        if e.words == 0 {
            return Ok(None);
        }
        let tx = self.conn.transaction()?;
        if e.outcome != "command" {
            tx.execute(
                "INSERT INTO dictation_days(day, words, secs, count) VALUES (?1, ?2, ?3, 1)
             ON CONFLICT(day) DO UPDATE SET words = words + excluded.words,
                secs = secs + excluded.secs, count = count + 1",
                params![day.iso(), e.words, e.secs],
            )?;
            if let Some(app) = &e.app {
                tx.execute(
                    "INSERT INTO dictation_apps(app, words) VALUES (?1, ?2)
                 ON CONFLICT(app) DO UPDATE SET words = words + excluded.words",
                    params![app, e.words],
                )?;
            }
            tx.execute(
                "INSERT INTO dictation_app_days(day, app, words) VALUES (?1, ?2, ?3)
             ON CONFLICT(day, app) DO UPDATE SET words = words + excluded.words",
                params![day.iso(), e.app.as_deref().unwrap_or(""), e.words],
            )?;
        }
        let keep =
            retention.cutoff(now).is_none_or(|cutoff| e.at >= cutoff) && retention != Retention::NotKept;
        let id = if keep {
            tx.execute(
                "INSERT INTO dictations(at, app, session, heard, text, style, language, secs, words, fixes, outcome,
                    total_ms, stt_ms, pipeline_ms, cleanup_ms, insert_ms, output_language)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
                params![e.at, e.app, e.session, e.heard, e.text, e.style, e.language,
                    e.secs, e.words, e.fixes, e.outcome, e.timings.total_ms, e.timings.stt_ms,
                    e.timings.pipeline_ms, e.timings.cleanup_ms, e.timings.insert_ms, e.output_language],
            )?;
            Some(tx.last_insert_rowid())
        } else {
            None
        };
        tx.commit()?;
        Ok(id)
    }

    pub fn dictations_prune(&mut self, retention: Retention, now: i64) -> Result<(), Error> {
        let removed = match retention {
            Retention::NotKept => self.conn.execute("DELETE FROM dictations", [])?,
            _ => match retention.cutoff(now) {
                Some(cutoff) => self
                    .conn
                    .execute("DELETE FROM dictations WHERE at < ?1", [cutoff])?,
                None => 0,
            },
        };
        if removed > 0 {
            self.compact()?;
        }
        Ok(())
    }

    /// Newest first, bounded to 500 entries; equal timestamps use descending ids.
    pub fn dictations(&self, limit: u32) -> Result<Vec<(i64, Entry)>, Error> {
        let mut stmt = self.conn.prepare(
            "SELECT id, at, app, session, heard, text, style, language, secs, words, fixes, outcome,
                total_ms, stt_ms, pipeline_ms, cleanup_ms, insert_ms, output_language
             FROM dictations ORDER BY at DESC, id DESC LIMIT ?1",
        )?;
        Ok(stmt
            .query_map([limit.min(500)], entry)?
            .collect::<Result<_, _>>()?)
    }

    pub fn dictation(&self, id: i64) -> Result<Option<Entry>, Error> {
        Ok(self
            .conn
            .query_row(
                "SELECT id, at, app, session, heard, text, style, language, secs, words, fixes, outcome,
                total_ms, stt_ms, pipeline_ms, cleanup_ms, insert_ms, output_language FROM dictations WHERE id = ?1",
                [id],
                entry,
            )
            .optional()?
            .map(|(_, entry)| entry))
    }

    pub fn dictation_delete(&mut self, id: i64) -> Result<(), Error> {
        if self.conn.execute("DELETE FROM dictations WHERE id = ?1", [id])? > 0 {
            self.compact()?;
        }
        Ok(())
    }

    pub fn dictations_clear(&mut self) -> Result<(), Error> {
        self.dictations_prune(Retention::NotKept, 0)
    }

    /// Per-stage nearest-rank percentiles from the newest 50 retained deliveries, without text.
    /// Open with `open_dictations` first so expiry and Not kept apply to this window too.
    pub fn dictation_timings(&self) -> Result<TimingSummary, Error> {
        let mut stmt = self.conn.prepare(
            "SELECT total_ms, stt_ms, pipeline_ms, cleanup_ms, insert_ms FROM dictations
             WHERE outcome IN ('copied', 'pasted', 'session', 'held')
             ORDER BY at DESC, id DESC LIMIT 50",
        )?;
        let rows = stmt
            .query_map([], |r| Timings::read(r, 0))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(TimingSummary::of(&rows))
    }

    pub fn dictation_days(&self) -> Result<BTreeMap<String, Day>, Error> {
        let mut stmt = self
            .conn
            .prepare("SELECT day, words, secs, count FROM dictation_days ORDER BY day")?;
        Ok(stmt
            .query_map([], |r| {
                Ok((
                    r.get(0)?,
                    Day {
                        words: r.get(1)?,
                        secs: r.get(2)?,
                        count: r.get(3)?,
                    },
                ))
            })?
            .collect::<Result<_, _>>()?)
    }

    pub fn dictation_apps(&self) -> Result<BTreeMap<String, i64>, Error> {
        let mut stmt = self
            .conn
            .prepare("SELECT app, words FROM dictation_apps ORDER BY app")?;
        Ok(stmt
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect::<Result<_, _>>()?)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    const NOW: i64 = 2_000_000_000;

    #[test]
    fn output_language_belongs_to_retained_delivered_text_without_relabeling_heard_language() {
        let dir = crate::temp("style-language");
        let mut db = Store::open(&dir).unwrap();
        let mut e = sample(100);
        e.language = Some("pt".into());
        e.output_language = Some("en".into());
        e.style = "prompt".into();
        let id = db
            .dictation_add(
                &e,
                vults_core::looks::Date::of(100, 0),
                crate::dictations::Retention::Days30,
                100,
            )
            .unwrap()
            .unwrap();
        assert_eq!(db.dictation(id).unwrap(), Some(e.clone()));
        assert_eq!(db.dictations(1).unwrap()[0].1, e);
        db.dictations_prune(crate::dictations::Retention::NotKept, 100)
            .unwrap();
        assert!(db.dictation(id).unwrap().is_none());
    }
    pub(super) fn sample(at: i64) -> Entry {
        Entry {
            at,
            app: Some("Editor".into()),
            session: None,
            heard: "Heard example".into(),
            text: "Written example".into(),
            style: "verbatim".into(),
            language: Some("en".into()),
            output_language: None,
            secs: 2.5,
            words: 2,
            fixes: None,
            outcome: "copied".into(),
            timings: Timings::default(),
        }
    }

    fn day() -> Date {
        Date::new(2033, 5, 18)
    }

    #[test]
    fn timing_migration_keeps_legacy_measurements_unknown() {
        let dir = crate::temp("dictation-timing-upgrade");
        std::fs::create_dir_all(&dir).unwrap();
        let conn = rusqlite::Connection::open(crate::path(&dir)).unwrap();
        for sql in &crate::MIGRATIONS[..22] {
            conn.execute_batch(sql).unwrap();
        }
        conn.execute_batch(
            "INSERT INTO dictations(at, heard, text, style, secs, words, outcome)
            VALUES (2000000000, 'old heard', 'old text', 'verbatim', 2, 2, 'copied');
            PRAGMA user_version = 22;",
        )
        .unwrap();
        drop(conn);
        let store = Store::open_dictations(&dir, Retention::Always, NOW).unwrap();
        assert_eq!(store.dictations(1).unwrap()[0].1.timings, Timings::default());
        let summary = store.dictation_timings().unwrap();
        assert_eq!(summary.total.count, 0);
        assert_eq!(summary.total.p50_ms, None);
        assert_eq!(summary.cleanup.p95_ms, None);
    }

    #[test]
    fn timing_window_is_fifty_retained_deliveries_with_independent_known_stages() {
        let dir = crate::temp("dictation-timing-window");
        let mut store = Store::open_dictations(&dir, Retention::Always, NOW).unwrap();
        for n in 0..=50 {
            let mut e = sample(NOW);
            e.timings = Timings {
                total_ms: Some(if n == 0 { 999_999 } else { n }),
                stt_ms: (n % 2 == 0).then_some(n),
                pipeline_ms: Some(0),
                cleanup_ms: None,
                insert_ms: Some(1),
            };
            let id = store
                .dictation_add(&e, day(), Retention::Always, NOW)
                .unwrap()
                .unwrap();
            assert_eq!(store.dictation(id).unwrap().unwrap().timings, e.timings);
        }
        let summary = store.dictation_timings().unwrap();
        assert_eq!(
            (summary.total.count, summary.total.p50_ms, summary.total.p95_ms),
            (50, Some(25), Some(48))
        );
        assert_eq!(
            (summary.stt.count, summary.stt.p50_ms, summary.stt.p95_ms),
            (25, Some(26), Some(48))
        );
        assert_eq!(summary.pipeline.p50_ms, Some(0));
        assert_eq!(summary.cleanup.count, 0);
        assert_eq!(summary.cleanup.p50_ms, None);
        store
            .conn
            .execute_batch(
                "INSERT INTO dictations(at, heard, text, style, secs, words, outcome, total_ms)
            VALUES (2000000001, 'discarded', 'discarded', 'verbatim', 1, 1, 'cancelled', 999999);",
            )
            .unwrap();
        assert_eq!(
            store.dictation_timings().unwrap(),
            summary,
            "Legacy cancelled rows never enter the window"
        );
        for outcome in ["cancelled", "failed", "empty"] {
            let mut e = sample(NOW);
            e.outcome = outcome.into();
            assert!(store.dictation_add(&e, day(), Retention::Always, NOW).is_err());
        }
        let mut invalid = sample(NOW);
        invalid.timings.insert_ms = Some(-1);
        assert!(
            store
                .dictation_add(&invalid, day(), Retention::Always, NOW)
                .is_err()
        );
        assert_eq!(store.dictation_timings().unwrap(), summary);
        store.dictation_delete(store.dictations(1).unwrap()[0].0).unwrap();
        assert_eq!(
            store.dictation_timings().unwrap(),
            summary,
            "Deleting a cancelled row cannot change samples"
        );
        store
            .dictation_add(&sample(NOW), day(), Retention::NotKept, NOW)
            .unwrap();
        assert_eq!(store.dictation_timings().unwrap().total.count, 0);
        assert_eq!(store.dictation_days().unwrap()[&day().iso()].count, 52);
    }

    #[test]
    fn upgrades_schema_20_without_changing_old_data() {
        let dir = crate::temp("dictation-upgrade");
        std::fs::create_dir_all(&dir).unwrap();
        let conn = rusqlite::Connection::open(crate::path(&dir)).unwrap();
        for sql in &crate::MIGRATIONS[..20] {
            conn.execute_batch(sql).unwrap();
        }
        conn.execute("INSERT INTO days VALUES ('2026-10-17', 3, 100)", [])
            .unwrap();
        conn.pragma_update(None, "user_version", 20).unwrap();
        drop(conn);
        for _ in 0..2 {
            let store = Store::open_dictations(&dir, Retention::default(), NOW).unwrap();
            assert_eq!(store.schema_version().unwrap(), crate::MIGRATIONS.len() as i64);
            assert_eq!(store.days().unwrap()["2026-10-17"].turns, 3);
            assert!(store.dictations(20).unwrap().is_empty());
        }
    }

    #[test]
    fn round_trip_totals_order_limits_and_deletion_keep_independent_figures() {
        let dir = crate::temp("dictation-roundtrip");
        let mut store = Store::open_dictations(&dir, Retention::default(), NOW).unwrap();
        let first = sample(NOW);
        let first_id = store
            .dictation_add(&first, day(), Retention::default(), NOW)
            .unwrap()
            .unwrap();
        let mut second = sample(NOW);
        second.app = Some("Terminal".into());
        second.session = Some("codex:session-1".into());
        second.outcome = "session".into();
        second.fixes = Some(0);
        let second_id = store
            .dictation_add(&second, day(), Retention::Always, NOW)
            .unwrap()
            .unwrap();
        assert_eq!(
            store.dictations(20).unwrap(),
            vec![(second_id, second), (first_id, first)]
        );
        assert_eq!(store.dictations(1).unwrap().len(), 1);
        assert!(store.dictations(0).unwrap().is_empty());
        let totals = store.dictation_days().unwrap();
        assert_eq!(
            totals[&day().iso()],
            Day {
                words: 4,
                secs: 5.0,
                count: 2
            }
        );
        let apps = store.dictation_apps().unwrap();
        assert_eq!(
            apps,
            BTreeMap::from([("Editor".into(), 2), ("Terminal".into(), 2)])
        );
        store.dictation_delete(first_id).unwrap();
        store.dictation_delete(first_id).unwrap();
        store.dictations_clear().unwrap();
        let new_id = store
            .dictation_add(&sample(NOW), day(), Retention::Always, NOW)
            .unwrap()
            .unwrap();
        assert!(new_id > second_id, "Deleted ids never target a newer transcript");
        store.dictation_delete(second_id).unwrap();
        assert_eq!(store.dictations(20).unwrap().len(), 1);
        store.clear_all(&dir).unwrap();
        assert!(store.dictations(20).unwrap().is_empty());
        assert_eq!(store.dictation_days().unwrap()[&day().iso()].count, 3);
        assert_eq!(store.dictation_apps().unwrap()["Terminal"], 2);
    }

    #[test]
    fn each_retention_prunes_at_the_boundary_on_open_and_record() {
        for (name, retention, days) in [
            ("week", Retention::Days7, 7),
            ("month", Retention::Days30, 30),
            ("year", Retention::Days365, 365),
        ] {
            let dir = crate::temp(&format!("dictation-{name}"));
            let mut store = Store::open(&dir).unwrap();
            let cutoff = NOW - days * 86_400;
            for at in [cutoff - 1, cutoff] {
                store
                    .dictation_add(&sample(at), day(), Retention::Always, NOW)
                    .unwrap();
            }
            drop(store);
            let mut store = Store::open_dictations(&dir, retention, NOW).unwrap();
            assert_eq!(store.dictations(20).unwrap().len(), 1);
            assert_eq!(store.dictations(20).unwrap()[0].1.at, cutoff);
            store
                .dictation_add(&sample(NOW + 1), day(), retention, NOW + 1)
                .unwrap();
            assert_eq!(
                store.dictations(20).unwrap().len(),
                1,
                "Recording also prunes expired rows"
            );
            assert_eq!(store.dictation_days().unwrap()[&day().iso()].count, 3);
            assert_eq!(store.dictation_apps().unwrap()["Editor"], 6);
            assert_eq!(
                store
                    .dictation_add(&sample(cutoff - 10), day(), retention, NOW)
                    .unwrap(),
                None,
                "An already expired record contributes figures without retaining text"
            );
        }
    }

    #[test]
    fn not_kept_never_writes_transcripts_and_zero_words_add_nothing() {
        let dir = crate::temp("dictation-not-kept");
        let mut store = Store::open(&dir).unwrap();
        let mut e = sample(NOW);
        e.heard = "private-heard-marker-68475".into();
        e.text = "private-written-marker-68475".into();
        e.session = Some("private-session-marker-68475".into());
        assert_eq!(
            store.dictation_add(&e, day(), Retention::NotKept, NOW).unwrap(),
            None
        );
        assert!(store.dictations(20).unwrap().is_empty());
        assert_eq!(store.dictation_days().unwrap()[&day().iso()].words, 2);
        assert_eq!(store.dictation_apps().unwrap()["Editor"], 2);
        e.words = 0;
        store.dictation_add(&e, day(), Retention::Always, NOW).unwrap();
        assert_eq!(store.dictation_days().unwrap()[&day().iso()].count, 1);
        drop(store);
        for file in std::fs::read_dir(&dir).unwrap() {
            let bytes = std::fs::read(file.unwrap().path()).unwrap();
            for secret in [&e.heard, &e.text, e.session.as_ref().unwrap()] {
                assert!(!bytes.windows(secret.len()).any(|w| w == secret.as_bytes()));
            }
        }
    }

    #[test]
    fn not_kept_erases_old_text_on_open_but_keeps_totals() {
        let dir = crate::temp("dictation-erase");
        let mut store = Store::open(&dir).unwrap();
        let mut e = sample(NOW);
        e.text = "sensitive-transcript-marker-493892".into();
        store.dictation_add(&e, day(), Retention::Always, NOW).unwrap();
        drop(store);
        let store = Store::open_dictations(&dir, Retention::NotKept, NOW).unwrap();
        assert!(store.dictations(20).unwrap().is_empty());
        assert_eq!(store.dictation_days().unwrap()[&day().iso()].count, 1);
        drop(store);
        for file in std::fs::read_dir(&dir).unwrap() {
            let bytes = std::fs::read(file.unwrap().path()).unwrap();
            assert!(!bytes.windows(e.text.len()).any(|w| w == e.text.as_bytes()));
        }
    }

    #[test]
    fn bad_measurements_and_failed_inserts_cannot_leave_partial_totals() {
        let mut store = Store::open(&crate::temp("dictation-atomic")).unwrap();
        for secs in [f64::NAN, f64::INFINITY, -1.0] {
            let mut e = sample(NOW);
            e.secs = secs;
            assert!(store.dictation_add(&e, day(), Retention::Always, NOW).is_err());
        }
        for e in [
            Entry {
                words: -1,
                ..sample(NOW)
            },
            Entry {
                fixes: Some(-1),
                ..sample(NOW)
            },
            Entry {
                outcome: "failed".into(),
                ..sample(NOW)
            },
            Entry {
                text: "x".repeat(256 * 1024 + 1),
                ..sample(NOW)
            },
        ] {
            assert!(store.dictation_add(&e, day(), Retention::Always, NOW).is_err());
        }
        store.conn.execute_batch("CREATE TRIGGER fail_dictation BEFORE INSERT ON dictations BEGIN SELECT RAISE(ABORT, 'test failure'); END;").unwrap();
        assert!(
            store
                .dictation_add(&sample(NOW), day(), Retention::Always, NOW)
                .is_err()
        );
        assert!(store.dictations(20).unwrap().is_empty());
        assert!(store.dictation_days().unwrap().is_empty());
        assert!(store.dictation_apps().unwrap().is_empty());
        store.conn.execute_batch("DROP TRIGGER fail_dictation;").unwrap();
        let mut unknown = sample(NOW);
        unknown.app = None;
        store
            .dictation_add(&unknown, day(), Retention::Always, NOW)
            .unwrap();
        assert!(
            store.dictation_apps().unwrap().is_empty(),
            "Unknown apps never become fabricated measurements"
        );
        assert_eq!(store.dictation_days().unwrap()[&day().iso()].count, 1);
    }
    #[test]
    fn commands_follow_retention_without_changing_dictation_usage_or_timings() {
        for retention in [Retention::Always, Retention::NotKept] {
            let mut store = Store::open(&crate::temp("command-history")).unwrap();
            let mut command = sample(NOW);
            command.outcome = "command".into();
            command.style = "command".into();
            command.timings.total_ms = Some(500);
            let id = store.dictation_add(&command, day(), retention, NOW).unwrap();
            assert_eq!(id.is_some(), retention == Retention::Always);
            assert!(store.dictation_days().unwrap().is_empty());
            assert!(store.dictation_apps().unwrap().is_empty());
            assert_eq!(store.dictation_timings().unwrap().total.count, 0);
            if let Some(id) = id {
                assert_eq!(store.dictation(id).unwrap().unwrap(), command);
            }
        }
    }
}
