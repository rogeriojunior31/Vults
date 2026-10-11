//! Recall over Zeca's journal (`docs/dev/plan-zeca.md` M5): full-text search (FTS5, stemmed) over
//! the journal's lines and the days' summaries, and, when a local embedding model gave vectors,
//! the summaries nearest the question; the lists merged by reciprocal rank fusion. Every hit is
//! the journal's words: whoever shows it to a model shows it as untrusted content.

use std::collections::HashMap;

use rusqlite::params;

use crate::{Error, Store};

/// Migration 10: indexes kept in step with `journal` and `journal_days` by triggers, so a prune or
/// Clear history leaves nothing behind in them; and the summaries' vectors. Shipped: never edit it.
pub(crate) const TABLE: &str = "
    CREATE VIRTUAL TABLE journal_fts USING fts5(
        text, project, content='journal', content_rowid='id',
        tokenize='porter unicode61 remove_diacritics 2'
    );
    CREATE TRIGGER journal_fts_add AFTER INSERT ON journal BEGIN
        INSERT INTO journal_fts (rowid, text, project) VALUES (new.id, new.text, new.project);
    END;
    CREATE TRIGGER journal_fts_drop AFTER DELETE ON journal BEGIN
        INSERT INTO journal_fts (journal_fts, rowid, text, project)
            VALUES ('delete', old.id, old.text, old.project);
    END;
    INSERT INTO journal_fts (journal_fts) VALUES ('rebuild');
    CREATE VIRTUAL TABLE days_fts USING fts5(
        summary, content='journal_days', content_rowid='rowid',
        tokenize='porter unicode61 remove_diacritics 2'
    );
    CREATE TRIGGER days_fts_add AFTER INSERT ON journal_days BEGIN
        INSERT INTO days_fts (rowid, summary) VALUES (new.rowid, new.summary);
    END;
    CREATE TRIGGER days_fts_drop AFTER DELETE ON journal_days BEGIN
        INSERT INTO days_fts (days_fts, rowid, summary) VALUES ('delete', old.rowid, old.summary);
    END;
    CREATE TRIGGER days_fts_change AFTER UPDATE ON journal_days BEGIN
        INSERT INTO days_fts (days_fts, rowid, summary) VALUES ('delete', old.rowid, old.summary);
        INSERT INTO days_fts (rowid, summary) VALUES (new.rowid, new.summary);
    END;
    INSERT INTO days_fts (days_fts) VALUES ('rebuild');
    CREATE TABLE recall_vectors (
        day TEXT PRIMARY KEY,
        model TEXT NOT NULL,
        vector BLOB NOT NULL
    );";

/// How far each list reads before the fusion, and the fusion's constant (the usual 60).
const DEPTH: i64 = 20;
const RRF_K: f64 = 60.0;

/// What a hit is.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Source {
    /// A day's summary.
    Summary,
    /// One line of the journal, by its id.
    Line(i64),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Hit {
    pub source: Source,
    pub day: String,
    /// Unix milliseconds of the line; none for a summary.
    pub ms: Option<i64>,
    pub kind: String,
    pub project: String,
    pub text: String,
}

/// Words a question is made of that no journal line is about: kept, they would match everything.
const STOP: &[&str] = &[
    "a",
    "about",
    "after",
    "again",
    "all",
    "an",
    "and",
    "any",
    "are",
    "as",
    "at",
    "be",
    "been",
    "before",
    "but",
    "by",
    "can",
    "could",
    "day",
    "did",
    "do",
    "does",
    "doing",
    "done",
    "for",
    "from",
    "get",
    "got",
    "had",
    "has",
    "have",
    "how",
    "i",
    "if",
    "in",
    "into",
    "is",
    "it",
    "its",
    "last",
    "me",
    "my",
    "of",
    "on",
    "or",
    "our",
    "remember",
    "should",
    "so",
    "that",
    "the",
    "their",
    "them",
    "then",
    "there",
    "they",
    "this",
    "time",
    "to",
    "was",
    "we",
    "week",
    "were",
    "what",
    "when",
    "where",
    "which",
    "who",
    "why",
    "will",
    "with",
    "work",
    "worked",
    "working",
    "would",
    "you",
    "your",
    "yesterday",
    "zeca",
];

/// The question as an FTS5 query: its words, each quoted (no operator of FTS5's can slip in),
/// any of them matching. None when no word is left.
fn fts_query(question: &str) -> Option<String> {
    let mut words: Vec<String> = question
        .split(|c: char| !c.is_alphanumeric())
        .map(str::to_lowercase)
        .filter(|w| w.chars().count() >= 2 && !STOP.contains(&w.as_str()))
        .collect();
    words.sort();
    words.dedup();
    (!words.is_empty()).then(|| {
        words
            .iter()
            .map(|w| format!("\"{w}\""))
            .collect::<Vec<_>>()
            .join(" OR ")
    })
}

