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

pub(crate) const SKILLS: &str =
    "ALTER TABLE agent_profile_revisions ADD COLUMN skills TEXT NOT NULL DEFAULT '[]';";

pub(crate) const STATE: &str = "
CREATE TABLE agent_profile_state (
    profile_key TEXT PRIMARY KEY NOT NULL,
    version INTEGER NOT NULL CHECK(version > 0),
    defaults TEXT NOT NULL,
    archived INTEGER NOT NULL CHECK(archived IN (0,1))
);";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "executor", rename_all = "lowercase", deny_unknown_fields)]
pub enum ExecutionDefaults {
    Claude {
        model: Option<String>,
        effort: Option<String>,
    },
    Codex {
        model: Option<String>,
        effort: Option<String>,
    },
    Api {
        provider: String,
        model: Option<String>,
    },
}
impl Default for ExecutionDefaults {
    fn default() -> Self {
        Self::Claude {
            model: None,
            effort: None,
        }
    }
}
impl ExecutionDefaults {
    pub fn validate(&self) -> Result<(), Error> {
        let (model, effort) = match self {
            Self::Claude { model, effort } | Self::Codex { model, effort } => (model, effort.as_deref()),
            Self::Api { provider, model } => {
                if provider.is_empty()
                    || provider.len() > 64
                    || !provider
                        .bytes()
                        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-')
                {
                    return Err(Error::Invalid("Invalid profile defaults."));
                }
                (model, None)
            }
        };
        if model
            .as_ref()
            .is_some_and(|m| m.is_empty() || m.len() > 160 || m.chars().any(char::is_control))
            || effort.is_some_and(|e| !matches!(e, "low" | "medium" | "high" | "xhigh" | "max"))
        {
            return Err(Error::Invalid("Invalid profile defaults."));
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProfileState {
    pub version: u32,
    pub defaults: ExecutionDefaults,
    pub archived: bool,
}
#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "action", rename_all = "camelCase", deny_unknown_fields)]
pub enum StateChange {
    Defaults { defaults: ExecutionDefaults },
    Archive { archived: bool },
}

fn profile_key(conn: &rusqlite::Connection, key: &str) -> Result<bool, Error> {
    if matches!(key, "developer" | "reviewer" | "researcher") {
        return Ok(false);
    }
    if let Some(id) = key
        .strip_prefix("custom-")
        .and_then(|id| id.parse::<i64>().ok())
        .filter(|id| *id > 0 && format!("custom-{id}") == key)
        && conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM agent_profiles WHERE id=?1)",
            [id],
            |r| r.get::<_, bool>(0),
        )?
    {
        return Ok(true);
    }
    Err(Error::Invalid("Profile no longer exists."))
}
fn profile_state(conn: &rusqlite::Connection, key: &str) -> Result<ProfileState, Error> {
    profile_key(conn, key)?;
    Ok(conn
        .query_row(
            "SELECT version,defaults,archived FROM agent_profile_state WHERE profile_key=?1",
            [key],
            |r| {
                let defaults: ExecutionDefaults = serde_json::from_str(&r.get::<_, String>(1)?)
                    .map_err(|_| rusqlite::Error::InvalidQuery)?;
                defaults.validate().map_err(|_| rusqlite::Error::InvalidQuery)?;
                Ok(ProfileState {
                    version: r.get(0)?,
                    defaults,
                    archived: r.get(2)?,
                })
            },
        )
        .optional()?
        .unwrap_or_default())
}
fn write_profile_state(
    conn: &rusqlite::Connection,
    key: &str,
    mut state: ProfileState,
) -> Result<ProfileState, Error> {
    state.defaults.validate()?;
    state.version = state
        .version
        .checked_add(1)
        .ok_or(Error::Invalid("Profile state revision limit reached."))?;
    conn.execute("INSERT INTO agent_profile_state(profile_key,version,defaults,archived) VALUES(?1,?2,?3,?4)
        ON CONFLICT(profile_key) DO UPDATE SET version=excluded.version,defaults=excluded.defaults,archived=excluded.archived",params![key,state.version,serde_json::to_string(&state.defaults).map_err(|_|Error::Invalid("Invalid profile defaults."))?,state.archived])?;
    Ok(state)
}
impl Store {
    pub fn agent_profile_state(&self, key: &str) -> Result<ProfileState, Error> {
        profile_state(&self.conn, key)
    }
    pub fn agent_profile_state_set(
        &mut self,
        key: &str,
        version: u32,
        change: &StateChange,
    ) -> Result<ProfileState, Error> {
        let tx = self
            .conn
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let custom = profile_key(&tx, key)?;
        let mut state = profile_state(&tx, key)?;
        if state.version != version {
            return Err(Error::Invalid("Profile changed. Reload before saving."));
        }
        match change {
            StateChange::Defaults { defaults } => {
                if state.archived {
                    return Err(Error::Invalid("Profile is archived."));
                }
                state.defaults = defaults.clone();
            }
            StateChange::Archive { archived } => {
                if !custom {
                    return Err(Error::Invalid("Built-in profiles cannot be archived."));
                }
                state.archived = *archived;
            }
        }
        let state = write_profile_state(&tx, key, state)?;
        tx.commit()?;
        Ok(state)
    }
}

/// Skill content is copied into the profile revision so later catalog edits cannot change it.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SkillSnapshot {
    pub id: String,
    pub revision: u32,
    pub title: String,
    pub content: String,
}

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
    pub skills: Vec<SkillSnapshot>,
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
    #[serde(default)]
    pub skills: Vec<SkillSnapshot>,
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
            skills: profile.skills,
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
        validate_skills(&self.skills)?;
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

fn validate_skills(skills: &[SkillSnapshot]) -> Result<(), Error> {
    if skills.len() > 8 {
        return Err(Error::Invalid("Invalid profile skills."));
    }
    let mut ids = std::collections::HashSet::new();
    for skill in skills {
        if skill.id.is_empty()
            || skill.id.len() > 64
            || !skill
                .id
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
            || skill.revision == 0
            || skill.title.trim().is_empty()
            || skill.title.len() > 120
            || skill.content.trim().is_empty()
            || skill.content.len() > 8192
            || skill
                .content
                .chars()
                .any(|c| c.is_control() && c != '\n' && c != '\t' && c != '\r')
            || !ids.insert(skill.id.as_str())
        {
            return Err(Error::Invalid("Invalid profile skills."));
        }
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
        skills: serde_json::from_str(&r.get::<_, String>(9)?).map_err(|_| rusqlite::Error::InvalidQuery)?,
    })
}

