//! Exact tmux pane delivery; no shell, clipboard, focus recovery or submission.
use super::{focus::Window, process};
use crate::jump::{ancestors, my_uid};
use std::io::Read;
use std::os::unix::fs::{FileTypeExt, MetadataExt};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use vults_protocol::Terminal;

#[derive(Clone, PartialEq, Eq)]
struct Socket {
    path: String,
    device: u64,
    inode: u64,
}
impl Socket {
    fn open(path: &str) -> Option<Self> {
        if !path.starts_with('/') || path.len() > 107 || path.chars().any(char::is_control) {
            return None;
        }
        let meta = std::fs::symlink_metadata(path).ok()?;
        (meta.file_type().is_socket() && meta.uid() == my_uid()?).then(|| Self {
            path: path.into(),
            device: meta.dev(),
            inode: meta.ino(),
        })
    }
    fn current(&self) -> bool {
        Self::open(&self.path).as_ref() == Some(self)
    }
}

pub struct Pane {
    socket: Socket,
    id: String,
    agent: u32,
    started: String,
    client: u32,
    client_started: String,
    pane_pid: u32,
    pane_started: String,
    kitty: Option<(Socket, u64)>,
}
impl std::fmt::Debug for Pane {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Pane(tmux)")
    }
}

impl Pane {
    /// The caller must also retain a key-down window capability; titles never select a pane.
    pub fn capture(window: &Window, terminal: &Terminal) -> Option<Self> {
        let agent = terminal.pid?;
        let id = terminal.env.get("TMUX_PANE")?;
        if !numeric_id(id, '%') {
            return None;
        }
        let socket = Socket::open(terminal.env.get("TMUX")?.split(',').next()?)?;
        let rows = clients(&socket)?;
        let mut matches = rows.iter().filter(|row| row.id == *id && row.safe);
        let row = matches.next()?;
        // Several clients displaying this pane would make foreground ownership ambiguous.
        if matches.next().is_some()
            || !ancestors(agent).contains(&row.pid)
            || !foreground(agent)
            || !ancestors(row.client).contains(&window.pid?)
        {
            return None;
        }
        let class = window.class.to_ascii_lowercase();
        let kitty = if matches!(class.as_str(), "kitty" | "net.kovidgoyal.kitty") {
            let socket = Socket::open(terminal.env.get("KITTY_LISTEN_ON")?.strip_prefix("unix:")?)?;
            let id = terminal.env.get("KITTY_WINDOW_ID")?.parse::<u64>().ok()?;
            kitty_focus(&socket, id, row.client)
                .then_some((socket, id))?
                .into()
        } else {
            // Shared terminal servers need a native active-pane proof of their own.
            if !matches!(class.as_str(), "xterm" | "uxterm") {
                return None;
            }
            None
        };
        Some(Self {
            socket,
            id: id.clone(),
            agent,
            started: process(agent)?,
            client: row.client,
            client_started: process(row.client)?,
            pane_pid: row.pid,
            pane_started: process(row.pid)?,
            kitty,
        })
    }

    pub fn current(&self, window: &Window) -> bool {
        if !self.alive() {
            return false;
        }
        let Some(rows) = clients(&self.socket) else {
            return false;
        };
        let matches: Vec<_> = rows.iter().filter(|r| r.id == self.id && r.safe).collect();
        if matches.len() != 1 || matches[0].client != self.client || matches[0].pid != self.pane_pid {
            return false;
        }
        match &self.kitty {
            Some((socket, id)) => kitty_focus(socket, *id, self.client),
            None => window
                .pid
                .is_some_and(|pid| ancestors(self.client).contains(&pid)),
        }
    }

    fn alive(&self) -> bool {
        self.socket.current()
            && process(self.agent).as_deref() == Some(&self.started)
            && process(self.client).as_deref() == Some(&self.client_started)
            && process(self.pane_pid).as_deref() == Some(&self.pane_started)
            && ancestors(self.agent).contains(&self.pane_pid)
            && foreground(self.agent)
    }

    /// The server checks the pane again in the same command queue as the literal byte write.
    pub fn send(&self, text: &str) -> bool {
        let Some(args) = send_args(&self.id, self.pane_pid, self.client, text) else {
            return false;
        };
        self.alive() && run(tmux(&self.socket, &args)).is_some_and(|reply| reply.trim() == "vults-delivered")
    }
}

fn numeric_id(id: &str, prefix: char) -> bool {
    id.strip_prefix(prefix)
        .is_some_and(|n| !n.is_empty() && n.len() < 12 && n.bytes().all(|c| c.is_ascii_digit()))
}

fn foreground(pid: u32) -> bool {
    let Ok(stat) = std::fs::read_to_string(format!("/proc/{pid}/stat")) else {
        return false;
    };
    let Some((_, tail)) = stat.rsplit_once(')') else {
        return false;
    };
    let fields: Vec<_> = tail.split_whitespace().collect();
    // A background agent must not feed the shell or a different foreground job.
    fields
        .get(2)
        .zip(fields.get(5))
        .is_some_and(|(group, foreground)| group == foreground && *group != "0")
}

