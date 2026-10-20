//! Explicit text-only meeting snapshots. Source clocks and anonymous identities remain separate.

pub mod export;

use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{Error, Store};

pub(crate) const TABLE: &str = "
CREATE TABLE meetings (
    id TEXT PRIMARY KEY, recorded_ms INTEGER NOT NULL, title TEXT NOT NULL,
    snapshot TEXT NOT NULL, fingerprint BLOB NOT NULL
);
CREATE INDEX meetings_recorded ON meetings(recorded_ms DESC, id);
CREATE TABLE meeting_speakers (
    meeting_id TEXT NOT NULL REFERENCES meetings(id) ON DELETE CASCADE,
    source TEXT NOT NULL, speaker INTEGER NOT NULL, name TEXT,
    PRIMARY KEY(meeting_id, source, speaker)
);
CREATE TABLE meeting_segments (
    meeting_id TEXT NOT NULL REFERENCES meetings(id) ON DELETE CASCADE,
    source TEXT NOT NULL, ordinal INTEGER NOT NULL,
    start_ms INTEGER NOT NULL, end_ms INTEGER NOT NULL, text TEXT NOT NULL, speaker INTEGER,
    PRIMARY KEY(meeting_id, source, ordinal),
    FOREIGN KEY(meeting_id, source, speaker)
        REFERENCES meeting_speakers(meeting_id, source, speaker)
);";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Source {
    Microphone,
    System,
}

