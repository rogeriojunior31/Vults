//! The Settings page owns rebinding. Reserve keys, save, then publish the new active map.
use std::collections::BTreeMap;
use tauri::AppHandle;
#[cfg(windows)]
use tauri::Manager;

#[tauri::command]
pub fn windows_shortcut_settings(
    app: AppHandle,
    window: tauri::Webview,
) -> Result<Option<serde_json::Value>, String> {
    crate::settings_page(&window)?;
    #[cfg(windows)]
    return app
        .try_state::<vults_platform::windows_shortcuts::Controller>()
        .ok_or("shortcut-unavailable")
        .and_then(|controller| {
            serde_json::to_value(controller.snapshot())
                .map(Some)
                .map_err(|_| "shortcut-unavailable")
        })
        .map_err(String::from);
    #[cfg(not(windows))]
    {
        let _ = app;
        Ok(None)
    }
}

#[tauri::command]
pub async fn set_windows_shortcuts(
    app: AppHandle,
    window: tauri::Webview,
    bindings: BTreeMap<String, String>,
) -> Result<serde_json::Value, String> {
    crate::settings_page(&window)?;
    #[cfg(windows)]
    {
        let controller = app
            .try_state::<vults_platform::windows_shortcuts::Controller>()
            .ok_or("shortcut-unavailable")?
            .inner()
            .clone();
        tauri::async_runtime::spawn_blocking(move || {
            let saved_bindings = bindings.clone();
            let snapshot = controller.apply(bindings, move || {
                let state = app.state::<crate::settings::SettingsState>();
                let mut saved = state.0.lock().map_err(|_| "shortcut-save-failed")?;
                let mut next = saved.clone();
                next.windows_shortcuts = saved_bindings;
                crate::settings::save(&next).map_err(|_| "shortcut-save-failed")?;
                *saved = next;
                Ok(())
            })?;
            serde_json::to_value(snapshot).map_err(|_| "shortcut-unavailable".into())
        })
        .await
        .map_err(|_| "shortcut-unavailable")?
    }
    #[cfg(not(windows))]
    {
        let _ = (app, bindings);
        Err("shortcut-unavailable".into())
    }
}
