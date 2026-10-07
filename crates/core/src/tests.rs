use super::*;

fn key(id: &str) -> SessionKey {
    SessionKey {
        agent: AgentKind::Claude,
        session_id: id.into(),
    }
}

fn agent(session: &str, event: AgentEvent) -> Input {
    Input::Agent(AgentUpdate {
        session: key(session),
        cwd: Some("/home/me/vultures-ai".into()),
        terminal: Terminal {
            pid: Some(42),
            ..Default::default()
        },
        agent_id: None,
        event,
    })
}

/// The same, from one of the session's subagents.
fn from_subagent(session: &str, sub: &str, event: AgentEvent) -> Input {
    let Input::Agent(mut u) = agent(session, event) else {
        unreachable!()
    };
    u.agent_id = Some(sub.into());
    Input::Agent(u)
}

fn requested(session: &str, id: &str) -> Input {
    agent(
        session,
        AgentEvent::PermissionRequested {
            request: RequestId(id.into()),
            tool: "Bash".into(),
            target: "Bash · cargo test".into(),
            ask: Ask::default(),
        },
    )
}

fn decide(id: &str, decision: Decision) -> Input {
    Input::User(Intent::Decide {
        request: RequestId(id.into()),
        decision,
    })
}

fn rid(id: &str) -> RequestId {
    RequestId(id.into())
}

#[test]
fn a_request_is_acked_and_answered_once() {
    let mut s = State::default();
    let now = Instant::now();
    assert_eq!(
        reduce(&mut s, requested("a", "r1"), now),
        vec![Effect::AckPermission(rid("r1"))]
    );
    assert_eq!(s.sessions[&key("a")].status, Status::Approval);
    assert_eq!(
        reduce(&mut s, decide("r1", Decision::Allow), now),
        vec![Effect::RespondPermission {
            request: rid("r1"),
            decision: Decision::Allow
        }]
    );
    assert_eq!(s.sessions[&key("a")].status, Status::Working);
    assert!(reduce(&mut s, decide("r1", Decision::Deny), now).is_empty());
}

#[test]
fn unknown_or_stale_requests_are_never_answered() {
    let mut s = State::default();
    let now = Instant::now();
    assert!(reduce(&mut s, decide("ghost", Decision::Allow), now).is_empty());
    reduce(&mut s, requested("a", "r1"), now);
    assert!(reduce(&mut s, decide("other", Decision::Allow), now).is_empty());
    assert!(
        !s.pending.is_empty(),
        "a click on another id must not drop the real card"
    );
}

#[test]
fn a_second_request_waits_its_turn() {
    let mut s = State::default();
    let now = Instant::now();
    reduce(&mut s, requested("a", "r1"), now);
    assert_eq!(
        reduce(&mut s, requested("b", "r2"), now),
        vec![Effect::AckPermission(rid("r2"))]
    );
    let view = s.view().approval.unwrap();
    assert_eq!((view.request.as_str(), view.queue), ("r1", 2));
    assert_eq!(s.sessions[&key("b")].status, Status::Approval);

    reduce(&mut s, decide("r1", Decision::Allow), now);
    assert_eq!(s.sessions[&key("a")].status, Status::Working);
    let view = s.view().approval.unwrap();
    assert_eq!(
        (view.request.as_str(), view.session.as_str(), view.queue),
        ("r2", "b", 1)
    );
    // The one behind can be answered too, but only by its own id.
    assert!(reduce(&mut s, decide("r1", Decision::Allow), now).is_empty());
    reduce(&mut s, decide("r2", Decision::Deny), now);
    assert!(s.pending.is_empty());
}

#[test]
fn every_waiting_card_expires_on_its_own_deadline() {
    let mut s = State::default();
    let now = Instant::now();
    reduce(&mut s, requested("a", "r1"), now);
    reduce(&mut s, requested("b", "r2"), now + PENDING_TTL / 2);
    assert_eq!(
        reduce(&mut s, Input::Tick, now + PENDING_TTL),
        vec![Effect::ReleasePermission(rid("r1"))]
    );
    assert_eq!(s.view().approval.map(|a| a.request), Some("r2".into()));
    assert_eq!(
        reduce(&mut s, Input::Tick, now + PENDING_TTL / 2 + PENDING_TTL),
        vec![Effect::ReleasePermission(rid("r2"))]
    );
}

#[test]
fn a_subagent_working_leaves_the_main_agents_card_alone() {
    let mut s = State::default();
    let now = Instant::now();
    reduce(&mut s, requested("a", "r1"), now);
    let step = AgentEvent::ToolStarted(Step {
        activity: Activity::Read,
        tool: "Read".into(),
        detail: None,
    });
    assert!(reduce(&mut s, from_subagent("a", "sub-1", step.clone()), now).is_empty());
    assert_eq!(s.pending.len(), 1);
    // The main agent itself moving on means the user answered in the terminal.
    assert_eq!(
        reduce(&mut s, agent("a", step), now),
        vec![Effect::ReleasePermission(rid("r1"))]
    );
}

#[test]
fn always_also_answers_the_same_request_waiting_again() {
    let mut s = State::default();
    let now = Instant::now();
    reduce(&mut s, requested("a", "r1"), now);
    reduce(&mut s, from_subagent("a", "sub-1", requested_event("r2")), now);
    let effects = reduce(
        &mut s,
        Input::User(Intent::DecideAlways { request: rid("r1") }),
        now,
    );
    assert_eq!(
        &effects[..2],
        &[
            Effect::RespondPermission {
                request: rid("r1"),
                decision: Decision::Allow
            },
            Effect::RespondPermission {
                request: rid("r2"),
                decision: Decision::Allow
            },
        ]
    );
    assert!(matches!(effects[2], Effect::SaveRules(_)));
    assert!(s.pending.is_empty());
    assert_eq!(s.sessions[&key("a")].status, Status::Working);
}

#[test]
fn a_step_a_rule_allowed_says_so() {
    let mut s = State::default();
    let now = Instant::now();
    reduce(&mut s, requested("a", "r1"), now);
    reduce(
        &mut s,
        Input::User(Intent::DecideAlways { request: rid("r1") }),
        now,
    );
    reduce(
        &mut s,
        agent(
            "a",
            AgentEvent::ToolStarted(Step {
                activity: Activity::Run,
                tool: "Bash".into(),
                detail: Some("cargo test".into()),
            }),
        ),
        now,
    );
    reduce(&mut s, requested("a", "r2"), now);
    assert!(s.pending.is_empty(), "the rule answered it");
    assert_eq!(
        s.view().sessions[0].step.as_deref(),
        Some("Testing cargo test · always allowed")
    );
}

fn requested_event(id: &str) -> AgentEvent {
    AgentEvent::PermissionRequested {
        request: RequestId(id.into()),
        tool: "Bash".into(),
        target: "Bash · cargo test".into(),
        ask: Ask::default(),
    }
}

#[test]
fn the_card_expires_with_the_hook() {
    let mut s = State::default();
    let now = Instant::now();
    reduce(&mut s, requested("a", "r1"), now);
    assert!(reduce(&mut s, Input::Tick, now + PENDING_TTL / 2).is_empty());
    assert_eq!(
        reduce(&mut s, Input::Tick, now + PENDING_TTL),
        vec![Effect::ReleasePermission(rid("r1"))]
    );
    assert!(s.pending.is_empty());
}

#[test]
fn the_session_moving_on_releases_its_card() {
    let mut s = State::default();
    let now = Instant::now();
    reduce(&mut s, requested("a", "r1"), now);
    // Another session's events leave the card alone.
    assert!(reduce(&mut s, agent("b", AgentEvent::PromptSubmitted), now).is_empty());
    assert_eq!(
        reduce(
            &mut s,
            agent(
                "a",
                AgentEvent::ToolFinished {
                    failed: false,
                    target: None,
                    diff: None,
                }
            ),
            now
        ),
        vec![Effect::ReleasePermission(rid("r1"))]
    );
}

#[test]
fn a_finished_call_leaves_a_parallel_card_waiting() {
    let mut s = State::default();
    let now = Instant::now();
    let ask = |id: &str, target: &str| {
        agent(
            "a",
            AgentEvent::PermissionRequested {
                request: rid(id),
                tool: "WebFetch".into(),
                target: target.into(),
                ask: Ask::default(),
            },
        )
    };
    reduce(&mut s, ask("r1", "WebFetch · a.dev"), now);
    reduce(&mut s, ask("r2", "WebFetch · b.dev"), now);
    reduce(&mut s, decide("r1", Decision::Allow), now);
    let done = |target: &str| {
        agent(
            "a",
            AgentEvent::ToolFinished {
                failed: false,
                target: Some(target.into()),
                diff: None,
            },
        )
    };
    // The allowed call ran: the other card still waits for the user, and stays on screen (the
    // island shows the card only while the session says it needs approval).
    assert!(reduce(&mut s, done("WebFetch · a.dev"), now).is_empty());
    assert_eq!(s.pending.len(), 1);
    let view = s.view();
    assert_eq!(view.sessions[0].status, Status::Approval);
    assert_eq!(view.approval.map(|a| a.request), Some("r2".to_string()));
    // The waiting call finishing means the user answered it in the terminal.
    assert_eq!(
        reduce(&mut s, done("WebFetch · b.dev"), now),
        vec![Effect::ReleasePermission(rid("r2"))]
    );
}

fn asked(session: &str, id: &str) -> Input {
    let choice = |label: &str| Choice {
        label: label.into(),
        description: None,
    };
    agent(
        session,
        AgentEvent::QuestionAsked {
            request: rid(id),
            target: "AskUserQuestion".into(),
            questions: vec![
                Question {
                    question: "Which color?".into(),
                    header: "Color".into(),
                    options: vec![choice("Red"), choice("Blue")],
                    multi: false,
                },
                Question {
                    question: "Which sizes?".into(),
                    header: "Sizes".into(),
                    options: vec![choice("S"), choice("M")],
                    multi: true,
                },
            ],
        },
    )
}

fn answer(id: &str, answers: Vec<Answer>) -> Input {
    Input::User(Intent::Answer {
        request: rid(id),
        answers,
    })
}

fn one(t: &str) -> Answer {
    Answer::One(t.into())
}

