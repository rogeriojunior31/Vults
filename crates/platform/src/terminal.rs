//! A new terminal window running one fixed command, for a step only the user can take there (an
//! agent's sign-in). The command is the app's own words, never text the user or an agent typed.

use std::process::{Command, Stdio};

/// Opens a terminal running `command` (its program and arguments) and leaves it to the user.
pub fn run(command: &[&str]) -> Result<(), String> {
    if command.is_empty()
        || command
            .iter()
            .any(|a| a.is_empty() || a.contains(['\0', '\n', '\r']))
    {
        return Err("Nothing to run.".into());
    }
    for mut candidate in candidates(command) {
        let started = candidate
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
        if started.is_ok() {
            return Ok(());
        }
    }
    Err("No terminal could be opened. Run the command in your own terminal.".into())
}

#[cfg(windows)]
fn candidates(command: &[&str]) -> Vec<Command> {
    use std::os::windows::process::CommandExt;
    // Windows Terminal when it is there; the console host otherwise. `cmd /k` keeps the window
    // open after the command, so the user reads how it went.
    let mut wt = Command::new("wt.exe");
    wt.args(["new-tab", "cmd", "/k"]).args(command);
    let mut cmd = Command::new("cmd.exe");
    // `start` opens a window of its own; the empty string is its title.
    cmd.args(["/c", "start", "", "cmd", "/k"]).args(command);
    cmd.creation_flags(0x0800_0000);
    vec![wt, cmd]
}

#[cfg(target_os = "linux")]
fn candidates(command: &[&str]) -> Vec<Command> {
    let mut out = Vec::new();
    // The user's own choice first, as many desktops and tools honour it.
    if let Ok(terminal) = std::env::var("TERMINAL")
        && !terminal.trim().is_empty()
        && !terminal.contains(char::is_whitespace)
    {
        let mut c = Command::new(terminal);
        c.arg("-e").args(command);
        out.push(c);
    }
    for (program, flag) in [
        ("konsole", Some("-e")),
        ("gnome-terminal", Some("--")),
        ("kgx", Some("--")),
        ("kitty", None),
        ("alacritty", Some("-e")),
        ("foot", None),
        ("wezterm", Some("start")),
        ("xfce4-terminal", Some("-x")),
        ("x-terminal-emulator", Some("-e")),
        ("xterm", Some("-e")),
    ] {
        let mut c = Command::new(program);
        if let Some(flag) = flag {
            c.arg(flag);
        }
        c.args(command);
        out.push(c);
    }
    out
}

#[cfg(not(any(windows, target_os = "linux")))]
fn candidates(_command: &[&str]) -> Vec<Command> {
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_plain_fixed_command_runs() {
        assert!(run(&[]).is_err());
        assert!(run(&["claude", ""]).is_err());
        assert!(run(&["claude", "auth\nrm -rf /"]).is_err());
        let built = candidates(&["claude", "auth", "login"]);
        assert!(!built.is_empty());
        for c in &built {
            let args: Vec<String> = c.get_args().map(|a| a.to_string_lossy().into_owned()).collect();
            assert!(
                args.ends_with(&["claude".into(), "auth".into(), "login".into()]),
                "{args:?}"
            );
        }
    }
}
