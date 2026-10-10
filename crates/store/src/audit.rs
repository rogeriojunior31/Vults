//! The audit log (ADR 0014, `docs/dev/plan-zeca.md` S2): every answer to a card, by whom, and every
//! agent config the installer wrote. Append-only: the database itself refuses to change a row, or
//! to delete one younger than [`KEEP_DAYS`].

use rusqlite::params;
use vults_core::audit::Audit;

use crate::{Error, Store};

pub const KEEP_DAYS: i64 = 90;

/// Migration 2: the table and its two guards (90 is [`KEEP_DAYS`]). Shipped: never edit it.
pub(crate) const TABLE: &str = "
    CREATE TABLE audit (
        id INTEGER PRIMARY KEY,
        at INTEGER NOT NULL,
        actor TEXT NOT NULL,
        act TEXT NOT NULL,
        agent TEXT NOT NULL,
        project TEXT NOT NULL,
        tool TEXT NOT NULL,
        target TEXT NOT NULL
    );
    CREATE INDEX audit_at ON audit (at);
    CREATE TRIGGER audit_never_changes BEFORE UPDATE ON audit
    BEGIN
        SELECT RAISE(ABORT, 'the audit log is append-only');
    END;
    CREATE TRIGGER audit_keeps_90_days BEFORE DELETE ON audit
    WHEN old.at > CAST(strftime('%s', 'now') AS INTEGER) - 90 * 86400
    BEGIN
        SELECT RAISE(ABORT, 'audit rows are kept 90 days');
    END;";

/// A row as read back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    /// Unix seconds.
    pub at: i64,
    pub actor: String,
    pub act: String,
    pub agent: String,
    pub project: String,
    pub tool: String,
    pub target: String,
}

impl Store {
    /// One more line, at `at` (Unix seconds).
    pub fn append_audit(&mut self, at: i64, a: &Audit) -> Result<(), Error> {
        self.conn.execute(
            // Never later than the database's own clock: a line from a clock run ahead would
            // otherwise be guarded, and kept, for years.
            "INSERT INTO audit (at, actor, act, agent, project, tool, target)
             VALUES (MIN(?1, CAST(strftime('%s', 'now') AS INTEGER)), ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                at,
                a.actor.name(),
                a.act.name(),
                a.agent,
                a.project,
                a.tool,
                a.target
            ],
        )?;
        Ok(())
    }

