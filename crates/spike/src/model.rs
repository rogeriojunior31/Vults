//! A fake model for agent spikes: Anthropic Messages, OpenAI Chat Completions and OpenAI
//! Responses, streamed. A request's step is the number of tool results already in it, and the
//! script says what that step answers:
//!
//! ```json
//! [{"run": "echo hi"}, {"read": "f.txt"}, {"write": ["f.txt", "text"]},
//!  {"edit": ["f.txt", "old", "new"]}, {"ask": "Which color?"}, {"text": "done"}]
//! ```
//!
//! Each step becomes a call to the tool the agent offers under that family's name (`Bash`,
//! `run_shell_command`, `exec_command`…); a request without tools (a title, a summary) gets a
//! short text. Plain `std`: one thread per connection, each closed after its answer.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use serde_json::{Value, json};

/// Tool names per family, first match wins.
const RUN: &[&str] = &[
    "Bash",
    "run_shell_command",
    "bash",
    "shell",
    "exec_command",
    "local_shell",
];
const READ: &[&str] = &["Read", "read_file", "read"];
const WRITE: &[&str] = &["Write", "write_file", "write"];
const EDIT: &[&str] = &["Edit", "replace", "edit"];
const ASK: &[&str] = &["AskUserQuestion", "ask_user_question", "question"];
/// Qwen Code 0.25 hides these behind its `tool_call` tool.
const HIDDEN: &[&str] = &[
    "run_shell_command",
    "write_file",
    "edit",
    "read_file",
    "ask_user_question",
];

#[derive(Debug)]
struct Model {
    script: Vec<Value>,
    /// The folder the agent works in: file steps name their files under it.
    workdir: PathBuf,
    log: Option<Mutex<std::fs::File>>,
    ids: AtomicU64,
}

/// Serves on `127.0.0.1:port` until killed.
pub fn serve(port: u16, script: Vec<Value>, workdir: PathBuf, log: Option<&Path>) -> std::io::Result<()> {
    let log = match log {
        Some(p) => Some(Mutex::new(
            std::fs::OpenOptions::new().create(true).append(true).open(p)?,
        )),
        None => None,
    };
    let model = Arc::new(Model {
        script,
        workdir,
        log,
        ids: AtomicU64::new(1),
    });
    let listener = TcpListener::bind(("127.0.0.1", port))?;
    for stream in listener.incoming().flatten() {
        let model = Arc::clone(&model);
        std::thread::spawn(move || {
            let _ = model.handle(stream);
        });
    }
    Ok(())
}

fn pick<'a>(names: &[&'a str], have: &[String]) -> Option<&'a str> {
    names.iter().copied().find(|n| have.iter().any(|h| h == n))
}

/// The tools a request offers, by name.
fn tools_of(req: &Value) -> Vec<String> {
    req["tools"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|t| {
            t.get("name")
                .or_else(|| t.pointer("/function/name"))
                .or_else(|| t.get("type"))
                .and_then(Value::as_str)
                .map(str::to_owned)
        })
        .collect()
}

/// A call to a tool: its name, and its arguments (an object, or a custom tool's raw text).
type Call = (String, Value);

impl Model {
    fn id(&self, prefix: &str) -> String {
        format!("{prefix}{:012x}", self.ids.fetch_add(1, Ordering::Relaxed))
    }

    fn path(&self, name: &str) -> String {
        self.workdir.join(name).to_string_lossy().into_owned()
    }

    /// The call a script step makes with these tools; none when the agent offers no such tool.
    fn call(&self, step: &Value, tools: &[String]) -> Option<Call> {
        if tools.iter().any(|t| t == "tool_call") {
            let mut all = tools.to_vec();
            all.extend(HIDDEN.iter().map(|s| s.to_string()));
            let (name, mut args) = self.plain(step, &all)?;
            if name == "edit" {
                // Qwen's edit takes Claude Code's names, not OpenCode's.
                args = json!({ "file_path": args["filePath"], "old_string": args["oldString"], "new_string": args["newString"] });
            }
            if HIDDEN.contains(&name.as_str()) && !tools.contains(&name) {
                return Some(("tool_call".into(), json!({ "name": name, "arguments": args })));
            }
            return Some((name, args));
        }
        self.plain(step, tools)
    }

