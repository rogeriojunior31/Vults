//! Any provider on the OpenAI Chat Completions wire (OpenAI, Gemini, OpenRouter, Groq, local
//! servers…), with the user's key. Like the Anthropic API chat, it has no tools: it only talks.
//! Dropped text files and images go in the message; other files are named, not sent.

use std::path::Path;

use base64::Engine;
use serde_json::{Value, json};
use tokio::sync::mpsc;

use crate::api::{MAX_TEXT_FILE, PERSONA, display_name, image_type};
use crate::providers::{Provider, client};
use crate::{Delta, Turn};

/// One turn; `history` only grows when the turn succeeds, as on the Anthropic wire.
pub(crate) async fn turn(
    provider: &Provider,
    key: Option<&str>,
    model: &str,
    history: &mut Vec<Value>,
    turn: &Turn,
    out: &mpsc::Sender<Delta>,
) -> Result<(), String> {
    let name = provider.label;
    let mut messages = history.clone();
    messages.push(json!({ "role": "user", "content": user_content(turn) }));
    let mut body_messages = vec![json!({ "role": "system", "content": crate::personal(PERSONA) })];
    body_messages.extend(messages.iter().cloned());
    let mut request = client()
        .post(format!("{}/chat/completions", provider.base_url))
        .json(&json!({
            "model": model,
            "stream": true,
            "messages": body_messages,
        }));
    if let Some(key) = key {
        request = request.bearer_auth(key);
    }
    let mut response = request.send().await.map_err(|e| {
        if provider.local && e.is_connect() {
            format!("{name} isn't running on {}.", provider.host())
        } else if e.is_connect() || e.is_timeout() {
            format!("Can't reach {name}. Check the connection.")
        } else {
            format!("The request to {name} failed.")
        }
    })?;
    let status = response.status().as_u16();
    if status != 200 {
        let text = response.text().await.unwrap_or_default();
        return Err(http_error(name, status, &text));
    }

    let mut reply = String::new();
    let mut finished = false;
    let mut events = crate::SseEvents::default();
    let mut think = ThinkFilter::default();
    loop {
        let chunk = response
            .chunk()
            .await
            .map_err(|_| format!("{name} stopped mid-reply."))?;
        let Some(chunk) = chunk else { break };
        for event in events.push(&chunk) {
            match event_text(&event) {
                Event::Text(text) => {
                    let text = think.push(&text);
                    if !text.is_empty() {
                        reply.push_str(&text);
                        let _ = out.send(Delta::Text { text }).await;
                    }
                }
                Event::Done => finished = true,
                Event::Error(message) => return Err(format!("{name}: {message}")),
                Event::Nothing => {}
            }
        }
    }
    let rest = think.finish();
    if !rest.is_empty() {
        reply.push_str(&rest);
        let _ = out.send(Delta::Text { text: rest }).await;
    }
    // Some servers end with `finish_reason` and no `[DONE]`; either counts.
    if !finished && reply.is_empty() {
        return Err(format!("{name} stopped mid-reply."));
    }
    messages.push(json!({ "role": "assistant", "content": reply }));
    *history = messages;
    Ok(())
}

#[derive(Debug, PartialEq)]
enum Event {
    Text(String),
    Done,
    Error(String),
    Nothing,
}

fn event_text(raw: &str) -> Event {
    let data: String = raw
        .lines()
        .filter_map(|l| l.strip_prefix("data:"))
        .map(str::trim_start)
        .collect();
    if data == "[DONE]" {
        return Event::Done;
    }
    let Ok(v) = serde_json::from_str::<Value>(&data) else {
        return Event::Nothing;
    };
    if let Some(message) = v["error"]["message"].as_str() {
        return Event::Error(message.to_string());
    }
    let choice = &v["choices"][0];
    match choice["delta"]["content"].as_str() {
        Some(text) if !text.is_empty() => Event::Text(text.to_string()),
        _ if choice["finish_reason"].is_string() => Event::Done,
        _ => Event::Nothing,
    }
}

/// Reasoning models on local servers (DeepSeek-R1, Qwen3) put their thinking in the reply as
/// `<think>…</think>`. It is hidden as it streams, so a tag split across chunks is held back until
/// the next chunk says what it is.
#[derive(Default)]
struct ThinkFilter {
    inside: bool,
    /// Text that may be the start of a tag.
    held: String,
    /// Visible text was shown; before that, whitespace (the blank lines after a block) is dropped.
    started: bool,
}

