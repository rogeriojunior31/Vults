//! A stand-in for a provider's API on this machine, for the tool loop's tests: each request gets
//! the next scripted stream, and its body is kept for the test to read.

use std::sync::{Arc, Mutex};

use serde_json::Value;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::providers::Provider;

/// The bodies of the requests the stand-in received, in order.
pub(crate) type Seen = Arc<Mutex<Vec<Value>>>;

/// `provider` (its id in `providers`) pointed at a local server that answers each request with the
/// next of `streams` (server-sent events), and a model lookup with a large output cap.
pub(crate) async fn scripted(provider: &str, streams: Vec<String>) -> (Provider, Seen) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/v1", listener.local_addr().unwrap());
    let provider = Provider {
        base_url: Box::leak(url.into_boxed_str()),
        ..*crate::providers::find(provider).unwrap()
    };
    let seen: Seen = Arc::default();
    let bodies = seen.clone();
    tokio::spawn(async move {
        let mut streams = streams.into_iter();
        loop {
            let Ok((mut sock, _)) = listener.accept().await else {
                return;
            };
            let mut raw = Vec::new();
            let mut buf = [0u8; 65536];
            let (head, body) = loop {
                let n = sock.read(&mut buf).await.unwrap_or(0);
                if n == 0 {
                    break (String::new(), String::new());
                }
                raw.extend_from_slice(&buf[..n]);
                let text = String::from_utf8_lossy(&raw).to_string();
                let Some(end) = text.find("\r\n\r\n") else {
                    continue;
                };
                let len: usize = text
                    .lines()
                    .find_map(|l| {
                        l.to_lowercase()
                            .strip_prefix("content-length:")
                            .map(|v| v.trim().parse().unwrap_or(0))
                    })
                    .unwrap_or(0);
                if raw.len() >= end + 4 + len {
                    break (text[..end].to_string(), text[end + 4..].to_string());
                }
            };
            let (kind, reply) = if head.starts_with("GET ") {
                ("application/json", r#"{"max_tokens":64000}"#.to_string())
            } else {
                if let Ok(v) = serde_json::from_str(&body) {
                    bodies.lock().unwrap().push(v);
                }
                ("text/event-stream", streams.next().unwrap_or_default())
            };
            let head = format!(
                "HTTP/1.1 200 OK\r\ncontent-type: {kind}\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
                reply.len()
            );
            let _ = sock.write_all(head.as_bytes()).await;
            let _ = sock.write_all(reply.as_bytes()).await;
        }
    });
    (provider, seen)
}

/// Server-sent events from JSON values, named by their `type` (as the Anthropic wire does).
pub(crate) fn anthropic(events: &[Value]) -> String {
    events
        .iter()
        .map(|e| format!("event: {}\ndata: {e}\n\n", e["type"].as_str().unwrap_or("x")))
        .collect()
}

/// Chat Completions chunks, then `[DONE]`.
pub(crate) fn openai(chunks: &[Value]) -> String {
    let mut s: String = chunks.iter().map(|c| format!("data: {c}\n\n")).collect();
    s.push_str("data: [DONE]\n\n");
    s
}