#[test]
fn a_question_card_is_answered_only_with_one_reply_per_question() {
    let mut s = State::default();
    let now = Instant::now();
    assert_eq!(
        reduce(&mut s, asked("a", "q1"), now),
        vec![Effect::AckPermission(rid("q1"))]
    );
    assert_eq!(s.sessions[&key("a")].status, Status::Question);
    assert_eq!(s.sessions[&key("a")].note.as_deref(), Some("Which color?"));
    let view = s.view();
    assert_eq!(view.approval.as_ref().unwrap().questions.len(), 2);

    // Allow, Always, a missing reply, a blank one, or several where one is asked: nothing.
    assert!(reduce(&mut s, decide("q1", Decision::Allow), now).is_empty());
    assert!(reduce(&mut s, always("q1"), now).is_empty());
    assert!(reduce(&mut s, answer("q1", vec![one("Blue")]), now).is_empty());
    assert!(reduce(&mut s, answer("q1", vec![one(" "), one("S")]), now).is_empty());
    let many = Answer::Many(vec!["Red".into(), "Blue".into()]);
    assert!(reduce(&mut s, answer("q1", vec![many, one("S")]), now).is_empty());
    assert_eq!(s.pending.len(), 1);

    // The user's own words count as a reply.
    let replies = vec![one("Teal, please"), Answer::Many(vec!["S".into(), "M".into()])];
    assert_eq!(
        reduce(&mut s, answer("q1", replies.clone()), now),
        vec![Effect::AnswerQuestion {
            request: rid("q1"),
            answers: replies.clone()
        }]
    );
    assert_eq!(s.sessions[&key("a")].status, Status::Working);
    assert!(reduce(&mut s, answer("q1", replies), now).is_empty());
}

#[test]
fn a_question_card_can_go_back_to_the_terminal() {
    let mut s = State::default();
    let now = Instant::now();
    reduce(&mut s, asked("a", "q1"), now);
    assert_eq!(
        reduce(&mut s, Input::User(Intent::Release { request: rid("q1") }), now),
        vec![Effect::ReleasePermission(rid("q1"))]
    );
    assert!(s.pending.is_empty());
    // A permission can't be answered as a question.
    reduce(&mut s, requested("a", "r1"), now);
    assert!(reduce(&mut s, answer("r1", vec![one("yes")]), now).is_empty());
}

/// Where a quiet intent (one that must never answer a card) sits in the test's coverage list;
/// `None` for the three that may answer (ADR 0004). Exhaustive on purpose: a new `Intent` stops
/// the build here until someone decides which it is, and a quiet one needs a sample below.
fn quiet_index(intent: &Intent) -> Option<usize> {
    match intent {
        Intent::Decide { .. } | Intent::DecideAlways { .. } | Intent::Answer { .. } => None,
        Intent::Release { .. } => Some(0),
        Intent::OpenAlert { .. } => Some(1),
        Intent::Jump { .. } => Some(2),
        Intent::DismissAlert { .. } => Some(3),
        Intent::OpenRow { .. } => Some(4),
        Intent::Focus { .. } => Some(5),
        Intent::FocusNext => Some(6),
        Intent::FocusPrevious => Some(7),
        Intent::OpenFolder { .. } => Some(8),
        Intent::OpenFile { .. } => Some(9),
        Intent::SetProjectPref { .. } => Some(10),
    }
}

const QUIET_INTENTS: usize = 11;

/// Every quiet intent, aimed at the waiting permission, the waiting question, and things gone.
fn quiet_intents() -> Vec<Intent> {
    let mut intents = Vec::new();
    for id in ["r1", "q1", "gone"] {
        intents.push(Intent::Release { request: rid(id) });
    }
    for session in ["a", "b", "gone"] {
        intents.push(Intent::Jump {
            session: key(session),
        });
        intents.push(Intent::Focus {
            session: Some(key(session)),
        });
        intents.push(Intent::OpenFolder {
            session: key(session),
        });
        intents.push(Intent::OpenFile {
            session: key(session),
            step: 1,
            file: 0,
        });
        for pref in [ProjectPref::Mute, ProjectPref::Pin, ProjectPref::Hide] {
            for on in [true, false] {
                intents.push(Intent::SetProjectPref {
                    session: key(session),
                    pref,
                    on,
                });
            }
        }
    }
    intents.push(Intent::Focus { session: None });
    intents.push(Intent::FocusNext);
    intents.push(Intent::FocusPrevious);
    for k in ["k1", "gone"] {
        intents.push(Intent::OpenAlert { key: k.into() });
        intents.push(Intent::DismissAlert { key: k.into() });
    }
    for item in ["i1", "gone"] {
        intents.push(Intent::OpenRow {
            connector: "github".into(),
            item: item.into(),
        });
    }
    intents
}

#[test]
fn only_decide_can_respond() {
    // Every other input, two at a time, with a permission and a question waiting: none answers
    // either card.
    let events = [
        AgentEvent::SessionStarted,
        AgentEvent::PromptSubmitted,
        AgentEvent::ToolStarted(Step {
            activity: Activity::Run,
            tool: "Bash".into(),
            detail: None,
        }),
        AgentEvent::ToolFinished {
            failed: true,
            target: None,
            diff: None,
        },
        AgentEvent::Question { message: "?".into() },
        AgentEvent::RateLimited,
        AgentEvent::Stopped { message: None },
        AgentEvent::StopFailed { error: None },
        AgentEvent::SubagentStarted,
        AgentEvent::SubagentStopped,
        AgentEvent::SessionEnded,
    ];
    let intents = quiet_intents();
    let mut covered = [false; QUIET_INTENTS];
    for intent in &intents {
        let i = quiet_index(intent).unwrap_or_else(|| panic!("{intent:?} may answer a card"));
        covered[i] = true;
    }
    assert!(covered.iter().all(|c| *c), "every quiet intent has a sample");

    // A deciding intent aimed at the other kind of card, or at one that is gone, answers nothing.
    let misaimed = [
        decide("q1", Decision::Allow),
        decide("gone", Decision::Allow),
        always("q1"),
        always("gone"),
        answer("r1", vec![]),
        answer("r1", vec![one("yes")]),
        answer("gone", vec![one("Red"), one("S")]),
    ];
    // A rule saved later answers the next request, never a card already waiting.
    let rule = Rule {
        agent: AgentKind::Claude,
        cwd: "/home/me/vultures-ai".into(),
        tool: "Bash".into(),
        target: "Bash · cargo test".into(),
    };
    let others = [
        Input::SetRules(vec![rule]),
        Input::SetRules(Vec::new()),
        Input::SetFlock(flock::Flock::World),
        Input::SetOutfit(looks::Outfit::WitchHat),
        Input::Today(looks::Date::new(2026, 10, 31)),
        Input::SetPresence(Presence::Island),
        Input::SetPresence(Presence::Panel),
        Input::SetPresence(Presence::Quiet),
        Input::SetPresence(Presence::Paused),
        alert("k2", "https://github.com/me/app/pull/13"),
        card(Vec::new()),
    ];
    let inputs: Vec<Input> = events
        .iter()
        .flat_map(|e| [agent("a", e.clone()), agent("b", e.clone())])
        .chain(intents.into_iter().map(Input::User))
        .chain(misaimed)
        .chain(others)
        .collect();
    for first in &inputs {
        for second in &inputs {
            let mut s = State::default();
            let now = Instant::now();
            let mut effects = reduce(&mut s, requested("a", "r1"), now);
            effects.extend(reduce(&mut s, asked("b", "q1"), now));
            reduce(&mut s, alert("k1", "https://github.com/me/app/pull/12"), now);
            reduce(
                &mut s,
                card(vec![row("i1", "https://github.com/me/app/pull/12")]),
                now,
            );
            effects.extend(reduce(&mut s, first.clone(), now));
            effects.extend(reduce(&mut s, Input::Tick, now));
            effects.extend(reduce(&mut s, second.clone(), now));
            effects.extend(reduce(&mut s, Input::Tick, now + PENDING_TTL));
            assert!(
                !effects.iter().any(|e| matches!(
                    e,
                    Effect::RespondPermission { .. } | Effect::AnswerQuestion { .. }
                )),
                "{first:?} then {second:?} answered a card"
            );
            // Nor does any of them say a card was answered here: an outcome only tells.
            assert!(
                s.ended.iter().all(|e| matches!(
                    e.outcome,
                    Outcome::Released | Outcome::Terminal | Outcome::Expired
                )),
                "{first:?} then {second:?} ended a card as answered"
            );
        }
    }
}

#[test]
fn steps_and_subagents() {
    let mut s = State::default();
    let now = Instant::now();
    for i in 0..10 {
        let step = Step {
            activity: Activity::Read,
            tool: "Read".into(),
            detail: Some(format!("f{i}.rs")),
        };
        reduce(&mut s, agent("a", AgentEvent::ToolStarted(step)), now);
    }
    reduce(&mut s, agent("a", AgentEvent::SubagentStarted), now);
    reduce(&mut s, agent("a", AgentEvent::SubagentStopped), now);
    reduce(&mut s, agent("a", AgentEvent::SubagentStopped), now);
    let session = &s.sessions[&key("a")];
    assert_eq!(session.steps.len(), MAX_STEPS);
    assert_eq!(session.step_count, 10);
    let view = s.view();
    assert_eq!(view.sessions[0].steps.len(), MAX_STEPS);
    assert_eq!(
        view.sessions[0].steps.last().map(String::as_str),
        Some("Reading f9.rs")
    );
    assert_eq!(
        session.steps.back().and_then(|s| s.detail.as_deref()),
        Some("f9.rs")
    );
    assert_eq!(session.subagents, 0);
    assert_eq!(session.project, "vultures-ai");
    assert_eq!(session.activity, Some(Activity::Read));
}

#[test]
fn ending_a_session_forgets_it() {
    let mut s = State::default();
    let now = Instant::now();
    reduce(&mut s, agent("a", AgentEvent::SessionStarted), now);
    reduce(&mut s, agent("a", AgentEvent::SessionEnded), now);
    assert!(s.sessions.is_empty());
}

#[test]
fn project_names() {
    assert_eq!(project_name("/home/me/app/").as_deref(), Some("app"));
    assert_eq!(project_name(r"C:\work\site").as_deref(), Some("site"));
    assert_eq!(project_name("/"), None);
}

#[test]
fn the_view_shows_the_card_and_labels() {
    let mut s = State::default();
    let now = Instant::now();
    let step = Step {
        activity: Activity::Edit,
        tool: "Edit".into(),
        detail: Some("main.rs".into()),
    };
    reduce(&mut s, agent("a", AgentEvent::ToolStarted(step)), now);
    reduce(&mut s, requested("a", "r1"), now);
    let view = s.view();
    assert_eq!(view.sessions.len(), 1);
    assert_eq!(view.sessions[0].step.as_deref(), Some("Editing main.rs"));
    let approval = view.approval.unwrap();
    assert_eq!(approval.request, "r1");
    assert_eq!(approval.target, "Bash · cargo test");
}

fn alert(key: &str, url: &str) -> Input {
    news(key, None, url)
}

fn news(key: &str, topic: Option<&str>, url: &str) -> Input {
    Input::Connector(Alert {
        key: key.into(),
        topic: topic.map(Into::into),
        seq: 0,
        connector: "github".into(),
        level: AlertLevel::Error,
        title: "Checks failed · me/app#12".into(),
        detail: "Add the flock".into(),
        url: SafeUrl::parse(url),
    })
}

#[test]
fn alerts_are_kept_newest_first_and_capped() {
    let mut s = State::default();
    let now = Instant::now();
    for i in 0..7 {
        reduce(
            &mut s,
            alert(&format!("k{i}"), "https://github.com/me/app/pull/12"),
            now,
        );
    }
    // The same key replaces, it does not pile up.
    reduce(&mut s, alert("k6", "https://github.com/me/app/pull/12"), now);
    let keys: Vec<_> = s.view().alerts.into_iter().map(|a| a.key).collect();
    assert_eq!(keys, ["k6", "k5", "k4", "k3", "k2"]);
}

