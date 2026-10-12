//! The paired phone (ADR 0017, `docs/dev/plan-zeca.md` L2): pairing by QR from Settings, the cards
//! sent to each paired phone, and its answers read back and handed to `core` as `Input::Paired`.
//! Every request to the relay asks the network gate first (feature `phone`, ADR 0019: off until
//! the user turns it on, a daily budget, a local trace). The desktop's key lives in the keyring.
//! In `direct`, this computer runs the relay itself on an address the user chose.

use std::collections::BTreeSet;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::mpsc;
use vults_core::State;
use vults_link::answer::Verified;
use vults_link::card::{Card, ToPhone};
use vults_link::envelope::{Link, Side};
use vults_link::pair::{INVITE_LIFE, Invite};
use vults_link::{Device, SealKey};
use vults_relay::client::{Client, Error as RelayError};

/// The phone's state in the app: what is paired and the tasks that serve it.
#[derive(Default)]
pub struct Phone(Mutex<Inner>);

impl std::fmt::Debug for Phone {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Phone")
    }
}

type Task = tauri::async_runtime::JoinHandle<()>;

#[derive(Default)]
struct Inner {
    /// Each phone that may answer, with its link.
    links: Vec<(Device, Arc<Link>)>,
    /// What goes to the phones, in order, through one sender.
    queue: Option<mpsc::UnboundedSender<ToPhone>>,
    /// The requests sent to the phones as cards, so each goes once and its leaving is told.
    sent: BTreeSet<String>,
    /// After a restart every waiting card goes again: a phone just paired has none, and one a
    /// failed send missed gets it now. A phone keeps one card per request.
    resend: bool,
    /// The tasks reading the phones' boxes and sending to them: stopped on every restart.
    tasks: Vec<Task>,
    /// The relay this computer runs in `direct`, and its address: kept across restarts (it holds
    /// what is not read yet) unless the address changes or the phone is turned off.
    direct: Option<(SocketAddr, Task)>,
    /// The pairing on screen, by number: a newer one or a cancel ends the older's wait.
    pairing: u64,
    answers: Option<mpsc::Sender<Verified>>,
}

/// The gate, as the relay's client asks it.
struct Net(AppHandle);

impl vults_relay::client::Gate for Net {
    fn allow(&self) -> bool {
        use vults_connectors::Gate;
        crate::gate::shared(&self.0).allow(crate::gate::PHONE)
    }

    fn done(&self, ok: bool, took: Duration) {
        use vults_connectors::Gate;
        crate::gate::shared(&self.0).done(crate::gate::PHONE, ok, took);
    }
}

/// The phone's part of the settings, read once.
struct Config {
    on: bool,
    /// Where both ends reach the relay.
    base: Option<String>,
    /// Where this computer listens, in `direct`.
    listen: Option<SocketAddr>,
}

fn config(app: &AppHandle) -> Config {
    let s = app
        .state::<crate::settings::SettingsState>()
        .0
        .lock()
        .map(|s| s.clone())
        .unwrap_or_default();
    let listen = s
        .phone_listen
        .as_deref()
        .and_then(|a| a.parse::<SocketAddr>().ok());
    let (base, listen) = if s.phone_transport == "direct" {
        (listen.map(|a| format!("http://{a}")), listen)
    } else {
        (s.phone_relay.clone(), None)
    };
    Config {
        on: s.phone,
        base,
        listen,
    }
}

/// Starts serving the paired phones, if the user turned it on; their answers go to `answers`.
pub fn start(app: &AppHandle, answers: mpsc::Sender<Verified>) {
    app.manage(Phone::default());
    if let Ok(mut inner) = app.state::<Phone>().0.lock() {
        inner.answers = Some(answers);
    }
    restart(app);
}

/// Serves the phones again from the settings and the database (a switch, an address, a pairing
/// or a revoke changed them).
pub fn restart(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        if let Err(e) = restart_now(&app).await {
            tracing::warn!(error = %e, "phone not served");
            let _ = app.emit("phone", serde_json::json!({ "error": "serve", "detail": e }));
        }
    });
}

/// Tasks that are stopped when dropped, unless kept: a restart that fails midway leaves none.
struct Tasks(Vec<Task>);

