use super::*;
use serde_json::json;
use vults_core::{AgentEvent, AgentKind, AgentUpdate, Ask, RequestId, Terminal};

fn key(id: &str) -> SessionKey {
    SessionKey {
        agent: AgentKind::Codex,
        session_id: id.into(),
    }
}
fn session(state: &mut State, id: &str, cwd: &str, now: Instant) {
    core::reduce(
        state,
        Input::Agent(AgentUpdate {
            session: key(id),
            cwd: Some(cwd.into()),
            terminal: Terminal::default(),
            agent_id: None,
            event: AgentEvent::SessionStarted,
            received: None,
        }),
        now,
    );
}
pub(super) fn fixture(now: Instant) -> State {
    let mut state = State::default();
    session(&mut state, "a", "/work/api", now);
    session(&mut state, "b", "/work/web", now);
    state
}
fn say(commands: &mut Commands, state: &mut State, text: &str, now: Instant) -> Report {
    commands.execute(state, text, &[], json!([]), now)
}
fn no_answers(effects: &[Effect]) {
    assert!(
        effects.iter().all(|effect| matches!(
            effect,
            Effect::JumpToTerminal(_) | Effect::OpenFolder(_) | Effect::SaveProjects(_)
        )),
        "{effects:?}"
    );
}

#[test]
fn navigation_and_project_preferences_use_only_existing_intents() {
    let now = Instant::now();
    let mut state = fixture(now);
    let mut commands = Commands::new(now);
    assert_eq!(
        say(&mut commands, &mut state, "open web", now).feedback,
        Feedback::Done
    );
    assert_eq!(state.focus, Some(key("b")));
    assert_eq!(
        say(&mut commands, &mut state, "previous", now).feedback,
        Feedback::Done
    );
    assert_eq!(state.focus, Some(key("a")));
    assert_eq!(
        say(&mut commands, &mut state, "next", now).feedback,
        Feedback::Done
    );
    assert_eq!(state.focus, Some(key("b")));
    let jump = say(&mut commands, &mut state, "go to the api terminal", now);
    assert!(matches!(jump.effects.as_slice(), [Effect::JumpToTerminal(_)]));
    let folder = say(&mut commands, &mut state, "open the api folder", now);
    assert!(matches!(folder.effects.as_slice(),[Effect::OpenFolder(path)] if path=="/work/api"));
    for (text, pref) in [
        ("mute api", ProjectPref::Mute),
        ("pin api", ProjectPref::Pin),
        ("hide api", ProjectPref::Hide),
    ] {
        let report = say(&mut commands, &mut state, text, now);
        assert_eq!(report.feedback, Feedback::Done, "{text}: {report:?}");
        no_answers(&report.effects);
        assert!(preference(&state, &key("a"), pref));
    }
    assert_eq!(
        say(&mut commands, &mut state, "unhide api", now).feedback,
        Feedback::Done
    );
    assert!(!state.projects["/work/api"].hide);
}

#[test]
fn an_ambiguous_later_clause_has_no_partial_effect_and_reply_resumes_once() {
    let now = Instant::now();
    let mut state = fixture(now);
    session(&mut state, "c", "/work/api-gateway", now);
    let mut commands = Commands::new(now);
    let question = say(&mut commands, &mut state, "mute web and then open apy", now);
    assert_eq!(question.feedback, Feedback::WhichOne);
    assert_eq!(question.choices.len(), 2);
    assert!(question.effects.is_empty());
    assert!(state.projects.is_empty());
    assert!(state.focus.is_none());
    let answer = say(&mut commands, &mut state, "api", now + Duration::from_secs(90));
    assert_eq!(answer.feedback, Feedback::Done);
    assert!(state.projects["/work/web"].mute);
    assert_eq!(state.focus, Some(key("a")));
    no_answers(&answer.effects);
    assert_eq!(
        say(&mut commands, &mut state, "api", now + Duration::from_secs(90)).feedback,
        Feedback::NotCommand
    );
}

#[test]
fn pending_questions_expire_and_revalidate_all_original_choices() {
    let now = Instant::now();
    for stale in [false, true] {
        let mut state = fixture(now);
        session(&mut state, "c", "/work/api-gateway", now);
        let mut commands = Commands::new(now);
        assert_eq!(
            say(&mut commands, &mut state, "mute web and then open apy", now).feedback,
            Feedback::WhichOne
        );
        if stale {
            state.sessions.remove(&key("c"));
        }
        let report = say(
            &mut commands,
            &mut state,
            "api",
            now + Duration::from_secs(if stale { 1 } else { 91 }),
        );
        assert_eq!(
            report.feedback,
            if stale { Feedback::Stale } else { Feedback::Expired }
        );
        assert!(state.projects.is_empty());
        assert!(report.effects.is_empty());
    }
}