#[test]
fn a_newer_alert_of_the_same_story_retires_the_older() {
    let mut s = State::default();
    let now = Instant::now();
    let pr = "https://github.com/me/app/pull/12";
    let ci = Some("pr:me/app#12:ci");
    reduce(&mut s, news("pr:me/app#12:ci-failed:p1", ci, pr), now);
    reduce(
        &mut s,
        news("pr:me/app#12:approved", Some("pr:me/app#12:review"), pr),
        now,
    );
    reduce(
        &mut s,
        news("branch:me/app:ci-failed:b1", Some("branch:me/app:ci"), pr),
        now,
    );
    reduce(&mut s, news("pr:me/app#12:ci-passed:p1", ci, pr), now);
    let keys = |s: &State| s.view().alerts.into_iter().map(|a| a.key).collect::<Vec<_>>();
    assert_eq!(
        keys(&s),
        [
            "pr:me/app#12:ci-passed:p1",
            "branch:me/app:ci-failed:b1",
            "pr:me/app#12:approved"
        ],
        "fail then pass on one pull request leaves one alert; other stories stay"
    );
    // A failure on a newer commit retires the pass too.
    reduce(&mut s, news("pr:me/app#12:ci-failed:p2", ci, pr), now);
    assert_eq!(keys(&s)[0], "pr:me/app#12:ci-failed:p2");
    assert_eq!(s.alerts.iter().filter(|a| a.topic.as_deref() == ci).count(), 1);
}

#[test]
fn a_re_requested_review_alerts_again() {
    let mut s = State::default();
    let now = Instant::now();
    let requested = || {
        alert(
            "review:team/lib#7:requested",
            "https://github.com/team/lib/pull/7",
        )
    };
    reduce(&mut s, requested(), now);
    let first = s.view().alerts[0].seq;
    // Still on screen when it is requested again: one alert, but news again.
    reduce(&mut s, requested(), now);
    let view = s.view();
    assert_eq!(view.alerts.len(), 1);
    assert!(view.alerts[0].seq > first);
    // Dismissed, then requested again.
    let dismiss = Intent::DismissAlert {
        key: "review:team/lib#7:requested".into(),
    };
    reduce(&mut s, Input::User(dismiss), now);
    reduce(&mut s, requested(), now);
    assert!(s.view().alerts[0].seq > view.alerts[0].seq);
}

#[test]
fn opening_an_alert_opens_only_a_safe_link() {
    let mut s = State::default();
    let now = Instant::now();
    reduce(&mut s, alert("good", "https://github.com/me/app/pull/12"), now);
    reduce(&mut s, alert("bad", "https://github.com.evil.example/x"), now);
    assert!(!s.view().alerts.iter().find(|a| a.key == "bad").unwrap().link);
    let open = |k: &str| Input::User(Intent::OpenAlert { key: k.into() });
    assert_eq!(
        reduce(&mut s, open("good"), now),
        vec![Effect::OpenUrl(
            SafeUrl::parse("https://github.com/me/app/pull/12").unwrap()
        )]
    );
    assert!(reduce(&mut s, open("bad"), now).is_empty());
    assert!(s.alerts.is_empty(), "opened alerts are done");
}

fn row(item: &str, url: &str) -> board::Row {
    board::Row {
        item: item.into(),
        group: board::Group::Yours,
        name: "app#12".into(),
        title: "Add the flock".into(),
        checks: Some(board::Checks::Failing),
        review: None,
        url: SafeUrl::parse(url),
    }
}

fn card(rows: Vec<board::Row>) -> Input {
    Input::Board {
        connector: "github".into(),
        rows: Some(rows),
    }
}

#[test]
fn a_card_shows_until_its_connector_is_switched_off() {
    let mut s = State::default();
    let now = Instant::now();
    assert!(s.view().boards.is_empty(), "no card before the first poll");
    reduce(
        &mut s,
        card(vec![row("pr:me/app#12", "https://github.com/me/app/pull/12")]),
        now,
    );
    let view = s.view();
    assert_eq!(view.boards.len(), 1);
    assert_eq!(view.boards[0].connector, "github");
    assert_eq!(view.boards[0].rows[0].item, "pr:me/app#12");
    assert!(view.boards[0].rows[0].link);
    // Nothing open is still a card: it says so.
    reduce(&mut s, card(vec![]), now);
    assert!(s.view().boards[0].rows.is_empty());
    reduce(
        &mut s,
        Input::Board {
            connector: "github".into(),
            rows: None,
        },
        now,
    );
    assert!(s.view().boards.is_empty());
}

#[test]
fn opening_a_row_opens_only_a_safe_link() {
    let mut s = State::default();
    let now = Instant::now();
    reduce(
        &mut s,
        card(vec![
            row("pr:me/app#12", "https://github.com/me/app/pull/12"),
            row("pr:me/app#13", "https://github.com.evil.example/x"),
        ]),
        now,
    );
    assert!(!s.view().boards[0].rows[1].link);
    let open = |connector: &str, item: &str| {
        Input::User(Intent::OpenRow {
            connector: connector.into(),
            item: item.into(),
        })
    };
    assert_eq!(
        reduce(&mut s, open("github", "pr:me/app#12"), now),
        vec![Effect::OpenUrl(
            SafeUrl::parse("https://github.com/me/app/pull/12").unwrap()
        )]
    );
    assert!(reduce(&mut s, open("github", "pr:me/app#13"), now).is_empty());
    assert!(reduce(&mut s, open("github", "pr:me/app#99"), now).is_empty());
    assert!(reduce(&mut s, open("other", "pr:me/app#12"), now).is_empty());
    assert_eq!(s.view().boards[0].rows.len(), 2, "a row stays after it is opened");
}

#[test]
fn a_pull_request_that_left_the_card_takes_its_alerts() {
    let mut s = State::default();
    let now = Instant::now();
    let pr = "https://github.com/me/app/pull/12";
    reduce(
        &mut s,
        card(vec![
            row("pr:me/app#12", pr),
            row("pr:me/app#1", pr),
            row("review:team/lib#7", pr),
        ]),
        now,
    );
    reduce(
        &mut s,
        news("pr:me/app#12:ci-failed:p1", Some("pr:me/app#12:ci"), pr),
        now,
    );
    reduce(&mut s, alert("pr:me/app#12:approved", pr), now);
    reduce(&mut s, alert("pr:me/app#1:changes", pr), now);
    reduce(&mut s, alert("review:team/lib#7:requested", pr), now);
    let mut elsewhere = news("pr:me/app#12:ci-failed:x", None, pr);
    if let Input::Connector(a) = &mut elsewhere {
        a.connector = "other".into();
    }
    reduce(&mut s, elsewhere, now);
    // Merged: #12 is gone. `#1` is not a prefix match of `#12`, nor the other way round.
    reduce(
        &mut s,
        card(vec![row("pr:me/app#1", pr), row("review:team/lib#7", pr)]),
        now,
    );
    let keys: Vec<_> = s.view().alerts.into_iter().map(|a| a.key).collect();
    assert_eq!(
        keys,
        [
            "pr:me/app#12:ci-failed:x",
            "review:team/lib#7:requested",
            "pr:me/app#1:changes"
        ],
        "another connector's alert under the same key stays"
    );
    // The review request withdrawn, then the connector switched off: switching off keeps alerts.
    reduce(&mut s, card(vec![row("pr:me/app#1", pr)]), now);
    reduce(
        &mut s,
        Input::Board {
            connector: "github".into(),
            rows: None,
        },
        now,
    );
    let keys: Vec<_> = s.view().alerts.into_iter().map(|a| a.key).collect();
    assert_eq!(keys, ["pr:me/app#12:ci-failed:x", "pr:me/app#1:changes"]);
}

#[test]
fn a_branch_without_checks_keeps_its_alerts_off_the_card() {
    let mut s = State::default();
    let now = Instant::now();
    let url = "https://github.com/me/app/commit/b2";
    let branch = |checks| board::Row {
        group: board::Group::Branches,
        checks,
        ..row("branch:me/app", url)
    };
    reduce(&mut s, card(vec![branch(Some(board::Checks::Failing))]), now);
    reduce(&mut s, alert("branch:me/app:ci-failed:b1", url), now);
    // A `[skip ci]` push: no checks on the new head.
    reduce(&mut s, card(vec![branch(None)]), now);
    assert_eq!(s.view().alerts.len(), 1, "the failure was not fixed");
    assert!(
        s.view().boards[0].rows.is_empty(),
        "but the card has nothing to say about it"
    );
}

#[test]
fn silent_sessions_leave_the_wire() {
    let mut s = State::default();
    let now = Instant::now();
    reduce(&mut s, agent("busy", AgentEvent::PromptSubmitted), now);
    reduce(&mut s, agent("done", AgentEvent::Stopped { message: None }), now);
    reduce(&mut s, requested("asking", "r1"), now);

    reduce(&mut s, Input::Tick, now + FINISHED_TTL);
    assert!(
        !s.sessions.contains_key(&key("done")),
        "a finished session leaves after a while"
    );
    assert!(s.sessions.contains_key(&key("busy")));

    // A newer event keeps a session alive.
    reduce(
        &mut s,
        agent("busy", AgentEvent::PromptSubmitted),
        now + SESSION_TTL / 2,
    );
    reduce(&mut s, Input::Tick, now + SESSION_TTL);
    assert!(s.sessions.contains_key(&key("busy")));
    assert!(s.pending.is_empty(), "the card expired with its hook long ago");
    reduce(&mut s, Input::Tick, now + SESSION_TTL * 2);
    assert!(s.sessions.is_empty());
}

#[test]
fn a_click_on_a_session_jumps_to_its_terminal() {
    let mut s = State::default();
    let now = Instant::now();
    reduce(&mut s, agent("a", AgentEvent::PromptSubmitted), now);
    assert_eq!(
        reduce(&mut s, Input::User(Intent::Jump { session: key("a") }), now),
        vec![Effect::JumpToTerminal(Terminal {
            pid: Some(42),
            ..Default::default()
        })]
    );
    assert!(reduce(&mut s, Input::User(Intent::Jump { session: key("gone") }), now).is_empty());
}

fn always(id: &str) -> Input {
    Input::User(Intent::DecideAlways {
        request: RequestId(id.into()),
    })
}

