//! The shared tool-free text client for journal, memory, sleep and voice questions.
//! User/project settings and implicit file/memory context are excluded; managed policy remains
//! enforced. Each app caller admits requests through its own network gate (ADR 0019).

use std::path::Path;
use std::process::Stdio;
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::Command;
use tokio::sync::watch;

/// Claude Code's name for its smallest model.
pub const MODEL: &str = "haiku";
/// A summary is a few sentences: past this, the call is stuck.
const TIMEOUT: Duration = Duration::from_secs(120);
/// Most of the reply read: a runaway answer never fills memory.
const MAX_REPLY: u64 = 64 * 1024;

/// Is `claude` on the `PATH`? Checked before the gate: a call that can't start sends nothing and
/// must not spend the budget.
pub fn installed() -> bool {
    on_path("claude")
}

pub(crate) fn on_path(program: &str) -> bool {
    std::env::var_os("PATH").is_some_and(|path| {
        std::env::split_paths(&path).any(|d| {
            d.join(program).is_file()
                || d.join(format!("{program}.exe")).is_file()
                || d.join(format!("{program}.cmd")).is_file()
        })
    })
}

fn command(system: &str, dir: &Path) -> Command {
    let mut cmd = Command::new("claude");
    cmd.envs(vults_agents::shell_env());
    cmd.args(["-p", "--output-format", "text", "--model", MODEL])
        // No offered built-in/MCP tools or saved session; managed policy remains enforced.
        .args(["--tools", ""])
        .args(["--setting-sources", ""])
        .arg("--disable-slash-commands")
        .args(["--disallowedTools", "mcp__*"])
        .arg("--strict-mcp-config")
        .arg("--no-session-persistence")
        .args(["--system-prompt", system])
        .env(vults_protocol::ZECA_ENV, "1")
        .env("CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC", "1")
        .env("CLAUDE_CODE_DISABLE_FILE_MENTIONS", "1")
        .env("CLAUDE_CODE_DISABLE_CLAUDE_MDS", "1")
        .env("CLAUDE_CODE_DISABLE_AUTO_MEMORY", "1")
        .current_dir(dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    crate::dies_with_app(&mut cmd);
    cmd
}

/// The model's answer to `text` under `system`, run in `dir` (an empty folder of the app's).
pub async fn summarize(system: &str, text: &str, dir: &Path) -> Result<String, String> {
    let (_stop, mut cancel) = watch::channel(false);
    question(system, text, dir, &mut cancel).await
}

/// The same tool-free client, with explicit child-exit acknowledgement on cancellation.
pub async fn question(
    system: &str,
    text: &str,
    dir: &Path,
    cancel: &mut watch::Receiver<bool>,
) -> Result<String, String> {
    question_bounded(
        system,
        text,
        dir,
        cancel,
        tokio::time::Instant::now() + TIMEOUT,
        MAX_REPLY - 1,
    )
    .await
}

/// One absolute deadline covers version qualification, stdin, output and exit acknowledgement.
/// Budget admission belongs to the app; this transport never retries or changes provider/model.
pub async fn question_bounded(
    system: &str,
    text: &str,
    dir: &Path,
    cancel: &mut watch::Receiver<bool>,
    deadline: tokio::time::Instant,
    max_reply: u64,
) -> Result<String, String> {
    if max_reply == 0 || max_reply >= MAX_REPLY {
        return Err("Invalid text reply limit.".into());
    }
    // Older clients silently ignore the switches that stop @file expansion and memory loading.
    let mut version = Command::new("claude");
    version
        .envs(vults_agents::shell_env())
        .arg("--version")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    crate::dies_with_app(&mut version);
    if !supported(&run_bounded(version, "", cancel, deadline, max_reply).await?) {
        return Err(
            "Tool-free text questions require Claude Code 2.1.299 or later in the 2.1 series.".into(),
        );
    }
    run_bounded(command(system, dir), text, cancel, deadline, max_reply).await
}
fn supported(version: &str) -> bool {
    let Some(version) = version.split_whitespace().next() else {
        return false;
    };
    let parts: Vec<_> = version.split('.').map(str::parse::<u32>).collect();
    matches!(parts.as_slice(), [Ok(2), Ok(1), Ok(patch)] if *patch >= 299)
}

pub(crate) async fn run(
    cmd: Command,
    text: &str,
    cancel: &mut watch::Receiver<bool>,
) -> Result<String, String> {
    run_bounded(
        cmd,
        text,
        cancel,
        tokio::time::Instant::now() + TIMEOUT,
        MAX_REPLY - 1,
    )
    .await
}

async fn run_bounded(
    mut cmd: Command,
    text: &str,
    cancel: &mut watch::Receiver<bool>,
    deadline: tokio::time::Instant,
    max_reply: u64,
) -> Result<String, String> {
    if *cancel.borrow() || cancel.has_changed().is_err() {
        return Err("Text consultation cancelled.".into());
    }
    if tokio::time::Instant::now() >= deadline {
        return Err("The text client took too long to answer.".into());
    }
    let child = cmd.spawn().map_err(|e| format!("Can't start text client: {e}"))?;
    run_child_bounded(child, text, cancel, deadline, max_reply).await
}
#[cfg(test)]
async fn run_child(
    child: tokio::process::Child,
    text: &str,
    cancel: &mut watch::Receiver<bool>,
) -> Result<String, String> {
    run_child_bounded(
        child,
        text,
        cancel,
        tokio::time::Instant::now() + TIMEOUT,
        MAX_REPLY - 1,
    )
    .await
}

async fn run_child_bounded(
    mut child: tokio::process::Child,
    text: &str,
    cancel: &mut watch::Receiver<bool>,
    deadline: tokio::time::Instant,
    max_reply: u64,
) -> Result<String, String> {
    let run = async {
        let mut stdin = child.stdin.take().ok_or("no stdin")?;
        stdin
            .write_all(text.as_bytes())
            .await
            .map_err(|e| e.to_string())?;
        drop(stdin);
        let mut out = Vec::new();
        child
            .stdout
            .take()
            .ok_or("no stdout")?
            .take(max_reply + 1)
            .read_to_end(&mut out)
            .await
            .map_err(|e| e.to_string())?;
        // Past the cap the rest is never read: end it rather than wait on a full pipe.
        if out.len() as u64 > max_reply {
            return Err("The text client's answer was too long.".into());
        }
        let status = child.wait().await.map_err(|e| e.to_string())?;
        if !status.success() {
            return Err(format!("The text client ended with {status}"));
        }
        // A cut inside a character never fails the call.
        Ok(String::from_utf8_lossy(&out).trim().to_string())
    };
    let result = tokio::select! { biased;
        _ = cancel.changed() => Err("Text consultation cancelled.".into()),
        result = tokio::time::timeout_at(deadline, run) => result.unwrap_or_else(|_| Err("The text client took too long to answer.".into())),
    };
    // The caller retains native ownership until the child has actually exited.
    if result.is_err() {
        let _ = child.kill().await;
        let _ = child.wait().await;
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_tools_no_settings_no_session() {
        let cmd = command("Sum up.", Path::new("/w"));
        let args: Vec<String> = cmd
            .as_std()
            .get_args()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        for pair in [
            ["--tools", ""],
            ["--setting-sources", ""],
            ["--model", MODEL],
            ["--system-prompt", "Sum up."],
        ] {
            assert!(args.windows(2).any(|w| w == pair), "{pair:?} in {args:?}");
        }
        assert!(args.contains(&"--strict-mcp-config".to_string()));
        assert!(args.contains(&"--no-session-persistence".to_string()));
        assert!(args.contains(&"--disable-slash-commands".to_string()));
        assert!(args.windows(2).any(|w| w == ["--disallowedTools", "mcp__*"]));
        for name in [
            "CLAUDE_CODE_DISABLE_FILE_MENTIONS",
            "CLAUDE_CODE_DISABLE_CLAUDE_MDS",
            "CLAUDE_CODE_DISABLE_AUTO_MEMORY",
            "CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC",
        ] {
            assert!(
                cmd.as_std()
                    .get_envs()
                    .any(|(k, v)| k == name && v == Some(std::ffi::OsStr::new("1")))
            );
        }
        assert!(!args.iter().any(|a| a.contains("permission")));
        for version in ["2.1.299 (Claude Code)", "2.1.304 (Claude Code)"] {
            assert!(supported(version));
        }
        for version in ["", "2.1.298", "2.1.299-dev", "3.0.0", "2.2.0", "other"] {
            assert!(!supported(version));
        }
    }

    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn cancellation_and_timeout_reap_the_tool_free_child_before_returning() {
        for timed_out in [false, true] {
            let mut cmd = Command::new("/bin/sleep");
            cmd.arg("600")
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::null());
            let child = crate::dies_with_app(&mut cmd).spawn().unwrap();
            let pid = child.id().unwrap();
            let (stop, mut cancel) = watch::channel(false);
            let worker = tokio::spawn(async move { run_child(child, "", &mut cancel).await });
            tokio::task::yield_now().await;
            if timed_out {
                tokio::time::pause();
                tokio::time::advance(TIMEOUT + Duration::from_secs(1)).await;
                tokio::time::resume();
            } else {
                stop.send(true).unwrap();
            }
            assert!(worker.await.unwrap().is_err());
            assert!(
                !Path::new(&format!("/proc/{pid}")).exists(),
                "return requires child exit, not just a kill request"
            );
        }
    }

    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn a_runaway_answer_is_bounded_and_never_accepted() {
        let mut cmd = Command::new("/usr/bin/head");
        cmd.args(["-c", "65536", "/dev/zero"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        let (_stop, mut cancel) = watch::channel(false);
        assert!(run(cmd, "", &mut cancel).await.unwrap_err().contains("too long"));
    }
    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn bounded_questions_enforce_inclusive_reply_limits_and_one_deadline_without_starting_expired_work()
    {
        let (_stop, mut cancel) = watch::channel(false);
        for bytes in [8192, 8193] {
            let mut cmd = Command::new("/usr/bin/head");
            cmd.args(["-c", &bytes.to_string(), "/dev/zero"])
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::null());
            let result = run_bounded(
                cmd,
                "",
                &mut cancel,
                tokio::time::Instant::now() + Duration::from_secs(2),
                8192,
            )
            .await;
            if bytes == 8192 {
                assert_eq!(result.unwrap().len(), 8192);
            } else {
                assert!(result.unwrap_err().contains("too long"));
            }
        }
        let mut cmd = Command::new("/bin/sleep");
        cmd.arg("600")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        let child = cmd.spawn().unwrap();
        let pid = child.id().unwrap();
        assert!(
            run_child_bounded(
                child,
                "",
                &mut cancel,
                tokio::time::Instant::now() + Duration::from_millis(20),
                8192
            )
            .await
            .is_err()
        );
        assert!(
            !Path::new(&format!("/proc/{pid}")).exists(),
            "deadline completion requires child exit"
        );
        let expired = tokio::time::Instant::now() - Duration::from_secs(1);
        assert!(
            run_bounded(
                Command::new("/binary-that-must-never-start"),
                "",
                &mut cancel,
                expired,
                8192
            )
            .await
            .unwrap_err()
            .contains("too long")
        );
        assert!(
            question_bounded("system", "text", Path::new("/unused"), &mut cancel, expired, 8192)
                .await
                .unwrap_err()
                .contains("too long")
        );
        assert!(
            question_bounded("system", "text", Path::new("/unused"), &mut cancel, expired, 0)
                .await
                .unwrap_err()
                .contains("Invalid")
        );
    }
}