#[test]
fn repeat_and_undo_have_an_inclusive_clock_and_restore_actual_previous_values() {
    let now = Instant::now();
    let mut state = fixture(now);
    let mut commands = Commands::new(now);
    say(&mut commands, &mut state, "mute api", now);
    say(&mut commands, &mut state, "mute api", now);
    assert_eq!(
        say(&mut commands, &mut state, "undo", now).feedback,
        Feedback::Done
    );
    assert!(
        state.projects["/work/api"].mute,
        "Undo of a no-op must not invert the preference"
    );
    assert_eq!(
        say(&mut commands, &mut state, "undo", now).feedback,
        Feedback::NoContext
    );
    say(&mut commands, &mut state, "pin api", now);
    assert_eq!(
        say(
            &mut commands,
            &mut state,
            "and web too",
            now + Duration::from_secs(90)
        )
        .feedback,
        Feedback::Done
    );
    assert!(state.projects["/work/web"].pin);
    assert_eq!(
        say(
            &mut commands,
            &mut state,
            "same for api",
            now + Duration::from_secs(181)
        )
        .feedback,
        Feedback::NoContext
    );
    say(
        &mut commands,
        &mut state,
        "open api",
        now + Duration::from_secs(182),
    );
    say(
        &mut commands,
        &mut state,
        "open web",
        now + Duration::from_secs(183),
    );
    say(&mut commands, &mut state, "undo", now + Duration::from_secs(184));
    assert_eq!(state.focus, Some(key("a")));
}

#[test]
fn undo_does_not_overwrite_intervening_edits_or_follow_a_changed_folder() {
    let now = Instant::now();
    for moved in [false, true] {
        let mut state = fixture(now);
        let mut commands = Commands::new(now);
        say(&mut commands, &mut state, "mute api", now);
        if moved {
            state.sessions.get_mut(&key("a")).expect("Session").cwd = Some("/elsewhere/api".into());
        } else {
            state.projects.get_mut("/work/api").expect("Preference").mute = false;
        }
        let report = say(&mut commands, &mut state, "undo", now + Duration::from_secs(1));
        assert_eq!(report.feedback, Feedback::CannotUndo);
        assert!(report.effects.is_empty());
    }
}

#[test]
fn same_project_sessions_share_preferences_but_navigation_requires_a_choice() {
    let now = Instant::now();
    let mut state = fixture(now);
    session(&mut state, "c", "/work/api", now);
    let mut commands = Commands::new(now);
    assert_eq!(
        say(&mut commands, &mut state, "mute api", now).feedback,
        Feedback::Done
    );
    let question = say(&mut commands, &mut state, "open api", now);
    assert_eq!(question.feedback, Feedback::WhichOne);
    assert_eq!(question.choices.len(), 2);
    assert_eq!(say(&mut commands, &mut state, "c", now).feedback, Feedback::Done);
    assert_eq!(state.focus, Some(key("c")));
}

#[test]
fn saved_aliases_keep_their_priority_and_only_target_known_names() {
    let now = Instant::now();
    let mut state = fixture(now);
    let mut commands = Commands::new(now);
    let aliases = vec![
        ("backend".into(), "api".into()),
        ("web".into(), "api".into()),
        ("danger".into(), "approve".into()),
    ];
    assert_eq!(
        commands
            .execute(&mut state, "open backend", &aliases, json!([]), now)
            .feedback,
        Feedback::Done
    );
    assert_eq!(state.focus, Some(key("a")));
    say(&mut commands, &mut state, "open web", now);
    commands.execute(&mut state, "open web", &aliases, json!([]), now);
    assert_eq!(state.focus, Some(key("a")));
    assert_eq!(
        commands
            .execute(&mut state, "open danger", &aliases, json!([]), now)
            .feedback,
        Feedback::UnknownName
    );
}

#[test]
fn reads_are_local_redacted_templates_in_all_languages_and_use_exact_session_keys() {
    let now = Instant::now();
    let mut state = fixture(now);
    session(&mut state, "c", "/elsewhere/api", now);
    state.sessions.get_mut(&key("a")).expect("Session").project = "api".into();
    let mut commands = Commands::new(now);
    for lang in [
        core::i18n::Lang::En,
        core::i18n::Lang::PtBr,
        core::i18n::Lang::Es,
        core::i18n::Lang::Zh,
    ] {
        state.lang = lang;
        let question = say(&mut commands, &mut state, "what is api doing", now);
        assert_eq!(question.feedback, Feedback::WhichOne);
        let report = say(&mut commands, &mut state, "a", now);
        assert_eq!(report.feedback, Feedback::Answer, "{report:?}");
        assert!(!report.text.is_empty());
        assert!(report.effects.is_empty());
        let usage = commands.execute(
            &mut state,
            "what is my usage",
            &[],
            json!([{"agent":"codex","minutes":300,"used_percent":42,"resets_at":null}]),
            now,
        );
        assert_eq!(usage.feedback, Feedback::Answer);
        assert!(usage.text.contains("42"));
        assert_eq!(
            say(&mut commands, &mut state, "who is waiting", now).feedback,
            Feedback::Answer
        );
        assert_eq!(
            say(&mut commands, &mut state, "how are my pull requests", now).feedback,
            Feedback::Unavailable
        );
    }
    state.boards.insert("github".into(), vec![]);
    assert_eq!(
        say(&mut commands, &mut state, "how are my pull requests", now).feedback,
        Feedback::Answer
    );
}

