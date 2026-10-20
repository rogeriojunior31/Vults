use super::*;
use pricing::REVIEWED_MS;

fn plan() -> Plan {
    Plan::new("gpt-realtime-2.1-mini", 8192, 1024).unwrap()
}

fn send(session: &mut Session, event: &Value) -> Result<Option<Settlement>, Rejection> {
    session.receive(&serde_json::to_vec(event).unwrap())
}

fn configured() -> Session {
    let p = plan();
    let mut config = p.session_update()["session"].clone();
    config["model"] = json!(p.model());
    config["id"] = json!("sess_fixture");
    let mut session = Session::new(p);
    send(&mut session, &json!({"type":"session.updated","session":config})).unwrap();
    session
}

fn creating() -> Session {
    let mut session = configured();
    let prepared = session.prepare("fixture_a", REVIEWED_MS).unwrap();
    assert_eq!(prepared.ceiling_microusd, 179_200);
    assert_eq!(prepared.event["response"]["max_output_tokens"], 1024);
    assert_eq!(prepared.event["response"]["tools"], json!([]));
    assert_eq!(prepared.event["response"]["tool_choice"], "none");
    assert_eq!(
        prepared.event["response"]["metadata"]["vults_attempt"],
        "fixture_a"
    );
    session
}

fn responding() -> Session {
    let mut session = creating();
    let created: Value = serde_json::from_str(
        include_str!("../../tests/fixtures/realtime-session.jsonl")
            .lines()
            .nth(1)
            .unwrap(),
    )
    .unwrap();
    send(&mut session, &created).unwrap();
    session
}

fn done() -> Value {
    serde_json::from_str(
        include_str!("../../tests/fixtures/realtime-session.jsonl")
            .lines()
            .nth(3)
            .unwrap(),
    )
    .unwrap()
}

#[test]
fn cassette_accounts_for_cached_context_and_a_cancelled_billable_response() {
    let mut session = Session::new(plan());
    assert!(session.prepare("too_early", REVIEWED_MS).is_err());
    let mut settlements = Vec::new();
    for line in include_str!("../../tests/fixtures/realtime-session.jsonl").lines() {
        let mut event: Value = serde_json::from_str(line).unwrap();
        if event["type"] == "session.updated" {
            assert_eq!(event["session"]["instructions"], "<persona>");
            event["session"]["instructions"] = json!(persona!());
        } else if event["type"] == "response.created" {
            let prepared = session
                .prepare(
                    event["response"]["metadata"]["vults_attempt"].as_str().unwrap(),
                    REVIEWED_MS,
                )
                .unwrap();
            assert_eq!(prepared.ceiling_microusd, 179_200);
            assert!(session.prepare("parallel", REVIEWED_MS).is_err());
        }
        if let Some(settlement) = send(&mut session, &event).unwrap() {
            settlements.push(settlement);
        }
    }
    assert_eq!(
        settlements,
        vec![
            Settlement {
                attempt: "fixture_a".into(),
                actual_microusd: Some(2059),
                ok: true,
                within_bounds: true
            },
            Settlement {
                attempt: "fixture_b".into(),
                actual_microusd: Some(818),
                ok: false,
                within_bounds: true
            },
        ]
    );
}

#[test]
fn unknown_models_unbounded_limits_and_unqualified_rate_dates_cannot_prepare() {
    for model in ["gpt-realtime", "gpt-realtime-2.1-custom", "", "other"] {
        assert_eq!(Plan::new(model, 8192, 1024).unwrap_err(), Rejection::Model);
    }
    for (input, output) in [(0, 1024), (1, 0), (1, 4097), (128000, 1), (u64::MAX, u64::MAX)] {
        assert_eq!(
            Plan::new("gpt-realtime-2.1", input, output).unwrap_err(),
            Rejection::Limits
        );
    }
    let p = Plan::new("gpt-realtime-2.1", 8192, 1024).unwrap();
    assert_eq!(p.ceiling_microusd(REVIEWED_MS), Some(839_680));
    for now in [REVIEWED_MS - 1, REVIEWED_MS + 30 * 86_400_000, i64::MAX] {
        assert_eq!(p.ceiling_microusd(now), None);
        assert_eq!(configured().prepare("expired", now).err(), Some(Rejection::Rates));
    }
}

