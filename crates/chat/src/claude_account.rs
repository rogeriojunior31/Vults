//! A read-only Claude control request: no initialization, user message, tool or model turn.
//! Experimental CLI versions can refuse it; statusLine remains an independent source.

use serde_json::{Value, json};
use std::{path::Path, process::Stdio};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    process::Command,
};

fn command() -> Command {
    let mut cmd = Command::new("claude");
    cmd.envs(vults_agents::shell_env()).args([
        "--print",
        "--input-format",
        "stream-json",
        "--output-format",
        "stream-json",
        "--verbose",
        "--safe-mode",
        "--strict-mcp-config",
        "--mcp-config",
        "{\"mcpServers\":{}}",
        "--no-session-persistence",
    ]);
    cmd
}

pub(crate) async fn read(dir: &Path) -> Result<Value, String> {
    let mut cmd = command();
    read_command(crate::dies_with_app(&mut cmd).current_dir(dir)).await
}

async fn read_command(cmd: &mut Command) -> Result<Value, String> {
    let mut child = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| e.to_string())?;
    let mut stdin = child.stdin.take().ok_or("Claude usage: no stdin")?;
    let stdout = child.stdout.take().ok_or("Claude usage: no stdout")?;
    const ID: &str = "vults-account-usage";
    let request = json!({"type":"control_request","request_id":ID,"request":{"subtype":"get_usage","skip_behaviors":true}});
    stdin
        .write_all(format!("{request}\n").as_bytes())
        .await
        .map_err(|e| e.to_string())?;
    stdin.flush().await.map_err(|e| e.to_string())?;
    // Cap the entire response stream, including unrecognized messages; never retain stderr.
    let mut lines = BufReader::new(stdout).take(256 * 1024).lines();
    let result = loop {
        let Some(line) = lines.next_line().await.map_err(|e| e.to_string())? else {
            break Err("Claude usage: no supported response".into());
        };
        let Ok(message) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        let response = &message["response"];
        if message["type"] == "control_response" && response["request_id"] == ID {
            break if response["subtype"] == "success" {
                Ok(response["response"].clone())
            } else {
                Err("Claude usage: control request unavailable".into())
            };
        }
    };
    let _ = child.kill().await;
    let _ = child.wait().await;
    result
}

#[cfg(test)]
mod tests {
    #[cfg(unix)]
    use super::*;

    #[cfg(unix)]
    #[tokio::test]
    async fn allowance_probe_sends_one_control_request_and_keeps_only_its_reply() {
        let path = std::env::temp_dir().join(format!("vults-claude-usage-{}", std::process::id()));
        let mut cmd = Command::new("sh");
        cmd.args(["-c", r#"read line
printf '%s\n' "$line" > "$VULTS_USAGE_REQUEST"
printf '%s\n' '{"type":"control_response","response":{"request_id":"another-reader","subtype":"success","response":{"wrong":true}}}'
printf '%s\n' '{"type":"control_response","response":{"request_id":"vults-account-usage","subtype":"success","response":{"rate_limits_available":true,"rate_limits":{"seven_day":{"utilization":12}}}}}'
cat > /dev/null"#]).env("VULTS_USAGE_REQUEST",&path);
        let response = read_command(&mut cmd).await.unwrap();
        assert_eq!(response["rate_limits"]["seven_day"]["utilization"], 12);
        let request: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(
            request,
            json!({"type":"control_request","request_id":"vults-account-usage","request":{"subtype":"get_usage","skip_behaviors":true}})
        );
        std::fs::remove_file(path).unwrap();
        let cmd = command();
        let args: Vec<_> = cmd
            .as_std()
            .get_args()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        for flag in ["--safe-mode", "--strict-mcp-config", "--no-session-persistence"] {
            assert!(args.iter().any(|arg| arg == flag));
        }
        assert!(!args.iter().any(|arg| arg == "--resume" || arg == "--continue"));
    }

    #[tokio::test]
    #[ignore = "Read-only installed Claude account probe; may contact Anthropic"]
    async fn installed_claude_allowance_is_reported_without_a_model_turn() {
        let dir = std::env::temp_dir();
        let windows = crate::claude_usage(&dir).await.unwrap();
        assert!(!windows.is_empty());
        assert!(
            windows
                .iter()
                .all(|w| w.agent == vults_protocol::AgentKind::Claude)
        );
        eprintln!("{} numeric quota windows reported", windows.len());
    }
}