    fn plain(&self, step: &Value, tools: &[String]) -> Option<Call> {
        let s = |k: &str, i: usize| step[k][i].as_str().unwrap_or_default().to_string();
        if let Some(cmd) = step["run"].as_str() {
            let n = pick(RUN, tools)?;
            let args = match n {
                "shell" | "local_shell" => json!({ "command": ["bash", "-lc", cmd] }),
                "exec_command" => json!({ "cmd": cmd }),
                _ => json!({ "command": cmd, "description": "run it" }),
            };
            return Some((n.into(), args));
        }
        if let Some(file) = step["read"].as_str() {
            // An agent without a read tool (Codex) reads through its shell.
            let Some(n) = pick(READ, tools) else {
                return self.plain(&json!({ "run": format!("cat {file}") }), tools);
            };
            let key = if n == "read" { "filePath" } else { "file_path" };
            return Some((n.into(), json!({ key: self.path(file) })));
        }
        if step["write"].is_array() {
            let (file, text) = (s("write", 0), s("write", 1));
            return match pick(WRITE, tools) {
                Some("write") => Some((
                    "write".into(),
                    json!({ "filePath": self.path(&file), "content": text }),
                )),
                Some(n) => Some((
                    n.into(),
                    json!({ "file_path": self.path(&file), "content": text }),
                )),
                None => pick(&["apply_patch"], tools).map(|n| {
                    (
                        n.into(),
                        json!(format!(
                            "*** Begin Patch\n*** Add File: {file}\n+{text}\n*** End Patch\n"
                        )),
                    )
                }),
            };
        }
        if step["edit"].is_array() {
            let (file, old, new) = (s("edit", 0), s("edit", 1), s("edit", 2));
            return match pick(EDIT, tools) {
                Some("edit") => Some((
                    "edit".into(),
                    json!({ "filePath": self.path(&file), "oldString": old, "newString": new }),
                )),
                Some(n) => Some((
                    n.into(),
                    json!({ "file_path": self.path(&file), "old_string": old, "new_string": new }),
                )),
                None => pick(&["apply_patch"], tools).map(|n| {
                    (
                        n.into(),
                        json!(format!(
                            "*** Begin Patch\n*** Update File: {file}\n@@\n-{old}\n+{new}\n*** End Patch\n"
                        )),
                    )
                }),
            };
        }
        if let Some(q) = step["ask"].as_str() {
            let n = pick(ASK, tools)?;
            let options =
                json!([{ "label": "Blue", "description": "b" }, { "label": "Red", "description": "r" }]);
            let mut question = json!({ "question": q, "header": "Pick", "options": options });
            if n != "question" {
                question["multiSelect"] = json!(false);
            }
            return Some((n.into(), json!({ "questions": [question] })));
        }
        None
    }

    /// What step `done` (tool results so far) answers: a call, or a text.
    fn step(&self, req: &Value, done: usize) -> Result<Call, String> {
        let tools = tools_of(req);
        if tools.is_empty() {
            return Err("Fake title".into());
        }
        let Some(step) = self.script.get(done) else {
            return Err("done".into());
        };
        if let Some(text) = step["text"].as_str() {
            return Err(text.into());
        }
        // A step the agent has no tool for ends the turn, saying so.
        self.call(step, &tools)
            .ok_or_else(|| format!("no tool for {step}"))
    }

    fn handle(&self, stream: TcpStream) -> std::io::Result<()> {
        let mut reader = BufReader::new(stream.try_clone()?);
        let mut line = String::new();
        reader.read_line(&mut line)?;
        let mut parts = line.split_whitespace();
        let method = parts.next().unwrap_or_default().to_string();
        let path = parts
            .next()
            .unwrap_or_default()
            .split('?')
            .next()
            .unwrap_or_default()
            .to_string();
        let mut length = 0;
        loop {
            let mut header = String::new();
            if reader.read_line(&mut header)? == 0 || header.trim().is_empty() {
                break;
            }
            if let Some((k, v)) = header.split_once(':')
                && k.eq_ignore_ascii_case("content-length")
            {
                length = v.trim().parse().unwrap_or(0);
            }
        }
        let mut body = vec![0; length];
        reader.read_exact(&mut body)?;
        let req: Value = serde_json::from_slice(&body).unwrap_or(json!({}));
        if let Some(log) = &self.log
            && let Ok(mut f) = log.lock()
        {
            let _ = writeln!(f, "{}", json!({ "path": path, "tools": tools_of(&req) }));
        }
        let mut out = stream;
        if method != "POST" {
            let models = json!({ "object": "list", "data": [{ "id": "fake-model", "object": "model", "type": "model", "display_name": "Fake" }],
                                 "models": [{ "slug": "fake-model", "id": "fake-model" }] });
            return reply_json(&mut out, &models);
        }
        if path.ends_with("/messages/count_tokens") {
            return reply_json(&mut out, &json!({ "input_tokens": 10 }));
        }
        if path.ends_with("/messages") {
            return self.anthropic(&mut out, &req);
        }
        if path.ends_with("/chat/completions") {
            return self.chat(&mut out, &req);
        }
        if path.ends_with("/responses") {
            return self.responses(&mut out, &req);
        }
        reply_json(&mut out, &json!({}))
    }

