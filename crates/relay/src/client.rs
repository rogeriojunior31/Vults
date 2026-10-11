//! The client both ends use to reach a relay. The desktop's requests are made on the user's behalf
//! (ADR 0019), so each one asks the app's network gate first: a closed gate means no request.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// The app's network gate, as the relay client sees it (the app implements it, `app/src/gate.rs`).
pub trait Gate: Send + Sync {
    /// May the relay make a request now?
    fn allow(&self) -> bool;
    /// The request it let through ended (`ok`: the relay answered), after `took`.
    fn done(&self, ok: bool, took: Duration);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// The gate said no: the switch is off or the day's budget is spent.
    Closed,
    /// The relay could not be reached, or answered with this status.
    Relay(Option<u16>),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Closed => f.write_str("the network gate is closed"),
            Self::Relay(Some(s)) => write!(f, "the relay answered {s}"),
            Self::Relay(None) => f.write_str("the relay could not be reached"),
        }
    }
}

impl std::error::Error for Error {}

/// The most a read's answer may hold: a full box, each message's JSON escaping at its worst.
const MAX_ANSWER: usize = 64 * 2 * vults_link::MAX_MESSAGE + 64 * 1024;

pub struct Client {
    http: reqwest::Client,
    base: String,
    gate: Arc<dyn Gate>,
    /// Per box, the last message handed to the caller: the next read acknowledges it, and only then
    /// does the relay let it go.
    cursors: Mutex<HashMap<String, u64>>,
}

impl std::fmt::Debug for Client {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Client")
            .field("base", &self.base)
            .finish_non_exhaustive()
    }
}

impl Client {
    /// A client of the relay at `base` (`https://relay.example`, or the desktop's own address on the
    /// LAN or a tailnet).
    pub fn new(base: &str, gate: Arc<dyn Gate>) -> Result<Self, Error> {
        let http = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(10))
            // One request the gate allowed is one request: never another, to another host.
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|_| Error::Relay(None))?;
        Ok(Self {
            http,
            base: base.trim_end_matches('/').to_string(),
            gate,
            cursors: Mutex::new(HashMap::new()),
        })
    }

    /// Posts one sealed envelope to `box_id`.
    pub async fn send(&self, box_id: &str, envelope: &str) -> Result<(), Error> {
        let url = format!("{}/v1/boxes/{box_id}", self.base);
        let request = self
            .http
            .post(url)
            .timeout(Duration::from_secs(15))
            .body(envelope.to_string());
        self.gated(request).await.map(|_| ())
    }

    /// What came to `box_id` since the last read, waiting up to `wait` for something. What the last
    /// read handed out is acknowledged by this one: a caller that read it has it.
    pub async fn receive(&self, box_id: &str, wait: Duration) -> Result<Vec<String>, Error> {
        let after = self.cursor(box_id);
        let url = format!(
            "{}/v1/boxes/{box_id}?after={after}&wait={}",
            self.base,
            wait.as_secs()
        );
        let request = self.http.get(url).timeout(wait + Duration::from_secs(15));
        let mut response = self.gated(request).await?;
        // The relay is not trusted with the size of its answer.
        let mut body = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(|_| Error::Relay(None))? {
            if body.len() + chunk.len() > MAX_ANSWER {
                return Err(Error::Relay(None));
            }
            body.extend_from_slice(&chunk);
        }
        let messages: Vec<crate::server::Message> =
            serde_json::from_slice(&body).map_err(|_| Error::Relay(None))?;
        // Only new ones: a relay that sends an old one again is not believed.
        let fresh: Vec<_> = messages.into_iter().filter(|m| m.seq > after).collect();
        if let Some(last) = fresh.iter().map(|m| m.seq).max()
            && let Ok(mut cursors) = self.cursors.lock()
        {
            cursors.insert(box_id.to_string(), last);
        }
        Ok(fresh.into_iter().map(|m| m.msg).collect())
    }

    fn cursor(&self, box_id: &str) -> u64 {
        self.cursors
            .lock()
            .ok()
            .and_then(|c| c.get(box_id).copied())
            .unwrap_or(0)
    }

    async fn gated(&self, request: reqwest::RequestBuilder) -> Result<reqwest::Response, Error> {
        if !self.gate.allow() {
            return Err(Error::Closed);
        }
        // Reported when the request ends, even one dropped midway (a long poll at shutdown).
        let mut call = Call {
            gate: &*self.gate,
            started: Instant::now(),
            ok: false,
        };
        let result = match request.send().await {
            Ok(r) if r.status().is_success() => Ok(r),
            Ok(r) => Err(Error::Relay(Some(r.status().as_u16()))),
            Err(_) => Err(Error::Relay(None)),
        };
        call.ok = result.is_ok();
        result
    }
}

struct Call<'a> {
    gate: &'a dyn Gate,
    started: Instant,
    ok: bool,
}

impl Drop for Call<'_> {
    fn drop(&mut self) {
        self.gate.done(self.ok, self.started.elapsed());
    }
}

/// A gate that is always open: for the command-line test client and tests, never the app.
#[derive(Debug)]
pub struct Open;

impl Gate for Open {
    fn allow(&self) -> bool {
        true
    }
    fn done(&self, _: bool, _: Duration) {}
}