    /// The newest `limit` lines, newest first.
    pub fn audit(&self, limit: u32) -> Result<Vec<Line>, Error> {
        let mut stmt = self.conn.prepare(
            "SELECT at, actor, act, agent, project, tool, target FROM audit
             ORDER BY at DESC, id DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map([limit], |r| {
            Ok(Line {
                at: r.get(0)?,
                actor: r.get(1)?,
                act: r.get(2)?,
                agent: r.get(3)?,
                project: r.get(4)?,
                tool: r.get(5)?,
                target: r.get(6)?,
            })
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    /// Drops the lines older than 90 days, counted back from `now` (Unix seconds).
    pub fn prune_audit(&mut self, now: i64) -> Result<(), Error> {
        self.conn
            .execute("DELETE FROM audit WHERE at < ?1", [now - KEEP_DAYS * 86400])?;
        Ok(())
    }

    /// The user's own "remove everything": the guards step aside for one transaction, every line
    /// goes, and the same guards (as the schema holds them now) come back.
    pub fn clear_audit(&mut self) -> Result<(), Error> {
        let tx = self
            .conn
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        clear_audit_in(&tx)?;
        tx.commit()?;
        self.compact()
    }
}

/// Inside a write transaction already taken: every line, the live guards kept.
pub(crate) fn clear_audit_in(tx: &rusqlite::Transaction) -> Result<(), Error> {
    {
        let guards: Vec<(String, String)> = {
            let mut stmt = tx.prepare(
                "SELECT name, sql FROM sqlite_master WHERE type = 'trigger' AND tbl_name = 'audit'",
            )?;
            stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
                .collect::<Result<_, _>>()?
        };
        for (name, _) in &guards {
            tx.execute_batch(&format!("DROP TRIGGER \"{}\";", name.replace('"', "\"\"")))?;
        }
        tx.execute("DELETE FROM audit", [])?;
        for (_, sql) in &guards {
            tx.execute_batch(sql)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use crate::temp;
    use vults_core::audit::{Act, Actor};

    const DAY: i64 = 86400;

    fn now() -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64
    }

    fn line() -> Audit {
        Audit::new(Actor::Human, Act::Allow, "claude", "site", "Bash", "cargo test")
    }

    #[test]
    fn a_line_is_kept_and_read_back_newest_first() {
        let mut s = Store::open(&temp("a-append")).unwrap();
        s.append_audit(100, &line()).unwrap();
        let rule = Audit::new(Actor::Rule, Act::Allow, "codex", "api", "shell", "ls");
        s.append_audit(200, &rule).unwrap();
        let lines = s.audit(10).unwrap();
        assert_eq!(lines.len(), 2);
        assert_eq!(
            (lines[0].at, lines[0].actor.as_str(), lines[0].agent.as_str()),
            (200, "rule", "codex")
        );
        assert_eq!(
            (lines[1].act.as_str(), lines[1].target.as_str()),
            ("allow", "cargo test")
        );
    }

    #[test]
    fn the_table_refuses_to_change_or_drop_a_recent_line() {
        let mut s = Store::open(&temp("a-guard")).unwrap();
        s.append_audit(now(), &line()).unwrap();
        assert!(s.conn.execute("UPDATE audit SET actor = 'rule'", []).is_err());
        assert!(s.conn.execute("DELETE FROM audit", []).is_err());
        assert_eq!(s.audit(10).unwrap()[0].actor, "human");
    }

    #[test]
    fn prune_drops_only_lines_past_90_days() {
        let mut s = Store::open(&temp("a-prune")).unwrap();
        let t = now();
        for ago in [0, 30, 89, 91, 200] {
            s.append_audit(t - ago * DAY, &line()).unwrap();
        }
        s.prune_audit(t).unwrap();
        assert_eq!(s.audit(10).unwrap().len(), 3);
    }

    #[test]
    fn clear_empties_it_and_the_guards_come_back() {
        let mut s = Store::open(&temp("a-clear")).unwrap();
        s.append_audit(now(), &line()).unwrap();
        s.clear_audit().unwrap();
        assert!(s.audit(10).unwrap().is_empty());
        s.append_audit(now(), &line()).unwrap();
        assert!(s.conn.execute("DELETE FROM audit", []).is_err());
    }

    #[test]
    fn a_line_from_a_clock_run_ahead_is_timed_now() {
        let mut s = Store::open(&temp("a-future")).unwrap();
        s.append_audit(now() + 100 * 365 * DAY, &line()).unwrap();
        assert!(s.audit(1).unwrap()[0].at <= now() + 1);
    }

    #[test]
    fn clear_all_takes_history_and_audit_together() {
        let dir = temp("a-clear-all");
        let mut s = Store::open(&dir).unwrap();
        s.append_audit(now(), &line()).unwrap();
        let turn = vults_core::recap::Entry {
            v: 1,
            end: 1,
            day: "2026-10-09".into(),
            secs: 1,
            agent: vults_protocol::AgentKind::Claude,
            project: "site".into(),
            steps: 0,
            commands: 0,
            files: 0,
            added: 0,
            removed: 0,
            allowed: 0,
            denied: 0,
            answered: 0,
            questions: 0,
            failed: false,
        };
        s.append_turn(&turn).unwrap();
        s.clear_all(&dir).unwrap();
        assert!(s.audit(10).unwrap().is_empty());
        assert!(s.turns().unwrap().is_empty() && s.days().unwrap().is_empty());
        let wal = dir.join(format!("{}-wal", crate::FILE));
        assert!(!wal.exists() || std::fs::metadata(&wal).unwrap().len() == 0);
    }
}
