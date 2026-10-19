//! Application style preferences survive transcript retention and history erasure.
use crate::{Error, Store};
use rusqlite::params;
use std::collections::BTreeMap;

pub(crate) const TABLE: &str = "CREATE TABLE dictation_styles (
    app TEXT PRIMARY KEY NOT NULL CHECK(length(app)>0 AND length(app)<=1024),
    style TEXT NOT NULL CHECK(style IN ('verbatim','natural','polished','prompt'))
); ALTER TABLE dictations ADD COLUMN output_language TEXT CHECK(output_language IN ('en','pt-BR','es','zh'));";

fn app(value: &str) -> Result<(), Error> {
    if value.is_empty() || value.len() > 1024 || value.trim() != value || value.chars().any(char::is_control)
    {
        return Err(Error::Invalid(
            "Use an application identity up to 1024 bytes without control characters.",
        ));
    }
    Ok(())
}
impl Store {
    pub fn dictation_styles(&self) -> Result<BTreeMap<String, String>, Error> {
        let mut stmt = self
            .conn
            .prepare("SELECT app,style FROM dictation_styles ORDER BY app")?;
        Ok(stmt
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect::<Result<_, _>>()?)
    }
    /// The application owns desktop-alias normalization; the store validates the durable shape.
    pub fn dictation_style_set(&self, identity: &str, style: Option<&str>) -> Result<(), Error> {
        app(identity)?;
        if let Some(style) = style {
            if !matches!(style, "verbatim" | "natural" | "polished" | "prompt") {
                return Err(Error::Invalid("Choose a known dictation style."));
            }
            self.conn.execute("INSERT INTO dictation_styles(app,style) VALUES(?1,?2) ON CONFLICT(app) DO UPDATE SET style=excluded.style",params![identity,style])?;
        } else {
            self.conn
                .execute("DELETE FROM dictation_styles WHERE app=?1", [identity])?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn migration_retains_old_text_and_styles_survive_clear_and_round_trip() {
        let dir = crate::temp("styles-upgrade");
        {
            std::fs::create_dir_all(&dir).unwrap();
            let conn = rusqlite::Connection::open(dir.join(crate::FILE)).unwrap();
            for migration in &crate::MIGRATIONS[..25] {
                conn.execute_batch(migration).unwrap();
            }
            conn.pragma_update(None, "user_version", 25).unwrap();
            conn.execute("INSERT INTO dictations(at,heard,text,style,secs,words,outcome) VALUES(1,'old','old','verbatim',1,1,'copied')",[]).unwrap();
        }
        let mut db = Store::open(&dir).unwrap();
        assert_eq!(db.dictations(1).unwrap()[0].1.output_language, None);
        assert!(db.dictation_styles().unwrap().is_empty());
        for (app, style) in [
            ("konsole", "verbatim"),
            ("thunderbird", "polished"),
            ("slack", "natural"),
            ("sessions", "prompt"),
        ] {
            db.dictation_style_set(app, Some(style)).unwrap();
        }
        let expected = db.dictation_styles().unwrap();
        db.clear_all(&dir).unwrap();
        drop(db);
        let db = Store::open(&dir).unwrap();
        assert_eq!(db.dictation_styles().unwrap(), expected);
        db.dictation_style_set("slack", Some("polished")).unwrap();
        assert_eq!(db.dictation_styles().unwrap()["slack"], "polished");
        db.dictation_style_set("slack", None).unwrap();
        assert!(!db.dictation_styles().unwrap().contains_key("slack"));
        for invalid in ["", " mail", "mail\nchat", &"x".repeat(1025)] {
            assert!(db.dictation_style_set(invalid, Some("natural")).is_err());
        }
        assert!(db.dictation_style_set("mail", Some("assistant")).is_err());
        assert!(!db.dictation_styles().unwrap().contains_key("mail"));
    }
}
