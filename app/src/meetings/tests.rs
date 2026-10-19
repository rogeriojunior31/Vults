use super::*;

fn ready(now: Instant) -> Slot {
    let mut slot = Slot::default();
    slot.view.id = 7;
    slot.view.phase = "ready";
    slot.view.model_id = Some("base".into());
    slot.reviewed = Some(now);
    slot
}

#[test]
fn only_the_nest_may_review_start_read_or_stop_recording() {
    assert!(authorized(crate::NEST).is_ok());
    for label in [
        crate::SETTINGS,
        crate::ISLAND,
        crate::WIDGET,
        crate::NEST_WINDOW,
        "other",
        "",
    ] {
        assert!(authorized(label).is_err());
    }
}

#[test]
fn each_start_requires_fresh_source_review_and_explicit_consent() {
    let now = Instant::now();
    for (id, consent, at) in [
        (6, true, now),
        (7, false, now),
        (7, true, now + REVIEW + Duration::from_secs(1)),
    ] {
        let mut slot = ready(now);
        assert!(slot.begin(id, consent, false, at).is_err());
        assert_eq!(slot.view.phase, "ready");
        assert!(slot.action.is_none());
    }
    let mut slot = ready(now);
    slot.begin(7, true, false, now).expect("reviewed explicit start");
    assert_eq!(slot.view.phase, "starting");
    assert!(slot.reviewed.is_none());
    assert!(
        slot.begin(7, true, false, now).is_err(),
        "a review is consumed once"
    );
}

#[test]
fn stop_is_idempotent_cancel_wins_and_stale_actions_do_not_touch_new_recordings() {
    let mut slot = ready(Instant::now());
    slot.view.phase = "recording";
    slot.end(6, true);
    assert_eq!(slot.view.phase, "recording");
    assert!(slot.action.is_none());
    slot.end(7, false);
    slot.end(7, false);
    assert_eq!(slot.view.phase, "stopping");
    assert!(matches!(slot.action, Some(Action::End(7, false))));
    slot.end(7, true);
    slot.end(7, false);
    assert_eq!(slot.view.phase, "cancelling");
    assert!(matches!(slot.action, Some(Action::End(7, true))));
}

#[test]
fn startup_stops_cancel_and_finished_actions_never_restart() {
    for phase in ["starting", "preparing"] {
        let mut slot = ready(Instant::now());
        slot.view.phase = phase;
        slot.end(7, false);
        assert_eq!(slot.view.phase, "cancelling");
    }
    for phase in ["idle", "cancelled", "failed", "recovering", "unsupported"] {
        let mut slot = ready(Instant::now());
        slot.view.phase = phase;
        slot.end(7, true);
        assert_eq!(slot.view.phase, phase);
        assert!(slot.action.is_none());
    }
}

#[test]
fn ready_reviews_and_finished_transcripts_are_discarded_only_by_their_own_cancel() {
    for phase in ["ready", "finished"] {
        let mut slot = ready(Instant::now());
        slot.view.phase = phase;
        slot.view.has_transcript = true;
        slot.end(6, true);
        assert_eq!(slot.view.phase, phase);
        assert!(slot.view.has_transcript);
        slot.end(7, true);
        assert_eq!(slot.view.phase, "cancelled");
        assert!(!slot.view.has_transcript);
        assert!(slot.reviewed.is_none());
    }
}

#[cfg(target_os = "linux")]
#[test]
fn completion_publishes_only_after_cleanup_and_cancellation_discards_late_success() {
    use vults_audio::meeting::{End, Outcome, Report, Sources, Track};
    let report = || {
        Outcome::Finished(Report {
            sources: Sources {
                microphone: "fixture-mic".into(),
                monitor: "fixture.monitor".into(),
                monitor_fallback: false,
            },
            microphone: Track {
                rate: 16_000,
                channels: 1,
                frames: 10,
                peak: 0.1,
            },
            system: Track {
                rate: 16_000,
                channels: 1,
                frames: 10,
                peak: 0.1,
            },
            startup: Duration::ZERO,
            end: End::Stopped,
        })
    };
    for cancelled in [false, true] {
        let mut slot = ready(Instant::now());
        slot.view.phase = "transcribing";
        let mut job = transcription::Job {
            result: Some(transcription::Transcript {
                id: 7,
                microphone: vults_voice::meeting::Transcript::default().into(),
                system: vults_voice::meeting::Transcript::default().into(),
            }),
            ..Default::default()
        };
        slot.end(7, false);
        assert_eq!(
            slot.view.phase, "transcribing",
            "Stop cannot restart or cancel inference"
        );
        if cancelled {
            slot.end(7, true);
        }
        slot.complete(Ok(report()), &mut job);
        assert!(
            job.result.is_none(),
            "no hidden worker copy may retain a finished transcript"
        );
        assert_eq!(slot.view.phase, if cancelled { "cancelled" } else { "finished" });
        assert_eq!(slot.view.has_transcript, !cancelled);
        assert_eq!(slot.transcript.is_some(), !cancelled);
        if !cancelled {
            slot.end(7, true);
            assert!(slot.transcript.is_none());
        }
    }
    let mut slot = ready(Instant::now());
    slot.view.phase = "transcribing";
    let mut job = transcription::Job::default();
    slot.complete(Err("cleanup failed".into()), &mut job);
    assert_eq!(slot.view.phase, "failed");
    assert_eq!(slot.view.error, Some("capture"));
    assert!(!slot.view.has_transcript);
}

#[test]
fn identification_requires_an_available_preview_and_a_new_explicit_choice() {
    let now = Instant::now();
    let mut slot = ready(now);
    slot.view.identification_available = false;
    assert!(slot.begin(7, true, true, now).is_err());
    assert_eq!(slot.view.phase, "ready");
    assert!(slot.reviewed.is_some());
    slot.view.identification_available = true;
    slot.begin(7, true, true, now).expect("explicit preview choice");
    assert!(matches!(slot.action, Some(Action::Begin(7, true))));
    let mut next = ready(now);
    next.view.identification_available = true;
    next.begin(7, true, false, now).expect("normal transcription");
    assert!(matches!(next.action, Some(Action::Begin(7, false))));
}