impl Store {
    pub fn agent_profiles(&self) -> Result<Vec<Profile>, Error> {
        Ok(self.conn.prepare("SELECT profile_id, revision, title, instructions, preset, preset_revision, species, about, look, skills
            FROM agent_profile_revisions r WHERE revision =
            (SELECT MAX(revision) FROM agent_profile_revisions WHERE profile_id = r.profile_id) ORDER BY profile_id")?
            .query_map([], row)?.collect::<Result<_, _>>()?)
    }

    /// A retained revision, not the latest editable profile.
    pub fn agent_profile_revision(&self, id: i64, revision: u32) -> Result<Option<Profile>, Error> {
        Ok(self.conn.query_row("SELECT profile_id, revision, title, instructions, preset, preset_revision, species, about, look, skills
            FROM agent_profile_revisions WHERE profile_id=?1 AND revision=?2", params![id,revision], row).optional()?)
    }

    /// Expected revision prevents a stale editor from overwriting another saved change.
    pub fn agent_profile_save(&mut self, profile: &Profile) -> Result<Profile, Error> {
        self.save_profile(profile, None)
    }
    pub fn agent_profile_save_with_defaults(
        &mut self,
        profile: &Profile,
        version: u32,
        defaults: &ExecutionDefaults,
    ) -> Result<Profile, Error> {
        defaults.validate()?;
        self.save_profile(profile, Some((version, defaults)))
    }
    fn save_profile(
        &mut self,
        profile: &Profile,
        preferences: Option<(u32, &ExecutionDefaults)>,
    ) -> Result<Profile, Error> {
        validate(&profile.title, &profile.instructions)?;
        appearance(&profile.species, &profile.look)?;
        validate_skills(&profile.skills)?;
        let tx = self
            .conn
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        if profile.id != 0 {
            let state = profile_state(&tx, &format!("custom-{}", profile.id))?;
            if state.archived {
                return Err(Error::Invalid("Profile is archived."));
            }
            if preferences.is_some_and(|(version, _)| version != state.version) {
                return Err(Error::Invalid("Profile changed. Reload before saving."));
            }
        } else if preferences.is_some_and(|(version, _)| version != 0) {
            return Err(Error::Invalid("Profile changed. Reload before saving."));
        }
        let mut saved = if profile.id == 0 {
            let count: i64 = tx.query_row("SELECT COUNT(*) FROM agent_profiles", [], |r| r.get(0))?;
            if count >= 100 {
                return Err(Error::Invalid("Profile limit reached."));
            }
            let blank = profile.preset.is_empty() && profile.preset_revision == 0;
            let copied = matches!(profile.preset.as_str(), "developer" | "reviewer" | "researcher")
                && profile.preset_revision > 0;
            if (!blank && !copied) || profile.about.len() > 1024 {
                return Err(Error::Invalid("Invalid profile source."));
            }
            tx.execute("INSERT INTO agent_profiles DEFAULT VALUES", [])?;
            let mut p = profile.clone();
            p.id = tx.last_insert_rowid();
            p.revision = 1;
            p
        } else {
            let mut old = tx.query_row("SELECT profile_id, revision, title, instructions, preset, preset_revision, species, about, look, skills
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
            old.skills.clone_from(&profile.skills);
            old
        };
        saved.title = saved.title.trim().to_owned();
        tx.execute("INSERT INTO agent_profile_revisions
            (profile_id, revision, title, instructions, preset, preset_revision, species, about, look, skills) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
            params![saved.id, saved.revision, saved.title, saved.instructions, saved.preset, saved.preset_revision, saved.species, saved.about, saved.look, serde_json::to_string(&saved.skills).map_err(|_| Error::Invalid("Invalid profile skills."))?])?;
        if let Some((_, defaults)) = preferences {
            let key = format!("custom-{}", saved.id);
            let mut state = profile_state(&tx, &key)?;
            state.defaults = defaults.clone();
            write_profile_state(&tx, &key, state)?;
        }
        tx.commit()?;
        Ok(saved)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_profiles_keep_no_preset_and_retain_their_own_revisions() {
        let dir = std::env::temp_dir().join(format!("vults-original-profiles-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut store = Store::open(&dir).unwrap();
        let mut original = Profile {
            id: 0,
            revision: 0,
            title: "Release specialist".into(),
            instructions: "Review release readiness.".into(),
            preset: String::new(),
            preset_revision: 0,
            species: "vultur".into(),
            look: "crown".into(),
            about: "Custom agent profile".into(),
            skills: vec![],
        };
        original.preset_revision = 1;
        assert!(store.agent_profile_save(&original).is_err());
        original.preset_revision = 0;
        original.preset = "forged".into();
        assert!(store.agent_profile_save(&original).is_err());
        original.preset.clear();
        let first = store.agent_profile_save(&original).unwrap();
        assert!(first.preset.is_empty());
        assert_eq!(first.preset_revision, 0);
        assert_eq!(first.revision, 1);
        Snapshot::from(first.clone()).validate(&store).unwrap();
        let mut edit = first.clone();
        edit.instructions = "Updated instructions".into();
        let second = store.agent_profile_save(&edit).unwrap();
        assert_eq!(second.revision, 2);
        assert!(second.preset.is_empty());
        drop(store);
        let reopened = Store::open(&dir).unwrap();
        assert_eq!(reopened.agent_profiles().unwrap(), vec![second]);
        assert_eq!(reopened.agent_profile_revision(first.id, 1).unwrap(), Some(first));
        drop(reopened);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn pre_skill_conversation_snapshots_default_to_no_skills() {
        let snapshot: Snapshot = serde_json::from_value(serde_json::json!({
            "source": { "kind": "preset", "id": "developer", "revision": 1 },
            "title": "Developer", "about": "Description", "instructions": "Do the work",
            "species": "atratus", "look": "none"
        }))
        .unwrap();
        assert!(snapshot.skills.is_empty());
    }

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
            skills: vec![SkillSnapshot {
                id: "verify-change".into(),
                revision: 1,
                title: "Verify the change".into(),
                content: "Run focused checks".into(),
            }],
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
        assert_eq!(changed.skills, base.skills);
        assert_eq!(s.conn.query_row("SELECT species || '/' || look FROM agent_profile_revisions WHERE profile_id=?1 AND revision=1",[changed.id],|r|r.get::<_,String>(0)).unwrap(),"atratus/none");
        assert!(s.agent_profile_save(&copy).is_err());
        copy.title = " ".into();
        assert!(s.agent_profile_save(&copy).is_err());
        copy.title = "Name".into();
        copy.instructions = "\0".into();
        assert!(s.agent_profile_save(&copy).is_err());
        copy.instructions = "Valid instructions".into();
        let duplicate = copy.skills[0].clone();
        copy.skills.push(duplicate);
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
            INSERT INTO agent_profile_revisions(profile_id,revision,title,instructions,preset,preset_revision,species,about)
            VALUES(1,1,'My reviewer','Review','reviewer',1,'gypaetus','Description');").unwrap();
        drop(s);
        let s = Store::open(&dir).unwrap();
        let profiles = s.agent_profiles().unwrap();
        assert_eq!(profiles[0].species, "gypaetus");
        assert_eq!(profiles[0].look, "none");
        assert_eq!(profiles[0].revision, 1);
        assert!(profiles[0].skills.is_empty());
        drop(s);
        std::fs::remove_dir_all(dir).unwrap();
    }
}

#[cfg(test)]
mod state_tests {
    use super::*;
    fn own() -> Profile {
        Profile {
            id: 0,
            revision: 0,
            title: "Agent".into(),
            instructions: "Keep scope explicit.".into(),
            preset: String::new(),
            preset_revision: 0,
            species: "atratus".into(),
            look: "none".into(),
            about: "Custom agent profile".into(),
            skills: vec![],
        }
    }
    #[test]
    fn defaults_archive_and_restore_preserve_revisions_and_survive_restart() {
        let dir = crate::temp("profile-state");
        let mut s = Store::open(&dir).unwrap();
        let original = s.agent_profile_save(&own()).unwrap();
        let key = format!("custom-{}", original.id);
        assert_eq!(s.agent_profile_state(&key).unwrap(), ProfileState::default());
        let defaults = ExecutionDefaults::Codex {
            model: Some("chosen-model".into()),
            effort: Some("high".into()),
        };
        let state = s
            .agent_profile_state_set(
                &key,
                0,
                &StateChange::Defaults {
                    defaults: defaults.clone(),
                },
            )
            .unwrap();
        assert_eq!(state.version, 1);
        let archived = s
            .agent_profile_state_set(&key, 1, &StateChange::Archive { archived: true })
            .unwrap();
        assert!(archived.archived);
        assert!(
            s.agent_profile_state_set(&key, 1, &StateChange::Archive { archived: false })
                .is_err()
        );
        assert!(
            s.agent_profile_save_with_defaults(&original, 2, &ExecutionDefaults::default())
                .is_err()
        );
        assert!(s.agent_profile_save(&original).is_err());
        assert!(
            s.agent_profile_state_set(
                &key,
                2,
                &StateChange::Defaults {
                    defaults: ExecutionDefaults::default()
                }
            )
            .is_err()
        );
        let snapshot = Snapshot::from(original.clone());
        snapshot.validate(&s).unwrap();
        s.clear_all(&dir).unwrap();
        drop(s);
        let mut s = Store::open(&dir).unwrap();
        assert_eq!(s.agent_profile_state(&key).unwrap(), archived);
        assert_eq!(s.agent_profiles().unwrap(), vec![original.clone()]);
        let restored = s
            .agent_profile_state_set(&key, 2, &StateChange::Archive { archived: false })
            .unwrap();
        assert_eq!(restored.defaults, defaults);
        assert_eq!(restored.version, 3);
        assert_eq!(s.agent_profile_revision(original.id, 1).unwrap(), Some(original));
    }
    #[test]
    fn profile_and_defaults_are_atomic_and_builtin_content_never_changes() {
        let mut s = Store::open(&crate::temp("profile-state-atomic")).unwrap();
        let a = s
            .agent_profile_save_with_defaults(&own(), 0, &ExecutionDefaults::default())
            .unwrap();
        let key = format!("custom-{}", a.id);
        let mut edit = a.clone();
        edit.title = "Changed".into();
        assert!(
            s.agent_profile_save_with_defaults(&edit, 0, &ExecutionDefaults::default())
                .is_err()
        );
        assert_eq!(s.agent_profiles().unwrap(), vec![a.clone()]);
        s.conn
            .execute(
                "UPDATE agent_profile_state SET version=?1 WHERE profile_key=?2",
                params![u32::MAX, key],
            )
            .unwrap();
        assert!(
            s.agent_profile_save_with_defaults(&edit, u32::MAX, &ExecutionDefaults::default())
                .is_err()
        );
        assert_eq!(s.agent_profiles().unwrap(), vec![a]);
        let defaults = ExecutionDefaults::Api {
            provider: "ollama".into(),
            model: Some("local-model".into()),
        };
        s.agent_profile_state_set("developer", 0, &StateChange::Defaults { defaults })
            .unwrap();
        assert!(
            s.agent_profile_state_set("developer", 1, &StateChange::Archive { archived: true })
                .is_err()
        );
        for key in ["custom-0", "custom-01", "custom-999", "forged"] {
            assert!(s.agent_profile_state(key).is_err());
        }
        assert!(
            serde_json::from_value::<ExecutionDefaults>(
                serde_json::json!({"executor":"api","provider":"openai","model":null,"effort":"high"})
            )
            .is_err()
        );
    }
    #[test]
    fn migration_adds_no_default_authority_and_keeps_existing_profiles() {
        let dir = crate::temp("profile-state-migration");
        let s = Store::connect(&dir).unwrap();
        for sql in &crate::MIGRATIONS[..32] {
            s.conn.execute_batch(sql).unwrap();
        }
        s.conn.pragma_update(None, "user_version", 32).unwrap();
        s.conn.execute_batch("INSERT INTO agent_profiles(id) VALUES(1); INSERT INTO agent_profile_revisions(profile_id,revision,title,instructions,preset,preset_revision,species,about,look,skills) VALUES(1,1,'Original','Keep scope explicit.','',0,'atratus','Custom agent profile','none','[]');").unwrap();
        drop(s);
        let s = Store::open(&dir).unwrap();
        assert_eq!(
            s.agent_profile_state("custom-1").unwrap(),
            ProfileState::default()
        );
        assert_eq!(s.agent_profiles().unwrap()[0].title, "Original");
    }
}
