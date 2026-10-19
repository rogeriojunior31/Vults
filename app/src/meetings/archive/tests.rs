use super::super::Slot;
use super::*;
use vults_chat::meeting_summary::{Summary, Todo};

struct Temp(std::path::PathBuf);
impl Temp {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("vults-archive-{}", crate::traces::new_id()));
        std::fs::create_dir_all(&path).expect("archive test directory");
        Self(path)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn track(start: u64, text: &str) -> super::super::transcription::Track {
    vults_voice::meeting::Transcript {
        duration_ms: start + 500,
        segments: vec![vults_voice::meeting::Segment {
            start_ms: start,
            end_ms: start + 500,
            text: text.into(),
        }],
    }
    .into()
}
fn ready() -> Slot {
    let mut capture = Capture::new();
    capture.epoch = 7;
    Slot {
        view: super::super::Snapshot {
            id: 3,
            phase: "finished",
            has_transcript: true,
            ..Default::default()
        },
        transcript: Some(super::super::transcription::Transcript {
            id: 3,
            microphone: track(0, "Microphone words."),
            system: track(2000, "System words."),
        }),
        archive: Some(capture),
        ..Default::default()
    }
}
fn notes() -> Summary {
    Summary {
        title: "Agreed notes".into(),
        summary: "The original commitment.".into(),
        key_points: vec!["Keep independent sources.".into()],
        todos: vec![Todo {
            text: "Send notes".into(),
            owner: None,
            due: None,
        }],
    }
}

#[test]
fn only_an_explicit_complete_snapshot_is_saved_and_retries_keep_its_first_contents() {
    let dir = Temp::new();
    let mut slot = ready();
    assert!(Store::open(&dir.0).unwrap().meetings(500).unwrap().is_empty());
    for (id, phase, epoch) in [(2, "finished", 7), (3, "recording", 7), (3, "finished", 8)] {
        slot.view.phase = phase;
        assert!(save_at(&dir.0, &mut slot, id, "A meeting", epoch, false).is_err());
        assert!(Store::open(&dir.0).unwrap().meetings(500).unwrap().is_empty());
    }
    slot.view.phase = "finished";
    assert!(save_at(&dir.0, &mut slot, 3, "", 7, false).is_err());
    let id = save_at(&dir.0, &mut slot, 3, "Saved meeting", 7, false).unwrap();
    assert_eq!(
        save_at(&dir.0, &mut slot, 3, "Later draft title", 7, false).unwrap(),
        id
    );
    let mut store = Store::open(&dir.0).unwrap();
    let saved = store.meeting(&id).unwrap().unwrap();
    assert_eq!(saved.title, "Saved meeting");
    assert_eq!(saved.microphone.segments[0].start_ms, 0);
    assert_eq!(saved.system.segments[0].start_ms, 2000);
    assert_eq!(saved.microphone.segments[0].text, "Microphone words.");
    assert!(saved.speakers.is_empty());
    assert!(saved.notes.is_none());
    assert_eq!(store.meetings(500).unwrap().len(), 1);
}

#[test]
fn frozen_notes_and_anonymous_metadata_remain_text_only() {
    let mut slot = ready();
    let transcript = slot.transcript.as_ref().unwrap();
    let capture = slot.archive.as_mut().unwrap();
    let saved = capture
        .freeze(transcript, "Meeting with notes", Some(notes()))
        .unwrap();
    let saved_notes = saved.notes.as_ref().unwrap();
    assert_eq!(saved_notes.summary, "The original commitment.");
    assert_eq!(saved_notes.todos[0].owner, None);
    assert_eq!(saved_notes.todos[0].due, None);
    assert_eq!(capture.freeze(transcript, "Changed", None).unwrap(), saved);
}

#[test]
fn deleting_current_archive_revokes_queued_saves_and_cannot_resurrect_deleted_text() {
    let dir = Temp::new();
    let mut slot = ready();
    let id = save_at(&dir.0, &mut slot, 3, "To erase", 7, false).unwrap();
    assert!(delete_at(&dir.0, &mut slot, &id).unwrap());
    assert!(slot.archive.is_none());
    assert!(save_at(&dir.0, &mut slot, 3, "Old queued save", 7, false).is_err());
    assert!(Store::open(&dir.0).unwrap().meetings(500).unwrap().is_empty());
}

#[test]
fn deletion_io_failure_still_revokes_the_pending_copy() {
    let dir = Temp::new();
    let mut slot = ready();
    let id = slot.archive.as_ref().unwrap().id.clone();
    let blocked = dir.0.join("blocked");
    std::fs::write(&blocked, b"not a directory").unwrap();
    assert!(delete_at(&blocked, &mut slot, &id).is_err());
    assert!(slot.archive.is_none());
    assert!(save_at(&dir.0, &mut slot, 3, "Old copy", 7, false).is_err());
}

#[test]
fn navigation_or_new_review_revokes_the_old_page_owned_capture() {
    let dir = Temp::new();
    let mut slot = ready();
    slot.end(3, true);
    assert!(slot.archive.is_none());
    assert!(save_at(&dir.0, &mut slot, 3, "Discarded", 7, false).is_err());
    assert!(Store::open(&dir.0).unwrap().meetings(500).unwrap().is_empty());
}
