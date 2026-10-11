//! Any provider on the OpenAI Chat Completions wire (OpenAI, Gemini, OpenRouter, Groq, local
//! servers…), with the user's key. Like the Anthropic API chat, it may call the conversation's
//! read-only tools in a loop (`tools`), none of which runs a command or edits a file.
//! Dropped text files and images go in the message; other files are named, not sent.

use std::path::Path;

use base64::Engine;
use serde_json::{Value, json};
use tokio::sync::mpsc;

use crate::api::{
    MAX_TEXT_FILE, PERSONA, PERSONA_TOOLS, WHAT_IS_IN_THIS, display_name, file_foreign, file_text,
    file_unreadable, image_type,
};
use crate::providers::{Provider, client};
use crate::tools;
use crate::{Delta, Turn, Usage};

/// The window assumed for [`tools::fit`]: the smallest of the chat models these providers list.
const WINDOW: usize = 128_000;

/// One turn, with the same tool loop as the Anthropic wire; `history` only grows when the turn
/// succeeds.
pub(crate) async fn turn(
    provider: &Provider,
    key: Option<&str>,
    model: &str,
    history: &mut Vec<Value>,
    turn: &Turn,
    kit: tools::Kit<'_>,
    out: &mpsc::Sender<Delta>,
) -> Result<(), String> {
    let (tools, outside) = (kit.tools, kit.outside);
    let name = provider.label;
    let mut messages = history.clone();
    messages.push(json!({ "role": "user", "content": user_content(turn) }));
    let mut usage: Option<Usage> = None;
    for step in 0..=tools::MAX_STEPS {
        tools::fit(&mut messages, WINDOW);
        let reply = call(provider, key, model, &messages, kit, out).await?;
        if let Some(u) = &reply.usage {
            usage = Some(usage.map_or_else(|| u.clone(), |sum| sum.plus(u)));
        }
        if !reply.calls.is_empty() && !tools.is_empty() {
            // Only calls the model finished: a reply cut short (`length`, or a dropped stream)
            // may hold arguments that end midway.
            let complete = match reply.reason.as_deref() {
                Some(r) => r == "tool_calls",
                None => reply.finished,
            };
            if !complete {
                return Err(format!("{name} stopped mid-reply."));
            }
            if step == tools::MAX_STEPS {
                return Err(format!("Zeca stopped after {} tool calls.", tools::MAX_STEPS));
            }
            let calls: Vec<Value> = reply
                .calls
                .iter()
                .map(|c| json!({ "id": c.id, "type": "function", "function": { "name": c.name, "arguments": c.arguments } }))
                .collect();
            // No text beside calls is `null`: some servers refuse an empty string there.
            let content = (!reply.text.is_empty()).then(|| reply.text.clone());
            messages.push(json!({ "role": "assistant", "content": content, "tool_calls": calls }));
            for c in &reply.calls {
                let _ = out.send(Delta::Tool { tool: c.name.clone() }).await;
                let args = if c.arguments.trim().is_empty() {
                    "{}"
                } else {
                    &c.arguments
                };
                // Arguments that don't read as JSON: the tool is not run with ones it wasn't sent.
                let text = match serde_json::from_str::<Value>(args) {
                    Ok(input) => {
                        let (text, error) = tools::run(tools, &c.name, &input).await;
                        if !error && tools::outside(tools, &c.name) {
                            crate::taint(outside, c.name.clone());
                        }
                        text
                    }
                    Err(_) => "The arguments were not valid JSON.".to_string(),
                };
                messages.push(json!({ "role": "tool", "tool_call_id": c.id, "content": text }));
            }
            continue;
        }
        // Some servers end with `finish_reason` and no `[DONE]`; either counts.
        if !reply.finished && reply.reason.is_none() && reply.text.is_empty() {
            return Err(format!("{name} stopped mid-reply."));
        }
        if let Some(u) = usage {
            let _ = out.send(Delta::Usage(u)).await;
        }
        messages.push(json!({ "role": "assistant", "content": reply.text }));
        *history = messages;
        return Ok(());
    }
    Err(format!("Zeca stopped after {} tool calls.", tools::MAX_STEPS))
}

