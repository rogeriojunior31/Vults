use super::*;
use serde_json::json;
use vults_core::{self as core, AgentEvent, AgentKind, AgentUpdate, Input, SessionKey, State, Terminal};
fn key(id: &str) -> SessionKey {
    SessionKey {
        agent: AgentKind::Codex,
        session_id: id.into(),
    }
}
pub(super) fn fixture(now: Instant) -> State {
    let mut state = State::default();
    for (id, path) in [("a", "/work/api"), ("b", "/work/web")] {
        core::reduce(
            &mut state,
            Input::Agent(AgentUpdate {
                session: key(id),
                cwd: Some(path.into()),
                terminal: Terminal::default(),
                agent_id: None,
                event: AgentEvent::SessionStarted,
                received: None,
            }),
            now,
        );
    }
    state
}
#[test]
fn commands_are_consumed_and_only_unrecognized_text_can_reach_the_model() {
    let now = Instant::now();
    let mut state = fixture(now);
    let mut commands = Commands::new(now);
    assert!(route(&mut commands, &mut state, "tell me a story", true, now, json!([])).is_none());
    for (text, expected) in [
        ("next", Feedback::Done),
        ("approve all", Feedback::NotByVoice),
        ("open unknown-name", Feedback::UnknownName),
        ("go to the api terminal", Feedback::Requested),
        ("who is waiting", Feedback::Answer),
    ] {
        let report = route(&mut commands, &mut state, text, true, now, json!([])).unwrap();
        assert_eq!(report.feedback, expected, "{text}: {report:?}");
        assert!(!report.chat_fallback(true));
        assert!(report.effects.iter().all(|effect| matches!(
            effect,
            core::Effect::JumpToTerminal(_) | core::Effect::OpenFolder(_) | core::Effect::SaveProjects(_)
        )));
    }
    let before = state.focus.clone();
    let report = route(&mut commands, &mut state, "previous", false, now, json!([])).unwrap();
    assert_eq!(report.feedback, Feedback::Unavailable);
    assert_eq!(state.focus, before);
    assert!(report.effects.is_empty());
    assert_eq!(
        route(&mut commands, &mut state, "approve all", false, now, json!([]))
            .unwrap()
            .feedback,
        Feedback::NotByVoice
    );
}
#[test]
fn isolated_context_expiry_and_bilingual_reads_reuse_v8_dispatch() {
    let now = Instant::now();
    let mut state = fixture(now);
    let mut commands = Commands::new(now);
    core::reduce(
        &mut state,
        Input::Agent(AgentUpdate {
            session: key("c"),
            cwd: Some("/work/api-gateway".into()),
            terminal: Terminal::default(),
            agent_id: None,
            event: AgentEvent::SessionStarted,
            received: None,
        }),
        now,
    );
    let focus = state.focus.clone();
    let question = route(&mut commands, &mut state, "open apy", true, now, json!([])).unwrap();
    assert_eq!(question.feedback, Feedback::WhichOne);
    assert_eq!(state.focus, focus);
    let resolved = route(&mut commands, &mut state, "api-gateway", true, now, json!([])).unwrap();
    assert_eq!(resolved.feedback, Feedback::Done);
    assert_eq!(state.focus, Some(key("c")));
    let mut next_session = Commands::new(now);
    assert_eq!(
        route(&mut next_session, &mut state, "undo", true, now, json!([]))
            .unwrap()
            .feedback,
        Feedback::NoContext
    );
    for lang in [
        core::i18n::Lang::En,
        core::i18n::Lang::PtBr,
        core::i18n::Lang::Es,
        core::i18n::Lang::Zh,
    ] {
        state.lang = lang;
        let answer = route(&mut commands, &mut state, "who is waiting", true, now, json!([])).unwrap();
        assert_eq!(answer.text, core::i18n::voice_feedback(lang, Feedback::NoWaiting));
    }
}
#[test]
fn a_dropped_waiter_revokes_a_queued_request_and_debug_never_contains_text() {
    let turn = Turn::new(vec![0; 2400])
        .unwrap()
        .with_transcript("private capture words".into())
        .unwrap();
    assert!(!format!("{turn:?}").contains("private capture"));
    let pending = Pending::new();
    let queued = pending.0.clone();
    drop(pending);
    assert!(!*queued.lock().unwrap());
    assert!(
        Turn::new(vec![0; 2400])
            .unwrap()
            .with_transcript("x".repeat(4097))
            .is_err()
    );
    assert!(
        Turn::new(vec![0; 2400])
            .unwrap()
            .with_transcript("bad\0text".into())
            .is_err()
    );
}

