//! Synthetic auth and policy only: real home and system config are hidden before client startup.
#![cfg(target_os = "linux")]
#![allow(clippy::unwrap_used)]

use std::{
    os::unix::fs::{DirBuilderExt, PermissionsExt},
    path::PathBuf,
    process::Stdio,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    process::Command,
};

#[tokio::test]
#[ignore = "requires the installed Codex CLI, bwrap and a loopback-only network namespace"]
async fn managed_auth_refuses_api_credentials_without_forced_exec_logout() {
    let original_home = PathBuf::from(std::env::var_os("HOME").expect("original home"));
    assert!(original_home.is_absolute());
    assert!(
        std::env::var_os("CODEX_HOME").is_none(),
        "this probe qualifies the default auth path; do not change CODEX_HOME"
    );
    let binary = std::env::split_paths(&std::env::var_os("PATH").expect("client path"))
        .map(|path| path.join("codex"))
        .find(|path| path.is_file())
        .expect("installed Codex CLI")
        .canonicalize()
        .unwrap();
    let path = std::env::temp_dir().join(format!(
        "vults-codex-auth-{}-{}",
        std::process::id(),
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()
    ));
    std::fs::DirBuilder::new().mode(0o700).create(&path).unwrap();
    let directory = Directory(path);
    let synthetic_home = directory.0.join("home");
    let auth_dir = synthetic_home.join(".codex");
    std::fs::create_dir_all(&auth_dir).unwrap();
    let auth_path = auth_dir.join("auth.json");
    let sentinel = br#"{"auth_mode":"apikey","OPENAI_API_KEY":"sk-vults-synthetic-not-real"}"#;
    std::fs::write(&auth_path, sentinel).unwrap();
    std::fs::set_permissions(&auth_path, std::fs::Permissions::from_mode(0o600)).unwrap();

    let scratch = directory.0.join("scratch");
    std::fs::DirBuilder::new().mode(0o700).create(&scratch).unwrap();
    let exec = super::command("Answer only the supplied text.", &scratch).unwrap();
    let empty_etc = directory.0.join("empty-etc");
    std::fs::create_dir(&empty_etc).unwrap();
    let synthetic_etc = directory.0.join("etc");
    std::fs::create_dir_all(synthetic_etc.join("codex")).unwrap();
    std::fs::write(
        synthetic_etc.join("codex/requirements.toml"),
        "allowed_login_methods = [\"chatgpt\"]\n",
    )
    .unwrap();
    let interfaces = std::fs::read_to_string("/proc/net/dev").unwrap();
    assert!(
        interfaces
            .lines()
            .skip(2)
            .all(|line| line.split_once(':').is_some_and(|(name, _)| name.trim() == "lo")),
        "run in a loopback-only network namespace"
    );
    let token = format!("e30.{}.synthetic", URL_SAFE_NO_PAD.encode(serde_json::json!({
        "email":"synthetic@example.invalid", "exp":4102444800_u64,
        "https://api.openai.com/auth":{"chatgpt_account_id":"synthetic-account", "chatgpt_user_id":"synthetic-user", "chatgpt_plan_type":"plus"}
    }).to_string()));
    let chatgpt = serde_json::json!({"auth_mode":"chatgpt", "OPENAI_API_KEY":null,
        "tokens":{"id_token":token,"access_token":"synthetic-access","refresh_token":"synthetic-refresh","account_id":"synthetic-account"},
        "last_refresh":"2099-01-01T00:00:00Z"}).to_string();
    for mode in [
        Case::ApiStatus,
        Case::RestrictedStatus,
        Case::ManagedApiStatus,
        Case::ApiExec,
        Case::ManagedApiExec,
        Case::ManagedChatgptStatus,
        Case::SwitchedApiExec,
        Case::ManagedChatgpt401Switch,
        Case::RestrictedExec,
    ] {
        if matches!(mode, Case::ManagedChatgptStatus | Case::ManagedChatgpt401Switch) {
            std::fs::write(&auth_path, &chatgpt).unwrap();
        }
        let network = matches!(
            mode,
            Case::ApiExec | Case::ManagedApiExec | Case::SwitchedApiExec | Case::ManagedChatgpt401Switch
        );
        let listener = if network {
            Some(tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap())
        } else {
            None
        };
        let candidate = super::subscription_status();
        let mut command = Command::new("bwrap");
        command.args(["--unshare-all", "--die-with-parent", "--ro-bind", "/", "/"]);
        command.args(["--bind"]).arg(&synthetic_home).arg(&original_home);
        command.args(["--ro-bind"]).arg(&binary).arg(&binary);
        command.args([
            "--tmpfs", "/run", "--tmpfs", "/tmp", "--proc", "/proc", "--dev", "/dev", "--chdir", "/tmp",
        ]);
        command.args(["--ro-bind"]).arg(&directory.0).arg(&directory.0);
        if matches!(
            mode,
            Case::ManagedApiStatus
                | Case::ManagedApiExec
                | Case::ManagedChatgptStatus
                | Case::SwitchedApiExec
                | Case::ManagedChatgpt401Switch
        ) {
            command.args(["--ro-bind"]).arg(&synthetic_etc).arg("/etc");
        } else {
            command.args(["--ro-bind"]).arg(&empty_etc).arg("/etc");
        }
        if network {
            command.arg("--share-net");
        }
        command.arg(&binary);
        if mode.exec() {
            command.args(exec.as_std().get_args());
            if let Some(listener) = &listener {
                command.args([
                    "-c",
                    &format!("openai_base_url=\"http://{}/v1\"", listener.local_addr().unwrap()),
                    "--disable",
                    "enable_request_compression",
                ]);
            }
            if mode == Case::RestrictedExec {
                command.args(["-c", "forced_login_method=\"chatgpt\""]);
            }
        } else if mode == Case::RestrictedStatus {
            command.args(candidate.as_std().get_args());
        } else {
            command.args(["login", "status"]);
        }
        // Keep HOME's value, but expose only its synthetic mount; no host keyring or auth env.
        for (key, _) in std::env::vars_os() {
            if !["HOME", "PATH", "USER", "LOGNAME", "LANG", "LC_ALL", "TERM"]
                .iter()
                .any(|name| key == *name)
            {
                command.env_remove(key);
            }
        }
        command
            .stdin(if mode.exec() {
                Stdio::piped()
            } else {
                Stdio::null()
            })
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        command.kill_on_drop(true);
        let captured = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let count = captured.clone();
        let request =
            listener.map(|listener| tokio::spawn(capture(listener, mode, auth_path.clone(), count)));
        let mut child = command.spawn().expect("isolated client");
        let input = child.stdin.take();
        let mut stdout = child.stdout.take().unwrap().take(4096);
        let mut stderr = child.stderr.take().unwrap().take(4096);
        let result = tokio::time::timeout(Duration::from_secs(10), async {
            if let Some(mut input) = input {
                input
                    .write_all(super::input("Synthetic question").as_bytes())
                    .await?;
            }
            let (mut out, mut error) = (Vec::new(), Vec::new());
            tokio::try_join!(stdout.read_to_end(&mut out), stderr.read_to_end(&mut error))?;
            if out.len() == 4096 || error.len() == 4096 {
                return Err(std::io::Error::other("status output exceeds probe limit"));
            }
            let status = child.wait().await?;
            Ok::<_, std::io::Error>((status, out, error))
        })
        .await;
        if !matches!(&result, Ok(Ok(_))) {
            let _ = child.kill().await;
            let _ = child.wait().await;
        }
        if !matches!(&result, Ok(Ok(_))) {
            println!(
                "probe mode={mode:?} captured={}",
                captured.load(std::sync::atomic::Ordering::SeqCst)
            );
        }
        if let Some(request) = request {
            request.abort();
            assert!(
                request.await.is_err_and(|error| error.is_cancelled()),
                "stand-in rejected authentication or request limits"
            );
            assert!(
                captured.load(std::sync::atomic::Ordering::SeqCst) > 0,
                "the installed client must reach the stand-in"
            );
        }
        let (status, out, error) = result.expect("status deadline").expect("status IO");
        let attempts = captured.load(std::sync::atomic::Ordering::SeqCst);
        if mode == Case::ManagedChatgpt401Switch {
            assert!(
                attempts >= 2,
                "the auth-switch case must observe a retry after its 401"
            );
        }
        println!("Codex auth {mode:?}: observed_requests={attempts}");
        if !mode.exec() {
            assert!(out.is_empty(), "status unexpectedly used stdout");
        }
        if matches!(
            mode,
            Case::ManagedApiStatus
                | Case::ApiExec
                | Case::ManagedApiExec
                | Case::SwitchedApiExec
                | Case::ManagedChatgpt401Switch
        ) {
            assert_eq!(status.code(), Some(1));
            assert!(
                std::fs::read(&auth_path).is_ok_and(|bytes| bytes == sentinel),
                "synthetic API auth must be retained"
            );
            continue;
        }
        if mode == Case::RestrictedExec {
            assert_eq!(status.code(), Some(1));
            assert!(
                !auth_path.exists(),
                "forced exec must characterize the destructive mismatch path"
            );
            assert!(
                error
                    .windows(b"Logging out".len())
                    .any(|bytes| bytes == b"Logging out"),
                "expected fixed mismatch logout diagnostic"
            );
            continue;
        }
        if mode == Case::ManagedChatgptStatus {
            assert!(
                status.success(),
                "managed policy must accept a synthetic ChatGPT login"
            );
            assert!(error.starts_with(b"Logged in using ChatGPT"));
            assert!(
                std::fs::read_to_string(&auth_path).is_ok_and(|text| text == chatgpt),
                "synthetic ChatGPT auth must be retained"
            );
            // Replace the fake login after status, before the next fresh exec loads auth.
            std::fs::write(&auth_path, sentinel).unwrap();
            continue;
        }
        if mode == Case::RestrictedStatus {
            assert_eq!(status.code(), Some(1));
            assert!(
                error == b"Not logged in\n",
                "expected the fixed auth refusal diagnostic"
            );
        } else {
            assert!(status.success(), "baseline status must accept synthetic API auth");
            assert!(
                error.starts_with(b"Logged in using an API key"),
                "expected API auth metadata"
            );
        }
        assert!(
            std::fs::read(&auth_path).is_ok_and(|bytes| bytes == sentinel),
            "status must retain the synthetic auth bytes"
        );
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Case {
    ApiStatus,
    RestrictedStatus,
    ManagedApiStatus,
    ApiExec,
    ManagedApiExec,
    ManagedChatgptStatus,
    SwitchedApiExec,
    ManagedChatgpt401Switch,
    RestrictedExec,
}
impl Case {
    fn exec(self) -> bool {
        matches!(
            self,
            Self::ApiExec
                | Self::ManagedApiExec
                | Self::SwitchedApiExec
                | Self::ManagedChatgpt401Switch
                | Self::RestrictedExec
        )
    }
}

struct Directory(PathBuf);
impl Drop for Directory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

async fn capture(
    listener: tokio::net::TcpListener,
    mode: Case,
    auth_path: PathBuf,
    count: std::sync::Arc<std::sync::atomic::AtomicUsize>,
) {
    loop {
        let mut socket: tokio::net::TcpStream = listener.accept().await.unwrap().0;
        let mut raw = Vec::new();
        let mut buf = [0u8; 8192];
        loop {
            let n = socket.read(&mut buf).await.unwrap();
            assert!(n > 0);
            raw.extend_from_slice(&buf[..n]);
            assert!(raw.len() < 128 * 1024);
            let Some(end) = raw.windows(4).position(|w| w == b"\r\n\r\n") else {
                continue;
            };
            let head = std::str::from_utf8(&raw[..end]).unwrap();
            assert!(head.starts_with("POST /v1/responses ") || head.starts_with("GET /v1/responses "));
            println!(
                "Codex auth {mode:?}: request={}",
                if head.starts_with("POST ") { "POST" } else { "GET" }
            );
            let attempt = count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            assert!(attempt < 16);
            let expected = match mode {
                Case::ApiExec => Some("Bearer sk-vults-synthetic-not-real"),
                Case::ManagedChatgpt401Switch => Some("Bearer synthetic-access"),
                _ => None,
            };
            let auth: Vec<_> = head
                .lines()
                .filter_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    name.eq_ignore_ascii_case("authorization").then_some(value.trim())
                })
                .collect();
            assert!(
                auth == expected.into_iter().collect::<Vec<_>>(),
                "only the expected synthetic authentication may reach the stand-in"
            );
            let response = if mode == Case::ManagedChatgpt401Switch && attempt == 0 {
                std::fs::write(
                    &auth_path,
                    br#"{"auth_mode":"apikey","OPENAI_API_KEY":"sk-vults-synthetic-not-real"}"#,
                )
                .unwrap();
                "HTTP/1.1 401 Unauthorized"
            } else {
                "HTTP/1.1 400 Bad Request"
            };
            socket.write_all(format!("{response}\r\ncontent-type: application/json\r\ncontent-length: 2\r\nconnection: close\r\n\r\n{{}}").as_bytes()).await.unwrap();
            break;
        }
    }
}
