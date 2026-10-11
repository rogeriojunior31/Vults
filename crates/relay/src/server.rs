//! The relay itself: boxes of sealed envelopes, each addressed by 32 bytes only the desktop and the
//! phone can derive (`vults_link::envelope::Link::outbox`). It keeps what is posted until the reader
//! says it has it (the next read's `after`), and cannot open, check or link one message to another.
//! A message leaves after its time even unread; boxes, messages, bytes and connections are capped,
//! and a read never makes a box, so reading cannot fill the relay.

use std::collections::{HashMap, VecDeque};
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use axum::extract::{DefaultBodyLimit, Path, Query, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{Notify, OwnedSemaphorePermit, Semaphore};
use tokio::time::Sleep;

#[derive(Debug, Clone, Copy)]
pub struct Limits {
    /// A message unread this long is dropped.
    pub ttl: Duration,
    /// Messages one box holds before it refuses more.
    pub per_box: usize,
    /// Boxes the relay holds at once.
    pub boxes: usize,
    /// Bytes of messages the relay holds at once.
    pub bytes: usize,
    /// Bytes in one message.
    pub message: usize,
    /// The longest a read waits for a message.
    pub wait: Duration,
    /// Connections open at once; one more waits to be accepted.
    pub connections: usize,
    /// A connection that sends nothing this long is closed.
    pub idle: Duration,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            ttl: Duration::from_secs(600),
            per_box: 64,
            boxes: 10_000,
            bytes: 256 * 1024 * 1024,
            message: vults_link::MAX_MESSAGE,
            wait: Duration::from_secs(25),
            connections: 1000,
            idle: Duration::from_secs(60),
        }
    }
}

/// One message as a read hands it out: `seq` is what the next read's `after` acknowledges.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Message {
    pub seq: u64,
    pub msg: String,
}

#[derive(Debug)]
struct Kept {
    seq: u64,
    at: Instant,
    msg: String,
}

#[derive(Debug, Default)]
struct Inner {
    /// Only boxes holding a message: an empty one is gone.
    boxes: HashMap<String, VecDeque<Kept>>,
    /// The bell of each box someone waits on, rung by a post. Each waiter holds a connection, so
    /// the connection cap bounds them.
    bells: HashMap<String, Arc<Notify>>,
    bytes: usize,
    seq: u64,
}

impl Inner {
    /// Drops the messages `gone` picks from a box, and the box when it is left empty.
    fn drop_where(&mut self, id: &str, gone: impl Fn(&Kept) -> bool) {
        let Some(q) = self.boxes.get_mut(id) else { return };
        let before: usize = q.iter().map(|k| k.msg.len()).sum();
        q.retain(|k| !gone(k));
        let after: usize = q.iter().map(|k| k.msg.len()).sum();
        self.bytes -= before - after;
        if q.is_empty() {
            self.boxes.remove(id);
        }
    }
}

#[derive(Debug)]
pub struct Relay {
    inner: Mutex<Inner>,
    limits: Limits,
}

impl Relay {
    pub fn new(limits: Limits) -> Arc<Self> {
        // A sequence from the clock: a reader's cursor from before a restart is lower than any
        // new message, so a restart never makes it skip one.
        let start = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(0))
            .saturating_mul(1000);
        Arc::new(Self {
            inner: Mutex::new(Inner {
                seq: start,
                ..Inner::default()
            }),
            // A wait as long as a message's life could outlive what it waits for.
            limits: Limits {
                wait: limits.wait.min(limits.ttl / 2),
                ..limits
            },
        })
    }

    /// Drops every message past its time.
    pub fn sweep(&self, now: Instant) {
        let ttl = self.limits.ttl;
        if let Ok(mut inner) = self.inner.lock() {
            let ids: Vec<String> = inner.boxes.keys().cloned().collect();
            for id in ids {
                inner.drop_where(&id, |k| now.duration_since(k.at) >= ttl);
            }
        }
    }

    fn post(&self, id: &str, msg: String, now: Instant) -> StatusCode {
        let Ok(mut inner) = self.inner.lock() else {
            return StatusCode::INTERNAL_SERVER_ERROR;
        };
        let ttl = self.limits.ttl;
        inner.drop_where(id, |k| now.duration_since(k.at) >= ttl);
        let held = inner.boxes.get(id).map_or(0, VecDeque::len);
        if held == 0 && inner.boxes.len() >= self.limits.boxes {
            return StatusCode::SERVICE_UNAVAILABLE;
        }
        if inner.bytes + msg.len() > self.limits.bytes {
            return StatusCode::SERVICE_UNAVAILABLE;
        }
        if held >= self.limits.per_box {
            return StatusCode::TOO_MANY_REQUESTS;
        }
        inner.seq += 1;
        inner.bytes += msg.len();
        let kept = Kept {
            seq: inner.seq,
            at: now,
            msg,
        };
        inner.boxes.entry(id.to_string()).or_default().push_back(kept);
        if let Some(bell) = inner.bells.get(id) {
            bell.notify_waiters();
        }
        StatusCode::ACCEPTED
    }

    /// Drops what the reader acknowledged (`after`) and what is past its time, and hands out the
    /// rest, which stays until a later read acknowledges it.
    fn read(&self, id: &str, after: u64, now: Instant) -> Vec<Message> {
        let Ok(mut inner) = self.inner.lock() else {
            return Vec::new();
        };
        let ttl = self.limits.ttl;
        inner.drop_where(id, |k| k.seq <= after || now.duration_since(k.at) >= ttl);
        inner.boxes.get(id).map_or_else(Vec::new, |q| {
            q.iter()
                .map(|k| Message {
                    seq: k.seq,
                    msg: k.msg.clone(),
                })
                .collect()
        })
    }

    fn bell(&self, id: &str) -> Arc<Notify> {
        let Ok(mut inner) = self.inner.lock() else {
            return Arc::new(Notify::new());
        };
        inner.bells.entry(id.to_string()).or_default().clone()
    }

    /// A waiter is done with `bell`: the last one out takes it away.
    fn unring(&self, id: &str, bell: Arc<Notify>) {
        drop(bell);
        if let Ok(mut inner) = self.inner.lock()
            && inner.bells.get(id).is_some_and(|b| Arc::strong_count(b) == 1)
        {
            inner.bells.remove(id);
        }
    }

    #[cfg(test)]
    fn boxes(&self) -> usize {
        self.inner.lock().map_or(0, |i| i.boxes.len())
    }
}

