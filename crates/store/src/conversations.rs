//! Zeca's conversations (`docs/dev/plan-zeca.md` Z4): each one's engine, folder, title and what
//! resumes it (the CLI's session or thread, or the API's history), and what was said, so a
//! conversation goes on after the app restarts. The CLIs keep their own copy of the context; this
//! is what the chat shows. Kept until the user deletes them (one, or all with Clear history), the
//! newest [`KEEP`] at most.

use rusqlite::{OptionalExtension, params};

use crate::{Error, Store, profiles::Snapshot};

/// A pinned profile is never rewritten, including before a first reply.
pub(crate) const PROFILE: &str = "ALTER TABLE conversations ADD COLUMN profile TEXT;
CREATE TRIGGER conversation_profile_fixed BEFORE UPDATE OF profile ON conversations
WHEN OLD.profile IS NOT NEW.profile BEGIN SELECT RAISE(ABORT, 'Conversation profile is immutable.'); END;";

/// Migration 7: the tool that first brought outside content into a conversation (Z8), so it
/// still warns when opened again. Shipped: never edit it.
pub(crate) const OUTSIDE: &str = "ALTER TABLE conversations ADD COLUMN outside TEXT;";

/// Migration 8: the frozen context a conversation began with (M2), so its system prompt is the
/// same bytes after a restart. Shipped: never edit it.
pub(crate) const FROZEN: &str = "ALTER TABLE conversations ADD COLUMN frozen TEXT;";

/// Conversations kept; a new one past this drops the one left alone longest.
pub const KEEP: i64 = 200;
/// Longest title kept, in characters.
pub const MAX_TITLE: usize = 80;

/// Migration 5. Ids never come back (`AUTOINCREMENT`): a late line for a deleted conversation must
/// never land in a newer one. Shipped: never edit it.
pub(crate) const TABLE: &str = "
    CREATE TABLE conversations (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        engine TEXT NOT NULL,
        folder TEXT NOT NULL,
        title TEXT NOT NULL,
        session TEXT,
        model TEXT,
        effort TEXT,
        settings TEXT NOT NULL,
        history TEXT,
        created_ms INTEGER NOT NULL,
        updated_ms INTEGER NOT NULL
    );
    CREATE INDEX conversations_updated ON conversations (updated_ms);
    CREATE TABLE chat_messages (
        id INTEGER PRIMARY KEY,
        conversation INTEGER NOT NULL,
        who TEXT NOT NULL,
        text TEXT NOT NULL,
        files TEXT NOT NULL,
        ms INTEGER NOT NULL
    );
    CREATE INDEX chat_messages_conversation ON chat_messages (conversation, id);";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ConversationId(pub i64);

/// A conversation as it starts.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct New {
    /// `claude`, `codex`, or `api:<provider>`.
    pub engine: String,
    /// The folder it works in; `""` for the chat's own.
    pub folder: String,
    pub title: String,
    pub model: Option<String>,
    pub effort: Option<String>,
    /// Which of the user's settings its CLI reads: `isolated`, `user` or `project`.
    pub settings: String,
    /// What follows the persona in its system prompt (M2): the memory as it began.
    pub frozen: Option<String>,
    /// None for Zeca; a complete retained identity for a specialist.
    pub profile: Option<Snapshot>,
}

/// A conversation as read back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Conversation {
    pub id: ConversationId,
    pub engine: String,
    pub folder: String,
    pub title: String,
    /// Claude Code's session id or Codex's thread id; none before the first reply.
    pub session: Option<String>,
    pub model: Option<String>,
    pub effort: Option<String>,
    pub settings: String,
    /// The API chat's wire history, as JSON.
    pub history: Option<String>,
    /// The tool that first brought outside content in, if one did (Z8).
    pub outside: Option<String>,
    /// What follows the persona in its system prompt (M2).
    pub frozen: Option<String>,
    /// None for Zeca; a complete retained identity for a specialist.
    pub profile: Option<Snapshot>,
    /// Unix milliseconds.
    pub created_ms: i64,
    pub updated_ms: i64,
}

