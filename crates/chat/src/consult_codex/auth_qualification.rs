//! Synthetic auth only: the real home is covered before the installed client starts.
#![cfg(target_os = "linux")]
#![allow(clippy::unwrap_used)]

use std::{
    os::unix::fs::{DirBuilderExt, PermissionsExt},
    path::PathBuf,
    process::Stdio,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use tokio::{io::AsyncReadExt, process::Command};

#[tokio::test]
#[ignore = "requires the installed Codex CLI and bwrap user/network/mount namespaces"]
async fn subscription_status_refuses_synthetic_api_auth_without_changing_it() {
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

    for restricted in [false, true] {
        let candidate = super::subscription_status();
        let mut command = Command::new("bwrap");
        command.args(["--unshare-all", "--die-with-parent", "--ro-bind", "/", "/"]);
        command.args(["--bind"]).arg(&synthetic_home).arg(&original_home);
        command.args(["--ro-bind"]).arg(&binary).arg(&binary);
        command.args([
            "--tmpfs", "/run", "--tmpfs", "/tmp", "--proc", "/proc", "--dev", "/dev", "--chdir", "/tmp",
        ]);
        command.arg(&binary);
        if restricted {
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
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        command.kill_on_drop(true);
        let mut child = command.spawn().expect("isolated client");
        let mut stdout = child.stdout.take().unwrap().take(4096);
        let mut stderr = child.stderr.take().unwrap().take(4096);
        let result = tokio::time::timeout(Duration::from_secs(10), async {
            let (mut out, mut error) = (Vec::new(), Vec::new());
            tokio::try_join!(stdout.read_to_end(&mut out), stderr.read_to_end(&mut error))?;
            if out.len() == 4096 || error.len() == 4096 {
                return Err(std::io::Error::other("status output exceeds probe limit"));
            }
            Ok::<_, std::io::Error>((child.wait().await?, out, error))
        })
        .await;
        if !matches!(&result, Ok(Ok(_))) {
            let _ = child.kill().await;
            let _ = child.wait().await;
        }
        let (status, out, error) = result.expect("status deadline").expect("status IO");
        assert!(out.is_empty(), "status unexpectedly used stdout");
        if restricted {
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

struct Directory(PathBuf);
impl Drop for Directory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
