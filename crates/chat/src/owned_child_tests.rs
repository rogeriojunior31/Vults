use std::time::{Duration, Instant};

/// A shell that starts a grandchild, as an npm shim starts node, and both keep running.
fn shim(marker: &std::path::Path) -> tokio::process::Command {
    let script = format!(
        "start /b \"\" powershell.exe -NoProfile -Command \"$PID | Out-File -Encoding ascii '{}'; Start-Sleep 60\" & ping -n 60 127.0.0.1 >nul",
        marker.display()
    );
    let mut cmd = crate::process::command("cmd.exe");
    cmd.raw_arg(format!("/d /s /c \"{script}\""))
        .stdout(std::process::Stdio::null())
        .kill_on_drop(true);
    cmd
}

async fn grandchild(marker: &std::path::Path) -> u32 {
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        if let Some(pid) = std::fs::read_to_string(marker)
            .ok()
            .and_then(|s| s.trim().parse().ok())
        {
            return pid;
        }
        assert!(Instant::now() < deadline, "the grandchild never started");
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

async fn gone(pid: u32) -> bool {
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        if vults_peer::process_birth(pid) == Some(None) {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    false
}

#[tokio::test]
async fn dropping_a_cli_ends_what_its_shim_started() {
    let dir = std::env::temp_dir();
    let pid = std::process::id();
    // Before: kill_on_drop ends the shell only; its grandchild runs on.
    let plain = dir.join(format!("vults-owned-plain-{pid}"));
    let _ = std::fs::remove_file(&plain);
    let child = shim(&plain).spawn().unwrap();
    let survivor = grandchild(&plain).await;
    drop(child);
    let survived = !gone(survivor).await;
    let _ = std::process::Command::new("taskkill")
        .args(["/PID", &survivor.to_string(), "/F"])
        .output();
    assert!(survived, "a plain spawn already ended the tree");
    // Owned: the job ends it too.
    let owned = dir.join(format!("vults-owned-job-{pid}"));
    let _ = std::fs::remove_file(&owned);
    let child = crate::spawn_owned(&mut shim(&owned)).unwrap();
    let tree = grandchild(&owned).await;
    drop(child);
    assert!(gone(tree).await, "the grandchild outlived its CLI");
    let _ = std::fs::remove_file(plain);
    let _ = std::fs::remove_file(owned);
}