impl Tasks {
    fn keep(mut self) -> Vec<Task> {
        std::mem::take(&mut self.0)
    }
}

impl Drop for Tasks {
    fn drop(&mut self) {
        for task in self.0.drain(..) {
            task.abort();
        }
    }
}

async fn restart_now(app: &AppHandle) -> Result<(), String> {
    // Two restarts at once would each start the tasks.
    static ONE: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
    let _one = ONE.lock().await;
    let config = config(app);
    let wanted = if config.on { config.listen } else { None };
    let (answers, running) = {
        let phone = app.state::<Phone>();
        let mut inner = phone.0.lock().map_err(|_| "phone state busy")?;
        for task in inner.tasks.drain(..) {
            task.abort();
        }
        inner.links.clear();
        inner.queue = None;
        inner.resend = true;
        if inner
            .direct
            .as_ref()
            .is_some_and(|(addr, _)| Some(*addr) != wanted)
            && let Some((_, server)) = inner.direct.take()
        {
            server.abort();
        }
        (inner.answers.clone(), inner.direct.is_some())
    };
    if let (Some(addr), false) = (wanted, running) {
        let server = serve_direct(addr).await?;
        if let Ok(mut inner) = app.state::<Phone>().0.lock() {
            inner.direct = Some((addr, server));
        }
    }
    let (true, Some(base), Some(answers)) = (config.on, config.base, answers) else {
        return Ok(());
    };
    let mut tasks = Tasks(Vec::new());
    let client = Arc::new(Client::new(&base, Arc::new(Net(app.clone()))).map_err(|e| e.to_string())?);
    let mut links = Vec::new();
    if let Some(key) = key(false).await? {
        for device in paired().await? {
            let link = Arc::new(
                Link::new(Side::Desktop, &key, &device.seal_key, &device.id).map_err(|e| e.to_string())?,
            );
            tasks.0.push(tauri::async_runtime::spawn(listen(
                client.clone(),
                key.clone(),
                device.clone(),
                link.clone(),
                answers.clone(),
            )));
            links.push((device, link));
        }
    }
    let (queue, out) = mpsc::unbounded_channel();
    let sends: Vec<Arc<Link>> = links.iter().map(|(_, l)| l.clone()).collect();
    tasks
        .0
        .push(tauri::async_runtime::spawn(sender(client, sends, out)));
    let phone = app.state::<Phone>();
    let mut inner = phone.0.lock().map_err(|_| "phone state busy")?;
    inner.tasks = tasks.keep();
    inner.links = links;
    inner.queue = Some(queue);
    drop(inner);
    // The cards already waiting go to the phones on the loop's next turn.
    crate::runtime::recheck(app);
    Ok(())
}

