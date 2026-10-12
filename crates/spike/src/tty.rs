//! A terminal for an agent's TUI with nobody at it: a pty that answers what the TUI asks the
//! terminal (cursor position, colors, its kind), so it starts; types nothing unless asked; and
//! stops the agent after a while. Our hook answers its cards, so a turn runs to its end alone.

use std::ffi::CString;
use std::io::Write;
use std::os::fd::{FromRawFd, OwnedFd, RawFd};
use std::time::{Duration, Instant};

/// What a TUI asks the terminal, and what a dark 120×40 xterm answers.
const ANSWERS: &[(&[u8], &[u8])] = &[
    (b"\x1b[6n", b"\x1b[1;1R"),
    (b"\x1b]10;?", b"\x1b]10;rgb:ffff/ffff/ffff\x1b\\"),
    (b"\x1b]11;?", b"\x1b]11;rgb:0000/0000/0000\x1b\\"),
    (b"\x1b[c", b"\x1b[?62;22c"),
    (b"\x1b[?u", b"\x1b[?0u"),
];

/// Runs `cmd` for at most `limit`, its screen appended to `screen`; `keys` are typed once, `at`
/// after it starts.
pub fn run(
    cmd: &[String],
    limit: Duration,
    screen: Option<&std::path::Path>,
    keys: Option<(&[u8], Duration)>,
) -> std::io::Result<()> {
    let argv: Vec<CString> = cmd
        .iter()
        .map(|a| CString::new(a.as_bytes()))
        .collect::<Result<_, _>>()
        .map_err(std::io::Error::other)?;
    if argv.is_empty() {
        return Err(std::io::Error::other("no command"));
    }
    let mut ptrs: Vec<*const libc::c_char> = argv.iter().map(|a| a.as_ptr()).collect();
    ptrs.push(std::ptr::null());
    let size = libc::winsize {
        ws_row: 40,
        ws_col: 120,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };
    let mut master: RawFd = -1;
    // SAFETY: forkpty with a valid out-pointer and window size; the child only execs (or exits).
    let pid = unsafe { libc::forkpty(&mut master, std::ptr::null_mut(), std::ptr::null(), &size) };
    if pid < 0 {
        return Err(std::io::Error::last_os_error());
    }
    if pid == 0 {
        // SAFETY: argv is a null-terminated array of valid C strings that outlives the call.
        unsafe {
            if std::env::var_os("TERM").is_none() {
                libc::setenv(c"TERM".as_ptr(), c"xterm-256color".as_ptr(), 1);
            }
            libc::execvp(ptrs[0], ptrs.as_ptr());
            libc::_exit(127);
        }
    }
    // SAFETY: forkpty gave the parent this descriptor, owned here alone.
    let master = unsafe { OwnedFd::from_raw_fd(master) };
    let mut log = match screen {
        Some(p) => Some(std::fs::OpenOptions::new().create(true).append(true).open(p)?),
        None => None,
    };
    let start = Instant::now();
    let mut keys = keys;
    let mut buf = [0u8; 65536];
    while start.elapsed() < limit {
        if let Some((k, at)) = keys
            && start.elapsed() >= at
        {
            write_all(&master, k);
            keys = None;
        }
        let mut poll = libc::pollfd {
            fd: raw(&master),
            events: libc::POLLIN,
            revents: 0,
        };
        // SAFETY: one valid pollfd.
        if unsafe { libc::poll(&mut poll, 1, 200) } <= 0 {
            continue;
        }
        // SAFETY: reading into a buffer of its own length.
        let n = unsafe { libc::read(raw(&master), buf.as_mut_ptr().cast(), buf.len()) };
        if n <= 0 {
            break;
        }
        let data = &buf[..n as usize];
        if let Some(l) = log.as_mut() {
            let _ = l.write_all(data);
        }
        for (asked, answer) in ANSWERS {
            if data.windows(asked.len()).any(|w| w == *asked) {
                write_all(&master, answer);
            }
        }
    }
    // SAFETY: signalling the child this function started.
    unsafe {
        libc::kill(pid, libc::SIGTERM);
        std::thread::sleep(Duration::from_millis(500));
        libc::kill(pid, libc::SIGKILL);
        libc::waitpid(pid, std::ptr::null_mut(), 0);
    }
    Ok(())
}

fn raw(fd: &OwnedFd) -> RawFd {
    std::os::fd::AsRawFd::as_raw_fd(fd)
}

fn write_all(fd: &OwnedFd, mut bytes: &[u8]) {
    while !bytes.is_empty() {
        // SAFETY: writing from a live slice of its own length.
        let n = unsafe { libc::write(raw(fd), bytes.as_ptr().cast(), bytes.len()) };
        if n <= 0 {
            return;
        }
        bytes = &bytes[n as usize..];
    }
}