    fn anthropic(&self, out: &mut TcpStream, req: &Value) -> std::io::Result<()> {
        let done = req["messages"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|m| m["content"].as_array())
            .flatten()
            .filter(|c| c["type"] == "tool_result")
            .count();
        let step = self.step(req, done);
        let id = self.id("msg_");
        let model = req["model"].as_str().unwrap_or("fake");
        let start = json!({ "type": "message_start", "message": { "id": id, "type": "message", "role": "assistant", "model": model,
            "content": [], "stop_reason": null, "usage": { "input_tokens": 10, "output_tokens": 1 } } });
        let (block, delta, stop, content) = match &step {
            Err(text) => (
                json!({ "type": "text", "text": "" }),
                json!({ "type": "text_delta", "text": text }),
                "end_turn",
                json!([{ "type": "text", "text": text }]),
            ),
            Ok((name, args)) => {
                let tool = self.id("toolu_");
                (
                    json!({ "type": "tool_use", "id": tool, "name": name, "input": {} }),
                    json!({ "type": "input_json_delta", "partial_json": args.to_string() }),
                    "tool_use",
                    json!([{ "type": "tool_use", "id": tool, "name": name, "input": args }]),
                )
            }
        };
        if req["stream"] != json!(true) {
            return reply_json(
                out,
                &json!({ "id": id, "type": "message", "role": "assistant", "model": model, "content": content,
                         "stop_reason": stop, "usage": { "input_tokens": 10, "output_tokens": 5 } }),
            );
        }
        sse(
            out,
            &[
                (Some("message_start"), start),
                (
                    Some("content_block_start"),
                    json!({ "type": "content_block_start", "index": 0, "content_block": block }),
                ),
                (
                    Some("content_block_delta"),
                    json!({ "type": "content_block_delta", "index": 0, "delta": delta }),
                ),
                (
                    Some("content_block_stop"),
                    json!({ "type": "content_block_stop", "index": 0 }),
                ),
                (
                    Some("message_delta"),
                    json!({ "type": "message_delta", "delta": { "stop_reason": stop }, "usage": { "output_tokens": 5 } }),
                ),
                (Some("message_stop"), json!({ "type": "message_stop" })),
            ],
        )
    }

    fn chat(&self, out: &mut TcpStream, req: &Value) -> std::io::Result<()> {
        let done = req["messages"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|m| m["role"] == "tool")
            .count();
        let step = self.step(req, done);
        let base = json!({ "id": self.id("chatcmpl-"), "object": "chat.completion.chunk", "created": 1,
                           "model": req["model"].as_str().unwrap_or("fake") });
        let chunk = |choices: Value| {
            let mut c = base.clone();
            c["choices"] = choices;
            c
        };
        let (delta, message, finish) = match &step {
            Err(text) => (
                json!({ "role": "assistant", "content": text }),
                json!({ "role": "assistant", "content": text }),
                "stop",
            ),
            Ok((name, args)) => {
                let call = json!({ "index": 0, "id": self.id("call_"), "type": "function",
                                   "function": { "name": name, "arguments": args.to_string() } });
                (
                    json!({ "role": "assistant", "tool_calls": [call] }),
                    json!({ "role": "assistant", "content": null, "tool_calls": [call] }),
                    "tool_calls",
                )
            }
        };
        let usage = json!({ "prompt_tokens": 10, "completion_tokens": 5, "total_tokens": 15 });
        if req["stream"] != json!(true) {
            let mut whole = base.clone();
            whole["object"] = json!("chat.completion");
            whole["choices"] = json!([{ "index": 0, "message": message, "finish_reason": finish }]);
            whole["usage"] = usage;
            return reply_json(out, &whole);
        }
        let mut last = chunk(json!([]));
        last["usage"] = usage;
        sse(
            out,
            &[
                (
                    None,
                    chunk(json!([{ "index": 0, "delta": delta, "finish_reason": null }])),
                ),
                (
                    None,
                    chunk(json!([{ "index": 0, "delta": {}, "finish_reason": finish }])),
                ),
                (None, last),
                (None, json!("[DONE]")),
            ],
        )
    }

