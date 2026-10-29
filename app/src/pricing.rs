//! Public price metadata only: independently consented, cached and never used as a bill.
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};
use vults_agents::prices::Catalog;
use vults_connectors::Gate;

const EVERY: Duration = Duration::from_secs(15 * 60);
static WAKE: tokio::sync::Notify = tokio::sync::Notify::const_new();

#[derive(Debug, Default)]
pub struct PriceState {
    book: Mutex<Option<Arc<Catalog>>>,
    updating: AtomicBool,
    failed: AtomicBool,
}

#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    enabled: bool,
    checked_at: Option<i64>,
    stale: bool,
    updating: bool,
    failed: bool,
}

pub fn book(app: &AppHandle) -> Option<Arc<Catalog>> {
    app.try_state::<PriceState>()?.book.lock().ok()?.clone()
}

#[tauri::command]
pub fn model_price_status(app: AppHandle) -> Status {
    let state = app.state::<PriceState>();
    let book = book(&app);
    Status {
        enabled: crate::settings::price_updates(&app),
        checked_at: book.as_ref().map(|b| b.checked_at),
        stale: book.as_ref().is_none_or(|b| !b.fresh(crate::usage::epoch_now())),
        updating: state.updating.load(Ordering::Acquire),
        failed: state.failed.load(Ordering::Acquire),
    }
}

fn publish(app: &AppHandle) {
    let _ = app.emit("model-prices", model_price_status(app.clone()));
}
pub fn wake() {
    WAKE.notify_one();
}

pub fn start(app: &AppHandle) {
    app.manage(PriceState::default());
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let path = crate::paths::data_dir().join("model-prices.json");
        if let Some(book) = std::fs::read(&path)
            .ok()
            .and_then(|raw| serde_json::from_slice::<Catalog>(&raw).ok())
            .filter(Catalog::valid)
        {
            *app.state::<PriceState>()
                .book
                .lock()
                .unwrap_or_else(|e| e.into_inner()) = Some(Arc::new(book));
        }
        publish(&app);
        let mut force = false;
        loop {
            let due = book(&app).is_none_or(|b| {
                crate::usage::epoch_now().saturating_sub(b.checked_at) >= EVERY.as_secs() as i64
            });
            if crate::settings::price_updates(&app) && (due || force) && !crate::lock::locked(&app) {
                let _ = refresh_model_prices(app.clone()).await;
            }
            force = tokio::select! {
                ()=tokio::time::sleep(EVERY)=>false,
                ()=WAKE.notified()=>true,
            };
        }
    });
}

#[tauri::command]
pub async fn refresh_model_prices(app: AppHandle) -> Result<Status, String> {
    let state = app.state::<PriceState>();
    if state.updating.swap(true, Ordering::AcqRel) {
        return Ok(model_price_status(app.clone()));
    }
    publish(&app);
    let result = refresh(&app).await;
    state.failed.store(result.is_err(), Ordering::Release);
    state.updating.store(false, Ordering::Release);
    publish(&app);
    result.map(|_| model_price_status(app))
}

async fn refresh(app: &AppHandle) -> Result<(), String> {
    if !crate::settings::price_updates(app) || crate::lock::locked(app) {
        return Err("Model price updates are off".into());
    }
    let gate = crate::gate::shared(app);
    if !gate.allow(crate::gate::MODEL_PRICES) {
        return Err("Model price refresh unavailable".into());
    }
    let previous = book(app);
    let at = Instant::now();
    let result = vults_chat::model_prices::fetch(previous.as_ref().and_then(|p| p.etag.as_deref())).await;
    gate.done(crate::gate::MODEL_PRICES, result.is_ok(), at.elapsed());
    let updated = result?;
    // The switch may have changed while the request was running.
    if !crate::settings::price_updates(app) {
        return Ok(());
    }
    let book = match updated {
        Some(book) => book,
        None => {
            let mut book = previous.ok_or("No cached model prices")?.as_ref().clone();
            book.checked_at = crate::usage::epoch_now();
            book
        }
    };
    let path = crate::paths::data_dir().join("model-prices.json");
    let temp = path.with_extension("json.new");
    let raw = serde_json::to_vec(&book).map_err(|_| "Could not cache model prices")?;
    std::fs::write(&temp, raw)
        .and_then(|_| std::fs::rename(&temp, &path))
        .map_err(|_| "Could not cache model prices")?;
    *app.state::<PriceState>()
        .book
        .lock()
        .unwrap_or_else(|e| e.into_inner()) = Some(Arc::new(book));
    Ok(())
}