/// A tool call as it streams: its pieces arrive by `index`.
#[derive(Debug, Default, Clone, PartialEq)]
struct Call {
    id: String,
    name: String,
    arguments: String,
}

/// One reply, whole.
#[derive(Debug, Default)]
struct Reply {
    text: String,
    calls: Vec<Call>,
    /// `[DONE]` came.
    finished: bool,
    /// The last `finish_reason` heard (`stop`, `tool_calls`, `length`).
    reason: Option<String>,
    usage: Option<Usage>,
}

/// One request: streams the reply's text to the chat and returns the whole reply.
async fn call(
    provider: &Provider,
    key: Option<&str>,
    model: &str,
    messages: &[Value],
    kit: tools::Kit<'_>,
    out: &mpsc::Sender<Delta>,
) -> Result<Reply, String> {
    let name = provider.label;
    let mut body_messages = vec![json!({ "role": "system", "content": kit.system(PERSONA, PERSONA_TOOLS) })];
    body_messages.extend(messages.iter().cloned());
    let mut body = json!({
        "model": model,
        "stream": true,
        "messages": body_messages,
    });
    let tools = kit.tools;
    if !tools.is_empty() {
        body["tools"] = tools
            .iter()
            .map(|t| json!({ "type": "function", "function": { "name": t.name(), "description": t.description(), "parameters": t.schema() } }))
            .collect();
    }
    let mut request = client()
        .post(format!("{}/chat/completions", provider.base_url))
        .json(&body);
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

    let mut reply = Reply::default();
    let mut events = crate::SseEvents::default();
    let mut think = ThinkFilter::default();
    loop {
        let chunk = response
            .chunk()
            .await
            .map_err(|_| format!("{name} stopped mid-reply."))?;
        let Some(chunk) = chunk else { break };
        for event in events.push(&chunk) {
            if let Some(reason) = finish_reason(&event) {
                reply.reason = Some(reason);
            }
            for (index, piece) in tool_pieces(&event) {
                gather(&mut reply.calls, index, piece);
            }
            match event_text(&event) {
                Event::Text(text) => {
                    let text = think.push(&text);
                    if !text.is_empty() {
                        reply.text.push_str(&text);
                        let _ = out.send(Delta::Text { text }).await;
                    }
                }
                Event::Done => reply.finished = true,
                Event::Usage(u) => reply.usage = Some(u),
                Event::Error(message) => return Err(format!("{name}: {message}")),
                Event::Nothing => {}
            }
        }
    }
    let rest = think.finish();
    if !rest.is_empty() {
        reply.text.push_str(&rest);
        let _ = out.send(Delta::Text { text: rest }).await;
    }
    reply.calls.retain(|c| !c.name.is_empty());
    Ok(reply)
}

/// Adds one piece to the calls being gathered. The id and name come once (a server that repeats
/// them in every piece is read the same); only the arguments are appended. A server that sends no
/// `index` starts a new call with each new id.
fn gather(calls: &mut Vec<Call>, index: Option<usize>, piece: Call) {
    let at = match index {
        Some(i) => i,
        None => match calls.last() {
            Some(last) if piece.id.is_empty() || piece.id == last.id => calls.len() - 1,
            _ => calls.len(),
        },
    };
    if calls.len() <= at {
        calls.resize(at + 1, Call::default());
    }
    let call = &mut calls[at];
    if call.id.is_empty() {
        call.id = if piece.id.is_empty() {
            format!("call_{at}")
        } else {
            piece.id
        };
    }
    if call.name.is_empty() {
        call.name = piece.name;
    }
    call.arguments.push_str(&piece.arguments);
}

/// The `finish_reason` in one event, if it has one.
fn finish_reason(raw: &str) -> Option<String> {
    let data: String = raw
        .lines()
        .filter_map(|l| l.strip_prefix("data:"))
        .map(str::trim_start)
        .collect();
    let v: Value = serde_json::from_str(&data).ok()?;
    v["choices"][0]["finish_reason"].as_str().map(str::to_string)
}

