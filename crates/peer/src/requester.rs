//! MCP requester identity from the OS's own view of the peer and a live, same-user process chain:
//! kernel credentials and `/proc` on Linux, the pipe's client and process handles on Windows.
#[cfg(target_os = "linux")]
use std::os::fd::AsRawFd;
#[cfg(target_os = "linux")]
use std::os::unix::fs::MetadataExt;

#[cfg(windows)]
pub(crate) mod windows;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Process {
    pub pid: u32,
    pub started: u64,
}

/// What the process runs: device and inode on Linux, the lowercased image path on Windows.
#[cfg(target_os = "linux")]
type Executable = (u64, u64);
#[cfg(windows)]
type Executable = String;

#[derive(Clone, Debug, PartialEq, Eq)]
struct Snapshot {
    process: Process,
    parent: u32,
    name: String,
    executable: Executable,
}

#[cfg(target_os = "linux")]
impl Snapshot {
    fn read(pid: u32) -> Option<Self> {
        let dir = format!("/proc/{pid}");
        if pid <= 1 || std::fs::metadata(&dir).ok()?.uid() != crate::current_uid() {
            return None;
        }
        let read = || parse(&std::fs::read_to_string(format!("{dir}/stat")).ok()?, pid);
        let (name, parent, started) = read()?;
        let exe = std::fs::metadata(format!("{dir}/exe")).ok()?;
        if read()? != (name.clone(), parent, started) {
            return None;
        }
        if std::fs::metadata(&dir).ok()?.uid() != crate::current_uid() {
            return None;
        }
        Some(Self {
            process: Process { pid, started },
            parent,
            name,
            executable: (exe.dev(), exe.ino()),
        })
    }

    fn is_shell(&self) -> bool {
        // comm is mutable; require a system shell. A multicall binary does not prove shell mode.
        shell_name(&self.name)
            && ["/bin", "/usr/bin"].iter().any(|dir| {
                let Ok(path) = std::fs::canonicalize(format!("{dir}/{}", self.name)) else {
                    return false;
                };
                path.file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(shell_name)
                    && std::fs::metadata(path)
                        .is_ok_and(|exe| exe.uid() == 0 && (exe.dev(), exe.ino()) == self.executable)
            })
    }
}

#[cfg(target_os = "linux")]
impl Snapshot {
    /// Linux launchers exec the tool in their own process: nothing stays between.
    fn is_launcher(&self) -> bool {
        false
    }
}

#[cfg(target_os = "linux")]
fn shell_name(name: &str) -> bool {
    name != "busybox" && super::unix::SHELLS.contains(&name)
}

#[cfg(target_os = "linux")]
fn parse(stat: &str, pid: u32) -> Option<(String, u32, u64)> {
    let open = stat.find('(')?;
    if stat.get(..open)?.trim().parse::<u32>().ok()? != pid {
        return None;
    }
    let close = stat.rfind(')')?;
    let name = stat.get(open + 1..close)?.to_owned();
    let fields: Vec<_> = stat.get(close + 1..)?.split_whitespace().collect();
    if matches!(*fields.first()?, "Z" | "X" | "x") {
        return None;
    }
    let parent = fields.get(1)?.parse().ok()?;
    let started = fields.get(19)?.parse().ok()?;
    (started > 0).then_some((name, parent, started))
}

/// Not deserializable: names and PIDs in MCP messages cannot manufacture this proof.
#[derive(Clone, Debug)]
pub struct Requester {
    chain: Vec<Snapshot>,
    /// Verified wrappers between the client and whoever started it: on Windows an npm `.cmd` shim
    /// runs the CLI under `cmd.exe`, a mise shim under `mise.exe`. Always empty on Linux.
    above: Vec<Snapshot>,
}

impl Requester {
    /// The stdio server and its parent, skipping only verified system-shell wrappers.
    pub fn stdio() -> Option<Self> {
        Self::capture(std::process::id())
    }

    /// The connected relay must be this executable, not merely a process claiming its name.
    #[cfg(target_os = "linux")]
    pub fn socket(socket: &impl AsRawFd) -> Option<Self> {
        if !crate::peer_is_same_user(socket) {
            return None;
        }
        let relay = crate::peer_pid(socket)?;
        let proof = Self::capture(relay)?;
        let own = Snapshot::read(std::process::id())?;
        (proof.chain.first()?.executable == own.executable).then_some(proof)
    }

    fn capture(relay: u32) -> Option<Self> {
        Self::with(relay, Snapshot::read)
    }

    fn with(relay: u32, mut read: impl FnMut(u32) -> Option<Snapshot>) -> Option<Self> {
        let server = read(relay)?;
        let mut current = server.parent;
        let mut chain = vec![server];
        for _ in 0..=4 {
            if chain.iter().any(|s| s.process.pid == current) {
                return None;
            }
            let process = read(current)?;
            // A Windows parent id outlives its process: one born after its child is someone else.
            if cfg!(windows)
                && chain
                    .last()
                    .is_some_and(|child| process.process.started > child.process.started)
            {
                return None;
            }
            let shell = process.is_shell();
            current = process.parent;
            chain.push(process);
            if !shell {
                let above = if cfg!(windows) {
                    Self::shells_above(&chain, &mut read)
                } else {
                    Vec::new()
                };
                return Some(Self { chain, above });
            }
        }
        None
    }

    /// Up to three verified shells or version-manager launchers (an npm `.cmd` shim, a mise shim and
    /// `mise.exe`) right above the client, oldest last; none past a stale parent.
    fn shells_above(chain: &[Snapshot], read: &mut impl FnMut(u32) -> Option<Snapshot>) -> Vec<Snapshot> {
        let mut above: Vec<Snapshot> = Vec::new();
        let Some(mut child) = chain.last().cloned() else {
            return above;
        };
        while above.len() < 3 {
            let Some(parent) = read(child.parent) else { break };
            if !(parent.is_shell() || parent.is_launcher())
                || parent.process.started > child.process.started
                || chain
                    .iter()
                    .chain(&above)
                    .any(|s| s.process.pid == parent.process.pid)
            {
                break;
            }
            above.push(parent.clone());
            child = parent;
        }
        above
    }

    pub fn relay(&self) -> Process {
        self.chain[0].process
    }
    pub fn client(&self) -> Process {
        self.chain[self.chain.len() - 1].process
    }

    /// The caller is a direct child of the app, after verified shell wrappers.
    pub fn launched_by(&self, app_pid: u32) -> bool {
        self.above
            .last()
            .or(self.chain.last())
            .is_some_and(|top| top.parent == app_pid)
            && self.is_current()
    }

    /// Recheck before publishing: PID reuse, executable changes and reparenting revoke it.
    pub fn is_current(&self) -> bool {
        self.chain
            .iter()
            .chain(&self.above)
            .all(|s| Snapshot::read(s.process.pid).as_ref() == Some(s))
    }
}

#[cfg(all(test, target_os = "linux"))]
mod tests;
