//! The relay itself: boxes of sealed envelopes, each addressed by 32 bytes only the desktop and the
//! phone can derive (`vults_link::envelope::Link::outbox`). It stores what is posted and hands it,
//! once, to whoever reads the box; it cannot open, check or link one message to another. A message
//! leaves after its time even unread, and every box and message is capped.

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use axum::extract::{DefaultBodyLimit, Path, Query, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;
use tokio::sync::Notify;

#[derive(Debug, Clone, Copy)]
pub struct Limits {
    /// A message unread this long is dropped.
    pub ttl: Duration,
    /// Messages one box holds before it refuses more.
    pub per_box: usize,
    /// Boxes the relay holds at once.
    pub boxes: usize,
    /// Bytes in one message.
    pub message: usize,
    /// The longest a read waits for a message.
    pub wait: Duration,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            ttl: Duration::from_secs(600),
            per_box: 64,
            boxes: 10_000,
            message: vults_link::MAX_MESSAGE,
            wait: Duration::from_secs(25),
        }
    }
}

#[derive(Debug)]
struct Mailbox {
    queue: VecDeque<(Instant, String)>,
    notify: Arc<Notify>,
    touched: Instant,
}

#[derive(Debug)]
pub struct Relay {
    boxes: Mutex<HashMap<String, Mailbox>>,
    limits: Limits,
}

impl Relay {
    pub fn new(limits: Limits) -> Arc<Self> {
        Arc::new(Self {
            boxes: Mutex::new(HashMap::new()),
            limits,
        })
    }

    /// Drops what is past its time, and boxes left empty that long.
    pub fn sweep(&self, now: Instant) {
        let ttl = self.limits.ttl;
        if let Ok(mut boxes) = self.boxes.lock() {
            for b in boxes.values_mut() {
                b.queue.retain(|(at, _)| now.duration_since(*at) < ttl);
            }
            boxes.retain(|_, b| !b.queue.is_empty() || now.duration_since(b.touched) < ttl);
        }
    }

    fn post(&self, id: String, message: String, now: Instant) -> StatusCode {
        let Ok(mut boxes) = self.boxes.lock() else {
            return StatusCode::INTERNAL_SERVER_ERROR;
        };
        if !boxes.contains_key(&id) && boxes.len() >= self.limits.boxes {
            return StatusCode::SERVICE_UNAVAILABLE;
        }
        let b = boxes.entry(id).or_insert_with(|| Mailbox {
            queue: VecDeque::new(),
            notify: Arc::new(Notify::new()),
            touched: now,
        });
        b.queue
            .retain(|(at, _)| now.duration_since(*at) < self.limits.ttl);
        if b.queue.len() >= self.limits.per_box {
            return StatusCode::TOO_MANY_REQUESTS;
        }
        b.queue.push_back((now, message));
        b.touched = now;
        b.notify.notify_waiters();
        StatusCode::ACCEPTED
    }

    /// What waits in the box, taken out; and the box's bell, to wait on when there is nothing.
    fn take(&self, id: &str, now: Instant) -> (Vec<String>, Arc<Notify>) {
        let Ok(mut boxes) = self.boxes.lock() else {
            return (Vec::new(), Arc::new(Notify::new()));
        };
        if !boxes.contains_key(id) && boxes.len() >= self.limits.boxes {
            return (Vec::new(), Arc::new(Notify::new()));
        }
        let b = boxes.entry(id.to_string()).or_insert_with(|| Mailbox {
            queue: VecDeque::new(),
            notify: Arc::new(Notify::new()),
            touched: now,
        });
        b.touched = now;
        let ttl = self.limits.ttl;
        let out = b
            .queue
            .drain(..)
            .filter(|(at, _)| now.duration_since(*at) < ttl)
            .map(|(_, m)| m)
            .collect();
        (out, b.notify.clone())
    }
}