/// The pieces of tool calls in one event, with their `index` when the server sends one.
fn tool_pieces(raw: &str) -> Vec<(Option<usize>, Call)> {
    let data: String = raw
        .lines()
        .filter_map(|l| l.strip_prefix("data:"))
        .map(str::trim_start)
        .collect();
    let Ok(v) = serde_json::from_str::<Value>(&data) else {
        return Vec::new();
    };
    v["choices"][0]["delta"]["tool_calls"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or_default()
        .iter()
        .map(|c| {
            let text = |v: &Value| v.as_str().unwrap_or_default().to_string();
            (
                c["index"].as_u64().map(|i| i as usize),
                Call {
                    id: text(&c["id"]),
                    name: text(&c["function"]["name"]),
                    arguments: text(&c["function"]["arguments"]),
                },
            )
        })
        .collect()
}

#[derive(Debug, PartialEq)]
enum Event {
    Text(String),
    Done,
    /// Only from a server that sends it unasked (OpenRouter, some local ones): nothing is
    /// requested, so no server is made to refuse an option it doesn't know.
    Usage(Usage),
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
    let usage = &v["usage"];
    if usage.is_object() && choice["delta"]["content"].as_str().is_none_or(str::is_empty) {
        let n = |k: &str| usage[k].as_u64().unwrap_or(0);
        let u = Usage {
            model: v["model"].as_str().map(str::to_owned),
            input: n("prompt_tokens"),
            output: n("completion_tokens"),
            cache_read: usage["prompt_tokens_details"]["cached_tokens"]
                .as_u64()
                .unwrap_or(0),
            ..Usage::default()
        };
        if !u.is_empty() {
            return Event::Usage(u);
        }
    }
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
        WHAT_IS_IN_THIS
    } else {
        &turn.text
    };
    // The app's context first, apart from the words: the same blocks every engine reads (M2).
    if let Some(context) = &turn.context {
        content.insert(0, json!({ "type": "text", "text": context }));
    }
    content.push(json!({ "type": "text", "text": text }));
    content
}