#[test]
fn all_transcripts_leave_waiting_cards_unanswered_and_unknown_batches_untouched() {
    let now = Instant::now();
    let mut state = fixture(now);
    core::reduce(
        &mut state,
        Input::Agent(AgentUpdate {
            session: key("a"),
            cwd: None,
            terminal: Terminal::default(),
            agent_id: None,
            event: AgentEvent::PermissionRequested {
                request: RequestId("permission".into()),
                tool: "Bash".into(),
                target: "TOKEN=secret".into(),
                ask: Ask::default(),
            },
            received: None,
        }),
        now,
    );
    core::reduce(
        &mut state,
        Input::Agent(AgentUpdate {
            session: key("b"),
            cwd: None,
            terminal: Terminal::default(),
            agent_id: None,
            event: AgentEvent::QuestionAsked {
                request: RequestId("question".into()),
                target: "AskUserQuestion".into(),
                questions: vec![core::Question {
                    question: "Continue?".into(),
                    header: "Choice".into(),
                    options: vec![],
                    multi: false,
                }],
            },
            received: None,
        }),
        now,
    );
    let mut commands = Commands::new(now);
    for text in [
        "approve",
        "yes",
        "allow",
        "release",
        "next and then approve",
        "mute api and then write a poem",
    ] {
        let before = state.projects.clone();
        let report = say(&mut commands, &mut state, text, now);
        no_answers(&report.effects);
        assert_eq!(state.pending.len(), 2);
        assert_eq!(before, state.projects);
    }
    let mut seed = 7u64;
    let tokens = [
        "open",
        "api",
        "web",
        "mute",
        "pin",
        "undo",
        "next",
        "approve",
        "yes",
        "allow",
        "and then",
        "who is waiting",
        "release",
        "\0",
        "go to the api terminal",
    ];
    let (mut dispatched, mut refused) = (0, 0);
    for _ in 0..2000 {
        let mut words = Vec::new();
        let length = 1 + (seed as usize % 3);
        for _ in 0..length {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            words.push(tokens[(seed as usize >> 16) % tokens.len()]);
        }
        let report = say(&mut commands, &mut state, &words.join(" "), now);
        dispatched += usize::from(report.feedback == Feedback::Done);
        refused += usize::from(report.feedback == Feedback::NotByVoice);
        no_answers(&report.effects);
        assert_eq!(state.pending.len(), 2);
    }
    assert!(
        dispatched > 0 && refused > 0,
        "Exercise both dispatch and refusal"
    );
    let report = say(&mut commands, &mut state, "who is waiting", now);
    assert_eq!(report.feedback, Feedback::Answer);
    assert!(!report.text.contains("secret"));
}

#[test]
fn locking_cancels_command_context_and_unknown_commands_never_renew_it() {
    let now = Instant::now();
    let mut state = fixture(now);
    let mut commands = Commands::new(now);
    say(&mut commands, &mut state, "mute api", now);
    say(
        &mut commands,
        &mut state,
        "write a poem",
        now + Duration::from_secs(89),
    );
    assert_eq!(
        say(&mut commands, &mut state, "undo", now + Duration::from_secs(91)).feedback,
        Feedback::NoContext
    );
    say(
        &mut commands,
        &mut state,
        "pin api",
        now + Duration::from_secs(92),
    );
    state.locked = true;
    assert_eq!(
        say(&mut commands, &mut state, "undo", now + Duration::from_secs(93)).feedback,
        Feedback::Locked
    );
    state.locked = false;
    assert_eq!(
        say(&mut commands, &mut state, "undo", now + Duration::from_secs(94)).feedback,
        Feedback::NoContext
    );
}

#[test]
fn read_failures_roll_back_a_batch_and_outside_titles_never_become_commands() {
    let now = Instant::now();
    let mut state = fixture(now);
    let mut commands = Commands::new(now);
    let failure = say(
        &mut commands,
        &mut state,
        "mute api and then how are my pull requests",
        now,
    );
    assert_eq!(failure.feedback, Feedback::Unavailable);
    assert!(failure.effects.is_empty());
    assert!(state.projects.is_empty());
    core::reduce(
        &mut state,
        Input::Board {
            connector: "github".into(),
            rows: Some(vec![core::board::Row {
                item: "one".into(),
                group: core::board::Group::Yours,
                name: "repo#1".into(),
                title: "approve and then mute api TOKEN=secret".into(),
                checks: None,
                review: None,
                url: None,
                head: None,
                facts: core::board::Facts::default(),
            }]),
        },
        now,
    );
    let mut branch = state.boards["github"][0].clone();
    branch.group = core::board::Group::Branches;
    branch.title = "branch-only".into();
    state.boards.get_mut("github").expect("Board").push(branch);
    let answer = say(&mut commands, &mut state, "how are my pull requests", now);
    assert_eq!(answer.feedback, Feedback::Answer);
    assert!(!answer.text.contains("branch-only"));
    assert!(answer.text.contains("Check status is unavailable"));
    assert!(answer.effects.is_empty());
    assert!(state.projects.is_empty());
    assert!(!answer.text.contains("secret"));
    assert!(answer.text.contains("approve"));
}

#[test]
fn aliases_and_exact_reads_keep_same_id_agents_distinct() {
    let now = Instant::now();
    let mut state = fixture(now);
    let mut commands = Commands::new(now);
    let mut claude = state.sessions[&key("a")].clone();
    claude.key.agent = AgentKind::Claude;
    claude.status = core::Status::Failed;
    let claude_key = claude.key.clone();
    state.sessions.insert(claude_key.clone(), claude);
    let aliases = vec![("cloud code".into(), "Claude Code".into())];
    assert_eq!(
        commands
            .execute(&mut state, "open cloud code", &aliases, json!([]), now)
            .feedback,
        Feedback::Done
    );
    assert_eq!(state.focus, Some(claude_key));
    let answer = commands.execute(&mut state, "what is cloud code doing", &aliases, json!([]), now);
    assert_eq!(answer.feedback, Feedback::Answer);
    assert!(answer.text.contains("failed"));
    let codex = say(&mut commands, &mut state, "what is api doing", now);
    assert_eq!(codex.feedback, Feedback::WhichOne);
    let answer = say(&mut commands, &mut state, "api codex", now);
    assert_eq!(answer.feedback, Feedback::Answer);
    assert!(!answer.text.contains("failed"));
}