/// One line of a conversation: `you`, `zeca`, `error` or `stopped`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    pub who: String,
    pub text: String,
    /// Names of the files sent with it.
    pub files: Vec<String>,
    pub ms: i64,
}

fn title(text: &str) -> String {
    let line = text
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .unwrap_or_default();
    match line.char_indices().nth(MAX_TITLE) {
        Some((end, _)) => format!("{}…", &line[..end]),
        None => line.to_string(),
    }
}

fn conversation(r: &rusqlite::Row<'_>) -> rusqlite::Result<Conversation> {
    Ok(Conversation {
        id: ConversationId(r.get(0)?),
        engine: r.get(1)?,
        folder: r.get(2)?,
        title: r.get(3)?,
        session: r.get(4)?,
        model: r.get(5)?,
        effort: r.get(6)?,
        settings: r.get(7)?,
        history: r.get(8)?,
        created_ms: r.get(9)?,
        updated_ms: r.get(10)?,
        outside: r.get(11)?,
        frozen: r.get(12)?,
        profile: r
            .get::<_, Option<String>>(13)?
            .map(|json| {
                serde_json::from_str(&json).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(13, rusqlite::types::Type::Text, Box::new(e))
                })
            })
            .transpose()?,
    })
}

const COLUMNS: &str = "id, engine, folder, title, session, model, effort, settings, history, created_ms, updated_ms, outside, frozen, profile";

