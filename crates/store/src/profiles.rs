//! User profile revisions are immutable snapshots, separate from chat access and history retention.
use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};

use crate::{Error, Store};

pub(crate) const TABLE: &str = "
CREATE TABLE agent_profiles (id INTEGER PRIMARY KEY AUTOINCREMENT);
CREATE TABLE agent_profile_revisions (
    profile_id INTEGER NOT NULL REFERENCES agent_profiles(id),
    revision INTEGER NOT NULL CHECK(revision > 0),
    title TEXT NOT NULL, instructions TEXT NOT NULL,
    preset TEXT NOT NULL, preset_revision INTEGER NOT NULL,
    species TEXT NOT NULL, about TEXT NOT NULL,
    PRIMARY KEY(profile_id, revision)
);";

pub(crate) const APPEARANCE: &str =
    "ALTER TABLE agent_profile_revisions ADD COLUMN look TEXT NOT NULL DEFAULT 'none';";

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct Profile {
    pub id: i64,
    pub revision: u32,
    pub title: String,
    pub instructions: String,
    pub preset: String,
    pub preset_revision: u32,
    pub species: String,
    pub look: String,
    pub about: String,
}

fn validate(title: &str, instructions: &str) -> Result<(), Error> {
    if title.trim().is_empty()
        || title.len() > 160
        || title.chars().any(char::is_control)
        || instructions.trim().is_empty()
        || instructions.len() > 32768
        || instructions
            .chars()
            .any(|c| c.is_control() && c != '\n' && c != '\t' && c != '\r')
    {
        return Err(Error::Invalid("Invalid profile text."));
    }
    Ok(())
}

/// The exact identity and content a conversation started with, independent of later edits.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Snapshot {
    pub source: Source,
    pub title: String,
    pub about: String,
    pub instructions: String,
    pub species: String,
    pub look: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum Source {
    Preset { id: String, revision: u32 },
    Custom { id: i64, revision: u32 },
}

impl From<Profile> for Snapshot {
    fn from(profile: Profile) -> Self {
        Self {
            source: Source::Custom {
                id: profile.id,
                revision: profile.revision,
            },
            title: profile.title,
            about: profile.about,
            instructions: profile.instructions,
            species: profile.species,
            look: profile.look,
        }
    }
}

impl Snapshot {
    pub(crate) fn validate(&self, store: &Store) -> Result<(), Error> {
        validate(&self.title, &self.instructions)?;
        appearance(&self.species, &self.look)?;
        if self.about.len() > 1024 {
            return Err(Error::Invalid("Invalid profile snapshot."));
        }
        match &self.source {
            // The native caller resolves the preset revision from its catalog before snapshotting.
            Source::Preset { id, revision }
                if *revision > 0 && matches!(id.as_str(), "developer" | "reviewer" | "researcher") =>
            {
                Ok(())
            }
            Source::Custom { id, revision } => {
                let actual = store.agent_profile_revision(*id, *revision)?.map(Snapshot::from);
                if actual.as_ref() != Some(self) {
                    return Err(Error::Invalid("Profile snapshot does not match its revision."));
                }
                Ok(())
            }
            _ => Err(Error::Invalid("Invalid profile snapshot.")),
        }
    }
}

fn appearance(species: &str, look: &str) -> Result<(), Error> {
    let look =
        serde_json::from_value::<vults_core::looks::Outfit>(serde_json::Value::String(look.to_owned()));
    if (vults_core::flock::chosen(species).is_none() && species != vults_core::flock::KING)
        || !matches!(look, Ok(outfit) if outfit != vults_core::looks::Outfit::Auto)
    {
        return Err(Error::Invalid("Invalid profile appearance."));
    }
    Ok(())
}

fn row(r: &rusqlite::Row<'_>) -> rusqlite::Result<Profile> {
    Ok(Profile {
        id: r.get(0)?,
        revision: r.get(1)?,
        title: r.get(2)?,
        instructions: r.get(3)?,
        preset: r.get(4)?,
        preset_revision: r.get(5)?,
        species: r.get(6)?,
        about: r.get(7)?,
        look: r.get(8)?,
    })
}

