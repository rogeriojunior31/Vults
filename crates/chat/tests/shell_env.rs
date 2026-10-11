//! An agent the app starts itself gets the folder variables the user's login shell sets and the
//! app lacks (started from the desktop): Codex's usage read, against a stand-in `codex` on the
//! PATH that writes down the `CODEX_HOME` it was given. One test: it changes the environment. The
//! stand-in is a shell script, which Windows can't run as `codex`: Unix only.
#![cfg(unix)]
#![allow(clippy::unwrap_used)]

#[tokio::test]
async fn the_usage_reads_codex_with_the_shells_codex_home() {
    let root = std::env::temp_dir().join(format!("vults-shell-env-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let bin = root.join("bin");
    std::fs::create_dir_all(&bin).unwrap();
    let out = root.join("codex-home.txt");
    // Says nothing and ends: the read fails, and nothing is left running.
    std::fs::write(
        bin.join("codex"),
        format!("#!/bin/sh\nprintf '%s' \"$CODEX_HOME\" > '{}'\n", out.display()),
    )
    .unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(bin.join("codex"), std::fs::Permissions::from_mode(0o755)).unwrap();
    let path = format!("{}:{}", bin.display(), std::env::var("PATH").unwrap_or_default());
    // SAFETY: this binary's only test, before any other thread reads the environment.
    unsafe {
        std::env::set_var("PATH", path);
        std::env::remove_var("CODEX_HOME");
    }
    let home = root.join("codex-elsewhere");
    let given = home.clone();
    vults_agents::set_shell_env(move || vec![("CODEX_HOME".into(), given.clone().into())]);

    assert!(vults_chat::codex_usage(&root).await.is_err());
    assert_eq!(std::fs::read_to_string(&out).unwrap(), home.to_string_lossy());
    let _ = std::fs::remove_dir_all(&root);
}