/// The relay this computer runs for the phone in `direct`.
async fn serve_direct(addr: SocketAddr) -> Result<Task, String> {
    // The relay just stopped may not have let go of the port yet.
    let mut tries = 0;
    let listener = loop {
        match tokio::net::TcpListener::bind(addr).await {
            Ok(l) => break l,
            Err(_) if tries < 20 => {
                tries += 1;
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
            Err(e) => return Err(format!("can't listen on {addr}: {e}")),
        }
    };
    tracing::info!(%addr, "phone relay listening");
    Ok(tauri::async_runtime::spawn(async move {
        if let Err(e) = vults_relay::server::serve(listener, vults_relay::server::Limits::default()).await {
            tracing::warn!(error = %e, "phone relay stopped");
        }
    }))
}

/// Sends what the loop queues to every phone, in order: a card's `Gone` never passes its card. A
/// send that fails is tried twice more; a closed gate is not.
async fn sender(client: Arc<Client>, links: Vec<Arc<Link>>, mut out: mpsc::UnboundedReceiver<ToPhone>) {
    while let Some(message) = out.recv().await {
        let bytes = message.to_bytes();
        for link in &links {
            for attempt in 1..=3u32 {
                match client.send(link.outbox(), &link.seal(&bytes)).await {
                    Ok(()) | Err(RelayError::Closed) => break,
                    Err(e) => {
                        tracing::debug!(error = %e, attempt, "phone send failed");
                        tokio::time::sleep(Duration::from_secs(2 * u64::from(attempt))).await;
                    }
                }
            }
        }
    }
}

/// Reads one phone's box for good, handing each answer that is its own to the loop.
async fn listen(
    client: Arc<Client>,
    key: Arc<SealKey>,
    device: Device,
    link: Arc<Link>,
    answers: mpsc::Sender<Verified>,
) {
    let mut backoff = Duration::from_secs(5);
    loop {
        let asked = Instant::now();
        match client.receive(link.inbox(), Duration::from_secs(25)).await {
            Ok(envelopes) => {
                backoff = Duration::from_secs(5);
                // A relay that answers an empty box at once (not holding the read) would spend the
                // day's budget in minutes.
                if envelopes.is_empty() && asked.elapsed() < Duration::from_secs(1) {
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
                for envelope in envelopes {
                    match vults_link::answer::receive(&key, &device, &envelope) {
                        Ok(answer) => {
                            if answers.send(answer).await.is_err() {
                                return;
                            }
                        }
                        Err(e) => tracing::warn!(device = %device.id, error = %e, "phone answer refused"),
                    }
                }
            }
            // Switched off or the day's budget spent: ask again in a minute.
            Err(RelayError::Closed) => tokio::time::sleep(Duration::from_secs(60)).await,
            Err(e) => {
                tracing::debug!(error = %e, "phone relay unreachable");
                tokio::time::sleep(backoff).await;
                backoff = (backoff * 2).min(Duration::from_secs(120));
            }
        }
    }
}

/// After every turn of the loop: each new card goes to every phone, and each card that left is told.
pub fn sync(app: &AppHandle, state: &State, now: Instant) {
    let Some(phone) = app.try_state::<Phone>() else {
        return;
    };
    let Ok(mut inner) = phone.0.lock() else {
        return;
    };
    let Some(queue) = inner.queue.clone() else {
        return;
    };
    if inner.links.is_empty() {
        inner.sent.clear();
        return;
    }
    let again = std::mem::take(&mut inner.resend);
    let waiting: BTreeSet<String> = state.pending.iter().map(|p| p.request.0.clone()).collect();
    let cards = state
        .pending
        .iter()
        .filter(|p| again || !inner.sent.contains(&p.request.0))
        .map(|p| ToPhone::Card(Box::new(Card::of(state, p, now))));
    let gone = inner
        .sent
        .difference(&waiting)
        .map(|r| ToPhone::Gone { request: r.clone() });
    for message in cards.chain(gone).collect::<Vec<_>>() {
        let _ = queue.send(message);
    }
    inner.sent = waiting;
}

/// The desktop's key from the keyring; made and saved there when `create` and there is none.
async fn key(create: bool) -> Result<Option<Arc<SealKey>>, String> {
    use vults_secrets::Secret;
    tauri::async_runtime::spawn_blocking(move || {
        if let Some(text) = vults_secrets::get(Secret::PhoneKey)? {
            return SealKey::from_keyring(&text)
                .map(|k| Some(Arc::new(k)))
                .map_err(|e| e.to_string());
        }
        if !create {
            return Ok(None);
        }
        let key = SealKey::generate();
        vults_secrets::set(Secret::PhoneKey, &key.to_keyring())?;
        Ok(Some(Arc::new(key)))
    })
    .await
    .map_err(|e| e.to_string())?
}

/// The phones paired and not revoked, from the database.
async fn paired() -> Result<Vec<Device>, String> {
    let rows = tauri::async_runtime::spawn_blocking(|| {
        vults_store::Store::open(&crate::history::dir()).and_then(|s| s.devices())
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;
    Ok(rows
        .into_iter()
        .filter(|d| d.revoked_at.is_none())
        .filter_map(|d| {
            Some(Device {
                seal_key: d.seal_key.as_slice().try_into().ok()?,
                id: d.id,
                name: d.name,
                sign_key: d.sign_key,
                bio_key: d.bio_key,
                revoked: false,
            })
        })
        .collect())
}

/// The phone's counter, saved before its answer is acted on: if it is not, the answer is not.
pub async fn save_counter(device: String, counter: u64) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        vults_store::Store::open(&crate::history::dir())?.raise_counter(&device, counter)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())
}

// ── Settings ─────────────────────────────────────────────────────────────────

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PhoneView {
    on: bool,
    transport: String,
    relay: Option<String>,
    listen: Option<String>,
    devices: Vec<DeviceView>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DeviceView {
    id: String,
    name: String,
    /// Unix seconds.
    paired_at: i64,
    revoked: bool,
}

#[tauri::command]
pub async fn phone_settings(app: AppHandle) -> Result<PhoneView, String> {
    let s = app
        .state::<crate::settings::SettingsState>()
        .0
        .lock()
        .map(|s| s.clone())
        .unwrap_or_default();
    let rows = tauri::async_runtime::spawn_blocking(|| {
        vults_store::Store::open(&crate::history::dir()).and_then(|s| s.devices())
    })
    .await
    .map_err(|e| e.to_string())?
    .unwrap_or_default();
    Ok(PhoneView {
        on: s.phone,
        transport: s.phone_transport,
        relay: s.phone_relay,
        listen: s.phone_listen,
        devices: rows
            .into_iter()
            .map(|d| DeviceView {
                id: d.id,
                name: d.name,
                paired_at: d.paired_at,
                revoked: d.revoked_at.is_some(),
            })
            .collect(),
    })
}

/// The phone on or off; its relay's requests are on the user's behalf, so only their switch.
#[tauri::command]
pub fn set_phone(window: tauri::Webview, app: AppHandle, on: bool) -> Result<(), String> {
    crate::settings_page(&window)?;
    crate::settings::edit(&app, |s| s.phone = on)?;
    restart(&app);
    Ok(())
}

#[tauri::command]
pub fn set_phone_transport(window: tauri::Webview, app: AppHandle, transport: String) -> Result<(), String> {
    crate::settings_page(&window)?;
    let transport = ["relay", "direct"]
        .into_iter()
        .find(|t| *t == transport)
        .ok_or("Unknown transport.")?;
    crate::settings::edit(&app, |s| s.phone_transport = transport.to_string())?;
    restart(&app);
    Ok(())
}

/// The relay's address: https, or http for one on a LAN or a tailnet.
#[tauri::command]
pub fn set_phone_relay(window: tauri::Webview, app: AppHandle, url: String) -> Result<(), String> {
    crate::settings_page(&window)?;
    let url = url.trim().trim_end_matches('/').to_string();
    let ok = (url.starts_with("https://") || url.starts_with("http://"))
        && url.len() <= 300
        && !url.chars().any(|c| c.is_whitespace() || c.is_control());
    if !ok {
        return Err("Give an address that starts with https://.".into());
    }
    crate::settings::edit(&app, |s| s.phone_relay = Some(url))?;
    restart(&app);
    Ok(())
}

/// Where this computer listens in `direct`: an address the phone can reach, never "any".
#[tauri::command]
pub fn set_phone_listen(window: tauri::Webview, app: AppHandle, addr: String) -> Result<(), String> {
    crate::settings_page(&window)?;
    let parsed: SocketAddr = addr
        .trim()
        .parse()
        .map_err(|_| "Give an address and a port, like 100.64.0.2:8787.")?;
    if parsed.ip().is_unspecified() || parsed.port() == 0 {
        return Err("Give this computer's own address on your network or tailnet, and a port.".into());
    }
    crate::settings::edit(&app, |s| s.phone_listen = Some(parsed.to_string()))?;
    restart(&app);
    Ok(())
}

/// A QR on screen: its text, and its modules row by row (`1` dark) for Settings to draw.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Pairing {
    text: String,
    size: usize,
    cells: String,
    seconds: u64,
}

#[tauri::command]
pub async fn phone_pair(window: tauri::Webview, app: AppHandle) -> Result<Pairing, String> {
    crate::settings_page(&window)?;
    let config = config(&app);
    if !config.on {
        return Err("Turn on the phone first.".into());
    }
    let base = config
        .base
        .ok_or("Set how the phone reaches this computer first.")?;
    let key = key(true).await?.ok_or("Can't make this computer's key.")?;
    let invite = Invite::new(&key, &base, Instant::now());
    let text = invite.qr();
    let qr = qrcode::QrCode::new(text.as_bytes()).map_err(|e| e.to_string())?;
    let cells = qr
        .to_colors()
        .iter()
        .map(|c| if *c == qrcode::Color::Dark { '1' } else { '0' })
        .collect();
    let number = {
        let phone = app.state::<Phone>();
        let mut inner = phone.0.lock().map_err(|_| "phone state busy")?;
        inner.pairing += 1;
        inner.pairing
    };
    let client = Client::new(&base, Arc::new(Net(app.clone()))).map_err(|e| e.to_string())?;
    tauri::async_runtime::spawn(wait_for_phone(app, client, key, invite, number));
    Ok(Pairing {
        text,
        size: qr.width(),
        cells,
        seconds: INVITE_LIFE.as_secs(),
    })
}

#[tauri::command]
pub fn phone_pair_cancel(window: tauri::Webview, app: AppHandle) -> Result<(), String> {
    crate::settings_page(&window)?;
    if let Ok(mut inner) = app.state::<Phone>().0.lock() {
        inner.pairing += 1;
    }
    Ok(())
}

/// Settings → Phone → Revoke: nothing it signs is taken from now on.
#[tauri::command]
pub async fn phone_revoke(window: tauri::Webview, app: AppHandle, id: String) -> Result<(), String> {
    crate::settings_page(&window)?;
    tauri::async_runtime::spawn_blocking(move || {
        vults_store::Store::open(&crate::history::dir())?.revoke_device(&id, crate::history::unix_now())
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;
    restart_now(&app).await
}

fn pairing_now(app: &AppHandle, number: u64) -> bool {
    app.state::<Phone>().0.lock().is_ok_and(|i| i.pairing == number)
}

/// Waits for the phone's answer to the QR on screen, for its two minutes or until another
/// pairing or a cancel. Settings hears how it ended (`phone` event), unless it was superseded.
async fn wait_for_phone(app: AppHandle, client: Client, key: Arc<SealKey>, mut invite: Invite, number: u64) {
    let tell = |what: serde_json::Value| {
        if pairing_now(&app, number) {
            let _ = app.emit("phone", what);
        }
    };
    let until = Instant::now() + INVITE_LIFE;
    while Instant::now() < until && pairing_now(&app, number) {
        let messages = match client.receive(&invite.pair_box(), Duration::from_secs(20)).await {
            Ok(m) => m,
            Err(RelayError::Closed) => {
                tell(serde_json::json!({ "error": "closed" }));
                return;
            }
            Err(e) => {
                tracing::debug!(error = %e, "pairing box unreachable");
                tokio::time::sleep(Duration::from_secs(3)).await;
                continue;
            }
        };
        for message in messages {
            // Cancelled (or another QR shown) while the read was out: what it brought is not taken.
            if !pairing_now(&app, number) {
                return;
            }
            let device = match invite.accept(&key, &message, Instant::now()) {
                Ok(d) => d,
                Err(e) => {
                    tracing::warn!(error = %e, "pairing answer refused");
                    continue;
                }
            };
            if let Err(e) = welcome(&app, &client, &key, &device).await {
                tracing::warn!(error = %e, "phone not paired");
                tell(serde_json::json!({ "error": "save" }));
                return;
            }
            tell(serde_json::json!({ "paired": device.name }));
            return;
        }
    }
    tell(serde_json::json!({ "expired": true }));
}

/// Saves the new phone, serves it, and tells it it is paired.
async fn welcome(app: &AppHandle, client: &Client, key: &SealKey, device: &Device) -> Result<(), String> {
    let row = vults_store::devices::Device {
        id: device.id.clone(),
        name: device.name.clone(),
        seal_key: device.seal_key.to_vec(),
        sign_key: device.sign_key.clone(),
        bio_key: device.bio_key.clone(),
        paired_at: crate::history::unix_now(),
        revoked_at: None,
        counter: 0,
    };
    tauri::async_runtime::spawn_blocking(move || {
        vults_store::Store::open(&crate::history::dir())?.pair_device(&row)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;
    let link = Link::new(Side::Desktop, key, &device.seal_key, &device.id).map_err(|e| e.to_string())?;
    let hello = ToPhone::Welcome {
        device: device.id.clone(),
        name: device.name.clone(),
    };
    client
        .send(link.outbox(), &link.seal(&hello.to_bytes()))
        .await
        .map_err(|e| e.to_string())?;
    restart_now(app).await
}
