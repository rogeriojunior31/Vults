//! Claude through the Messages API with the user's own key (read from the OS keyring for each
//! turn). It may call the conversation's read-only tools in a loop (`tools`); nothing here can run
//! a command or touch a file. Dropped files go in the message itself.
//!
//! The history is append-only and replayed verbatim, thinking blocks included: the API rejects
//! (or drops) a thinking block whose earlier conversation changed.

use std::path::Path;

use base64::Engine;
use serde_json::{Value, json};
use tokio::sync::mpsc;

use crate::providers::{Provider, client};
use crate::tools;
use crate::{Delta, Turn, Usage};

/// Thinking counts toward it, so it is sized for the thinking as well as the reply.
const MAX_TOKENS: u32 = 64_000;
/// With `fallbacks: "default"`, a safety decline is retried on the model Anthropic recommends,
/// inside the same call, instead of ending the turn.
const FALLBACK_BETA: &str = "server-side-fallback-2026-07-01";
/// Text files bigger than this are named, not inlined.
pub(crate) const MAX_TEXT_FILE: usize = 512 * 1024;

pub(crate) const PERSONA: &str = concat!(
    persona!(),
    " Here you have no tools: you can't run commands or open files, only read what the user \
attaches to the message. When a task needs a command or an edit, say what to run."
);

/// With tools offered: they only read, so commands and edits still are the user's to run.
pub(crate) const PERSONA_TOOLS: &str = concat!(
    persona!(),
    " Use the tools offered when they help: they only read. You can't run commands or edit files; \
when a task needs that, say what to run."
);

/// [`MAX_TOKENS`], or less for a model that writes less: the picker lists every model the key
/// can use, and asking an older one for more than its cap is a 400 on every turn. The cap comes
/// from `GET /models/{id}`, once per model; when that can't be read, [`MAX_TOKENS`].
async fn max_tokens(provider: &Provider, key: &str, model: &str) -> u32 {
    use std::collections::HashMap;
    use std::sync::{Mutex, OnceLock};
    static CAPS: OnceLock<Mutex<HashMap<String, u32>>> = OnceLock::new();
    let caps = CAPS.get_or_init(Default::default);
    if let Some(cap) = caps.lock().ok().and_then(|c| c.get(model).copied()) {
        return cap;
    }
    // A model id goes into the URL as one path segment: nothing that could leave it.
    if model.is_empty()
        || !model
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "-._:@".contains(c))
    {
        return MAX_TOKENS;
    }
    let read = async {
        let response = client()
            .get(format!("{}/models/{model}", provider.base_url))
            .header("x-api-key", key)
            .header("anthropic-version", "2023-06-01")
            .send()
            .await
            .ok()?
            .error_for_status()
            .ok()?;
        response.json::<Value>().await.ok()?["max_tokens"].as_u64()
    };
    let Some(cap) = tokio::time::timeout(std::time::Duration::from_secs(10), read)
        .await
        .ok()
        .flatten()
    else {
        return MAX_TOKENS;
    };
    let cap = u32::try_from(cap).unwrap_or(u32::MAX).clamp(1, MAX_TOKENS);
    if let Ok(mut c) = caps.lock() {
        c.insert(model.to_string(), cap);
    }
    cap
}

/// The model's window, for [`tools::fit`]: every Claude the chat offers has at least this.
const WINDOW: usize = 200_000;

