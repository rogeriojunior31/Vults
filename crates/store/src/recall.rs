//! Recall over Zeca's journal (`docs/dev/plan-zeca.md` M5): full-text search (FTS5, stemmed) over
//! the journal's lines and the days' summaries, and, when a local embedding model gave vectors,
//! the summaries nearest the question; the lists merged by reciprocal rank fusion. Every hit is
//! the journal's words: whoever shows it to a model shows it as untrusted content.

use std::collections::HashMap;

use rusqlite::params;

use crate::{Error, Store};

/// Migration 10: whether a journal line came from a conversation that read outside content; the
/// full-text indexes, kept in step by triggers (a prune or Clear history leaves nothing in them, a
/// changed summary drops its stale vector); the summaries' vectors. The summaries' index holds its
/// own copy keyed by day: `journal_days` has no integer key a VACUUM must keep. Shipped: never
/// edit it.
pub(crate) const TABLE: &str = "
    ALTER TABLE journal ADD COLUMN outside INTEGER NOT NULL DEFAULT 0;
    CREATE TABLE recall_vectors (
        day TEXT PRIMARY KEY,
        model TEXT NOT NULL,
        vector BLOB NOT NULL
    );
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
        day UNINDEXED, summary, tokenize='porter unicode61 remove_diacritics 2'
    );
    CREATE TRIGGER days_fts_add AFTER INSERT ON journal_days BEGIN
        INSERT INTO days_fts (day, summary) VALUES (new.day, new.summary);
    END;
    CREATE TRIGGER days_fts_drop AFTER DELETE ON journal_days BEGIN
        DELETE FROM days_fts WHERE day = old.day;
        DELETE FROM recall_vectors WHERE day = old.day;
    END;
    CREATE TRIGGER days_fts_change AFTER UPDATE ON journal_days BEGIN
        DELETE FROM days_fts WHERE day = old.day;
        INSERT INTO days_fts (day, summary) VALUES (new.day, new.summary);
        DELETE FROM recall_vectors WHERE day = old.day;
    END;
    INSERT INTO days_fts (day, summary) SELECT day, summary FROM journal_days;";

/// How far each list reads before the fusion, and the fusion's constant (the usual 60).
const DEPTH: i64 = 20;
const RRF_K: f64 = 60.0;
/// Words of a question used at most: a pasted page must not become a query of a thousand terms.
const MAX_TERMS: usize = 12;
/// Below this, a summary is not near the question, whatever its rank.
const MIN_COSINE: f32 = 0.5;

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

/// How a recall is asked.
#[derive(Debug, Clone, Copy, Default)]
pub struct Ask<'a> {
    /// The question's embedding by that model, when one is at hand.
    pub vector: Option<(&'a str, &'a [f32])>,
    /// Chat lines from this day on are left out: in front of a turn, the conversation going on is
    /// already in the context, and Zeca must not be fed his own words back as memories.
    pub chat_before: Option<&'a str>,
    /// Only passages holding a third of the question's words (one at least), or near it by
    /// meaning: "ok, thanks" must not bring three passages along.
    pub strict: bool,
}

