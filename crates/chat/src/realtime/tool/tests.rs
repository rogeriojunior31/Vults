use super::*;
use crate::tools::tests::Echo;
use std::sync::Arc;
fn registry() -> Registry {
    Registry::new(vec![Arc::new(Echo)]).unwrap()
}

#[test]
fn tool_deadlines_are_bounded_without_extending_the_turn() {
    use crate::tools::Tool;
    use std::time::Duration;
    #[derive(Debug)]
    struct Slow(Duration);
    impl Tool for Slow {
        fn name(&self) -> &str {
            "slow"
        }
        fn description(&self) -> &str {
            "A bounded consultation."
        }
        fn schema(&self) -> Value {
            json!({"type":"object","properties":{}})
        }
        fn realtime_timeout(&self) -> Duration {
            self.0
        }
        fn call<'a>(&'a self, _: &'a Value) -> vults_agents::driver::BoxFuture<'a, Result<String, String>> {
            Box::pin(async { Ok("answer".into()) })
        }
    }
    for (seconds, expected) in [(10, 10), (60, 60), (600, 60)] {
        let registry = Registry::new(vec![Arc::new(Slow(Duration::from_secs(seconds)))]).unwrap();
        let call = Call {
            identity: Identity {
                attempt: "a".into(),
                response: "r".into(),
                item: "i".into(),
            },
            call: "c".into(),
            name: "slow".into(),
            arguments: json!({}),
        };
        assert_eq!(registry.timeout(&call), Duration::from_secs(expected));
    }
}
fn events(call: &str, args: &str) -> Vec<Value> {
    let item = json!({"id":format!("item_{call}"),"type":"function_call","name":"echo","call_id":call,"arguments":args,"status":"completed"});
    let mut added = item.clone();
    added["arguments"] = json!("");
    added["status"] = json!("in_progress");
    vec![
        json!({"type":"response.output_item.added","response_id":"resp","output_index":0,"item":added}),
        json!({"type":"response.function_call_arguments.delta","response_id":"resp","output_index":0,"item_id":item["id"],"call_id":call,"delta":args}),
        json!({"type":"response.function_call_arguments.done","response_id":"resp","output_index":0,"item_id":item["id"],"call_id":call,"name":"echo","arguments":args}),
        json!({"type":"response.output_item.done","response_id":"resp","output_index":0,"item":item.clone()}),
        json!({"type":"response.done","response":{"id":"resp","status":"completed","output":[item]}}),
    ]
}
#[test]
fn arguments_are_flat_bounded_offered_and_have_no_duplicate_keys() {
    let r = registry();
    assert_eq!(
        r.arguments("echo", r#"{"text":"hi"}"#).unwrap(),
        json!({"text":"hi"})
    );
    for args in [
        "{}",
        r#"{"text":1}"#,
        r#"{"text":"a","extra":"b"}"#,
        r#"{"text":"a","text":"b"}"#,
        r#"{"text":{"x":"a"}}"#,
        "[]",
        "null",
    ] {
        assert!(r.arguments("echo", args).is_err(), "{args}");
    }
    assert!(
        r.arguments("echo", &json!({"text":"x".repeat(257)}).to_string())
            .is_err()
    );
    assert!(r.arguments("approve", r#"{"text":"hi"}"#).is_err());
    assert!(Registry::new(vec![Arc::new(Echo), Arc::new(Echo)]).is_err());
}
#[tokio::test]
async fn completed_calls_keep_stream_and_final_identities_and_return_bounded_inert_data() {
    let r = registry();
    let mut calls = Calls::default();
    let mut output = Output::default();
    let events = events("call_a", r#"{"text":"hi"}"#);
    for event in &events[..4] {
        assert!(calls.event(event, "attempt", &mut output, &r).unwrap());
    }
    let call = calls.finish(&events[4], &r).unwrap();
    assert_eq!(
        call.identity,
        Identity {
            attempt: "attempt".into(),
            response: "resp".into(),
            item: "item_call_a".into()
        }
    );
    let result: Value = serde_json::from_str(&r.execute(&call).await).unwrap();
    assert_eq!(
        result,
        json!({"is_error":false,"outside":false,"text":"echo: hi","truncated":false})
    );
    output.reset();
    calls.new_turn();
    assert!(
        calls.event(&events[0], "other_attempt", &mut output, &r).is_err(),
        "a fresh response cannot replay an old call"
    );
}
#[test]
fn mismatched_partial_cancelled_parallel_and_mixed_audio_calls_never_complete() {
    let r = registry();
    for mode in 0..8 {
        let mut calls = Calls::default();
        let mut output = Output::default();
        let mut events = events("call_a", r#"{"text":"hi"}"#);
        match mode {
            0 => events[2]["arguments"] = json!(r#"{"text":"changed"}"#),
            1 => events[3]["item"]["call_id"] = json!("call_other"),
            2 => events[4]["response"]["status"] = json!("cancelled"),
            3 => events[4]["response"]["output"][0]["arguments"] = json!("{}"),
            4 => events[1]["response_id"] = json!("stale"),
            5 => events[2]["item_id"] = json!("stale"),
            6 => {
                output
                    .bind(
                        &json!({"response_id":"resp","item_id":"audio","output_index":0,"content_index":0}),
                        "attempt",
                    )
                    .unwrap();
            }
            7 => events.insert(1, events[0].clone()),
            _ => unreachable!(),
        }
        let streamed = events[..events.len() - 1]
            .iter()
            .try_for_each(|e| calls.event(e, "attempt", &mut output, &r).map(|_| ()));
        assert!(
            streamed.is_err() || calls.finish(events.last().unwrap(), &r).is_err(),
            "mode {mode}"
        );
    }
}

#[test]
fn serialized_success_and_error_envelopes_include_escaping_in_the_bound() {
    let text = "\0\"\\\u{e9}".repeat(5000);
    for error in [false, true] {
        let encoded = envelope(&text, error, true);
        assert!(encoded.len() <= tools::MAX_RESULT);
        let value: Value = serde_json::from_str(&encoded).unwrap();
        assert_eq!(value["is_error"], error);
        assert_eq!(value["outside"], true);
        assert_eq!(value["truncated"], true);
        assert!(text.starts_with(value["text"].as_str().unwrap()));
    }
}
