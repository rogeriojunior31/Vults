//! A short text from a small model with no tools (plan-zeca M4: each past day of the journal):
//! through the user's Claude Code, on their subscription, with none of their settings, hooks or
//! MCP servers, and no tool at all, so the text it reads (agents' commands, chat lines) can only
//! ever become words. The app asks the network gate before each call (ADR 0019).

use std::path::Path;
use std::process::Stdio;
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::Command;

/// Claude Code's name for its smallest model.
pub const MODEL: &str = "haiku";
/// A summary is a few sentences: past this, the call is stuck.
const TIMEOUT: Duration = Duration::from_secs(120);
/// Most of the reply read: a runaway answer never fills memory.
const MAX_REPLY: u64 = 64 * 1024;

fn command(system: &str, dir: &Path) -> Command {
    let mut cmd = Command::new("claude");
    cmd.envs(vults_agents::shell_env());
    cmd.args(["-p", "--output-format", "text", "--model", MODEL])
        // No tools, no settings (no hooks, no allow rules), no MCP servers, nothing kept.
        .args(["--tools", ""])
        .args(["--setting-sources", ""])
        .arg("--strict-mcp-config")
        .arg("--no-session-persistence")
        .args(["--system-prompt", system])
        .env(vults_protocol::ZECA_ENV, "1")
        .current_dir(dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    crate::dies_with_app(&mut cmd);
    cmd
}

/// The model's answer to `text` under `system`, run in `dir` (an empty folder of the app's).
pub async fn summarize(system: &str, text: &str, dir: &Path) -> Result<String, String> {
    let run = async {
        let mut child = command(system, dir)
            .spawn()
            .map_err(|e| format!("Can't start Claude Code: {e}"))?;
        let mut stdin = child.stdin.take().ok_or("no stdin")?;
        stdin
            .write_all(text.as_bytes())
            .await
            .map_err(|e| e.to_string())?;
        drop(stdin);
        let mut out = String::new();
        child
            .stdout
            .take()
            .ok_or("no stdout")?
            .take(MAX_REPLY)
            .read_to_string(&mut out)
            .await
            .map_err(|e| e.to_string())?;
        let status = child.wait().await.map_err(|e| e.to_string())?;
        if !status.success() {
            return Err(format!("Claude Code ended with {status}"));
        }
        Ok(out.trim().to_string())
    };
    tokio::time::timeout(TIMEOUT, run)
        .await
        .map_err(|_| "Claude Code took too long to answer.".to_string())?
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
        assert!(!args.iter().any(|a| a.contains("permission")));
    }
}
