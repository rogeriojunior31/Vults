//! The Tauri shell: wires the hook server, the core and the UI together, and executes effects.
//! Domain rules live in `core`; this file only moves data and talks to the OS.

mod chat;
mod connectors;
mod installer;
mod log;
mod media;
mod notify;
mod panel;
mod paths;
mod runtime;
mod settings;
mod tray;
mod usage;
mod voice;
mod widget;

use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

pub const ISLAND: &str = "island";
/// The corner widget: built only while the user has it on (`widget.rs`).
pub const WIDGET: &str = "widget";
const SETTINGS: &str = "settings";

/// The windows that are layer surfaces (mapped once at a fixed size, one input region each).
/// Each needs an entry in `layer_spec`, `app/capabilities/` and, for its own page, `ui/vite.config.ts`.
pub const SURFACES: &[&str] = &[ISLAND, WIDGET];

/// How each surface sits on the screen.
#[cfg(target_os = "linux")]
fn layer_spec(label: &str) -> Option<vultures_ai_platform::linux::LayerSpec> {
    use vultures_ai_platform::linux::{Edges, LayerSpec};
    match label {
        ISLAND => {
            let (width, height) = runtime::ISLAND_SIZE;
            Some(LayerSpec {
                namespace: vultures_ai_brand::SLUG.into(),
                width,
                height,
                edges: Edges::TOP,
                margin: 0,
                keyboard: false,
                overlay: true,
            })
        }
        // Never built from here: `widget::build` uses the corner the setting holds.
        WIDGET => Some(widget::layer_spec(widget::Corner::BottomRight)),
        _ => None,
    }
}

pub fn run() {
    let _log = log::init();
    tracing::info!(
        version = env!("CARGO_PKG_VERSION"),
        "{} starting",
        vultures_ai_brand::NAME
    );
    tauri::Builder::default()
        // The socket is removed and rebound on start, so a second instance would steal it.
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            open_settings(app, None)
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .invoke_handler(tauri::generate_handler![
            runtime::current_view,
            runtime::step_diff,
            runtime::decide,
            runtime::layout,
            runtime::surface_keyboard,
            installer::install_status,
            installer::install_preview,
            installer::install_apply,
            chat::chat_send,
            chat::chat_reset,
            chat::chat_decide,
            chat::chat_stop,
            chat::api_key_status,
            chat::api_key_set,
            chat::api_key_clear,
            chat::api_providers,
            chat::api_provider_set,
            chat::api_model_set,
            chat::api_models,
            runtime::session_jump,
            runtime::session_focus,
            runtime::decide_always,
            runtime::question_answer,
            runtime::question_release,
            runtime::rules_list,
            runtime::rule_remove,
            runtime::alert_open,
            runtime::alert_dismiss,
            runtime::board_open,
            connectors::connectors_status,
            connectors::connector_enable,
            connectors::connectors_refresh,
            settings::app_settings,
            settings::set_sounds,
            settings::set_volume,
            settings::set_autostart,
            settings::set_fold_after,
            settings::monitors,
            settings::set_monitor,
            settings::set_now_playing,
            settings::set_zeca_species,
            settings::set_visitors,
            settings::set_notifications,
            settings::set_zeca,
            panel::set_presence,
            panel::island_place,
            widget::set_widget,
            runtime::set_flock,
            runtime::set_zeca_look,
            media::media_control,
            media::media_now,
            voice::voice_status,
            voice::voice_download,
            voice::voice_select,
            voice::voice_language_set,
            voice::voice_off,
            voice::voice_start,
            voice::voice_stop,
            voice::voice_cancel,
            usage::usage,
            open_settings_window,
            shortcut_keys,
            first_name,
        ])
        .on_window_event(|win, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event
                && SURFACES.contains(&win.label())
            {
                // Nothing closes a surface on purpose (Quit exits the app). On Linux this is
                // the compositor closing it because its monitor left: bring it back.
                api.prevent_close();
                revive(win.app_handle(), win.label());
            }
            if let tauri::WindowEvent::Focused(true) = event
                && !SURFACES.contains(&win.label())
            {
                away(win.app_handle());
            }
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
            handle.manage(media::MediaState::default());
            handle.manage(voice::VoiceState::default());
            handle.manage(usage::UsageState::default());
            init_surface(&handle, ISLAND);
            panel::apply(&handle);
            widget::apply(&handle);
            media::apply(&handle, settings::now_playing(&handle));
            usage::start(&handle);
            #[cfg(target_os = "linux")]
            {
                use tauri::Emitter;
                let app = handle.clone();
                vultures_ai_platform::linux::on_monitors_changed(move || {
                    place_surfaces(&app);
                    // The settings list the screens: a plugged one shows up without reopening.
                    let _ = app.emit("monitors", ());
                });
                if let Some(gtk) = handle
                    .get_webview_window(ISLAND)
                    .and_then(|w| w.gtk_window().ok())
                {
                    let app = handle.clone();
                    vultures_ai_platform::linux::on_pointer_crossing(&gtk, move |inside| {
                        tracing::debug!(inside, "pointer crossed the island's edge");
                        let _ = app.emit_to(ISLAND, "pointer", inside);
                    });
                }
            }
            installer::ensure_hook_exe(&handle);
            handle.manage(chat::ChatState::new());
            chat::clean_inbox();
            tray::start(&handle)?;
            handle.manage(ShortcutKeys::default());
            #[cfg(target_os = "linux")]
            listen_shortcuts(&handle);
            notify::start(&handle);
            runtime::start(handle);
            Ok(())
        })
        .build(tauri::generate_context!())
        .map(|app| {
            app.run(|_, event| {
                // A clean quit leaves no socket behind; a crash's leftover is cleared at the next start.
                if let tauri::RunEvent::Exit = event {
                    vultures_ai_ipc::remove_socket();
                }
            })
        })
        .unwrap_or_else(|err| tracing::error!("{} stopped: {err}", vultures_ai_brand::NAME));
}

