//! Integer micro-USD admissions. An unfinished operation keeps its entire hold, even after a day.
use crate::{Error, Store, TransactionBehavior, traces::Span};
use rusqlite::{OptionalExtension, params};
use serde_json::json;

pub(crate) const TABLE: &str = "CREATE TABLE request_costs (
    attempt TEXT PRIMARY KEY, trace TEXT NOT NULL, feature TEXT NOT NULL,
    start_ms INTEGER NOT NULL CHECK(start_ms>=0),
    reserved_microusd INTEGER NOT NULL CHECK(reserved_microusd>0),
    actual_microusd INTEGER CHECK(actual_microusd>=0),
    end_ms INTEGER CHECK(end_ms>=start_ms)
); CREATE INDEX request_costs_feature_end ON request_costs(feature,end_ms);";
const DAY_MS: i64 = 86_400_000;

fn amount(value: u64) -> Result<i64, Error> {
    i64::try_from(value).map_err(|_| Error::Invalid("Monetary amount exceeds the ledger range"))
}

fn charged(conn: &rusqlite::Connection, feature: &str, now_ms: i64) -> Result<i64, Error> {
    // Unknown usage is not a zero-cost result. SUM overflow also refuses admission.
    Ok(conn.query_row(
        "SELECT coalesce(sum(coalesce(actual_microusd,reserved_microusd)),0)
         FROM request_costs WHERE feature=?1 AND (end_ms IS NULL OR end_ms>?2)",
        params![feature, now_ms.saturating_sub(DAY_MS)],
        |r| r.get(0),
    )?)
}