#[test]
fn always_allows_that_exact_thing_in_that_project_only() {
    let mut s = State::default();
    let now = Instant::now();
    reduce(&mut s, requested("a", "r1"), now);
    let effects = reduce(&mut s, always("r1"), now);
    assert_eq!(
        effects[0],
        Effect::RespondPermission {
            request: rid("r1"),
            decision: Decision::Allow
        }
    );
    assert!(matches!(&effects[1], Effect::SaveRules(r) if r.len() == 1));

    // The same command in the same project: answered at once, no card.
    assert_eq!(
        reduce(&mut s, requested("a", "r2"), now),
        vec![
            Effect::AckPermission(rid("r2")),
            Effect::RespondPermission {
                request: rid("r2"),
                decision: Decision::Allow
            }
        ]
    );
    assert!(s.pending.is_empty());

    // Another command: a card as usual.
    let other = Input::Agent(AgentUpdate {
        session: key("a"),
        cwd: Some("/home/me/vultures-ai".into()),
        terminal: Terminal::default(),
        agent_id: None,
        event: AgentEvent::PermissionRequested {
            request: rid("r3"),
            tool: "Bash".into(),
            target: "Bash · cargo test && rm -rf build".into(),
            ask: Ask::default(),
        },
    });
    assert_eq!(reduce(&mut s, other, now), vec![Effect::AckPermission(rid("r3"))]);
    assert!(!s.pending.is_empty());
}

#[test]
fn a_rule_is_scoped_to_its_folder() {
    let mut s = State::default();
    let now = Instant::now();
    reduce(
        &mut s,
        Input::SetRules(vec![Rule {
            agent: AgentKind::Claude,
            cwd: "/elsewhere".into(),
            tool: "Bash".into(),
            target: "Bash · cargo test".into(),
        }]),
        now,
    );
    // Same tool and target, but this session works in another folder.
    assert_eq!(
        reduce(&mut s, requested("a", "r1"), now),
        vec![Effect::AckPermission(rid("r1"))]
    );
}

#[test]
fn always_on_a_gone_card_does_nothing() {
    let mut s = State::default();
    assert!(reduce(&mut s, always("ghost"), Instant::now()).is_empty());
    assert!(s.rules.is_empty());
}

#[test]
fn a_note_explains_the_state_and_leaves_with_it() {
    let mut s = State::default();
    let now = Instant::now();
    let note = |s: &State| s.view().sessions[0].note.clone();
    reduce(
        &mut s,
        agent(
            "a",
            AgentEvent::Question {
                message: " Which theme? ".into(),
            },
        ),
        now,
    );
    assert_eq!(note(&s).as_deref(), Some("Which theme?"));
    reduce(&mut s, agent("a", AgentEvent::PromptSubmitted), now);
    assert_eq!(note(&s), None);
    reduce(
        &mut s,
        agent(
            "a",
            AgentEvent::Stopped {
                message: Some("All tests pass.".into()),
            },
        ),
        now,
    );
    assert_eq!(note(&s).as_deref(), Some("All tests pass."));
    // A subagent finishing late does not change what the session said.
    reduce(&mut s, agent("a", AgentEvent::SubagentStopped), now);
    assert_eq!(note(&s).as_deref(), Some("All tests pass."));
    reduce(
        &mut s,
        agent(
            "a",
            AgentEvent::StopFailed {
                error: Some("   ".into()),
            },
        ),
        now,
    );
    assert_eq!(note(&s), None);
}

#[test]
fn the_editor_comes_from_the_terminal() {
    let term = |vars: &[(&str, &str)]| Terminal {
        env: vars.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect(),
        ..Default::default()
    };
    assert_eq!(view::editor(&term(&[("TERM_PROGRAM", "kitty")])), None);
    assert_eq!(
        view::editor(&term(&[("TERM_PROGRAM", "vscode")])),
        Some("VS Code")
    );
    assert_eq!(
        view::editor(&term(&[
            ("TERM_PROGRAM", "vscode"),
            ("VSCODE_GIT_ASKPASS_NODE", "/usr/share/code/code"),
        ])),
        Some("VS Code")
    );
    assert_eq!(
        view::editor(&term(&[
            ("TERM_PROGRAM", "vscode"),
            (
                "VSCODE_GIT_ASKPASS_NODE",
                "/tmp/.mount_CursorAbc/usr/share/cursor/Cursor"
            ),
        ])),
        Some("Cursor")
    );
    assert_eq!(
        view::editor(&term(&[("TERM_PROGRAM", "vscode"), ("CURSOR_TRACE_ID", "f00")])),
        Some("Cursor")
    );
    // A folder named after Cursor is not the editor: only the binary's own name counts.
    assert_eq!(
        view::editor(&term(&[
            ("TERM_PROGRAM", "vscode"),
            ("VSCODE_GIT_ASKPASS_NODE", "/home/me/cursor-tools/code"),
        ])),
        Some("VS Code")
    );
}

// ---- the flock: which vulture each session's bird is ----

fn in_project(session: &str, project: &str) -> Input {
    in_project_event(session, project, AgentEvent::SessionStarted)
}

fn in_project_event(session: &str, project: &str, event: AgentEvent) -> Input {
    let Input::Agent(mut u) = agent(session, event) else {
        unreachable!()
    };
    u.cwd = Some(format!("/home/me/{project}"));
    Input::Agent(u)
}

fn species_of(s: &State, id: &str) -> &'static str {
    s.view().sessions.iter().find(|v| v.id == id).unwrap().species
}

#[test]
fn a_session_keeps_its_species_while_it_lives() {
    let mut s = State {
        season: 7,
        ..State::default()
    };
    let now = Instant::now();
    reduce(&mut s, in_project("a", "site"), now);
    let first = species_of(&s, "a");
    assert!(flock::POOL.contains(&first));
    reduce(
        &mut s,
        agent("a", AgentEvent::Stopped { message: None }),
        now + Duration::from_secs(60),
    );
    assert_eq!(species_of(&s, "a"), first);
}

#[test]
fn a_new_season_draws_a_new_flock_from_the_pool() {
    let ids: Vec<String> = (0..40).map(|i| format!("session-{i}")).collect();
    let draw = |season| {
        ids.iter()
            .map(|id| flock::drawn(&flock::POOL, season, id))
            .collect::<Vec<_>>()
    };
    let (one, two) = (draw(1), draw(2));
    assert_ne!(one, two);
    for species in one.iter().chain(&two) {
        assert!(flock::POOL.contains(species), "{species} is not in the pool");
    }
    // Every species of the pool shows up in a flock this size.
    for species in flock::POOL {
        assert!(one.contains(&species), "{species} never drawn");
    }
    // A season reshuffles the flock, not just its labels: six sessions see many different flocks.
    let six = &ids[..6];
    let flocks: std::collections::BTreeSet<Vec<&str>> = (0..48u64)
        .map(|season| {
            six.iter()
                .map(|id| flock::drawn(&flock::POOL, season, id))
                .collect()
        })
        .collect();
    assert!(flocks.len() > 4, "only {} flocks across 48 seasons", flocks.len());
}

#[test]
fn the_oldest_session_of_a_busy_project_is_king_and_stays_king() {
    let mut s = State::default();
    let t0 = Instant::now();
    reduce(&mut s, in_project("a", "api"), t0);
    reduce(&mut s, in_project("b", "api"), t0 + Duration::from_secs(1));
    assert!(
        s.view().sessions.iter().all(|v| v.species != flock::KING),
        "two sessions: no king yet"
    );
    reduce(&mut s, in_project("c", "api"), t0 + Duration::from_secs(2));
    reduce(&mut s, in_project("x", "site"), t0 + Duration::from_secs(3));
    assert_eq!(species_of(&s, "a"), flock::KING);
    for id in ["b", "c", "x"] {
        assert_ne!(species_of(&s, id), flock::KING, "{id}");
    }
    // Activity elsewhere in the project does not move the crown.
    reduce(
        &mut s,
        in_project_event("c", "api", AgentEvent::Stopped { message: None }),
        t0 + Duration::from_secs(9),
    );
    assert_eq!(species_of(&s, "a"), flock::KING);
    // The king leaves: the next oldest inherits, once the project still has three.
    reduce(&mut s, in_project("d", "api"), t0 + Duration::from_secs(10));
    reduce(
        &mut s,
        in_project_event("a", "api", AgentEvent::SessionEnded),
        t0 + Duration::from_secs(11),
    );
    assert_eq!(species_of(&s, "b"), flock::KING);
    // Down to two sessions: no king, and b draws like everyone else.
    reduce(
        &mut s,
        in_project_event("c", "api", AgentEvent::SessionEnded),
        t0 + Duration::from_secs(12),
    );
    assert_ne!(species_of(&s, "b"), flock::KING);
}

#[test]
fn a_session_with_no_project_is_never_king() {
    let mut s = State::default();
    let t0 = Instant::now();
    for (i, id) in ["n1", "n2", "n3", "n4"].into_iter().enumerate() {
        let Input::Agent(mut u) = agent(id, AgentEvent::SessionStarted) else {
            unreachable!()
        };
        u.cwd = None;
        reduce(&mut s, Input::Agent(u), t0 + Duration::from_secs(i as u64));
    }
    assert!(s.view().sessions.iter().all(|v| v.species != flock::KING));
}

#[test]
fn every_species_the_core_names_exists_in_the_renderer() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../ui/src/character/flock/species.ts"
    );
    let species = std::fs::read_to_string(path).expect("the renderer's species");
    let all = [flock::Flock::Brazil, flock::Flock::Americas, flock::Flock::World];
    for id in all.iter().flat_map(|f| f.pool()).chain([&flock::KING]) {
        assert!(
            species.contains(&format!("id: \"{id}\"")),
            "{id} is not a species of the renderer"
        );
    }
}

#[test]
fn the_chosen_pool_is_what_the_flock_draws_from() {
    let mut s = State {
        season: 3,
        ..State::default()
    };
    let now = Instant::now();
    for i in 0..60 {
        reduce(&mut s, in_project(&format!("s{i}"), &format!("p{i}")), now);
    }
    let drawn = |s: &State| s.view().sessions.iter().map(|v| v.species).collect::<Vec<_>>();
    assert!(drawn(&s).iter().all(|id| flock::POOL.contains(id)));
    reduce(&mut s, Input::SetFlock(flock::Flock::World), now);
    let world = drawn(&s);
    assert!(world.iter().all(|id| flock::Flock::World.pool().contains(id)));
    assert!(
        world.iter().any(|id| !flock::POOL.contains(id)),
        "the world pool reaches past Brazil"
    );
}

fn edit_step(file: &str) -> AgentEvent {
    AgentEvent::ToolStarted(Step {
        activity: Activity::Edit,
        tool: "Edit".into(),
        detail: Some(file.into()),
    })
}

fn edited(path: &str, failed: bool) -> AgentEvent {
    AgentEvent::ToolFinished {
        failed,
        target: None,
        diff: Some(Diff {
            files: vec![FileDiff {
                path: path.into(),
                added: 2,
                removed: 1,
                hunks: vec![Hunk {
                    old_start: Some(3),
                    new_start: Some(3),
                    lines: vec!["-a".into(), "+b".into(), "+c".into()],
                }],
            }],
            cut: false,
        }),
    }
}