/// One turn: the model may call the conversation's tools, up to [`tools::MAX_STEPS`] times, each
/// result handed back before it goes on. `history` only grows when the turn succeeds: a failed or
/// stopped turn leaves it as it was, so the next request replays exactly what the API has seen.
pub(crate) async fn turn(
    provider: &Provider,
    key: &str,
    model: &str,
    history: &mut Vec<Value>,
    turn: &Turn,
    kit: tools::Kit<'_>,
    out: &mpsc::Sender<Delta>,
) -> Result<(), String> {
    let (tools, outside) = (kit.tools, kit.outside);
    let mut messages = history.clone();
    messages.push(json!({ "role": "user", "content": user_content(turn) }));
    let max = max_tokens(provider, key, model).await;
    let mut usage = Usage::default();
    for step in 0..=tools::MAX_STEPS {
        tools::fit(&mut messages, WINDOW);
        let stream = call(provider, key, model, max, &messages, kit, out).await?;
        usage = usage.plus(&stream.usage);
        match stream.stop_reason.as_deref() {
            Some("refusal") => return Err("Claude declined to answer this.".into()),
            None => return Err("The Claude API stopped mid-reply.".into()),
            Some("tool_use")
                if !tools.is_empty() && stream.blocks.iter().any(|b| b["type"] == "tool_use") =>
            {
                if step == tools::MAX_STEPS {
                    return Err(format!("Zeca stopped after {} tool calls.", tools::MAX_STEPS));
                }
                let mut blocks = stream.blocks;
                let mut results = Vec::new();
                for b in blocks.iter_mut().filter(|b| b["type"] == "tool_use") {
                    let name = b["name"].as_str().unwrap_or_default().to_string();
                    let _ = out.send(Delta::ToolState { active: true }).await;
                    let _ = out.send(Delta::Tool { tool: name.clone() }).await;
                    // Input that never read as JSON: the tool is not run with one it wasn't sent.
                    let broken = b.as_object_mut().and_then(|o| o.remove(PARTIAL)).is_some();
                    let (text, error) = if broken {
                        b["input"] = json!({});
                        ("The arguments were not valid JSON.".to_string(), true)
                    } else {
                        tools::run(tools, &name, &b["input"]).await
                    };
                    if !error && tools::outside(tools, &name) {
                        crate::taint(outside, name.clone());
                    }
                    let _ = out.send(Delta::ToolState { active: false }).await;
                    results.push(json!({ "type": "tool_result", "tool_use_id": b["id"], "content": text, "is_error": error }));
                }
                messages.push(json!({ "role": "assistant", "content": blocks }));
                messages.push(json!({ "role": "user", "content": results }));
            }
            Some(_) => {
                // A reply cut short (`max_tokens`) may hold a call never answered: kept, it would
                // make every later request a 400. Only what the user can read is kept.
                let blocks: Vec<Value> = stream
                    .blocks
                    .into_iter()
                    .filter(|b| b["type"] != "tool_use" && !b.is_null())
                    .collect();
                if blocks.is_empty() {
                    return Err("The Claude API stopped mid-reply.".into());
                }
                if !usage.is_empty() {
                    let _ = out.send(Delta::Usage(usage)).await;
                }
                messages.push(json!({ "role": "assistant", "content": blocks }));
                *history = messages;
                return Ok(());
            }
        }
    }
    Err(format!("Zeca stopped after {} tool calls.", tools::MAX_STEPS))
}

/// One request: streams the reply's text to the chat and returns the whole reply.
async fn call(
    provider: &Provider,
    key: &str,
    model: &str,
    max: u32,
    messages: &[Value],
    kit: tools::Kit<'_>,
    out: &mpsc::Sender<Delta>,
) -> Result<Stream, String> {
    let mut body = json!({
        "model": model,
        "max_tokens": max,
        "stream": true,
        "fallbacks": "default",
        "system": kit.system(PERSONA, PERSONA_TOOLS),
        "messages": messages,
        // The whole history, attachments included, goes again every turn: cached up to its last
        // block, the next turn reads it at a tenth of the price instead of paying for it again.
        "cache_control": { "type": "ephemeral" },
    });
    let tools = kit.tools;
    if !tools.is_empty() {
        body["tools"] = tools
            .iter()
            .map(|t| json!({ "name": t.name(), "description": t.description(), "input_schema": t.schema() }))
            .collect();
    }
    let mut response = client()
        .post(format!("{}/messages", provider.base_url))
        .header("x-api-key", key)
        .header("anthropic-version", "2023-06-01")
        .header("anthropic-beta", FALLBACK_BETA)
        .json(&body)
        .send()
        .await
        .map_err(|e| {
            if e.is_connect() || e.is_timeout() {
                "Can't reach the Claude API. Check the connection.".to_string()
            } else {
                "The request to the Claude API failed.".to_string()
            }
        })?;
    let status = response.status().as_u16();
    if status != 200 {
        let retry = response
            .headers()
            .get("retry-after")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<u64>().ok());
        let text = response.text().await.unwrap_or_default();
        return Err(http_error(status, retry, &text));
    }
    let mut stream = Stream::default();
    let mut events = crate::SseEvents::default();
    loop {
        let chunk = response
            .chunk()
            .await
            .map_err(|_| "The Claude API stopped mid-reply.".to_string())?;
        let Some(chunk) = chunk else { break };
        for event in events.push(&chunk) {
            if let Some(text) = stream.event(&event)? {
                let _ = out.send(Delta::Text { text }).await;
            }
        }
    }
    Ok(stream)
}

/// Where a tool call's input gathers while it streams; never sent back.
const PARTIAL: &str = "_partial_json";

/// The reply as it streams: content blocks rebuilt from their deltas, to replay next turn.
#[derive(Default, Debug)]
struct Stream {
    blocks: Vec<Value>,
    stop_reason: Option<String>,
    usage: Usage,
}

