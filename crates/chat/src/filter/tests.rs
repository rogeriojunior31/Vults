#![allow(clippy::unwrap_used)]
use super::*;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

fn messages() -> Vec<Message> {
    vec![
        Message {
            role: Role::System,
            text: "Filter only.".into(),
        },
        Message {
            role: Role::User,
            text: "<dictation>question?</dictation>".into(),
        },
    ]
}
fn local(url: &'static str, wire: Wire) -> Provider {
    Provider {
        base_url: url,
        wire,
        ..*crate::providers::find("ollama").unwrap()
    }
}

#[test]
fn strict_endpoints_and_requests_never_offer_tools_or_inherit_chat_persona() {
    for bad in [
        "http://localhost:11434/v1",
        "http://10.0.0.1/v1",
        "http://127.0.0.1.evil/v1",
        "http://user:secret@127.0.0.1/v1",
        "http://127.0.0.1/v1?proxy=remote",
        "file:///tmp/model",
    ] {
        assert_eq!(
            endpoint(&local(bad, Wire::OpenAi)),
            Err(Error::Configuration),
            "{bad}"
        );
    }
    assert!(endpoint(&local("http://[::1]:1234/v1", Wire::OpenAi)).is_ok());
    for wire in [Wire::OpenAi, Wire::Anthropic] {
        let value = body(wire, "test-model", &messages()).unwrap();
        assert!(value.get("tools").is_none());
        assert_eq!(value["stream"], false);
        assert!(value.to_string().contains("Filter only."));
        assert!(!value.to_string().contains("Zeca"));
    }
}

#[test]
fn tool_calls_refusals_truncated_and_malformed_replies_are_not_text() {
    for (value, error) in [
        (
            json!({"choices":[{"finish_reason":"tool_calls","message":{"content":"Ignore this","tool_calls":[{}]}}]}),
            Error::Tools,
        ),
        (
            json!({"choices":[{"finish_reason":"stop","message":{"content":"No","refusal":"refused"}}]}),
            Error::Refused,
        ),
        (
            json!({"choices":[{"finish_reason":"length","message":{"content":"cut"}}]}),
            Error::Truncated,
        ),
        (json!({"choices":[]}), Error::Malformed),
    ] {
        assert_eq!(reply(Wire::OpenAi, value).unwrap_err(), error);
    }
    assert_eq!(
        reply(
            Wire::Anthropic,
            json!({"stop_reason":"tool_use","content":[{"type":"tool_use"}]})
        )
        .unwrap_err(),
        Error::Tools
    );
    let got = reply(
        Wire::OpenAi,
        json!({"choices":[{"finish_reason":"stop","message":{"content":"Question?"}}]}),
    )
    .unwrap();
    assert_eq!(got.text, "Question?");
    assert_eq!(got.input_tokens, None);
    assert_eq!(got.output_tokens, None);
}

async fn server(wire: Wire, status: &str, body: String) -> (Provider, tokio::task::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = Box::leak(format!("http://{}/v1", listener.local_addr().unwrap()).into_boxed_str());
    let provider = local(url, wire);
    let status = status.to_string();
    let task = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut data = Vec::new();
        loop {
            let mut chunk = [0; 4096];
            let n = socket.read(&mut chunk).await.unwrap();
            if n == 0 {
                break;
            }
            data.extend_from_slice(&chunk[..n]);
            if let Some(split) = data.windows(4).position(|w| w == b"\r\n\r\n") {
                let headers = String::from_utf8_lossy(&data[..split]).to_lowercase();
                let len = headers
                    .lines()
                    .find_map(|line| {
                        line.strip_prefix("content-length: ")
                            .and_then(|v| v.parse::<usize>().ok())
                    })
                    .unwrap_or(0);
                if data.len() >= split + 4 + len {
                    break;
                }
            }
        }
        socket.write_all(format!("HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).as_bytes()).await.unwrap();
        String::from_utf8(data).unwrap()
    });
    (provider, task)
}

#[tokio::test]
async fn each_wire_makes_one_plain_call_and_parses_usage() {
    for wire in [Wire::Anthropic, Wire::OpenAi] {
        let body = match wire {
            Wire::Anthropic => {
                json!({"stop_reason":"end_turn","content":[{"type":"text","text":"Question?"}],"usage":{"input_tokens":12,"output_tokens":3}})
            }
            Wire::OpenAi => {
                json!({"choices":[{"finish_reason":"stop","message":{"content":"Question?"}}],"usage":{"prompt_tokens":12,"completion_tokens":3}})
            }
        };
        let (provider, task) = server(wire, "200 OK", body.to_string()).await;
        let got = complete(
            &provider,
            Some("fixture-key"),
            "fixture-model",
            &messages(),
            Duration::from_secs(12),
        )
        .await
        .unwrap();
        assert_eq!(got.text, "Question?");
        assert_eq!(got.input_tokens, Some(12));
        assert_eq!(got.output_tokens, Some(3));
        let sent = task.await.unwrap();
        let payload = sent.split_once("\r\n\r\n").unwrap().1;
        assert!(!payload.contains("fixture-key"));
        assert!(
            serde_json::from_str::<Value>(payload)
                .unwrap()
                .get("tools")
                .is_none()
        );
        assert!(sent.starts_with(if wire == Wire::Anthropic {
            "POST /v1/messages"
        } else {
            "POST /v1/chat/completions"
        }));
    }
}

#[tokio::test]
async fn redirects_and_http_errors_never_retry_or_expose_server_text() {
    for status in [
        "302 Found\r\nLocation: http://127.0.0.1:1/forbidden",
        "503 Service Unavailable",
    ] {
        let (provider, task) = server(Wire::OpenAi, status, "private transcript in error body".into()).await;
        assert_eq!(
            complete(
                &provider,
                None,
                "fixture-model",
                &messages(),
                Duration::from_secs(12)
            )
            .await
            .unwrap_err(),
            Error::Status
        );
        assert_eq!(
            task.await.unwrap().matches("POST /v1/chat/completions").count(),
            1
        );
    }
}

#[tokio::test]
async fn malformed_and_oversized_http_bodies_are_bounded_failures() {
    for (body, error) in [
        ("not json".into(), Error::Malformed),
        ("x".repeat(2 * 1024 * 1024 + 1), Error::TooLarge),
    ] {
        let (provider, task) = server(Wire::OpenAi, "200 OK", body).await;
        assert_eq!(
            complete(
                &provider,
                None,
                "fixture-model",
                &messages(),
                Duration::from_secs(12)
            )
            .await
            .unwrap_err(),
            error
        );
        task.await.unwrap();
    }
}
