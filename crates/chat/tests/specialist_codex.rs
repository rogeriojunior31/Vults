//! Exercise the isolated subscription protocol without a real login, network or model request.
#![cfg(target_os = "linux")]
#![allow(clippy::unwrap_used)]
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot};
use vults_agents::driver::Spec;
use vults_chat::{Answer, Approver, Chat, Delta, Provider, Turn};

const CLI: &str = r#"#!/usr/bin/python3
import json, os, pathlib, sys
project=pathlib.Path(__PROJECT__)
def record(value):
    with (project / "wire.jsonl").open("a") as stream:
        stream.write(json.dumps(value)+"\n")
def emit(value):
    print(json.dumps(value),flush=True)
record({"startup":True,"args":sys.argv[1:],"home":os.environ.get("HOME"),"state":os.environ.get("CODEX_HOME")})
assert sys.argv[1] == "app-server", "Specialists cannot fall back to exec"
assert "OPENAI_API_KEY" not in os.environ
assert "--strict-config" in sys.argv
assert 'cli_auth_credentials_store="ephemeral"' in sys.argv
for line in sys.stdin:
    request=json.loads(line)
    method=request.get("method")
    params=request.get("params") or {}
    if method == "account/login/start":
        assert params["type"] == "chatgptAuthTokens" and params["accessToken"] in ("e30.eyJzdWIiOiJGQUtFX1VTRVIifQ.test","e30.eyJzdWIiOiJPVEhFUl9VU0VSIn0.test") and params["chatgptAccountId"] in ("FAKE_ACCOUNT","OTHER_ACCOUNT")
        record({"method":method,"memoryOnly":True})
    else:
        record(request)
    result={}
    if method == "config/read":
        servers={"outside":{"command":"false"}} if (project / "fail").exists() else {"vults-specialist":{}}
        result={"config":{"project_doc_max_bytes":0,"cli_auth_credentials_store":"ephemeral","mcp_servers":servers}}
    elif method in ("thread/start","thread/resume"):
        assert params["baseInstructions"] == "PROFILE_ONLY"
        assert params["developerInstructions"] is None
        assert params["approvalPolicy"] == "untrusted"
        assert params["model"] == "codex-test-model"
        if method == "thread/resume":
            assert params["threadId"] == "owned-thread"
        result={"thread":{"id":"owned-thread"}}
    elif method == "turn/start":
        assert params["threadId"] == "owned-thread"
        emit({"id":request["id"],"result":{"turn":{"id":"owned-turn"}}})
        emit({"method":"item/agentMessage/delta","params":{"delta":"Isolated answer"}})
        emit({"method":"turn/completed","params":{"threadId":"owned-thread","turn":{"id":"owned-turn","status":"completed"}}})
        continue
    if "id" in request:
        emit({"id":request["id"],"result":result})