#[test]
fn every_effective_cost_control_must_be_explicitly_confirmed() {
    let mut config = plan().session_update()["session"].clone();
    config["model"] = json!("gpt-realtime-2.1-mini");
    config["id"] = json!("sess_fixture");
    assert!(
        send(
            &mut Session::new(plan()),
            &json!({"type":"session.updated","session":config})
        )
        .is_ok()
    );
    for pointer in [
        "/model",
        "/instructions",
        "/tools",
        "/tool_choice",
        "/output_modalities",
        "/max_output_tokens",
        "/truncation",
        "/audio/input/format",
        "/audio/input/transcription",
        "/audio/input/turn_detection",
        "/audio/output/format",
        "/audio/output/voice",
    ] {
        let mut changed = config.clone();
        changed["model"] = json!("gpt-realtime-2.1-mini");
        *changed.pointer_mut(pointer).unwrap() = json!("unexpected");
        let mut session = Session::new(plan());
        assert_eq!(
            send(&mut session, &json!({"type":"session.updated","session":changed})),
            Err(Rejection::Configuration),
            "{pointer}"
        );
        assert_eq!(
            session.prepare("refused", REVIEWED_MS).err(),
            Some(Rejection::State)
        );
    }
    let mut missing = config;
    missing["model"] = json!("gpt-realtime-2.1-mini");
    missing["audio"]["input"]
        .as_object_mut()
        .unwrap()
        .remove("transcription");
    assert_eq!(
        send(
            &mut Session::new(plan()),
            &json!({"type":"session.updated","session":missing})
        ),
        Err(Rejection::Configuration)
    );
}

#[test]
fn incomplete_inconsistent_or_new_usage_never_invents_a_refund() {
    for pointer in [
        "/response/usage",
        "/response/usage/input_tokens",
        "/response/usage/input_token_details/cached_tokens_details",
        "/response/usage/output_token_details/audio_tokens",
    ] {
        let mut event = done();
        *event.pointer_mut(pointer).unwrap() = Value::Null;
        let mut session = responding();
        let settlement = send(&mut session, &event).unwrap().unwrap();
        assert_eq!(settlement.actual_microusd, None, "{pointer}");
        assert!(!settlement.within_bounds);
        assert!(settlement.ok);
        assert!(session.prepare("next", REVIEWED_MS).is_err());
    }
    let mut variations = Vec::new();
    for (pointer, value) in [
        ("/response/usage/total_tokens", json!(254)),
        ("/response/usage/input_token_details/image_tokens", json!(1)),
        (
            "/response/usage/input_token_details/cached_tokens_details/audio_tokens",
            json!(1),
        ),
        (
            "/response/usage/input_token_details/cached_tokens_details/text_tokens",
            json!(120),
        ),
        ("/response/usage/input_tokens", json!(-1)),
        ("/response/usage/output_tokens", json!(1.5)),
        ("/response/usage/total_tokens", json!(u64::MAX)),
    ] {
        let mut event = done();
        *event.pointer_mut(pointer).unwrap() = value;
        variations.push(event);
    }
    let mut new_field = done();
    new_field["response"]["usage"]["output_token_details"]["reasoning_tokens"] = json!(1);
    variations.push(new_field);
    for event in variations {
        assert_eq!(
            send(&mut responding(), &event).unwrap().unwrap().actual_microusd,
            None
        );
    }
}

#[test]
fn known_overruns_are_returned_in_full_and_close_the_owner() {
    let mut event = done();
    let usage = &mut event["response"]["usage"];
    usage["output_tokens"] = json!(10_000);
    usage["total_tokens"] = json!(10_132);
    usage["output_token_details"] = json!({"text_tokens":0,"audio_tokens":10_000});
    let mut session = responding();
    let result = send(&mut session, &event).unwrap().unwrap();
    assert_eq!(result.actual_microusd, Some(200_167));
    assert!(!result.within_bounds);
    assert!(session.prepare("cannot_resume", REVIEWED_MS).is_err());
}