impl ThinkFilter {
    const OPEN: &str = "<think>";
    const CLOSE: &str = "</think>";

    /// The visible part of `chunk`.
    fn push(&mut self, chunk: &str) -> String {
        self.held.push_str(chunk);
        let mut shown = String::new();
        loop {
            let tag = if self.inside { Self::CLOSE } else { Self::OPEN };
            if let Some(at) = self.held.find(tag) {
                if !self.inside {
                    shown.push_str(&self.held[..at]);
                }
                self.held.drain(..at + tag.len());
                self.inside = !self.inside;
                continue;
            }
            // Keep only what could still grow into the tag.
            let keep = (1..tag.len())
                .rev()
                .find(|n| self.held.ends_with(&tag[..*n]))
                .unwrap_or(0);
            let cut = self.held.len() - keep;
            if !self.inside {
                shown.push_str(&self.held[..cut]);
            }
            self.held.drain(..cut);
            break;
        }
        if !self.started {
            shown = shown.trim_start().to_string();
            self.started = !shown.is_empty();
        }
        shown
    }

    /// What was held back as a possible tag once the stream ends; an unclosed block stays hidden.
    fn finish(&mut self) -> String {
        let held = std::mem::take(&mut self.held);
        if self.inside { String::new() } else { held }
    }
}

/// Files first, then the question, as content parts.
fn user_content(turn: &Turn) -> Vec<Value> {
    let mut content: Vec<Value> = turn.files.iter().map(|f| file_part(f)).collect();
    let text = if turn.text.trim().is_empty() {
        "What is in this?"
    } else {
        &turn.text
    };
    content.push(json!({ "type": "text", "text": text }));
    content
}

fn file_part(path: &Path) -> Value {
    let name = display_name(path);
    let Ok(bytes) = std::fs::read(path) else {
        return json!({ "type": "text", "text": format!("(The file \"{name}\" could not be read.)") });
    };
    if let Some(media) = image_type(path) {
        let data = base64::engine::general_purpose::STANDARD.encode(&bytes);
        return json!({ "type": "image_url", "image_url": { "url": format!("data:{media};base64,{data}") } });
    }
    match String::from_utf8(bytes) {
        Ok(text) if text.len() <= MAX_TEXT_FILE => json!({
            "type": "text",
            "text": format!("<file name=\"{name}\">\n{text}\n</file>"),
        }),
        _ => json!({
            "type": "text",
            "text": format!("(The user attached \"{name}\", a file this chat can't read.)"),
        }),
    }
}

