//! What the agents cost (`docs/dev/plan-zeca.md` W9), by local day, by what spent it (a task's run,
//! a watched session) and by roost: for the Nest's costs and the user's daily budget. Dollars are
//! what the agent itself reported, never guessed from a price list; on a subscription they are
//! what the tokens would cost at API prices, which the plan does not charge.

use rusqlite::{OptionalExtension, params};

use crate::{Error, Store};

/// Migration 19. Shipped: never edit it.
pub(crate) const TABLE: &str = "
    CREATE TABLE spend (
        day TEXT NOT NULL,
        source TEXT NOT NULL,
        roost TEXT NOT NULL DEFAULT '',
        plan TEXT NOT NULL DEFAULT '',
        usd REAL NOT NULL DEFAULT 0,
        tokens INTEGER NOT NULL DEFAULT 0,
        PRIMARY KEY (day, source)
    );
    CREATE INDEX spend_roost ON spend (roost);
    CREATE TABLE spend_seen (
        source TEXT PRIMARY KEY,
        total REAL NOT NULL
    );";

/// Where a cost came from and how it is billed.
#[derive(Debug, Clone, PartialEq)]
pub struct Source<'a> {
    /// `YYYY-MM-DD`, the user's local day.
    pub day: &'a str,
    /// `task:12`, or a watched session's `claude:<id>`.
    pub source: &'a str,
    /// The roost's key; empty outside any.
    pub roost: &'a str,
    /// `subscription`, `api`, or empty when the agent did not say.
    pub plan: &'a str,
}

/// One roost's costs.
#[derive(Debug, Clone, PartialEq)]
pub struct Roost {
    pub roost: String,
    pub today: f64,
    pub total: f64,
    /// Whether any of it was on a subscription: then it is "at API prices".
    pub subscription: bool,
}

/// A day's costs, and whether any was on a subscription.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Day {
    pub usd: f64,
    pub tokens: i64,
    pub subscription: bool,
}

fn finite(usd: f64) -> f64 {
    if usd.is_finite() && usd > 0.0 { usd } else { 0.0 }
}

impl Store {
    /// Adds what one run spent.
    pub fn spend_add(&mut self, at: &Source<'_>, usd: f64, tokens: i64) -> Result<(), Error> {
        let (usd, tokens) = (finite(usd), tokens.max(0));
        if usd == 0.0 && tokens == 0 {
            return Ok(());
        }
        self.conn.execute(
            "INSERT INTO spend (day, source, roost, plan, usd, tokens) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT (day, source) DO UPDATE SET usd = usd + ?5, tokens = tokens + ?6,
                 plan = CASE WHEN ?4 = '' THEN plan ELSE ?4 END",
            params![at.day, at.source, at.roost, at.plan, usd, tokens],
        )?;
        Ok(())
    }

    /// A source that reports its running total (Claude Code's statusLine): only what it grew by
    /// since the last report counts, so a report repeated on every redraw costs nothing. A total
    /// below the last one is a new count (a resumed session starts from zero) and counts whole.
    /// Gives back what was added.
    pub fn spend_total(&mut self, at: &Source<'_>, total: f64) -> Result<f64, Error> {
        let total = finite(total);
        let tx = self
            .conn
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let seen: Option<f64> = tx
            .query_row(
                "SELECT total FROM spend_seen WHERE source = ?1",
                [at.source],
                |r| r.get(0),
            )
            .optional()?;
        let grew = match seen {
            Some(s) if total >= s => total - s,
            _ => total,
        };
        tx.execute(
            "INSERT INTO spend_seen (source, total) VALUES (?1, ?2)
             ON CONFLICT (source) DO UPDATE SET total = ?2",
            params![at.source, total],
        )?;
        if grew > 0.0 {
            tx.execute(
                "INSERT INTO spend (day, source, roost, plan, usd) VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT (day, source) DO UPDATE SET usd = usd + ?5,
                     plan = CASE WHEN ?4 = '' THEN plan ELSE ?4 END",
                params![at.day, at.source, at.roost, at.plan, grew],
            )?;
        }
        tx.commit()?;
        Ok(grew)
    }

