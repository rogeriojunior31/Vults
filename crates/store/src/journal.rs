//! Zeca's journal (`docs/dev/plan-zeca.md` M4): what the flock and the chat did, a line per event,
//! written only while the user has Zeca's memory on, and each past day's summary. Secrets are
//! redacted here, the last step before the disk, whoever the caller is: a token typed into a
//! command or a chat must never be written. Kept [`KEEP_DAYS`] days.
//!
//! Every line may hold an agent's words (a command, a reply), and a summary is a model's reading
//! of them: whoever puts either in front of Zeca (recall, M5; sleep, M8) gives it as untrusted
//! content, never as instructions.

use rusqlite::{OptionalExtension, params};

use crate::{Error, Store};

pub const KEEP_DAYS: i64 = 90;
/// Longest line kept, in characters: a journal is what happened, not every word of it.
pub const MAX_TEXT: usize = 400;
/// Longest summary kept, in bytes (the board's "under 2 KB").
pub const MAX_SUMMARY: usize = 2_000;

/// Migration 9. Shipped: never edit it.
pub(crate) const TABLE: &str = "
    CREATE TABLE journal (
        id INTEGER PRIMARY KEY,
        ms INTEGER NOT NULL,
        day TEXT NOT NULL,
        kind TEXT NOT NULL,
        agent TEXT NOT NULL,
        project TEXT NOT NULL,
        text TEXT NOT NULL
    );
    CREATE INDEX journal_day ON journal (day, id);
    CREATE TABLE journal_days (
        day TEXT PRIMARY KEY,
        summary TEXT NOT NULL,
        ms INTEGER NOT NULL
    );";

/// One event of the journal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// Unix milliseconds.
    pub ms: i64,
    /// The user's calendar day, `YYYY-MM-DD`.
    pub day: String,
    /// `turn` (an agent's turn ended), `decision` (a card was answered), `chat` (a line said in
    /// Zeca's chat).
    pub kind: String,
    pub agent: String,
    pub project: String,
    pub text: String,
    /// Said in a conversation that read content from outside (Z8): recall and the day's summary
    /// leave it out, so outside words never come back as the user's own memory (M5).
    pub outside: bool,
}

/// Secrets out, then cut at `max` characters (or bytes, for `bytes`), at a character's edge.
fn kept(text: &str, max: usize, bytes: bool) -> String {
    let clean = vults_core::redact::secrets(text);
    let end = if bytes {
        let mut end = clean.len().min(max);
        while !clean.is_char_boundary(end) {
            end -= 1;
        }
        end
    } else {
        clean.char_indices().nth(max).map_or(clean.len(), |(i, _)| i)
    };
    clean[..end].to_string()
}