/// A vector as bytes (little-endian `f32`s), and back.
pub fn to_blob(v: &[f32]) -> Vec<u8> {
    v.iter().flat_map(|x| x.to_le_bytes()).collect()
}

fn from_blob(b: &[u8]) -> Vec<f32> {
    b.as_chunks::<4>()
        .0
        .iter()
        .map(|c| f32::from_le_bytes(*c))
        .collect()
}

fn cosine(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let (mut dot, mut na, mut nb) = (0.0_f32, 0.0_f32, 0.0_f32);
    for (x, y) in a.iter().zip(b) {
        dot += x * y;
        na += x * x;
        nb += y * y;
    }
    if na == 0.0 || nb == 0.0 {
        0.0
    } else {
        dot / (na.sqrt() * nb.sqrt())
    }
}

impl Store {
    /// A day's summary's vector, from `model` (a vector from another model is never compared).
    pub fn recall_vector_set(&mut self, day: &str, model: &str, vector: &[f32]) -> Result<(), Error> {
        self.conn.execute(
            "INSERT INTO recall_vectors (day, model, vector) VALUES (?1, ?2, ?3)
             ON CONFLICT (day) DO UPDATE SET model = excluded.model, vector = excluded.vector",
            params![day, model, to_blob(vector)],
        )?;
        Ok(())
    }

