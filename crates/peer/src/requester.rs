//! Linux MCP requester identity from kernel credentials and a live, same-user process chain.
use std::os::fd::AsRawFd;
use std::os::unix::fs::MetadataExt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Process {
    pub pid: u32,
    pub started: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Snapshot {
    process: Process,
    parent: u32,
    name: String,
    executable: (u64, u64),
}

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

fn shell_name(name: &str) -> bool {
    name != "busybox" && super::unix::SHELLS.contains(&name)
}

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
}

impl Requester {
    /// The stdio server and its parent, skipping only verified system-shell wrappers.
    pub fn stdio() -> Option<Self> {
        Self::capture(std::process::id())
    }

    /// The connected relay must be this executable, not merely a process claiming its name.
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
            let shell = process.is_shell();
            current = process.parent;
            chain.push(process);
            if !shell {
                return Some(Self { chain });
            }
        }
        None
    }

    pub fn relay(&self) -> Process {
        self.chain[0].process
    }
    pub fn client(&self) -> Process {
        self.chain[self.chain.len() - 1].process
    }

    /// Recheck before publishing: PID reuse, executable changes and reparenting revoke it.
    pub fn is_current(&self) -> bool {
        self.chain
            .iter()
            .all(|s| Snapshot::read(s.process.pid).as_ref() == Some(s))
    }
}

#[cfg(test)]
mod tests;