"#;
struct Human;
impl Approver for Human {
    fn wait(&self, _: &str, _: &str, _: &str) -> oneshot::Receiver<Answer> {
        let (tx, rx) = oneshot::channel();
        let _ = tx.send(Answer::Deny);
        rx
    }
}
async fn send(chat: &mut Chat) -> Vec<Delta> {
    let (tx, mut rx) = mpsc::channel(64);
    let (stop, stopped) = oneshot::channel();
    chat.send(
        Turn {
            text: "Only this profile".into(),
            ..Turn::default()
        },
        tx,
        Arc::new(Human),
        stopped,
    )
    .await;
    drop(stop);
    let mut events = Vec::new();
    while let Some(event) = rx.recv().await {
        events.push(event);
    }
    events
}
#[tokio::test]
async fn specialist_starts_resumes_and_fails_closed_without_inheriting_another_login_or_driver() {
    use std::os::unix::fs::PermissionsExt;
    let root = std::env::temp_dir().join(format!("vults-specialist-codex-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let project = root.join("project");
    let bin = root.join("bin");
    let source = root.join("login");
    let source_file = source.join("auth.json");
    let state = root.join("managed");
    for path in [&project, &bin, &source] {
        std::fs::create_dir_all(path).unwrap();
    }
    std::fs::write(source.join("auth.json"),r#"{"OPENAI_API_KEY":"DO_NOT_USE","tokens":{"access_token":"e30.eyJzdWIiOiJGQUtFX1VTRVIifQ.test","refresh_token":"DO_NOT_TRANSFER","account_id":"FAKE_ACCOUNT"}}"#).unwrap();
    let script = CLI.replace("__PROJECT__", &serde_json::to_string(&project).unwrap());
    let executable = bin.join("codex");
    std::fs::write(&executable, script).unwrap();
    std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o700)).unwrap();
    let path = format!("{}:{}", bin.display(), std::env::var("PATH").unwrap_or_default());
    vults_agents::set_shell_env(move || {
        vec![
            ("PATH".into(), path.clone().into()),
            ("CODEX_HOME".into(), source.clone().into()),
            ("OPENAI_API_KEY".into(), "DO_NOT_INHERIT".into()),
        ]
    });
    let mut spec = Spec::in_dir(project.clone());
    spec.state_dir = Some(state.clone());
    spec.model = Some("codex-test-model".into());
    spec.effort = Some(vults_chat::Effort::High);
    let mut chat = Chat::specialist_for(
        Provider::Codex,
        spec.clone(),
        None,
        None,
        None,
        "PROFILE_ONLY".into(),
    )
    .unwrap();
    for _ in 0..2 {
        let events = send(&mut chat).await;
        assert!(
            events.iter().any(|event| matches!(event, Delta::Done)),
            "{events:?}"
        );
        assert_eq!(chat.session().as_deref(), Some("owned-thread"));
    }
    std::fs::write(
        &source_file,
        r#"{"tokens":{"access_token":"e30.eyJzdWIiOiJPVEhFUl9VU0VSIn0.test","account_id":"FAKE_ACCOUNT"}}"#,
    )
    .unwrap();
    assert!(
        send(&mut chat)
            .await
            .iter()
            .any(|event| matches!(event,Delta::Error{message} if message.contains("another login")))
    );
    let mut other =
        Chat::specialist_for(Provider::Codex, spec, None, None, None, "PROFILE_ONLY".into()).unwrap();
    assert!(
        send(&mut other)
            .await
            .iter()
            .any(|event| matches!(event, Delta::Done)),
        "A new conversation can use the newly selected login"
    );
    drop(other);
    std::fs::write(
        &source_file,
        r#"{"tokens":{"access_token":"e30.eyJzdWIiOiJGQUtFX1VTRVIifQ.test","account_id":"FAKE_ACCOUNT"}}"#,
    )
    .unwrap();
    std::fs::write(project.join("fail"), "external server").unwrap();
    assert!(
        send(&mut chat)
            .await
            .iter()
            .any(|event| matches!(event,Delta::Error{message} if message.contains("external configuration")))
    );
    let trace: Vec<Value> = std::fs::read_to_string(project.join("wire.jsonl"))
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(trace.iter().filter(|v| v["method"] == "thread/start").count(), 2);
    assert_eq!(trace.iter().filter(|v| v["method"] == "thread/resume").count(), 1);
    assert_eq!(
        trace
            .iter()
            .filter(|v| v["method"] == "account/login/start")
            .count(),
        3,
        "Unqualified configuration must fail before login"
    );
    for start in trace.iter().filter(|v| v["startup"] == true) {
        let account_root = std::path::Path::new(start["state"].as_str().unwrap());
        assert_eq!(account_root.parent(), Some(state.join("accounts").as_path()));
        assert_eq!(start["home"].as_str(), account_root.join("home").to_str());
        assert!(!account_root.join("auth.json").exists());
        assert!(start["args"].as_array().unwrap().iter().any(|a| {
            a.as_str()
                .is_some_and(|s| s.starts_with("mcp_servers.vults-specialist.command="))
        }));
    }
    assert!(!state.join("auth.json").exists());
    drop(chat);
    std::fs::remove_dir_all(root).unwrap();
}