impl Store {
    /// Starts a conversation; its title is the first line of `new.title`, cut. Past [`KEEP`], the
    /// one left alone longest goes, with what was said in it.
    pub fn conversation_new(&mut self, new: &New, now_ms: i64) -> Result<ConversationId, Error> {
        if let Some(profile) = &new.profile {
            profile.validate(self)?;
            if new.frozen.is_some() {
                return Err(Error::Invalid(
                    "Specialist conversations cannot use Zeca's frozen context.",
                ));
            }
        }
        let profile = new
            .profile
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .map_err(|_| Error::Invalid("Invalid profile snapshot."))?;
        let tx = self.conn.transaction()?;
        tx.execute(
            "INSERT INTO conversations (engine, folder, title, model, effort, settings, created_ms, updated_ms, frozen, profile)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7, ?8, ?9)",
            params![new.engine, new.folder, title(&new.title), new.model, new.effort, new.settings, now_ms, new.frozen, profile],
        )?;
        let id = tx.last_insert_rowid();
        tx.execute(
            "DELETE FROM chat_messages WHERE conversation IN (
                SELECT id FROM conversations ORDER BY updated_ms DESC, id DESC LIMIT -1 OFFSET ?1)",
            [KEEP],
        )?;
        tx.execute(
            "DELETE FROM conversations WHERE id IN (
                SELECT id FROM conversations ORDER BY updated_ms DESC, id DESC LIMIT -1 OFFSET ?1)",
            [KEEP],
        )?;
        tx.commit()?;
        Ok(ConversationId(id))
    }

    /// A conversation that never got a reply moved (another folder, model or provider before its
    /// first answer): its record follows, its title stays.
    pub fn conversation_restart(&mut self, id: ConversationId, new: &New) -> Result<(), Error> {
        if let Some(old) = self.conversation(id)?
            && (old.profile != new.profile || (old.profile.is_some() && new.frozen.is_some()))
        {
            return Err(Error::Invalid(
                "A conversation's profile and context owner cannot be changed.",
            ));
        }
        self.conn.execute(
            "UPDATE conversations SET engine = ?2, folder = ?3, model = ?4, effort = ?5, settings = ?6, frozen = ?7
             WHERE id = ?1 AND session IS NULL AND history IS NULL",
            params![id.0, new.engine, new.folder, new.model, new.effort, new.settings, new.frozen],
        )?;
        Ok(())
    }

    /// What resumes it, after a turn: the CLI's session, or the API's history; and the tool that
    /// brought outside content in, once one did (never cleared).
    pub fn conversation_save(
        &mut self,
        id: ConversationId,
        session: Option<&str>,
        history: Option<&str>,
        outside: Option<&str>,
        now_ms: i64,
    ) -> Result<(), Error> {
        self.conn.execute(
            "UPDATE conversations SET session = COALESCE(?2, session), history = COALESCE(?3, history),
             outside = COALESCE(outside, ?4), updated_ms = ?5 WHERE id = ?1",
            params![id.0, session, history, outside, now_ms],
        )?;
        Ok(())
    }

    /// A line said in it. Nothing is added to a conversation that is gone.
    pub fn conversation_say(
        &mut self,
        id: ConversationId,
        who: &str,
        text: &str,
        files: &[String],
        now_ms: i64,
    ) -> Result<(), Error> {
        let files = serde_json::to_string(files).unwrap_or_else(|_| "[]".into());
        self.conn.execute(
            "INSERT INTO chat_messages (conversation, who, text, files, ms)
             SELECT ?1, ?2, ?3, ?4, ?5 WHERE EXISTS (SELECT 1 FROM conversations WHERE id = ?1)",
            params![id.0, who, text, files, now_ms],
        )?;
        Ok(())
    }

    /// The newest `limit` conversations, the latest used first.
    pub fn conversations(&self, limit: u32) -> Result<Vec<Conversation>, Error> {
        let mut stmt = self.conn.prepare(&format!(
            "SELECT {COLUMNS} FROM conversations ORDER BY updated_ms DESC, id DESC LIMIT ?1"
        ))?;
        let rows = stmt.query_map([limit], conversation)?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    pub fn conversation(&self, id: ConversationId) -> Result<Option<Conversation>, Error> {
        Ok(self
            .conn
            .query_row(
                &format!("SELECT {COLUMNS} FROM conversations WHERE id = ?1"),
                [id.0],
                conversation,
            )
            .optional()?)
    }

    /// What was said, in order.
    pub fn conversation_messages(&self, id: ConversationId) -> Result<Vec<Message>, Error> {
        let mut stmt = self
            .conn
            .prepare("SELECT who, text, files, ms FROM chat_messages WHERE conversation = ?1 ORDER BY id")?;
        let rows = stmt.query_map([id.0], |r| {
            let files: String = r.get(2)?;
            Ok(Message {
                who: r.get(0)?,
                text: r.get(1)?,
                files: serde_json::from_str(&files).unwrap_or_default(),
                ms: r.get(3)?,
            })
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    pub fn conversation_rename(&mut self, id: ConversationId, to: &str) -> Result<(), Error> {
        let to = title(to);
        if !to.is_empty() {
            self.conn.execute(
                "UPDATE conversations SET title = ?2 WHERE id = ?1",
                params![id.0, to],
            )?;
        }
        Ok(())
    }

    /// Deletes it and what was said in it, then compacts, so nothing of it lingers in the file.
    pub fn conversation_delete(&mut self, id: ConversationId) -> Result<(), Error> {
        let tx = self.conn.transaction()?;
        tx.execute("DELETE FROM chat_messages WHERE conversation = ?1", [id.0])?;
        tx.execute("DELETE FROM conversations WHERE id = ?1", [id.0])?;
        tx.commit()?;
        self.compact()
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    fn store(name: &str) -> Store {
        Store::open(&crate::temp(name)).unwrap()
    }

    fn new(text: &str) -> New {
        New {
            engine: "claude".into(),
            folder: "/w".into(),
            title: text.into(),
            settings: "user".into(),
            ..New::default()
        }
    }

    #[test]
    fn a_conversation_goes_on_where_it_stopped() {
        let mut s = store("conv-resume");
        let id = s
            .conversation_new(&new("\n  What is this project?\nMore"), 1)
            .unwrap();
        s.conversation_say(id, "you", "What is this project?", &["a.png".into()], 2)
            .unwrap();
        s.conversation_say(id, "zeca", "A desktop app.", &[], 3).unwrap();
        s.conversation_save(id, Some("session-1"), None, None, 4).unwrap();
        // A later save without a session keeps the one it had.
        s.conversation_save(id, None, None, None, 5).unwrap();
        s.conversation_save(id, None, None, Some("WebFetch"), 6).unwrap();
        s.conversation_save(id, None, None, Some("mcp__x"), 7).unwrap();
        let c = s.conversation(id).unwrap().unwrap();
        // The first tool that read from outside stays the one named.
        assert_eq!(c.outside.as_deref(), Some("WebFetch"));
        assert_eq!(c.title, "What is this project?");
        assert_eq!(c.session.as_deref(), Some("session-1"));
        assert_eq!((c.created_ms, c.updated_ms), (1, 7));
        let said = s.conversation_messages(id).unwrap();
        assert_eq!(said.len(), 2);
        assert_eq!(said[0].files, ["a.png"]);
        assert_eq!(said[1].who, "zeca");
    }

    #[test]
    fn the_frozen_context_comes_back_and_follows_a_restart_only_before_a_reply() {
        let mut s = store("conv-frozen");
        let id = s
            .conversation_new(
                &New {
                    frozen: Some("memory as it began".into()),
                    ..new("hi")
                },
                1,
            )
            .unwrap();
        assert_eq!(
            s.conversation(id).unwrap().unwrap().frozen.as_deref(),
            Some("memory as it began")
        );
        let moved = New {
            frozen: Some("rebuilt before the first reply".into()),
            ..new("hi")
        };
        s.conversation_restart(id, &moved).unwrap();
        assert_eq!(
            s.conversation(id).unwrap().unwrap().frozen.as_deref(),
            Some("rebuilt before the first reply")
        );
        s.conversation_save(id, Some("session"), None, None, 2).unwrap();
        s.conversation_restart(
            id,
            &New {
                frozen: Some("never".into()),
                ..new("hi")
            },
        )
        .unwrap();
        assert_eq!(
            s.conversation(id).unwrap().unwrap().frozen.as_deref(),
            Some("rebuilt before the first reply")
        );
    }

    #[test]
    fn the_latest_used_comes_first_and_a_title_is_cut() {
        let mut s = store("conv-list");
        let a = s.conversation_new(&new("first"), 10).unwrap();
        let b = s.conversation_new(&new(&"x".repeat(200)), 20).unwrap();
        s.conversation_save(a, None, None, None, 30).unwrap();
        let list = s.conversations(10).unwrap();
        assert_eq!(list.iter().map(|c| c.id).collect::<Vec<_>>(), [a, b]);
        assert_eq!(list[1].title.chars().count(), MAX_TITLE + 1);
        s.conversation_rename(b, "  Renamed  ").unwrap();
        s.conversation_rename(b, "   ").unwrap();
        assert_eq!(s.conversation(b).unwrap().unwrap().title, "Renamed");
    }

    #[test]
    fn deleting_takes_what_was_said_and_nothing_is_added_after() {
        let mut s = store("conv-delete");
        let id = s.conversation_new(&new("x"), 1).unwrap();
        s.conversation_say(id, "you", "secret", &[], 2).unwrap();
        s.conversation_delete(id).unwrap();
        assert!(s.conversation(id).unwrap().is_none());
        assert!(s.conversation_messages(id).unwrap().is_empty());
        // A turn that ends after the delete writes nothing.
        s.conversation_say(id, "zeca", "late", &[], 3).unwrap();
        assert!(s.conversation_messages(id).unwrap().is_empty());
    }

    #[test]
    fn an_id_never_comes_back_and_clear_history_takes_them_all() {
        let dir = crate::temp("conv-ids");
        let mut s = Store::open(&dir).unwrap();
        let first = s.conversation_new(&new("a"), 1).unwrap();
        s.conversation_delete(first).unwrap();
        let second = s.conversation_new(&new("b"), 2).unwrap();
        assert_ne!(first, second);
        s.conversation_say(second, "you", "b", &[], 3).unwrap();
        s.clear_all(&dir).unwrap();
        assert!(s.conversations(10).unwrap().is_empty());
        assert!(s.conversation_messages(second).unwrap().is_empty());
    }

    #[test]
    fn a_conversation_with_no_reply_follows_where_it_moved() {
        let mut s = store("conv-restart");
        let id = s.conversation_new(&new("x"), 1).unwrap();
        let moved = New {
            folder: "/other".into(),
            model: Some("opus".into()),
            ..new("ignored")
        };
        s.conversation_restart(id, &moved).unwrap();
        let c = s.conversation(id).unwrap().unwrap();
        assert_eq!(
            (c.folder.as_str(), c.model.as_deref(), c.title.as_str()),
            ("/other", Some("opus"), "x")
        );
        // Once it replied, it stays where it was.
        s.conversation_save(id, Some("s"), None, None, 2).unwrap();
        s.conversation_restart(
            id,
            &New {
                folder: "/third".into(),
                ..moved
            },
        )
        .unwrap();
        assert_eq!(s.conversation(id).unwrap().unwrap().folder, "/other");
    }

    #[test]
    fn past_the_cap_the_one_left_alone_longest_goes() {
        let mut s = store("conv-cap");
        let oldest = s.conversation_new(&new("old"), 0).unwrap();
        s.conversation_say(oldest, "you", "old", &[], 0).unwrap();
        for i in 1..=KEEP {
            s.conversation_new(&new("n"), i).unwrap();
        }
        assert!(s.conversation(oldest).unwrap().is_none());
        assert!(s.conversation_messages(oldest).unwrap().is_empty());
        assert_eq!(s.conversations(u32::MAX).unwrap().len() as i64, KEEP);
    }
    #[test]
    fn a_conversation_keeps_its_exact_profile_after_edits_restart_and_reopen() {
        use crate::profiles::{Profile, Snapshot, Source};
        let dir = crate::temp("conv-profile");
        let mut s = Store::open(&dir).unwrap();
        let mut profile = s
            .agent_profile_save(&Profile {
                id: 0,
                revision: 0,
                title: "My developer".into(),
                instructions: "Original instructions".into(),
                preset: "developer".into(),
                preset_revision: 1,
                species: "atratus".into(),
                look: "none".into(),
                about: "Description".into(),
            })
            .unwrap();
        let pinned = Snapshot::from(profile.clone());
        let request = New {
            profile: Some(pinned.clone()),
            ..new("A specialist conversation")
        };
        let id = s.conversation_new(&request, 1).unwrap();
        profile.title = "Edited name".into();
        profile.instructions = "Edited instructions".into();
        profile.species = "vultur".into();
        profile.look = "crown".into();
        let edited = s.agent_profile_save(&profile).unwrap();
        assert_eq!(edited.revision, 2);
        assert_eq!(
            s.agent_profile_revision(profile.id, 1)
                .unwrap()
                .unwrap()
                .instructions,
            "Original instructions"
        );
        assert_eq!(s.conversation(id).unwrap().unwrap().profile, Some(pinned.clone()));
        s.conversation_restart(
            id,
            &New {
                folder: "/another".into(),
                ..request.clone()
            },
        )
        .unwrap();
        assert!(s.conversation_restart(id, &new("Zeca")).is_err());
        assert!(
            s.conversation_restart(
                id,
                &New {
                    profile: Some(Snapshot::from(edited)),
                    ..request.clone()
                }
            )
            .is_err()
        );
        assert!(
            s.conversation_new(
                &New {
                    frozen: Some("Zeca private memory".into()),
                    ..request.clone()
                },
                2
            )
            .is_err()
        );
        let mut forged = pinned.clone();
        forged.instructions = "Forged revision".into();
        assert!(
            s.conversation_new(
                &New {
                    profile: Some(forged),
                    ..new("Forged")
                },
                2
            )
            .is_err()
        );
        let mut missing = pinned.clone();
        missing.source = Source::Custom {
            id: profile.id,
            revision: 999,
        };
        assert!(
            s.conversation_new(
                &New {
                    profile: Some(missing),
                    ..new("Missing")
                },
                2
            )
            .is_err()
        );
        assert!(
            s.conn
                .execute("UPDATE conversations SET profile=NULL WHERE id=?1", [id.0])
                .is_err()
        );
        assert_eq!(
            s.conversations(50).unwrap().len(),
            1,
            "Failed binding does not create a partial conversation"
        );
        s.conversation_save(id, Some("session-1"), None, None, 3).unwrap();
        s.conversation_rename(id, "Renamed").unwrap();
        drop(s);
        let mut s = Store::open(&dir).unwrap();
        assert_eq!(s.conversation(id).unwrap().unwrap().profile, Some(pinned));
        s.conversation_delete(id).unwrap();
        assert!(s.conversation(id).unwrap().is_none());
        assert_eq!(
            s.agent_profiles().unwrap().len(),
            1,
            "Deleting a conversation preserves the user profile"
        );
    }

    #[test]
    fn preset_snapshots_are_retained_and_corrupt_snapshots_never_fall_back_to_zeca() {
        use crate::profiles::{Snapshot, Source};
        let dir = crate::temp("conv-preset-profile");
        let mut s = Store::open(&dir).unwrap();
        let preset = Snapshot {
            source: Source::Preset {
                id: "reviewer".into(),
                revision: 1,
            },
            title: "Reviewer".into(),
            about: "Review changes".into(),
            instructions: "Review the changes".into(),
            species: "gypaetus".into(),
            look: "none".into(),
        };
        let legacy = s.conversation_new(&new("Zeca"), 1).unwrap();
        assert!(s.conversation(legacy).unwrap().unwrap().profile.is_none());
        let id = s
            .conversation_new(
                &New {
                    profile: Some(preset.clone()),
                    ..new("Review")
                },
                2,
            )
            .unwrap();
        assert_eq!(s.conversation(id).unwrap().unwrap().profile, Some(preset));
        s.conn.execute("INSERT INTO conversations(engine,folder,title,settings,created_ms,updated_ms,profile) VALUES('claude','','Broken','isolated',3,3,'{}')",[]).unwrap();
        let broken = ConversationId(s.conn.last_insert_rowid());
        assert!(s.conversation(broken).is_err());
        assert!(s.conversations(50).is_err());
        s.conversation_delete(broken).unwrap();
        s.clear_all(&dir).unwrap();
        assert!(s.conversations(50).unwrap().is_empty());
    }
    #[test]
    fn adding_profile_snapshots_preserves_legacy_conversation_context() {
        let dir = crate::temp("conv-profile-migration");
        let s = Store::connect(&dir).unwrap();
        for sql in &crate::MIGRATIONS[..30] {
            s.conn.execute_batch(sql).unwrap();
        }
        s.conn.pragma_update(None, "user_version", 30).unwrap();
        s.conn.execute("INSERT INTO conversations(id,engine,folder,title,settings,created_ms,updated_ms,frozen,session)
            VALUES(1,'claude','','Legacy','isolated',1,2,'original frozen context','legacy-session')",[]).unwrap();
        drop(s);
        let s = Store::open(&dir).unwrap();
        let legacy = s.conversation(ConversationId(1)).unwrap().unwrap();
        assert!(legacy.profile.is_none());
        assert_eq!(legacy.frozen.as_deref(), Some("original frozen context"));
        assert_eq!(legacy.session.as_deref(), Some("legacy-session"));
    }
}
