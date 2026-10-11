//! The client both ends use to reach a relay. The desktop's requests are made on the user's behalf
//! (ADR 0019), so each one asks the app's network gate first: a closed gate means no request.

use std::sync::Arc;
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

pub struct Client {
    http: reqwest::Client,
    base: String,
    gate: Arc<dyn Gate>,
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
            .build()
            .map_err(|_| Error::Relay(None))?;
        Ok(Self {
            http,
            base: base.trim_end_matches('/').to_string(),
            gate,
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

    /// What waits in `box_id`, waiting up to `wait` for something to come.
    pub async fn receive(&self, box_id: &str, wait: Duration) -> Result<Vec<String>, Error> {
        let url = format!("{}/v1/boxes/{box_id}?wait={}", self.base, wait.as_secs());
        let request = self.http.get(url).timeout(wait + Duration::from_secs(15));
        let response = self.gated(request).await?;
        response.json().await.map_err(|_| Error::Relay(None))
    }

    async fn gated(&self, request: reqwest::RequestBuilder) -> Result<reqwest::Response, Error> {
        if !self.gate.allow() {
            return Err(Error::Closed);
        }
        let started = Instant::now();
        let result = match request.send().await {
            Ok(r) if r.status().is_success() => Ok(r),
            Ok(r) => Err(Error::Relay(Some(r.status().as_u16()))),
            Err(_) => Err(Error::Relay(None)),
        };
        self.gate.done(result.is_ok(), started.elapsed());
        result
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