fn file_part(path: &Path) -> Value {
    let name = display_name(path);
    let Ok(bytes) = std::fs::read(path) else {
        return json!({ "type": "text", "text": file_unreadable(&name) });
    };
    if let Some(media) = image_type(path) {
        let data = base64::engine::general_purpose::STANDARD.encode(&bytes);
        return json!({ "type": "image_url", "image_url": { "url": format!("data:{media};base64,{data}") } });
    }
    match String::from_utf8(bytes) {
        Ok(text) if text.len() <= MAX_TEXT_FILE => json!({ "type": "text", "text": file_text(&name, &text) }),
        _ => json!({ "type": "text", "text": file_foreign(&name) }),
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
    use crate::tools::Tools;

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
            ..Turn::default()
        };
        turn(
            &provider,
            None,
            "llama-test",
            &mut history,
            &t,
            tools::Kit {
                tools: &Vec::new(),
                outside: &Default::default(),
                frozen: None,
            },
            &tx,
        )
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
        turn(
            &provider,
            None,
            "llama-test",
            &mut history,
            &Turn::default(),
            tools::Kit {
                tools: &Vec::new(),
                outside: &Default::default(),
                frozen: None,
            },
            &tx,
        )
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
        turn(
            &provider,
            None,
            "qwen3",
            &mut history,
            &Turn::default(),
            tools::Kit {
                tools: &Vec::new(),
                outside: &Default::default(),
                frozen: None,
            },
            &tx,
        )
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
        let err = turn(
            &provider,
            None,
            "m",
            &mut Vec::new(),
            &Turn::default(),
            tools::Kit {
                tools: &Vec::new(),
                outside: &Default::default(),
                frozen: None,
            },
            &tx,
        )
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
        let dir = std::env::temp_dir().join("vults-openai-test");
        std::fs::create_dir_all(&dir).unwrap();
        let png = dir.join("1700000000001-shot.png");
        std::fs::write(&png, [137u8, 80, 78, 71]).unwrap();
        let content = user_content(&Turn {
            text: String::new(),
            files: vec![png],
            ..Turn::default()
        });
        assert!(
            content[0]["image_url"]["url"]
                .as_str()
                .unwrap()
                .starts_with("data:image/png;base64,")
        );
        assert_eq!(content[1]["text"], "What is in this?");
    }

    #[test]
    fn usage_is_read_when_the_server_sends_it() {
        let ev = |data: &str| event_text(&format!("data: {data}\n\n"));
        assert_eq!(
            ev(
                r#"{"model":"llama3","choices":[],"usage":{"prompt_tokens":50,"completion_tokens":7,"prompt_tokens_details":{"cached_tokens":40}}}"#
            ),
            Event::Usage(Usage {
                model: Some("llama3".into()),
                input: 50,
                output: 7,
                cache_read: 40,
                ..Usage::default()
            })
        );
    }

    /// M2: the system message is the same bytes on every turn.
    #[tokio::test]
    async fn the_system_message_stays_byte_identical_between_turns() {
        let answer = || {
            crate::mock::openai(&[
                json!({"choices": [{"delta": {"content": "ok"}}]}),
                json!({"choices": [{"delta": {}, "finish_reason": "stop"}]}),
            ])
        };
        let (provider, seen) = crate::mock::scripted("openrouter", vec![answer(), answer()]).await;
        let (tx, _rx) = mpsc::channel(32);
        let mut history = Vec::new();
        for today in ["<today>Sunday</today>", "<today>Monday</today>"] {
            turn(
                &provider,
                Some("key"),
                "m",
                &mut history,
                &Turn {
                    text: "hi".into(),
                    context: Some(today.into()),
                    ..Turn::default()
                },
                tools::Kit {
                    tools: &Vec::new(),
                    outside: &Default::default(),
                    frozen: Some("# Your memory"),
                },
                &tx,
            )
            .await
            .unwrap();
        }
        let bodies = seen.lock().unwrap().clone();
        assert_eq!(bodies[0]["messages"][0], bodies[1]["messages"][0]);
        assert!(
            bodies[0]["messages"][0]["content"]
                .as_str()
                .unwrap()
                .ends_with("# Your memory")
        );
        let last = bodies[1]["messages"].as_array().unwrap().last().unwrap().clone();
        assert_eq!(last["content"][0]["text"], "<today>Monday</today>");
    }

    #[tokio::test]
    async fn chained_tool_calls_on_chat_completions() {
        let tools: Tools = vec![std::sync::Arc::new(crate::tools::tests::Echo)];
        let call = |id: &str, text: &str| {
            crate::mock::openai(&[
                json!({"choices": [{"delta": {"tool_calls": [{"index": 0, "id": id, "type": "function", "function": {"name": "echo", "arguments": ""}}]}}]}),
                json!({"choices": [{"delta": {"tool_calls": [{"index": 0, "function": {"arguments": format!("{{\"text\":\"{text}\"}}")}}]}}]}),
                json!({"choices": [{"delta": {}, "finish_reason": "tool_calls"}]}),
            ])
        };
        let answer = crate::mock::openai(&[
            json!({"choices": [{"delta": {"content": "done"}}]}),
            json!({"choices": [{"delta": {}, "finish_reason": "stop"}]}),
        ]);
        let (provider, seen) =
            crate::mock::scripted("openrouter", vec![call("c1", "one"), call("c2", "two"), answer]).await;
        let (tx, _rx) = mpsc::channel(32);
        let mut history = Vec::new();
        turn(
            &provider,
            Some("key"),
            "m",
            &mut history,
            &Turn::default(),
            tools::Kit {
                tools: &tools,
                outside: &Default::default(),
                frozen: None,
            },
            &tx,
        )
        .await
        .unwrap();
        let bodies = seen.lock().unwrap().clone();
        assert_eq!(bodies.len(), 3);
        assert_eq!(bodies[0]["tools"][0]["function"]["name"], "echo");
        let sent = bodies[1]["messages"].as_array().unwrap();
        let result = sent.last().unwrap();
        assert_eq!(result["role"], "tool");
        assert_eq!(result["tool_call_id"], "c1");
        assert_eq!(result["content"], "echo: one");
        assert_eq!(
            sent[sent.len() - 2]["tool_calls"][0]["function"]["arguments"],
            r#"{"text":"one"}"#
        );
        assert_eq!(
            history.last().unwrap(),
            &json!({"role": "assistant", "content": "done"})
        );
    }

    #[test]
    fn pieces_gather_with_or_without_an_index() {
        let piece = |id: &str, name: &str, args: &str| Call {
            id: id.into(),
            name: name.into(),
            arguments: args.into(),
        };
        // Indexed, the id and name repeated in every piece (some servers do): read once.
        let mut calls = Vec::new();
        gather(&mut calls, Some(0), piece("a", "echo", "{\"te"));
        gather(&mut calls, Some(1), piece("b", "echo", "{}"));
        gather(&mut calls, Some(0), piece("a", "echo", "xt\":1}"));
        assert_eq!(
            calls,
            [piece("a", "echo", "{\"text\":1}"), piece("b", "echo", "{}")]
        );
        // No index: a new id starts a new call; none at all gets one made up.
        let mut calls = Vec::new();
        gather(&mut calls, None, piece("a", "echo", "{"));
        gather(&mut calls, None, piece("", "", "}"));
        gather(&mut calls, None, piece("b", "echo", "{}"));
        assert_eq!(calls, [piece("a", "echo", "{}"), piece("b", "echo", "{}")]);
        let mut calls = Vec::new();
        gather(&mut calls, Some(0), piece("", "echo", "{}"));
        assert_eq!(calls[0].id, "call_0");
    }

    #[tokio::test]
    async fn a_call_cut_short_never_runs_and_one_without_done_still_does() {
        let tools: Tools = vec![std::sync::Arc::new(crate::tools::tests::Echo)];
        // Cut by the token limit midway through its arguments.
        let cut = crate::mock::openai(&[
            json!({"choices": [{"delta": {"tool_calls": [{"index": 0, "id": "c1", "function": {"name": "echo", "arguments": "{\"te"}}]}}]}),
            json!({"choices": [{"delta": {}, "finish_reason": "length"}]}),
        ]);
        let (provider, seen) = crate::mock::scripted("openrouter", vec![cut]).await;
        let (tx, _rx) = mpsc::channel(16);
        let err = turn(
            &provider,
            Some("k"),
            "m",
            &mut Vec::new(),
            &Turn::default(),
            tools::Kit {
                tools: &tools,
                outside: &Default::default(),
                frozen: None,
            },
            &tx,
        )
        .await
        .unwrap_err();
        assert!(err.contains("stopped mid-reply"), "{err}");
        assert_eq!(
            seen.lock().unwrap().len(),
            1,
            "nothing ran, nothing was sent back"
        );
        // A server that ends with `finish_reason` and no `[DONE]`.
        let no_done = |chunks: &[Value]| {
            chunks
                .iter()
                .map(|c| format!("data: {c}\n\n"))
                .collect::<String>()
        };
        let call = no_done(&[
            json!({"choices": [{"delta": {"tool_calls": [{"index": 0, "id": "c1", "function": {"name": "echo", "arguments": "{\"text\":\"hi\"}"}}]}}]}),
            json!({"choices": [{"delta": {}, "finish_reason": "tool_calls"}]}),
        ]);
        let answer = no_done(&[
            json!({"choices": [{"delta": {"content": "done"}}]}),
            json!({"choices": [{"delta": {}, "finish_reason": "stop"}]}),
        ]);
        let (provider, seen) = crate::mock::scripted("openrouter", vec![call, answer]).await;
        let mut history = Vec::new();
        turn(
            &provider,
            Some("k"),
            "m",
            &mut history,
            &Turn::default(),
            tools::Kit {
                tools: &tools,
                outside: &Default::default(),
                frozen: None,
            },
            &tx,
        )
        .await
        .unwrap();
        let second = seen.lock().unwrap()[1]["messages"].clone();
        let sent = second.as_array().unwrap();
        assert_eq!(
            sent[sent.len() - 2]["content"],
            Value::Null,
            "no text beside the calls is null"
        );
        assert_eq!(sent[sent.len() - 1]["content"], "echo: hi");
        assert_eq!(history.last().unwrap()["content"], "done");
    }
}
