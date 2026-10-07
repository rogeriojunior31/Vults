//! Server end of the hook relay, independent of Tauri.
//!
//! Every event is handed to the app as an [`Incoming`]. An event that `wants_reply` keeps its
//! connection open until the app answers through its [`ReplyHandle`]. The agent is never
//! blocked by us, because:
//! * a human is only awaited once the UI has *acknowledged* the card is on screen, so an
//!   app that is paused or not listening costs [`limits::ACK_TIMEOUT`], not two minutes;
//! * whatever happens the connection is dropped after [`limits::SERVER_DECISION_TIMEOUT`],
//!   and no answer at all means the agent asks in its terminal.

use std::io;
use std::sync::Arc;

use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::sync::{Semaphore, mpsc};
use tokio::time::timeout;
use vults_protocol::{self as protocol, Answer, Decision, DecodeError, Event, Reply, limits};

#[derive(Debug)]
pub enum Incoming {
    /// Fire and forget.
    Event(Event),
    /// The hook is waiting: answer through `reply`.
    Request { event: Event, reply: ReplyHandle },
}

/// How the app answers a waiting hook. Cheap to clone; extra answers are ignored.
#[derive(Clone, Debug)]
pub struct ReplyHandle(mpsc::Sender<Signal>);

#[derive(Debug)]
enum Signal {
    Ack,
    Decide(Decision),
    Answer(Vec<Answer>),
    Decline,
}

impl ReplyHandle {
    /// The card is on screen and a human can act on it: the long wait may begin.
    pub fn ack(&self) {
        let _ = self.0.try_send(Signal::Ack);
    }

    /// Only ever called from a human's click (enforced in `core`).
    pub fn decide(&self, decision: Decision) {
        let _ = self.0.try_send(Signal::Decide(decision));
    }

    /// The user's replies to a question, one per question (checked in `core`).
    pub fn answer(&self, answers: Vec<Answer>) {
        let _ = self.0.try_send(Signal::Answer(answers));
    }

    /// Nobody can act on it: the agent asks in its terminal right away.
    /// Dropping every clone of the handle does the same.
    pub fn decline(&self) {
        let _ = self.0.try_send(Signal::Decline);
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Endpoint {
    #[cfg(unix)]
    Unix(std::path::PathBuf),
    #[cfg(windows)]
    Pipe(String),
}

impl Endpoint {
    /// The endpoint `vults-hook` connects to for the current user.
    pub fn for_current_user() -> io::Result<Self> {
        #[cfg(unix)]
        {
            let runtime = std::env::var_os("XDG_RUNTIME_DIR").map(std::path::PathBuf::from);
            Ok(Self::Unix(protocol::socket_path(
                runtime.as_deref(),
                &std::env::temp_dir(),
                vults_peer::current_uid(),
            )))
        }
        #[cfg(windows)]
        {
            let sid = vults_peer::current_user_sid()
                .ok_or_else(|| io::Error::other("cannot read the current user's SID"))?;
            Ok(Self::Pipe(protocol::pipe_name(&sid)))
        }
    }
}

/// Binds the endpoint and serves until the receiver of `incoming` is dropped.
/// Binding happens before the first `.await`, so a failure is reported at once.
pub async fn serve(endpoint: Endpoint, incoming: mpsc::Sender<Incoming>) -> io::Result<()> {
    let slots = Arc::new(Semaphore::new(limits::MAX_CONNECTIONS));
    #[cfg(unix)]
    {
        let Endpoint::Unix(path) = endpoint;
        serve_unix(path, incoming, slots).await
    }
    #[cfg(windows)]
    {
        let Endpoint::Pipe(name) = endpoint;
        serve_pipe(name, incoming, slots).await
    }
}

/// The socket this process bound (path, device, inode), so a quit removes only its own file.
#[cfg(unix)]
static BOUND: std::sync::Mutex<Option<(std::path::PathBuf, u64, u64)>> = std::sync::Mutex::new(None);

/// Removes the socket on a clean quit, unless the file there is no longer the one this process
/// bound (another instance took the path since). Nothing to do on Windows: a pipe goes with us.
pub fn remove_socket() {
    #[cfg(unix)]
    {
        if let Some((path, dev, ino)) = BOUND.lock().unwrap_or_else(|e| e.into_inner()).take() {
            remove_if_same(&path, dev, ino);
        }
    }
}

#[cfg(unix)]
fn remove_if_same(path: &std::path::Path, dev: u64, ino: u64) {
    use std::os::unix::fs::MetadataExt;
    if std::fs::metadata(path).is_ok_and(|m| m.dev() == dev && m.ino() == ino) {
        let _ = std::fs::remove_file(path);
    }
}

#[cfg(unix)]
async fn serve_unix(
    path: std::path::PathBuf,
    incoming: mpsc::Sender<Incoming>,
    slots: Arc<Semaphore>,
) -> io::Result<()> {
    use std::os::unix::fs::{DirBuilderExt, PermissionsExt};

    if let Some(dir) = path.parent() {
        // Only matters for the temp-dir fallback; $XDG_RUNTIME_DIR is already 0700.
        std::fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(dir)?;
        if !vults_peer::is_private_dir(dir) {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                format!(
                    "{} is not a private folder of this user; not serving there",
                    dir.display()
                ),
            ));
        }
    }
    // Single instance is the app's job; a socket file still here is a leftover from a crash.
    let _ = std::fs::remove_file(&path);
    let listener = tokio::net::UnixListener::bind(&path)?;
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
    if let Ok(meta) = std::fs::metadata(&path) {
        use std::os::unix::fs::MetadataExt;
        *BOUND.lock().unwrap_or_else(|e| e.into_inner()) = Some((path.clone(), meta.dev(), meta.ino()));
    }

