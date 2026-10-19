//! Durable request admissions: clearing transcript/history records cannot reset a model quota.
use crate::{Error, Store, TransactionBehavior, traces::Span};
use rusqlite::params;

pub(crate) const TABLE: &str = "CREATE TABLE request_attempts (
    attempt TEXT PRIMARY KEY, feature TEXT NOT NULL, at_ms INTEGER NOT NULL
); CREATE INDEX request_attempts_feature_time ON request_attempts(feature, at_ms);
CREATE TABLE request_budget (singleton INTEGER PRIMARY KEY CHECK(singleton=1), identity TEXT NOT NULL);
INSERT INTO request_budget VALUES(1, lower(hex(randomblob(16))));";
const DAY_MS: i64 = 86_400_000;

impl Store {
    /// Bind explicit consent to this ledger; a recreated database has a different identity.
    pub fn request_budget_id(&self) -> Result<String, Error> {
        Ok(self
            .conn
            .query_row("SELECT identity FROM request_budget WHERE singleton=1", [], |r| {
                r.get(0)
            })?)
    }

    pub fn request_attempt_starts(&self, feature: &str, since_ms: i64) -> Result<Vec<i64>, Error> {
        let mut stmt = self
            .conn
            .prepare("SELECT at_ms FROM request_attempts WHERE feature=?1 AND at_ms>?2 ORDER BY at_ms")?;
        let rows = stmt.query_map(params![feature, since_ms], |r| r.get(0))?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    /// Count, reserve and persist the content-free starting span in one write transaction.
    pub fn reserve_request(
        &mut self,
        expected: &str,
        feature: &str,
        limit: usize,
        span: &Span,
    ) -> Result<bool, Error> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let identity: String =
            tx.query_row("SELECT identity FROM request_budget WHERE singleton=1", [], |r| {
                r.get(0)
            })?;
        if identity != expected {
            return Ok(false);
        }
        let since = span.start_ms.saturating_sub(DAY_MS);
        let count: i64 = tx.query_row(
            "SELECT count(*) FROM request_attempts WHERE feature=?1 AND at_ms>?2",
            params![feature, since],
            |r| r.get(0),
        )?;
        if usize::try_from(count).unwrap_or(usize::MAX) >= limit {
            return Ok(false);
        }
        tx.execute(
            "INSERT INTO request_attempts(attempt,feature,at_ms) VALUES(?1,?2,?3)",
            params![span.span, feature, span.start_ms],
        )?;
        tx.execute("INSERT INTO traces(trace,span,parent,name,kind,start_ms,end_ms,status,attrs) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)",
            params![span.trace,span.span,span.parent,span.name,span.kind,span.start_ms,span.end_ms,span.status,span.attrs.to_string()])?;
        // Future timestamps stay charged after a clock rollback.
        tx.execute(
            "DELETE FROM request_attempts WHERE feature=?1 AND at_ms<=?2",
            params![feature, since],
        )?;
        tx.commit()?;
        Ok(true)
    }