#[test]
fn a_pending_batch_never_rebinds_an_earlier_target_after_a_session_moves() {
    let now = Instant::now();
    let mut state = fixture(now);
    session(&mut state, "c", "/work/api-gateway", now);
    let mut commands = Commands::new(now);
    assert_eq!(
        say(&mut commands, &mut state, "mute web and then open apy", now).feedback,
        Feedback::WhichOne
    );
    state.sessions.get_mut(&key("b")).expect("Session").cwd = Some("/new/web".into());
    let reply = say(&mut commands, &mut state, "api", now + Duration::from_secs(1));
    assert_eq!(reply.feedback, Feedback::Stale);
    assert!(reply.effects.is_empty());
    assert!(state.projects.is_empty());
}

#[test]
fn only_unrecognized_speech_can_opt_into_chat_fallback() {
    let now = Instant::now();
    let mut state = fixture(now);
    let mut commands = Commands::new(now);
    for text in [
        "approve",
        "allow",
        "yes",
        "next and then approve",
        "open missing",
        "undo",
        "next session",
    ] {
        let report = say(&mut commands, &mut state, text, now);
        assert!(!report.chat_fallback(true), "{text}");
    }
    let unknown = say(&mut commands, &mut state, "tell me a story", now);
    assert!(!unknown.chat_fallback(false));
    assert!(unknown.chat_fallback(true));
    session(&mut state, "c", "/work/api-gateway", now);
    let asked = say(&mut commands, &mut state, "open apy", now);
    assert!(!asked.chat_fallback(true));
    let unknown_reply = say(&mut commands, &mut state, "tell me a story", now);
    assert!(
        !unknown_reply.chat_fallback(true),
        "An unresolved clarification cannot become a model request"
    );
    let expired = say(&mut commands, &mut state, "api", now + Duration::from_secs(91));
    assert!(!expired.chat_fallback(true));
}

#[test]
fn playback_and_flock_batches_only_plan_before_native_admission() {
    use vults_dictation::command::MediaAction;
    let now = Instant::now();
    let mut state = fixture(now);
    let mut commands = Commands::new(now);
    let report = say(&mut commands, &mut state, "pause media", now);
    assert_eq!(report.feedback, Feedback::Requested);
    assert_eq!(report.desktop, vec![DesktopAction::Media(MediaAction::Pause)]);
    assert!(report.effects.is_empty());
    assert!(!report.chat_fallback(true));
    let before = format!("{state:?}");
    for text in ["mute web and then pause media", "pause media and then open api"] {
        let report = say(&mut commands, &mut state, text, now);
        assert_eq!(report.feedback, Feedback::Requested);
        assert!(report.effects.is_empty() && report.desktop.len() == 2);
        assert!(!report.chat_fallback(true));
        assert_eq!(format!("{state:?}"), before);
    }
    state.locked = true;
    let report = say(&mut commands, &mut state, "pause media", now);
    assert_eq!(report.feedback, Feedback::Locked);
    assert!(report.desktop.is_empty());
}

pub(super) fn installed_app(id: &str, name: &str) -> vults_platform::applications::Application {
    vults_platform::applications::Application {
        id: id.into(),
        name: name.into(),
        aliases: vec![],
        source: format!("/fixtures/{id}").into(),
        definition: std::sync::Arc::from(&b"original"[..]),
    }
}

#[test]
fn installed_apps_and_live_projects_share_open_but_keep_explicit_namespaces() {
    let now = Instant::now();
    let mut state = fixture(now);
    let mut commands = Commands::new(now);
    let apps = [
        installed_app("notes.desktop", "Obsidian"),
        installed_app("web.desktop", "web"),
    ];
    let out =
        commands.execute_with_applications(&mut state, "open Obsidian", &[], json!([]), now, Some(&apps));
    assert_eq!(out.feedback, Feedback::Requested);
    assert_eq!(application_id(&out).unwrap(), "notes.desktop");
    assert!(out.effects.is_empty());
    let out = commands.execute_with_applications(&mut state, "open web", &[], json!([]), now, Some(&apps));
    assert_eq!(out.feedback, Feedback::WhichOne);
    assert!(application_id(&out).is_none() && out.effects.is_empty());
    assert_eq!(out.choices.len(), 2);
    let out =
        commands.execute_with_applications(&mut state, "open app web", &[], json!([]), now, Some(&apps));
    assert_eq!(application_id(&out).unwrap(), "web.desktop");
    let out =
        commands.execute_with_applications(&mut state, "open project web", &[], json!([]), now, Some(&apps));
    assert_eq!(out.feedback, Feedback::Done);
    assert!(application_id(&out).is_none());
    assert_eq!(state.focus, Some(key("b")));
    let out = commands.execute_with_applications(
        &mut state,
        "open a session Obsidian",
        &[],
        json!([]),
        now,
        Some(&apps),
    );
    assert_eq!(out.feedback, Feedback::UnknownName);
    assert!(application_id(&out).is_none());
    let out = commands.execute_with_applications(&mut state, "open app Obsidian", &[], json!([]), now, None);
    assert_eq!(out.feedback, Feedback::Unavailable);
    assert!(application_id(&out).is_none() && !out.chat_fallback(true));
    assert!(out.text.contains("Enable application"));
}

