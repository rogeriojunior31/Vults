//! The Tauri shell: wires the hook server, the core and the UI together, and executes effects.
//! Domain rules live in `core`; this file only moves data and talks to the OS.

mod audit;
mod chat;
mod connectors;
mod context;
mod doctor;
mod flock_tools;
mod gate;
mod handoff;
mod history;
mod installer;
mod journal;
mod lock;
mod log;
mod mcp;
mod media;
mod memory;
mod notify;
mod open;
mod panel;
mod paths;
mod phone;
mod policies;
mod recall;
mod records;
mod roosts;
mod runs;
mod runtime;
mod settings;
mod sleep;
mod spend;
mod tasks;
mod traces;
mod tray;
mod usage;
mod voice;
mod web_tools;
mod widget;
mod writer;

use tauri::{AppHandle, Manager, WebviewUrl};

pub const ISLAND: &str = "island";
/// The corner widget: built only while the user has it on (`widget.rs`).
pub const WIDGET: &str = "widget";
/// Settings' webview, in the Nest's window (ADR 0023). Commands check the webview's label, never
/// the window's: the Nest's own webview shares the window and shows agents' text.
pub(crate) const SETTINGS: &str = "settings";
/// The Nest's webview: built with its window when opened, gone when it closes (ADR 0008, D6).
pub const NEST: &str = "nest";
/// The Nest's window, holding the Nest's webview and Settings' (one shown at a time). A label of
/// its own, apart from both webviews'.
pub const NEST_WINDOW: &str = "nest-window";

/// Only Settings' webview writes an agent's config or a key: another webview's call is refused,
/// whatever its page does.
pub(crate) fn settings_page<R: tauri::Runtime>(window: &tauri::Webview<R>) -> Result<(), String> {
    if window.label() == SETTINGS {
        Ok(())
    } else {
        Err("only Settings can do this".into())
    }
}

/// The windows that are layer surfaces (mapped once at a fixed size, one input region each).
/// Each needs an entry in `layer_spec`, `app/capabilities/` and, for its own page, `ui/vite.config.ts`.
pub const SURFACES: &[&str] = &[ISLAND, WIDGET];