struct Client {
    client: u32,
    id: String,
    pid: u32,
    safe: bool,
}
fn clients(socket: &Socket) -> Option<Vec<Client>> {
    let reply = run(tmux(
        socket,
        &[
            "list-clients".into(),
            "-F".into(),
            "#{client_pid} #{pane_id} #{pane_pid} #{pane_active} #{pane_in_mode} #{pane_dead}".into(),
        ],
    ))?;
    Some(
        reply
            .lines()
            .filter_map(|line| {
                let parts: Vec<_> = line.split_whitespace().collect();
                if parts.len() != 6 || !numeric_id(parts[1], '%') {
                    return None;
                }
                Some(Client {
                    client: parts[0].parse().ok()?,
                    id: parts[1].into(),
                    pid: parts[2].parse().ok()?,
                    safe: parts[3..] == ["1", "0", "0"],
                })
            })
            .collect(),
    )
}

fn send_args(id: &str, pid: u32, client: u32, text: &str) -> Option<Vec<String>> {
    if !numeric_id(id, '%')
        || pid < 2
        || client < 2
        || text.is_empty()
        || text.len() > 8192
        || text
            .chars()
            .any(|c| c.is_control() || matches!(c, '\u{2028}' | '\u{2029}'))
    {
        return None;
    }
    // Hex bytes avoid tmux's command separator parser and any interpretation of key names.
    let bytes = text
        .as_bytes()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<Vec<_>>()
        .join(" ");
    let attached = format!("#{{m:*|{client}|*,#{{L:#{{?client_session,|#{{client_pid}}|,}}}}}}");
    let pane = format!(
        "#{{&&:#{{window_active}},#{{&&:#{{==:#{{pane_pid}},{pid}}},#{{&&:#{{pane_active}},#{{&&:#{{==:#{{pane_in_mode}},0}},#{{==:#{{pane_dead}},0}}}}}}}}}}"
    );
    Some(vec![
        "if-shell".into(),
        "-F".into(),
        "-t".into(),
        id.into(),
        format!("#{{&&:{attached},{pane}}}"),
        format!("send-keys -H -t {id} {bytes} ; display-message -p vults-delivered"),
        String::new(),
    ])
}

fn tmux(socket: &Socket, args: &[String]) -> Command {
    let mut command = Command::new("tmux");
    command.args(["-S", &socket.path]).args(args);
    command
}

fn kitty_focus(socket: &Socket, id: u64, client: u32) -> bool {
    if !socket.current() {
        return false;
    }
    let mut command = Command::new("kitten");
    command.args([
        "@",
        "--to",
        &format!("unix:{}", socket.path),
        "--use-password",
        "never",
        "ls",
        "--match",
        &format!("id:{id}"),
    ]);
    let Some(reply) = run(command) else { return false };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&reply) else {
        return false;
    };
    let Some(root) = kitty_pane(&value, id) else {
        return false;
    };
    if !ancestors(client).contains(&root) || !foreground(client) {
        return false;
    }
    let tty = |pid| {
        std::fs::metadata(format!("/proc/{pid}/fd/0"))
            .ok()
            .filter(|m| m.file_type().is_char_device())
            .map(|m| m.rdev())
    };
    tty(root)
        .zip(tty(client))
        .is_some_and(|(root, client)| root == client)
}
fn kitty_pane(value: &serde_json::Value, id: u64) -> Option<u32> {
    let windows = value.as_array()?;
    let focused: Vec<_> = windows
        .iter()
        .filter(|w| w["is_focused"] == true)
        .flat_map(|w| w["tabs"].as_array().into_iter().flatten())
        .filter(|tab| tab["is_focused"] == true)
        .flat_map(|tab| tab["windows"].as_array().into_iter().flatten())
        .filter(|pane| pane["is_focused"] == true)
        .collect();
    if focused.len() != 1 || focused[0]["id"].as_u64() != Some(id) {
        return None;
    }
    u32::try_from(focused[0]["pid"].as_u64()?).ok()
}

/// Bounded, noninteractive local IPC. Neither command output nor arguments are logged.
fn run(mut command: Command) -> Option<String> {
    use rustix::fs::{OFlags, fcntl_getfl, fcntl_setfl};
    struct Child(std::process::Child);
    impl Drop for Child {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    let mut child = Child(
        command
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .ok()?,
    );
    let mut output = child.0.stdout.take()?;
    fcntl_setfl(&output, fcntl_getfl(&output).ok()? | OFlags::NONBLOCK).ok()?;
    let started = Instant::now();
    let mut bytes = Vec::new();
    loop {
        let mut chunk = [0u8; 4096];
        match output.read(&mut chunk) {
            Ok(n) if bytes.len() + n <= 65_536 => bytes.extend_from_slice(&chunk[..n]),
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
            _ => return None,
        }
        if let Some(status) = child.0.try_wait().ok()? {
            if !status.success() {
                return None;
            }
            output
                .take((65_537 - bytes.len()) as u64)
                .read_to_end(&mut bytes)
                .ok()?;
            return (bytes.len() <= 65_536)
                .then(|| String::from_utf8(bytes).ok())
                .flatten();
        }
        if started.elapsed() >= Duration::from_millis(150) {
            return None;
        }
        std::thread::sleep(Duration::from_millis(2));
    }
}

#[cfg(test)]
mod tests;