impl Store {
    pub fn agent_profiles(&self) -> Result<Vec<Profile>, Error> {
        Ok(self.conn.prepare("SELECT profile_id, revision, title, instructions, preset, preset_revision, species, about, look
            FROM agent_profile_revisions r WHERE revision =
            (SELECT MAX(revision) FROM agent_profile_revisions WHERE profile_id = r.profile_id) ORDER BY profile_id")?
            .query_map([], row)?.collect::<Result<_, _>>()?)
    }

    /// A retained revision, not the latest editable profile.
    pub fn agent_profile_revision(&self, id: i64, revision: u32) -> Result<Option<Profile>, Error> {
        Ok(self.conn.query_row("SELECT profile_id, revision, title, instructions, preset, preset_revision, species, about, look
            FROM agent_profile_revisions WHERE profile_id=?1 AND revision=?2", params![id,revision], row).optional()?)
    }

    /// Expected revision prevents a stale editor from overwriting another saved change.
    pub fn agent_profile_save(&mut self, profile: &Profile) -> Result<Profile, Error> {
        validate(&profile.title, &profile.instructions)?;
        appearance(&profile.species, &profile.look)?;
        let tx = self
            .conn
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let mut saved = if profile.id == 0 {
            let count: i64 = tx.query_row("SELECT COUNT(*) FROM agent_profiles", [], |r| r.get(0))?;
            if count >= 100 {
                return Err(Error::Invalid("Profile limit reached."));
            }
            if !matches!(profile.preset.as_str(), "developer" | "reviewer" | "researcher")
                || profile.preset_revision == 0
                || profile.about.len() > 1024
            {
                return Err(Error::Invalid("Invalid profile source."));
            }
            tx.execute("INSERT INTO agent_profiles DEFAULT VALUES", [])?;
            let mut p = profile.clone();
            p.id = tx.last_insert_rowid();
            p.revision = 1;
            p
        } else {
            let mut old = tx.query_row("SELECT profile_id, revision, title, instructions, preset, preset_revision, species, about, look
                FROM agent_profile_revisions WHERE profile_id = ?1 ORDER BY revision DESC LIMIT 1", [profile.id], row)
                .optional()?.ok_or(Error::Invalid("Profile no longer exists."))?;
            if old.revision != profile.revision {
                return Err(Error::Invalid("Profile changed. Reload before saving."));
            }
            old.revision = old
                .revision
                .checked_add(1)
                .ok_or(Error::Invalid("Profile revision limit reached."))?;
            old.species.clone_from(&profile.species);
            old.look.clone_from(&profile.look);
            old.title.clone_from(&profile.title);
            old.instructions.clone_from(&profile.instructions);
            old
        };
        saved.title = saved.title.trim().to_owned();
        tx.execute("INSERT INTO agent_profile_revisions
            (profile_id, revision, title, instructions, preset, preset_revision, species, about, look) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
            params![saved.id, saved.revision, saved.title, saved.instructions, saved.preset, saved.preset_revision, saved.species, saved.about, saved.look])?;
        tx.commit()?;
        Ok(saved)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn copies_and_revisions_survive_reopen_and_reject_stale_or_invalid_edits() {
        let dir = std::env::temp_dir().join(format!("vults-profiles-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut s = Store::open(&dir).unwrap();
        let base = Profile {
            id: 0,
            revision: 0,
            title: "My developer".into(),
            instructions: "Keep\nthis literal.".into(),
            preset: "developer".into(),
            preset_revision: 1,
            species: "atratus".into(),
            look: "none".into(),
            about: "Original description".into(),
        };
        let mut copy = s.agent_profile_save(&base).unwrap();
        assert_eq!(copy.revision, 1);
        copy.instructions = "Edited instructions".into();
        copy.species = "forged".into();
        assert!(s.agent_profile_save(&copy).is_err());
        copy.species = "vultur".into();
        copy.look = "auto".into();
        assert!(s.agent_profile_save(&copy).is_err());
        copy.look = "crown".into();
        let changed = s.agent_profile_save(&copy).unwrap();
        assert_eq!(changed.revision, 2);
        assert_eq!(changed.species, "vultur");
        assert_eq!(changed.look, "crown");
        assert_eq!(s.conn.query_row("SELECT species || '/' || look FROM agent_profile_revisions WHERE profile_id=?1 AND revision=1",[changed.id],|r|r.get::<_,String>(0)).unwrap(),"atratus/none");
        assert!(s.agent_profile_save(&copy).is_err());
        copy.title = " ".into();
        assert!(s.agent_profile_save(&copy).is_err());
        copy.title = "Name".into();
        copy.instructions = "\0".into();
        assert!(s.agent_profile_save(&copy).is_err());
        assert_eq!(
            s.conn
                .query_row(
                    "SELECT instructions FROM agent_profile_revisions WHERE profile_id=?1 AND revision=1",
                    [changed.id],
                    |r| r.get::<_, String>(0)
                )
                .unwrap(),
            base.instructions
        );
        let mut oversized = base.clone();
        oversized.instructions = "x".repeat(32769);
        assert!(s.agent_profile_save(&oversized).is_err());
        s.clear_all(&dir).unwrap();
        drop(s);
        assert_eq!(
            Store::open(&dir).unwrap().agent_profiles().unwrap(),
            vec![changed]
        );
        std::fs::remove_dir_all(dir).unwrap();
    }
}

#[cfg(test)]
mod migration_tests {
    use super::*;
    #[test]
    fn older_profile_revisions_keep_their_species_and_gain_no_accessory() {
        let dir = std::env::temp_dir().join(format!("vults-profile-appearance-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let s = Store::connect(&dir).unwrap();
        for sql in &crate::MIGRATIONS[..29] {
            s.conn.execute_batch(sql).unwrap();
        }
        s.conn.pragma_update(None, "user_version", 29).unwrap();
        s.conn.execute_batch("INSERT INTO agent_profiles(id) VALUES(1);
            INSERT INTO agent_profile_revisions VALUES(1,1,'My reviewer','Review','reviewer',1,'gypaetus','Description');").unwrap();
        drop(s);
        let s = Store::open(&dir).unwrap();
        let profiles = s.agent_profiles().unwrap();
        assert_eq!(profiles[0].species, "gypaetus");
        assert_eq!(profiles[0].look, "none");
        assert_eq!(profiles[0].revision, 1);
        drop(s);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
