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
fn fixture(now: Instant) -> State {
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