fn http_error(name: &str, status: u16, body: &str) -> String {
    // OpenAI-style `{"error": {"message"}}`; some servers send a list of those.
    let v = serde_json::from_str::<Value>(body).unwrap_or(Value::Null);
    let error = if v.is_array() { &v[0]["error"] } else { &v["error"] };
    let message = error["message"].as_str().filter(|m| !m.is_empty());
    match status {
        401 => format!("{name} rejected the API key. Check it in Settings."),
        403 => format!("This {name} key can't use this model."),
        429 => format!("The {name} rate limit was reached. Try again in a moment."),
        400 | 404 | 422 if message.is_some() => format!("{name}: {}", message.unwrap_or_default()),
        s if s >= 500 => format!("{name} had a problem ({s}). Try again in a moment."),
        s => format!("{name} answered {s}."),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A one-shot OpenAI-compatible server on this machine: answers one request with `reply`
    /// and hands back what it was sent.
    async fn server(reply: &'static str) -> (Provider, tokio::task::JoinHandle<String>) {
        server_in_parts(vec![reply.as_bytes().to_vec()]).await
    }

    /// The same, sending the reply in separate writes a moment apart: separate chunks on the
    /// client, cut wherever the test says (in the middle of a character, say).
    async fn server_in_parts(parts: Vec<Vec<u8>>) -> (Provider, tokio::task::JoinHandle<String>) {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/v1", listener.local_addr().unwrap());
        let provider = Provider {
            base_url: Box::leak(url.into_boxed_str()),
            ..*crate::providers::find("ollama").unwrap()
        };
        let task = tokio::spawn(async move {
            let (mut sock, _) = listener.accept().await.unwrap();
            let mut seen = Vec::new();
            let mut buf = [0u8; 8192];
            // Headers, then a body of Content-Length bytes.
            loop {
                let n = sock.read(&mut buf).await.unwrap();
                seen.extend_from_slice(&buf[..n]);
                let text = String::from_utf8_lossy(&seen).to_string();
                if let Some(end) = text.find("\r\n\r\n") {
                    let len: usize = text
                        .lines()
                        .find_map(|l| {
                            l.to_lowercase()
                                .strip_prefix("content-length:")
                                .map(|v| v.trim().parse().unwrap())
                        })
                        .unwrap_or(0);
                    if seen.len() >= end + 4 + len {
                        break;
                    }
                }
            }
            let head = "HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\nconnection: close\r\n\r\n";
            sock.write_all(head.as_bytes()).await.unwrap();
            for part in parts {
                sock.write_all(&part).await.unwrap();
                sock.flush().await.unwrap();
                tokio::time::sleep(std::time::Duration::from_millis(60)).await;
            }
            String::from_utf8_lossy(&seen).to_string()
        });
        (provider, task)
    }

    #[tokio::test]
    async fn a_turn_streams_from_a_local_server() {
        let (provider, request) = server(concat!(
            "data: {\"choices\":[{\"delta\":{\"role\":\"assistant\"}}]}\n\n",
            "data: {\"choices\":[{\"delta\":{\"content\":\"Hel\"}}]}\n\n",
            "data: {\"choices\":[{\"delta\":{\"content\":\"lo\"}}]}\n\n",
            "data: {\"choices\":[{\"delta\":{},\"finish_reason\":\"stop\"}]}\n\n",
            "data: [DONE]\n\n",
        ))
        .await;
        let (tx, mut rx) = mpsc::channel(16);
        let mut history = Vec::new();
        let t = Turn {
            text: "hi".into(),
            files: vec![],
        };
        turn(&provider, None, "llama-test", &mut history, &t, &tx)
            .await
            .unwrap();
        let mut shown = String::new();
        while let Ok(Delta::Text { text }) = rx.try_recv() {
            shown.push_str(&text);
        }
        assert_eq!(shown, "Hello");
        assert_eq!(history.len(), 2);
        assert_eq!(history[1], json!({ "role": "assistant", "content": "Hello" }));
        let sent = request.await.unwrap();
        assert!(sent.starts_with("POST /v1/chat/completions"));
        // A local server gets no key at all.
        assert!(!sent.to_lowercase().contains("authorization:"));
        assert!(sent.contains("\"model\":\"llama-test\""));
        assert!(sent.contains("\"role\":\"system\""));
    }

    #[tokio::test]
    async fn a_character_cut_between_two_chunks_arrives_whole() {
        let body = "data: {\"choices\":[{\"delta\":{\"content\":\"I did the a\u{e7}\u{e3}o 🦅\"}}]}\n\ndata: [DONE]\n\n";
        // Cut between the two bytes of the c-cedilla.
        let cut = body.find('\u{e7}').unwrap() + 1;
        let (provider, _request) = server_in_parts(vec![
            body.as_bytes()[..cut].to_vec(),
            body.as_bytes()[cut..].to_vec(),
        ])
        .await;
        let (tx, mut rx) = mpsc::channel(16);
        let mut history = Vec::new();
        turn(&provider, None, "llama-test", &mut history, &Turn::default(), &tx)
            .await
            .unwrap();
        let mut shown = String::new();
        while let Ok(Delta::Text { text }) = rx.try_recv() {
            shown.push_str(&text);
        }
        assert_eq!(shown, "I did the a\u{e7}\u{e3}o 🦅");
        // The history is replayed on every later turn: it must be right too.
        assert_eq!(history[1]["content"], "I did the a\u{e7}\u{e3}o 🦅");
    }

    #[tokio::test]
    async fn thinking_never_reaches_the_island_or_the_history() {
        let (provider, _request) = server(concat!(
            "data: {\"choices\":[{\"delta\":{\"reasoning_content\":\"hmm\"}}]}\n\n",
            "data: {\"choices\":[{\"delta\":{\"content\":\"<thi\"}}]}\n\n",
            "data: {\"choices\":[{\"delta\":{\"content\":\"nk>The user said hi.</th\"}}]}\n\n",
            "data: {\"choices\":[{\"delta\":{\"content\":\"ink>\\n\\nHel\"}}]}\n\n",
            "data: {\"choices\":[{\"delta\":{\"content\":\"lo\"}}]}\n\n",
            "data: [DONE]\n\n",
        ))
        .await;
        let (tx, mut rx) = mpsc::channel(16);
        let mut history = Vec::new();
        turn(&provider, None, "qwen3", &mut history, &Turn::default(), &tx)
            .await
            .unwrap();
        let mut shown = Vec::new();
        while let Ok(Delta::Text { text }) = rx.try_recv() {
            shown.push(text);
        }
        assert_eq!(shown, ["Hel", "lo"]);
        assert_eq!(history[1], json!({ "role": "assistant", "content": "Hello" }));
    }

    #[test]
    fn the_think_filter() {
        let run = |chunks: &[&str]| {
            let mut f = ThinkFilter::default();
            let mut out: String = chunks.iter().map(|c| f.push(c)).collect();
            out.push_str(&f.finish());
            out
        };
        // No thinking: unchanged, a lone `<` included.
        assert_eq!(run(&["a < b", " and <", "br>"]), "a < b and <br>");
        assert_eq!(run(&["ends with <thi"]), "ends with <thi");
        // Tags split anywhere.
        assert_eq!(run(&["<", "think", ">x</", "think", ">", " Hi"]), "Hi");
        assert_eq!(run(&["<think>x</think>\n\nHi <think>y</think>there"]), "Hi there");
        // Thinking cut off by the end of the stream stays hidden.
        assert_eq!(run(&["Hi<think>never closed"]), "Hi");
    }

    #[tokio::test]
    async fn a_local_server_that_is_off_says_so() {
        // A port nothing listens on: bind, then drop.
        let port = std::net::TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        let provider = Provider {
            base_url: Box::leak(format!("http://127.0.0.1:{port}/v1").into_boxed_str()),
            ..*crate::providers::find("ollama").unwrap()
        };
        let (tx, _rx) = mpsc::channel(4);
        let err = turn(&provider, None, "m", &mut Vec::new(), &Turn::default(), &tx)
            .await
            .unwrap_err();
        assert_eq!(err, format!("Ollama isn't running on 127.0.0.1:{port}."));
    }

    #[test]
    fn reads_the_stream() {
        let ev = |data: &str| event_text(&format!("data: {data}\n\n"));
        assert_eq!(
            ev(r#"{"choices":[{"delta":{"content":"Hi"}}]}"#),
            Event::Text("Hi".into())
        );
        assert_eq!(
            ev(r#"{"choices":[{"delta":{"role":"assistant"}}]}"#),
            Event::Nothing
        );
        assert_eq!(
            ev(r#"{"choices":[{"delta":{},"finish_reason":"stop"}]}"#),
            Event::Done
        );
        assert_eq!(ev("[DONE]"), Event::Done);
        assert_eq!(
            ev(r#"{"error":{"message":"bad model"}}"#),
            Event::Error("bad model".into())
        );
        assert_eq!(event_text(": keep-alive\n\n"), Event::Nothing);
    }

    #[test]
    fn errors_name_the_provider() {
        assert!(http_error("Groq", 401, "").starts_with("Groq rejected"));
        assert_eq!(
            http_error("OpenAI", 404, r#"{"error":{"message":"no such model"}}"#),
            "OpenAI: no such model"
        );
        assert_eq!(
            http_error("Google Gemini", 400, r#"[{"error":{"message":"bad"}}]"#),
            "Google Gemini: bad"
        );
        assert!(http_error("xAI", 502, "").contains("502"));
    }

    #[test]
    fn images_become_data_urls() {
        let dir = std::env::temp_dir().join("vultures-ai-openai-test");
        std::fs::create_dir_all(&dir).unwrap();
        let png = dir.join("1700000000001-shot.png");
        std::fs::write(&png, [137u8, 80, 78, 71]).unwrap();
        let content = user_content(&Turn {
            text: String::new(),
            files: vec![png],
        });
        assert!(
            content[0]["image_url"]["url"]
                .as_str()
                .unwrap()
                .starts_with("data:image/png;base64,")
        );
        assert_eq!(content[1]["text"], "What is in this?");
    }
}
