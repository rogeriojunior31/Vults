//! Private same-user transport (a Unix socket, or a named pipe on Windows), never the
//! hook/permission endpoint.
use super::Command;
use serde::{Deserialize, Serialize};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, Ordering},
};
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader};
use tokio::sync::{Semaphore, mpsc, oneshot};
use vults_peer::requester::Requester;
const LIMIT: u64 = 32 * 1024;
const POLL: Duration = Duration::from_millis(200);
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    request: String,
    question: String,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Response {
    text: Option<String>,
    error: Option<String>,
}

#[cfg(unix)]
type Socket = std::os::unix::net::UnixStream;
#[cfg(windows)]
type Socket = std::fs::File;

pub(crate) fn path() -> io::Result<PathBuf> {
    #[cfg(unix)]
    {
        let vults_ipc::Endpoint::Unix(path) = vults_ipc::Endpoint::for_current_user()?;
        Ok(path.with_extension("questions.sock"))
    }
    #[cfg(windows)]
    {
        let vults_ipc::Endpoint::Pipe(name) = vults_ipc::Endpoint::for_current_user()?;
        Ok(PathBuf::from(format!("{name}-questions")))
    }
}

static NEXT: AtomicU64 = AtomicU64::new(1);

fn spawn_connection<S>(socket: S, proof: Requester, slots: &Arc<Semaphore>, tx: &mpsc::Sender<Command>)
where
    S: AsyncRead + AsyncWrite + Send + 'static,
{
    let Ok(permit) = slots.clone().try_acquire_owned() else {
        return;
    };
    let connection = NEXT.fetch_add(1, Ordering::Relaxed);
    let tx = tx.clone();
    tokio::spawn(async move {
        let _permit = permit;
        let _ = connection_run(socket, proof, connection, &tx).await;
        let _ = tx.send(Command::Disconnect(connection)).await;
    });
}

/// Like the hook's pipe: a name another process holds is waited for (this app's previous run may
/// still be closing), and a client that is not this user's, or not this program, is dropped.
#[cfg(windows)]
pub(crate) async fn serve(path: PathBuf, tx: mpsc::Sender<Command>) -> io::Result<()> {
    use tokio::net::windows::named_pipe::ServerOptions;
    let slots = Arc::new(Semaphore::new(8));
    let create = |first| {
        ServerOptions::new()
            .first_pipe_instance(first)
            .reject_remote_clients(true)
            .create(&path)
    };
    let mut wait = Duration::from_millis(100);
    let mut server = loop {
        match create(true) {
            Ok(server) => break server,
            Err(_) if !tx.is_closed() => {
                tokio::time::sleep(wait).await;
                wait = (wait * 2).min(Duration::from_secs(5));
            }
            Err(e) => return Err(e),
        }
    };
    loop {
        if tx.is_closed() {
            return Ok(());
        }
        if server.connect().await.is_err() {
            tokio::time::sleep(POLL).await;
            continue;
        }
        let next = loop {
            match create(false) {
                Ok(next) => break next,
                Err(_) if !tx.is_closed() => tokio::time::sleep(POLL).await,
                Err(e) => return Err(e),
            }
        };
        let socket = std::mem::replace(&mut server, next);
        if let Some(proof) = Requester::pipe(&socket) {
            spawn_connection(socket, proof, &slots, &tx);
        }
    }
}