#[test]
fn a_finished_edit_keeps_its_diff_on_its_step() {
    let mut s = State::default();
    let now = Instant::now();
    reduce(&mut s, agent("a", edit_step("main.rs")), now);
    reduce(&mut s, agent("a", edit_step("lib.rs")), now);
    // Calls may finish out of order: each diff finds its own file's step.
    reduce(&mut s, agent("a", edited("/w/src/main.rs", false)), now);
    let view = s.view();
    let summary = view.sessions[0].diffs.clone();
    assert_eq!(summary.len(), 2);
    assert_eq!(
        summary[0],
        Some(DiffSummary {
            step: 1,
            added: 2,
            removed: 1,
            files: 1
        })
    );
    assert_eq!(summary[1], None);
    assert_eq!(
        s.diff(&key("a"), 1).map(|d| d.files[0].path.as_str()),
        Some("/w/src/main.rs")
    );
    assert!(s.diff(&key("a"), 2).is_none());
    // A failed edit changed nothing; a diff with no step of its file goes nowhere.
    reduce(&mut s, agent("a", edited("/w/src/lib.rs", true)), now);
    reduce(&mut s, agent("a", edited("/w/other.rs", false)), now);
    assert!(s.diff(&key("a"), 2).is_none());
    assert_eq!(s.sessions[&key("a")].diffs.len(), 1);
}

#[test]
fn a_diff_goes_with_its_step() {
    let mut s = State::default();
    let now = Instant::now();
    reduce(&mut s, agent("a", edit_step("main.rs")), now);
    reduce(&mut s, agent("a", edited("/w/main.rs", false)), now);
    for _ in 0..MAX_STEPS {
        reduce(&mut s, agent("a", edit_step("x.rs")), now);
    }
    assert!(s.diff(&key("a"), 1).is_none());
    assert!(s.sessions[&key("a")].diffs.is_empty());
    assert!(s.view().sessions[0].diffs.iter().all(Option::is_none));
}

#[test]
fn zeca_wears_the_look_of_the_day_the_app_gives() {
    use looks::{Date, Outfit};
    let mut s = State::default();
    let now = Instant::now();
    // No date yet: Auto shows nothing rather than guess.
    assert_eq!(s.view().look, None);
    reduce(&mut s, Input::Today(Date::new(2026, 10, 4)), now);
    assert_eq!(s.view().look, Some(Outfit::WitchHat));
    // The next day comes in on a tick: the look follows it.
    reduce(&mut s, Input::Today(Date::new(2026, 11, 2)), now);
    assert_eq!(s.view().look, None);
    reduce(&mut s, Input::SetOutfit(Outfit::Sunglasses), now);
    assert_eq!(s.view().look, Some(Outfit::Sunglasses));
    reduce(&mut s, Input::Today(Date::new(2026, 12, 25)), now);
    reduce(&mut s, Input::SetOutfit(Outfit::None), now);
    assert_eq!(s.view().look, None);
    reduce(&mut s, Input::SetOutfit(Outfit::Auto), now);
    assert_eq!(s.view().look, Some(Outfit::SantaHat));
}

#[test]
fn every_look_the_core_names_is_drawn() {
    use looks::Outfit::*;
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../ui/src/character/zeca/zeca.json"
    );
    let sprites: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(path).expect("Zeca's sprites")).expect("JSON");
    for look in [WitchHat, SantaHat, PartyHat, BunnyEars, Sunglasses] {
        let id = serde_json::to_value(look).expect("an id");
        let id = id.as_str().expect("a string id");
        assert!(sprites["looks"].get(id).is_some(), "{id} is not drawn in zeca.py");
    }
}

#[test]
fn every_look_has_one_place_in_the_pickers_groups() {
    use looks::Outfit::{self, *};
    // Exhaustive: a new look stops the build here; add it to the match and to `all` below.
    let named = |o: Outfit| match o {
        Auto | None | WitchHat | SantaHat | PartyHat | BunnyEars | Sunglasses | WestCoast | FittedCap
        | MountainHat | Headband | Dreads | FrontKnot | Durag | Crown | BucketHat | ClockChain
        | Headphones | ShutterShades | ChromeChain | EyePatch => o,
    };
    let all = [
        Auto,
        None,
        WitchHat,
        SantaHat,
        PartyHat,
        BunnyEars,
        Sunglasses,
        WestCoast,
        FittedCap,
        MountainHat,
        Headband,
        Dreads,
        FrontKnot,
        Durag,
        Crown,
        BucketHat,
        ClockChain,
        Headphones,
        ShutterShades,
        ChromeChain,
        EyePatch,
    ];
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../ui/src/character/looks.ts");
    let ts = std::fs::read_to_string(path).expect("looks.ts");
    let groups = &ts[ts.find("LOOK_GROUPS").expect("the groups")..];
    for look in all.map(named) {
        let id = serde_json::to_value(look).expect("an id");
        let entry = format!("value: \"{}\"", id.as_str().expect("a string id"));
        assert_eq!(groups.matches(&entry).count(), 1, "{entry} in LOOK_GROUPS");
    }
}

fn session_view(s: &State, id: &str) -> SessionView {
    s.view()
        .sessions
        .into_iter()
        .find(|v| v.id == id)
        .expect("session in the view")
}

#[test]
fn each_status_asks_its_own_attention_in_order() {
    let cases = [
        (Status::Idle, Attention::Quiet),
        (Status::Thinking, Attention::Quiet),
        (Status::Working, Attention::Quiet),
        (Status::RateLimited, Attention::Info),
        (Status::Finished, Attention::Done),
        (Status::Failed, Attention::Failed),
        (Status::Approval, Attention::NeedsYou),
        (Status::Question, Attention::NeedsYou),
    ];
    for (status, attention) in cases {
        assert_eq!(status.attention(), attention, "{status:?}");
    }
    assert!(
        Attention::Quiet < Attention::Info
            && Attention::Info < Attention::Done
            && Attention::Done < Attention::Failed
            && Attention::Failed < Attention::NeedsYou
    );
    assert_eq!(
        serde_json::to_value(Attention::NeedsYou).expect("json"),
        "needs-you"
    );
}

#[test]
fn the_view_carries_each_sessions_attention_and_the_most_of_them() {
    let mut s = State::default();
    let now = Instant::now();
    assert_eq!(s.view().attention, Attention::Quiet);
    reduce(&mut s, agent("a", AgentEvent::PromptSubmitted), now);
    assert_eq!(s.view().attention, Attention::Quiet);
    reduce(&mut s, agent("b", AgentEvent::RateLimited), now);
    assert_eq!(s.view().attention, Attention::Info);
    reduce(&mut s, agent("c", AgentEvent::Stopped { message: None }), now);
    assert_eq!(s.view().attention, Attention::Done);
    reduce(&mut s, agent("d", AgentEvent::StopFailed { error: None }), now);
    assert_eq!(s.view().attention, Attention::Failed);
    reduce(&mut s, requested("a", "r1"), now);
    let view = s.view();
    assert_eq!(view.attention, Attention::NeedsYou);
    let by_id = |id: &str| {
        view.sessions
            .iter()
            .find(|v| v.id == id)
            .expect("session")
            .attention
    };
    assert_eq!(
        ["a", "b", "c", "d"].map(by_id),
        [
            Attention::NeedsYou,
            Attention::Info,
            Attention::Done,
            Attention::Failed
        ]
    );
    // Answered: it works again, and the most is what the others ask.
    reduce(&mut s, decide("r1", Decision::Allow), now);
    assert_eq!(s.view().attention, Attention::Failed);
}

#[test]
fn only_the_session_of_the_card_in_line_has_the_card() {
    let mut s = State::default();
    let now = Instant::now();
    reduce(&mut s, requested("a", "r1"), now);
    reduce(&mut s, requested("b", "r2"), now);
    // Both need the user; only the first in line is on the card.
    assert!(session_view(&s, "a").card);
    let b = session_view(&s, "b");
    assert_eq!((b.card, b.attention), (false, Attention::NeedsYou));
    reduce(&mut s, decide("r1", Decision::Deny), now);
    assert!(!session_view(&s, "a").card);
    assert!(session_view(&s, "b").card);
}

#[test]
fn a_question_card_is_the_card_and_a_terminal_question_is_not() {
    let mut s = State::default();
    let now = Instant::now();
    reduce(&mut s, asked("a", "q1"), now);
    assert!(session_view(&s, "a").card);

    // Asked in the terminal: it needs the user, but there is no card to show.
    reduce(
        &mut s,
        agent(
            "b",
            AgentEvent::Question {
                message: "Go on?".into(),
            },
        ),
        now,
    );
    let b = session_view(&s, "b");
    assert_eq!((b.card, b.attention), (false, Attention::NeedsYou));

    // A subagent's permission in line while the session asks in the terminal: no card for it.
    let mut s = State::default();
    reduce(&mut s, from_subagent("c", "sub-1", requested_event("r1")), now);
    reduce(
        &mut s,
        agent(
            "c",
            AgentEvent::Question {
                message: "Which one?".into(),
            },
        ),
        now,
    );
    assert_eq!(s.pending.len(), 1);
    assert!(!session_view(&s, "c").card);
}

#[test]
fn a_card_whose_session_moved_on_is_not_shown() {
    let mut s = State::default();
    let now = Instant::now();
    reduce(&mut s, from_subagent("a", "sub-1", requested_event("r1")), now);
    assert!(session_view(&s, "a").card);
    // The main agent goes on working: the subagent's card still waits, but the session no
    // longer says so.
    let step = AgentEvent::ToolStarted(Step {
        activity: Activity::Read,
        tool: "Read".into(),
        detail: None,
    });
    reduce(&mut s, agent("a", step), now);
    assert_eq!(s.pending.len(), 1);
    let a = session_view(&s, "a");
    assert_eq!((a.card, a.attention), (false, Attention::Quiet));
}

/// How each card left the line, newest first, as the view carries it.
fn outcomes(s: &State) -> Vec<(String, Outcome)> {
    s.view()
        .ended
        .into_iter()
        .map(|e| (e.request, e.outcome))
        .collect()
}

#[test]
fn a_card_answered_here_says_how() {
    let mut s = State::default();
    let now = Instant::now();
    reduce(&mut s, requested("a", "r1"), now);
    reduce(&mut s, requested("a", "r2"), now);
    reduce(&mut s, asked("b", "q1"), now);
    reduce(&mut s, asked("b", "q2"), now);
    reduce(&mut s, decide("r1", Decision::Allow), now);
    reduce(&mut s, decide("r2", Decision::Deny), now);
    reduce(&mut s, answer("q1", vec![one("Red"), one("S")]), now);
    reduce(&mut s, Input::User(Intent::Release { request: rid("q2") }), now);
    assert_eq!(
        outcomes(&s),
        [
            ("q2".to_string(), Outcome::Released),
            ("q1".to_string(), Outcome::Answered),
            ("r2".to_string(), Outcome::Denied),
            ("r1".to_string(), Outcome::Allowed),
        ]
    );
    let last = &s.view().ended[0];
    assert_eq!((last.agent, last.session.as_str()), (AgentKind::Claude, "b"));
    // A click on a card that is gone ends nothing again.
    reduce(&mut s, decide("r1", Decision::Deny), now);
    assert_eq!(outcomes(&s).len(), 4);
}

