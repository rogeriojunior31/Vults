use std::os::fd::AsRawFd;

pub fn current_uid() -> u32 {
    // SAFETY: getuid has no preconditions and cannot fail.
    unsafe { libc::getuid() }
}

/// The parent process, which for the hook is the agent that spawned it.
pub fn parent_pid() -> u32 {
    // SAFETY: getppid has no preconditions and cannot fail.
    unsafe { libc::getppid() as u32 }
}

/// Shells an agent may run its hooks through (`fish -c` keeps itself between them).
const SHELLS: &[&str] = &[
    "sh", "bash", "dash", "zsh", "fish", "ksh", "mksh", "tcsh", "csh", "busybox",
];

/// The agent that ran this hook: the parent, or past the shells it ran the hook through (on Linux,
/// a few hops at most), so the process recorded is the agent's own, not a shell gone once the hook
/// ends. Elsewhere, the parent.
pub fn agent_pid() -> u32 {
    agent_of(parent_pid())
}

/// The first process from `pid` up that is not a shell.
pub fn agent_of(pid: u32) -> u32 {
    #[cfg(target_os = "linux")]
    {
        let mut pid = pid;
        for _ in 0..4 {
            match stat(pid) {
                Some((comm, ppid)) if SHELLS.contains(&comm.as_str()) && ppid > 1 => pid = ppid,
                _ => break,
            }
        }
        pid
    }
    #[cfg(not(target_os = "linux"))]
    pid
}

/// A process's name and parent, from `/proc/<pid>/stat` (the name sits in parentheses and may
/// hold spaces or parentheses itself: the last `)` closes it).
#[cfg(target_os = "linux")]
fn stat(pid: u32) -> Option<(String, u32)> {
    let text = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    let (open, close) = (text.find('(')?, text.rfind(')')?);
    let comm = text.get(open + 1..close)?.to_string();
    let mut rest = text.get(close + 1..)?.split_whitespace();
    let _state = rest.next()?;
    let ppid = rest.next()?.parse().ok()?;
    Some((comm, ppid))
}

/// UID of the process on the other end of a connected Unix socket.
#[cfg(any(target_os = "linux", target_os = "android"))]
pub fn peer_uid(socket: &impl AsRawFd) -> Option<u32> {
    let mut cred = libc::ucred {
        pid: 0,
        uid: 0,
        gid: 0,
    };
    let mut len = std::mem::size_of::<libc::ucred>() as libc::socklen_t;
    // SAFETY: `cred` and `len` are valid for writes and sized for SO_PEERCRED.
    let ok = unsafe {
        libc::getsockopt(
            socket.as_raw_fd(),
            libc::SOL_SOCKET,
            libc::SO_PEERCRED,
            (&mut cred as *mut libc::ucred).cast(),
            &mut len,
        )
    } == 0;
    ok.then_some(cred.uid)
}

/// UID of the process on the other end of a connected Unix socket.
#[cfg(not(any(target_os = "linux", target_os = "android")))]
pub fn peer_uid(socket: &impl AsRawFd) -> Option<u32> {
    let (mut uid, mut gid) = (0, 0);
    // SAFETY: both out-pointers are valid for writes.
    let ok = unsafe { libc::getpeereid(socket.as_raw_fd(), &mut uid, &mut gid) } == 0;
    ok.then_some(uid)
}

pub fn peer_is_same_user(socket: &impl AsRawFd) -> bool {
    peer_uid(socket) == Some(current_uid())
}

/// A real directory (not a symlink), owned by us, closed to group and others. The socket's
/// folder must be one: in the shared temp dir another account could create it first.
pub fn is_private_dir(dir: &std::path::Path) -> bool {
    use std::os::unix::fs::MetadataExt;
    std::fs::symlink_metadata(dir)
        .is_ok_and(|m| m.file_type().is_dir() && m.uid() == current_uid() && m.mode() & 0o077 == 0)
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_socket_pair_is_the_same_user() {
        let (a, _b) = std::os::unix::net::UnixStream::pair().unwrap();
        assert!(super::peer_is_same_user(&a));
    }

    #[test]
    fn only_a_closed_real_directory_of_ours_is_private() {
        use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
        use std::path::Path;

        let root = std::env::temp_dir().join(format!("vults-peer-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();

        let private = root.join("private");
        std::fs::DirBuilder::new().mode(0o700).create(&private).unwrap();
        assert!(super::is_private_dir(&private));

        let open = root.join("open");
        std::fs::DirBuilder::new().mode(0o700).create(&open).unwrap();
        std::fs::set_permissions(&open, std::fs::Permissions::from_mode(0o755)).unwrap();
        assert!(!super::is_private_dir(&open));

        // The link points at a private folder, but the link is what the path names.
        let link = root.join("link");
        std::os::unix::fs::symlink(&private, &link).unwrap();
        assert!(!super::is_private_dir(&link));

        assert!(!super::is_private_dir(&root.join("missing")));
        assert!(!super::is_private_dir(Path::new("/")));
        let _ = std::fs::remove_dir_all(&root);
    }
}

#[cfg(all(test, target_os = "linux"))]
mod agent_tests {
    use super::*;

    #[test]
    fn the_agent_is_found_past_a_shell() {
        // A shell that keeps itself (two commands), as `fish -c` does for one: from it, the walk
        // reaches whoever ran it, this test.
        let mut sh = std::process::Command::new("sh")
            .args(["-c", "sleep 2; true"])
            .spawn()
            .unwrap();
        std::thread::sleep(std::time::Duration::from_millis(100));
        assert_eq!(agent_of(sh.id()), std::process::id());
        // A process that isn't a shell is its own agent.
        assert_eq!(agent_of(std::process::id()), std::process::id());
        let _ = sh.kill();
        let _ = sh.wait();
    }
}