impl Store {
    /// A new voice owner cannot establish remaining spend while a previous owner is unresolved.
    pub fn has_open_cost(&self, expected: &str, feature: &str) -> Result<bool, Error> {
        if self.request_budget_id()? != expected {
            return Err(Error::Invalid("The monetary ledger identity changed"));
        }
        Ok(self.conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM request_costs WHERE feature=?1 AND end_ms IS NULL)",
            [feature],
            |r| r.get(0),
        )?)
    }

    /// Atomically hold a known upper cost and its starting trace against a shared rolling limit.
    /// One USD is 1,000,000 micro-USD. None means no safe quote, never unlimited admission.
    /// The caller must bound all billable work before sending; this ledger does not quote prices.
    pub fn reserve_cost(
        &mut self,
        expected: &str,
        feature: &str,
        limit_microusd: u64,
        ceiling_microusd: Option<u64>,
        span: &Span,
    ) -> Result<bool, Error> {
        let Some(ceiling) = ceiling_microusd.filter(|v| *v > 0) else {
            return Ok(false);
        };
        if ceiling > limit_microusd {
            return Ok(false);
        }
        let (limit, ceiling) = (amount(limit_microusd)?, amount(ceiling)?);
        if feature.is_empty()
            || span.span.is_empty()
            || span.start_ms < 0
            || span.end_ms != span.start_ms
            || span.kind != "request"
            || span.status != "started"
            || !span.attrs.is_object()
        {
            return Err(Error::Invalid("Invalid monetary request span"));
        }
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
        let used = charged(&tx, feature, span.start_ms)?;
        if used.checked_add(ceiling).is_none_or(|total| total > limit) {
            return Ok(false);
        }
        tx.execute(
            "INSERT INTO request_costs(attempt,trace,feature,start_ms,reserved_microusd) VALUES(?1,?2,?3,?4,?5)",
            params![span.span, span.trace, feature, span.start_ms, ceiling],
        )?;
        let mut span = span.clone();
        span.attrs["vults.cost.reserved_microusd"] = json!(ceiling);
        span.attrs["vults.cost.actual_microusd"] = json!(null);
        tx.execute(
            "INSERT INTO traces(trace,span,parent,name,kind,start_ms,end_ms,status,attrs)
             VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)",
            params![
                span.trace,
                span.span,
                span.parent,
                span.name,
                span.kind,
                span.start_ms,
                span.end_ms,
                span.status,
                span.attrs.to_string()
            ],
        )?;
        tx.commit()?;
        Ok(true)
    }

    /// Settle once, only after the transport has stopped and final usage is known or unavailable.
    /// None keeps the full hold as a conservative debit, without inventing an actual cost.
    /// A provider overrun is charged in full; subsequent admissions see the overrun too.
    /// Request outcome is independent of whether the provider reported usable billing data.
    pub fn settle_cost(
        &mut self,
        expected: &str,
        attempt: &str,
        actual_microusd: Option<u64>,
        finished_ms: i64,
        ok: bool,
    ) -> Result<bool, Error> {
        if finished_ms < 0 {
            return Err(Error::Invalid("Invalid monetary completion time"));
        }
        let actual = actual_microusd.map(amount).transpose()?;
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
        let row: Option<(i64, String)> = tx
            .query_row(
                "SELECT start_ms,trace FROM request_costs WHERE attempt=?1 AND end_ms IS NULL",
                [attempt],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        let Some((start, trace)) = row else {
            return Ok(false);
        };
        // A backward clock must not shorten the charged window.
        let end = finished_ms.max(start);
        tx.execute(
            "UPDATE request_costs SET actual_microusd=?1,end_ms=?2 WHERE attempt=?3",
            params![actual, end, attempt],
        )?;
        // UPDATE cannot recreate a trace the user erased while the request was running.
        tx.execute(
            "UPDATE traces SET end_ms=?1,status=?2,
             attrs=json_set(attrs,'$.\"vults.cost.actual_microusd\"',?3)
             WHERE span=?4 AND trace=?5 AND kind='request'",
            params![end, if ok { "ok" } else { "error" }, actual, attempt, trace],
        )?;
        tx.commit()?;
        Ok(true)
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
            name: "request voice-realtime".into(),
            kind: "request".into(),
            start_ms: at,
            end_ms: at,
            status: "started".into(),
            attrs: json!({"vults.feature":"voice-realtime"}),
        }
    }
    const FEATURE: &str = "voice-realtime";
    const LIMIT: u64 = 1_000_000;

    #[test]
    fn holds_share_the_limit_and_only_final_usage_releases_unused_money() {
        let mut db = Store::open(&crate::temp("money-settle")).unwrap();
        let id = db.request_budget_id().unwrap();
        assert!(
            db.reserve_cost(&id, FEATURE, LIMIT, Some(600_000), &span(DAY_MS, "a"))
                .unwrap()
        );
        assert!(
            !db.reserve_cost(&id, FEATURE, LIMIT, Some(400_001), &span(DAY_MS, "b"))
                .unwrap()
        );
        assert!(
            db.reserve_cost(&id, FEATURE, LIMIT, Some(400_000), &span(DAY_MS, "b"))
                .unwrap()
        );
        assert!(
            !db.reserve_cost(&id, FEATURE, LIMIT, Some(1), &span(DAY_MS, "c"))
                .unwrap()
        );
        assert!(
            db.settle_cost(&id, "a", Some(100_000), DAY_MS + 10, false)
                .unwrap()
        );
        assert!(!db.settle_cost(&id, "a", Some(0), DAY_MS + 11, true).unwrap());
        assert!(
            db.reserve_cost(&id, FEATURE, LIMIT, Some(500_000), &span(DAY_MS + 12, "c"))
                .unwrap()
        );
        let traces = db.spans(10).unwrap();
        let a = traces.iter().find(|s| s.span == "a").unwrap();
        assert_eq!(a.attrs["vults.cost.actual_microusd"], 100_000);
        assert_eq!(a.attrs["vults.cost.reserved_microusd"], 600_000);
        assert_eq!(a.status, "error");
    }

    #[test]
    fn unknown_cost_and_open_holds_survive_restart_and_history_erasure() {
        let dir = crate::temp("money-retention");
        let mut db = Store::open(&dir).unwrap();
        let id = db.request_budget_id().unwrap();
        assert!(
            db.reserve_cost(&id, FEATURE, LIMIT, Some(LIMIT), &span(DAY_MS, "a"))
                .unwrap()
        );
        db.clear_all(&dir).unwrap();
        drop(db);
        let mut db = Store::open_current(&dir).unwrap();
        assert!(
            !db.reserve_cost(&id, FEATURE, LIMIT, Some(1), &span(5 * DAY_MS, "b"))
                .unwrap()
        );
        assert!(db.settle_cost(&id, "a", None, 5 * DAY_MS, false).unwrap());
        assert!(db.spans(10).unwrap().is_empty());
        assert!(
            !db.reserve_cost(&id, FEATURE, LIMIT, Some(1), &span(6 * DAY_MS - 1, "b"))
                .unwrap()
        );
        assert!(
            db.reserve_cost(&id, FEATURE, LIMIT, Some(LIMIT), &span(6 * DAY_MS, "b"))
                .unwrap()
        );
        assert!(
            !db.reserve_cost(&id, FEATURE, LIMIT, Some(1), &span(0, "rollback"))
                .unwrap()
        );
    }

    #[test]
    fn zero_unknown_invalid_and_unbounded_quotes_never_grant_admission() {
        let mut db = Store::open(&crate::temp("money-quote")).unwrap();
        let id = db.request_budget_id().unwrap();
        for (limit, quote) in [
            (0, Some(1)),
            (LIMIT, None),
            (LIMIT, Some(0)),
            (LIMIT, Some(LIMIT + 1)),
        ] {
            assert!(
                !db.reserve_cost(&id, FEATURE, limit, quote, &span(0, "a"))
                    .unwrap()
            );
        }
        assert!(
            db.reserve_cost(&id, FEATURE, u64::MAX, Some(1), &span(0, "a"))
                .is_err()
        );
        assert!(
            db.reserve_cost(&id, FEATURE, LIMIT, Some(1), &span(-1, "a"))
                .is_err()
        );
        assert!(db.spans(10).unwrap().is_empty());
        assert_eq!(charged(&db.conn, FEATURE, 0).unwrap(), 0);
    }

    #[test]
    fn unknown_usage_stays_null_and_overruns_are_not_truncated() {
        let mut db = Store::open(&crate::temp("money-overrun")).unwrap();
        let id = db.request_budget_id().unwrap();
        assert!(
            db.reserve_cost(&id, FEATURE, LIMIT, Some(1), &span(DAY_MS, "unknown"))
                .unwrap()
        );
        assert!(db.settle_cost(&id, "unknown", None, DAY_MS, true).unwrap());
        let s = &db.spans(1).unwrap()[0];
        assert_eq!(s.attrs["vults.cost.actual_microusd"], serde_json::Value::Null);
        assert_eq!(s.status, "ok");
        assert!(
            db.reserve_cost(&id, FEATURE, LIMIT, Some(1), &span(DAY_MS, "overrun"))
                .unwrap()
        );
        assert!(
            db.settle_cost(&id, "overrun", Some(LIMIT + 1), DAY_MS, false)
                .unwrap()
        );
        assert_eq!(charged(&db.conn, FEATURE, DAY_MS).unwrap(), LIMIT as i64 + 2);
        assert!(
            !db.reserve_cost(&id, FEATURE, LIMIT, Some(1), &span(DAY_MS, "next"))
                .unwrap()
        );
        assert!(
            db.reserve_cost(&id, "independent", LIMIT, Some(LIMIT), &span(DAY_MS, "other"))
                .unwrap()
        );
    }

    #[test]
    fn settlement_is_atomic_bound_to_the_original_trace_and_cannot_refund_twice() {
        let mut db = Store::open(&crate::temp("money-settlement-failure")).unwrap();
        let id = db.request_budget_id().unwrap();
        assert!(
            db.reserve_cost(&id, FEATURE, LIMIT, Some(100), &span(DAY_MS, "a"))
                .unwrap()
        );
        assert!(
            db.reserve_cost(&id, FEATURE, LIMIT, Some(1), &span(DAY_MS, "a"))
                .is_err()
        );
        let mut unrelated = span(DAY_MS, "a");
        unrelated.trace = "unrelated".into();
        db.append_span(&unrelated).unwrap();
        db.conn.execute_batch("CREATE TRIGGER refuse_cost_settlement BEFORE UPDATE ON traces BEGIN SELECT RAISE(ABORT,'test'); END;").unwrap();
        assert!(db.settle_cost(&id, "a", Some(1), DAY_MS, true).is_err());
        assert_eq!(charged(&db.conn, FEATURE, 5 * DAY_MS).unwrap(), 100);
        db.conn
            .execute_batch("DROP TRIGGER refuse_cost_settlement;")
            .unwrap();
        assert!(db.settle_cost(&id, "a", Some(1), 0, true).unwrap());
        assert!(!db.settle_cost(&id, "a", Some(0), DAY_MS, true).unwrap());
        let traces = db.spans(10).unwrap();
        let original = traces.iter().find(|s| s.trace == "a").unwrap();
        assert_eq!(original.end_ms, DAY_MS);
        assert_eq!(original.attrs["vults.cost.actual_microusd"], 1);
        assert_eq!(
            traces.iter().find(|s| s.trace == "unrelated").unwrap(),
            &unrelated
        );
    }

    #[test]
    fn concurrent_connections_cannot_spend_the_same_last_microdollar() {
        let dir = crate::temp("money-race");
        let id = Store::open(&dir).unwrap().request_budget_id().unwrap();
        let threads: Vec<_> = (0..8)
            .map(|n| {
                let (dir, id) = (dir.clone(), id.clone());
                std::thread::spawn(move || {
                    Store::open_current(&dir)
                        .unwrap()
                        .reserve_cost(&id, FEATURE, 1, Some(1), &span(DAY_MS, &n.to_string()))
                        .unwrap()
                })
            })
            .collect();
        assert_eq!(
            threads
                .into_iter()
                .map(|t| t.join().unwrap())
                .filter(|ok| *ok)
                .count(),
            1
        );
    }

    #[test]
    fn replaced_missing_or_overflowing_ledgers_and_trace_failure_fail_closed() {
        let dir = crate::temp("money-failure");
        let mut db = Store::open(&dir).unwrap();
        let old = db.request_budget_id().unwrap();
        drop(db);
        std::fs::remove_dir_all(&dir).unwrap();
        assert!(Store::open_current(&dir).is_err());
        db = Store::open(&dir).unwrap();
        let id = db.request_budget_id().unwrap();
        assert!(
            !db.reserve_cost(&old, FEATURE, LIMIT, Some(1), &span(0, "a"))
                .unwrap()
        );
        assert!(!db.settle_cost(&old, "a", Some(0), 0, true).unwrap());
        db.conn.execute_batch("CREATE TRIGGER refuse_money_trace BEFORE INSERT ON traces BEGIN SELECT RAISE(ABORT,'test'); END;").unwrap();
        assert!(
            db.reserve_cost(&id, FEATURE, LIMIT, Some(1), &span(0, "a"))
                .is_err()
        );
        assert_eq!(charged(&db.conn, FEATURE, 0).unwrap(), 0);
        db.conn.execute_batch("DROP TRIGGER refuse_money_trace;").unwrap();
        for n in ["a", "b"] {
            assert!(
                db.reserve_cost(&id, FEATURE, LIMIT, Some(1), &span(0, n))
                    .unwrap()
            );
        }
        for n in ["a", "b"] {
            assert!(db.settle_cost(&id, n, Some(i64::MAX as u64), 0, true).unwrap());
        }
        assert!(
            db.reserve_cost(&id, FEATURE, LIMIT, Some(1), &span(0, "overflow"))
                .is_err()
        );
        db.conn.execute("DROP TABLE request_costs", []).unwrap();
        assert!(
            db.reserve_cost(&id, FEATURE, LIMIT, Some(1), &span(0, "missing"))
                .is_err()
        );
        assert!(db.settle_cost(&id, "a", Some(0), 0, true).is_err());
    }
}