/// A box's address: 64 lowercase hex characters, nothing else.
fn valid(id: &str) -> bool {
    id.len() == 64 && id.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

#[derive(Deserialize)]
struct Wait {
    wait: Option<u64>,
}

async fn post(State(relay): State<Arc<Relay>>, Path(id): Path<String>, body: String) -> StatusCode {
    if !valid(&id) {
        return StatusCode::NOT_FOUND;
    }
    relay.post(id, body, Instant::now())
}

async fn read(
    State(relay): State<Arc<Relay>>,
    Path(id): Path<String>,
    Query(q): Query<Wait>,
) -> Result<Json<Vec<String>>, StatusCode> {
    if !valid(&id) {
        return Err(StatusCode::NOT_FOUND);
    }
    let wait = Duration::from_secs(q.wait.unwrap_or(0)).min(relay.limits.wait);
    let (out, bell) = relay.take(&id, Instant::now());
    if !out.is_empty() || wait.is_zero() {
        return Ok(Json(out));
    }
    // Rung after the take above would be missed: arm first, then look again.
    let rung = bell.notified();
    tokio::pin!(rung);
    rung.as_mut().enable();
    let (out, _) = relay.take(&id, Instant::now());
    if !out.is_empty() {
        return Ok(Json(out));
    }
    let _ = tokio::time::timeout(wait, rung).await;
    Ok(Json(relay.take(&id, Instant::now()).0))
}

/// The relay's routes: `POST /v1/boxes/{id}` stores a message, `GET /v1/boxes/{id}?wait=s` takes
/// what waits (holding the request up to `wait` seconds when there is nothing), `GET /health`.
pub fn router(relay: Arc<Relay>) -> Router {
    let limit = relay.limits.message;
    Router::new()
        .route("/v1/boxes/{id}", get(read).post(post))
        .route("/health", get(|| async { "ok" }))
        .layer(DefaultBodyLimit::max(limit))
        .with_state(relay)
}

/// Serves the relay on `listener` until the process ends, sweeping every 30 seconds.
pub async fn serve(listener: tokio::net::TcpListener, limits: Limits) -> std::io::Result<()> {
    let relay = Relay::new(limits);
    let sweeper = relay.clone();
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(Duration::from_secs(30));
        loop {
            tick.tick().await;
            sweeper.sweep(Instant::now());
        }
    });
    axum::serve(listener, router(relay)).await
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    #[test]
    fn a_message_is_taken_once() {
        let r = Relay::new(Limits::default());
        let now = Instant::now();
        assert_eq!(r.post(A.into(), "m1".into(), now), StatusCode::ACCEPTED);
        assert_eq!(r.post(A.into(), "m2".into(), now), StatusCode::ACCEPTED);
        assert_eq!(r.take(A, now).0, vec!["m1", "m2"]);
        assert!(r.take(A, now).0.is_empty());
    }

    #[test]
    fn a_message_past_its_time_is_gone() {
        let limits = Limits::default();
        let r = Relay::new(limits);
        let now = Instant::now();
        r.post(A.into(), "old".into(), now);
        assert!(r.take(A, now + limits.ttl).0.is_empty());
        r.post(A.into(), "old".into(), now);
        r.sweep(now + limits.ttl);
        assert!(r.boxes.lock().map(|b| b.is_empty()).unwrap_or(false));
    }

    #[test]
    fn boxes_and_messages_are_capped() {
        let r = Relay::new(Limits {
            per_box: 2,
            boxes: 1,
            ..Limits::default()
        });
        let now = Instant::now();
        r.post(A.into(), "1".into(), now);
        r.post(A.into(), "2".into(), now);
        assert_eq!(r.post(A.into(), "3".into(), now), StatusCode::TOO_MANY_REQUESTS);
        let b = "b".repeat(64);
        assert_eq!(
            r.post(b.clone(), "x".into(), now),
            StatusCode::SERVICE_UNAVAILABLE
        );
        assert!(r.take(&b, now).0.is_empty());
    }

    #[test]
    fn only_a_box_address_is_one() {
        assert!(valid(A));
        assert!(!valid(&A.to_uppercase()));
        assert!(!valid(&A[1..]));
        assert!(!valid("../etc/passwd"));
    }
}
