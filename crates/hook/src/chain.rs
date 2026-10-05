//! The user's own Claude Code status line, kept running behind ours. The installer saved its
//! statusLine object beside this binary; we run its command with the same stdin and print what
//! it prints, unchanged (colors included). Whatever goes wrong prints nothing: a blank status
//! line, never a broken Claude Code screen, and still exit 0.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use serde_json::Value;

/// The sidecar's name, as the installer writes it (`agent_config::status_line::PREVIOUS_FILE`).
pub const PREVIOUS_FILE: &str = "statusline-previous.json";
/// How long the old command may take. Claude Code redraws the line often: a slow script just
/// shows nothing this time.
pub const TIMEOUT: Duration = Duration::from_secs(10);
/// A status line is one or a few lines; a runaway command gets cut here.
const MAX_OUTPUT: u64 = 64 * 1024;

/// The sidecar beside this binary.
pub fn previous_file() -> Option<PathBuf> {
    Some(std::env::current_exe().ok()?.with_file_name(PREVIOUS_FILE))
}

pub struct Running {
    child: Child,
    output: mpsc::Receiver<Vec<u8>>,
    started: Instant,
}

/// Starts the saved command, if there is one, feeding it `input`.
pub fn start(previous: &Path, input: &[u8]) -> Option<Running> {
    let saved: Value = serde_json::from_slice(&std::fs::read(previous).ok()?).ok()?;
    let command = saved.get("command")?.as_str()?.trim();
    // Ours would run itself over and over.
    if command.is_empty() || command.contains(vultures_ai_brand::HOOK_BIN) {
        return None;
    }
    let mut sh = Command::new("sh");
    sh.arg("-c")
        .arg(command)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    // A group of its own, so a timeout takes what the script started too (a hung git or curl).
    #[cfg(unix)]
    std::os::unix::process::CommandExt::process_group(&mut sh, 0);
    let mut child = sh.spawn().ok()?;
    // Both ends on threads: a command that never reads its stdin, or never closes its stdout,
    // must not hold us past the deadline.
    if let Some(mut stdin) = child.stdin.take() {
        let input = input.to_vec();
        std::thread::spawn(move || {
            let _ = stdin.write_all(&input);
        });
    }
    let stdout = child.stdout.take()?;
    let (tx, output) = mpsc::channel();
    std::thread::spawn(move || {
        let mut out = Vec::new();
        let _ = stdout.take(MAX_OUTPUT).read_to_end(&mut out);
        let _ = tx.send(out);
    });
    Some(Running {
        child,
        output,
        started: Instant::now(),
    })
}

impl Running {
    /// What the command printed, once it closes its stdout; nothing if that takes past `timeout`.
    pub fn finish(mut self, timeout: Duration) -> Vec<u8> {
        let left = timeout.saturating_sub(self.started.elapsed());
        match self.output.recv_timeout(left) {
            Ok(out) => {
                let _ = self.child.try_wait();
                out
            }
            Err(_) => {
                #[cfg(unix)]
                let _ = Command::new("kill")
                    .args(["-KILL", "--", &format!("-{}", self.child.id())])
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status();
                let _ = self.child.kill();
                let _ = self.child.wait();
                Vec::new()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sidecar(name: &str, saved: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("vultures-ai-chain-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(PREVIOUS_FILE);
        std::fs::write(&path, saved).unwrap();
        path
    }

    fn chain(name: &str, command: &str, input: &[u8], timeout: Duration) -> Vec<u8> {
        let saved = serde_json::json!({ "type": "command", "command": command, "padding": 0 });
        let path = sidecar(name, &saved.to_string());
        start(&path, input).map(|r| r.finish(timeout)).unwrap_or_default()
    }

    #[test]
    fn the_same_name_as_the_installer() {
        assert_eq!(
            PREVIOUS_FILE,
            vultures_ai_agent_config::status_line::PREVIOUS_FILE
        );
    }

    #[test]
    fn its_output_comes_through_unchanged_with_the_same_stdin() {
        let input = br#"{"model":{"display_name":"Opus"},"rate_limits":null}"#;
        // `cat` proves the stdin; the ANSI color and the missing final newline stay as printed.
        let out = chain("echo", "printf '\\033[32mok\\033[0m '; cat", input, TIMEOUT);
        let mut want = b"\x1b[32mok\x1b[0m ".to_vec();
        want.extend_from_slice(input);
        assert_eq!(out, want);
        // A failing command still shows what it printed.
        assert_eq!(chain("fail", "echo half; exit 3", b"", TIMEOUT), b"half\n");
    }

    #[test]
    fn a_slow_command_prints_nothing_in_time() {
        let started = Instant::now();
        let out = chain("slow", "sleep 5; echo late", b"{}", Duration::from_millis(300));
        assert!(out.is_empty());
        assert!(started.elapsed() < Duration::from_secs(3));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn a_timeout_takes_what_the_command_started() {
        let pid_file = sidecar("grandchild-pid", "{}").with_file_name("sleep.pid");
        let command = format!("sleep 30 & echo $! > '{}'; wait", pid_file.display());
        assert!(chain("grandchild", &command, b"{}", Duration::from_secs(1)).is_empty());
        let pid = std::fs::read_to_string(&pid_file).unwrap();
        let proc = PathBuf::from(format!("/proc/{}", pid.trim()));
        // Killed, then reaped by whoever adopted it.
        let gone = (0..50).any(|_| {
            std::thread::sleep(Duration::from_millis(20));
            std::fs::read_to_string(proc.join("stat")).map_or(true, |stat| stat.contains(") Z "))
        });
        assert!(gone, "the sleep outlived the timeout");
    }

    #[test]
    fn nothing_to_run_prints_nothing() {
        // A command that does not exist: sh says so on stderr, which never reaches the screen.
        assert!(chain("missing", "/no/such/status-line", b"{}", TIMEOUT).is_empty());
        // Empty-ish output stays what it was.
        assert_eq!(chain("blank", "printf '\\n'", b"{}", TIMEOUT), b"\n");
        assert!(chain("empty", "true", b"{}", TIMEOUT).is_empty());
        // No sidecar, a broken one, no command, or our own command: nothing starts.
        let dir = std::env::temp_dir().join(format!("vultures-ai-chain-{}-none", std::process::id()));
        assert!(start(&dir.join(PREVIOUS_FILE), b"{}").is_none());
        assert!(start(&sidecar("broken", "{oops"), b"{}").is_none());
        assert!(start(&sidecar("nocmd", r#"{"type":"command"}"#), b"{}").is_none());
        assert!(start(&sidecar("blankcmd", r#"{"command":"  "}"#), b"{}").is_none());
        let ours = r#"{"command":"'/x/vultures-ai-hook' --agent claude --statusline"}"#;
        assert!(start(&sidecar("ours", ours), b"{}").is_none());
    }

    #[test]
    fn a_command_that_never_reads_its_stdin_is_fine() {
        let big = vec![b'x'; 1 << 20];
        assert_eq!(chain("deaf", "echo hi", &big, TIMEOUT), b"hi\n");
    }
}
