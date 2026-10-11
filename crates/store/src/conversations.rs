//! Zeca's conversations (`docs/dev/plan-zeca.md` Z4): each one's engine, folder, title and what
//! resumes it (the CLI's session or thread, or the API's history), and what was said, so a
//! conversation goes on after the app restarts. The CLIs keep their own copy of the context; this
//! is what the chat shows. Kept until the user deletes them (one, or all with Clear history), the
//! newest [`KEEP`] at most.

use rusqlite::{OptionalExtension, params};

use crate::{Error, Store};

/// Conversations kept; a new one past this drops the one left alone longest.
pub const KEEP: i64 = 200;
/// Longest title kept, in characters.
pub const MAX_TITLE: usize = 80;

/// Migration 5. Shipped: never edit it.
pub(crate) const TABLE: &str = "
    CREATE TABLE conversations (
        id INTEGER PRIMARY KEY,
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
    /// Unix milliseconds.
    pub created_ms: i64,
    pub updated_ms: i64,
}

/// One line of a conversation: `you`, `zeca`, `error` or `note`.
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
    })
}

const COLUMNS: &str =
    "id, engine, folder, title, session, model, effort, settings, history, created_ms, updated_ms";

impl Store {
    /// Starts a conversation; its title is the first line of `new.title`, cut. Past [`KEEP`], the
    /// one left alone longest goes, with what was said in it.
    pub fn conversation_new(&mut self, new: &New, now_ms: i64) -> Result<ConversationId, Error> {
        let tx = self.conn.transaction()?;
        tx.execute(
            "INSERT INTO conversations (engine, folder, title, model, effort, settings, created_ms, updated_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
            params![new.engine, new.folder, title(&new.title), new.model, new.effort, new.settings, now_ms],
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

    /// What resumes it, after a turn: the CLI's session, or the API's history.
    pub fn conversation_save(
        &mut self,
        id: ConversationId,
        session: Option<&str>,
        history: Option<&str>,
        now_ms: i64,
    ) -> Result<(), Error> {
        self.conn.execute(
            "UPDATE conversations SET session = COALESCE(?2, session), history = COALESCE(?3, history),
             updated_ms = ?4 WHERE id = ?1",
            params![id.0, session, history, now_ms],
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
        s.conversation_save(id, Some("session-1"), None, 4).unwrap();
        // A later save without a session keeps the one it had.
        s.conversation_save(id, None, None, 5).unwrap();
        let c = s.conversation(id).unwrap().unwrap();
        assert_eq!(c.title, "What is this project?");
        assert_eq!(c.session.as_deref(), Some("session-1"));
        assert_eq!((c.created_ms, c.updated_ms), (1, 5));
        let said = s.conversation_messages(id).unwrap();
        assert_eq!(said.len(), 2);
        assert_eq!(said[0].files, ["a.png"]);
        assert_eq!(said[1].who, "zeca");
    }

    #[test]
    fn the_latest_used_comes_first_and_a_title_is_cut() {
        let mut s = store("conv-list");
        let a = s.conversation_new(&new("first"), 10).unwrap();
        let b = s.conversation_new(&new(&"x".repeat(200)), 20).unwrap();
        s.conversation_save(a, None, None, 30).unwrap();
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
}
