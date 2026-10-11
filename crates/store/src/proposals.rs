//! Memory proposals (`docs/dev/plan-zeca.md` M6): what Zeca would remember, waiting for the user's
//! yes in the Nest. Nothing reaches his memory without it; a proposal already waiting, or one the
//! user turned down lately, is never made again.

use rusqlite::{OptionalExtension, params};

use crate::{Error, Store};

/// A proposal turned down stays a no this long: asking again every conversation would nag.
const NO_FOR_MS: i64 = 30 * 24 * 3600 * 1000;
/// Decided proposals are kept this long, for the acceptance rate.
pub const KEEP_DAYS: i64 = 90;

/// Migration 11. Shipped: never edit it.
pub(crate) const TABLE: &str = "
    CREATE TABLE memory_proposals (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        ms INTEGER NOT NULL,
        op TEXT NOT NULL,
        file TEXT NOT NULL,
        place TEXT NOT NULL,
        old TEXT NOT NULL,
        new TEXT NOT NULL,
        reason TEXT NOT NULL,
        source TEXT NOT NULL,
        status TEXT NOT NULL DEFAULT 'pending',
        decided_ms INTEGER,
        note TEXT,
        folder TEXT NOT NULL DEFAULT '',
        norm TEXT NOT NULL DEFAULT ''
    );
    CREATE INDEX memory_proposals_status ON memory_proposals (status, id);
    CREATE UNIQUE INDEX memory_proposals_waiting ON memory_proposals (file, op, norm)
        WHERE status = 'pending';
    ALTER TABLE conversations ADD COLUMN extracted INTEGER NOT NULL DEFAULT 0;";

/// A proposal as made.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct New {
    /// `add`, `replace` or `remove`.
    pub op: String,
    /// The memory file it changes (`system/USER.md`, `roosts/<name>.md`).
    pub file: String,
    /// What the user reads as its place: "About you", or the roost's name. The roost comes from
    /// the folder's own claim (M3), so the user must see it before saying yes.
    pub place: String,
    /// The fact replaced or removed, as a substring; empty for `add`.
    pub old: String,
    /// The fact added or replacing; empty for `remove`.
    pub new: String,
    /// Why, in the model's words.
    pub reason: String,
    /// `tool` (the chat's model asked) or `extraction` (read from a finished conversation).
    pub source: String,
    /// For a roost, the folder its claim came from, shown with its name.
    pub folder: String,
}

