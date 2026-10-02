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
        Some("Running cargo test · always allowed")
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
                    target: None
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
            },
        )
    };
    // The allowed call ran: the other card still waits for the user.
    assert!(reduce(&mut s, done("WebFetch · a.dev"), now).is_empty());
    assert_eq!(s.pending.len(), 1);
    // The waiting call finishing means the user answered it in the terminal.
    assert_eq!(
        reduce(&mut s, done("WebFetch · b.dev"), now),
        vec![Effect::ReleasePermission(rid("r2"))]
    );
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
    Input::Connector(Alert {
        key: key.into(),
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
