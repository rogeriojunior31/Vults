//! Installed-client qualification only; never linked into the application.
//! Run in an empty network namespace (see docs/dev/plan-dictation.md).
#![cfg(target_os = "linux")]
#![allow(clippy::unwrap_used)]

use std::collections::BTreeSet;
use std::os::unix::fs::DirBuilderExt;
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;

use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;

const QUESTION: &str = "VULTS_SYNTHETIC_TEXT_QUESTION: $vults-consultation-canary @canary.txt";
const CANARY: &str = "VULTS_PRIVATE_CONTEXT_MUST_NOT_BE_SENT";

#[tokio::test]
#[ignore = "requires the installed Codex CLI and an isolated loopback-only network namespace"]
async fn installed_client_must_not_be_mistaken_for_a_tool_free_consultation() {
    probe(false, false).await;
}

#[tokio::test]
#[ignore = "requires the installed Codex CLI and an isolated loopback-only network namespace"]
async fn app_server_without_environments_must_still_qualify_its_full_context() {
    probe(true, false).await;
}

#[tokio::test]
#[ignore = "requires the installed Codex CLI and an isolated loopback-only network namespace"]
async fn custom_catalog_consultation_candidate() {
    probe(false, true).await;
}

async fn probe(app_server: bool, catalog: bool) {
    // Even managed configuration cannot redirect this probe to an external provider.
    let interfaces = std::fs::read_to_string("/proc/net/dev").unwrap();
    assert!(
        interfaces
            .lines()
            .skip(2)
            .all(|line| { line.split_once(':').is_some_and(|(name, _)| name.trim() == "lo") }),
        "run in a loopback-only network namespace"
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let dir = std::env::temp_dir().join(format!(
        "vults-codex-qualification-{}-{app_server}-{catalog}",
        std::process::id()
    ));
    std::fs::DirBuilder::new().mode(0o700).create(&dir).unwrap();
    let _cleanup = Directory(dir.clone());
    let mut cmd = if catalog {
        super::command("Answer only the supplied question. No tools or actions.", &dir).unwrap()
    } else {
        Command::new("codex")
    };
    if app_server {
        cmd.arg("app-server");
    } else if !catalog {
        cmd.args([
            "exec",
            "--ignore-user-config",
            "--ignore-rules",
            "--strict-config",
            "--ephemeral",
            "--skip-git-repo-check",
            "--sandbox",
            "read-only",
            "--json",
        ]);
    }
    for setting in [
        "model_provider=\"vults_qualification\"".to_string(),
        "model=\"gpt-6.1-sol\"".into(),
        "sandbox_mode=\"read-only\"".into(),
        "model_providers.vults_qualification.name=\"Local qualification only\"".into(),
        format!(
            "model_providers.vults_qualification.base_url=\"http://{}/v1\"",
            listener.local_addr().unwrap()
        ),
        "model_providers.vults_qualification.wire_api=\"responses\"".into(),
        "model_providers.vults_qualification.requires_openai_auth=false".into(),
        "model_providers.vults_qualification.env_key=\"VULTS_CODEX_QUALIFICATION_TOKEN\"".into(),
        "model_providers.vults_qualification.request_max_retries=0".into(),
        "model_providers.vults_qualification.stream_max_retries=0".into(),
        "analytics.enabled=false".into(),
        "feedback.enabled=false".into(),
        "otel.exporter=\"none\"".into(),
        "otel.trace_exporter=\"none\"".into(),
        "otel.metrics_exporter=\"none\"".into(),
        "web_search=\"disabled\"".into(),
        "project_doc_max_bytes=0".into(),
        "mcp_servers={}".into(),
    ] {
        cmd.args(["-c", &setting]);
    }
    for feature in [
        "shell_tool",
        "unified_exec",
        "multi_agent",
        "hooks",
        "plugins",
        "remote_plugin",
        "apps",
        "browser_use",
        "in_app_browser",
        "computer_use",
        "code_mode_host",
        "skill_search",
        "skill_mcp_dependency_install",
        "sleep_tool",
        "view_image",
        "image_generation",
        "goals",
        "memories",
        "shell_snapshot",
        "enable_request_compression",
        "daemon_auto_start",
        "workspace_dependencies",
        "tool_suggest",
    ] {
        cmd.args(["--disable", feature]);
    }
    if !app_server && !catalog {
        cmd.arg("-");
    }
    cmd.env("VULTS_CODEX_QUALIFICATION_TOKEN", "synthetic-not-a-secret")
        .env(vults_protocol::ZECA_ENV, "1")
        .current_dir(&dir)
        .stdin(Stdio::piped())
        .stdout(if app_server || catalog {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stderr(Stdio::null())
        .kill_on_drop(true);
    if catalog {
        std::fs::write(dir.join("AGENTS.md"), CANARY).unwrap();
        std::fs::write(dir.join("canary.txt"), CANARY).unwrap();
        let skill = dir.join(".agents/skills/vults-consultation-canary");
        std::fs::create_dir_all(&skill).unwrap();
        std::fs::write(skill.join("SKILL.md"), format!("---\nname: vults-consultation-canary\ndescription: Private qualification canary\n---\n{CANARY}\n")).unwrap();
    }
    let mut child = cmd.spawn().unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take();
    let mut request = tokio::spawn(async move {
        if app_server {
            let mut lines = BufReader::new(stdout.unwrap().take(2 * 1024 * 1024)).lines();
            send(
                &mut stdin,
                serde_json::json!({"id":1,"method":"initialize","params":{
                    "clientInfo":{"name":"vults_qualification","version":"0.0.0"},
                    "capabilities":{"experimentalApi":true}}}),
            )
            .await;
            receive(&mut lines, 1).await;
            send(&mut stdin, serde_json::json!({"method":"initialized"})).await;
            send(
                &mut stdin,
                serde_json::json!({"id":2,"method":"thread/start","params":{
                    "model":"gpt-6.1-sol", "modelProvider":"vults_qualification",
                    "ephemeral":true, "sandbox":"read-only", "environments":[],
                    "selectedCapabilityRoots":[], "dynamicTools":[],
                    "baseInstructions":"Answer only the supplied text. No tools or actions.",
                    "developerInstructions":"Use no outside context."}}),
            )
            .await;
            let reply = receive(&mut lines, 2).await;
            let thread = reply["thread"]["id"].as_str().expect("thread identity");
            send(
                &mut stdin,
                serde_json::json!({"id":3,"method":"turn/start","params":{
                    "threadId":thread, "input":[{"type":"text","text":QUESTION}],
                    "environments":[]}}),
            )
            .await;
            // Keep protocol streams alive until the model request is captured.
            let body = capture(listener, false).await;
            drop((stdin, lines));
            body
        } else {
            let question = if catalog {
                super::input(QUESTION)
            } else {
                QUESTION.into()
            };
            stdin.write_all(question.as_bytes()).await.unwrap();
            drop(stdin);
            let body = capture(listener, catalog).await;
            if catalog {
                let mut raw = Vec::new();
                stdout
                    .unwrap()
                    .take(64 * 1024)
                    .read_to_end(&mut raw)
                    .await
                    .unwrap();
                assert!(raw.len() < 64 * 1024);
                assert_eq!(
                    super::answer(std::str::from_utf8(&raw).unwrap()).unwrap(),
                    "Qualified answer"
                );
            }
            body
        }
    });
    let captured = tokio::time::timeout(Duration::from_secs(30), &mut request).await;
    let _ = child.kill().await;
    if captured.is_err() {
        request.abort();
        let _ = request.await;
    }
    assert!(
        child.try_wait().unwrap().is_some(),
        "child exit must be acknowledged"
    );
    let body = captured
        .expect("the installed client did not reach the local stand-in")
        .expect("the stand-in rejected the request");
    let expected = if catalog {
        super::input(QUESTION)
    } else {
        QUESTION.into()
    };
    assert!(contains_text(&body["input"], &expected));
    let names = offered_tools(&body);
    let input = body["input"].to_string();
    let skills = input.contains("Available skills") && input.contains("SKILL.md");
    println!(
        "Codex candidate app_server={app_server} catalog={catalog}: tools={names:?}, ambient_skills={skills}"
    );
    if catalog {
        assert!(names.is_empty(), "the isolated candidate offered tools");
        assert!(!skills, "the isolated candidate inherited skills");
        assert!(
            !body.to_string().contains(CANARY),
            "file, project or explicit skill context escaped"
        );
    } else {
        assert!(
            !names.is_empty() || skills,
            "candidate changed; review full qualification before enabling Codex"
        );
    }
}

fn contains_text(value: &Value, expected: &str) -> bool {
    match value {
        Value::String(text) => text.contains(expected),
        Value::Array(items) => items.iter().any(|item| contains_text(item, expected)),
        Value::Object(items) => items.values().any(|item| contains_text(item, expected)),
        _ => false,
    }
}

async fn send(stdin: &mut tokio::process::ChildStdin, value: Value) {
    stdin.write_all(format!("{value}\n").as_bytes()).await.unwrap();
}
async fn receive<R: tokio::io::AsyncBufRead + Unpin>(lines: &mut tokio::io::Lines<R>, id: u64) -> Value {
    loop {
        let value: Value =
            serde_json::from_str(&lines.next_line().await.unwrap().expect("protocol ended")).unwrap();
        assert!(
            value.get("error").is_none(),
            "app-server rejected qualification request"
        );
        assert!(
            !(value.get("method").is_some() && value.get("id").is_some()),
            "unexpected server request; no automatic answers"
        );
        if value["id"] == id {
            return value["result"].clone();
        }
    }
}
async fn capture(listener: tokio::net::TcpListener, reply: bool) -> Value {
    let (mut socket, _) = listener.accept().await.unwrap();
    let mut raw = Vec::new();
    let mut buf = [0u8; 8192];
    let body = loop {
        let n = socket.read(&mut buf).await.unwrap();
        assert_ne!(n, 0, "incomplete request");
        raw.extend_from_slice(&buf[..n]);
        assert!(raw.len() < 2 * 1024 * 1024, "request exceeds probe limit");
        let Some(end) = raw.windows(4).position(|w| w == b"\r\n\r\n") else {
            continue;
        };
        let head = std::str::from_utf8(&raw[..end]).unwrap();
        assert!(head.starts_with("POST /v1/responses "));
        assert!(
            head.lines()
                .any(|line| line.eq_ignore_ascii_case("authorization: Bearer synthetic-not-a-secret")),
            "only the synthetic credential may reach the stand-in"
        );
        let length: usize = head
            .lines()
            .find_map(|line| {
                let (name, value) = line.split_once(':')?;
                name.eq_ignore_ascii_case("content-length")
                    .then(|| value.trim().parse().unwrap())
            })
            .expect("uncompressed request with a content length");
        if raw.len() >= end + 4 + length {
            break serde_json::from_slice::<Value>(&raw[end + 4..end + 4 + length]).unwrap();
        }
    };
    if reply {
        // Only the positively isolated client gets a fixed text-only model response.
        assert!(offered_tools(&body).is_empty());
        assert!(!body.to_string().contains(CANARY));
        let message = serde_json::json!({"type":"message","id":"msg_qualified","role":"assistant","phase":"final_answer",
            "content":[{"type":"output_text","text":"Qualified answer","annotations":[]}]});
        let events = [
            serde_json::json!({"type":"response.created","response":{"id":"resp_qualified","status":"in_progress"}}),
            serde_json::json!({"type":"response.output_item.added","output_index":0,"item":message}),
            serde_json::json!({"type":"response.output_item.done","output_index":0,"item":message}),
            serde_json::json!({"type":"response.completed","response":{"id":"resp_qualified","status":"completed",
                "output":[message],"usage":{"input_tokens":1,"output_tokens":1,"total_tokens":2}}}),
        ];
        let payload: String = events
            .iter()
            .map(|e| format!("event: {}\ndata: {e}\n\n", e["type"].as_str().unwrap()))
            .collect();
        socket.write_all(format!("HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{payload}",payload.len()).as_bytes()).await.unwrap();
    }
    body
}

fn offered_tools(body: &Value) -> BTreeSet<&str> {
    fn names<'a>(tool: &'a Value, out: &mut BTreeSet<&'a str>) {
        let container = tool["tools"].is_array() || tool["functions"].is_array();
        if !container
            && let Some(name) = tool["name"]
                .as_str()
                .or_else(|| tool["type"].as_str().filter(|kind| *kind != "additional_tools"))
        {
            out.insert(name);
        }
        // Namespace and code-mode tool registries can contain nested definitions.
        for key in ["tools", "functions"] {
            if let Some(children) = tool[key].as_array() {
                for child in children {
                    names(child, out);
                }
            }
        }
    }
    let mut out = BTreeSet::new();
    for tool in body["tools"].as_array().into_iter().flatten() {
        names(tool, &mut out);
    }
    for item in body["input"].as_array().into_iter().flatten() {
        if item["type"] == "additional_tools" {
            names(item, &mut out);
        }
    }
    out
}

#[test]
fn qualification_checks_inline_code_mode_tools_as_well_as_top_level_tools() {
    let body = serde_json::json!({
        "input": [{"type": "additional_tools", "tools": [
            {"name": "functions", "tools": [{"name": "exec_command"}]}]}]
    });
    assert!(body.get("tools").is_none());
    assert!(offered_tools(&body).contains("exec_command"));
    assert!(offered_tools(&serde_json::json!({"tools": [{"name": "shell"}]})).contains("shell"));
    assert!(offered_tools(&serde_json::json!({"tools": [{"type": "web_search"}]})).contains("web_search"));
    assert!(offered_tools(&serde_json::json!({"tools": [], "input": []})).is_empty());
    assert!(offered_tools(&serde_json::json!({"tools": [{"name": "functions", "tools": []}]})).is_empty());
}

struct Directory(PathBuf);
impl Drop for Directory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