    pub fn finish_request(&mut self, span: &Span) -> Result<(), Error> {
        self.conn.execute(
            "UPDATE traces SET end_ms=?1,status=?2,attrs=?3 WHERE span=?4 AND trace=?5 AND kind='request'",
            params![
                span.end_ms,
                span.status,
                span.attrs.to_string(),
                span.span,
                span.trace
            ],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn span(at: i64, id: &str) -> Span {
        Span {
            trace: id.into(),
            span: id.into(),
            parent: None,
            name: "request dictation-cleanup".into(),
            kind: "request".into(),
            start_ms: at,
            end_ms: at,
            status: "started".into(),
            attrs: serde_json::json!({"vults.feature":"dictation-cleanup"}),
        }
    }
    #[test]
    fn reservations_survive_restart_clear_and_failures_and_expire_at_the_boundary() {
        let dir = crate::temp("request-quota");
        let mut db = Store::open(&dir).unwrap();
        let identity = db.request_budget_id().unwrap();
        assert!(
            db.reserve_request(&identity, "dictation-cleanup", 2, &span(DAY_MS, "one"))
                .unwrap()
        );
        assert!(
            db.reserve_request(&identity, "dictation-cleanup", 2, &span(DAY_MS + 1, "two"))
                .unwrap()
        );
        assert!(
            !db.reserve_request(&identity, "dictation-cleanup", 2, &span(DAY_MS + 2, "three"))
                .unwrap()
        );
        let mut failed = span(DAY_MS, "one");
        failed.status = "error".into();
        db.finish_request(&failed).unwrap();
        db.clear_all(&dir).unwrap();
        db.finish_request(&failed).unwrap();
        assert_eq!(
            db.conn
                .query_row("SELECT count(*) FROM traces", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            0,
            "Late completion must not recreate erased traces"
        );
        drop(db);
        let mut db = Store::open(&dir).unwrap();
        assert_eq!(
            db.request_attempt_starts("dictation-cleanup", 0).unwrap().len(),
            2
        );
        assert!(
            !db.reserve_request(&identity, "dictation-cleanup", 2, &span(1, "rollback"))
                .unwrap()
        );
        assert!(
            db.reserve_request(&identity, "dictation-cleanup", 2, &span(2 * DAY_MS, "boundary"))
                .unwrap()
        );
        assert!(
            !db.reserve_request(&identity, "dictation-cleanup", 2, &span(2 * DAY_MS, "same-time"))
                .unwrap()
        );
        assert!(
            db.reserve_request(&identity, "other", 1, &span(2 * DAY_MS, "independent"))
                .unwrap()
        );
    }
    #[test]
    fn the_exact_quota_and_lost_or_recreated_ledger_never_grant_an_old_allowance() {
        let dir = crate::temp("request-identity");
        let mut db = Store::open(&dir).unwrap();
        let identity = db.request_budget_id().unwrap();
        for n in 0..2000 {
            assert!(
                db.reserve_request(
                    &identity,
                    "dictation-cleanup",
                    2000,
                    &span(DAY_MS, &n.to_string())
                )
                .unwrap()
            );
        }
        assert!(
            !db.reserve_request(&identity, "dictation-cleanup", 2000, &span(DAY_MS, "overflow"))
                .unwrap()
        );
        drop(db);
        std::fs::remove_dir_all(&dir).unwrap();
        assert!(Store::open_current(&dir).is_err());
        let mut db = Store::open(&dir).unwrap();
        assert_ne!(db.request_budget_id().unwrap(), identity);
        assert!(
            !db.reserve_request(&identity, "dictation-cleanup", 2000, &span(DAY_MS, "new-file"))
                .unwrap()
        );
        db.conn.execute("DELETE FROM request_budget", []).unwrap();
        assert!(db.request_budget_id().is_err());
        assert!(
            db.reserve_request(&identity, "dictation-cleanup", 2000, &span(DAY_MS, "missing-id"))
                .is_err()
        );
        assert!(
            db.request_attempt_starts("dictation-cleanup", 0)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn concurrent_connections_share_one_last_slot_and_missing_ledger_grants_nothing() {
        let dir = crate::temp("request-race");
        let identity = Store::open(&dir).unwrap().request_budget_id().unwrap();
        let threads: Vec<_> = (0..8)
            .map(|n| {
                let dir = dir.clone();
                let identity = identity.clone();
                std::thread::spawn(move || {
                    Store::open(&dir)
                        .unwrap()
                        .reserve_request(&identity, "dictation-cleanup", 1, &span(DAY_MS, &n.to_string()))
                        .unwrap()
                })
            })
            .collect();
        assert_eq!(
            threads
                .into_iter()
                .filter_map(|t| t.join().ok())
                .filter(|ok| *ok)
                .count(),
            1
        );
        let mut db = Store::open(&dir).unwrap();
        db.conn.execute("DROP TABLE request_attempts", []).unwrap();
        assert!(
            db.reserve_request(&identity, "dictation-cleanup", 2, &span(DAY_MS, "missing"))
                .is_err()
        );
        assert!(db.request_attempt_starts("dictation-cleanup", 0).is_err());
    }
}