    /// Days with a summary and no vector from `model` yet.
    pub fn recall_unembedded(&self, model: &str) -> Result<Vec<(String, String)>, Error> {
        let mut stmt = self.conn.prepare(
            "SELECT d.day, d.summary FROM journal_days d LEFT JOIN recall_vectors v
             ON v.day = d.day AND v.model = ?1 WHERE v.day IS NULL ORDER BY d.day",
        )?;
        let rows = stmt.query_map([model], |r| Ok((r.get(0)?, r.get(1)?)))?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    /// The `limit` passages that best answer `question`; `vector` is the question's embedding by
    /// that model, when one is at hand.
    pub fn recall(
        &self,
        question: &str,
        vector: Option<(&str, &[f32])>,
        limit: usize,
    ) -> Result<Vec<Hit>, Error> {
        let mut lists: Vec<Vec<Hit>> = Vec::new();
        if let Some(q) = fts_query(question) {
            let mut stmt = self.conn.prepare(
                "SELECT d.day, d.summary FROM days_fts JOIN journal_days d ON d.rowid = days_fts.rowid
                 WHERE days_fts MATCH ?1 ORDER BY bm25(days_fts) LIMIT ?2",
            )?;
            let summaries = stmt
                .query_map(params![q, DEPTH], |r| {
                    Ok(Hit {
                        source: Source::Summary,
                        day: r.get(0)?,
                        ms: None,
                        kind: "summary".into(),
                        project: String::new(),
                        text: r.get(1)?,
                    })
                })?
                .collect::<Result<Vec<_>, _>>()?;
            lists.push(summaries);
            let mut stmt = self.conn.prepare(
                "SELECT j.id, j.day, j.ms, j.kind, j.project, j.text FROM journal_fts
                 JOIN journal j ON j.id = journal_fts.rowid
                 WHERE journal_fts MATCH ?1 ORDER BY bm25(journal_fts) LIMIT ?2",
            )?;
            let lines = stmt
                .query_map(params![q, DEPTH], |r| {
                    Ok(Hit {
                        source: Source::Line(r.get(0)?),
                        day: r.get(1)?,
                        ms: Some(r.get(2)?),
                        kind: r.get(3)?,
                        project: r.get(4)?,
                        text: r.get(5)?,
                    })
                })?
                .collect::<Result<Vec<_>, _>>()?;
            lists.push(lines);
        }
        if let Some((model, q)) = vector {
            let mut stmt = self.conn.prepare(
                "SELECT d.day, d.summary, v.vector FROM recall_vectors v
                 JOIN journal_days d ON d.day = v.day WHERE v.model = ?1",
            )?;
            let mut near: Vec<(f32, Hit)> = stmt
                .query_map([model], |r| {
                    let blob: Vec<u8> = r.get(2)?;
                    Ok((
                        cosine(q, &from_blob(&blob)),
                        Hit {
                            source: Source::Summary,
                            day: r.get(0)?,
                            ms: None,
                            kind: "summary".into(),
                            project: String::new(),
                            text: r.get(1)?,
                        },
                    ))
                })?
                .collect::<Result<Vec<_>, _>>()?;
            near.sort_by(|a, b| b.0.total_cmp(&a.0));
            lists.push(near.into_iter().take(DEPTH as usize).map(|(_, h)| h).collect());
        }
        Ok(fuse(lists, limit))
    }
}

/// Reciprocal rank fusion: each hit scores 1 / (k + rank) in each list it is in.
fn fuse(lists: Vec<Vec<Hit>>, limit: usize) -> Vec<Hit> {
    let mut scores: HashMap<(Source, String), (f64, Hit)> = HashMap::new();
    for list in lists {
        for (rank, hit) in list.into_iter().enumerate() {
            let score = 1.0 / (RRF_K + rank as f64 + 1.0);
            scores
                .entry((hit.source.clone(), hit.day.clone()))
                .and_modify(|(s, _)| *s += score)
                .or_insert((score, hit));
        }
    }
    let mut all: Vec<(f64, Hit)> = scores.into_values().collect();
    // Ties go to the newer day, then to the summary: the steadier answer.
    all.sort_by(|a, b| {
        b.0.total_cmp(&a.0)
            .then_with(|| b.1.day.cmp(&a.1.day))
            .then_with(|| (a.1.source != Source::Summary).cmp(&(b.1.source != Source::Summary)))
    });
    all.into_iter().take(limit).map(|(_, h)| h).collect()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use crate::journal::Entry;

    fn store(name: &str) -> Store {
        Store::open(&crate::temp(name)).unwrap()
    }

    fn line(s: &mut Store, day: &str, project: &str, text: &str) {
        s.journal_add(&Entry {
            ms: 1,
            day: day.into(),
            kind: "chat".into(),
            agent: "zeca".into(),
            project: project.into(),
            text: text.into(),
        })
        .unwrap();
    }

    #[test]
    fn words_are_quoted_and_stop_words_dropped() {
        assert_eq!(
            fts_query("What did we decide about the Vitest migration?").as_deref(),
            Some("\"decide\" OR \"migration\" OR \"vitest\"")
        );
        // FTS5's own syntax never gets through.
        assert_eq!(
            fts_query("x\" OR NEAR(a b) NOT *").as_deref(),
            Some("\"near\" OR \"not\"")
        );
        assert_eq!(fts_query("what did I do?"), None);
    }

    #[test]
    fn lines_and_summaries_are_found_by_stem_and_fused() {
        let mut s = store("recall-find");
        line(
            &mut s,
            "2026-10-08",
            "site",
            "you: migrate the tests from Jest to Vitest",
        );
        line(&mut s, "2026-10-09", "vults", "you: fix the island's fold timer");
        s.journal_summary_set(
            "2026-10-08",
            "Moved the site's test suite to Vitest; two snapshot tests still fail.",
            1,
        )
        .unwrap();
        s.journal_summary_set("2026-10-09", "Fixed the island folding too early.", 1)
            .unwrap();
        // "migrating" finds "migrate"; the summary and the line of that day both come.
        let hits = s
            .recall("when was I migrating tests to vitest?", None, 3)
            .unwrap();
        assert!(hits.len() >= 2, "{hits:?}");
        assert!(hits.iter().take(2).all(|h| h.day == "2026-10-08"), "{hits:?}");
        let folded = s.recall("the fold bug", None, 3).unwrap();
        assert_eq!(folded[0].day, "2026-10-09");
        assert!(s.recall("kubernetes", None, 3).unwrap().is_empty());
    }

    #[test]
    fn vectors_find_what_words_miss_and_only_from_their_model() {
        let mut s = store("recall-vectors");
        s.journal_summary_set("2026-10-01", "Planned the trip budget.", 1)
            .unwrap();
        s.journal_summary_set("2026-10-02", "Wrote the release notes.", 1)
            .unwrap();
        assert_eq!(s.recall_unembedded("m").unwrap().len(), 2);
        s.recall_vector_set("2026-10-01", "m", &[1.0, 0.0]).unwrap();
        s.recall_vector_set("2026-10-02", "m", &[0.0, 1.0]).unwrap();
        assert!(s.recall_unembedded("m").unwrap().is_empty());
        assert_eq!(s.recall_unembedded("other").unwrap().len(), 2);
        // No shared word with either summary: only the vector can answer.
        let hits = s
            .recall("money for travelling", Some(("m", &[0.9, 0.1])), 1)
            .unwrap();
        assert_eq!(hits[0].day, "2026-10-01");
        assert!(
            s.recall("money for travelling", Some(("other", &[0.9, 0.1])), 1)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn a_prune_or_a_clear_leaves_nothing_to_recall() {
        let dir = crate::temp("recall-clear");
        let mut s = Store::open(&dir).unwrap();
        line(&mut s, "2026-01-01", "old", "you: the ancient zebra plan");
        line(&mut s, "2026-10-10", "new", "you: the zebra crossing");
        s.journal_summary_set("2026-01-01", "The zebra plan began.", 1)
            .unwrap();
        s.journal_prune("2026-06-01").unwrap();
        let hits = s.recall("zebra", None, 5).unwrap();
        assert_eq!(hits.len(), 1, "{hits:?}");
        assert_eq!(hits[0].day, "2026-10-10");
        s.clear_all(&dir).unwrap();
        assert!(s.recall("zebra", None, 5).unwrap().is_empty());
    }
}