#[test]
fn app_clarification_preserves_original_definitions_and_never_applies_a_batch_prefix() {
    let now = Instant::now();
    let mut state = fixture(now);
    let mut commands = Commands::new(now);
    let apps = [
        installed_app("one.desktop", "Editor"),
        installed_app("two.desktop", "Editor"),
    ];
    let out =
        commands.execute_with_applications(&mut state, "open app Editor", &[], json!([]), now, Some(&apps));
    assert_eq!(out.feedback, Feedback::WhichOne);
    let mut changed = apps.clone();
    changed[1].definition = std::sync::Arc::from(&b"changed"[..]);
    let out = commands.execute_with_applications(
        &mut state,
        "Editor / one.desktop",
        &[],
        json!([]),
        now,
        Some(&changed),
    );
    assert_eq!(out.feedback, Feedback::Stale);
    assert!(application_id(&out).is_none());
    commands.execute_with_applications(&mut state, "open app Editor", &[], json!([]), now, Some(&apps));
    let out = commands.execute_with_applications(
        &mut state,
        "Editor / two.desktop",
        &[],
        json!([]),
        now,
        Some(&apps),
    );
    assert_eq!(application_id(&out).unwrap(), "two.desktop");
    let before = format!("{state:?}");
    for text in [
        "mute api and then open app Editor",
        "open app Editor and then mute api",
    ] {
        let single = [installed_app("one.desktop", "Editor")];
        let out = commands.execute_with_applications(&mut state, text, &[], json!([]), now, Some(&single));
        assert_eq!(out.feedback, Feedback::Requested);
        assert!(out.desktop.len() == 2 && out.effects.is_empty());
        assert_eq!(format!("{state:?}"), before);
    }
    let aliases = [("notes".into(), "Editor".into())];
    let one = [apps[0].clone()];
    let out =
        commands.execute_with_applications(&mut state, "open notes", &aliases, json!([]), now, Some(&one));
    assert_eq!(application_id(&out).unwrap(), "one.desktop");
    state.locked = true;
    let out =
        commands.execute_with_applications(&mut state, "open app Editor", &[], json!([]), now, Some(&one));
    assert_eq!(out.feedback, Feedback::Locked);
    assert!(application_id(&out).is_none());
}

#[test]
fn computer_audio_is_only_requested_and_mixed_batches_do_not_change_project_state() {
    use vults_dictation::command::OutputAction;
    let now = Instant::now();
    let mut state = fixture(now);
    let mut commands = Commands::new(now);
    let out = say(&mut commands, &mut state, "mute computer", now);
    assert_eq!(out.feedback, Feedback::Requested);
    assert_eq!(out.desktop, vec![DesktopAction::Output(OutputAction::Mute(true))]);
    assert!(out.effects.is_empty() && application_id(&out).is_none());
    assert!(!out.chat_fallback(true));
    let before = format!("{state:?}");
    for text in [
        "mute web and then mute computer",
        "mute computer and then mute web",
    ] {
        let out = say(&mut commands, &mut state, text, now);
        assert_eq!(out.feedback, Feedback::Requested);
        assert!(out.effects.is_empty() && out.desktop.len() == 2);
        assert_eq!(format!("{state:?}"), before);
    }
    state.locked = true;
    let out = say(&mut commands, &mut state, "mute computer", now);
    assert_eq!(out.feedback, Feedback::Locked);
    assert!(out.desktop.is_empty());
}

#[test]
fn invalid_native_volume_is_never_sent_to_chat_or_executed() {
    let now = Instant::now();
    let mut state = fixture(now);
    let mut commands = Commands::new(now);
    for text in [
        "media volume -10",
        "computer volume 101",
        "pause media and then computer volume 200",
        "consegue set music volume to -1",
        "quero que voce ajuste o volume do computador para 101",
        "pon el volumen de la musica a 200",
    ] {
        let out = say(&mut commands, &mut state, text, now);
        assert_eq!(out.feedback, Feedback::InvalidVolume);
        assert!(!out.chat_fallback(true));
        assert!(out.effects.is_empty() && out.desktop.is_empty());
    }
}

#[test]
fn mixed_audio_batches_preserve_order_without_mutating_flock_or_falling_back() {
    use vults_dictation::command::{MediaAction, OutputAction};
    let now = Instant::now();
    let mut state = fixture(now);
    let mut commands = Commands::new(now);
    let before = format!("{state:?}");
    let out = say(
        &mut commands,
        &mut state,
        "pause media and then computer volume 40 and then resume music and then mute computer",
        now,
    );
    assert_eq!(out.feedback, Feedback::Requested);
    assert_eq!(
        out.desktop,
        vec![
            DesktopAction::Media(MediaAction::Pause),
            DesktopAction::Output(OutputAction::Volume(40)),
            DesktopAction::Media(MediaAction::Resume),
            DesktopAction::Output(OutputAction::Mute(true)),
        ]
    );
    assert!(out.effects.is_empty() && application_id(&out).is_none());
    assert!(!out.chat_fallback(true));
    assert_eq!(format!("{state:?}"), before);
    for text in [
        "mute computer and then pause media and then approve all cards",
        "pause media and then computer volume 101",
        "mute computer and then tell me a joke",
    ] {
        let out = say(&mut commands, &mut state, text, now);
        assert!(out.desktop.is_empty() && out.effects.is_empty());
        assert_eq!(format!("{state:?}"), before);
    }
    let out = say(&mut commands, &mut state, "repeat", now);
    assert!(
        out.desktop.is_empty(),
        "native batches never enter repeat context"
    );
}

