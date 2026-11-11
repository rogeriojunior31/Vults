//! Read-only Gitea API access, with an atomic keyring host/token binding.
use crate::{Connector, Error, Event, Gate, Poll, Row, Snapshot};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    sync::{
        Arc, RwLock,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use vults_core::SafeUrl;

mod dashboard;
#[cfg(test)]
use dashboard::insert;
use dashboard::{board, diff, read};

#[derive(Debug)]
pub struct Gitea;
pub const INVALID: &str = "Gitea returned incomplete or invalid data. The previous snapshot is kept.";
pub const FAILED: &str = "Could not read Gitea. Check the connection and try again.";
pub const AUTH: &str = "Check the Gitea URL and token in Settings.";
const MAX_BYTES: usize = 1024 * 1024;
const MAX_PAGES: usize = 5;
static EPOCH: AtomicU64 = AtomicU64::new(0);
static META: RwLock<Option<Connection>> = RwLock::new(None);

/// Metadata only; the token never crosses the native boundary.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Connection {
    pub base_url: String,
    pub generation: String,
}
#[derive(Serialize, Deserialize)]
struct Credential {
    connection: Connection,
    token: String,
    #[cfg(test)]
    #[serde(skip)]
    fixture_base: Option<String>,
}
impl Credential {
    fn new(connection: Connection, token: String) -> Self {
        Self {
            connection,
            token,
            #[cfg(test)]
            fixture_base: None,
        }
    }
    fn api_base(&self) -> &str {
        #[cfg(test)]
        if let Some(base) = &self.fixture_base {
            return base;
        }
        &self.connection.base_url
    }
}
fn credential() -> Result<Option<Credential>, String> {
    let Some(raw) = vults_secrets::get(vults_secrets::Secret::Gitea)
        .map_err(|_| "Could not read the Gitea connection.")?
    else {
        return Ok(None);
    };
    let value: Credential = serde_json::from_str(&raw).map_err(|_| "Could not read the Gitea connection.")?;
    if SafeUrl::service_base(&value.connection.base_url).as_deref() != Some(&value.connection.base_url)
        || !valid_token(&value.token)
    {
        return Err("Could not read the Gitea connection.".into());
    }
    Ok(Some(value))
}
fn valid_token(token: &str) -> bool {
    !token.is_empty() && token.len() <= 4096 && token.bytes().all(|c| c.is_ascii_graphic())
}
pub fn connection() -> Result<Connection, String> {
    let connection = credential()?.map(|c| c.connection).unwrap_or_default();
    *META.write().map_err(|_| "Could not read the Gitea connection.")? =
        (!connection.base_url.is_empty()).then_some(connection.clone());
    Ok(connection)
}
/// Called only while disabled, under the app's configuration mutex. A blank token keeps the
/// old one only for the exact same origin and subfolder.
pub fn save(base: &str, token: &str, expected: &str) -> Result<Connection, String> {
    let (base, token) = binding(credential()?, base, token, expected)?;
    persist(base, token)
}
fn binding(
    previous: Option<Credential>,
    base: &str,
    token: &str,
    expected: &str,
) -> Result<(String, String), String> {
    if previous
        .as_ref()
        .map(|c| c.connection.generation.as_str())
        .unwrap_or_default()
        != expected
    {
        return Err("The Gitea connection changed. Reload Settings and try again.".into());
    }
    let base = SafeUrl::service_base(base)
        .ok_or("Enter a valid HTTPS Gitea base URL without credentials, query or fragment.")?;
    let token = if token.is_empty() {
        previous
            .filter(|c| c.connection.base_url == base)
            .map(|c| c.token)
            .ok_or("Enter a Gitea access token for this URL.")?
    } else {
        if !valid_token(token) {
            return Err("Enter a Gitea access token for this URL.".into());
        }
        token.into()
    };
    Ok((base, token))
}
fn persist(base: String, token: String) -> Result<Connection, String> {
    EPOCH.fetch_add(1, Ordering::SeqCst);
    let generation = format!(
        "{:x}-{:x}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos(),
        EPOCH.load(Ordering::SeqCst)
    );
    let connection = Connection {
        base_url: base,
        generation,
    };
    let value = serde_json::to_string(&Credential::new(connection.clone(), token))
        .map_err(|_| "Could not save the Gitea connection.")?;
    vults_secrets::set(vults_secrets::Secret::Gitea, &value)
        .map_err(|_| "Could not save the Gitea connection.")?;
    *META.write().map_err(|_| "Could not save the Gitea connection.")? = Some(connection.clone());
    Ok(connection)
}
pub fn clear(expected: &str) -> Result<Connection, String> {
    let current = credential()?;
    if current
        .as_ref()
        .map(|c| c.connection.generation.as_str())
        .unwrap_or_default()
        != expected
    {
        return Err("The Gitea connection changed. Reload Settings and try again.".into());
    }
    EPOCH.fetch_add(1, Ordering::SeqCst);
    vults_secrets::delete(vults_secrets::Secret::Gitea)
        .map_err(|_| "Could not save the Gitea connection.")?;
    *META.write().map_err(|_| "Could not save the Gitea connection.")? = None;
    Ok(Connection::default())
}
/// In-memory metadata only: the runtime's owner loop must never block on the OS keyring.
pub fn trusted_link(item: &str, raw: &str) -> Option<SafeUrl> {
    let meta = META.read().ok()?;
    let meta = meta.as_ref()?;
    item.starts_with(&format!("gitea:{}:", meta.generation))
        .then_some(())?;
    SafeUrl::for_service(raw, &meta.base_url)
}
impl Connector for Gitea {
    fn id(&self) -> &'static str {
        "gitea"
    }
    fn interval(&self, snapshot: &Snapshot) -> Duration {
        let floor = if snapshot.values().any(|v| v["ci"] == "running") {
            60
        } else {
            300
        };
        Duration::from_secs(
            floor.max(
                snapshot
                    .get("meta:poll")
                    .and_then(|v| v["requests"].as_u64())
                    .unwrap_or(0)
                    .saturating_mul(36),
            ),
        )
    }
    fn poll(&self) -> Poll<'_> {
        Box::pin(async { Err(Error::Budget) })
    }
    fn poll_admitted(&self, gate: Arc<dyn Gate>) -> Poll<'_> {
        Box::pin(async move {
            let epoch = EPOCH.load(Ordering::SeqCst);
            let c = tokio::task::spawn_blocking(credential)
                .await
                .map_err(|_| Error::Auth(AUTH.into()))?
                .map_err(|_| Error::Auth(AUTH.into()))?
                .ok_or_else(|| Error::Auth(AUTH.into()))?;
            {
                let mut meta = META.write().map_err(|_| Error::Other(FAILED.into()))?;
                if EPOCH.load(Ordering::SeqCst) != epoch {
                    return Err(Error::Budget);
                }
                *meta = Some(c.connection.clone());
            }
            let client = reqwest::Client::builder()
                .no_proxy()
                .redirect(reqwest::redirect::Policy::none())
                .timeout(Duration::from_secs(15))
                .build()
                .map_err(|_| Error::Other(FAILED.into()))?;
            let counted = Counted {
                gate: &*gate,
                requests: std::sync::atomic::AtomicUsize::new(0),
            };
            let result = tokio::time::timeout(Duration::from_secs(60), read(&client, &c, &counted, epoch))
                .await
                .map_err(|_| Error::Other(FAILED.into()))?;
            result.map(|mut snapshot| {
                snapshot.insert(
                    "meta:poll".into(),
                    json!({"requests":counted.requests.load(Ordering::SeqCst)}),
                );
                snapshot
            })
        })
    }
    fn board(&self, s: &Snapshot) -> Option<Vec<Row>> {
        Some(board(s))
    }
    fn identity(&self, s: &Snapshot) -> Option<(String, String)> {
        Some((
            s.get("meta")?["login"].as_str()?.into(),
            s.get("meta")?["base"].as_str()?.into(),
        ))
    }
    fn watching(&self, s: &Snapshot) -> usize {
        s.values()
            .filter_map(|v| {
                let number = v["number"].as_u64();
                Some((
                    v["repo"].as_str()?,
                    number,
                    if number.is_some() {
                        "pr"
                    } else {
                        v["group"].as_str()?
                    },
                ))
            })
            .collect::<std::collections::BTreeSet<_>>()
            .len()
    }
    fn diff(&self, before: &Snapshot, after: &Snapshot) -> Vec<Event> {
        if before.get("meta") != after.get("meta") {
            return Vec::new();
        }
        diff(before, after)
    }
}
struct Counted<'a> {
    gate: &'a dyn Gate,
    requests: std::sync::atomic::AtomicUsize,
}
impl Gate for Counted<'_> {
    fn allow(&self, id: &str) -> bool {
        if !self.gate.allow(id) {
            return false;
        }
        self.requests.fetch_add(1, Ordering::SeqCst);
        true
    }
    fn done(&self, id: &str, ok: bool, took: Duration) {
        self.gate.done(id, ok, took);
    }
}
fn invalid() -> Error {
    Error::Partial(INVALID.into())
}
struct Request<'a> {
    gate: &'a dyn Gate,
    started: Instant,
    ok: bool,
}
impl Drop for Request<'_> {
    fn drop(&mut self) {
        self.gate.done("gitea", self.ok, self.started.elapsed());
    }
}
async fn get(
    client: &reqwest::Client,
    c: &Credential,
    gate: &dyn Gate,
    epoch: u64,
    path: &str,
) -> Result<(Value, Option<usize>), Error> {
    request(client, c, gate, epoch, path, false)
        .await?
        .ok_or_else(invalid)
}
async fn request(
    client: &reqwest::Client,
    c: &Credential,
    gate: &dyn Gate,
    epoch: u64,
    path: &str,
    missing_ok: bool,
) -> Result<Option<(Value, Option<usize>)>, Error> {
    if EPOCH.load(Ordering::SeqCst) != epoch || !gate.allow("gitea") {
        return Err(Error::Budget);
    }
    let mut request = Request {
        gate,
        started: Instant::now(),
        ok: false,
    };
    let result = async {
        let mut authorization = reqwest::header::HeaderValue::from_str(&format!("token {}", c.token))
            .map_err(|_| Error::Auth(AUTH.into()))?;
        authorization.set_sensitive(true);
        let mut response = client
            .get(format!("{}api/v1/{path}", c.api_base()))
            .header(reqwest::header::AUTHORIZATION, authorization)
            .header(reqwest::header::ACCEPT, "application/json")
            .send()
            .await
            .map_err(|_| Error::Other(FAILED.into()))?;
        match response.status().as_u16() {
            200 => {}
            404 if missing_ok => return Ok(None),
            401 | 403 => return Err(Error::Auth(AUTH.into())),
            429 => {
                return Err(Error::RateLimited {
                    retry_after: Duration::from_secs(
                        response
                            .headers()
                            .get("retry-after")
                            .and_then(|h| h.to_str().ok())
                            .and_then(|v| v.parse::<u64>().ok())
                            .unwrap_or(900)
                            .clamp(60, 86400),
                    ),
                });
            }
            _ => return Err(Error::Other(FAILED.into())),
        }
        if response
            .content_length()
            .is_some_and(|len| len > MAX_BYTES as u64)
        {
            return Err(invalid());
        }
        let total = response
            .headers()
            .get("x-total-count")
            .map(|h| {
                h.to_str()
                    .ok()
                    .and_then(|v| v.parse::<usize>().ok())
                    .ok_or_else(invalid)
            })
            .transpose()?;
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(|_| Error::Other(FAILED.into()))? {
            if bytes.len() + chunk.len() > MAX_BYTES {
                return Err(invalid());
            }
            bytes.extend_from_slice(&chunk);
        }
        Ok(Some((
            serde_json::from_slice(&bytes).map_err(|_| invalid())?,
            total,
        )))
    }
    .await;
    request.ok = result.is_ok();
    if EPOCH.load(Ordering::SeqCst) != epoch {
        return Err(Error::Budget);
    }
    result
}
#[cfg(test)]
#[path = "gitea_tests.rs"]
mod tests;