#[test]
fn stale_duplicate_out_of_order_or_malformed_events_cannot_settle_another_operation() {
    assert_eq!(
        send(
            &mut responding(),
            &json!({"type":"response.output_audio.delta","response_id":"resp_old","delta":"AAAA"})
        ),
        Err(Rejection::Event)
    );
    assert_eq!(
        send(
            &mut creating(),
            &json!({"type":"response.output_audio.delta","response_id":"resp_a","delta":"AAAA"})
        ),
        Err(Rejection::State)
    );
    let mut wrong_id = done();
    wrong_id["response"]["id"] = json!("resp_old");
    assert_eq!(send(&mut responding(), &wrong_id), Err(Rejection::Event));
    assert_eq!(send(&mut creating(), &done()), Err(Rejection::State));
    let mut wrong_attempt: Value = serde_json::from_str(
        include_str!("../../tests/fixtures/realtime-session.jsonl")
            .lines()
            .nth(1)
            .unwrap(),
    )
    .unwrap();
    wrong_attempt["response"]["metadata"]["vults_attempt"] = json!("old");
    assert_eq!(send(&mut creating(), &wrong_attempt), Err(Rejection::Event));
    let mut session = responding();
    assert!(send(&mut session, &done()).unwrap().is_some());
    assert_eq!(send(&mut session, &done()), Err(Rejection::State));
    for bytes in [b"not JSON".to_vec(), vec![b' '; MAX_EVENT_BYTES + 1]] {
        let mut session = responding();
        assert_eq!(session.receive(&bytes), Err(Rejection::Event));
        assert_eq!(send(&mut session, &done()), Err(Rejection::State));
    }
}

#[test]
fn a_replaced_session_or_configuration_cannot_reopen_a_closed_owner() {
    let mut session = Session::new(plan());
    send(
        &mut session,
        &json!({"type":"session.created","session":{"model":"gpt-realtime-2.1-mini","id":"sess_original"}}),
    )
    .unwrap();
    let mut config = plan().session_update()["session"].clone();
    config["model"] = json!("gpt-realtime-2.1-mini");
    config["id"] = json!("sess_replaced");
    let event = json!({"type":"session.updated","session":config});
    assert_eq!(send(&mut session, &event), Err(Rejection::Configuration));
    assert_eq!(send(&mut session, &event), Err(Rejection::State));
    assert_eq!(send(&mut responding(), &event), Err(Rejection::Configuration));
    assert_eq!(
        send(&mut configured(), &json!({"type":"error"})),
        Err(Rejection::Event)
    );
}

#[test]
fn attempts_are_consumed_once_and_replay_history_is_bounded() {
    let mut session = configured();
    let created: Value = serde_json::from_str(
        include_str!("../../tests/fixtures/realtime-session.jsonl")
            .lines()
            .nth(1)
            .unwrap(),
    )
    .unwrap();
    for n in 0..MAX_RESPONSES {
        let attempt = format!("attempt_{n}");
        session.prepare(&attempt, REVIEWED_MS).unwrap();
        let mut event = created.clone();
        event["response"]["metadata"]["vults_attempt"] = json!(attempt);
        send(&mut session, &event).unwrap();
        assert!(send(&mut session, &done()).unwrap().is_some());
        assert_eq!(
            session.prepare(&attempt, REVIEWED_MS).err(),
            Some(Rejection::State)
        );
    }
    assert_eq!(
        session.prepare("new_after_limit", REVIEWED_MS).err(),
        Some(Rejection::State)
    );
    assert_eq!(send(&mut session, &created), Err(Rejection::State));
    assert_eq!(send(&mut session, &done()), Err(Rejection::State));
}

#[test]
fn rounding_and_all_modalities_remain_below_the_conservative_ceiling() {
    for model in ["gpt-realtime-2.1", "gpt-realtime-2.1-mini"] {
        let p = Plan::new(model, 8192, 1024).unwrap();
        for (audio, text) in [(0, 0), (8192, 0), (0, 128000), (8192, 119808)] {
            for (out_audio, out_text) in [(0, 1024), (1024, 0), (512, 512)] {
                let usage: Usage = serde_json::from_value(json!({
                    "input_tokens":audio+text,"output_tokens":1024,"total_tokens":audio+text+1024,
                    "input_token_details":{"text_tokens":text,"audio_tokens":audio,"cached_tokens":0},
                    "output_token_details":{"text_tokens":out_text,"audio_tokens":out_audio}
                }))
                .unwrap();
                assert!(p.rates.cost(&usage).unwrap() <= p.ceiling_microusd(REVIEWED_MS).unwrap());
            }
        }
    }
    let mut event = done();
    event["response"]["usage"] = json!({"total_tokens":1,"input_tokens":1,"output_tokens":0,
        "input_token_details":{"text_tokens":1,"audio_tokens":0,"cached_tokens":1,
            "cached_tokens_details":{"text_tokens":1,"audio_tokens":0}},
        "output_token_details":{"text_tokens":0,"audio_tokens":0}});
    assert_eq!(
        send(&mut responding(), &event).unwrap().unwrap().actual_microusd,
        Some(1)
    );
}
