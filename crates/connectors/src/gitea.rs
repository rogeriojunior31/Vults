//! Read-only Gitea API access, with an atomic keyring host/token binding.
use crate::{Connector, Error, Event, Facts, Gate, Group, Level, Poll, Row, Snapshot, Story, StoryKind};
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

#[derive(Debug)]
pub struct Gitea;
pub const INVALID: &str = "Gitea returned incomplete or invalid data. The previous snapshot is kept.";
pub const FAILED: &str = "Could not read Gitea. Check the connection and try again.";
pub const AUTH: &str = "Check the Gitea URL and token in Settings.";
const MAX_BYTES: usize = 1024 * 1024;
const MAX_PAGES: usize = 5;
const LIMIT: usize = 50;
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
    let value = serde_json::to_string(&Credential {
        connection: connection.clone(),
        token,
    })
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
    fn interval(&self, _: &Snapshot) -> Duration {
        Duration::from_secs(300)
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
            tokio::time::timeout(Duration::from_secs(60), read(&client, &c, &*gate, epoch))
                .await
                .map_err(|_| Error::Other(FAILED.into()))?
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
            .filter_map(|v| Some((v["repo"].as_str()?, v["number"].as_u64()?)))
            .collect::<std::collections::BTreeSet<_>>()
            .len()
    }
    fn diff(&self, before: &Snapshot, after: &Snapshot) -> Vec<Event> {
        if before.get("meta") != after.get("meta") {
            return Vec::new();
        }
        after
            .iter()
            .filter(|(key, item)| item["group"] == "review" && !before.contains_key(*key))
            .map(|(key, item)| {
                let name = format!("{}#{}", item["repo"].as_str().unwrap_or_default(), item["number"]);
                Event {
                    connector: "gitea".into(),
                    key: format!("{key}:requested"),
                    topic: None,
                    level: Level::Info,
                    title: format!("Review requested · {name}"),
                    detail: item["title"].as_str().unwrap_or_default().into(),
                    url: item["url"].as_str().map(str::to_owned),
                    story: Some(Story {
                        kind: StoryKind::ReviewRequested,
                        name,
                        branch: None,
                    }),
                }
            })
            .collect()
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
            .get(format!("{}api/v1/{path}", c.connection.base_url))
            .header(reqwest::header::AUTHORIZATION, authorization)
            .header(reqwest::header::ACCEPT, "application/json")
            .send()
            .await
            .map_err(|_| Error::Other(FAILED.into()))?;
        match response.status().as_u16() {
            200 => {}
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
        Ok((serde_json::from_slice(&bytes).map_err(|_| invalid())?, total))
    }
    .await;
    request.ok = result.is_ok();
    if EPOCH.load(Ordering::SeqCst) != epoch {
        return Err(Error::Budget);
    }
    result
}
async fn read(
    client: &reqwest::Client,
    c: &Credential,
    gate: &dyn Gate,
    epoch: u64,
) -> Result<Snapshot, Error> {
    let (user, _) = get(client, c, gate, epoch, "user").await?;
    let login = user["login"]
        .as_str()
        .filter(|s| segment(s))
        .ok_or_else(invalid)?;
    let mut s = Snapshot::from([(
        "meta".into(),
        json!({"login":login,"base":c.connection.base_url,"generation":c.connection.generation}),
    )]);
    for (group, filter) in [("yours", "created=true"), ("review", "review_requested=true")] {
        let mut seen = 0;
        let mut total = None;
        for page in 1..=MAX_PAGES {
            let (value, count) = get(
                client,
                c,
                gate,
                epoch,
                &format!("repos/issues/search?state=open&type=pulls&{filter}&limit={LIMIT}&page={page}"),
            )
            .await?;
            let count = count.filter(|n| *n <= LIMIT * MAX_PAGES).ok_or_else(invalid)?;
            if total.is_some_and(|previous| previous != count) {
                return Err(invalid());
            }
            total = Some(count);
            let rows = value
                .as_array()
                .filter(|rows| rows.len() <= LIMIT)
                .ok_or_else(invalid)?;
            for row in rows {
                insert(&mut s, row, group, login, &c.connection)?;
            }
            seen += rows.len();
            if seen > count || (rows.is_empty() && seen != count) {
                return Err(invalid());
            }
            if seen == count {
                break;
            }
            if page == MAX_PAGES {
                return Err(invalid());
            }
        }
    }
    if EPOCH.load(Ordering::SeqCst) != epoch {
        return Err(Error::Budget);
    }
    Ok(s)
}
fn segment(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 255
        && !matches!(s, "." | "..")
        && s.bytes()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'_' | b'-' | b'.'))
}
fn insert(s: &mut Snapshot, v: &Value, group: &str, login: &str, c: &Connection) -> Result<(), Error> {
    let repo = v["repository"]["full_name"]
        .as_str()
        .filter(|s| s.split('/').count() == 2 && s.split('/').all(segment))
        .ok_or_else(invalid)?;
    let number = v["number"]
        .as_u64()
        .filter(|n| *n > 0 && *n <= u32::MAX as u64)
        .ok_or_else(invalid)?;
    let title = v["title"]
        .as_str()
        .filter(|s| !s.is_empty() && s.len() <= 4096 && !s.chars().any(char::is_control))
        .ok_or_else(invalid)?;
    if v["state"] != "open" || !v["pull_request"].is_object() {
        return Err(invalid());
    }
    let url = format!("{}{repo}/pulls/{number}", c.base_url);
    if SafeUrl::for_service(&url, &c.base_url).is_none() {
        return Err(invalid());
    }
    let key = format!("gitea:{}:{login}:{group}:{repo}#{number}:item", c.generation);
    let rank = s.len();
    if s.insert(key,json!({"group":group,"repo":repo,"number":number,"title":title,"url":url,"author":v["user"]["login"].as_str().filter(|s| segment(s)),"rank":rank})).is_some() { return Err(invalid()); }
    Ok(())
}
fn board(s: &Snapshot) -> Vec<Row> {
    let mut rows: Vec<_> = s
        .iter()
        .filter_map(|(key, v)| {
            let group = match v["group"].as_str()? {
                "yours" => Group::Yours,
                "review" => Group::ToReview,
                _ => return None,
            };
            Some((
                v["rank"].as_u64()?,
                Row {
                    item: key.clone(),
                    group,
                    name: format!("{}#{}", v["repo"].as_str()?.rsplit('/').next()?, v["number"]),
                    title: v["title"].as_str()?.into(),
                    checks: None,
                    review: None,
                    url: v["url"].as_str().map(str::to_owned),
                    head: None,
                    facts: Facts {
                        repo: v["repo"].as_str().map(str::to_owned),
                        author: v["author"].as_str().map(str::to_owned),
                        ..Facts::default()
                    },
                },
            ))
        })
        .collect();
    rows.sort_by_key(|(rank, _)| *rank);
    rows.into_iter().map(|(_, row)| row).collect()
}

#[cfg(test)]
#[path = "gitea_tests.rs"]
mod tests;
