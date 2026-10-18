//! User-maintained dictation vocabulary. History retention never removes these preferences.

use rusqlite::{OptionalExtension, params};
use serde::Serialize;

use crate::{Error, Store};

pub(crate) const TABLE: &str = "
    CREATE TABLE dictionary_words (word TEXT NOT NULL, key TEXT PRIMARY KEY NOT NULL);
    CREATE TABLE dictionary_replacements (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        \"from\" TEXT NOT NULL, \"to\" TEXT NOT NULL, key TEXT NOT NULL UNIQUE
    );
    CREATE TABLE snippets (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        trigger TEXT NOT NULL, text TEXT NOT NULL, key TEXT NOT NULL UNIQUE
    );";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Replacement {
    pub id: i64,
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Snippet {
    pub id: i64,
    pub trigger: String,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Words {
    pub words: Vec<String>,
    pub replacements: Vec<Replacement>,
    pub snippets: Vec<Snippet>,
}

fn phrase(value: &str) -> Result<&str, Error> {
    let value = value.trim();
    if value.is_empty() || value.len() > 1024 || value.chars().any(char::is_control) {
        return Err(Error::Invalid(
            "Use a non-empty phrase up to 1024 bytes without control characters.",
        ));
    }
    Ok(value)
}

fn body(value: &str) -> Result<(), Error> {
    // Preserve literal spaces/newlines; neither a shell nor a replacement expression reads it.
    if value.trim().is_empty() || value.len() > 65_536 || value.contains('\0') {
        return Err(Error::Invalid(
            "Use non-empty text up to 65536 bytes without NUL characters.",
        ));
    }
    Ok(())
}

fn key(value: &str) -> String {
    // SQLite's NOCASE misses accented names; lower then upper also joins sharp-S/final sigma.
    value.to_lowercase().to_uppercase()
}

impl Store {
    /// One read snapshot, with replacements/snippets in stable insertion order.
    pub fn dictation_words(&mut self) -> Result<Words, Error> {
        let tx = self.conn.transaction()?;
        let words = tx
            .prepare("SELECT word FROM dictionary_words ORDER BY key")?
            .query_map([], |r| r.get(0))?
            .collect::<Result<_, _>>()?;
        let replacements = tx
            .prepare("SELECT id, \"from\", \"to\" FROM dictionary_replacements ORDER BY id")?
            .query_map([], |r| {
                Ok(Replacement {
                    id: r.get(0)?,
                    from: r.get(1)?,
                    to: r.get(2)?,
                })
            })?
            .collect::<Result<_, _>>()?;
        let snippets = tx
            .prepare("SELECT id, trigger, text FROM snippets ORDER BY id")?
            .query_map([], |r| {
                Ok(Snippet {
                    id: r.get(0)?,
                    trigger: r.get(1)?,
                    text: r.get(2)?,
                })
            })?
            .collect::<Result<_, _>>()?;
        tx.commit()?;
        Ok(Words {
            words,
            replacements,
            snippets,
        })
    }

    pub fn dictionary_word_add(&self, word: &str) -> Result<(), Error> {
        let word = phrase(word)?;
        if self.conn.execute(
            "INSERT INTO dictionary_words (word, key) VALUES (?1, ?2) ON CONFLICT(key) DO NOTHING",
            params![word, key(word)],
        )? == 0
        {
            return Err(Error::Invalid("This dictionary word already exists."));
        }
        Ok(())
    }

    pub fn dictionary_word_delete(&mut self, word: &str) -> Result<(), Error> {
        if self.conn.execute(
            "DELETE FROM dictionary_words WHERE key = ?1",
            [key(phrase(word)?)],
        )? > 0
        {
            self.compact()?;
        }
        Ok(())
    }

    /// Updating a source preserves its id and therefore its place in the text pipeline.
    pub fn dictionary_replacement_set(&self, from: &str, to: &str) -> Result<i64, Error> {
        let from = phrase(from)?;
        body(to)?;
        Ok(self.conn.query_row("INSERT INTO dictionary_replacements (\"from\", \"to\", key) VALUES (?1, ?2, ?3)
            ON CONFLICT(key) DO UPDATE SET \"from\" = excluded.\"from\", \"to\" = excluded.\"to\" RETURNING id",
            params![from, to, key(from)], |r| r.get(0))?)
    }

    pub fn dictionary_replacement_delete(&mut self, id: i64) -> Result<(), Error> {
        if id <= 0 {
            return Err(Error::Invalid("Invalid replacement id."));
        }
        if self
            .conn
            .execute("DELETE FROM dictionary_replacements WHERE id = ?1", [id])?
            > 0
        {
            self.compact()?;
        }
        Ok(())
    }

    pub fn snippet_add(&self, trigger: &str, text: &str) -> Result<i64, Error> {
        let trigger = phrase(trigger)?;
        body(text)?;
        self.conn.query_row("INSERT INTO snippets (trigger, text, key) VALUES (?1, ?2, ?3) ON CONFLICT(key) DO NOTHING RETURNING id",
            params![trigger, text, key(trigger)], |r| r.get(0)).optional()?
            .ok_or(Error::Invalid("This snippet trigger already exists."))
    }

    pub fn snippet_delete(&mut self, id: i64) -> Result<(), Error> {
        if id <= 0 {
            return Err(Error::Invalid("Invalid snippet id."));
        }
        if self.conn.execute("DELETE FROM snippets WHERE id = ?1", [id])? > 0 {
            self.compact()?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
