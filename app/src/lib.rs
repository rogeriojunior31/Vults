//! The Tauri shell: wires the hook server, the core and the UI together, and executes effects.
//! Domain rules live in `core`; this file only moves data and talks to the OS.

mod chat;
mod connectors;
mod installer;
mod log;
mod paths;
mod runtime;
mod settings;

use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

pub const ISLAND: &str = "island";
const SETTINGS: &str = "settings";

pub fn run() {
    let _log = log::init();
    tracing::info!(
        version = env!("CARGO_PKG_VERSION"),
        "{} starting",
        vultures_ai_brand::NAME
    );
    tauri::Builder::default()
        // The socket is removed and rebound on start, so a second instance would steal it.
        .plugin(tauri_plugin_single_instance::init(|app, _, _| open_settings(app)))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .invoke_handler(tauri::generate_handler![
            runtime::current_view,
            runtime::decide,
            runtime::layout,
            installer::install_status,
            installer::install_preview,
            installer::install_apply,
            chat::chat_send,
            chat::chat_reset,
            chat::chat_decide,
            chat::chat_stop,
            chat::island_keyboard,
            chat::api_key_status,
            chat::api_key_set,
            chat::api_key_clear,
            chat::api_providers,
            chat::api_provider_set,
            chat::api_model_set,
            chat::api_models,
            runtime::session_jump,
            runtime::decide_always,
            runtime::rules_list,
            runtime::rule_remove,
            runtime::alert_open,
            runtime::alert_dismiss,
            connectors::connectors_status,
            connectors::connector_enable,
            settings::app_settings,
            settings::set_sounds,
            settings::set_autostart,
            settings::set_fold_after,
            open_settings_window,
            shortcut_keys,
        ])
        .on_window_event(|win, event| {
            if let tauri::WindowEvent::DragDrop(drag) = event
                && win.label() == ISLAND
            {
                match drag {
                    tauri::DragDropEvent::Enter { .. } => chat::on_drag(win.app_handle(), true),
                    tauri::DragDropEvent::Leave => chat::on_drag(win.app_handle(), false),
                    tauri::DragDropEvent::Drop { paths, .. } => {
                        chat::on_drag(win.app_handle(), false);
                        chat::on_drop(win.app_handle(), paths);
                    }
                    _ => {}
                }
            }
        })
        .setup(|app| {
            let handle = app.handle().clone();
            handle.manage(settings::SettingsState(std::sync::Mutex::new(settings::load())));
            init_island(&handle);
            installer::ensure_hook_exe(&handle);
            handle.manage(chat::ChatState::new());
            chat::clean_inbox();
            tray(&handle)?;
            handle.manage(ShortcutKeys::default());
            #[cfg(target_os = "linux")]
            listen_shortcuts(&handle);
            runtime::start(handle);
            Ok(())
        })
        .run(tauri::generate_context!())
        .unwrap_or_else(|err| tracing::error!("{} stopped: {err}", vultures_ai_brand::NAME));
}

/// Layer-shell has to be chosen before the island is first mapped, which is why
/// tauri.conf.json creates it hidden. `setup` runs on the main thread.
fn init_island(app: &AppHandle) {
    let Some(win) = app.get_webview_window(ISLAND) else {
        return;
    };
    #[cfg(target_os = "linux")]
    {
        let (w, h) = runtime::ISLAND_SIZE;
        let layered = win
            .gtk_window()
            .map(|g| vultures_ai_platform::linux::init_island(&g, w, h))
            .unwrap_or(false);
        if !layered {
            runtime::place_top_center(&win);
            let _ = win.show();
        }
    }
    #[cfg(not(target_os = "linux"))]
    {
        runtime::place_top_center(&win);
        let _ = win.show();
    }
}

fn tray(app: &AppHandle) -> tauri::Result<()> {
    use tauri::menu::{Menu, MenuItem};
    use tauri::tray::TrayIconBuilder;

    let chat = MenuItem::with_id(app, "chat", "Chat…", true, None::<&str>)?;
    let setup = MenuItem::with_id(app, "setup", "Set up agents…", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&chat, &setup, &quit])?;
    let mut tray = TrayIconBuilder::with_id("main")
        .tooltip(vultures_ai_brand::NAME)
        .menu(&menu)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "chat" => {
                use tauri::Emitter;
                let _ = app.emit_to(ISLAND, "open-chat", ());
            }
            "setup" => open_settings(app),
            "quit" => app.exit(0),
            _ => {}
        });
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.build(app)?;
    Ok(())
}

/// The keys the desktop bound for the global shortcuts, by id, for the island's buttons. Kept, so
/// an island that loads after the binding still gets them.
#[derive(Default)]
struct ShortcutKeys(std::sync::Mutex<std::collections::BTreeMap<String, String>>);

#[tauri::command]
fn shortcut_keys(state: tauri::State<'_, ShortcutKeys>) -> std::collections::BTreeMap<String, String> {
    state.0.lock().map(|m| m.clone()).unwrap_or_default()
}

/// Ctrl+Alt+Y / N through the desktop's global shortcuts. The island decides whether a card is on
/// screen to answer; a press with nothing waiting does nothing.
#[cfg(target_os = "linux")]
fn listen_shortcuts(app: &AppHandle) {
    use tauri::Emitter;
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let emit = app.clone();
        let keys = app.clone();
        let result = vultures_ai_platform::shortcuts::listen(
            vultures_ai_brand::BUNDLE_ID,
            move |bound| {
                tracing::info!(count = bound.len(), "global shortcuts bound");
                let map: std::collections::BTreeMap<_, _> = bound.into_iter().collect();
                if let Ok(mut kept) = keys.state::<ShortcutKeys>().0.lock() {
                    kept.clone_from(&map);
                }
                let _ = keys.emit_to(ISLAND, "shortcut-keys", map);
            },
            move |id| {
                let _ = emit.emit_to(ISLAND, "shortcut", id);
            },
        )
        .await;
        if let Err(e) = result {
            tracing::warn!("global shortcuts unavailable: {e}");
        }
    });
}

/// The island's gear button.
#[tauri::command]
fn open_settings_window(app: AppHandle) {
    open_settings(&app);
}

fn open_settings(app: &AppHandle) {
    if let Some(win) = app.get_webview_window(SETTINGS) {
        let _ = win.show();
        let _ = win.set_focus();
        return;
    }
    let _ = WebviewWindowBuilder::new(app, SETTINGS, WebviewUrl::App("settings.html".into()))
        .title(format!("{} settings", vultures_ai_brand::NAME))
        .inner_size(720.0, 560.0)
        .build();
}