/// Words a question is made of that no journal line is about, in English, Portuguese and Spanish
/// (compared without accents): kept, they would match everything.
const STOP: &[&str] = &[
    // English
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
    "hi",
    "hello",
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
    "no",
    "of",
    "ok",
    "okay",
    "on",
    "or",
    "our",
    "please",
    "remember",
    "should",
    "so",
    "sure",
    "thanks",
    "thank",
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
    "would",
    "yes",
    "you",
    "your",
    "yesterday",
    "zeca",
    // Portuguese
    "as",
    "ate",
    "com",
    "como",
    "da",
    "das",
    "de",
    "dia",
    "do",
    "dos",
    "e",
    "ela",
    "ele",
    "em",
    "entao",
    "esse",
    "essa",
    "este",
    "esta",
    "estava",
    "eu",
    "fiz",
    "fizemos",
    "foi",
    "hoje",
    "isso",
    "isto",
    "lembra",
    "mais",
    "mas",
    "me",
    "meu",
    "minha",
    "na",
    "nao",
    "nas",
    "no",
    "nos",
    "o",
    "obrigado",
    "onde",
    "ontem",
    "os",
    "ou",
    "para",
    "pela",
    "pelo",
    "por",
    "pra",
    "quando",
    "que",
    "qual",
    "quem",
    "se",
    "sim",
    "sobre",
    "um",
    "uma",
    "voce",
    "vc",
    // Spanish
    "al",
    "ayer",
    "con",
    "cual",
    "cuando",
    "del",
    "donde",
    "el",
    "ella",
    "en",
    "es",
    "eso",
    "esto",
    "gracias",
    "hice",
    "hicimos",
    "hoy",
    "la",
    "las",
    "lo",
    "los",
    "mi",
    "muy",
    "para",
    "pero",
    "porque",
    "que",
    "quien",
    "si",
    "sobre",
    "su",
    "tu",
    "un",
    "una",
    "y",
    "yo",
];

/// Lowercase, with the accents of Latin letters taken off (written as escapes: the code stays in
/// plain English letters).
fn fold(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .map(|c| match c {
            '\u{e0}'..='\u{e5}' => 'a',
            '\u{e7}' => 'c',
            '\u{e8}'..='\u{eb}' => 'e',
            '\u{ec}'..='\u{ef}' => 'i',
            '\u{f1}' => 'n',
            '\u{f2}'..='\u{f6}' => 'o',
            '\u{f9}'..='\u{fc}' => 'u',
            '\u{fd}' | '\u{ff}' => 'y',
            c => c,
        })
        .collect()
}

/// Chinese, Japanese and Korean script: no spaces between its words, so it is searched by pairs
/// of characters, not by words.
fn cjk(c: char) -> bool {
    matches!(c, '\u{3040}'..='\u{30ff}' | '\u{3400}'..='\u{4dbf}' | '\u{4e00}'..='\u{9fff}' | '\u{ac00}'..='\u{d7af}')
}

/// A question's terms: its words (stop words out) and the character pairs of its CJK runs.
#[derive(Debug, Default, PartialEq)]
struct Terms {
    words: Vec<String>,
    pairs: Vec<String>,
}

impl Terms {
    fn of(question: &str) -> Self {
        let folded = fold(question);
        let mut runs: Vec<(bool, String)> = Vec::new();
        for c in folded.chars() {
            if !c.is_alphanumeric() {
                runs.push((false, String::new()));
                continue;
            }
            let is_cjk = cjk(c);
            match runs.last_mut() {
                Some((k, run)) if *k == is_cjk && !run.is_empty() => run.push(c),
                _ => runs.push((is_cjk, c.to_string())),
            }
        }
        let mut t = Terms::default();
        for (is_cjk, run) in runs.into_iter().filter(|(_, r)| !r.is_empty()) {
            if is_cjk {
                let chars: Vec<char> = run.chars().collect();
                t.pairs
                    .extend(chars.windows(2).map(|w| w.iter().collect::<String>()));
            } else if run.chars().count() >= 2 && !STOP.contains(&run.as_str()) {
                t.words.push(run);
            }
        }
        t.words.sort();
        t.words.dedup();
        t.words.truncate(MAX_TERMS);
        t.pairs.sort();
        t.pairs.dedup();
        t.pairs.truncate(MAX_TERMS);
        t
    }

    fn count(&self) -> usize {
        self.words.len() + self.pairs.len()
    }

    /// The words as an FTS5 query: each quoted (no operator of FTS5's can slip in), any matching.
    fn fts(&self) -> Option<String> {
        (!self.words.is_empty()).then(|| {
            self.words
                .iter()
                .map(|w| format!("\"{w}\""))
                .collect::<Vec<_>>()
                .join(" OR ")
        })
    }