/// Layer-shell has to be chosen before a surface is first mapped, which is why
/// tauri.conf.json creates the island hidden. `setup` runs on the main thread.
fn init_surface(app: &AppHandle, label: &str) {
    let Some(win) = app.get_webview_window(label) else {
        return;
    };
    #[cfg(target_os = "linux")]
    {
        let monitor = settings::monitor(app);
        let layered = layer_spec(label).is_some_and(|spec| {
            win.gtk_window()
                .is_ok_and(|g| vultures_ai_platform::linux::init_layer(&g, label, &spec, monitor.as_deref()))
        });
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

/// Puts every surface on the chosen monitor, or lets the compositor choose. From any thread.
pub fn place_surfaces(app: &AppHandle) {
    #[cfg(target_os = "linux")]
    for label in SURFACES {
        let Some(win) = app.get_webview_window(label) else {
            continue;
        };
        let wanted = settings::monitor(app);
        let _ = app.run_on_main_thread(move || {
            if let Ok(gtk) = win.gtk_window() {
                vultures_ai_platform::linux::place(&gtk, label, wanted.as_deref());
            }
        });
    }
    #[cfg(not(target_os = "linux"))]
    let _ = app;
}

fn revive(app: &AppHandle, label: &str) {
    #[cfg(target_os = "linux")]
    if let Some(win) = app.get_webview_window(label)
        && let Ok(gtk) = win.gtk_window()
    {
        tracing::info!(label, "a surface was closed; mapping it again");
        vultures_ai_platform::linux::revive(&gtk, label, settings::monitor(app));
    }
    #[cfg(not(target_os = "linux"))]
    let _ = (app, label);
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    #[test]
    fn every_surface_has_a_layer_spec() {
        // Without one it would quietly come up as a plain window.
        for label in super::SURFACES {
            assert!(super::layer_spec(label).is_some(), "{label} has no layer spec");
        }
    }
}

/// The keys the desktop bound for the global shortcuts, by id, for the island's buttons. Kept, so
/// an island that loads after the binding still gets them.
#[derive(Default)]
struct ShortcutKeys(std::sync::Mutex<std::collections::BTreeMap<String, String>>);

#[tauri::command]
fn shortcut_keys(state: tauri::State<'_, ShortcutKeys>) -> std::collections::BTreeMap<String, String> {
    state.0.lock().map(|m| m.clone()).unwrap_or_default()
}

/// The desktop's global shortcuts (`platform::shortcuts::SHORTCUTS`). Next and previous go to
/// core; the rest go to the island, which decides whether a card is on screen to answer (a press
/// with nothing waiting does nothing).
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
            move |id, down| {
                // Next and previous move core's focus; the view brings it to every surface.
                let intent = match (id, down) {
                    ("next", true) => Some(vultures_ai_core::Intent::FocusNext),
                    ("previous", true) => Some(vultures_ai_core::Intent::FocusPrevious),
                    _ => None,
                };
                if let Some(intent) = intent {
                    runtime::shortcut_intent(&emit, intent);
                    return;
                }
                // The talk key is the chat's mic: nothing to hold with Zeca off.
                if id == "talk" && !settings::zeca(&emit) {
                    return;
                }
                // Only the talk key cares about being let go.
                let event = match (id, down) {
                    (_, true) => id.to_string(),
                    ("talk", false) => "talk-up".to_string(),
                    _ => return,
                };
                let _ = emit.emit_to(ISLAND, "shortcut", event);
            },
        )
        .await;
        if let Err(e) = result {
            tracing::warn!("global shortcuts unavailable: {e}");
        }
    });
}

/// For the island's hello; none when the account only has a login name.
#[tauri::command]
fn first_name() -> Option<String> {
    vultures_ai_chat::user::first_name()
}

/// The island's gear button, or a link to one section (`agents`, `chat`…).
#[tauri::command]
fn open_settings_window(app: AppHandle, section: Option<String>) {
    open_settings(&app, section.as_deref());
}

/// The user went to another of the app's windows (Settings). By the panel the open island folds,
/// or it would cover that window's corner; the island decides (a waiting card keeps it open).
/// Settings' page says so itself on a click too: a window that already has the focus gets no
/// focus event.
fn away(app: &AppHandle) {
    use tauri::Emitter;
    let _ = app.emit_to(ISLAND, "away", ());
}

/// Opens Settings, at `section` when given (a page id of `ui/src/settings.ts`).
pub(crate) fn open_settings(app: &AppHandle, section: Option<&str>) {
    away(app);
    // A page id is a plain word: nothing else reaches the URL or the event.
    let section = section.filter(|s| !s.is_empty() && s.chars().all(|c| c.is_ascii_lowercase()));
    if let Some(win) = app.get_webview_window(SETTINGS) {
        if let Some(section) = section {
            use tauri::Emitter;
            let _ = app.emit_to(SETTINGS, "settings-section", section);
        }
        let _ = win.show();
        let _ = win.set_focus();
        return;
    }
    let page = match section {
        Some(section) => format!("settings.html#{section}"),
        None => "settings.html".into(),
    };
    let _ = WebviewWindowBuilder::new(app, SETTINGS, WebviewUrl::App(page.into()))
        .title(format!("{} settings", vultures_ai_brand::NAME))
        .inner_size(720.0, 560.0)
        .build();
}
