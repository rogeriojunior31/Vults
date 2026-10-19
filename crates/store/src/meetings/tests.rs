use super::*;
use export::{Subtitle, markdown, subtitles};
use vults_core::i18n::Lang;

fn fixture() -> Meeting {
    Meeting {
        id: "meeting-1".into(),
        recorded_ms: 123,
        title: "Planning <test>".into(),
        microphone: Track {
            duration_ms: 3000,
            segments: vec![
                Segment {
                    start_ms: 0,
                    end_ms: 1500,
                    text: "Hello <script>\n\n00:00:01 --> injected".into(),
                    speaker: Some(0),
                },
                Segment {
                    start_ms: 1500,
                    end_ms: 1500,
                    text: "Untimed estimate".into(),
                    speaker: None,
                },
            ],
        },
        system: Track {
            duration_ms: 2000,
            segments: vec![Segment {
                start_ms: 500,
                end_ms: 2000,
                text: "I will send the notes.".into(),
                speaker: Some(0),
            }],
        },
        speakers: vec![
            Speaker {
                source: Source::Microphone,
                id: 0,
                name: None,
            },
            Speaker {
                source: Source::System,
                id: 0,
                name: Some("Alex".into()),
            },
        ],
        notes: Some(Notes {
            title: "Planning".into(),
            summary: "Notes agreed.".into(),
            key_points: vec!["Keep <source> clocks separate.".into()],
            todos: vec![Todo {
                text: "Send the notes".into(),
                owner: Some("Alex".into()),
                due: None,
            }],
        }),
    }
}

#[test]
fn roundtrip_retry_rename_and_delete_keep_sources_separate() {
    let dir = crate::temp("meetings-roundtrip");
    let mut store = Store::open(&dir).unwrap();
    let meeting = fixture();
    store.save_meeting(&meeting).unwrap();
    assert_eq!(store.meeting(&meeting.id).unwrap(), Some(meeting.clone()));
    assert!(
        store
            .rename_meeting_speaker(&meeting.id, Source::Microphone, 0, Some("Sam"))
            .unwrap()
    );
    store.save_meeting(&meeting).unwrap();
    let loaded = store.meeting(&meeting.id).unwrap().unwrap();
    assert_eq!(loaded.speakers[0].name.as_deref(), Some("Sam"));
    assert_eq!(loaded.speakers[1].name.as_deref(), Some("Alex"));
    let mut conflict = meeting.clone();
    conflict.title = "Different".into();
    assert!(store.save_meeting(&conflict).is_err());
    assert_eq!(store.meetings(1).unwrap().len(), 1);
    assert!(
        store
            .rename_meeting_speaker(&meeting.id, Source::Microphone, 0, None)
            .unwrap()
    );
    assert!(
        !store
            .rename_meeting_speaker(&meeting.id, Source::Microphone, 99, Some("Nobody"))
            .unwrap()
    );
    assert!(store.delete_meeting(&meeting.id).unwrap());
    assert!(store.meeting(&meeting.id).unwrap().is_none());
    for table in ["meeting_segments", "meeting_speakers"] {
        assert_eq!(
            store
                .conn
                .query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            0
        );
    }
    drop(store);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn migration_preserves_existing_history_and_global_erasure_removes_meeting_text() {
    let dir = crate::temp("meetings-migrate");
    std::fs::create_dir_all(&dir).unwrap();
    let conn = rusqlite::Connection::open(crate::path(&dir)).unwrap();
    for sql in &crate::MIGRATIONS[..26] {
        conn.execute_batch(sql).unwrap();
    }
    conn.pragma_update(None, "user_version", 26).unwrap();
    conn.execute("INSERT INTO dictation_styles VALUES ('app','natural')", [])
        .unwrap();
    drop(conn);
    let mut store = Store::open(&dir).unwrap();
    assert_eq!(store.dictation_styles().unwrap()["app"], "natural");
    store.save_meeting(&fixture()).unwrap();
    store.clear_all(&dir).unwrap();
    assert!(store.meetings(500).unwrap().is_empty());
    assert!(
        !std::fs::read(crate::path(&dir))
            .unwrap()
            .windows(b"I will send the notes".len())
            .any(|s| s == b"I will send the notes")
    );
    drop(store);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn invalid_snapshots_and_names_never_partially_write() {
    let dir = crate::temp("meetings-invalid");
    let mut store = Store::open(&dir).unwrap();
    for change in 0..6 {
        let mut value = fixture();
        match change {
            0 => value.system.segments[0].end_ms = 2001,
            1 => value.speakers.pop().map(|_| ()).unwrap(),
            2 => value.speakers[0].name = Some("Bad\nname".into()),
            3 => value.microphone.segments[0].text = "bad\0text".into(),
            4 => value.id = "../escape".into(),
            _ => value.speakers.push(value.speakers[0].clone()),
        }
        assert!(store.save_meeting(&value).is_err());
        assert!(store.meetings(500).unwrap().is_empty());
    }
    store.save_meeting(&fixture()).unwrap();
    assert!(
        store
            .rename_meeting_speaker("meeting-1", Source::System, 0, Some(" "))
            .is_err()
    );
    drop(store);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn exporters_match_golden_files_and_never_merge_clocks() {
    let meeting = fixture();
    assert_eq!(
        markdown(&meeting, Lang::En).unwrap(),
        include_str!("golden/meeting.md")
    );
    assert_eq!(
        subtitles(&meeting, Source::Microphone, Subtitle::Srt, Lang::En).unwrap(),
        include_str!("golden/microphone.srt")
    );
    assert_eq!(
        subtitles(&meeting, Source::System, Subtitle::Vtt, Lang::En).unwrap(),
        include_str!("golden/system.vtt")
    );
    for lang in [Lang::En, Lang::PtBr, Lang::Es, Lang::Zh] {
        assert!(
            markdown(&meeting, lang)
                .unwrap()
                .contains(vults_core::i18n::meeting_export(lang).microphone)
        );
        assert!(
            !subtitles(&meeting, Source::System, Subtitle::Srt, lang)
                .unwrap()
                .contains("Hello")
        );
    }
}
