//! Traces (`docs/dev/plan-zeca.md` S6): one span per model call, tool and action Zeca makes, named
//! after OpenTelemetry's GenAI conventions so they read like any other GenAI trace. Never a prompt
//! or a reply: names, timings, tokens, cost, and a tool's target with secrets redacted. Kept
//! [`KEEP_DAYS`] days.

use rusqlite::params;
use serde_json::Value;

use crate::{Error, Store};

pub const KEEP_DAYS: i64 = 30;

/// Migration 3. Shipped: never edit it.
pub(crate) const TABLE: &str = "
    CREATE TABLE traces (
        id INTEGER PRIMARY KEY,
        trace TEXT NOT NULL,
        span TEXT NOT NULL,
        parent TEXT,
        name TEXT NOT NULL,
        kind TEXT NOT NULL,
        start_ms INTEGER NOT NULL,
        end_ms INTEGER NOT NULL,
        status TEXT NOT NULL,
        attrs TEXT NOT NULL
    );
    CREATE INDEX traces_start ON traces (start_ms);
    CREATE INDEX traces_trace ON traces (trace);";

/// One span. `attrs` is a JSON object of OpenTelemetry attributes (`gen_ai.usage.input_tokens`…).
#[derive(Debug, Clone, PartialEq)]
pub struct Span {
    /// The turn it belongs to.
    pub trace: String,
    pub span: String,
    pub parent: Option<String>,
    /// `chat claude-opus-5-5`, `execute_tool Bash`.
    pub name: String,
    /// `chat`, `execute_tool`, `action`.
    pub kind: String,
    /// Unix milliseconds.
    pub start_ms: i64,
    pub end_ms: i64,
    /// `ok`, `error`, `stopped`, `denied`.
    pub status: String,
    pub attrs: Value,
}

impl Store {
    pub fn append_span(&mut self, s: &Span) -> Result<(), Error> {
        self.conn.execute(
            "INSERT INTO traces (trace, span, parent, name, kind, start_ms, end_ms, status, attrs)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                s.trace,
                s.span,
                s.parent,
                s.name,
                s.kind,
                s.start_ms,
                s.end_ms,
                s.status,
                s.attrs.to_string()
            ],
        )?;
        Ok(())
    }

    /// The newest `limit` spans, newest first.
    pub fn spans(&self, limit: u32) -> Result<Vec<Span>, Error> {
        let mut stmt = self.conn.prepare(
            "SELECT trace, span, parent, name, kind, start_ms, end_ms, status, attrs FROM traces
             ORDER BY start_ms DESC, id DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map([limit], |r| {
            Ok(Span {
                trace: r.get(0)?,
                span: r.get(1)?,
                parent: r.get(2)?,
                name: r.get(3)?,
                kind: r.get(4)?,
                start_ms: r.get(5)?,
                end_ms: r.get(6)?,
                status: r.get(7)?,
                attrs: serde_json::from_str(&r.get::<_, String>(8)?).unwrap_or(Value::Null),
            })
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    /// The start (Unix ms) of every span of `kind` and `name` since `since_ms`, oldest first.
    pub fn span_starts(&self, kind: &str, name: &str, since_ms: i64) -> Result<Vec<i64>, Error> {
        let mut stmt = self.conn.prepare(
            "SELECT start_ms FROM traces WHERE kind = ?1 AND name = ?2 AND start_ms >= ?3 ORDER BY start_ms",
        )?;
        let rows = stmt.query_map(rusqlite::params![kind, name, since_ms], |r| r.get(0))?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    /// Drops the spans older than 30 days, counted back from `now_ms`.
    pub fn prune_traces(&mut self, now_ms: i64) -> Result<(), Error> {
        self.conn.execute(
            "DELETE FROM traces WHERE start_ms < ?1",
            [now_ms - KEEP_DAYS * 86_400_000],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use crate::temp;

    fn span(start_ms: i64) -> Span {
        Span {
            trace: "t1".into(),
            span: "s1".into(),
            parent: None,
            name: "chat claude-opus-5-5".into(),
            kind: "chat".into(),
            start_ms,
            end_ms: start_ms + 1200,
            status: "ok".into(),
            attrs: serde_json::json!({ "gen_ai.usage.input_tokens": 20 }),
        }
    }

    #[test]
    fn a_span_is_kept_and_read_back() {
        let mut s = Store::open(&temp("t-append")).unwrap();
        s.append_span(&span(1000)).unwrap();
        let mut tool = span(1500);
        tool.span = "s2".into();
        tool.parent = Some("s1".into());
        s.append_span(&tool).unwrap();
        let spans = s.spans(10).unwrap();
        assert_eq!(spans, vec![tool, span(1000)]);
    }

    #[test]
    fn prune_keeps_thirty_days() {
        let mut s = Store::open(&temp("t-prune")).unwrap();
        let day = 86_400_000;
        let now = 100 * day;
        for ago in [0, 29, 31] {
            s.append_span(&span(now - ago * day)).unwrap();
        }
        s.prune_traces(now).unwrap();
        assert_eq!(s.spans(10).unwrap().len(), 2);
    }

    #[test]
    fn span_starts_finds_one_kind_and_name_since_a_time() {
        let mut s = Store::open(&temp("t-starts")).unwrap();
        for (start, name) in [
            (10, "request github"),
            (20, "chat x"),
            (30, "request github"),
            (40, "request vercel"),
        ] {
            let mut sp = span(start);
            sp.kind = if name.starts_with("request") {
                "request"
            } else {
                "chat"
            }
            .into();
            sp.name = name.into();
            s.append_span(&sp).unwrap();
        }
        assert_eq!(s.span_starts("request", "request github", 15).unwrap(), vec![30]);
        assert_eq!(
            s.span_starts("request", "request github", 0).unwrap(),
            vec![10, 30]
        );
    }
}