    loop {
        if incoming.is_closed() {
            return Ok(());
        }
        let Ok((stream, _)) = listener.accept().await else {
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            continue;
        };
        // Belt and braces on top of the 0700 folder.
        if !vults_peer::peer_is_same_user(&stream) {
            continue;
        }
        spawn_connection(stream, &incoming, &slots);
    }
}

#[cfg(windows)]
async fn serve_pipe(name: String, incoming: mpsc::Sender<Incoming>, slots: Arc<Semaphore>) -> io::Result<()> {
    use tokio::net::windows::named_pipe::ServerOptions;

    // first_pipe_instance: refuse to serve on a name somebody else already owns.
    let mut server = ServerOptions::new().first_pipe_instance(true).create(&name)?;
    loop {
        if incoming.is_closed() {
            return Ok(());
        }
        if server.connect().await.is_err() {
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            continue;
        }
        let connected = std::mem::replace(&mut server, ServerOptions::new().create(&name)?);
        if !vults_peer::pipe_client_is_same_user(&connected) {
            continue;
        }
        spawn_connection(connected, &incoming, &slots);
    }
}

fn spawn_connection<S>(conn: S, incoming: &mpsc::Sender<Incoming>, slots: &Arc<Semaphore>)
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    // Over the limit the connection is dropped: the hook gives up and the agent carries on.
    let Ok(permit) = slots.clone().try_acquire_owned() else {
        return;
    };
    let incoming = incoming.clone();
    tokio::spawn(async move {
        handle(conn, incoming).await;
        drop(permit);
    });
}

async fn handle<S: AsyncRead + AsyncWrite + Unpin>(mut conn: S, incoming: mpsc::Sender<Incoming>) {
    let Ok(Some(line)) = timeout(limits::READ_TIMEOUT, read_line(&mut conn)).await else {
        return;
    };
    let event = match protocol::decode_event(&line) {
        Ok(event) => event,
        Err(DecodeError::Unsupported { id }) => {
            let reply = Reply::Unsupported {
                v: protocol::VERSION,
                id,
            };
            let _ = conn.write_all(&protocol::encode(&reply)).await;
            return;
        }
        Err(DecodeError::Malformed) => return,
    };

    if !event.wants_reply {
        let _ = incoming.send(Incoming::Event(event)).await;
        return;
    }

    let id = event.id.clone();
    let (tx, mut rx) = mpsc::channel(4);
    if incoming
        .send(Incoming::Request {
            event,
            reply: ReplyHandle(tx),
        })
        .await
        .is_err()
    {
        return;
    }
    // No decision: write nothing, the hook prints nothing, the agent asks in its terminal.
    let v = protocol::VERSION;
    let reply = match wait_for_decision(&mut rx).await {
        Some(Signal::Decide(decision)) => Reply::Decision {
            v,
            id,
            decision,
            reason: None,
        },
        Some(Signal::Answer(answers)) => Reply::Answer { v, id, answers },
        _ => return,
    };
    let _ = conn.write_all(&protocol::encode(&reply)).await;
    let _ = conn.flush().await;
}

/// Two waits: a short one for "the card is up", then the long one for a human. Returns the
/// human's [`Signal::Decide`] or [`Signal::Answer`], if one came.
async fn wait_for_decision(rx: &mut mpsc::Receiver<Signal>) -> Option<Signal> {
    let human = |s: Signal| matches!(s, Signal::Decide(_) | Signal::Answer(_)).then_some(s);
    match timeout(limits::ACK_TIMEOUT, rx.recv()).await {
        Ok(Some(Signal::Ack)) => {}
        // A click that beats the ack is still a click.
        Ok(Some(s)) => return human(s),
        _ => return None,
    }
    match timeout(limits::SERVER_DECISION_TIMEOUT, rx.recv()).await {
        Ok(Some(s)) => human(s),
        _ => None,
    }
}

