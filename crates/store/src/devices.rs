//! Paired phones (ADR 0017, `docs/dev/plan-zeca.md` L1): their public keys and the highest counter
//! each has signed with. The ledger keeps the counters in memory; here they outlive a restart, so
//! an answer replayed after one is still refused. A device is never deleted, only revoked once: an
//! id that comes back must pair again under a new one. Keys never change.

use rusqlite::{OptionalExtension, params};

use crate::{Error, Store};

/// Migration 6. The counter is a `u64`, which SQLite's integers cannot hold, so it is kept as 8
/// big-endian bytes: blobs compare byte by byte, which for a fixed width is numeric order, and the
/// guard can refuse one that goes down. Shipped: never edit it.
pub(crate) const TABLE: &str = "
    CREATE TABLE devices (
        id TEXT PRIMARY KEY NOT NULL,
        name TEXT NOT NULL,
        seal_key BLOB NOT NULL,
        sign_key BLOB NOT NULL,
        bio_key BLOB NOT NULL,
        paired_at INTEGER NOT NULL,
        revoked_at INTEGER,
        counter BLOB NOT NULL DEFAULT x'0000000000000000' CHECK (typeof(counter) = 'blob' AND length(counter) = 8)
    );
    CREATE TRIGGER devices_guarded BEFORE UPDATE ON devices
    WHEN new.id IS NOT old.id OR new.seal_key IS NOT old.seal_key
        OR new.sign_key IS NOT old.sign_key OR new.bio_key IS NOT old.bio_key
        OR new.paired_at IS NOT old.paired_at
        OR (old.revoked_at IS NOT NULL AND new.revoked_at IS NOT old.revoked_at)
        OR typeof(new.counter) <> 'blob' OR new.counter < old.counter
    BEGIN
        SELECT RAISE(ABORT, 'a device only gets a new name, a higher counter, or revoked once');
    END;
    CREATE TRIGGER devices_never_deleted BEFORE DELETE ON devices
    BEGIN
        SELECT RAISE(ABORT, 'a device is never deleted: revoke it');
    END;";

/// A paired device as read back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Device {
    pub id: String,
    /// What the user called it when pairing.
    pub name: String,
    /// Its X25519 public key: what the envelopes to and from it are sealed with.
    pub seal_key: Vec<u8>,
    /// Its signing key for what needs no biometrics (deny, dismiss).
    pub sign_key: Vec<u8>,
    /// Its signing key that unlocks only with the user's biometrics, for each use.
    pub bio_key: Vec<u8>,
    /// Unix seconds.
    pub paired_at: i64,
    pub revoked_at: Option<i64>,
    /// The highest counter it has signed with.
    pub counter: u64,
}