#[test]
fn revoked_or_consumed_admission_cannot_mutate_the_flock() {
    let now = Instant::now();
    let mut state = fixture(now);
    let mut commands = Commands::new(now);
    let pending = Pending::new();
    let queued = pending.0.clone();
    assert!(
        consume(
            &queued,
            now + std::time::Duration::from_secs(10),
            || false,
            || route(&mut commands, &mut state, "next", true, now, json!([]))
        )
        .is_none()
    );
    assert!(state.focus.is_none());
    let report = consume(
        &queued,
        now + std::time::Duration::from_secs(10),
        || true,
        || route(&mut commands, &mut state, "next", true, now, json!([])),
    )
    .unwrap()
    .unwrap();
    assert_eq!(report.feedback, Feedback::Done);
    let focus = state.focus.clone();
    assert!(
        consume(
            &queued,
            now + std::time::Duration::from_secs(10),
            || true,
            || route(&mut commands, &mut state, "next", true, now, json!([]))
        )
        .is_none()
    );
    assert_eq!(state.focus, focus);
    let pending = Pending::new();
    let queued = pending.0.clone();
    drop(pending);
    assert!(
        consume(
            &queued,
            now + std::time::Duration::from_secs(10),
            || true,
            || route(&mut commands, &mut state, "next", true, now, json!([]))
        )
        .is_none()
    );
    assert_eq!(state.focus, focus);
}

#[test]
fn an_expired_admission_is_checked_after_lock_acquisition() {
    let now = Instant::now();
    let mut state = fixture(now);
    let mut commands = Commands::new(now);
    let settings = Mutex::new(());
    let held = settings.lock().unwrap();
    let pending = Mutex::new(true);
    let deadline = Instant::now();
    drop(held);
    let _saved = settings.lock().unwrap();
    assert!(
        consume(
            &pending,
            deadline,
            || true,
            || route(&mut commands, &mut state, "next", true, Instant::now(), json!([]))
        )
        .is_none()
    );
    assert!(state.focus.is_none());
}

#[test]
fn playback_commands_are_consumed_without_upload_or_unadmitted_execution() {
    let now = Instant::now();
    let mut state = fixture(now);
    let mut commands = Commands::new(now);
    let report = route(&mut commands, &mut state, "pause media", true, now, json!([]));
    let report = report.expect("known local command must not reach the provider");
    assert_eq!(report.feedback, Feedback::Unavailable);
    assert!(report.desktop.is_empty() && report.effects.is_empty());
}

#[test]
fn app_open_requests_are_consumed_without_upload_or_a_native_launch() {
    let now = Instant::now();
    let mut state = fixture(now);
    let mut commands = Commands::new(now);
    let report = route(
        &mut commands,
        &mut state,
        "open app Obsidian",
        true,
        now,
        json!([]),
    )
    .unwrap();
    assert_eq!(report.feedback, Feedback::Unavailable);
    assert!(report.desktop.is_empty() && report.effects.is_empty());
}

#[test]
fn output_control_speech_never_uploads_or_executes_through_realtime() {
    let now = Instant::now();
    let mut state = fixture(now);
    let mut commands = Commands::new(now);
    let out = route(&mut commands, &mut state, "mute computer", true, now, json!([])).unwrap();
    assert_eq!(out.feedback, Feedback::Unavailable);
    assert!(out.desktop.is_empty() && out.effects.is_empty());
}

#[test]
fn mixed_audio_commands_stay_local_and_unavailable_in_realtime() {
    let now = Instant::now();
    let mut state = fixture(now);
    let mut commands = crate::voice_commands::Commands::new(now);
    let out = route(
        &mut commands,
        &mut state,
        "pause media and then mute computer",
        true,
        now,
        serde_json::json!([]),
    )
    .expect("recognized local batch");
    assert_eq!(out.feedback, crate::voice_commands::Feedback::Unavailable);
    assert!(out.desktop.is_empty() && out.effects.is_empty());
    assert!(!out.chat_fallback(true));
}

#[test]
fn batches_containing_native_launches_are_never_uploaded_by_realtime() {
    let now = Instant::now();
    let mut state = fixture(now);
    let mut commands = Commands::new(now);
    for text in [
        "pause media and then open app Editor",
        "open app Editor and then mute computer",
    ] {
        let out = route(&mut commands, &mut state, text, true, now, json!([])).expect("local command");
        assert!(out.desktop.is_empty() && out.effects.is_empty());
        assert!(!out.chat_fallback(true));
    }
}

#[test]
fn realtime_consumes_mixed_flock_batches_without_applying_their_prefix() {
    let now = Instant::now();
    let mut state = fixture(now);
    let before = format!("{state:?}");
    let mut commands = Commands::new(now);
    for text in [
        "mute api and then pause media",
        "mute computer and then open web and then who is waiting",
    ] {
        let out =
            route(&mut commands, &mut state, text, true, now, json!([])).expect("recognized local batch");
        assert_eq!(out.feedback, Feedback::Unavailable);
        assert!(out.desktop.is_empty() && out.effects.is_empty() && !out.chat_fallback(true));
        assert_eq!(format!("{state:?}"), before);
    }
}

#[test]
fn quoted_commands_are_consumed_locally_even_when_commands_are_disabled() {
    let now = Instant::now();
    let mut state = fixture(now);
    let mut commands = Commands::new(now);
    for enabled in [true, false] {
        let out = route(
            &mut commands,
            &mut state,
            "\"pause media\"",
            enabled,
            now,
            json!([]),
        )
        .expect("quoted input stays local");
        assert_eq!(out.feedback, Feedback::Quoted);
        assert!(out.effects.is_empty() && out.desktop.is_empty() && !out.chat_fallback(true));
    }
}
