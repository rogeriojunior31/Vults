//! App-owned restore tokens. Platform only receives them in memory and never logs or serializes them.
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicU64, Ordering},
};
use tauri::{AppHandle, Emitter, Manager};
use vults_platform::insert::{Permission, Portal};
use vults_secrets::{Secret, delete, get, set};

pub(crate) struct Insertion {
    pub portal: Portal,
    epoch: Arc<AtomicU64>,
    tokens: Arc<Mutex<()>>,
    focused: Mutex<std::collections::BTreeSet<String>>,
}

pub fn start(app: &AppHandle) {
    let emit = app.clone();
    let Ok(portal) = Portal::new(move |state| {
        let _ = emit.emit("dictation-permission", state.code());
        if state == Permission::Unavailable {
            let app = emit.clone();
            tauri::async_runtime::spawn_blocking(move || {
                let Some(state) = app.try_state::<Insertion>() else {
                    return;
                };
                let generation = state.epoch.load(Ordering::SeqCst);
                let Ok(_guard) = state.tokens.lock() else {
                    return;
                };
                if state.epoch.load(Ordering::SeqCst) == generation
                    && state.portal.state() == Permission::Unavailable
                {
                    let _ = delete(Secret::DictationKeyboard);
                }
            });
        }
    }) else {
        return;
    };
    app.manage(Insertion {
        portal,
        epoch: Arc::default(),
        tokens: Arc::default(),
        focused: Mutex::default(),
    });
    if crate::settings::dictation_insert(app) && vults_platform::insert::wayland_available() {
        let app = app.clone();
        tauri::async_runtime::spawn(async move {
            let _ = configure(app, true, false).await;
        });
    }
}

pub fn shutdown(app: &AppHandle) {
    if let Some(state) = app.try_state::<Insertion>() {
        state.epoch.fetch_add(1, Ordering::SeqCst);
        state.portal.shutdown();
    }
}

pub fn focus_changed(app: &AppHandle, label: &str, focused: bool) {
    if let Some(state) = app.try_state::<Insertion>()
        && let Ok(mut windows) = state.focused.lock()
    {
        if focused {
            windows.insert(label.into());
        } else {
            windows.remove(label);
        }
        state.portal.focus_changed(!windows.is_empty());
    }
}

pub fn revoke(app: &AppHandle) {
    if let Some(state) = app.try_state::<Insertion>() {
        state.epoch.fetch_add(1, Ordering::SeqCst);
        state.portal.disable();
    }
}

pub async fn configure(app: AppHandle, on: bool, interactive: bool) -> Result<(), String> {
    let Some(state) = app.try_state::<Insertion>() else {
        return Err("The keyboard permission worker is unavailable.".into());
    };
    let portal = state.portal.clone();
    let generation = state.epoch.fetch_add(1, Ordering::SeqCst) + 1;
    let epoch = state.epoch.clone();
    let tokens = state.tokens.clone();
    if !on {
        portal.disable();
    }
    if on && !vults_platform::insert::wayland_available() {
        return Ok(());
    }
    tauri::async_runtime::spawn_blocking(move || -> Result<(), String> {
        let _lock = tokens.lock().map_err(|_| "The keyboard permission token is unavailable.")?;
        if epoch.load(Ordering::SeqCst) != generation { return Ok(()); }
        if !on {
            return delete(Secret::DictationKeyboard).map_err(|_| "Insertion is off, but its saved keyboard token could not be removed. Retry turning insertion off.".to_string());
        }
        let (restore, persist) = match get(Secret::DictationKeyboard) {
            Ok(token) => {
                // Tokens are single-use. Never reuse one whose removal could not be confirmed.
                let usable = token.is_none() || delete(Secret::DictationKeyboard).is_ok();
                (if usable { token } else { None }, usable)
            },
            Err(_) => (None, false),
        };
        if !interactive && restore.is_none() { portal.unavailable(); return Ok(()); }
        if epoch.load(Ordering::SeqCst) != generation { return Ok(()); }
        let save_tokens = tokens.clone();
        let save_epoch = epoch.clone();
        portal.request(restore, persist, move |token| {
            let Ok(_guard) = save_tokens.lock() else { return false; };
            if save_epoch.load(Ordering::SeqCst) != generation { return false; }
            token.is_some_and(|token| set(Secret::DictationKeyboard, token).is_ok())
        });
        Ok(())
    }).await.map_err(|_| "The keyboard permission worker is unavailable.".to_string())?
}

pub fn status(app: AppHandle) -> String {
    app.try_state::<Insertion>()
        .map(|s| s.portal.state().code())
        .unwrap_or("unavailable")
        .into()
}

pub async fn retry(app: AppHandle, window: tauri::Webview) -> Result<(), String> {
    if window.label() != crate::SETTINGS || !crate::settings::dictation_insert(&app) {
        return Err("Enable dictation insertion in Settings first.".into());
    }
    configure(app, true, true).await
}
