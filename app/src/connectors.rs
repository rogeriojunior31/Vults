//! Connectors: started with the app, switched from the settings window.

use std::collections::BTreeMap;
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Manager};
use tokio::sync::mpsc;
use vultures_ai_connectors::{Runtime, Status, Update};

use crate::{paths, settings};

#[derive(Debug)]
pub struct Connectors {
    runtime: Runtime,
}

/// Starts every connector; each polls only while enabled. News and cards go to `events`.
pub fn start(app: &AppHandle, events: mpsc::Sender<Update>) {
    let enabled = app
        .state::<settings::SettingsState>()
        .0
        .lock()
        .map(|s| s.connectors.clone())
        .unwrap_or_default();
    // `setup` runs on the main thread, outside Tokio; the runtime spawns its tasks on Tauri's.
    let tokio = tauri::async_runtime::handle();
    let _inside = tokio.inner().enter();
    let runtime = Runtime::start(
        vultures_ai_connectors::all(),
        &enabled,
        paths::data_dir().join("connectors"),
        events,
    );
    app.manage(Connectors { runtime });
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectorStatus {
    pub id: String,
    #[serde(flatten)]
    pub status: Status,
}

#[tauri::command]
pub async fn connectors_status(state: tauri::State<'_, Connectors>) -> Result<Vec<ConnectorStatus>, ()> {
    let all: BTreeMap<String, Status> = state.runtime.statuses().await;
    Ok(all
        .into_iter()
        .map(|(id, status)| ConnectorStatus { id, status })
        .collect())
}

/// The island or a connector's card opened: news older than this is fetched again.
const FRESH_ON_OPEN: Duration = Duration::from_secs(60);

#[tauri::command]
pub fn connectors_refresh(state: tauri::State<'_, Connectors>) {
    for id in state.runtime.ids() {
        state.runtime.refresh_if_stale(id, FRESH_ON_OPEN);
    }
}

#[tauri::command]
pub fn connector_enable(app: AppHandle, id: String, on: bool) -> Result<(), String> {
    settings::edit(&app, |s| {
        s.connectors.insert(id.clone(), on);
    })?;
    app.state::<Connectors>().runtime.set_enabled(&id, on);
    Ok(())
}