fn application_id(report: &Report) -> Option<&str> {
    report.desktop.iter().find_map(|action| match action {
        DesktopAction::Application(app) => Some(app.id.as_str()),
        _ => None,
    })
}

#[test]
fn native_app_batches_resolve_every_target_before_admission_and_keep_spoken_order() {
    use vults_dictation::command::{MediaAction, OutputAction};
    let now = Instant::now();
    let mut state = fixture(now);
    let before = format!("{state:?}");
    let mut commands = Commands::new(now);
    let apps = [
        installed_app("notes.desktop", "Obsidian"),
        installed_app("web.desktop", "web"),
    ];
    let out = commands.execute_with_applications(
        &mut state,
        "pause media and then open app Obsidian and then computer volume 40 and then open app web",
        &[],
        json!([]),
        now,
        Some(&apps),
    );
    assert_eq!(out.feedback, Feedback::Requested);
    assert_eq!(
        out.desktop,
        vec![
            DesktopAction::Media(MediaAction::Pause),
            DesktopAction::Application(apps[0].clone()),
            DesktopAction::Output(OutputAction::Volume(40)),
            DesktopAction::Application(apps[1].clone())
        ]
    );
    assert!(out.effects.is_empty() && !out.chat_fallback(true));
    assert_eq!(format!("{state:?}"), before);
    let out = commands.execute_with_applications(
        &mut state,
        "open app Obsidian and then open app web",
        &[],
        json!([]),
        now,
        Some(&apps),
    );
    assert_eq!(
        out.desktop,
        vec![
            DesktopAction::Application(apps[0].clone()),
            DesktopAction::Application(apps[1].clone())
        ]
    );
    for text in [
        "pause media and then open app Missing",
        "open app Obsidian and then approve all cards",
    ] {
        let out = commands.execute_with_applications(&mut state, text, &[], json!([]), now, Some(&apps));
        assert!(out.desktop.is_empty() && out.effects.is_empty(), "{text}");
        assert!(!out.chat_fallback(true));
        assert_eq!(format!("{state:?}"), before);
    }
    let out = commands.execute_with_applications(
        &mut state,
        "pause media and then open app Obsidian",
        &[],
        json!([]),
        now,
        None,
    );
    assert_eq!(out.feedback, Feedback::Unavailable);
    assert!(out.desktop.is_empty() && !out.chat_fallback(true));
    let out = commands.execute_with_applications(
        &mut state,
        "pause media and then open web",
        &[],
        json!([]),
        now,
        Some(&apps),
    );
    assert_eq!(out.feedback, Feedback::WhichOne);
    assert!(out.desktop.is_empty() && out.effects.is_empty());
    let choice = out
        .choices
        .iter()
        .find(|name| name.kind == command::names::NameKind::Application)
        .unwrap()
        .name
        .clone();
    let out = commands.execute_with_applications(&mut state, &choice, &[], json!([]), now, Some(&apps));
    assert_eq!(
        out.desktop,
        vec![
            DesktopAction::Media(MediaAction::Pause),
            DesktopAction::Application(apps[1].clone())
        ]
    );
    assert_eq!(format!("{state:?}"), before);
}

#[test]
fn later_app_questions_preserve_earlier_definitions_and_the_original_deadline() {
    let now = Instant::now();
    let mut state = fixture(now);
    let before = format!("{state:?}");
    let apps = [
        installed_app("one.desktop", "Editor"),
        installed_app("two.desktop", "Editor"),
        installed_app("three.desktop", "Viewer"),
        installed_app("four.desktop", "Viewer"),
    ];
    for stale in [false, true] {
        let mut commands = Commands::new(now);
        let out = commands.execute_with_applications(
            &mut state,
            "open app Editor and then open app Viewer",
            &[],
            json!([]),
            now,
            Some(&apps),
        );
        assert_eq!(out.feedback, Feedback::WhichOne);
        assert!(out.desktop.is_empty());
        let out = commands.execute_with_applications(
            &mut state,
            "Editor / one.desktop",
            &[],
            json!([]),
            now + Duration::from_secs(80),
            Some(&apps),
        );
        assert_eq!(out.feedback, Feedback::WhichOne);
        assert!(out.desktop.is_empty());
        #[cfg(target_os = "linux")]
        assert_eq!(commands.question_deadline(), Some(now + Duration::from_secs(90)));
        let mut current = apps.clone();
        if stale {
            current[0].definition = std::sync::Arc::from(&b"changed"[..]);
        }
        let out = commands.execute_with_applications(
            &mut state,
            "Viewer / four.desktop",
            &[],
            json!([]),
            now + Duration::from_secs(if stale { 85 } else { 91 }),
            Some(&current),
        );
        assert_eq!(
            out.feedback,
            if stale { Feedback::Stale } else { Feedback::Expired }
        );
        assert!(out.desktop.is_empty() && out.effects.is_empty() && !out.chat_fallback(true));
        assert_eq!(format!("{state:?}"), before);
    }
    let mut commands = Commands::new(now);
    commands.execute_with_applications(
        &mut state,
        "open app Editor and then open app Viewer",
        &[],
        json!([]),
        now,
        Some(&apps),
    );
    commands.execute_with_applications(
        &mut state,
        "Editor / one.desktop",
        &[],
        json!([]),
        now + Duration::from_secs(30),
        Some(&apps),
    );
    let out = commands.execute_with_applications(
        &mut state,
        "Viewer / four.desktop",
        &[],
        json!([]),
        now + Duration::from_secs(60),
        Some(&apps),
    );
    assert_eq!(
        out.desktop,
        vec![
            DesktopAction::Application(apps[0].clone()),
            DesktopAction::Application(apps[3].clone())
        ]
    );
    assert_eq!(format!("{state:?}"), before);
}