impl Stream {
    /// Handles one server-sent event; returns reply text to show, if any.
    fn event(&mut self, raw: &str) -> Result<Option<String>, String> {
        let data: String = raw
            .lines()
            .filter_map(|l| l.strip_prefix("data:"))
            .map(str::trim_start)
            .collect();
        let Ok(v) = serde_json::from_str::<Value>(&data) else {
            return Ok(None);
        };
        match v["type"].as_str().unwrap_or_default() {
            "content_block_start" => {
                let i = v["index"].as_u64().unwrap_or(self.blocks.len() as u64) as usize;
                if self.blocks.len() <= i {
                    self.blocks.resize(i + 1, Value::Null);
                }
                self.blocks[i] = v["content_block"].clone();
            }
            "content_block_delta" => {
                let i = v["index"].as_u64().unwrap_or(0) as usize;
                let delta = &v["delta"];
                let Some(block) = self.blocks.get_mut(i) else {
                    return Ok(None);
                };
                let (field, piece) = match delta["type"].as_str().unwrap_or_default() {
                    "text_delta" => ("text", &delta["text"]),
                    "thinking_delta" => ("thinking", &delta["thinking"]),
                    "signature_delta" => ("signature", &delta["signature"]),
                    // A tool call's input comes as pieces of JSON, read whole at the block's stop.
                    "input_json_delta" => (PARTIAL, &delta["partial_json"]),
                    _ => return Ok(None),
                };
                let piece = piece.as_str().unwrap_or_default();
                let mut whole = block[field].as_str().unwrap_or_default().to_string();
                whole.push_str(piece);
                block[field] = Value::String(whole);
                if field == "text" && !piece.is_empty() {
                    return Ok(Some(piece.to_string()));
                }
            }
            "content_block_stop" => {
                let i = v["index"].as_u64().unwrap_or(0) as usize;
                // Read whole; JSON that doesn't parse stays where it gathered, and the loop answers
                // that call with an error rather than run it.
                if let Some(block) = self.blocks.get_mut(i)
                    && let Some(raw) = block.get(PARTIAL).and_then(Value::as_str)
                    && let Ok(input) = serde_json::from_str::<Value>(if raw.is_empty() { "{}" } else { raw })
                {
                    block["input"] = input;
                    if let Some(b) = block.as_object_mut() {
                        b.remove(PARTIAL);
                    }
                }
            }
            "message_start" => {
                let message = &v["message"];
                self.usage = Usage::anthropic(&message["usage"]);
                self.usage.model = message["model"].as_str().map(str::to_owned);
            }
            "message_delta" => {
                if let Some(reason) = v["delta"]["stop_reason"].as_str() {
                    self.stop_reason = Some(reason.to_string());
                }
                // Cumulative: the last one is the turn's.
                if let Some(n) = v["usage"]["output_tokens"].as_u64() {
                    self.usage.output = n;
                }
            }
            "error" => {
                return Err(api_error(&v["error"]));
            }
            _ => {}
        }
        Ok(None)
    }
}

/// Files first, then the question. Images and PDFs go as their own blocks, text files inline.
fn user_content(turn: &Turn) -> Vec<Value> {
    let mut content: Vec<Value> = turn.files.iter().map(|f| file_block(f)).collect();
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

/// What a turn with only files and no words asks.
pub(crate) const WHAT_IS_IN_THIS: &str = "What is in this?";

/// A text file, inline, as the model reads it.
pub(crate) fn file_text(name: &str, text: &str) -> String {
    format!("<file name=\"{name}\">\n{text}\n</file>")
}

/// A file that could not be opened.
pub(crate) fn file_unreadable(name: &str) -> String {
    format!("(The file \"{name}\" could not be read.)")
}

/// A file this chat can't pass on (not text, or too big).
pub(crate) fn file_foreign(name: &str) -> String {
    format!("(The user attached \"{name}\", a file this chat can't read.)")
}

fn file_block(path: &Path) -> Value {
    let name = display_name(path);
    let base64 = |bytes: &[u8]| base64::engine::general_purpose::STANDARD.encode(bytes);
    let Ok(bytes) = std::fs::read(path) else {
        return json!({ "type": "text", "text": file_unreadable(&name) });
    };
    if let Some(media) = image_type(path) {
        return json!({
            "type": "image",
            "source": { "type": "base64", "media_type": media, "data": base64(&bytes) },
        });
    }
    if has_ext(path, "pdf") {
        return json!({
            "type": "document",
            "source": { "type": "base64", "media_type": "application/pdf", "data": base64(&bytes) },
            "title": name,
        });
    }
    match String::from_utf8(bytes) {
        Ok(text) if text.len() <= MAX_TEXT_FILE => json!({ "type": "text", "text": file_text(&name, &text) }),
        _ => json!({ "type": "text", "text": file_foreign(&name) }),
    }
}

/// The name the user dropped, without the inbox's time stamp.
pub(crate) fn display_name(path: &Path) -> String {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    match name.split_once('-') {
        Some((stamp, rest)) if !rest.is_empty() && stamp.chars().all(|c| c.is_ascii_digit()) => {
            rest.to_string()
        }
        _ => name,
    }
}

fn has_ext(path: &Path, ext: &str) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case(ext))
}

