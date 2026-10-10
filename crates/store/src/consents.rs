//! The user's consents (ADR 0014, `docs/dev/plan-zeca.md` S13): each policy they accepted, the
//! diff they saw and the words it said. Apart from the audit log, because the audit can be cleared
//! and leaves after 90 days; a policy must never lose the proof that the user said yes. So the
//! database refuses every change but one `revoked_at`, set once, refuses every delete, and nothing
//! prunes it. Revoking is the only way to take a consent back, and it switches the policy off at
//! once: a policy answers only while [`Store::valid`] says its consent is.

use rusqlite::{OptionalExtension, params};
use sha2::{Digest, Sha256};

use crate::{Error, Store};

/// Migration 4: the table, its guards, and the audit's guard against `INSERT OR REPLACE` (which
/// removes the old row without firing a delete trigger). Shipped: never edit it.
pub(crate) const TABLE: &str = "
    CREATE TABLE consents (
        id INTEGER PRIMARY KEY,
        at INTEGER NOT NULL,
        kind TEXT NOT NULL,
        subject TEXT NOT NULL,
        diff_sha256 TEXT NOT NULL,
        text TEXT NOT NULL,
        revoked_at INTEGER
    );
    CREATE TRIGGER consents_only_revoked BEFORE UPDATE ON consents
    WHEN old.revoked_at IS NOT NULL OR new.revoked_at IS NULL
        OR new.id IS NOT old.id OR new.at IS NOT old.at OR new.kind IS NOT old.kind
        OR new.subject IS NOT old.subject OR new.diff_sha256 IS NOT old.diff_sha256
        OR new.text IS NOT old.text
    BEGIN
        SELECT RAISE(ABORT, 'a consent only changes to be revoked, once');
    END;
    CREATE TRIGGER consents_never_deleted BEFORE DELETE ON consents
    BEGIN
        SELECT RAISE(ABORT, 'a consent is never deleted: revoke it');
    END;
    CREATE TRIGGER consents_never_replaced BEFORE INSERT ON consents
    WHEN EXISTS (SELECT 1 FROM consents WHERE id = new.id)
    BEGIN
        SELECT RAISE(ABORT, 'a consent is never replaced');
    END;
    CREATE TRIGGER audit_never_replaced BEFORE INSERT ON audit
    WHEN EXISTS (SELECT 1 FROM audit WHERE id = new.id)
    BEGIN
        SELECT RAISE(ABORT, 'the audit log is append-only');
    END;";

/// A consent's row id; the policy that rests on it keeps it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ConsentId(pub i64);

/// A consent as read back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Consent {
    pub id: ConsentId,
    /// Unix seconds.
    pub at: i64,
    pub kind: String,
    pub subject: String,
    /// SHA-256 of the diff the user saw, in hex.
    pub diff_sha256: String,
    /// The words the user accepted.
    pub text: String,
    pub revoked_at: Option<i64>,
}

impl Store {
    /// Records that the user accepted `text` (of `kind`, about `subject`) after seeing `diff`.
    pub fn consent(&mut self, kind: &str, subject: &str, diff: &str, text: &str) -> Result<ConsentId, Error> {
        let digest: String = Sha256::digest(diff.as_bytes())
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        self.conn.execute(
            "INSERT INTO consents (at, kind, subject, diff_sha256, text)
             VALUES (CAST(strftime('%s', 'now') AS INTEGER), ?1, ?2, ?3, ?4)",
            params![kind, subject, digest, text],
        )?;
        Ok(ConsentId(self.conn.last_insert_rowid()))
    }

    /// Takes a consent back, now. Fails for one unknown or already revoked.
    pub fn revoke(&mut self, id: ConsentId) -> Result<(), Error> {
        let changed = self.conn.execute(
            "UPDATE consents SET revoked_at = CAST(strftime('%s', 'now') AS INTEGER) WHERE id = ?1",
            [id.0],
        )?;
        if changed == 0 {
            return Err(Error::Sqlite(rusqlite::Error::QueryReturnedNoRows));
        }
        Ok(())
    }

    /// Does this consent stand: recorded, and never revoked? However old it is.
    pub fn valid(&self, id: ConsentId) -> Result<bool, Error> {
        let revoked: Option<Option<i64>> = self
            .conn
            .query_row("SELECT revoked_at FROM consents WHERE id = ?1", [id.0], |r| {
                r.get(0)
            })
            .optional()?;
        Ok(matches!(revoked, Some(None)))
    }

