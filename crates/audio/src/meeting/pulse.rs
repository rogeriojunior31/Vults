//! One local connection per recording; shutdown never waits for a server acknowledgement.

use std::future::Future;
use std::net::Shutdown;
use std::os::unix::net::UnixStream;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::task::{Context, Poll, Waker};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use super::control::{Control, Phase};

pub(super) const TIMEOUT: Duration = Duration::from_secs(2);
const CHECK_EVERY: Duration = Duration::from_millis(10);

#[derive(Debug)]
pub(crate) struct Connection {
    pub client: pulseaudio::Client,
    socket: UnixStream,
    control: Option<Control>,
    stop: mpsc::Sender<()>,
    watchdog: Option<JoinHandle<()>>,
}

impl Connection {
    pub(crate) fn inspect() -> Result<Self, String> {
        Self::open(None, 0)
    }

    pub(super) fn open(control: Option<Control>, seconds: u32) -> Result<Self, String> {
        let path = local_socket()?;
        Self::connect(&path, control, seconds, cookie()?.as_deref())
    }

    fn connect(
        path: &std::path::Path,
        control: Option<Control>,
        seconds: u32,
        cookie: Option<&[u8]>,
    ) -> Result<Self, String> {
        let socket = socket2::Socket::new(socket2::Domain::UNIX, socket2::Type::STREAM, None)
            .map_err(|e| e.to_string())?;
        socket
            .connect_timeout(
                &socket2::SockAddr::unix(path).map_err(|e| e.to_string())?,
                TIMEOUT,
            )
            .map_err(|e| format!("audio connection failed: {e}"))?;
        let fd: std::os::fd::OwnedFd = socket.into();
        let socket = UnixStream::from(fd);
        let interrupt = socket.try_clone().map_err(|e| e.to_string())?;
        let transport = socket.try_clone().map_err(|e| e.to_string())?;
        transport
            .set_read_timeout(Some(TIMEOUT))
            .map_err(|e| e.to_string())?;
        transport
            .set_write_timeout(Some(TIMEOUT))
            .map_err(|e| e.to_string())?;
        let authenticated = Arc::new(AtomicBool::new(false));
        let ready = authenticated.clone();
        let shared = control.clone();
        let (stop, stopped) = mpsc::channel();
        let watchdog = std::thread::Builder::new()
            .name("meeting-audio-deadline".into())
            .spawn(move || {
                let started = Instant::now();
                loop {
                    let expired = !ready.load(Ordering::SeqCst) && started.elapsed() >= TIMEOUT;
                    let ended = shared.as_ref().is_some_and(|control| {
                        let status = control.status();
                        !matches!(status.phase, Phase::Starting | Phase::Recording)
                            || status.elapsed >= Duration::from_secs(u64::from(seconds))
                    });
                    if expired || ended {
                        break;
                    }
                    match stopped.recv_timeout(CHECK_EVERY) {
                        Err(RecvTimeoutError::Timeout) => {}
                        _ => break,
                    }
                }
                let _ = interrupt.shutdown(Shutdown::Both);
            })
            .map_err(|e| e.to_string())?;
        // The watchdog also bounds the protocol's synchronous authentication handshake.
        let client = pulseaudio::Client::new_unix(c"Vults meeting capture", transport, cookie);
        authenticated.store(true, Ordering::SeqCst);
        match client {
            Ok(client) => Ok(Self {
                client,
                socket,
                control,
                stop,
                watchdog: Some(watchdog),
            }),
            Err(error) => {
                let _ = stop.send(());
                let _ = socket.shutdown(Shutdown::Both);
                let _ = watchdog.join();
                Err(format!("audio authentication failed: {error}"))
            }
        }
    }

    pub fn wait<T>(
        &self,
        future: impl Future<Output = Result<T, pulseaudio::ClientError>>,
    ) -> Result<T, String> {
        let deadline = Instant::now() + TIMEOUT;
        let mut future = std::pin::pin!(future);
        let mut context = Context::from_waker(Waker::noop());
        loop {
            if self.control.as_ref().is_some_and(|control| !control.continuing()) {
                return Err("audio operation interrupted".into());
            }
            match future.as_mut().poll(&mut context) {
                Poll::Ready(result) => return result.map_err(|e| e.to_string()),
                Poll::Pending if Instant::now() >= deadline => return Err("audio operation timed out".into()),
                Poll::Pending => std::thread::sleep(CHECK_EVERY),
            }
        }
    }

    pub fn close(&self) {
        let _ = self.socket.shutdown(Shutdown::Both);
    }
}

impl Drop for Connection {
    fn drop(&mut self) {
        self.close();
        let _ = self.stop.send(());
        // This thread only waits on our bounded local channel, never on an audio reply.
        if let Some(watchdog) = self.watchdog.take() {
            let _ = watchdog.join();
        }
    }
}

fn local_socket() -> Result<std::path::PathBuf, String> {
    // Do not silently switch to another server when an explicit endpoint is invalid or remote.
    if let Some(server) = std::env::var_os("PULSE_SERVER") {
        return explicit_socket(&server);
    }
    std::env::var_os("PULSE_RUNTIME_PATH")
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::var_os("XDG_RUNTIME_DIR").map(|p| std::path::PathBuf::from(p).join("pulse")))
        .filter(|p| p.is_absolute())
        .map(|p| p.join("native"))
        .ok_or_else(|| "no local audio runtime directory found".into())
}

fn explicit_socket(server: &std::ffi::OsStr) -> Result<std::path::PathBuf, String> {
    server
        .to_str()
        .and_then(|s| s.strip_prefix("unix:"))
        .map(std::path::PathBuf::from)
        .filter(|p| p.is_absolute())
        .ok_or_else(|| "meeting audio requires an absolute local Unix socket".into())
}

fn cookie() -> Result<Option<Vec<u8>>, String> {
    use std::io::Read;
    use std::os::unix::fs::OpenOptionsExt;
    let Some(path) = pulseaudio::cookie_path_from_env() else {
        return Ok(None);
    };
    let file = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)
        .map_err(|_| "could not read the local audio authentication cookie")?;
    let meta = file
        .metadata()
        .map_err(|_| "could not inspect the local audio authentication cookie")?;
    if !meta.is_file() || meta.len() != 256 {
        return Err("the local audio authentication cookie must be a 256-byte regular file".into());
    }
    let mut cookie = Vec::new();
    file.take(257)
        .read_to_end(&mut cookie)
        .map_err(|_| "could not read the local audio authentication cookie")?;
    if cookie.len() != 256 {
        return Err("the local audio authentication cookie changed while reading".into());
    }
    Ok(Some(cookie))
}

#[cfg(test)]
mod tests;