#[test]
fn flock_and_native_batches_prepare_all_steps_without_changing_state() {
    let now = Instant::now();
    let mut state = fixture(now);
    let before = format!("{state:?}");
    let mut commands = Commands::new(now);
    let out = say(
        &mut commands,
        &mut state,
        "mute api and then pause media and then open web and then computer volume 40 and then who is waiting",
        now,
    );
    assert_eq!(out.feedback, Feedback::Requested);
    assert_eq!(out.desktop.len(), 5);
    assert!(out.effects.is_empty() && !out.chat_fallback(true));
    assert_eq!(format!("{state:?}"), before);
    let mut answers = vec![];
    for action in &out.desktop {
        if let DesktopAction::Flock(step) = action {
            let (effects, text) = step.apply(&mut state, json!([]), now).unwrap();
            assert!(
                effects
                    .iter()
                    .all(|effect| matches!(effect, Effect::SaveProjects(_)))
            );
            if !text.is_empty() {
                answers.push(text);
            }
        }
    }
    assert!(state.prefs(&state.sessions[&key("a")]).mute);
    assert_eq!(state.focus, Some(key("b")));
    assert!(!answers.is_empty());
    for text in ["pause media and then repeat", "pause media and then undo"] {
        let before = format!("{state:?}");
        let out = say(&mut commands, &mut state, text, now);
        assert!(out.desktop.is_empty() && out.effects.is_empty(), "{text}");
        assert_eq!(format!("{state:?}"), before);
    }
}

#[test]
fn terminal_and_folder_steps_mix_in_order_without_running_external_effects_during_planning() {
    let now = Instant::now();
    let mut state = fixture(now);
    let before = format!("{state:?}");
    let mut commands = Commands::new(now);
    let out = say(
        &mut commands,
        &mut state,
        "pause media and then open terminal api and then open folder web and then mute computer",
        now,
    );
    assert_eq!(out.feedback, Feedback::Requested);
    assert_eq!(out.desktop.len(), 4);
    assert!(out.effects.is_empty() && !out.chat_fallback(true));
    assert_eq!(format!("{state:?}"), before);
    let DesktopAction::Flock(terminal) = &out.desktop[1] else {
        panic!("terminal step");
    };
    let (effects, _) = terminal.apply(&mut state, json!([]), now).unwrap();
    assert!(matches!(effects.as_slice(), [Effect::JumpToTerminal(_)]));
    let DesktopAction::Flock(folder) = &out.desktop[2] else {
        panic!("folder step");
    };
    let (effects, _) = folder.apply(&mut state, json!([]), now).unwrap();
    assert!(matches!(effects.as_slice(), [Effect::OpenFolder(path)] if path == "/work/web"));
}

#[test]
fn command_key_external_only_batches_use_native_receipts_while_legacy_flock_dispatch_is_preserved() {
    let now = Instant::now();
    let mut state = fixture(now);
    let mut commands = Commands::new(now);
    for text in ["open folder api", "open terminal api and then open folder web"] {
        let before = format!("{state:?}");
        let out = commands.execute_with_applications(&mut state, text, &[], json!([]), now, None);
        assert_eq!(out.feedback, Feedback::Requested);
        assert!(!out.desktop.is_empty() && out.effects.is_empty() && !out.chat_fallback(true));
        assert_eq!(format!("{state:?}"), before);
    }
    let out = say(&mut commands, &mut state, "open folder api", now);
    assert!(out.desktop.is_empty());
    assert!(matches!(out.effects.as_slice(), [Effect::OpenFolder(_)]));
}

#[test]
fn quoted_commands_do_not_execute_or_enter_optional_chat_fallback() {
    let now = Instant::now();
    let mut state = fixture(now);
    let before = format!("{state:?}");
    let mut commands = Commands::new(now);
    for text in ["\"mute api\"", "'pause media'", "open app `Editor`"] {
        let out = say(&mut commands, &mut state, text, now);
        assert_eq!(out.feedback, Feedback::Quoted);
        assert!(out.effects.is_empty() && out.desktop.is_empty() && !out.chat_fallback(true));
        assert_eq!(format!("{state:?}"), before);
    }
}