impl Source {
    fn key(self) -> &'static str {
        match self {
            Self::Microphone => "microphone",
            Self::System => "system",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Segment {
    pub start_ms: u32,
    pub end_ms: u32,
    pub text: String,
    pub speaker: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Track {
    pub duration_ms: u32,
    pub segments: Vec<Segment>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Speaker {
    pub source: Source,
    pub id: u32,
    pub name: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Notes {
    pub title: String,
    pub summary: String,
    pub key_points: Vec<String>,
    pub todos: Vec<Todo>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Todo {
    pub text: String,
    pub owner: Option<String>,
    pub due: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Meeting {
    pub id: String,
    pub recorded_ms: i64,
    pub title: String,
    pub microphone: Track,
    pub system: Track,
    pub speakers: Vec<Speaker>,
    pub notes: Option<Notes>,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct Entry {
    pub id: String,
    pub recorded_ms: i64,
    pub title: String,
}

fn text(value: &str, max: usize) -> bool {
    !value.trim().is_empty()
        && value.len() <= max
        && !value
            .chars()
            .any(|c| c.is_control() && !matches!(c, '\n' | '\r' | '\t'))
}

fn name(value: &str) -> bool {
    text(value, 120) && value.trim() == value && !value.chars().any(char::is_control)
}

impl Meeting {
    pub fn track(&self, source: Source) -> &Track {
        match source {
            Source::Microphone => &self.microphone,
            Source::System => &self.system,
        }
    }

    pub fn validate(&self) -> Result<(), Error> {
        let invalid = || Error::Invalid("Invalid meeting snapshot");
        if self.id.is_empty()
            || self.id.len() > 64
            || !self
                .id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-'))
            || self.recorded_ms < 0
            || !text(&self.title, 512)
            || self.speakers.len() > 256
        {
            return Err(invalid());
        }
        for (index, speaker) in self.speakers.iter().enumerate() {
            if speaker.name.as_deref().is_some_and(|v| !name(v))
                || self.speakers[..index]
                    .iter()
                    .any(|s| s.source == speaker.source && s.id == speaker.id)
                || !self
                    .track(speaker.source)
                    .segments
                    .iter()
                    .any(|s| s.speaker == Some(speaker.id))
            {
                return Err(invalid());
            }
        }
        let mut bytes = 0usize;
        for source in [Source::Microphone, Source::System] {
            let track = self.track(source);
            if track.duration_ms > 86_400_000 || track.segments.len() > 100_000 {
                return Err(invalid());
            }
            let mut previous = 0;
            for segment in &track.segments {
                bytes = bytes.saturating_add(segment.text.len());
                if segment.start_ms < previous
                    || segment.start_ms > segment.end_ms
                    || segment.end_ms > track.duration_ms
                    || !text(&segment.text, 16_384)
                    || segment
                        .speaker
                        .is_some_and(|id| !self.speakers.iter().any(|s| s.source == source && s.id == id))
                {
                    return Err(invalid());
                }
                previous = segment.start_ms;
            }
        }
        if bytes > 4 * 1024 * 1024 || self.microphone.segments.is_empty() && self.system.segments.is_empty() {
            return Err(invalid());
        }
        if let Some(notes) = &self.notes
            && (!text(&notes.title, 160)
                || !text(&notes.summary, 8192)
                || notes.key_points.len() > 64
                || notes.todos.len() > 64
                || notes.key_points.iter().any(|p| !text(p, 4096))
                || notes.todos.iter().any(|t| {
                    !text(&t.text, 4096)
                        || [&t.owner, &t.due]
                            .into_iter()
                            .flatten()
                            .any(|v| !text(v, 256) || v.chars().any(char::is_control))
                }))
        {
            return Err(invalid());
        }
        Ok(())
    }
}

impl Store {
    /// A retry cannot overwrite a later human speaker rename. Reusing an ID for other content fails.
    pub fn save_meeting(&mut self, meeting: &Meeting) -> Result<(), Error> {
        meeting.validate()?;
        let snapshot =
            serde_json::to_string(meeting).map_err(|_| Error::Invalid("Invalid meeting snapshot"))?;
        let fingerprint = Sha256::digest(snapshot.as_bytes()).to_vec();
        let tx = self
            .conn
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let existing: Option<Vec<u8>> = tx
            .query_row(
                "SELECT fingerprint FROM meetings WHERE id=?1",
                [&meeting.id],
                |r| r.get(0),
            )
            .optional()?;
        if let Some(existing) = existing {
            return if existing == fingerprint {
                Ok(())
            } else {
                Err(Error::Invalid("Meeting ID already contains another snapshot"))
            };
        }
        // The parent snapshot holds metadata/notes; segment text and names have one authoritative row.
        let mut metadata = meeting.clone();
        metadata.microphone.segments.clear();
        metadata.system.segments.clear();
        metadata.speakers.clear();
        let metadata =
            serde_json::to_string(&metadata).map_err(|_| Error::Invalid("Invalid meeting metadata"))?;
        tx.execute(
            "INSERT INTO meetings VALUES (?1,?2,?3,?4,?5)",
            params![
                meeting.id,
                meeting.recorded_ms,
                meeting.title,
                metadata,
                fingerprint
            ],
        )?;
        for speaker in &meeting.speakers {
            tx.execute(
                "INSERT INTO meeting_speakers VALUES (?1,?2,?3,?4)",
                params![meeting.id, speaker.source.key(), speaker.id, speaker.name],
            )?;
        }
        for source in [Source::Microphone, Source::System] {
            for (ordinal, segment) in meeting.track(source).segments.iter().enumerate() {
                tx.execute(
                    "INSERT INTO meeting_segments VALUES (?1,?2,?3,?4,?5,?6,?7)",
                    params![
                        meeting.id,
                        source.key(),
                        ordinal as i64,
                        segment.start_ms,
                        segment.end_ms,
                        segment.text,
                        segment.speaker
                    ],
                )?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    pub fn meeting(&mut self, id: &str) -> Result<Option<Meeting>, Error> {
        // Read all rows under one snapshot, including concurrent renames/deletion.
        let tx = self.conn.transaction()?;
        let metadata: Option<String> = tx
            .query_row("SELECT snapshot FROM meetings WHERE id=?1", [id], |r| r.get(0))
            .optional()?;
        let Some(metadata) = metadata else { return Ok(None) };
        let mut meeting: Meeting =
            serde_json::from_str(&metadata).map_err(|_| Error::Invalid("Invalid saved meeting"))?;
        for source in [Source::Microphone, Source::System] {
            let segments = tx.prepare("SELECT start_ms,end_ms,text,speaker FROM meeting_segments WHERE meeting_id=?1 AND source=?2 ORDER BY ordinal")?
                .query_map(params![id,source.key()], |r| Ok(Segment {start_ms:r.get(0)?,end_ms:r.get(1)?,text:r.get(2)?,speaker:r.get(3)?}))?
                .collect::<Result<Vec<_>,_>>()?;
            match source {
                Source::Microphone => meeting.microphone.segments = segments,
                Source::System => meeting.system.segments = segments,
            }
            let speakers = tx.prepare("SELECT speaker,name FROM meeting_speakers WHERE meeting_id=?1 AND source=?2 ORDER BY speaker")?
                .query_map(params![id,source.key()], |r| Ok(Speaker {source,id:r.get(0)?,name:r.get(1)?}))?
                .collect::<Result<Vec<_>,_>>()?;
            meeting.speakers.extend(speakers);
        }
        meeting.validate()?;
        tx.commit()?;
        Ok(Some(meeting))
    }

    pub fn meetings(&self, limit: usize) -> Result<Vec<Entry>, Error> {
        Ok(self
            .conn
            .prepare("SELECT id,recorded_ms,title FROM meetings ORDER BY recorded_ms DESC,id LIMIT ?1")?
            .query_map([limit.min(500) as i64], |r| {
                Ok(Entry {
                    id: r.get(0)?,
                    recorded_ms: r.get(1)?,
                    title: r.get(2)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?)
    }

    pub fn rename_meeting_speaker(
        &self,
        id: &str,
        source: Source,
        speaker: u32,
        value: Option<&str>,
    ) -> Result<bool, Error> {
        if value.is_some_and(|v| !name(v)) {
            return Err(Error::Invalid("Invalid speaker name"));
        }
        Ok(self.conn.execute(
            "UPDATE meeting_speakers SET name=?4 WHERE meeting_id=?1 AND source=?2 AND speaker=?3",
            params![id, source.key(), speaker, value],
        )? == 1)
    }

    pub fn delete_meeting(&mut self, id: &str) -> Result<bool, Error> {
        let deleted = self.conn.execute("DELETE FROM meetings WHERE id=?1", [id])? != 0;
        self.compact()?;
        Ok(deleted)
    }
}

#[cfg(test)]
mod tests;