/// A fact compared as a person would: case, spaces and the final stop aside.
pub fn normal(text: &str) -> String {
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim_end_matches(['.', '!', ';'])
        .to_lowercase()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Proposal {
    pub id: i64,
    pub ms: i64,
    pub new: New,
    pub status: String,
}

fn proposal(r: &rusqlite::Row<'_>) -> rusqlite::Result<Proposal> {
    Ok(Proposal {
        id: r.get(0)?,
        ms: r.get(1)?,
        new: New {
            op: r.get(2)?,
            file: r.get(3)?,
            place: r.get(4)?,
            old: r.get(5)?,
            new: r.get(6)?,
            reason: r.get(7)?,
            source: r.get(8)?,
            folder: r.get(10)?,
        },
        status: r.get(9)?,
    })
}

const COLUMNS: &str = "id, ms, op, file, place, old, new, reason, source, status, folder";

impl Store {
    /// Keeps a proposal, unless the same one (as a person reads it) waits, or was decided within
    /// 30 days: none then. Its texts are redacted like the journal's. One transaction, and a
    /// unique index on what waits, so two at once never both get in.
    pub fn proposal_add(&mut self, p: &New, now_ms: i64) -> Result<Option<i64>, Error> {
        let clean = |t: &str| vults_core::redact::secrets(t);
        let (old, new, reason) = (clean(&p.old), clean(&p.new), clean(&p.reason));
        let norm = format!("{}\u{1f}{}", normal(&old), normal(&new));
        let tx = self
            .conn
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let seen: bool = tx.query_row(
            "SELECT EXISTS (SELECT 1 FROM memory_proposals WHERE op = ?1 AND file = ?2 AND norm = ?3
             AND (status = 'pending' OR decided_ms > ?4))",
            params![p.op, p.file, norm, now_ms - NO_FOR_MS],
            |r| r.get(0),
        )?;
        if seen {
            return Ok(None);
        }
        tx.execute(
            "INSERT INTO memory_proposals (ms, op, file, place, old, new, reason, source, folder, norm)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                now_ms, p.op, p.file, p.place, old, new, reason, p.source, p.folder, norm
            ],
        )?;
        let id = tx.last_insert_rowid();
        tx.commit()?;
        Ok(Some(id))
    }

    /// How many lines of a conversation the memory's suggestions already read.
    pub fn conversation_extracted(&self, id: crate::conversations::ConversationId) -> Result<i64, Error> {
        Ok(self
            .conn
            .query_row("SELECT extracted FROM conversations WHERE id = ?1", [id.0], |r| {
                r.get(0)
            })
            .optional()?
            .unwrap_or(0))
    }

    pub fn conversation_set_extracted(
        &mut self,
        id: crate::conversations::ConversationId,
        lines: i64,
    ) -> Result<(), Error> {
        self.conn.execute(
            "UPDATE conversations SET extracted = ?2 WHERE id = ?1",
            params![id.0, lines],
        )?;
        Ok(())
    }

    /// The proposals waiting, oldest first.
    pub fn proposals_pending(&self) -> Result<Vec<Proposal>, Error> {
        let mut stmt = self.conn.prepare(&format!(
            "SELECT {COLUMNS} FROM memory_proposals WHERE status = 'pending' ORDER BY id"
        ))?;
        let rows = stmt.query_map([], proposal)?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    pub fn proposal(&self, id: i64) -> Result<Option<Proposal>, Error> {
        Ok(self
            .conn
            .query_row(
                &format!("SELECT {COLUMNS} FROM memory_proposals WHERE id = ?1"),
                [id],
                proposal,
            )
            .optional()?)
    }

    /// The user's answer: `accepted`, `rejected`, or `failed` (accepted, but the memory refused
    /// it: full, gone, ambiguous; `note` says why). Only a waiting proposal can be decided.
    pub fn proposal_decide(
        &mut self,
        id: i64,
        status: &str,
        note: Option<&str>,
        now_ms: i64,
    ) -> Result<bool, Error> {
        let changed = self.conn.execute(
            "UPDATE memory_proposals SET status = ?2, note = ?3, decided_ms = ?4
             WHERE id = ?1 AND status = 'pending'",
            params![id, status, note, now_ms],
        )?;
        Ok(changed == 1)
    }

    /// Proposals accepted and turned down in the last [`KEEP_DAYS`]: the board's "half accepted".
    pub fn proposals_decided(&self, now_ms: i64) -> Result<(u32, u32), Error> {
        let since = now_ms - KEEP_DAYS * 24 * 3600 * 1000;
        Ok(self.conn.query_row(
            "SELECT COUNT(*) FILTER (WHERE status = 'accepted'),
                    COUNT(*) FILTER (WHERE status = 'rejected')
             FROM memory_proposals WHERE decided_ms > ?1",
            [since],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?)
    }

    /// Drops decided proposals past [`KEEP_DAYS`].
    pub fn proposals_prune(&mut self, now_ms: i64) -> Result<(), Error> {
        self.conn.execute(
            "DELETE FROM memory_proposals WHERE status != 'pending' AND decided_ms < ?1",
            [now_ms - KEEP_DAYS * 24 * 3600 * 1000],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    fn store(name: &str) -> Store {
        Store::open(&crate::temp(name)).unwrap()
    }

    fn add(fact: &str) -> New {
        New {
            op: "add".into(),
            file: "system/USER.md".into(),
            place: "About you".into(),
            new: fact.into(),
            reason: "said so".into(),
            source: "tool".into(),
            ..New::default()
        }
    }

    #[test]
    fn no_duplicate_waits_and_a_no_holds_for_a_month() {
        let mut s = store("proposals-dup");
        let day = 24 * 3600 * 1000;
        let a = s.proposal_add(&add("Uses fish."), 0).unwrap().unwrap();
        assert_eq!(
            s.proposal_add(&add("Uses fish."), 1).unwrap(),
            None,
            "already waiting"
        );
        assert_eq!(
            s.proposal_add(&add("  uses   FISH "), 1).unwrap(),
            None,
            "as a person reads it"
        );
        assert!(s.proposal_decide(a, "rejected", None, day).unwrap());
        assert!(
            !s.proposal_decide(a, "accepted", None, day).unwrap(),
            "decided once"
        );
        assert_eq!(
            s.proposal_add(&add("Uses fish."), 2 * day).unwrap(),
            None,
            "a no holds"
        );
        assert!(s.proposal_add(&add("Uses fish."), 40 * day).unwrap().is_some());
        let b = s.proposal_add(&add("Uses vim."), 40 * day).unwrap().unwrap();
        s.proposal_decide(b, "accepted", None, 41 * day).unwrap();
        assert_eq!(
            s.proposal_add(&add("Uses vim"), 42 * day).unwrap(),
            None,
            "a yes holds too"
        );
        assert_eq!(s.proposals_pending().unwrap().len(), 1);
        assert_eq!(s.proposals_decided(41 * day).unwrap(), (1, 1));
        s.proposals_prune(200 * day).unwrap();
        assert_eq!(s.proposals_decided(200 * day).unwrap(), (0, 0));
        assert_eq!(s.proposals_pending().unwrap().len(), 1, "waiting ones stay");
    }

    #[test]
    fn a_proposal_s_texts_are_redacted() {
        let mut s = store("proposals-secret");
        let id = s
            .proposal_add(&add("Deploys with ghp_abcdefghijklmnopqrstuvwxyz0123456789"), 0)
            .unwrap()
            .unwrap();
        let p = s.proposal(id).unwrap().unwrap();
        assert!(!p.new.new.contains("ghp_abc"), "{}", p.new.new);
    }
}
