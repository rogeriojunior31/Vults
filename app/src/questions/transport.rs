//! Private same-user Unix transport, never the hook/permission socket.
use super::Command;
use serde::{Deserialize, Serialize};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, Ordering},
};
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
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

pub(crate) fn path() -> io::Result<PathBuf> {
    let vults_ipc::Endpoint::Unix(path) = vults_ipc::Endpoint::for_current_user()?;
    Ok(path.with_extension("questions.sock"))
}
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
    let listener = UnixListener::bind(&path)?;
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
    let slots = Arc::new(Semaphore::new(8));
    static NEXT: AtomicU64 = AtomicU64::new(1);
    loop {
        let (socket, _) = listener.accept().await?;
        let Ok(permit) = slots.clone().try_acquire_owned() else {
            continue;
        };
        let Some(proof) = Requester::socket(&socket) else {
            continue;
        };
        let connection = NEXT.fetch_add(1, Ordering::Relaxed);
        let tx = tx.clone();
        tokio::spawn(async move {
            let _permit = permit;
            let _ = connection_run(socket, proof, connection, &tx).await;
            let _ = tx.send(Command::Disconnect(connection)).await;
        });
    }
}
async fn connection_run(
    socket: UnixStream,
    proof: Requester,
    connection: u64,
    tx: &mpsc::Sender<Command>,
) -> io::Result<()> {
    let (read, mut write) = socket.into_split();
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
    socket: std::os::unix::net::UnixStream,
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
    use std::io::{Read, Write};
    let failed =
        || "The question connection ended. Ask a new question; this reply is never retried.".to_string();
    let mut socket = std::os::unix::net::UnixStream::connect(path).map_err(|_| failed())?;
    socket.set_read_timeout(Some(POLL)).map_err(|_| failed())?;
    socket
        .set_write_timeout(Some(Duration::from_secs(1)))
        .map_err(|_| failed())?;
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
        match socket.read(&mut buffer) {
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