    fn responses(&self, out: &mut TcpStream, req: &Value) -> std::io::Result<()> {
        let outputs = [
            "function_call_output",
            "custom_tool_call_output",
            "local_shell_call_output",
        ];
        let done = req["input"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|i| i["type"].as_str().is_some_and(|t| outputs.contains(&t)))
            .count();
        let item = match self.step(req, done) {
            Err(text) => {
                json!({ "type": "message", "id": self.id("msg_"), "role": "assistant", "status": "completed",
                                 "content": [{ "type": "output_text", "text": text, "annotations": [] }] })
            }
            Ok((name, Value::String(input))) => {
                let call = self.id("call_");
                json!({ "type": "custom_tool_call", "id": format!("ctc_{call}"), "call_id": call, "name": name,
                        "input": input, "status": "completed" })
            }
            Ok((name, args)) => {
                let call = self.id("call_");
                json!({ "type": "function_call", "id": format!("fc_{call}"), "call_id": call, "name": name,
                        "arguments": args.to_string(), "status": "completed" })
            }
        };
        let resp = json!({ "id": self.id("resp_"), "object": "response", "status": "completed",
            "model": req["model"].as_str().unwrap_or("fake"), "output": [item],
            "usage": { "input_tokens": 10, "output_tokens": 5, "total_tokens": 15,
                       "input_tokens_details": { "cached_tokens": 0 }, "output_tokens_details": { "reasoning_tokens": 0 } } });
        let mut started = resp.clone();
        started["status"] = json!("in_progress");
        started["output"] = json!([]);
        sse(
            out,
            &[
                (
                    Some("response.created"),
                    json!({ "type": "response.created", "response": started }),
                ),
                (
                    Some("response.output_item.added"),
                    json!({ "type": "response.output_item.added", "output_index": 0, "item": item }),
                ),
                (
                    Some("response.output_item.done"),
                    json!({ "type": "response.output_item.done", "output_index": 0, "item": item }),
                ),
                (
                    Some("response.completed"),
                    json!({ "type": "response.completed", "response": resp }),
                ),
            ],
        )
    }
}

fn reply_json(out: &mut TcpStream, v: &Value) -> std::io::Result<()> {
    let body = v.to_string();
    write!(
        out,
        "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
        body.len()
    )?;
    out.flush()
}

/// Server-sent events, then the connection closes: the agent reads to the end.
fn sse(out: &mut TcpStream, events: &[(Option<&str>, Value)]) -> std::io::Result<()> {
    write!(
        out,
        "HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\ncache-control: no-cache\r\nconnection: close\r\n\r\n"
    )?;
    for (name, data) in events {
        if let Some(name) = name {
            writeln!(out, "event: {name}")?;
        }
        match data {
            Value::String(raw) => write!(out, "data: {raw}\n\n")?,
            data => write!(out, "data: {data}\n\n")?,
        }
    }
    out.flush()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    fn model(script: Value) -> Model {
        Model {
            script: serde_json::from_value(script).unwrap(),
            workdir: PathBuf::from("/w"),
            log: None,
            ids: AtomicU64::new(1),
        }
    }

    fn tools(names: &[&str]) -> Value {
        json!({ "tools": names.iter().map(|n| json!({ "name": n })).collect::<Vec<_>>() })
    }

    #[test]
    fn each_step_calls_the_tool_the_agent_offers() {
        let m = model(json!([{ "run": "echo hi" }, { "edit": ["a.txt", "old", "new"] }, { "text": "done" }]));
        let claude = tools(&["Bash", "Edit"]);
        assert_eq!(
            m.step(&claude, 0).unwrap(),
            (
                "Bash".into(),
                json!({ "command": "echo hi", "description": "run it" })
            )
        );
        assert_eq!(
            m.step(&claude, 1).unwrap(),
            (
                "Edit".into(),
                json!({ "file_path": "/w/a.txt", "old_string": "old", "new_string": "new" })
            )
        );
        assert_eq!(m.step(&claude, 2), Err("done".into()));
        // OpenCode's names, Codex's patch, and a request with no tools at all.
        let opencode = tools(&["bash", "edit"]);
        assert_eq!(m.step(&opencode, 1).unwrap().1["filePath"], "/w/a.txt");
        let codex = tools(&["exec_command", "apply_patch"]);
        assert_eq!(
            m.step(&codex, 0).unwrap(),
            ("exec_command".into(), json!({ "cmd": "echo hi" }))
        );
        assert!(
            m.step(&codex, 1)
                .unwrap()
                .1
                .as_str()
                .unwrap()
                .contains("*** Update File: a.txt")
        );
        assert_eq!(m.step(&json!({}), 0), Err("Fake title".into()));
        let read = model(json!([{ "read": "a.txt" }]));
        assert_eq!(
            read.step(&codex, 0).unwrap(),
            ("exec_command".into(), json!({ "cmd": "cat a.txt" }))
        );
    }

    #[test]
    fn qwens_hidden_tools_go_through_its_tool_call() {
        let m = model(json!([{ "edit": ["a.txt", "old", "new"] }]));
        let (name, args) = m.step(&tools(&["tool_call", "glob"]), 0).unwrap();
        assert_eq!(name, "tool_call");
        assert_eq!(args["name"], "edit");
        assert_eq!(args["arguments"]["old_string"], "old");
    }
}
