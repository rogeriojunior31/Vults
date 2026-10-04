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

#[test]
fn only_decide_can_respond() {
    // Every input except Decide, in every order we can cheaply enumerate: none responds.
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
    for first in &events {
        for second in &events {
            let mut s = State::default();
            let now = Instant::now();
            let mut effects = reduce(&mut s, requested("a", "r1"), now);
            for e in [first, second] {
                effects.extend(reduce(&mut s, agent("a", e.clone()), now));
                effects.extend(reduce(&mut s, Input::Tick, now + PENDING_TTL));
            }
            assert!(
                !effects
                    .iter()
                    .any(|e| matches!(e, Effect::RespondPermission { .. })),
                "{first:?} then {second:?} answered a permission"
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
    // Merged: #12 is gone. `#1` is not a prefix match of `#12`, nor the other way round.
    reduce(
        &mut s,
        card(vec![row("pr:me/app#1", pr), row("review:team/lib#7", pr)]),
        now,
    );
    let keys: Vec<_> = s.view().alerts.into_iter().map(|a| a.key).collect();
    assert_eq!(keys, ["review:team/lib#7:requested", "pr:me/app#1:changes"]);
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
    assert_eq!(keys, ["pr:me/app#1:changes"]);
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