impl Store {
    /// A device the user just paired. An id already known (even revoked) is refused.
    pub fn pair_device(&mut self, d: &Device) -> Result<(), Error> {
        self.conn.execute(
            "INSERT INTO devices (id, name, seal_key, sign_key, bio_key, paired_at, counter)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                d.id,
                d.name,
                d.seal_key,
                d.sign_key,
                d.bio_key,
                d.paired_at,
                d.counter.to_be_bytes()
            ],
        )?;
        Ok(())
    }

    /// Every device ever paired, revoked ones too, oldest first.
    pub fn devices(&self) -> Result<Vec<Device>, Error> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, seal_key, sign_key, bio_key, paired_at, revoked_at, counter
             FROM devices ORDER BY paired_at, id",
        )?;
        let rows = stmt.query_map([], |r| {
            let counter: Vec<u8> = r.get(7)?;
            Ok(Device {
                id: r.get(0)?,
                name: r.get(1)?,
                seal_key: r.get(2)?,
                sign_key: r.get(3)?,
                bio_key: r.get(4)?,
                paired_at: r.get(5)?,
                revoked_at: r.get(6)?,
                counter: counter.try_into().map_or(u64::MAX, u64::from_be_bytes),
            })
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    /// Settings → Phone → Revoke: from now on nothing it signs is taken. Once; a second time keeps
    /// the first date.
    pub fn revoke_device(&mut self, id: &str, at: i64) -> Result<(), Error> {
        self.conn.execute(
            "UPDATE devices SET revoked_at = ?2 WHERE id = ?1 AND revoked_at IS NULL",
            params![id, at],
        )?;
        Ok(())
    }

    /// The device used `counter`: saved if it is higher than the one kept, ignored otherwise.
    pub fn raise_counter(&mut self, id: &str, counter: u64) -> Result<(), Error> {
        self.conn.execute(
            "UPDATE devices SET counter = ?2 WHERE id = ?1 AND counter < ?2",
            params![id, counter.to_be_bytes()],
        )?;
        Ok(())
    }

    /// Whether `id` is paired and not revoked.
    pub fn device_valid(&self, id: &str) -> Result<bool, Error> {
        Ok(self
            .conn
            .query_row(
                "SELECT revoked_at IS NULL FROM devices WHERE id = ?1",
                params![id],
                |r| r.get(0),
            )
            .optional()?
            .unwrap_or(false))
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use crate::temp;

    fn store(name: &str) -> (Store, std::path::PathBuf) {
        let dir = temp(name);
        (Store::open(&dir).unwrap(), dir)
    }

    fn phone(id: &str) -> Device {
        Device {
            id: id.into(),
            name: "Pixel".into(),
            seal_key: vec![1; 32],
            sign_key: vec![2; 65],
            bio_key: vec![3; 65],
            paired_at: 100,
            revoked_at: None,
            counter: 0,
        }
    }

    #[test]
    fn a_counter_only_goes_up_and_survives() {
        let (mut s, dir) = store("devices-counter");
        s.pair_device(&phone("p1")).unwrap();
        s.raise_counter("p1", 7).unwrap();
        s.raise_counter("p1", 3).unwrap();
        drop(s);
        let mut s = Store::open(&dir).unwrap();
        assert_eq!(s.devices().unwrap()[0].counter, 7);
        // Past what an SQLite integer holds: still in order.
        s.raise_counter("p1", u64::MAX - 1).unwrap();
        s.raise_counter("p1", 1 << 63).unwrap();
        assert_eq!(s.devices().unwrap()[0].counter, u64::MAX - 1);
        assert!(
            s.conn
                .execute("UPDATE devices SET counter = x'0000000000000001'", [])
                .is_err()
        );
    }

    #[test]
    fn a_device_is_revoked_once_and_never_deleted() {
        let (mut s, _) = store("devices-revoke");
        s.pair_device(&phone("p1")).unwrap();
        assert!(s.device_valid("p1").unwrap());
        assert!(!s.device_valid("nope").unwrap());
        s.revoke_device("p1", 200).unwrap();
        s.revoke_device("p1", 300).unwrap();
        assert!(!s.device_valid("p1").unwrap());
        assert_eq!(s.devices().unwrap()[0].revoked_at, Some(200));
        assert!(
            s.conn
                .execute("UPDATE devices SET revoked_at = NULL", [])
                .is_err()
        );
        assert!(s.conn.execute("DELETE FROM devices", []).is_err());
        // The same id cannot pair again, even revoked.
        assert!(s.pair_device(&phone("p1")).is_err());
    }

    #[test]
    fn keys_never_change() {
        let (mut s, _) = store("devices-keys");
        s.pair_device(&phone("p1")).unwrap();
        assert!(s.conn.execute("UPDATE devices SET bio_key = x'00'", []).is_err());
        assert!(s.conn.execute("UPDATE devices SET sign_key = x'00'", []).is_err());
        assert!(s.conn.execute("UPDATE devices SET seal_key = x'00'", []).is_err());
        assert!(
            s.conn
                .execute("UPDATE devices SET name = 'Old phone'", [])
                .is_ok()
        );
    }
}