    /// What a day cost in all.
    pub fn spend_day(&self, day: &str) -> Result<Day, Error> {
        Ok(self.conn.query_row(
            "SELECT COALESCE(SUM(usd), 0), COALESCE(SUM(tokens), 0),
                    COALESCE(MAX(plan = 'subscription'), 0)
             FROM spend WHERE day = ?1",
            [day],
            |r| {
                Ok(Day {
                    usd: r.get(0)?,
                    tokens: r.get(1)?,
                    subscription: r.get::<_, i64>(2)? != 0,
                })
            },
        )?)
    }

    /// Each roost's cost, today's and in all, the costliest first.
    pub fn spend_roosts(&self, day: &str) -> Result<Vec<Roost>, Error> {
        let mut stmt = self.conn.prepare(
            "SELECT roost, COALESCE(SUM(CASE WHEN day = ?1 THEN usd END), 0), SUM(usd),
                    MAX(plan = 'subscription')
             FROM spend WHERE roost != '' GROUP BY roost ORDER BY SUM(usd) DESC",
        )?;
        let rows = stmt.query_map([day], |r| {
            Ok(Roost {
                roost: r.get(0)?,
                today: r.get(1)?,
                total: r.get(2)?,
                subscription: r.get::<_, i64>(3)? != 0,
            })
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use crate::temp;

    fn at<'a>(day: &'a str, source: &'a str, roost: &'a str, plan: &'a str) -> Source<'a> {
        Source {
            day,
            source,
            roost,
            plan,
        }
    }

    #[test]
    fn runs_add_up_by_day_and_roost() {
        let mut s = Store::open(&temp("spend")).unwrap();
        s.spend_add(&at("2026-10-10", "task:1", "github:me/site", ""), 0.50, 1000)
            .unwrap();
        s.spend_add(
            &at("2026-10-11", "task:1", "github:me/site", "subscription"),
            0.25,
            500,
        )
        .unwrap();
        s.spend_add(&at("2026-10-11", "task:1", "github:me/site", ""), 0.25, 0)
            .unwrap();
        s.spend_add(&at("2026-10-11", "task:2", "", "api"), 1.0, 0)
            .unwrap();
        // Nothing spent, or a garbled number, adds nothing.
        s.spend_add(&at("2026-10-11", "task:3", "github:me/api", ""), f64::NAN, 0)
            .unwrap();
        s.spend_add(&at("2026-10-11", "task:3", "github:me/api", ""), -4.0, -1)
            .unwrap();

        let day = s.spend_day("2026-10-11").unwrap();
        assert!((day.usd - 1.5).abs() < 1e-9);
        assert_eq!(day.tokens, 500);
        assert!(
            day.subscription,
            "a known plan is not forgotten by a run that did not say"
        );
        assert_eq!(s.spend_day("2026-10-09").unwrap(), Day::default());

        let roosts = s.spend_roosts("2026-10-11").unwrap();
        assert_eq!(roosts.len(), 1, "no roost, no row; nothing spent, no row");
        assert_eq!(roosts[0].roost, "github:me/site");
        assert!((roosts[0].today - 0.5).abs() < 1e-9);
        assert!((roosts[0].total - 1.0).abs() < 1e-9);
    }

    #[test]
    fn a_running_total_counts_only_what_it_grew() {
        let mut s = Store::open(&temp("spend-total")).unwrap();
        let a = at("2026-10-11", "claude:abc", "github:me/site", "subscription");
        assert!((s.spend_total(&a, 0.40).unwrap() - 0.40).abs() < 1e-9);
        assert_eq!(s.spend_total(&a, 0.40).unwrap(), 0.0, "a redraw costs nothing");
        assert!((s.spend_total(&a, 0.65).unwrap() - 0.25).abs() < 1e-9);
        // The next day, the same session's growth is the next day's.
        let b = at("2026-10-12", "claude:abc", "github:me/site", "subscription");
        assert!((s.spend_total(&b, 0.70).unwrap() - 0.05).abs() < 1e-9);
        // Resumed: a new count from zero.
        assert!((s.spend_total(&b, 0.10).unwrap() - 0.10).abs() < 1e-9);
        assert!((s.spend_day("2026-10-11").unwrap().usd - 0.65).abs() < 1e-9);
        assert!((s.spend_day("2026-10-12").unwrap().usd - 0.15).abs() < 1e-9);
    }
}