pub(crate) fn image_type(path: &Path) -> Option<&'static str> {
    let ext = path.extension()?.to_str()?.to_lowercase();
    Some(match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        _ => return None,
    })
}

fn http_error(status: u16, retry_after: Option<u64>, body: &str) -> String {
    let error = serde_json::from_str::<Value>(body)
        .map(|v| v["error"].clone())
        .unwrap_or(Value::Null);
    match status {
        401 => "The API key was rejected. Check it in Settings.".into(),
        403 => "This API key can't use this model.".into(),
        429 => match retry_after {
            Some(s) => format!("The API rate limit was reached. Try again in {s} s."),
            None => "The API rate limit was reached. Try again in a moment.".into(),
        },
        529 => "Claude is overloaded right now. Try again in a moment.".into(),
        400 | 404 | 413 if error.is_object() => api_error(&error),
        s if s >= 500 => format!("The Claude API had a problem ({s}). Try again in a moment."),
        s => format!("The Claude API answered {s}."),
    }
}

/// An error object from the API, as a sentence for the bubble.
fn api_error(error: &Value) -> String {
    match error["type"].as_str().unwrap_or_default() {
        "overloaded_error" => "Claude is overloaded right now. Try again in a moment.".into(),
        "rate_limit_error" => "The API rate limit was reached. Try again in a moment.".into(),
        "authentication_error" => "The API key was rejected. Check it in Settings.".into(),
        _ => match error["message"].as_str() {
            Some(m) if !m.is_empty() => format!("Claude API: {m}"),
            _ => "The Claude API returned an error.".into(),
        },
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_stream_says_what_the_turn_cost() {
        let mut s = Stream::default();
        let ev = |v: serde_json::Value| format!("event: x\ndata: {v}\n");
        s.event(&ev(json!({"type": "message_start", "message": {"id": "m", "model": "claude-opus-5-5", "usage": {"input_tokens": 20, "cache_read_input_tokens": 5000, "cache_creation_input_tokens": 100, "output_tokens": 1}}}))).unwrap();
        s.event(&ev(json!({"type": "message_delta", "delta": {"stop_reason": "end_turn"}, "usage": {"output_tokens": 88}}))).unwrap();
        assert_eq!(
            s.usage,
            Usage {
                model: Some("claude-opus-5-5".into()),
                input: 20 + 5000 + 100,
                output: 88,
                cache_read: 5000,
                cache_write: 100,
                cost_usd: None,
                session_cost_usd: None,
            }
        );
    }

    use super::*;
    use crate::tools::Tools;

    fn feed(stream: &mut Stream, events: &[Value]) -> String {
        let mut shown = String::new();
        for e in events {
            let raw = format!("event: {}\ndata: {e}\n\n", e["type"].as_str().unwrap());
            if let Some(t) = stream.event(&raw).unwrap() {
                shown.push_str(&t);
            }
        }
        shown
    }

    #[test]
    fn rebuilds_the_reply_blocks_from_the_stream() {
        let mut s = Stream::default();
        let shown = feed(
            &mut s,
            &[
                json!({"type": "message_start", "message": {"id": "m"}}),
                json!({"type": "content_block_start", "index": 0, "content_block": {"type": "thinking", "thinking": "", "signature": ""}}),
                json!({"type": "content_block_delta", "index": 0, "delta": {"type": "signature_delta", "signature": "sig"}}),
                json!({"type": "content_block_stop", "index": 0}),
                json!({"type": "content_block_start", "index": 1, "content_block": {"type": "text", "text": ""}}),
                json!({"type": "ping"}),
                json!({"type": "content_block_delta", "index": 1, "delta": {"type": "text_delta", "text": "Hel"}}),
                json!({"type": "content_block_delta", "index": 1, "delta": {"type": "text_delta", "text": "lo"}}),
                json!({"type": "content_block_stop", "index": 1}),
                json!({"type": "message_delta", "delta": {"stop_reason": "end_turn"}}),
                json!({"type": "message_stop"}),
            ],
        );
        assert_eq!(shown, "Hello");
        assert_eq!(s.stop_reason.as_deref(), Some("end_turn"));
        assert_eq!(
            s.blocks,
            vec![
                json!({"type": "thinking", "thinking": "", "signature": "sig"}),
                json!({"type": "text", "text": "Hello"}),
            ]
        );
    }

    #[test]
    fn a_stream_error_ends_the_turn() {
        let mut s = Stream::default();
        let raw = "event: error\ndata: {\"type\":\"error\",\"error\":{\"type\":\"overloaded_error\",\"message\":\"Overloaded\"}}\n\n";
        assert_eq!(
            s.event(raw).unwrap_err(),
            "Claude is overloaded right now. Try again in a moment."
        );
    }

    #[test]
    fn http_errors_read_as_sentences() {
        assert!(http_error(401, None, "").contains("rejected"));
        assert!(http_error(429, Some(12), "").contains("12 s"));
        assert_eq!(
            http_error(
                400,
                None,
                r#"{"type":"error","error":{"type":"invalid_request_error","message":"bad"}}"#
            ),
            "Claude API: bad"
        );
        assert!(http_error(503, None, "").contains("503"));
    }

    #[test]
    fn files_become_blocks() {
        let dir = std::env::temp_dir().join("vults-api-test");
        std::fs::create_dir_all(&dir).unwrap();
        let txt = dir.join("1700000000000-notes.md");
        let png = dir.join("1700000000001-shot.png");
        let bin = dir.join("1700000000002-blob.bin");
        std::fs::write(&txt, "hi").unwrap();
        std::fs::write(&png, [137u8, 80, 78, 71]).unwrap();
        std::fs::write(&bin, [0xffu8, 0xfe, 0x00]).unwrap();
        let content = user_content(&Turn {
            text: "look".into(),
            files: vec![txt, png, bin],
            ..Turn::default()
        });
        assert_eq!(content[0]["text"], "<file name=\"notes.md\">\nhi\n</file>");
        assert_eq!(content[1]["type"], "image");
        assert_eq!(content[1]["source"]["media_type"], "image/png");
        assert!(content[2]["text"].as_str().unwrap().contains("\"blob.bin\""));
        assert_eq!(content[3], json!({"type": "text", "text": "look"}));
    }

    /// A stand-in for the Claude API on this machine, for one model whose output stops at `cap`
    /// tokens: it lists the model, refuses a larger `max_tokens` the way the API does (400), and
    /// streams "ok" otherwise. Hands back the `max_tokens` each request asked for.
    async fn claude_api(
        model: &'static str,
        cap: u64,
    ) -> (Provider, std::sync::Arc<std::sync::Mutex<Vec<u64>>>) {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/v1", listener.local_addr().unwrap());
        let provider = Provider {
            base_url: Box::leak(url.into_boxed_str()),
            ..*crate::providers::find("anthropic").unwrap()
        };
        let asked = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let seen = asked.clone();
        tokio::spawn(async move {
            loop {
                let (mut sock, _) = listener.accept().await.unwrap();
                let mut raw = Vec::new();
                let mut buf = [0u8; 65536];
                let (head, body) = loop {
                    let n = sock.read(&mut buf).await.unwrap();
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
                                .map(|v| v.trim().parse().unwrap())
                        })
                        .unwrap_or(0);
                    if raw.len() >= end + 4 + len {
                        break (text[..end].to_string(), text[end + 4..].to_string());
                    }
                };
                let (status, kind, reply) = if head.starts_with(&format!("GET /v1/models/{model}")) {
                    (
                        200,
                        "application/json",
                        json!({ "id": model, "max_tokens": cap }).to_string(),
                    )
                } else {
                    let wanted = serde_json::from_str::<Value>(&body).unwrap()["max_tokens"]
                        .as_u64()
                        .unwrap();
                    seen.lock().unwrap().push(wanted);
                    if wanted > cap {
                        let message = format!(
                            "max_tokens: {wanted} > {cap}, which is the maximum allowed number of output tokens for {model}"
                        );
                        (400, "application/json", json!({ "type": "error", "error": { "type": "invalid_request_error", "message": message } }).to_string())
                    } else {
                        let events = [
                            json!({"type": "message_start", "message": {"id": "m"}}),
                            json!({"type": "content_block_start", "index": 0, "content_block": {"type": "text", "text": ""}}),
                            json!({"type": "content_block_delta", "index": 0, "delta": {"type": "text_delta", "text": "ok"}}),
                            json!({"type": "content_block_stop", "index": 0}),
                            json!({"type": "message_delta", "delta": {"stop_reason": "end_turn"}}),
                            json!({"type": "message_stop"}),
                        ];
                        let sse: String = events
                            .iter()
                            .map(|e| format!("event: {}\ndata: {e}\n\n", e["type"].as_str().unwrap()))
                            .collect();
                        (200, "text/event-stream", sse)
                    }
                };
                let head = format!(
                    "HTTP/1.1 {status} X\r\ncontent-type: {kind}\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
                    reply.len()
                );
                sock.write_all(head.as_bytes()).await.unwrap();
                sock.write_all(reply.as_bytes()).await.unwrap();
            }
        });
        (provider, asked)
    }

    #[tokio::test]
    async fn an_older_model_gets_no_more_tokens_than_it_can_write() {
        // Claude 3 Haiku writes at most 4096 tokens; asking it for 64000 is a 400 on every turn.
        let (provider, asked) = claude_api("claude-3-haiku-20240307", 4096).await;
        let (tx, mut rx) = mpsc::channel(16);
        let mut history = Vec::new();
        turn(
            &provider,
            "key",
            "claude-3-haiku-20240307",
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
        assert_eq!(shown, "ok");
        assert_eq!(*asked.lock().unwrap(), [4096]);
    }

    #[tokio::test]
    async fn a_current_model_keeps_the_full_budget() {
        let (provider, asked) = claude_api("claude-opus-5-5", 128_000).await;
        let (tx, _rx) = mpsc::channel(16);
        let mut history = Vec::new();
        turn(
            &provider,
            "key",
            "claude-opus-5-5",
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
        assert_eq!(*asked.lock().unwrap(), [u64::from(MAX_TOKENS)]);
    }

    fn tool_use(id: &str, input: &str) -> String {
        crate::mock::anthropic(&[
            json!({"type": "message_start", "message": {"id": "m", "model": "claude-opus-5-5", "usage": {"input_tokens": 10, "output_tokens": 1}}}),
            json!({"type": "content_block_start", "index": 0, "content_block": {"type": "tool_use", "id": id, "name": "echo", "input": {}}}),
            json!({"type": "content_block_delta", "index": 0, "delta": {"type": "input_json_delta", "partial_json": &input[..input.len() / 2]}}),
            json!({"type": "content_block_delta", "index": 0, "delta": {"type": "input_json_delta", "partial_json": &input[input.len() / 2..]}}),
            json!({"type": "content_block_stop", "index": 0}),
            json!({"type": "message_delta", "delta": {"stop_reason": "tool_use"}, "usage": {"output_tokens": 5}}),
        ])
    }

    fn says(text: &str) -> String {
        crate::mock::anthropic(&[
            json!({"type": "message_start", "message": {"id": "m", "usage": {"input_tokens": 10, "output_tokens": 1}}}),
            json!({"type": "content_block_start", "index": 0, "content_block": {"type": "text", "text": ""}}),
            json!({"type": "content_block_delta", "index": 0, "delta": {"type": "text_delta", "text": text}}),
            json!({"type": "content_block_stop", "index": 0}),
            json!({"type": "message_delta", "delta": {"stop_reason": "end_turn"}, "usage": {"output_tokens": 3}}),
        ])
    }

    /// M2: the system prompt is the same bytes on every turn of a conversation; what changes
    /// (`<today>`, `<flock>`) rides in front of each turn's words.
    #[tokio::test]
    async fn the_system_prompt_stays_byte_identical_between_turns() {
        let (provider, seen) = crate::mock::scripted("anthropic", vec![says("one"), says("two")]).await;
        let (tx, _rx) = mpsc::channel(32);
        let mut history = Vec::new();
        for (text, today) in [
            ("hi", "<today>Sunday</today>"),
            ("again", "<today>Monday</today>"),
        ] {
            turn(
                &provider,
                "key",
                "claude-opus-5-5",
                &mut history,
                &Turn {
                    text: text.into(),
                    context: Some(today.into()),
                    ..Turn::default()
                },
                tools::Kit {
                    tools: &Vec::new(),
                    outside: &Default::default(),
                    frozen: Some("# Your memory\n\n- Works on Vults."),
                },
                &tx,
            )
            .await
            .unwrap();
        }
        let seen = seen.lock().unwrap();
        let systems: Vec<&Value> = seen.iter().map(|b| &b["system"]).collect();
        assert_eq!(systems.len(), 2);
        assert_eq!(systems[0], systems[1]);
        assert!(
            systems[0]
                .as_str()
                .unwrap()
                .ends_with("\n\n# Your memory\n\n- Works on Vults.")
        );
        let last = seen[1]["messages"].as_array().unwrap().last().unwrap().clone();
        assert_eq!(last["content"][0]["text"], "<today>Monday</today>");
        assert_eq!(last["content"][1]["text"], "again");
    }

    #[tokio::test]
    async fn chained_tool_calls_run_and_the_turn_ends_with_the_answer() {
        let tools: Tools = vec![std::sync::Arc::new(crate::tools::tests::Echo)];
        let (provider, seen) = crate::mock::scripted(
            "anthropic",
            vec![
                tool_use("t1", r#"{"text":"one"}"#),
                tool_use("t2", r#"{"text":"two"}"#),
                says("done"),
            ],
        )
        .await;
        let (tx, mut rx) = mpsc::channel(32);
        let mut history = Vec::new();
        turn(
            &provider,
            "key",
            "claude-opus-5-5",
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
        assert_eq!(bodies[0]["tools"][0]["name"], "echo");
        // Each result goes back with its call, the input read whole from its pieces.
        let second = &bodies[1]["messages"];
        let last = second.as_array().unwrap().last().unwrap();
        assert_eq!(last["content"][0]["tool_use_id"], "t1");
        assert_eq!(last["content"][0]["content"], "echo: one");
        let call = &second.as_array().unwrap()[second.as_array().unwrap().len() - 2]["content"][0];
        assert_eq!(call["input"], json!({"text": "one"}));
        assert!(call.get(PARTIAL).is_none(), "the gathered JSON never goes back");
        // The kept history is the whole exchange; the chat heard each tool and the answer.
        assert_eq!(history.len(), 6);
        let mut tools_heard = 0;
        let mut phases = Vec::new();
        let mut usage = None;
        while let Ok(d) = rx.try_recv() {
            match d {
                Delta::Tool { tool } => {
                    assert_eq!(tool, "echo");
                    tools_heard += 1;
                }
                Delta::ToolState { active } => phases.push(active),
                Delta::Usage(u) => usage = Some(u),
                _ => {}
            }
        }
        assert_eq!(tools_heard, 2);
        assert_eq!(phases, [true, false, true, false]);
        // The turn's cost is every step's.
        assert_eq!(usage.unwrap().output, 5 + 5 + 3);
    }

    #[tokio::test]
    async fn a_loop_past_its_steps_ends_and_keeps_nothing() {
        let tools: Tools = vec![std::sync::Arc::new(crate::tools::tests::Echo)];
        let calls: Vec<String> = (0..=tools::MAX_STEPS)
            .map(|i| tool_use(&format!("t{i}"), r#"{"text":"again"}"#))
            .collect();
        let (provider, seen) = crate::mock::scripted("anthropic", calls).await;
        let (tx, _rx) = mpsc::channel(64);
        let mut history = vec![json!({"role": "user", "content": "before"})];
        let err = turn(
            &provider,
            "key",
            "claude-opus-5-5",
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
        .unwrap_err();
        assert!(err.contains("stopped after 8"), "{err}");
        assert_eq!(seen.lock().unwrap().len(), tools::MAX_STEPS + 1);
        assert_eq!(
            history,
            [json!({"role": "user", "content": "before"})],
            "a failed turn keeps nothing"
        );
    }

    #[tokio::test]
    async fn without_tools_nothing_is_offered() {
        let (provider, seen) = crate::mock::scripted("anthropic", vec![says("hi")]).await;
        let (tx, _rx) = mpsc::channel(16);
        let mut history = Vec::new();
        turn(
            &provider,
            "key",
            "m",
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
        let body = &seen.lock().unwrap()[0];
        assert!(body.get("tools").is_none());
        assert!(body["system"].as_str().unwrap().contains("no tools"));
    }

    #[tokio::test]
    async fn several_calls_in_one_reply_and_a_broken_one_are_all_answered() {
        let tools: Tools = vec![std::sync::Arc::new(crate::tools::tests::Echo)];
        let two = crate::mock::anthropic(&[
            json!({"type": "message_start", "message": {"id": "m"}}),
            json!({"type": "content_block_start", "index": 0, "content_block": {"type": "thinking", "thinking": "", "signature": ""}}),
            json!({"type": "content_block_delta", "index": 0, "delta": {"type": "thinking_delta", "thinking": "Two calls."}}),
            json!({"type": "content_block_delta", "index": 0, "delta": {"type": "signature_delta", "signature": "sig-1"}}),
            json!({"type": "content_block_stop", "index": 0}),
            json!({"type": "content_block_start", "index": 1, "content_block": {"type": "tool_use", "id": "a", "name": "echo", "input": {}}}),
            json!({"type": "content_block_delta", "index": 1, "delta": {"type": "input_json_delta", "partial_json": "{\"text\":\"x\"}"}}),
            json!({"type": "content_block_stop", "index": 1}),
            json!({"type": "content_block_start", "index": 2, "content_block": {"type": "tool_use", "id": "b", "name": "echo", "input": {}}}),
            json!({"type": "content_block_delta", "index": 2, "delta": {"type": "input_json_delta", "partial_json": "{\"text\": oops"}}),
            json!({"type": "content_block_stop", "index": 2}),
            json!({"type": "message_delta", "delta": {"stop_reason": "tool_use"}}),
        ]);
        let (provider, seen) = crate::mock::scripted("anthropic", vec![two, says("done")]).await;
        let (tx, _rx) = mpsc::channel(32);
        let mut history = Vec::new();
        turn(
            &provider,
            "key",
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
        let assistant = &sent[sent.len() - 2]["content"];
        // The thinking goes back as it came, signature and all.
        assert_eq!(
            assistant[0],
            json!({"type": "thinking", "thinking": "Two calls.", "signature": "sig-1"})
        );
        assert_eq!(assistant[2]["input"], json!({}));
        assert!(assistant[2].get(PARTIAL).is_none());
        let results = &sent[sent.len() - 1]["content"];
        assert_eq!(
            results[0],
            json!({"type": "tool_result", "tool_use_id": "a", "content": "echo: x", "is_error": false})
        );
        assert_eq!(results[1]["tool_use_id"], "b");
        assert_eq!(results[1]["is_error"], true);
        assert_eq!(results[1]["content"], "The arguments were not valid JSON.");
    }

    #[tokio::test]
    async fn a_reply_cut_midway_through_a_call_never_keeps_the_call() {
        let tools: Tools = vec![std::sync::Arc::new(crate::tools::tests::Echo)];
        let cut = crate::mock::anthropic(&[
            json!({"type": "message_start", "message": {"id": "m"}}),
            json!({"type": "content_block_start", "index": 0, "content_block": {"type": "text", "text": ""}}),
            json!({"type": "content_block_delta", "index": 0, "delta": {"type": "text_delta", "text": "Let me"}}),
            json!({"type": "content_block_stop", "index": 0}),
            json!({"type": "content_block_start", "index": 1, "content_block": {"type": "tool_use", "id": "a", "name": "echo", "input": {}}}),
            json!({"type": "content_block_delta", "index": 1, "delta": {"type": "input_json_delta", "partial_json": "{\"te"}}),
            json!({"type": "message_delta", "delta": {"stop_reason": "max_tokens"}}),
        ]);
        let (provider, _seen) = crate::mock::scripted("anthropic", vec![cut]).await;
        let (tx, _rx) = mpsc::channel(32);
        let mut history = Vec::new();
        turn(
            &provider,
            "key",
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
        // What the user read stays; the call the API would ask a result for does not.
        assert_eq!(history[1]["content"], json!([{"type": "text", "text": "Let me"}]));
    }

    #[tokio::test]
    async fn a_tool_that_reads_outside_taints_the_conversation() {
        let tools: Tools = vec![
            std::sync::Arc::new(crate::tools::tests::Echo),
            std::sync::Arc::new(crate::tools::tests::Fetch),
        ];
        let fetch = crate::mock::anthropic(&[
            json!({"type": "message_start", "message": {"id": "m"}}),
            json!({"type": "content_block_start", "index": 0, "content_block": {"type": "tool_use", "id": "f", "name": "fetch", "input": {}}}),
            json!({"type": "content_block_stop", "index": 0}),
            json!({"type": "message_delta", "delta": {"stop_reason": "tool_use"}}),
        ]);
        let (provider, _seen) = crate::mock::scripted(
            "anthropic",
            vec![tool_use("e", r#"{"text":"x"}"#), fetch, says("done")],
        )
        .await;
        let (tx, _rx) = mpsc::channel(32);
        let outside = std::sync::Mutex::new(None);
        turn(
            &provider,
            "key",
            "m",
            &mut Vec::new(),
            &Turn::default(),
            tools::Kit {
                tools: &tools,
                outside: &outside,
                frozen: None,
            },
            &tx,
        )
        .await
        .unwrap();
        assert_eq!(outside.lock().unwrap().as_deref(), Some("fetch"));
    }
}