impl Store {
    pub fn journal_add(&mut self, e: &Entry) -> Result<(), Error> {
        self.conn.execute(
            "INSERT INTO journal (ms, day, kind, agent, project, text, outside)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                e.ms,
                e.day,
                e.kind,
                kept(&e.agent, 64, false),
                kept(&e.project, 128, false),
                kept(&e.text, MAX_TEXT, false),
                e.outside
            ],
        )?;
        Ok(())
    }

    /// One day's events, in order.
    pub fn journal_day(&self, day: &str) -> Result<Vec<Entry>, Error> {
        let mut stmt = self.conn.prepare(
            "SELECT ms, day, kind, agent, project, text, outside FROM journal WHERE day = ?1 ORDER BY id",
        )?;
        let rows = stmt.query_map([day], |r| {
            Ok(Entry {
                ms: r.get(0)?,
                day: r.get(1)?,
                kind: r.get(2)?,
                agent: r.get(3)?,
                project: r.get(4)?,
                text: r.get(5)?,
                outside: r.get(6)?,
            })
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    /// Days from `since` to before `today` with events and no summary yet, the newest `limit`,
    /// oldest first.
    pub fn journal_unsummarized(&self, since: &str, today: &str, limit: u32) -> Result<Vec<String>, Error> {
        let mut stmt = self.conn.prepare(
            "SELECT DISTINCT day FROM journal WHERE day >= ?1 AND day < ?2
             AND day NOT IN (SELECT day FROM journal_days) ORDER BY day DESC LIMIT ?3",
        )?;
        let mut days: Vec<String> = stmt
            .query_map(params![since, today, limit], |r| r.get(0))?
            .collect::<Result<_, _>>()?;
        days.reverse();
        Ok(days)
    }

    /// A day's summary, redacted and cut at [`MAX_SUMMARY`] bytes; a second one replaces it.
    pub fn journal_summary_set(&mut self, day: &str, summary: &str, ms: i64) -> Result<(), Error> {
        self.conn.execute(
            "INSERT INTO journal_days (day, summary, ms) VALUES (?1, ?2, ?3)
             ON CONFLICT (day) DO UPDATE SET summary = excluded.summary, ms = excluded.ms",
            params![day, kept(summary, MAX_SUMMARY, true), ms],
        )?;
        Ok(())
    }

    pub fn journal_summary(&self, day: &str) -> Result<Option<String>, Error> {
        Ok(self
            .conn
            .query_row("SELECT summary FROM journal_days WHERE day = ?1", [day], |r| {
                r.get(0)
            })
            .optional()?)
    }

    /// Drops what is older than [`KEEP_DAYS`] before `cutoff` (a day, `YYYY-MM-DD`).
    pub fn journal_prune(&mut self, cutoff: &str) -> Result<(), Error> {
        let tx = self.conn.transaction()?;
        tx.execute("DELETE FROM journal WHERE day < ?1", [cutoff])?;
        tx.execute("DELETE FROM journal_days WHERE day < ?1", [cutoff])?;
        tx.execute("DELETE FROM recall_vectors WHERE day < ?1", [cutoff])?;
        Ok(tx.commit()?)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    fn store(name: &str) -> Store {
        Store::open(&crate::temp(name)).unwrap()
    }

    fn entry(day: &str, kind: &str, text: &str) -> Entry {
        Entry {
            ms: 1,
            day: day.into(),
            kind: kind.into(),
            agent: "claude".into(),
            project: "vults".into(),
            text: text.into(),
            outside: false,
        }
    }

    #[test]
    fn test_tokens_are_never_written() {
        let mut s = store("journal-secrets");
        let tokens = [
            "ghp_abcdefghijklmnopqrstuvwxyz0123456789",
            "sk-ant-api03-abcdefghijklmnopqrstuvwxyz0123456789",
        ];
        s.journal_add(&entry(
            "2026-10-11",
            "decision",
            &format!("Allowed Bash: GITHUB_TOKEN={} gh pr list", tokens[0]),
        ))
        .unwrap();
        s.journal_add(&entry(
            "2026-10-11",
            "chat",
            &format!("you: my key is {}", tokens[1]),
        ))
        .unwrap();
        s.journal_summary_set("2026-10-11", &format!("Pushed with token {}", tokens[0]), 2)
            .unwrap();
        drop(s);
        // Not in any row, and not anywhere in the database file.
        let raw = std::fs::read(crate::path(&dir("journal-secrets"))).unwrap();
        let raw = String::from_utf8_lossy(&raw);
        for t in tokens {
            assert!(!raw.contains(t), "{t} reached the disk");
        }
        let s = store_again("journal-secrets");
        let day = s.journal_day("2026-10-11").unwrap();
        assert!(day[0].text.contains("[redacted]"), "{}", day[0].text);
        assert!(day[0].text.ends_with("gh pr list"), "the command around it stays");
    }

    /// The same folder `store` made, without emptying it.
    fn dir(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("vults-store-{}-{name}", std::process::id()))
    }

    fn store_again(name: &str) -> Store {
        Store::open(&dir(name)).unwrap()
    }

    #[test]
    fn days_wait_for_their_summary_oldest_first() {
        let mut s = store("journal-days");
        for day in ["2026-10-08", "2026-10-09", "2026-10-10", "2026-10-11"] {
            s.journal_add(&entry(day, "turn", "14 min, 30 steps")).unwrap();
        }
        // Today is never summed up: it isn't over.
        assert_eq!(
            s.journal_unsummarized("2026-01-01", "2026-10-11", 7).unwrap(),
            ["2026-10-08", "2026-10-09", "2026-10-10"]
        );
        assert_eq!(
            s.journal_unsummarized("2026-01-01", "2026-10-11", 2).unwrap(),
            ["2026-10-09", "2026-10-10"],
            "the newest when there are too many"
        );
        assert_eq!(
            s.journal_unsummarized("2026-10-10", "2026-10-11", 7).unwrap(),
            ["2026-10-10"],
            "nothing from before the summary was turned on"
        );
        s.journal_summary_set("2026-10-09", "A day of tests.", 5).unwrap();
        assert_eq!(
            s.journal_unsummarized("2026-01-01", "2026-10-11", 7).unwrap(),
            ["2026-10-08", "2026-10-10"]
        );
        assert_eq!(
            s.journal_summary("2026-10-09").unwrap().as_deref(),
            Some("A day of tests.")
        );
        s.journal_prune("2026-10-10").unwrap();
        assert!(s.journal_day("2026-10-09").unwrap().is_empty());
        assert_eq!(s.journal_summary("2026-10-09").unwrap(), None);
        assert_eq!(s.journal_day("2026-10-10").unwrap().len(), 1);
    }

    #[test]
    fn lines_and_summaries_are_cut() {
        let mut s = store("journal-cut");
        s.journal_add(&entry("2026-10-11", "chat", &"ß".repeat(1_000)))
            .unwrap();
        assert_eq!(
            s.journal_day("2026-10-11").unwrap()[0].text.chars().count(),
            MAX_TEXT
        );
        s.journal_summary_set("2026-10-11", &"字".repeat(1_000), 1)
            .unwrap();
        let summary = s.journal_summary("2026-10-11").unwrap().unwrap();
        assert!(summary.len() <= MAX_SUMMARY && summary.len() > MAX_SUMMARY - 3);
    }
}