/// A box's address: 64 lowercase hex characters, nothing else.
fn valid(id: &str) -> bool {
    id.len() == 64 && id.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

#[derive(Deserialize)]
struct Read {
    wait: Option<u64>,
    after: Option<u64>,
}

async fn post(State(relay): State<Arc<Relay>>, Path(id): Path<String>, body: String) -> StatusCode {
    if !valid(&id) {
        return StatusCode::NOT_FOUND;
    }
    relay.post(&id, body, Instant::now())
}

async fn read(
    State(relay): State<Arc<Relay>>,
    Path(id): Path<String>,
    Query(q): Query<Read>,
) -> Result<Json<Vec<Message>>, StatusCode> {
    if !valid(&id) {
        return Err(StatusCode::NOT_FOUND);
    }
    let after = q.after.unwrap_or(0);
    let wait = Duration::from_secs(q.wait.unwrap_or(0)).min(relay.limits.wait);
    let out = relay.read(&id, after, Instant::now());
    if !out.is_empty() || wait.is_zero() {
        return Ok(Json(out));
    }
    let bell = relay.bell(&id);
    let out = {
        // A post between the read above and the wait below would be missed: arm first, look again.
        let rung = bell.notified();
        tokio::pin!(rung);
        rung.as_mut().enable();
        let out = relay.read(&id, after, Instant::now());
        if out.is_empty() {
            let _ = tokio::time::timeout(wait, rung).await;
            relay.read(&id, after, Instant::now())
        } else {
            out
        }
    };
    relay.unring(&id, bell);
    Ok(Json(out))
}

/// The relay's routes: `POST /v1/boxes/{id}` keeps a message; `GET /v1/boxes/{id}?after=seq&wait=s`
/// drops what `after` acknowledges and hands out the rest, holding the request up to `wait`
/// seconds when there is nothing; `GET /health`.
pub fn router(relay: Arc<Relay>) -> Router {
    let limit = relay.limits.message;
    // A request (a slow body included) gets the longest wait and some time to travel.
    let timeout = relay.limits.wait + Duration::from_secs(10);
    Router::new()
        .route("/v1/boxes/{id}", get(read).post(post))
        .route("/health", get(|| async { "ok" }))
        .layer(DefaultBodyLimit::max(limit))
        .layer(tower_http::timeout::TimeoutLayer::with_status_code(
            StatusCode::REQUEST_TIMEOUT,
            timeout,
        ))
        .with_state(relay)
}

/// Serves the relay on `listener` until the process ends, sweeping every 30 seconds.
pub async fn serve(listener: TcpListener, limits: Limits) -> std::io::Result<()> {
    let relay = Relay::new(limits);
    let sweeper = relay.clone();
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(Duration::from_secs(30));
        loop {
            tick.tick().await;
            sweeper.sweep(Instant::now());
        }
    });
    let capped = Capped {
        listener,
        open: Arc::new(Semaphore::new(limits.connections)),
        idle: limits.idle,
    };
    axum::serve(capped, router(relay)).await
}

/// A listener that holds at most so many connections and closes one that goes quiet.
struct Capped {
    listener: TcpListener,
    open: Arc<Semaphore>,
    idle: Duration,
}

impl axum::serve::Listener for Capped {
    type Io = Quiet;
    type Addr = std::net::SocketAddr;