#[test]
fn request_variations_plan_native_actions_without_chat_or_premature_effects() {
    use vults_dictation::command::{MediaAction, OutputAction};
    let now = Instant::now();
    let mut state = fixture(now);
    let mut commands = Commands::new(now);
    let before = format!("{state:?}");
    for text in [
        "consegue aumentar o volume da musica e depois ajuste o volume do computador para 35",
        "i would like you to increase music volume and then set system volume to 35",
        "puedes subir el volumen de la musica y luego pon el volumen del ordenador a 35",
    ] {
        let report = say(&mut commands, &mut state, text, now);
        assert_eq!(report.feedback, Feedback::Requested, "{text}");
        assert_eq!(
            report.desktop,
            vec![
                DesktopAction::Media(MediaAction::Louder),
                DesktopAction::Output(OutputAction::Volume(35)),
            ]
        );
        assert!(report.effects.is_empty() && !report.chat_fallback(true));
        assert_eq!(format!("{state:?}"), before);
    }
    state.locked = true;
    let report = say(
        &mut commands,
        &mut state,
        "consegue aumentar o volume da musica",
        now,
    );
    assert_eq!(report.feedback, Feedback::Locked);
    assert!(report.desktop.is_empty() && report.effects.is_empty() && !report.chat_fallback(true));
}

#[test]
fn clarification_never_rebinds_a_restarted_session_or_changed_terminal() {
    let now = Instant::now();
    for (target, restart, request) in [
        ("b", true, "open the web terminal and then open apy"),
        ("a", true, "open the web terminal and then open apy"),
        ("c", true, "open the web terminal and then open apy"),
        ("b", false, "open the web terminal and then open apy"),
        ("a", false, "open the apy terminal"),
    ] {
        let mut state = fixture(now);
        session(&mut state, "c", "/work/api-gateway", now);
        let mut commands = Commands::new(now);
        let asked = say(&mut commands, &mut state, request, now);
        assert_eq!(asked.feedback, Feedback::WhichOne);
        assert!(asked.effects.is_empty());
        let changed = state.sessions.get_mut(&key(target)).unwrap();
        if restart {
            changed.started += Duration::from_secs(1);
        } else {
            changed.terminal.pid = Some(99);
        }
        let before = format!("{state:?}");
        let reply = say(&mut commands, &mut state, "api", now + Duration::from_secs(2));
        assert_eq!(
            reply.feedback,
            Feedback::Stale,
            "target={target} restart={restart}"
        );
        assert!(reply.effects.is_empty() && reply.desktop.is_empty() && !reply.chat_fallback(true));
        assert_eq!(format!("{state:?}"), before);
    }
}

#[test]
fn clarification_keeps_ordinary_progress_and_unrelated_sessions_valid() {
    let now = Instant::now();
    let mut state = fixture(now);
    session(&mut state, "c", "/work/api-gateway", now);
    let mut commands = Commands::new(now);
    assert_eq!(
        say(
            &mut commands,
            &mut state,
            "open the web terminal and then open apy",
            now
        )
        .feedback,
        Feedback::WhichOne
    );
    for value in state.sessions.values_mut() {
        value.updated += Duration::from_secs(1);
        value.step_count += 1;
    }
    session(
        &mut state,
        "unrelated",
        "/work/docs",
        now + Duration::from_secs(1),
    );
    let reply = say(&mut commands, &mut state, "api", now + Duration::from_secs(2));
    assert_eq!(reply.feedback, Feedback::Done);
    assert_eq!(state.focus, Some(key("a")));
    no_answers(&reply.effects);
    assert!(!reply.chat_fallback(true));
}

#[test]
fn unclear_audio_and_extra_media_targets_stay_local_without_a_batch_prefix() {
    let now = Instant::now();
    let mut state = fixture(now);
    let before = format!("{state:?}");
    let mut commands = Commands::new(now);
    for text in [
        "lower volume",
        "abaixa o volume",
        "baja el volumen",
        "volume 30",
        "pause music in Firefox",
        "pare o que estou escutando no Firefox",
        "pausa la musica en Firefox",
        "mute web and then lower volume",
        "mute web and then pause music in Firefox",
    ] {
        let out = say(&mut commands, &mut state, text, now);
        assert_eq!(
            out.feedback,
            if text.contains("Firefox") {
                Feedback::UnsupportedMediaTarget
            } else {
                Feedback::AudioScopeRequired
            }
        );
        assert!(!out.text.is_empty() && !out.chat_fallback(true), "{text}");
        assert!(out.effects.is_empty() && out.desktop.is_empty(), "{text}");
        assert_eq!(format!("{state:?}"), before);
    }
}

#[test]
fn audio_scope_feedback_clears_pending_choices_and_is_available_in_every_language() {
    let now = Instant::now();
    for lang in [i18n::Lang::En, i18n::Lang::PtBr, i18n::Lang::Es, i18n::Lang::Zh] {
        for text in ["lower volume", "pause music in Firefox"] {
            let mut state = fixture(now);
            state.lang = lang;
            session(&mut state, "c", "/work/api-gateway", now);
            let mut commands = Commands::new(now);
            assert_eq!(
                say(&mut commands, &mut state, "open apy", now).feedback,
                Feedback::WhichOne
            );
            let report = say(&mut commands, &mut state, text, now);
            assert_eq!(report.text, i18n::voice_feedback(lang, report.feedback));
            assert!(report.choices.is_empty() && !report.chat_fallback(true));
            assert!(commands.pending.is_none());
            assert_eq!(
                say(&mut commands, &mut state, "api", now).feedback,
                Feedback::NotCommand
            );
            assert!(state.focus.is_none());
        }
    }
}
