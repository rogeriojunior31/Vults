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

#[cfg(test)]
mod tests {
    #[test]
    fn a_socket_pair_is_the_same_user() {
        let (a, _b) = std::os::unix::net::UnixStream::pair().unwrap();
        assert!(super::peer_is_same_user(&a));
    }
}