/// One line without its newline, or `None` past [`protocol::MAX_MESSAGE`] or on EOF without one.
async fn read_line<S: AsyncRead + Unpin>(conn: &mut S) -> Option<Vec<u8>> {
    let mut buf = Vec::new();
    let mut chunk = [0u8; 4096];
    loop {
        if let Some(end) = buf.iter().position(|b| *b == b'\n') {
            buf.truncate(end);
            return Some(buf);
        }
        if buf.len() > protocol::MAX_MESSAGE {
            return None;
        }
        match conn.read(&mut chunk).await {
            Ok(0) | Err(_) => return None,
            Ok(n) => buf.extend_from_slice(&chunk[..n]),
        }
    }
}

#[cfg(all(test, unix))]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tokio::net::UnixStream;

    fn temp_socket(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("vults-ipc-{}-{name}", std::process::id()));
        dir.join("test.sock")
    }

    async fn start(name: &str) -> (PathBuf, mpsc::Receiver<Incoming>) {
        let path = temp_socket(name);
        let (tx, rx) = mpsc::channel(8);
        tokio::spawn(serve(Endpoint::Unix(path.clone()), tx));
        for _ in 0..100 {
            if UnixStream::connect(&path).await.is_ok() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        (path, rx)
    }

    #[tokio::test]
    async fn quit_removes_only_its_own_socket() {
        use std::os::unix::fs::MetadataExt;
        let (path, _rx) = start("quit").await;
        let ours = std::fs::metadata(&path).unwrap();
        // Another instance took the path since: its socket stays.
        std::fs::remove_file(&path).unwrap();
        std::fs::write(&path, b"").unwrap();
        remove_if_same(&path, ours.dev(), ours.ino());
        assert!(path.exists());
        let now = std::fs::metadata(&path).unwrap();
        remove_if_same(&path, now.dev(), now.ino());
        assert!(!path.exists());
    }

    async fn reply_to(path: &PathBuf, line: &[u8]) -> String {
        let mut s = UnixStream::connect(path).await.unwrap();
        s.write_all(line).await.unwrap();
        let mut out = String::new();
        s.read_to_string(&mut out).await.unwrap();
        out
    }

    #[tokio::test]
    async fn unknown_versions_get_unsupported() {
        let (path, _rx) = start("version").await;
        let out = reply_to(&path, b"{\"kind\":\"event\",\"v\":99,\"id\":\"x\"}\n").await;
        assert_eq!(
            out,
            format!(
                "{{\"kind\":\"unsupported\",\"v\":{},\"id\":\"x\"}}\n",
                protocol::VERSION
            )
        );
    }

    fn request_line(event: &str) -> Vec<u8> {
        format!(
            r#"{{"kind":"event","v":{},"id":"r","agent":"claude","event":"{event}","wants_reply":true,"payload":{{}}}}"#,
            protocol::VERSION
        )
        .into_bytes()
    }

    #[tokio::test]
    async fn answers_go_back_in_order() {
        let (path, mut rx) = start("answers").await;
        let line = request_line("PreToolUse");
        let client = tokio::spawn(async move { reply_to(&path, &[&line[..], b"\n"].concat()).await });
        let Some(Incoming::Request { reply, .. }) = rx.recv().await else {
            panic!("expected a request")
        };
        reply.ack();
        reply.answer(vec![Answer::One("Blue".into()), Answer::Many(vec!["S".into()])]);
        assert_eq!(
            client.await.unwrap(),
            format!(
                "{{\"kind\":\"answer\",\"v\":{},\"id\":\"r\",\"answers\":[\"Blue\",[\"S\"]]}}\n",
                protocol::VERSION
            )
        );
    }

    #[tokio::test]
    async fn no_ack_means_no_answer() {
        let (path, mut rx) = start("noack").await;
        let line = request_line("PermissionRequest");
        let start = std::time::Instant::now();
        let client = tokio::spawn(async move { reply_to(&path, &[&line[..], b"\n"].concat()).await });
        let Some(Incoming::Request { reply: _held, .. }) = rx.recv().await else {
            panic!("expected a request")
        };
        // The UI holds the card but never acknowledges: silence after ACK_TIMEOUT.
        assert_eq!(client.await.unwrap(), "");
        assert!(start.elapsed() >= limits::ACK_TIMEOUT);
    }

    #[tokio::test]
    async fn a_dropped_handle_means_no_answer_at_once() {
        let (path, mut rx) = start("dropped").await;
        let line = request_line("PermissionRequest");
        let start = std::time::Instant::now();
        let client = tokio::spawn(async move { reply_to(&path, &[&line[..], b"\n"].concat()).await });
        drop(rx.recv().await);
        assert_eq!(client.await.unwrap(), "");
        assert!(start.elapsed() < limits::ACK_TIMEOUT);
    }

    #[tokio::test]
    async fn socket_is_private() {
        use std::os::unix::fs::PermissionsExt;
        let (path, _rx) = start("perms").await;
        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }
}