#[test]
fn always_ends_its_card_here_and_the_same_one_waiting_by_the_rule() {
    let mut s = State::default();
    let now = Instant::now();
    reduce(&mut s, requested("a", "r1"), now);
    reduce(&mut s, requested("a", "r2"), now);
    reduce(&mut s, always("r1"), now);
    assert_eq!(
        outcomes(&s),
        [
            ("r2".to_string(), Outcome::Rule),
            ("r1".to_string(), Outcome::Allowed)
        ]
    );
}

#[test]
fn a_card_settled_in_the_terminal_says_so() {
    let mut s = State::default();
    let now = Instant::now();
    reduce(&mut s, requested("a", "r1"), now);
    reduce(&mut s, agent("a", AgentEvent::PromptSubmitted), now);
    reduce(&mut s, requested("b", "r2"), now);
    reduce(&mut s, agent("b", AgentEvent::SessionEnded), now);
    assert_eq!(
        outcomes(&s),
        [
            ("r2".to_string(), Outcome::Terminal),
            ("r1".to_string(), Outcome::Terminal)
        ]
    );
}

#[test]
fn a_card_nobody_answered_expires() {
    let mut s = State::default();
    let now = Instant::now();
    reduce(&mut s, requested("a", "r1"), now);
    reduce(&mut s, Input::Tick, now + PENDING_TTL / 2);
    assert!(outcomes(&s).is_empty());
    reduce(&mut s, Input::Tick, now + PENDING_TTL);
    assert_eq!(outcomes(&s), [("r1".to_string(), Outcome::Expired)]);
}

#[test]
fn only_the_latest_ended_cards_are_kept() {
    let mut s = State::default();
    let now = Instant::now();
    for i in 0..MAX_ENDED + 3 {
        let id = format!("r{i}");
        reduce(&mut s, requested("a", &id), now);
        reduce(&mut s, decide(&id, Decision::Allow), now);
    }
    let kept = outcomes(&s);
    assert_eq!(kept.len(), MAX_ENDED);
    assert_eq!(kept[0].0, format!("r{}", MAX_ENDED + 2));
}

fn focus(session: Option<&str>) -> Input {
    Input::User(Intent::Focus {
        session: session.map(key),
    })
}

fn front(s: &State) -> Option<String> {
    s.view().front.map(|f| f.id)
}

#[test]
fn front_is_the_first_at_work_then_the_first() {
    let mut s = State::default();
    let now = Instant::now();
    assert_eq!(front(&s), None);
    reduce(&mut s, agent("a", AgentEvent::SessionStarted), now);
    reduce(
        &mut s,
        agent("b", AgentEvent::SessionStarted),
        now + Duration::from_secs(1),
    );
    assert_eq!(front(&s).as_deref(), Some("a"), "all idle: the first to arrive");
    reduce(
        &mut s,
        agent("b", AgentEvent::PromptSubmitted),
        now + Duration::from_secs(2),
    );
    assert_eq!(front(&s).as_deref(), Some("b"), "the first at work");
    // The order is arrival, not the latest news: a busy session does not jump the line.
    reduce(
        &mut s,
        agent("a", AgentEvent::PromptSubmitted),
        now + Duration::from_secs(3),
    );
    let ids: Vec<_> = s.view().sessions.iter().map(|v| v.id.clone()).collect();
    assert_eq!(ids, ["a", "b"]);
    assert_eq!(front(&s).as_deref(), Some("a"));
}

#[test]
fn focus_puts_a_session_in_front_until_it_leaves() {
    let mut s = State::default();
    let now = Instant::now();
    reduce(&mut s, agent("a", AgentEvent::PromptSubmitted), now);
    reduce(
        &mut s,
        agent("b", AgentEvent::SessionStarted),
        now + Duration::from_secs(1),
    );
    assert_eq!(front(&s).as_deref(), Some("a"));
    reduce(&mut s, focus(Some("b")), now);
    assert_eq!(front(&s).as_deref(), Some("b"), "an idle session the user chose");
    assert_eq!(s.view().focus.map(|f| f.id).as_deref(), Some("b"));
    // A session that is not there takes nothing.
    reduce(&mut s, focus(Some("gone")), now);
    assert_eq!(s.focus, Some(key("b")));
    reduce(&mut s, agent("b", AgentEvent::SessionEnded), now);
    assert_eq!(s.focus, None, "forgotten when its session leaves");
    assert_eq!(front(&s).as_deref(), Some("a"));
}

#[test]
fn focus_is_forgotten_when_its_session_times_out() {
    let mut s = State::default();
    let now = Instant::now();
    reduce(&mut s, agent("a", AgentEvent::PromptSubmitted), now);
    reduce(&mut s, focus(Some("a")), now);
    reduce(&mut s, Input::Tick, now + SESSION_TTL);
    assert!(s.sessions.is_empty());
    assert_eq!((s.focus.clone(), front(&s)), (None, None));
}

#[test]
fn focus_can_be_cleared() {
    let mut s = State::default();
    let now = Instant::now();
    reduce(&mut s, agent("a", AgentEvent::PromptSubmitted), now);
    reduce(
        &mut s,
        agent("b", AgentEvent::SessionStarted),
        now + Duration::from_secs(1),
    );
    reduce(&mut s, focus(Some("b")), now);
    reduce(&mut s, focus(None), now);
    assert_eq!(s.focus, None);
    assert_eq!(front(&s).as_deref(), Some("a"));
}

#[test]
fn a_waiting_card_wins_over_focus() {
    let mut s = State::default();
    let now = Instant::now();
    reduce(&mut s, agent("a", AgentEvent::PromptSubmitted), now);
    reduce(
        &mut s,
        agent("b", AgentEvent::SessionStarted),
        now + Duration::from_secs(1),
    );
    reduce(&mut s, focus(Some("a")), now);
    reduce(&mut s, requested("b", "r1"), now);
    assert_eq!(front(&s).as_deref(), Some("b"), "the card's session");
    // Focus chosen while it waits is kept for after: the card still comes first.
    reduce(&mut s, focus(Some("a")), now);
    assert_eq!(front(&s).as_deref(), Some("b"));
    reduce(&mut s, decide("r1", Decision::Allow), now);
    assert_eq!(
        front(&s).as_deref(),
        Some("a"),
        "the user's choice once the card is gone"
    );
}

#[test]
fn a_card_its_session_moved_past_does_not_take_the_front() {
    // A subagent's card while the main agent works on: not drawn (`card` false), so not in front.
    let mut s = State::default();
    let now = Instant::now();
    reduce(&mut s, agent("a", AgentEvent::PromptSubmitted), now);
    reduce(
        &mut s,
        agent("b", AgentEvent::SessionStarted),
        now + Duration::from_secs(1),
    );
    reduce(&mut s, focus(Some("a")), now);
    let Input::Agent(asked) = requested("b", "r1") else {
        unreachable!()
    };
    reduce(&mut s, from_subagent("b", "s1", asked.event), now);
    reduce(&mut s, agent("b", AgentEvent::PromptSubmitted), now);
    assert_eq!(s.pending.len(), 1);
    assert_eq!(front(&s).as_deref(), Some("a"));
}

#[test]
fn focus_answers_nothing() {
    let mut s = State::default();
    let now = Instant::now();
    reduce(&mut s, requested("a", "r1"), now);
    assert!(reduce(&mut s, focus(Some("a")), now).is_empty());
    assert!(reduce(&mut s, focus(None), now).is_empty());
    assert_eq!(s.pending.len(), 1);
}

#[test]
fn going_to_a_card_in_line_brings_it_first_and_answers_nothing() {
    let mut s = State::default();
    let now = Instant::now();
    reduce(&mut s, requested("a", "r1"), now);
    reduce(&mut s, asked("b", "q1"), now + Duration::from_secs(1));
    assert_eq!(s.view().approval.map(|a| a.request), Some("r1".into()));
    let view = s.view();
    assert!(view.sessions.iter().all(|v| v.waiting), "both cards wait");
    assert!(reduce(&mut s, focus(Some("b")), now).is_empty());
    assert_eq!(s.view().approval.map(|a| a.request), Some("q1".into()));
    assert_eq!(front(&s).as_deref(), Some("b"));
    // Only the order changed: each still keeps its own deadline.
    let effects = reduce(&mut s, Input::Tick, now + PENDING_TTL);
    assert_eq!(released(&effects), vec![rid("r1")]);
    assert_eq!(s.pending.len(), 1);
}

fn diffed(s: &mut State, cwd: Option<&str>, path: &str) {
    let now = Instant::now();
    let mut with = |event| {
        let Input::Agent(mut u) = agent("a", event) else {
            unreachable!()
        };
        u.cwd = cwd.map(Into::into);
        reduce(s, Input::Agent(u), now);
    };
    with(edit_step("main.rs"));
    with(edited(path, false));
}

fn open_file(step: u32, file: usize) -> Input {
    Input::User(Intent::OpenFile {
        session: key("a"),
        step,
        file,
    })
}

#[test]
fn a_quick_action_opens_the_folder_or_a_changed_file_by_absolute_path() {
    let mut s = State::default();
    diffed(&mut s, Some("/w"), "/w/src/main.rs");
    let folder = Input::User(Intent::OpenFolder { session: key("a") });
    assert_eq!(
        reduce(&mut s, folder, Instant::now()),
        vec![Effect::OpenFolder("/w".into())]
    );
    assert_eq!(
        reduce(&mut s, open_file(1, 0), Instant::now()),
        vec![Effect::OpenFile {
            path: "/w/src/main.rs".into(),
            line: Some(3)
        }]
    );
    // Past the hunk's leading context, at the first changed line.
    let mut s2 = State::default();
    let now = Instant::now();
    reduce(&mut s2, agent("a", edit_step("main.rs")), now);
    let mut with_context = edited("/w/src/main.rs", false);
    if let AgentEvent::ToolFinished { diff: Some(d), .. } = &mut with_context {
        d.files[0].hunks[0]
            .lines
            .splice(0..0, [" x".to_string(), " y".to_string()]);
    }
    reduce(&mut s2, agent("a", with_context), now);
    assert_eq!(
        reduce(&mut s2, open_file(1, 0), now),
        vec![Effect::OpenFile {
            path: "/w/src/main.rs".into(),
            line: Some(5)
        }]
    );
    // No such step or file, or another session: nothing.
    for input in [
        open_file(2, 0),
        open_file(1, 1),
        Input::User(Intent::OpenFolder { session: key("gone") }),
    ] {
        assert!(reduce(&mut s, input, Instant::now()).is_empty());
    }

    // A file named from the project's folder is found in it.
    let mut s = State::default();
    diffed(&mut s, Some("/w/"), "src/main.rs");
    assert_eq!(
        reduce(&mut s, open_file(1, 0), Instant::now()),
        vec![Effect::OpenFile {
            path: "/w/src/main.rs".into(),
            line: Some(3)
        }]
    );
    // With no absolute folder to start from, a relative path opens nothing: it could be read as
    // an option (`-x`) or from the app's own folder.
    let mut s = State::default();
    diffed(&mut s, Some("w"), "-x/main.rs");
    assert!(reduce(&mut s, open_file(1, 0), Instant::now()).is_empty());
    let folder = Input::User(Intent::OpenFolder { session: key("a") });
    assert!(reduce(&mut s, folder, Instant::now()).is_empty());
}

