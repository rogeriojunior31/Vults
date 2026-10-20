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
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::Command;

const QUESTION: &str = "VULTS_SYNTHETIC_TEXT_QUESTION: reply with one word.";

#[tokio::test]
#[ignore = "requires the installed Codex CLI and an isolated loopback-only network namespace"]
async fn installed_client_must_not_be_mistaken_for_a_tool_free_consultation() {
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
    let dir = std::env::temp_dir().join(format!("vults-codex-qualification-{}", std::process::id()));
    std::fs::DirBuilder::new().mode(0o700).create(&dir).unwrap();
    let _cleanup = Directory(dir.clone());
    let mut cmd = Command::new("codex");
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
        "--model",
        "gpt-6.1-sol",
    ]);
    for setting in [
        "model_provider=\"vults_qualification\"".to_string(),
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
    cmd.arg("-")
        .env("VULTS_CODEX_QUALIFICATION_TOKEN", "synthetic-not-a-secret")
        .env(vults_protocol::ZECA_ENV, "1")
        .current_dir(&dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .kill_on_drop(true);
    let mut child = cmd.spawn().unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let mut request =
        tokio::spawn(async move {
            stdin.write_all(QUESTION.as_bytes()).await.unwrap();
            drop(stdin);
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut raw = Vec::new();
            let mut buf = [0u8; 8192];
            loop {
                let n = socket.read(&mut buf).await.unwrap();
                assert_ne!(n, 0, "incomplete request");
                raw.extend_from_slice(&buf[..n]);
                assert!(raw.len() < 2 * 1024 * 1024, "request exceeds probe limit");
                let Some(end) = raw.windows(4).position(|w| w == b"\r\n\r\n") else {
                    continue;
                };
                let head = std::str::from_utf8(&raw[..end]).unwrap();
                assert!(head.starts_with("POST /v1/responses "));
                assert!(head.lines().any(|line| line.eq_ignore_ascii_case(
                "authorization: Bearer synthetic-not-a-secret"
            )), "only the synthetic credential may reach the stand-in");
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
            }
            // Never send a model response: the CLI cannot execute model-selected actions.
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
    assert!(body["input"].to_string().contains(QUESTION));
    let names = offered_tools(&body);
    let input = body["input"].to_string();
    let skills = input.contains("Available skills") && input.contains("SKILL.md");
    // Do not print captured instructions, headers, directories or prompt contents.
    println!("Codex consultation candidate: tools={names:?}, ambient_skills={skills}");
    assert!(
        !names.is_empty() || skills,
        "candidate behavior changed; review full text-only qualification before enabling Codex"
    );
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