#[cfg(unix)]
pub(crate) async fn serve(path: PathBuf, tx: mpsc::Sender<Command>) -> io::Result<()> {
    use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
    let dir = path.parent().ok_or(io::ErrorKind::InvalidInput)?;
    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(dir)?;
    if !vults_peer::is_private_dir(dir) {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    if std::fs::symlink_metadata(&path).is_ok() {
        std::fs::remove_file(&path)?;
    }
    let listener = tokio::net::UnixListener::bind(&path)?;
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
    let slots = Arc::new(Semaphore::new(8));
    loop {
        let (socket, _) = listener.accept().await?;
        if slots.available_permits() == 0 {
            continue;
        }
        if let Some(proof) = Requester::socket(&socket) {
            spawn_connection(socket, proof, &slots, &tx);
        }
    }
}
async fn connection_run(
    socket: impl AsyncRead + AsyncWrite,
    proof: Requester,
    connection: u64,
    tx: &mpsc::Sender<Command>,
) -> io::Result<()> {
    let (read, mut write) = tokio::io::split(socket);
    let mut reader = BufReader::new(read.take(LIMIT));
    let mut line = String::new();
    tokio::time::timeout(Duration::from_secs(2), reader.read_line(&mut line)).await??;
    let request: Request = serde_json::from_str(&line).map_err(|_| io::ErrorKind::InvalidData)?;
    if !line.ends_with('\n') || line.len() as u64 >= LIMIT || !proof.is_current() {
        return Err(io::ErrorKind::InvalidData.into());
    }
    let received = Instant::now();
    let expires = received + vults_core::questions::LIFE;
    let (reply, mut result) = oneshot::channel();
    tx.send(Command::Open {
        connection,
        proof: proof.clone(),
        request: request.request,
        question: request.question,
        received,
        reply,
    })
    .await
    .map_err(|_| io::ErrorKind::BrokenPipe)?;
    let mut ticker = tokio::time::interval(POLL);
    let deadline = tokio::time::sleep_until(expires.into());
    tokio::pin!(deadline);
    let mut extra = [0];
    let answer = loop {
        tokio::select! {
            biased;
            result = &mut result => break result.unwrap_or_else(|_| Err("The question was cancelled.".into())),
            _ = &mut deadline => return Err(io::ErrorKind::TimedOut.into()),
            _ = ticker.tick() => if !proof.is_current() { return Err(io::ErrorKind::BrokenPipe.into()); },
            _ = reader.read(&mut extra) => return Err(io::ErrorKind::BrokenPipe.into()),
        }
    };
    if Instant::now() >= expires || !proof.is_current() {
        return Err(io::ErrorKind::BrokenPipe.into());
    }
    let delivered = answer.is_ok();
    let response = match answer {
        Ok(text) => Response {
            text: Some(text),
            error: None,
        },
        Err(error) => Response {
            text: None,
            error: Some(error),
        },
    };
    let mut bytes = serde_json::to_vec(&response).map_err(|_| io::ErrorKind::InvalidData)?;
    bytes.push(b'\n');
    tokio::time::timeout(Duration::from_secs(1), write.write_all(&bytes)).await??;
    if delivered {
        let mut ack = String::new();
        let sent = matches!(
            tokio::time::timeout(Duration::from_secs(1), reader.read_line(&mut ack)).await,
            Ok(Ok(_))
        ) && ack == "sent\n"
            && proof.is_current();
        let _ = tx.send(Command::Delivered(connection, sent)).await;
    }
    Ok(())
}

/// One blocking relay worker; closing on cancellation prevents late text from being reused.
pub(crate) struct RelayReply {
    pub text: String,
    socket: Socket,
}
impl RelayReply {
    pub(crate) fn acknowledge(&mut self) -> io::Result<()> {
        use std::io::Write;
        self.socket.write_all(b"sent\n")
    }
}
pub(crate) fn ask(
    path: &Path,
    request: String,
    question: String,
    cancelled: Arc<AtomicBool>,
) -> Result<RelayReply, String> {
    use std::io::Write;
    let failed =
        || "The question connection ended. Ask a new question; this reply is never retried.".to_string();
    let mut socket = connect(path).ok_or_else(failed)?;
    let mut bytes = serde_json::to_vec(&Request { request, question }).map_err(|_| failed())?;
    bytes.push(b'\n');
    socket.write_all(&bytes).map_err(|_| failed())?;
    let deadline = Instant::now() + vults_core::questions::LIFE;
    let mut response = Vec::new();
    let mut buffer = [0; 1024];
    loop {
        if cancelled.load(Ordering::Acquire) || Instant::now() >= deadline {
            return Err(failed());
        }
        match read_some(&mut socket, &mut buffer) {
            Ok(0) => break,
            Ok(n) => {
                response.extend_from_slice(&buffer[..n]);
                if response.len() as u64 >= LIMIT {
                    return Err(failed());
                }
                if response.ends_with(b"\n") {
                    break;
                }
            }
            Err(e) if matches!(e.kind(), io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock) => continue,
            Err(_) => return Err(failed()),
        }
    }
    if cancelled.load(Ordering::Acquire) {
        return Err(failed());
    }
    let result: Response = serde_json::from_slice(&response).map_err(|_| failed())?;
    match (result.text, result.error) {
        (Some(text), None) => Ok(RelayReply { text, socket }),
        (None, Some(error)) => Err(error),
        _ => Err(failed()),
    }
}

#[cfg(unix)]
fn connect(path: &Path) -> Option<Socket> {
    let socket = std::os::unix::net::UnixStream::connect(path).ok()?;
    socket.set_read_timeout(Some(POLL)).ok()?;
    socket.set_write_timeout(Some(Duration::from_secs(1))).ok()?;
    Some(socket)
}

#[cfg(unix)]
fn read_some(socket: &mut Socket, buffer: &mut [u8]) -> io::Result<usize> {
    use std::io::Read;
    socket.read(buffer)
}

#[cfg(windows)]
fn connect(path: &Path) -> Option<Socket> {
    use std::os::windows::fs::OpenOptionsExt;
    const ERROR_PIPE_BUSY: i32 = 231;
    /// SECURITY_IDENTIFICATION: a server that is not ours can never act as this user.
    const IDENTIFICATION: u32 = 0x0001_0000;
    for _ in 0..20 {
        match std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .security_qos_flags(IDENTIFICATION)
            .open(path)
        {
            Ok(pipe) => return vults_peer::pipe_server_is_same_user(&pipe).then_some(pipe),
            // Between two connections the next instance may not be open yet.
            Err(e) if e.raw_os_error() == Some(ERROR_PIPE_BUSY) => std::thread::sleep(POLL / 4),
            Err(_) => return None,
        }
    }
    None
}

/// A pipe read has no timeout: what waits is peeked first, so a cancellation is seen within POLL.
#[cfg(windows)]
fn read_some(socket: &mut Socket, buffer: &mut [u8]) -> io::Result<usize> {
    use std::io::Read;
    use std::os::windows::io::AsRawHandle;
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn PeekNamedPipe(
            pipe: *mut std::ffi::c_void,
            buffer: *mut u8,
            size: u32,
            read: *mut u32,
            available: *mut u32,
            left: *mut u32,
        ) -> i32;
    }
    let mut available = 0u32;
    // SAFETY: an open pipe handle, no buffer to fill, one valid out-pointer.
    let peeked = unsafe {
        PeekNamedPipe(
            socket.as_raw_handle(),
            std::ptr::null_mut(),
            0,
            std::ptr::null_mut(),
            &mut available,
            std::ptr::null_mut(),
        )
    };
    if peeked == 0 {
        // A closed server end is the end of the reply, as a socket's 0 would be.
        return Ok(0);
    }
    if available == 0 {
        std::thread::sleep(POLL);
        return Err(io::ErrorKind::TimedOut.into());
    }
    socket.read(buffer)
}