#[test]
fn open_terminal_raises_a_window_only_on_kde_with_the_agents_process() {
    let mut s = State::default();
    reduce(&mut s, agent("a", AgentEvent::SessionStarted), Instant::now());
    assert!(!session_view(&s, "a").raise, "no desktop said");
    let Input::Agent(mut u) = agent("a", AgentEvent::PromptSubmitted) else {
        unreachable!()
    };
    u.terminal.env.insert("XDG_CURRENT_DESKTOP".into(), "KDE".into());
    reduce(&mut s, Input::Agent(u.clone()), Instant::now());
    assert!(session_view(&s, "a").raise);
    u.terminal.pid = None;
    u.terminal
        .env
        .insert("XDG_CURRENT_DESKTOP".into(), "GNOME".into());
    reduce(&mut s, Input::Agent(u.clone()), Instant::now());
    assert!(!session_view(&s, "a").raise);
    // A multiplexer's pane is brought forward on any desktop.
    u.terminal.env.insert("TMUX_PANE".into(), "%3".into());
    reduce(&mut s, Input::Agent(u), Instant::now());
    assert!(session_view(&s, "a").raise);
}

#[test]
fn going_to_a_card_that_would_not_be_drawn_leaves_the_shown_one() {
    let mut s = State::default();
    let now = Instant::now();
    reduce(&mut s, requested("a", "r1"), now);
    reduce(&mut s, requested("b", "r2"), now + Duration::from_secs(1));
    // A subagent of b works on: its card stays in line, but b no longer waits on it.
    let step = AgentEvent::ToolStarted(Step {
        activity: Activity::Read,
        tool: "Read".into(),
        detail: None,
    });
    reduce(&mut s, from_subagent("b", "s1", step), now);
    assert_eq!(s.pending.len(), 2);
    assert!(!session_view(&s, "b").waiting);
    reduce(&mut s, focus(Some("b")), now);
    let shown = s.view();
    assert_eq!(shown.approval.map(|a| a.request), Some("r1".into()));
    assert!(
        shown.sessions.iter().any(|v| v.card),
        "a's card still has its host"
    );
}

fn three(s: &mut State, now: Instant) {
    for (i, id) in ["a", "b", "c"].into_iter().enumerate() {
        reduce(
            s,
            agent(id, AgentEvent::SessionStarted),
            now + Duration::from_secs(i as u64),
        );
    }
}

#[test]
fn next_and_previous_walk_the_view_order_and_wrap() {
    let mut s = State::default();
    let now = Instant::now();
    let next = || Input::User(Intent::FocusNext);
    let previous = || Input::User(Intent::FocusPrevious);
    // Nobody there: nothing to move.
    assert!(reduce(&mut s, next(), now).is_empty());
    assert_eq!(s.focus, None);
    three(&mut s, now);
    assert_eq!(front(&s).as_deref(), Some("a"));
    let mut walked = Vec::new();
    for _ in 0..4 {
        reduce(&mut s, next(), now);
        walked.push(front(&s).unwrap());
    }
    assert_eq!(walked, ["b", "c", "a", "b"]);
    let mut walked = Vec::new();
    for _ in 0..3 {
        reduce(&mut s, previous(), now);
        walked.push(front(&s).unwrap());
    }
    assert_eq!(walked, ["a", "c", "b"]);
    // The focused session leaves: the walk goes on from core's front again.
    reduce(&mut s, agent("b", AgentEvent::SessionEnded), now);
    assert_eq!(s.focus, None);
    reduce(&mut s, next(), now);
    assert_eq!(front(&s).as_deref(), Some("c"));
}

#[test]
fn next_starts_from_the_session_in_front() {
    let mut s = State::default();
    let now = Instant::now();
    three(&mut s, now);
    // Nothing chosen, "b" at work is in front: next goes on from it.
    reduce(&mut s, agent("b", AgentEvent::PromptSubmitted), now);
    reduce(&mut s, Input::User(Intent::FocusNext), now);
    assert_eq!(front(&s).as_deref(), Some("c"));
}

#[test]
fn next_walks_behind_a_waiting_card() {
    let mut s = State::default();
    let now = Instant::now();
    three(&mut s, now);
    reduce(&mut s, requested("b", "r1"), now);
    reduce(&mut s, Input::User(Intent::FocusNext), now);
    assert_eq!(front(&s).as_deref(), Some("b"), "the card stays in front");
    assert_eq!(s.focus, Some(key("c")));
    reduce(&mut s, Input::User(Intent::FocusNext), now);
    assert_eq!(
        s.focus,
        Some(key("a")),
        "each press moves on, not back to the card"
    );
    assert_eq!(s.pending.len(), 1);
    reduce(&mut s, decide("r1", Decision::Deny), now);
    assert_eq!(front(&s).as_deref(), Some("a"));
}

// ── Desktop notifications ────────────────────────────────────────────────────

use notify::{Change, Kind, NEEDS_YOU_AFTER, Notifier, Prefs};

const ON: Prefs = Prefs { on: true };

/// An empty state in this preset.
fn in_preset(presence: Presence) -> State {
    State {
        presence,
        ..State::default()
    }
}

/// What each change does, as (session, kind) for a show and (session, None) for a withdrawal.
fn notes(changes: Vec<Change>) -> Vec<(String, Option<Kind>)> {
    changes
        .into_iter()
        .map(|c| match c {
            Change::Show { session, notice } => (session.session_id, Some(notice.kind)),
            Change::Withdraw { session } => (session.session_id, None),
        })
        .collect()
}

#[test]
fn a_card_notifies_at_once_by_the_panel_and_late_on_the_island() {
    let now = Instant::now();
    for (presence, at) in [
        (Presence::Panel, Duration::ZERO),
        (Presence::Island, NEEDS_YOU_AFTER),
        (Presence::Quiet, NEEDS_YOU_AFTER),
    ] {
        let prefs = ON;
        let mut s = in_preset(presence);
        let mut n = Notifier::default();
        reduce(&mut s, requested("a", "r1"), now);
        if !at.is_zero() {
            assert!(n.update(&s, now, prefs).is_empty(), "the island shows it already");
            assert!(n.update(&s, now + at - Duration::from_secs(1), prefs).is_empty());
        }
        let shown = n.update(&s, now + at, prefs);
        let Some(Change::Show { notice, .. }) = shown.first() else {
            panic!("{presence:?}: no notification");
        };
        assert_eq!(notice.title, "vultures-ai needs you");
        assert_eq!(notice.body, "Bash · cargo test");
        assert!(n.update(&s, now + at, prefs).is_empty(), "one per event");
        // Answered: it goes.
        reduce(&mut s, decide("r1", Decision::Allow), now + at);
        assert_eq!(notes(n.update(&s, now + at, prefs)), vec![("a".into(), None)]);
    }
}

#[test]
fn a_card_answered_before_its_time_never_notifies_on_the_island() {
    let mut s = State::default();
    let mut n = Notifier::default();
    let now = Instant::now();
    reduce(&mut s, asked("a", "q1"), now);
    assert!(n.update(&s, now, ON).is_empty());
    reduce(
        &mut s,
        answer("q1", vec![one("Red"), Answer::Many(vec!["S".into()])]),
        now,
    );
    assert!(n.update(&s, now + NEEDS_YOU_AFTER, ON).is_empty());
}

#[test]
fn one_notification_per_session_replaced_and_withdrawn() {
    let mut s = in_preset(Presence::Panel);
    let mut n = Notifier::default();
    let now = Instant::now();
    reduce(&mut s, agent("a", AgentEvent::PromptSubmitted), now);
    assert!(n.update(&s, now, ON).is_empty(), "work is not news");
    let stopped = AgentEvent::Stopped {
        message: Some("All   tests\npass.".into()),
    };
    reduce(&mut s, agent("a", stopped), now);
    let shown = n.update(&s, now, ON);
    let [Change::Show { notice, .. }] = shown.as_slice() else {
        panic!("{shown:?}");
    };
    assert_eq!(
        (notice.kind, notice.title.as_str(), notice.body.as_str()),
        (Kind::Finished, "vultures-ai finished", "All tests pass.")
    );
    // Back at work: the old news goes.
    reduce(&mut s, agent("a", AgentEvent::PromptSubmitted), now);
    assert_eq!(notes(n.update(&s, now, ON)), vec![("a".into(), None)]);
    // A card, then a failure: each replaces the session's one notification.
    reduce(&mut s, requested("a", "r1"), now);
    assert_eq!(
        notes(n.update(&s, now, ON)),
        vec![("a".into(), Some(Kind::NeedsYou))]
    );
    reduce(&mut s, decide("r1", Decision::Allow), now);
    let failed = AgentEvent::StopFailed {
        error: Some("overloaded".into()),
    };
    reduce(&mut s, agent("a", failed), now);
    assert_eq!(
        notes(n.update(&s, now, ON)),
        vec![("a".into(), Some(Kind::Failed))]
    );
    // The session leaves: so does its notification.
    reduce(&mut s, agent("a", AgentEvent::SessionEnded), now);
    assert_eq!(notes(n.update(&s, now, ON)), vec![("a".into(), None)]);
}

#[test]
fn turning_notifications_off_withdraws_them_and_shows_nothing() {
    let mut s = in_preset(Presence::Panel);
    let mut n = Notifier::default();
    let now = Instant::now();
    reduce(&mut s, requested("a", "r1"), now);
    reduce(&mut s, agent("b", AgentEvent::StopFailed { error: None }), now);
    assert_eq!(n.update(&s, now, ON).len(), 2);
    let off = Prefs { on: false };
    assert_eq!(
        notes(n.update(&s, now, off)),
        vec![("a".into(), None), ("b".into(), None)]
    );
    reduce(&mut s, requested("c", "r2"), now);
    assert!(n.update(&s, now + NEEDS_YOU_AFTER, off).is_empty());
}

#[test]
fn news_from_while_paused_is_not_raised_on_resume() {
    let mut s = in_preset(Presence::Paused);
    let mut n = Notifier::default();
    let now = Instant::now();
    reduce(&mut s, agent("a", AgentEvent::Stopped { message: None }), now);
    assert!(n.update(&s, now, ON).is_empty(), "paused: nothing");
    reduce(&mut s, Input::SetPresence(Presence::Island), now);
    assert!(
        n.update(&s, now, ON).is_empty(),
        "old news stays quiet after the pause"
    );
    // Something new after it does notify.
    reduce(&mut s, agent("a", AgentEvent::StopFailed { error: None }), now);
    assert_eq!(
        notes(n.update(&s, now, ON)),
        vec![("a".into(), Some(Kind::Failed))]
    );
    // The same with notifications switched off and on again.
    let off = Prefs { on: false };
    reduce(&mut s, agent("b", AgentEvent::Stopped { message: None }), now);
    n.update(&s, now, off);
    assert!(
        n.update(&s, now, ON)
            .iter()
            .all(|c| !matches!(c, Change::Show { session, .. } if session.session_id == "b"))
    );
}

