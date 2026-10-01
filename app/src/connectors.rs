//! Connectors: started with the app, switched from the settings window.

use std::collections::BTreeMap;
use std::sync::Mutex;

use serde::Serialize;
use tauri::{AppHandle, Manager};
use tokio::sync::mpsc;
use vultures_ai_connectors::{Event, Runtime, Status};

use crate::{paths, settings};

#[derive(Debug)]
pub struct Connectors {
    runtime: Runtime,
    settings: Mutex<settings::Settings>,
}

/// Starts every connector; each polls only while enabled. Events go to `events`.
pub fn start(app: &AppHandle, events: mpsc::Sender<Event>) {
    let settings = settings::load();
    // `setup` runs on the main thread, outside Tokio; the runtime spawns its tasks on Tauri's.
    let tokio = tauri::async_runtime::handle();
    let _inside = tokio.inner().enter();
    let runtime = Runtime::start(
        vultures_ai_connectors::all(),
        &settings.connectors,
        paths::data_dir().join("connectors"),
        events,
    );
    app.manage(Connectors {
        runtime,
        settings: Mutex::new(settings),
    });
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

#[tauri::command]
pub fn connector_enable(app: AppHandle, id: String, on: bool) -> Result<(), String> {
    let state = app.state::<Connectors>();
    let mut s = state.settings.lock().map_err(|_| "settings are busy")?;
    s.connectors.insert(id.clone(), on);
    settings::save(&s).map_err(|e| format!("can't save the settings: {e}"))?;
    state.runtime.set_enabled(&id, on);
    Ok(())
}