    async fn accept(&mut self) -> (Self::Io, Self::Addr) {
        loop {
            // Wait for room before taking the next one: past the cap, the kernel's backlog holds it.
            let Ok(permit) = self.open.clone().acquire_owned().await else {
                continue;
            };
            match self.listener.accept().await {
                Ok((io, addr)) => {
                    let quiet = Quiet {
                        io,
                        _permit: permit,
                        idle: self.idle,
                        timer: Box::pin(tokio::time::sleep(self.idle)),
                    };
                    return (quiet, addr);
                }
                // A failed accept (out of descriptors, a reset) is retried after a breath.
                Err(_) => tokio::time::sleep(Duration::from_millis(50)).await,
            }
        }
    }

    fn local_addr(&self) -> std::io::Result<Self::Addr> {
        self.listener.local_addr()
    }
}

/// A connection that ends when it has sent nothing for `idle`, freeing its place.
struct Quiet {
    io: TcpStream,
    _permit: OwnedSemaphorePermit,
    idle: Duration,
    timer: Pin<Box<Sleep>>,
}

impl AsyncRead for Quiet {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        let this = &mut *self;
        match Pin::new(&mut this.io).poll_read(cx, buf) {
            Poll::Ready(r) => {
                let deadline = tokio::time::Instant::now() + this.idle;
                this.timer.as_mut().reset(deadline);
                Poll::Ready(r)
            }
            Poll::Pending => match this.timer.as_mut().poll(cx) {
                Poll::Ready(()) => Poll::Ready(Err(std::io::ErrorKind::TimedOut.into())),
                Poll::Pending => Poll::Pending,
            },
        }
    }
}

impl AsyncWrite for Quiet {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        Pin::new(&mut self.io).poll_write(cx, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.io).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.io).poll_shutdown(cx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    fn texts(m: &[Message]) -> Vec<&str> {
        m.iter().map(|m| m.msg.as_str()).collect()
    }

    #[test]
    fn a_message_stays_until_acknowledged() {
        let r = Relay::new(Limits::default());
        let now = Instant::now();
        assert_eq!(r.post(A, "m1".into(), now), StatusCode::ACCEPTED);
        assert_eq!(r.post(A, "m2".into(), now), StatusCode::ACCEPTED);
        let first = r.read(A, 0, now);
        assert_eq!(texts(&first), vec!["m1", "m2"]);
        // A reader whose answer never arrived reads the same again.
        assert_eq!(r.read(A, 0, now), first);
        // Acknowledging the first leaves the second; acknowledging both empties the box.
        assert_eq!(texts(&r.read(A, first[0].seq, now)), vec!["m2"]);
        assert!(r.read(A, first[1].seq, now).is_empty());
        assert_eq!(r.boxes(), 0);
    }

    #[test]
    fn a_message_past_its_time_is_gone() {
        let limits = Limits::default();
        let r = Relay::new(limits);
        let now = Instant::now();
        r.post(A, "old".into(), now);
        assert!(r.read(A, 0, now + limits.ttl).is_empty());
        r.post(A, "old".into(), now);
        r.sweep(now + limits.ttl);
        assert_eq!(r.boxes(), 0);
        assert_eq!(r.inner.lock().map(|i| i.bytes).unwrap_or(1), 0);
    }

    #[test]
    fn boxes_messages_and_bytes_are_capped() {
        let r = Relay::new(Limits {
            per_box: 2,
            boxes: 1,
            bytes: 10,
            ..Limits::default()
        });
        let now = Instant::now();
        r.post(A, "1".into(), now);
        r.post(A, "2".into(), now);
        assert_eq!(r.post(A, "3".into(), now), StatusCode::TOO_MANY_REQUESTS);
        let b = "b".repeat(64);
        assert_eq!(r.post(&b, "x".into(), now), StatusCode::SERVICE_UNAVAILABLE);
        let seq = r.read(A, 0, now)[1].seq;
        r.read(A, seq, now);
        assert_eq!(r.post(&b, "x".repeat(11), now), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(r.post(&b, "x".repeat(10), now), StatusCode::ACCEPTED);
    }

    #[test]
    fn reading_makes_no_box() {
        let r = Relay::new(Limits {
            boxes: 1,
            ..Limits::default()
        });
        let now = Instant::now();
        for i in 0..100 {
            assert!(r.read(&format!("{i:064x}"), 0, now).is_empty());
        }
        assert_eq!(r.boxes(), 0);
        assert_eq!(r.post(A, "still room".into(), now), StatusCode::ACCEPTED);
    }

    #[test]
    fn a_new_message_comes_after_any_old_cursor() {
        let now = Instant::now();
        let before = Relay::new(Limits::default());
        before.post(A, "x".into(), now);
        let old = before.read(A, 0, now)[0].seq;
        std::thread::sleep(Duration::from_millis(2));
        // A restart: the new relay's messages still sort after the old cursor.
        let after = Relay::new(Limits::default());
        after.post(A, "y".into(), now);
        assert_eq!(texts(&after.read(A, old, now)), vec!["y"]);
    }

    #[test]
    fn only_a_box_address_is_one() {
        assert!(valid(A));
        assert!(!valid(&A.to_uppercase()));
        assert!(!valid(&A[1..]));
        assert!(!valid("../etc/passwd"));
    }
}