#[test]
fn a_long_note_is_cut_and_a_session_without_a_folder_is_named_by_its_agent() {
    let mut s = State::default();
    let mut n = Notifier::default();
    let now = Instant::now();
    let stopped = AgentEvent::Stopped {
        message: Some("word ".repeat(100)),
    };
    let Input::Agent(mut update) = agent("a", stopped) else {
        unreachable!()
    };
    update.cwd = None;
    reduce(&mut s, Input::Agent(update), now);
    let shown = n.update(&s, now, ON);
    let [Change::Show { notice, .. }] = shown.as_slice() else {
        panic!("{shown:?}");
    };
    assert_eq!(notice.title, "Claude Code finished");
    assert_eq!(notice.body.chars().count(), 160);
    assert!(notice.body.ends_with('…'));
}

#[test]
fn a_notification_can_only_bring_the_card_up() {
    // Its one action is a quiet intent (rule 2): it puts the session in front, nothing more.
    let open = notify::open(&key("a"));
    assert!(quiet_index(&open).is_some(), "{open:?} may answer a card");
    let mut s = State::default();
    let now = Instant::now();
    reduce(&mut s, requested("a", "r1"), now);
    reduce(&mut s, asked("b", "q1"), now);
    for session in ["a", "b", "gone"] {
        let effects = reduce(&mut s, Input::User(notify::open(&key(session))), now);
        assert!(effects.is_empty(), "{effects:?}");
    }
    assert_eq!(s.pending.len(), 2, "both cards still wait");
}

// ── Presence presets (ADR 0009) ──────────────────────────────────────────────

fn acked(effects: &[Effect]) -> Vec<RequestId> {
    effects
        .iter()
        .filter_map(|e| match e {
            Effect::AckPermission(r) => Some(r.clone()),
            _ => None,
        })
        .collect()
}

fn released(effects: &[Effect]) -> Vec<RequestId> {
    effects
        .iter()
        .filter_map(|e| match e {
            Effect::ReleasePermission(r) => Some(r.clone()),
            _ => None,
        })
        .collect()
}

/// Every acknowledged card not yet answered or released waits in the line, and the first in
/// line is the island's card, its session's `card` set: the island opens on it in every preset
/// that acknowledges.
fn every_acked_card_has_its_host(s: &State, effects: &[Effect]) {
    let gone: Vec<&RequestId> = effects
        .iter()
        .filter_map(|e| match e {
            Effect::ReleasePermission(r) | Effect::RespondPermission { request: r, .. } => Some(r),
            _ => None,
        })
        .collect();
    let acked = acked(effects);
    let open: Vec<&RequestId> = acked.iter().filter(|r| !gone.contains(r)).collect();
    for r in &open {
        assert!(
            s.pending.iter().any(|p| &p.request == *r),
            "{r:?} waits for nobody"
        );
    }
    let view = s.view();
    match open.first() {
        Some(first) => {
            let card = view
                .approval
                .as_ref()
                .expect("an acknowledged card on the island");
            assert_eq!(&card.request, &first.0);
            assert!(view.sessions.iter().any(|v| v.card && v.id == card.session));
        }
        None => assert_eq!(view.approval, None),
    }
}

fn pref(session: &str, pref: ProjectPref, on: bool) -> Input {
    Input::User(Intent::SetProjectPref {
        session: key(session),
        pref,
        on,
    })
}

#[test]
fn a_hidden_or_muted_project_still_shows_its_card_in_every_preset() {
    for presence in Presence::ALL {
        let mut s = in_preset(presence);
        let now = Instant::now();
        reduce(&mut s, agent("a", AgentEvent::SessionStarted), now);
        reduce(&mut s, pref("a", ProjectPref::Hide, true), now);
        reduce(&mut s, pref("a", ProjectPref::Mute, true), now);
        assert!(s.view().sessions.is_empty(), "hidden at rest");
        let effects = reduce(&mut s, requested("a", "r1"), now);
        every_acked_card_has_its_host(&s, &effects);
        if presence != Presence::Paused {
            assert!(session_view(&s, "a").muted);
        }
        // Answered, it hides again.
        reduce(&mut s, decide("r1", Decision::Allow), now);
        assert!(s.view().sessions.is_empty());
    }
}

#[test]
fn project_prefs_are_kept_by_folder_and_saved() {
    let mut s = State::default();
    let now = Instant::now();
    reduce(&mut s, agent("a", AgentEvent::SessionStarted), now);
    reduce(&mut s, in_project("b", "site"), now + Duration::from_secs(1));
    let effects = reduce(&mut s, pref("b", ProjectPref::Pin, true), now);
    let pinned = ProjectPrefs {
        pin: true,
        ..Default::default()
    };
    assert_eq!(
        effects,
        vec![Effect::SaveProjects(BTreeMap::from([(
            "/home/me/site".to_string(),
            pinned
        )]))]
    );
    // Pinned first, though it came later.
    let order: Vec<String> = s.view().sessions.iter().map(|v| v.id.clone()).collect();
    assert_eq!(order, vec!["b", "a"]);
    assert!(session_view(&s, "b").pinned);
    // A new session of the same folder is pinned too: the choice is the project's.
    reduce(&mut s, in_project("c", "site"), now);
    assert!(session_view(&s, "c").pinned);
    // Every choice off: the project is forgotten.
    let effects = reduce(&mut s, pref("b", ProjectPref::Pin, false), now);
    assert_eq!(effects, vec![Effect::SaveProjects(BTreeMap::new())]);
    // Hiding the session in front gives the front back.
    reduce(&mut s, focus(Some("a")), now);
    reduce(&mut s, pref("a", ProjectPref::Hide, true), now);
    assert_eq!(s.focus, None);
    assert!(s.view().sessions.iter().all(|v| v.id != "a"));
    // The settings bring a project back.
    reduce(&mut s, Input::SetProjects(BTreeMap::new()), now);
    assert!(s.view().sessions.iter().any(|v| v.id == "a"));
}

#[test]
fn a_muted_project_notifies_nothing_and_a_hidden_one_only_its_card() {
    let mut s = in_preset(Presence::Panel);
    let mut n = Notifier::default();
    let now = Instant::now();
    reduce(&mut s, agent("a", AgentEvent::Stopped { message: None }), now);
    reduce(&mut s, pref("a", ProjectPref::Mute, true), now);
    assert!(n.update(&s, now, ON).is_empty());
    reduce(&mut s, requested("a", "r1"), now);
    assert!(n.update(&s, now, ON).is_empty(), "not even its card");
    let mut s = in_preset(Presence::Panel);
    let mut n = Notifier::default();
    reduce(&mut s, agent("a", AgentEvent::Stopped { message: None }), now);
    reduce(&mut s, pref("a", ProjectPref::Hide, true), now);
    assert!(n.update(&s, now, ON).is_empty());
    reduce(&mut s, requested("a", "r1"), now);
    assert_eq!(
        notes(n.update(&s, now, ON)),
        vec![("a".into(), Some(Kind::NeedsYou))]
    );
}

#[test]
fn in_every_preset_an_acknowledged_card_has_its_host_and_paused_never_acknowledges() {
    let rule = Rule {
        agent: AgentKind::Claude,
        cwd: "/home/me/vultures-ai".into(),
        tool: "Bash".into(),
        target: "Bash · cargo test".into(),
    };
    for from in Presence::ALL {
        for to in Presence::ALL {
            let mut s = in_preset(from);
            let now = Instant::now();
            let mut effects = reduce(&mut s, requested("a", "r1"), now);
            effects.extend(reduce(&mut s, asked("b", "q1"), now));
            if from == Presence::Paused {
                assert!(acked(&effects).is_empty(), "paused acknowledged {effects:?}");
                assert_eq!(
                    released(&effects),
                    vec![rid("r1"), rid("q1")],
                    "the terminal asks at once"
                );
                // The agent waits on its terminal: the session says so, with no card.
                assert_eq!(session_view(&s, "a").status, Status::Approval);
            } else {
                assert_eq!(acked(&effects), vec![rid("r1"), rid("q1")], "{from:?}");
            }
            every_acked_card_has_its_host(&s, &effects);
            // Switching keeps every acknowledged card on the island, or (paused) sends it on.
            effects.extend(reduce(&mut s, Input::SetPresence(to), now));
            if to == Presence::Paused {
                assert!(s.pending.is_empty(), "{from:?} to paused kept a card waiting");
            }
            every_acked_card_has_its_host(&s, &effects);
            // A new request, one a rule answers included, is acknowledged only when not paused.
            reduce(&mut s, Input::SetRules(vec![rule.clone()]), now);
            for request in [requested("c", "r2"), requested("a", "r3")] {
                let more = reduce(&mut s, request, now);
                assert_eq!(
                    acked(&more).is_empty(),
                    to == Presence::Paused,
                    "{to:?}: {more:?}"
                );
                effects.extend(more);
            }
            every_acked_card_has_its_host(&s, &effects);
        }
    }
}

#[test]
fn pausing_sends_the_waiting_cards_to_the_terminal_as_released() {
    let mut s = State::default();
    let now = Instant::now();
    reduce(&mut s, requested("a", "r1"), now);
    reduce(&mut s, asked("b", "q1"), now);
    let effects = reduce(&mut s, Input::SetPresence(Presence::Paused), now);
    assert_eq!(released(&effects), vec![rid("r1"), rid("q1")]);
    assert_eq!(
        outcomes(&s),
        vec![("q1".into(), Outcome::Released), ("r1".into(), Outcome::Released)]
    );
    // Each agent asks in its terminal now, as one asking while paused does.
    assert_eq!(session_view(&s, "a").status, Status::Approval);
    assert_eq!(session_view(&s, "b").status, Status::Question);
    // Back from the pause: the next card is the island's again.
    reduce(&mut s, Input::SetPresence(Presence::Quiet), now);
    assert_eq!(acked(&reduce(&mut s, requested("a", "r2"), now)), vec![rid("r2")]);
}

#[test]
fn paused_shows_no_notification_and_quiet_waits_like_the_island() {
    let now = Instant::now();
    let mut s = in_preset(Presence::Paused);
    let mut n = Notifier::default();
    reduce(&mut s, requested("a", "r1"), now);
    reduce(&mut s, agent("b", AgentEvent::Stopped { message: None }), now);
    assert!(n.update(&s, now + NEEDS_YOU_AFTER, ON).is_empty());
    // Unpaused, what finished meanwhile is old news: no late notification (the away digest, C5,
    // is where it belongs).
    reduce(&mut s, Input::SetPresence(Presence::Quiet), now);
    assert!(n.update(&s, now, ON).is_empty());
}
