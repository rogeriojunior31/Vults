//! Nest-only read-only inventory and per-conversation, expiring route consent.
use serde::{Deserialize, Serialize};
#[cfg(target_os = "linux")]
use std::{
    sync::Mutex,
    time::{Duration, Instant},
};
use tauri::{AppHandle, Manager};

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    Headphones,
    Echo,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Selection {
    token: String,
    mode: Mode,
    source: String,
    sink: String,
    confirmed: bool,
}
#[derive(Serialize)]
pub struct Device {
    name: String,
    label: String,
}
#[derive(Serialize)]
pub struct Pair {
    source: String,
    sink: String,
}
#[derive(Serialize)]
pub struct Snapshot {
    token: String,
    sources: Vec<Device>,
    sinks: Vec<Device>,
    pairs: Vec<Pair>,
}
#[cfg(target_os = "linux")]
struct Cached {
    token: String,
    created: Instant,
    inventory: vults_audio::echo::Inventory,
}
#[derive(Default)]
pub(crate) struct Routes(#[cfg(target_os = "linux")] Mutex<(u64, Option<Cached>)>);

#[tauri::command]
pub async fn voice_conversation_routes(app: AppHandle, window: tauri::Webview) -> Result<Snapshot, String> {
    super::controls::page(&window)?;
    #[cfg(not(target_os = "linux"))]
    {
        let _ = app;
        Err("Voice conversation is available on Linux only.".into())
    }
    #[cfg(target_os = "linux")]
    {
        if !super::controls::visible(&app) || crate::lock::locked(&app) {
            return Err("Voice conversation is no longer visible.".into());
        }
        let seq = {
            let cache = app.state::<Routes>();
            let mut cache = cache.0.lock().map_err(|_| "Audio devices are busy.")?;
            cache.0 = cache
                .0
                .checked_add(1)
                .ok_or("Restart the app before listing devices again.")?;
            cache.1 = None;
            cache.0
        };
        let inventory = tauri::async_runtime::spawn_blocking(vults_audio::echo::inventory)
            .await
            .map_err(|_| "Could not list audio devices.")?
            .map_err(|_| "Could not list audio devices.")?;
        if !super::controls::visible(&app) || crate::lock::locked(&app) {
            return Err("Voice conversation is no longer visible.".into());
        }
        let token = crate::traces::new_id();
        let devices = |(name, label): (&str, &str)| Device {
            name: name.into(),
            label: label.into(),
        };
        let view = Snapshot {
            token: token.clone(),
            sources: inventory.sources().map(devices).collect(),
            sinks: inventory.sinks().map(devices).collect(),
            pairs: inventory
                .pairs()
                .iter()
                .map(|pair| Pair {
                    source: pair.source().into(),
                    sink: pair.sink().into(),
                })
                .collect(),
        };
        let cache = app.state::<Routes>();
        let mut cache = cache.0.lock().map_err(|_| "Audio devices are busy.")?;
        if cache.0 != seq {
            return Err("Audio device listing was replaced. Refresh and try again.".into());
        }
        cache.1 = Some(Cached {
            token,
            created: Instant::now(),
            inventory,
        });
        Ok(view)
    }
}
#[cfg(target_os = "linux")]
pub(super) fn resolve<R: tauri::Runtime>(
    app: &AppHandle<R>,
    selection: Option<Selection>,
) -> Result<Option<vults_audio::echo::duplex::Route>, String> {
    let Some(selection) = selection else {
        return Ok(None);
    };
    if !selection.confirmed {
        return Err("Confirm the continuous microphone route before starting.".into());
    }
    let cache = app.state::<Routes>();
    let cache = cache.0.lock().map_err(|_| "Audio devices are busy.")?;
    let cached = cache
        .1
        .as_ref()
        .filter(|cached| {
            cached.token == selection.token && cached.created.elapsed() < Duration::from_secs(300)
        })
        .ok_or("Audio devices have expired. Refresh and choose the route again.")?;
    let route = match selection.mode {
        Mode::Headphones => vults_audio::echo::duplex::Route::headphones_from(
            &cached.inventory,
            &selection.source,
            &selection.sink,
        )
        .map_err(|_| "Choose a listed microphone and output.")?,
        Mode::Echo => vults_audio::echo::duplex::Route::echo(
            cached
                .inventory
                .pairs()
                .iter()
                .find(|pair| pair.source() == selection.source && pair.sink() == selection.sink)
                .cloned()
                .ok_or("Choose a listed echo-cancellation pair.")?,
        ),
    };
    Ok(Some(route))
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;
    #[test]
    fn default_manual_needs_no_inventory_and_continuous_capture_requires_explicit_current_consent() {
        let app = tauri::test::mock_builder()
            .build(tauri::test::mock_context(tauri::test::noop_assets()))
            .unwrap();
        app.manage(Routes::default());
        assert!(resolve(app.handle(), None).unwrap().is_none());
        for confirmed in [false, true] {
            let selection = Selection {
                token: "expired".into(),
                mode: Mode::Headphones,
                source: "mic".into(),
                sink: "out".into(),
                confirmed,
            };
            let error = resolve(app.handle(), Some(selection)).err().unwrap();
            assert!(error.contains(if confirmed { "expired" } else { "Confirm" }));
        }
        for invalid in ["auto", "speakers", "manual", "Headphones"] {
            let value =
                serde_json::json!({"token":"t","mode":invalid,"source":"mic","sink":"out","confirmed":true});
            assert!(serde_json::from_value::<Selection>(value).is_err());
        }
        assert!(
            serde_json::from_value::<Selection>(
                serde_json::json!({"token":"t","mode":"echo","source":"mic","sink":"out"})
            )
            .is_err()
        );
    }
}