    /// How many of the terms `text` holds; a word by its first two thirds (four letters at
    /// least), close enough to the stemmer that matched it.
    fn matched(&self, text: &str) -> usize {
        let text = fold(text);
        let words = self.words.iter().filter(|w| {
            let n = w.chars().count();
            let stem: String = w.chars().take((n * 2 / 3).max(4).min(n)).collect();
            text.contains(&stem)
        });
        words.count() + self.pairs.iter().filter(|p| text.contains(p.as_str())).count()
    }
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

/// The lines recall may give: none from a conversation that read outside content, and no chat
/// line from `chat_before` on.
const LINES_KEPT: &str = "j.outside = 0 AND NOT (j.kind = 'chat' AND j.day >= ?2)";

fn line(r: &rusqlite::Row<'_>) -> rusqlite::Result<Hit> {
    Ok(Hit {
        source: Source::Line(r.get(0)?),
        day: r.get(1)?,
        ms: Some(r.get(2)?),
        kind: r.get(3)?,
        project: r.get(4)?,
        text: r.get(5)?,
    })
}

fn summary(day: String, text: String) -> Hit {
    Hit {
        source: Source::Summary,
        day,
        ms: None,
        kind: "summary".into(),
        project: String::new(),
        text,
    }
}

impl Store {
    /// A day's summary's vector, from `model`; kept only while that very summary is there (a
    /// Clear history or a new summary during the embedding wins).
    pub fn recall_vector_set(
        &mut self,
        day: &str,
        summary: &str,
        model: &str,
        vector: &[f32],
    ) -> Result<(), Error> {
        self.conn.execute(
            "INSERT INTO recall_vectors (day, model, vector)
             SELECT ?1, ?3, ?4 WHERE EXISTS
                 (SELECT 1 FROM journal_days WHERE day = ?1 AND summary = ?2)
             ON CONFLICT (day) DO UPDATE SET model = excluded.model, vector = excluded.vector",
            params![day, summary, model, to_blob(vector)],
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

    /// The `limit` passages that best answer `question`.
    pub fn recall(&self, question: &str, ask: Ask<'_>, limit: usize) -> Result<Vec<Hit>, Error> {
        let terms = Terms::of(question);
        // Past every day: no chat line is left out.
        let chat_before = ask.chat_before.unwrap_or("9999-12-31");
        let mut lists: Vec<Vec<Hit>> = Vec::new();
        if let Some(q) = terms.fts() {
            let mut stmt = self.conn.prepare(
                "SELECT day, summary FROM days_fts WHERE days_fts MATCH ?1
                 ORDER BY bm25(days_fts) LIMIT ?2",
            )?;
            lists.push(
                stmt.query_map(params![q, DEPTH], |r| Ok(summary(r.get(0)?, r.get(1)?)))?
                    .collect::<Result<_, _>>()?,
            );
            let mut stmt = self.conn.prepare(&format!(
                "SELECT j.id, j.day, j.ms, j.kind, j.project, j.text FROM journal_fts
                 JOIN journal j ON j.id = journal_fts.rowid
                 WHERE journal_fts MATCH ?1 AND {LINES_KEPT}
                 ORDER BY bm25(journal_fts) LIMIT ?3"
            ))?;
            lists.push(
                stmt.query_map(params![q, chat_before, DEPTH], line)?
                    .collect::<Result<_, _>>()?,
            );
        }
        if !terms.pairs.is_empty() {
            lists.extend(self.by_pairs(&terms, chat_before)?);
        }
        let mut by_meaning: Vec<String> = Vec::new();
        if let Some((model, q)) = ask.vector {
            let mut stmt = self.conn.prepare(
                "SELECT d.day, d.summary, v.vector FROM recall_vectors v
                 JOIN journal_days d ON d.day = v.day WHERE v.model = ?1",
            )?;
            let mut near: Vec<(f32, Hit)> = stmt
                .query_map([model], |r| {
                    let blob: Vec<u8> = r.get(2)?;
                    Ok((cosine(q, &from_blob(&blob)), summary(r.get(0)?, r.get(1)?)))
                })?
                .collect::<Result<Vec<_>, _>>()?;
            near.retain(|(c, _)| *c >= MIN_COSINE);
            near.sort_by(|a, b| b.0.total_cmp(&a.0));
            let near: Vec<Hit> = near.into_iter().take(DEPTH as usize).map(|(_, h)| h).collect();
            by_meaning = near.iter().map(|h| h.day.clone()).collect();
            lists.push(near);
        }
        // A third of the question's terms, one at least: a long question needs more than one of
        // its words in a passage, a short one only one.
        let need = (terms.count() + 1).div_ceil(3).max(1);
        let mut hits = fuse(lists, usize::MAX);
        if ask.strict {
            // A summary near the question by meaning stands on its own; a word match needs words.
            hits.retain(|h| {
                (h.source == Source::Summary && by_meaning.contains(&h.day))
                    || (terms.count() > 0 && terms.matched(&h.text) >= need)
            });
        }
        hits.truncate(limit);
        Ok(hits)
    }

    /// Lines and summaries holding the question's CJK character pairs, most pairs first.
    fn by_pairs(&self, terms: &Terms, chat_before: &str) -> Result<Vec<Vec<Hit>>, Error> {
        let like = terms.pairs.iter().map(|p| format!("%{p}%")).collect::<Vec<_>>();
        let any = |column: &str, from: usize| {
            (0..like.len())
                .map(|i| format!("{column} LIKE ?{}", i + from))
                .collect::<Vec<_>>()
                .join(" OR ")
        };
        let rank = |mut hits: Vec<Hit>| {
            hits.sort_by_key(|h| std::cmp::Reverse(terms.matched(&h.text)));
            hits.truncate(DEPTH as usize);
            hits
        };
        let mut stmt = self.conn.prepare(&format!(
            "SELECT day, summary FROM journal_days WHERE {}",
            any("summary", 1)
        ))?;
        let summaries: Vec<Hit> = stmt
            .query_map(rusqlite::params_from_iter(like.iter()), |r| {
                Ok(summary(r.get(0)?, r.get(1)?))
            })?
            .collect::<Result<_, _>>()?;
        let mut stmt = self.conn.prepare(&format!(
            "SELECT j.id, j.day, j.ms, j.kind, j.project, j.text FROM journal j
             WHERE j.outside = 0 AND NOT (j.kind = 'chat' AND j.day >= ?1) AND ({})",
            any("j.text", 2)
        ))?;
        let params = std::iter::once(chat_before.to_string()).chain(like.iter().cloned());
        let lines: Vec<Hit> = stmt
            .query_map(rusqlite::params_from_iter(params), line)?
            .collect::<Result<_, _>>()?;
        Ok(vec![rank(summaries), rank(lines)])
    }
}

/// Reciprocal rank fusion: each hit scores 1 / (k + rank) in each list it is in.
fn fuse(lists: Vec<Vec<Hit>>, limit: usize) -> Vec<Hit> {
    let mut scores: HashMap<String, (f64, Hit)> = HashMap::new();
    for list in lists {
        for (rank, hit) in list.into_iter().enumerate() {
            let score = 1.0 / (RRF_K + rank as f64 + 1.0);
            // One summary a day: the day is its identity; a line's is its id.
            let key = match &hit.source {
                Source::Summary => format!("summary {}", hit.day),
                Source::Line(id) => format!("line {id}"),
            };
            scores
                .entry(key)
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

    fn add(s: &mut Store, day: &str, kind: &str, text: &str, outside: bool) {
        s.journal_add(&Entry {
            ms: 1,
            day: day.into(),
            kind: kind.into(),
            agent: "zeca".into(),
            project: "site".into(),
            text: text.into(),
            outside,
        })
        .unwrap();
    }

    fn days(hits: &[Hit]) -> Vec<&str> {
        hits.iter().map(|h| h.day.as_str()).collect()
    }

    #[test]
    fn terms_are_quoted_folded_and_stop_words_dropped() {
        let t = Terms::of("What did we decide about the Vitest migration?");
        assert_eq!(
            t.fts().as_deref(),
            Some("\"decide\" OR \"migration\" OR \"vitest\"")
        );
        // FTS5's own syntax never gets through.
        assert_eq!(
            Terms::of("x\" OR NEAR(a b) NOT *").fts().as_deref(),
            Some("\"near\" OR \"not\"")
        );
        assert_eq!(Terms::of("what did I do?").fts(), None);
        assert_eq!(Terms::of("ok, thanks!").count(), 0);
        // Portuguese and Spanish small words go too, accents or not.
        let pt = Terms::of("O que fizemos ontem na migra\u{e7}\u{e3}o do Vitest?");
        assert_eq!(pt.words, ["migracao", "vitest"]);
        // Chinese is searched by character pairs.
        let zh = Terms::of("\u{6211}\u{4eec}\u{8fc1}\u{79fb}\u{4e86}\u{6d4b}\u{8bd5}");
        assert!(zh.pairs.contains(&"\u{8fc1}\u{79fb}".to_string()), "{zh:?}");
        let long = "word ".repeat(10) + &(0..40).map(|i| format!("w{i}x ")).collect::<String>();
        assert!(Terms::of(&long).count() <= MAX_TERMS);
    }

    #[test]
    fn lines_and_summaries_are_found_by_stem_and_fused() {
        let mut s = store("recall-find");
        add(
            &mut s,
            "2026-10-08",
            "chat",
            "you: migrate the tests from Jest to Vitest",
            false,
        );
        add(
            &mut s,
            "2026-10-09",
            "chat",
            "you: fix the island's fold timer",
            false,
        );
        s.journal_summary_set(
            "2026-10-08",
            "Moved the site's test suite to Vitest; two snapshot tests still fail.",
            1,
        )
        .unwrap();
        s.journal_summary_set("2026-10-09", "Fixed the island folding too early.", 1)
            .unwrap();
        let hits = s
            .recall("when was I migrating tests to vitest?", Ask::default(), 3)
            .unwrap();
        assert!(hits.len() >= 2, "{hits:?}");
        assert!(hits.iter().take(2).all(|h| h.day == "2026-10-08"), "{hits:?}");
        assert_eq!(
            s.recall("the fold bug", Ask::default(), 3).unwrap()[0].day,
            "2026-10-09"
        );
        assert!(s.recall("kubernetes", Ask::default(), 3).unwrap().is_empty());
    }

    #[test]
    fn today_s_chat_and_outside_words_never_come_back() {
        let mut s = store("recall-skip");
        add(
            &mut s,
            "2026-10-05",
            "chat",
            "you: plan the vitest snapshot fix",
            false,
        );
        add(
            &mut s,
            "2026-10-11",
            "chat",
            "you: let's plan the vitest snapshot tests",
            false,
        );
        add(&mut s, "2026-10-11", "turn", "vitest snapshot run, 3 min", false);
        add(
            &mut s,
            "2026-10-06",
            "chat",
            "zeca: the page says vitest snapshot is broken",
            true,
        );
        let ask = Ask {
            chat_before: Some("2026-10-11"),
            ..Ask::default()
        };
        let hits = s.recall("vitest snapshot tests", ask, 5).unwrap();
        // Today's chat is the conversation going on; today's turn is fair; outside words never.
        assert_eq!(days(&hits), ["2026-10-11", "2026-10-05"], "{hits:?}");
        assert_eq!(hits[0].kind, "turn");
    }

    #[test]
    fn a_strict_recall_needs_the_question_s_words() {
        let mut s = store("recall-strict");
        add(
            &mut s,
            "2026-10-05",
            "chat",
            "you: ok, deploy the site to netlify",
            false,
        );
        add(
            &mut s,
            "2026-10-06",
            "chat",
            "you: rename the site's netlify project",
            false,
        );
        let strict = Ask {
            strict: true,
            ..Ask::default()
        };
        assert!(s.recall("ok thanks, do it", strict, 3).unwrap().is_empty());
        // Five words asked: a passage needs two of them, one is not enough.
        let hits = s
            .recall("deploy pipeline staging netlify release", strict, 3)
            .unwrap();
        assert_eq!(days(&hits), ["2026-10-05"]);
        assert_eq!(
            s.recall("deploy pipeline staging netlify release", Ask::default(), 3)
                .unwrap()
                .len(),
            2
        );
    }

    #[test]
    fn chinese_is_found_by_its_character_pairs() {
        let mut s = store("recall-zh");
        // "We migrated the tests to Vitest", and an unrelated line.
        add(
            &mut s,
            "2026-10-05",
            "chat",
            "you: \u{6211}\u{4eec}\u{628a}\u{6d4b}\u{8bd5}\u{8fc1}\u{79fb}\u{5230} Vitest \u{4e86}",
            false,
        );
        add(
            &mut s,
            "2026-10-06",
            "chat",
            "you: \u{4fee}\u{597d}\u{4e86}\u{6258}\u{76d8}\u{56fe}\u{6807}",
            false,
        );
        // "when did we migrate the tests"
        let hits = s
            .recall(
                "\u{6211}\u{4eec}\u{4ec0}\u{4e48}\u{65f6}\u{5019}\u{8fc1}\u{79fb}\u{6d4b}\u{8bd5}",
                Ask::default(),
                3,
            )
            .unwrap();
        assert_eq!(days(&hits).first(), Some(&"2026-10-05"), "{hits:?}");
    }

    #[test]
    fn vectors_find_what_words_miss_above_a_floor_and_only_from_their_model() {
        let mut s = store("recall-vectors");
        s.journal_summary_set("2026-10-01", "Planned the trip budget.", 1)
            .unwrap();
        s.journal_summary_set("2026-10-02", "Wrote the release notes.", 1)
            .unwrap();
        assert_eq!(s.recall_unembedded("m").unwrap().len(), 2);
        s.recall_vector_set("2026-10-01", "Planned the trip budget.", "m", &[1.0, 0.0])
            .unwrap();
        s.recall_vector_set("2026-10-02", "Wrote the release notes.", "m", &[0.0, 1.0])
            .unwrap();
        assert!(s.recall_unembedded("m").unwrap().is_empty());
        let near = |v: &'static [f32], model: &'static str| Ask {
            vector: Some((model, v)),
            strict: true,
            ..Ask::default()
        };
        // No shared word with either summary: only the vector answers, and only the near one.
        let hits = s
            .recall("money for travelling", near(&[0.9, 0.1], "m"), 3)
            .unwrap();
        assert_eq!(days(&hits), ["2026-10-01"]);
        assert!(
            s.recall("money for travelling", near(&[0.9, 0.1], "other"), 3)
                .unwrap()
                .is_empty()
        );
        // A new summary drops its stale vector; a vector for a summary no longer there is not kept.
        s.journal_summary_set("2026-10-01", "Booked the flights.", 2)
            .unwrap();
        assert_eq!(s.recall_unembedded("m").unwrap().len(), 1);
        s.recall_vector_set("2026-10-01", "Planned the trip budget.", "m", &[1.0, 0.0])
            .unwrap();
        assert_eq!(
            s.recall_unembedded("m").unwrap().len(),
            1,
            "stale text, no vector"
        );
    }

    #[test]
    fn a_prune_or_a_clear_leaves_nothing_to_recall() {
        let dir = crate::temp("recall-clear");
        let mut s = Store::open(&dir).unwrap();
        add(&mut s, "2026-01-01", "chat", "you: the ancient zebra plan", false);
        add(&mut s, "2026-10-10", "chat", "you: the zebra crossing", false);
        s.journal_summary_set("2026-01-01", "The zebra plan began.", 1)
            .unwrap();
        s.recall_vector_set("2026-01-01", "The zebra plan began.", "m", &[1.0])
            .unwrap();
        s.journal_prune("2026-06-01").unwrap();
        let hits = s.recall("zebra", Ask::default(), 5).unwrap();
        assert_eq!(days(&hits), ["2026-10-10"]);
        s.clear_all(&dir).unwrap();
        assert!(s.recall("zebra", Ask::default(), 5).unwrap().is_empty());
        assert!(s.recall_unembedded("m").unwrap().is_empty());
    }
}