    /// Every consent, newest first.
    pub fn consents(&self) -> Result<Vec<Consent>, Error> {
        let mut stmt = self.conn.prepare(
            "SELECT id, at, kind, subject, diff_sha256, text, revoked_at FROM consents
             ORDER BY at DESC, id DESC",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok(Consent {
                id: ConsentId(r.get(0)?),
                at: r.get(1)?,
                kind: r.get(2)?,
                subject: r.get(3)?,
                diff_sha256: r.get(4)?,
                text: r.get(5)?,
                revoked_at: r.get(6)?,
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

    const DAY: i64 = 86400;

    fn store(name: &str) -> (Store, std::path::PathBuf) {
        let dir = temp(name);
        (Store::open(&dir).unwrap(), dir)
    }

    #[test]
    fn a_consent_stands_until_revoked_once() {
        let (mut s, _) = store("consent-revoke");
        let id = s
            .consent(
                "policy",
                "cargo test",
                "+ allow cargo test",
                "Allow cargo test here",
            )
            .unwrap();
        assert!(s.valid(id).unwrap());
        let c = &s.consents().unwrap()[0];
        assert_eq!((c.kind.as_str(), c.subject.as_str()), ("policy", "cargo test"));
        assert_eq!(c.diff_sha256.len(), 64);
        s.revoke(id).unwrap();
        assert!(!s.valid(id).unwrap(), "revoking switches it off at once");
        assert!(s.revoke(id).is_err(), "a second revoke fails");
        assert!(s.revoke(ConsentId(999)).is_err(), "an unknown one fails");
        assert!(!s.valid(ConsentId(999)).unwrap());
    }

    #[test]
    fn the_database_refuses_any_other_change() {
        let (mut s, _) = store("consent-guards");
        let id = s.consent("policy", "x", "d", "t").unwrap();
        for col in [
            "at = at - 1",
            "kind = 'other'",
            "subject = 'y'",
            "diff_sha256 = 'z'",
            "text = 'u'",
            "id = 77",
        ] {
            let sql = format!("UPDATE consents SET {col} WHERE id = {}", id.0);
            assert!(s.conn.execute(&sql, []).is_err(), "{col}");
        }
        assert!(
            s.conn
                .execute("UPDATE consents SET revoked_at = NULL", [])
                .is_err(),
            "even a no-op: the only change is a revoke"
        );
        assert!(s.conn.execute("DELETE FROM consents", []).is_err());
        assert!(
            s.conn
                .execute(
                    "INSERT OR REPLACE INTO consents (id, at, kind, subject, diff_sha256, text) VALUES (?1, 0, 'k', 's', 'd', 't')",
                    [id.0],
                )
                .is_err(),
            "a replace would delete the old row unseen"
        );
        s.revoke(id).unwrap();
        assert!(
            s.conn
                .execute("UPDATE consents SET revoked_at = NULL", [])
                .is_err(),
            "never un-revoked"
        );
        assert!(
            s.conn
                .execute("UPDATE consents SET revoked_at = revoked_at + 1", [])
                .is_err()
        );
        assert!(s.valid(id).is_ok_and(|v| !v));
    }

    #[test]
    fn nothing_prunes_or_clears_a_consent() {
        let (mut s, dir) = store("consent-old");
        s.conn
            .execute(
                "INSERT INTO consents (at, kind, subject, diff_sha256, text)
                 VALUES (CAST(strftime('%s', 'now') AS INTEGER) - 400 * 86400, 'policy', 's', 'd', 't')",
                [],
            )
            .unwrap();
        let old = ConsentId(s.conn.last_insert_rowid());
        let far = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64
            + 1000 * DAY;
        s.prune_audit(far).unwrap();
        s.prune_history(vults_core::looks::Date::new(2030, 1, 1)).unwrap();
        s.prune_traces(far * 1000).unwrap();
        s.clear_audit().unwrap();
        s.clear_all(&dir).unwrap();
        assert!(s.valid(old).unwrap(), "a 400-day-old consent still stands");
    }

    #[test]
    fn the_audit_refuses_a_replace() {
        let (mut s, _) = store("audit-replace");
        let a = vults_core::audit::Audit::new(
            vults_core::audit::Actor::Human,
            vults_core::audit::Act::Allow,
            "claude",
            "p",
            "Bash",
            "ls",
        );
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        s.append_audit(now, &a).unwrap();
        let id: i64 = s
            .conn
            .query_row("SELECT id FROM audit", [], |r| r.get(0))
            .unwrap();
        assert!(
            s.conn
                .execute(
                    "INSERT OR REPLACE INTO audit (id, at, actor, act, agent, project, tool, target) VALUES (?1, 0, 'x', 'x', '', '', '', '')",
                    [id],
                )
                .is_err()
        );
    }
}
