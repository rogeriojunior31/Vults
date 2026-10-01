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
        event,
    })
}

fn requested(session: &str, id: &str) -> Input {
    agent(
        session,
        AgentEvent::PermissionRequested {
            request: RequestId(id.into()),
            tool: "Bash".into(),
            target: "Bash · cargo test".into(),
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
        s.pending.is_some(),
        "a click on another id must not drop the real card"
    );
}

#[test]
fn a_second_request_goes_to_its_terminal() {
    let mut s = State::default();
    let now = Instant::now();
    reduce(&mut s, requested("a", "r1"), now);
    assert_eq!(
        reduce(&mut s, requested("b", "r2"), now),
        vec![Effect::ReleasePermission(rid("r2"))]
    );
    assert_eq!(s.pending.as_ref().map(|p| p.request.clone()), Some(rid("r1")));
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
    assert!(s.pending.is_none());
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
            agent("a", AgentEvent::ToolFinished { failed: false }),
            now
        ),
        vec![Effect::ReleasePermission(rid("r1"))]
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
        AgentEvent::ToolFinished { failed: true },
        AgentEvent::Question { message: "?".into() },
        AgentEvent::RateLimited,
        AgentEvent::Stopped,
        AgentEvent::StopFailed,
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