/// How each surface sits on the screen.
#[cfg(target_os = "linux")]
fn layer_spec(label: &str) -> Option<vults_platform::linux::LayerSpec> {
    use vults_platform::linux::{Edges, LayerSpec};
    match label {
        ISLAND => {
            let (width, height) = runtime::ISLAND_SIZE;
            Some(LayerSpec {
                namespace: vults_brand::SLUG.into(),
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

/// Runs this binary as Zeca's MCP server when asked (`vults --zeca-mcp`, M7): true when it did.
pub fn serve_mcp_if_asked() -> bool {
    if std::env::args().nth(1).as_deref() != Some(mcp::ARG) {
        return false;
    }
    mcp::run();
    true
}

pub fn run() {
    paths::move_legacy();
    let _log = log::init();
    tracing::info!(
        version = env!("CARGO_PKG_VERSION"),
        "{} starting",
        vults_brand::NAME
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
            chat::chat_models,
            chat::chat_conversations,
            roosts::roosts,
            doctor::agents_doctor,
            roosts::roost_share_preview,
            roosts::roost_share_apply,
            chat::chat_current,
            chat::chat_to_island,
            chat::chat_open,
            chat::chat_rename,
            chat::chat_delete,
            chat::chat_pick,
            chat::set_chat_pick,
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
            runtime::session_open_folder,
            runtime::session_open_file,
            runtime::session_project_pref,
            runtime::session_project_bird,
            runtime::session_hush,
            runtime::digest_dismiss,
            runtime::recap_dismiss,
            runtime::projects_list,
            runtime::project_set,
            open::editor_found,
            runtime::decide_always,
            runtime::question_answer,
            runtime::question_release,
            runtime::rules_list,
            runtime::rule_remove,
            tasks::task_command,
            spend::spend_today,
            spend::budget_set,
            tasks::tasks_list,
            tasks::task_create,
            tasks::task_edit,
            tasks::task_delete,
            tasks::task_open_issue,
            tasks::task_visit_issue,
            runs::task_start,
            runs::task_agents,
            records::nest_history,
            records::nest_audit,
            records::nest_traces,
            records::nest_usage,
            runs::task_start_fresh,
            runs::task_stop,
            runs::task_review,
            handoff::continue_command,
            runs::task_remove_worktree,
            policies::policies_list,
            policies::policy_preview,
            policies::policy_accept,
            policies::policy_revoke,
            runtime::alert_open,
            runtime::alert_dismiss,
            runtime::board_open,
            connectors::connectors_status,
            connectors::connector_enable,
            connectors::connectors_refresh,
            connectors::github_calendar,
            settings::app_settings,
            settings::set_sounds,
            settings::set_volume,
            settings::set_autostart,
            settings::set_fold_after,
            settings::set_open_on_hover,
            settings::set_history,
            settings::set_language,
            runtime::clear_history,
            runtime::activity,
            history::save_recap_image,
            settings::monitors,
            settings::set_monitor,
            settings::set_now_playing,
            settings::set_codex_usage,
            settings::set_native_chat,
            settings::web_settings,
            settings::set_web,
            settings::set_memory,
            settings::set_journal_summary,
            settings::set_local_embeddings,
            settings::set_memory_suggest,
            settings::set_sleep,
            memory::memory_command,
            memory::memory_view,
            memory::memory_decide,
            memory::memory_undo,
            phone::phone_settings,
            phone::set_phone,
            phone::set_phone_transport,
            phone::set_phone_relay,
            phone::set_phone_listen,
            phone::phone_pair,
            phone::phone_pair_cancel,
            phone::phone_revoke,
            settings::set_web_engine,
            settings::set_searxng_url,
            settings::set_web_key,
            settings::clear_web_key,
            settings::set_zeca_species,
            settings::set_visitors,
            settings::set_notifications,
            settings::set_dnd,
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
            open_nest_window,
            set_size,
            nest_is_full,
            nest_is_open,
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
                // Back from minimized (Wayland says no state for it): the Nest is in sight again,
                // and the island steps aside (ADR 0024).
                if win.label() == NEST_WINDOW {
                    use tauri::Emitter;
                    let _ = win.app_handle().emit_to(ISLAND, "nest-open", true);
                }
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
            #[cfg(target_os = "linux")]
            {
                use tauri::Emitter;
                let app = handle.clone();
                vults_platform::linux::on_monitors_changed(move || {
                    place_surfaces(&app);
                    // The settings list the screens: a plugged one shows up without reopening.
                    let _ = app.emit("monitors", ());
                });
                if let Some(gtk) = handle
                    .get_webview_window(ISLAND)
                    .and_then(|w| w.gtk_window().ok())
                {
                    let app = handle.clone();
                    vults_platform::linux::on_pointer_crossing(&gtk, move |inside| {
                        tracing::debug!(inside, "pointer crossed the island's edge");
                        let _ = app.emit_to(ISLAND, "pointer", inside);
                    });
                }
            }
            installer::ensure_hook_exe(&handle);
            #[cfg(target_os = "linux")]
            settings::move_legacy_autostart(&handle);
            handle.manage(chat::ChatState::new());
            chat::clean_inbox();
            tray::start(&handle)?;
            handle.manage(ShortcutKeys::default());
            #[cfg(target_os = "linux")]
            listen_shortcuts(&handle);
            notify::start(&handle);
            runtime::start(handle.clone());
            // After the runtime: a lock already on at start reaches its inbox.
            lock::start(&handle);
            // After the runtime too: its first read asks whether the screen is locked.
            usage::start(&handle);
            journal::start(handle.clone());
            sleep::start(handle.clone());
            Ok(())
        })
        .build(tauri::generate_context!())
        .map(|app| {
            app.run(|app, event| {
                // A clean quit leaves no socket behind; a crash's leftover is cleared at the next start.
                if let tauri::RunEvent::Exit = event {
                    save_nest_size(app);
                    vults_ipc::remove_socket();
                    // No flock for Zeca's MCP server once the app is gone.
                    let _ = std::fs::remove_file(mcp::flock_file());
                }
            })
        })
        .unwrap_or_else(|err| tracing::error!("{} stopped: {err}", vults_brand::NAME));
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
                .is_ok_and(|g| vults_platform::linux::init_layer(&g, label, &spec, monitor.as_deref()))
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
                vults_platform::linux::place(&gtk, label, wanted.as_deref());
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
        vults_platform::linux::revive(&gtk, label, settings::monitor(app));
    }
    #[cfg(not(target_os = "linux"))]
    let _ = (app, label);
}

// `tauri::test` is no dev dependency on Windows (app/Cargo.toml).
#[cfg(all(test, target_os = "linux"))]
mod label_tests {
    use super::*;

    #[test]
    fn only_settings_webview_may_write() {
        // ADR 0023: the Nest's webview shares Settings' window and shows agents' text; the check is
        // on the webview that called.
        let app = tauri::test::mock_app();
        let build = |label: &str| {
            tauri::WebviewWindowBuilder::new(&app, label, WebviewUrl::default())
                .build()
                .unwrap()
        };
        let (nest, settings, island) = (build(NEST), build(SETTINGS), build(ISLAND));
        assert!(settings_page(settings.as_ref()).is_ok());
        assert!(settings_page(nest.as_ref()).is_err());
        assert!(settings_page(island.as_ref()).is_err());
        assert!(policies::from_settings(settings.as_ref()).is_ok());
        assert!(policies::from_settings(nest.as_ref()).is_err());
    }
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
        let result = vults_platform::shortcuts::listen(
            vults_brand::BUNDLE_ID,
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
                    ("next", true) => Some(vults_core::Intent::FocusNext),
                    ("previous", true) => Some(vults_core::Intent::FocusPrevious),
                    _ => None,
                };
                if let Some(intent) = intent {
                    runtime::shortcut_intent(&emit, intent);
                    return;
                }
                // The sizes (ADR 0021) reach the Nest too, so the app switches them.
                if matches!(id, "mini" | "quick" | "room") {
                    if down {
                        size_to(&emit, id, None);
                    }
                    return;
                }
                // The talk key is the chat's mic: nothing to hold with Zeca off.
                if let Some(event) = vults_platform::shortcuts::island_event(id, down, settings::zeca(&emit))
                {
                    let _ = emit.emit_to(ISLAND, "shortcut", event);
                }
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
    vults_chat::user::first_name()
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

/// The chat's "Open in the Nest" button, and the size switch's Room and Full (ADR 0021), at `tab`
/// when given.
#[tauri::command]
fn open_nest_window(app: AppHandle, tab: Option<String>, full: Option<bool>) {
    open_nest(&app, tab.as_deref(), full);
}

/// Opens the Nest, or brings it forward, at `tab` when given (a tab id of
/// `ui/src/surfaces/nest/render.ts`, or `settings`). It opens with Zeca off too, without its Chat
/// (ADR 0021). `full`: Full (the whole screen) or Room (a window); `None` leaves it as it is.
pub(crate) fn open_nest(app: &AppHandle, tab: Option<&str>, full: Option<bool>) {
    use tauri::Emitter;
    // A tab id is a plain word: nothing else reaches the URL or the event.
    let tab = tab.filter(|t| !t.is_empty() && t.chars().all(|c| c.is_ascii_lowercase()));
    if tab == Some(SETTINGS) {
        nest_window(app, None, None, full);
        return;
    }
    if let Some(tab) = tab
        && app.get_webview(NEST).is_some()
    {
        let _ = app.emit_to(NEST, "nest-tab", tab);
    }
    nest_window(app, Some(tab.unwrap_or("")), None, full);
}

/// Brings the Nest's window forward, building it the first time, and shows one of its two
/// webviews: the Nest's (`nest`: Some, its tab or "" for where it was) or Settings' (`nest`: None,
/// at `section` when given). Only the shown one is in sight and takes the keys.
fn nest_window(app: &AppHandle, nest: Option<&str>, section: Option<&str>, full: Option<bool>) {
    use tauri::{LogicalPosition, WebviewBuilder, window::WindowBuilder};
    away(app);
    let window = match app.get_window(NEST_WINDOW) {
        Some(w) => w,
        None => {
            // The island grown larger (ADR 0024): no system title bar, the page's own frame with
            // the prototype's rounded corners; its header moves the window, its edges resize it.
            let first = nest_first_size(app);
            let built = WindowBuilder::new(app, NEST_WINDOW)
                .title(format!("{} Nest", vults_brand::NAME))
                .inner_size(first.width, first.height)
                .min_inner_size(NEST_MIN.0, NEST_MIN.1)
                .decorations(false)
                .transparent(true)
                .fullscreen(full == Some(true))
                .build();
            let window = match built {
                Ok(w) => w,
                Err(e) => {
                    tracing::warn!("the Nest's window could not be built: {e}");
                    return;
                }
            };
            let size = window
                .inner_size()
                .ok()
                .zip(window.scale_factor().ok())
                .map(|(s, f)| s.to_logical::<f64>(f))
                .unwrap_or(first);
            let page = |file: &str, hash: Option<&str>| match hash.filter(|h| !h.is_empty()) {
                Some(h) => format!("{file}#{h}"),
                None => file.to_string(),
            };
            // Both fill the window; on Linux they share its box, and the hidden one takes no room.
            for (label, url) in [
                (NEST, page("nest.html", nest)),
                (SETTINGS, page("settings.html", section)),
            ] {
                let builder = WebviewBuilder::new(label, WebviewUrl::App(url.into())).transparent(true);
                // On Linux the window's box sizes its children; auto-resize would pin each to a
                // size request and grow the window, the hidden one showing under the other.
                #[cfg(not(target_os = "linux"))]
                let builder = builder.auto_resize();
                if let Err(e) = window.add_child(builder, LogicalPosition::new(0.0, 0.0), size) {
                    tracing::warn!(label, "a webview of the Nest's window could not be built: {e}");
                    let _ = window.close();
                    return;
                }
            }
            // The switch shows what the window is, once the compositor says so: full screen is
            // settled after the request (on Wayland, a configure later), and the user may leave it
            // with the desktop's own keys.
            let app2 = app.clone();
            window.on_window_event(move |e| {
                use tauri::Emitter;
                match e {
                    tauri::WindowEvent::Resized(_) => {
                        if let Some(w) = app2.get_window(NEST_WINDOW) {
                            // The size the user picks, kept for the next time it opens. The opening
                            // size is no pick; full screen and maximized are the screen's (the
                            // full-screen flag misses the desktop's own keys, so the screen's size
                            // is checked too).
                            if !w.is_fullscreen().unwrap_or(true)
                                && !w.is_maximized().unwrap_or(true)
                                && let (Ok(size), Ok(scale)) = (w.inner_size(), w.scale_factor())
                            {
                                let size = size.to_logical::<f64>(scale);
                                let screen = w
                                    .current_monitor()
                                    .ok()
                                    .flatten()
                                    .map(|m| m.size().to_logical::<f64>(m.scale_factor()));
                                let whole = screen.is_some_and(|s| {
                                    size.width >= s.width - 1.0 && size.height >= s.height - 1.0
                                });
                                let opened = (size.width - first.width).abs() < 1.0
                                    && (size.height - first.height).abs() < 1.0;
                                if !whole
                                    && !opened
                                    && let Ok(mut last) = NEST_LAST.lock()
                                {
                                    *last = Some((size.width.round() as u32, size.height.round() as u32));
                                }
                            }
                            // Both webviews' headers show it.
                            let _ = app2.emit("nest-full", w.is_fullscreen().unwrap_or(false));
                            // Minimized, the Nest is out of sight: the island comes back (ADR 0024).
                            let _ = app2.emit_to(ISLAND, "nest-open", !w.is_minimized().unwrap_or(false));
                        }
                    }
                    tauri::WindowEvent::Destroyed => {
                        let _ = app2.emit_to(ISLAND, "nest-open", false);
                        save_nest_size(&app2);
                    }
                    _ => {}
                }
            });
            window
        }
    };
    if let Some(full) = full {
        let _ = window.set_fullscreen(full);
    }
    // One surface on screen at a time (ADR 0024): the island steps aside while the Nest is open.
    {
        use tauri::Emitter;
        let _ = app.emit_to(ISLAND, "nest-open", true);
    }
    let (shown, hidden) = if nest.is_some() {
        (NEST, SETTINGS)
    } else {
        (SETTINGS, NEST)
    };
    if let Some(section) = section.filter(|_| nest.is_none()) {
        use tauri::Emitter;
        let _ = app.emit_to(SETTINGS, "settings-section", section);
    }
    let _ = window.show();
    let _ = window.set_focus();
    if let Some(w) = app.get_webview(hidden) {
        set_shown(&w, false);
    }
    if let Some(w) = app.get_webview(shown) {
        set_shown(&w, true);
        let _ = w.set_focus();
    }
}

/// The Nest's smallest size, logical pixels: its header and a tab still fit.
const NEST_MIN: (f64, f64) = (480.0, 360.0);
/// The Nest's size on a first open: large, as the prototype's Room is, within the screen.
const NEST_DEFAULT: (f64, f64) = (1180.0, 820.0);
/// The last size the user gave the Nest's window, saved when it closes.
static NEST_LAST: std::sync::Mutex<Option<(u32, u32)>> = std::sync::Mutex::new(None);

/// The screen the Nest opens on, in logical pixels: the island's (GDK on Wayland names no primary
/// monitor), else the first one.
fn nest_screen(app: &AppHandle) -> Option<tauri::LogicalSize<f64>> {
    let island = app
        .get_webview_window(ISLAND)
        .and_then(|w| w.current_monitor().ok().flatten());
    let monitor = island.or_else(|| app.available_monitors().ok()?.into_iter().next())?;
    Some(monitor.size().to_logical::<f64>(monitor.scale_factor()))
}

/// Saves the Nest's size if the user changed it: on its close, and on a quit with it open (a quit
/// destroys no window one by one).
fn save_nest_size(app: &AppHandle) {
    let last = NEST_LAST.lock().ok().and_then(|mut l| l.take());
    if let Some(size) = last {
        let _ = settings::edit(app, |s| s.nest_size = Some(size));
    }
}

/// The size the Nest opens at: the one the user left, else the default at most 85% of the screen;
/// either way within the screen and no smaller than its minimum.
fn nest_first_size(app: &AppHandle) -> tauri::LogicalSize<f64> {
    let saved = app
        .state::<settings::SettingsState>()
        .0
        .lock()
        .ok()
        .and_then(|s| s.nest_size);
    let screen = nest_screen(app);
    let (w, h, share) = match saved {
        Some((w, h)) => (f64::from(w), f64::from(h), 1.0),
        None => (NEST_DEFAULT.0, NEST_DEFAULT.1, 0.85),
    };
    let (w, h) = match screen {
        Some(s) => (w.min(s.width * share), h.min(s.height * share)),
        None => (w, h),
    };
    tauri::LogicalSize::new(w.max(NEST_MIN.0), h.max(NEST_MIN.1))
}

/// Shows or hides one of the Nest window's webviews. On Linux every show of the window is GTK's
/// `show_all`, which would show the hidden one again under the other: it is marked no-show-all.
fn set_shown(webview: &tauri::Webview, on: bool) {
    #[cfg(target_os = "linux")]
    {
        let _ = webview.with_webview(move |p| {
            use gtk::prelude::WidgetExt;
            let w = p.inner();
            w.set_no_show_all(!on);
            if on {
                w.show();
            } else {
                w.hide();
            }
        });
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = if on { webview.show() } else { webview.hide() };
    }
}

/// Whether the Nest is open and in sight: the island asks when it starts (ADR 0024).
#[tauri::command]
fn nest_is_open(app: AppHandle) -> bool {
    app.get_window(NEST_WINDOW)
        .is_some_and(|w| !w.is_minimized().unwrap_or(false))
}

/// Whether the Nest is Full now: its switch asks when it starts.
#[tauri::command]
fn nest_is_full(app: AppHandle) -> bool {
    app.get_window(NEST_WINDOW)
        .and_then(|w| w.is_fullscreen().ok())
        .unwrap_or(false)
}

/// The size switch (ADR 0021): `mini` folds the island to its pill and `quick` opens it, closing
/// the Nest either way; `room` and `full` open the Nest as a window or on the whole screen. `tab`
/// keeps the place: the Nest's tab, or the island's chat when it was the Chat.
#[tauri::command]
fn set_size(app: AppHandle, size: String, tab: Option<String>) {
    size_to(&app, &size, tab.as_deref());
}

pub(crate) fn size_to(app: &AppHandle, size: &str, tab: Option<&str>) {
    use tauri::Emitter;
    match size {
        "mini" | "quick" => {
            if let Some(nest) = app.get_window(NEST_WINDOW) {
                let _ = nest.close();
            }
            if size == "quick" && tab == Some("chat") && settings::zeca(app) {
                let _ = app.emit_to(ISLAND, "open-chat", ());
            } else {
                let _ = app.emit_to(ISLAND, "shortcut", if size == "mini" { "fold" } else { "open" });
            }
        }
        "room" => open_nest(app, tab, Some(false)),
        "full" => open_nest(app, tab, Some(true)),
        _ => {}
    }
}

/// Opens Settings, at `section` when given (a page id of `ui/src/surfaces/settings/main.ts`): the
/// Nest's window, showing Settings' webview (ADR 0023).
pub(crate) fn open_settings(app: &AppHandle, section: Option<&str>) {
    // A page id is a plain word: nothing else reaches the URL or the event.
    let section = section.filter(|s| !s.is_empty() && s.chars().all(|c| c.is_ascii_lowercase()));
    nest_window(app, None, section, None);
}
