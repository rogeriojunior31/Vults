//! The Tauri shell: wires the hook server, the core and the UI together, and executes effects.
//! Domain rules live in `core`; this file only moves data and talks to the OS.

mod audit;
mod chat;
mod connectors;
mod context;
mod continuation;
#[cfg(target_os = "linux")]
mod dictation;
mod dictation_history;
mod dictation_overlay;
mod dictation_words;
mod doctor;
mod flock_tools;
mod folder;
mod gate;
mod handoff;
mod history;
mod installer;
mod journal;
mod lock;
mod log;
mod mcp;
mod media;
mod meeting_assets;
mod meetings;
mod memory;
mod mentions;
mod notify;
mod open;
mod panel;
mod paths;
mod phone;
mod policies;
mod pricing;
mod profiles;
#[cfg(target_os = "linux")]
mod questions;
pub mod realtime;
mod recall;
mod records;
mod roosts;
mod runtime;
mod session_presence;
mod settings;
mod sleep;
mod specialist_github;
mod spend;
mod traces;
mod tray;
mod usage;
mod voice;
pub mod voice_commands;
mod web_tools;
mod widget;
mod writer;

use std::sync::atomic::{AtomicBool, Ordering};

use tauri::{AppHandle, Manager, WebviewUrl};

pub const ISLAND: &str = "island";
/// The corner widget: built only while the user has it on (`widget.rs`).
pub const WIDGET: &str = "widget";
/// Settings' webview, in the Nest's window (ADR 0021). Commands check the webview's label, never
/// the window's: the Nest's own webview shares the window and shows agents' text.
pub(crate) const SETTINGS: &str = "settings";
/// The Nest's webview: built with its window when opened, gone when it closes (ADR 0021, D6).
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
pub const SURFACES: &[&str] = &[ISLAND, WIDGET, dictation_overlay::LABEL];

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
        dictation_overlay::LABEL => Some(dictation_overlay::layer_spec()),
        _ => None,
    }
}

/// Runs this binary as Zeca's MCP server when asked (`vults --zeca-mcp`, M7): true when it did.
pub fn serve_mcp_if_asked() -> bool {
    let argument = std::env::args().nth(1);
    #[cfg(target_os = "linux")]
    if argument.as_deref() == Some(mcp::QUESTIONS_ARG) {
        mcp::stdio::run(true);
        return true;
    }
    if argument.as_deref() != Some(mcp::ARG) {
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
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .invoke_handler(tauri::generate_handler![
            runtime::current_view,
            #[cfg(target_os = "linux")]
            questions::text_question_status,
            #[cfg(target_os = "linux")]
            questions::question_agents_set,
            #[cfg(target_os = "linux")]
            questions::install::question_install_status,
            #[cfg(target_os = "linux")]
            questions::install::question_install_preview,
            #[cfg(target_os = "linux")]
            questions::install::question_install_apply,
            #[cfg(target_os = "linux")]
            questions::text_question_action,
            #[cfg(target_os = "linux")]
            questions::text_question_depart,
            runtime::step_diff,
            runtime::workspace_diff,
            runtime::decide,
            runtime::layout,
            runtime::surface_keyboard,
            installer::install_status,
            installer::install_preview,
            installer::install_apply,
            chat::chat_send,
            chat::chat_pick_files,
            chat::chat_reset,
            chat::chat_models,
            chat::chat_conversations,
            chat::specialist_chat_conversations,
            chat::specialist_chat_current,
            chat::specialist_chat_draft_set,
            chat::specialist_chat_rename,
            chat::specialist_chat_delete,
            chat::specialist_chat_send,
            chat::specialist_chat_open,
            chat::specialist_chat_reset,
            chat::specialist_chat_choice_set,
            chat::specialist_chat_pick_files,
            chat::specialist_chat_stop,
            chat::specialist_chat_decide,
            roosts::roosts,
            doctor::agents_doctor,
            doctor::dictation::dictation_doctor,
            doctor::dictation::microphone::dictation_mic_start,
            doctor::dictation::microphone::dictation_mic_status,
            doctor::dictation::microphone::dictation_mic_stop,
            roosts::roost_share_preview,
            roosts::roost_share_apply,
            chat::chat_current,
            chat::chat_draft,
            chat::chat_draft_set,
            chat::chat_prepare_zeca,
            nest_connections_place,
            nest_connections_place_set,
            nest_tab_set,
            nest_tab_ready,
            nest_record_query,
            nest_flock_place,
            nest_close,
            chat::chat_open,
            chat::chat_rename,
            chat::chat_delete,
            chat::chat_pick,
            chat::set_chat_pick,
            chat::chat_decide,
            chat::chat_stop,
            chat::api_key_status,
            chat::chat_engines,
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
            profiles::agent_profiles,
            profiles::agent_profile_save,
            profiles::agent_profile_state_set,
            profiles::set_specialist_chat_access,
            specialist_github::set_specialist_github,
            runtime::projects_list,
            runtime::project_set,
            open::editor_found,
            runtime::decide_always,
            runtime::question_answer,
            runtime::question_release,
            runtime::card_skip,
            runtime::card_review,
            runtime::rules_list,
            runtime::rule_remove,
            spend::spend_today,
            spend::budget_set,
            records::nest_history,
            records::nest_audit,
            records::nest_traces,
            records::nest_usage,
            handoff::continue_command,
            policies::policies_list,
            policies::policy_preview,
            policies::policy_accept,
            policies::policy_revoke,
            runtime::alert_open,
            runtime::alert_dismiss,
            runtime::board_open,
            runtime::board_merge,
            connectors::connectors_status,
            connectors::connector_zeca_tools,
            connectors::set_connector_zeca_snapshot,
            connectors::set_connector_zeca_work_item,
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
            settings::set_theme,
            runtime::clear_history,
            runtime::activity,
            history::save_recap_image,
            settings::monitors,
            settings::set_monitor,
            settings::set_now_playing,
            settings::set_codex_usage,
            settings::set_claude_usage,
            settings::set_price_updates,
            pricing::model_price_status,
            pricing::refresh_model_prices,
            settings::set_native_chat,
            settings::set_dictation_insert,
            settings::set_dictation_caption,
            settings::set_dictation_sound,
            settings::set_dictation_gesture,
            settings::set_dictation_commands,
            settings::set_dictation_cleanup,
            settings::set_meeting_summary,
            settings::voice_realtime_settings,
            realtime::routes::voice_conversation_routes,
            realtime::controls::voice_conversation_options,
            realtime::controls::voice_conversation_start,
            realtime::controls::voice_conversation_status,
            realtime::controls::voice_conversation_control,
            settings::set_voice_realtime,
            settings::set_voice_consultation,
            settings::set_voice_codex_consultation,
            settings::set_dictation_style_options,
            settings::dictation_styles,
            settings::set_dictation_app_style,
            settings::set_dictation_processing,
            settings::set_dictation_music,
            settings::dictation_microphones,
            settings::set_dictation_microphone,
            settings::set_chat_access,
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
            voice_commands::preview::voice_command_check,
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
            nest_holds_cards,
            shortcut_keys,
            open_shortcut_settings,
            dictation_status,
            dictation_overlay::dictation_overlay_ready,
            dictation_overlay::dictation_overlay_started,
            meetings::meeting_status,
            meetings::meeting_transcript,
            meetings::archive::meeting_save,
            meetings::archive::meeting_archive_list,
            meetings::archive::meeting_archive_read,
            meetings::archive::meeting_archive_delete,
            meetings::archive::meeting_archive_rename,
            meetings::archive::meeting_archive_export,
            meetings::summary::meeting_summary_status,
            meetings::summary::meeting_summary_begin,
            meetings::summary::meeting_summary_cancel,
            meetings::meeting_prepare,
            meetings::meeting_begin,
            meetings::meeting_end,
            meetings::meeting_recover,
            meeting_assets::meeting_assets_status,
            meeting_assets::meeting_assets_review,
            meeting_assets::meeting_assets_begin,
            meeting_assets::meeting_assets_cancel,
            dictation_history::dictation_history,
            dictation_history::dictation_statistics,
            dictation_words::dictation_words,
            dictation_words::dictation_words_change,
            dictation_words::dictation_try,
            dictation_history::dictation_delete,
            dictation_history::dictation_clear,
            dictation_history::dictation_paste_again,
            dictation_history::set_dictation_keep,
            dictation_permission,
            retry_dictation_permission,
            first_name,
        ])
        .on_window_event(|win, event| {
            if matches!(event, tauri::WindowEvent::Destroyed) && win.label() == dictation_overlay::LABEL {
                dictation_overlay::reset(win.app_handle());
            }
            if let tauri::WindowEvent::CloseRequested { api, .. } = event
                && SURFACES.contains(&win.label())
            {
                // Nothing closes a surface on purpose (Quit exits the app). On Linux this is
                // the compositor closing it because its monitor left: bring it back.
                api.prevent_close();
                revive(win.app_handle(), win.label());
            }
            #[cfg(target_os = "linux")]
            if let tauri::WindowEvent::Focused(focused) = event {
                dictation::permission::focus_changed(win.app_handle(), win.label(), *focused);
            }
            #[cfg(target_os = "linux")]
            if matches!(event, tauri::WindowEvent::Destroyed) {
                dictation::permission::focus_changed(win.app_handle(), win.label(), false);
            }
            if let tauri::WindowEvent::Focused(true) = event
                && !SURFACES.contains(&win.label())
            {
                away(win.app_handle());
                // Back from minimized (Wayland says no state for it): the Nest is in sight again,
                // and the island steps aside (ADR 0021).
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
                        chat::on_drop(win.app_handle(), paths, ISLAND);
                    }
                    _ => {}
                }
            }
        })
        .setup(|app| {
            let handle = app.handle().clone();
            handle.manage(settings::SettingsState(std::sync::Mutex::new(settings::load())));
            handle.manage(std::sync::Arc::new(realtime::State::default()));
            handle.manage(realtime::controls::Controls::default());
            handle.manage(realtime::routes::Routes::default());
            handle.manage(media::MediaState::default());
            handle.manage(dictation_overlay::Ready::default());
            handle.manage(voice::VoiceState::default());
            meetings::init(&handle);
            handle.manage(meeting_assets::AssetsState::default());
            handle.manage(usage::UsageState::default());
            init_surface(&handle, ISLAND);
            #[cfg(target_os = "linux")]
            dictation_overlay::build(&handle);
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
            handle.manage(chat::SpecialistChatState::default());
            chat::clean_inbox();
            tray::start(&handle)?;
            handle.manage(ShortcutKeys::default());
            #[cfg(target_os = "linux")]
            dictation::start(&handle);
            #[cfg(target_os = "linux")]
            listen_shortcuts(&handle);
            notify::start(&handle);
            runtime::start(handle.clone());
            // After the runtime: a lock already on at start reaches its inbox.
            lock::start(&handle);
            // After the runtime too: its first read asks whether the screen is locked.
            pricing::start(&handle);
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
                    realtime::shutdown(app);
                    meetings::shutdown(app);
                    meeting_assets::shutdown(app);
                    #[cfg(target_os = "linux")]
                    {
                        dictation::permission::shutdown(app);
                        dictation::cancel(app, true);
                        dictation::cleanup::shutdown(app);
                    }
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
            if *label == dictation_overlay::LABEL {
                dictation_overlay::place(&win);
            } else if let Ok(gtk) = win.gtk_window() {
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
        // ADR 0021: the Nest's webview shares Settings' window and shows agents' text; the check is
        // on the webview that called.
        let app = tauri::test::mock_app();
        let build = |label: &str| {
            tauri::WebviewWindowBuilder::new(&app, label, WebviewUrl::default())
                .build()
                .unwrap()
        };
        let (nest, settings, island) = (build(NEST), build(SETTINGS), build(ISLAND));
        assert!(settings_page(settings.as_ref()).is_ok());
        assert_eq!(
            settings::set_voice_codex_consultation(app.handle().clone(), settings.as_ref().clone(), true),
            Err(vults_chat::consult_codex::UNAVAILABLE.into()),
            "unqualified authentication must refuse before accessing settings or the ledger"
        );
        assert!(settings_page(nest.as_ref()).is_err());
        assert!(settings_page(island.as_ref()).is_err());
        assert!(open_shortcut_settings(nest.as_ref().clone()).is_err());
        assert!(open_shortcut_settings(island.as_ref().clone()).is_err());
        for view in [nest.as_ref(), island.as_ref()] {
            assert!(settings::voice_realtime_settings(app.handle().clone(), view.clone()).is_err());
            assert!(settings::set_voice_consultation(app.handle().clone(), view.clone(), true).is_err());
            assert!(
                settings::set_voice_codex_consultation(app.handle().clone(), view.clone(), true).is_err()
            );
            assert!(settings::set_meeting_summary(app.handle().clone(), view.clone(), true).is_err());
            assert!(
                settings::set_voice_realtime(
                    app.handle().clone(),
                    view.clone(),
                    true,
                    "1000000".into(),
                    "gpt-realtime-2.1-mini".into()
                )
                .is_err()
            );
            assert!(settings::set_dictation_cleanup(app.handle().clone(), view.clone(), true).is_err());
            assert!(
                settings::set_dictation_style_options(
                    app.handle().clone(),
                    view.clone(),
                    vults_dictation::cleanup::Style::Natural,
                    None,
                    false
                )
                .is_err()
            );
            assert!(settings::dictation_styles(view.clone()).is_err());
            assert!(
                settings::set_dictation_app_style(
                    app.handle().clone(),
                    view.clone(),
                    "Konsole".into(),
                    Some(vults_dictation::cleanup::Style::Prompt)
                )
                .is_err()
            );
            assert!(
                settings::set_dictation_commands(app.handle().clone(), view.clone(), "chat".into(), true)
                    .is_err()
            );
            assert!(
                settings::set_dictation_music(
                    app.handle().clone(),
                    view.clone(),
                    vults_platform::music::Mode::Pause
                )
                .is_err()
            );
            assert!(
                settings::set_dictation_processing(
                    app.handle().clone(),
                    view.clone(),
                    "retryBase".into(),
                    false
                )
                .is_err()
            );
            assert!(
                settings::set_dictation_microphone(
                    app.handle().clone(),
                    view.clone(),
                    Some("USB mic".into())
                )
                .is_err()
            );
            assert!(tauri::async_runtime::block_on(settings::dictation_microphones(view.clone())).is_err());
            assert!(
                settings::set_dictation_gesture(
                    app.handle().clone(),
                    view.clone(),
                    "doubleTap".into(),
                    false
                )
                .is_err()
            );
        }
        assert!(policies::from_settings(settings.as_ref()).is_ok());
        assert!(policies::from_settings(nest.as_ref()).is_err());
        assert!(nest_record_query(settings.as_ref().clone(), "history".into(), Some("test".into())).is_err());
        assert!(nest_record_query(island.as_ref().clone(), "audit".into(), Some("test".into())).is_err());
        assert!(
            nest_record_query(settings.as_ref().clone(), "projects".into(), Some("test".into())).is_err()
        );
        assert!(nest_close(settings.as_ref().clone()).is_err());
        assert!(nest_close(island.as_ref().clone()).is_err());
        assert!(nest_flock_place(settings.as_ref().clone(), None).is_err());
        assert!(nest_flock_place(island.as_ref().clone(), None).is_err());
        let place = FlockPlace {
            query: "Alpha".into(),
            only_needs: true,
            status: None,
            detail: "context".into(),
            change: Some(FlockChange {
                owner: "claude:one".into(),
                step: 1,
                focused: true,
            }),
        };
        assert!(nest_flock_place(nest.as_ref().clone(), Some(place.clone())).is_ok());
        let mut invalid = place;
        invalid.detail = "unknown".into();
        assert!(nest_flock_place(nest.as_ref().clone(), Some(invalid.clone())).is_err());
        assert!(nest_flock_place(settings.as_ref().clone(), Some(invalid)).is_err());
        let kept = nest_flock_place(nest.as_ref().clone(), None).unwrap().unwrap();
        assert_eq!(kept.query, "Alpha");
        assert!(kept.only_needs);
        assert_eq!(kept.detail, "context");
        assert_eq!(kept.change.unwrap().step, 1);
    }
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    #[test]
    fn flock_navigation_is_bounded_and_only_accepts_known_details() {
        let mut place = super::FlockPlace {
            query: "\u{754c}".repeat(4096),
            only_needs: true,
            status: None,
            detail: "context".into(),
            change: None,
        };
        assert!(place.validate().is_ok());
        place.query.push('x');
        assert!(place.validate().is_err());
        place.query.clear();
        for detail in ["session", "diff", "activity", "context", "zeca"] {
            place.detail = detail.into();
            assert!(place.validate().is_ok());
        }
        for status in ["working", "failed", "quiet"] {
            place.status = Some(status.into());
            assert!(place.validate().is_ok());
        }
        place.status = Some("unknown".into());
        assert!(place.validate().is_err());
        place.status = None;
        place.detail = "unknown".into();
        assert!(place.validate().is_err());
        place.detail = "diff".into();
        place.change = Some(super::FlockChange {
            owner: "claude:one".into(),
            step: 0,
            focused: true,
        });
        assert!(place.validate().is_err());
        place.change.as_mut().unwrap().step = 1;
        assert!(place.validate().is_ok());
        place.change.as_mut().unwrap().owner = "x".repeat(4097);
        assert!(place.validate().is_err());
        place.change.as_mut().unwrap().owner.clear();
        assert!(place.validate().is_err());
        let old: super::FlockPlace =
            serde_json::from_str(r#"{"query":"Alpha","onlyNeeds":false,"detail":"diff"}"#).unwrap();
        assert!(old.change.is_none());
    }

    #[test]
    fn record_searches_survive_recreation_and_remain_separate_and_bounded() {
        let mut searches = super::RecordQueries::default();
        assert_eq!(searches.query("history", None).unwrap(), "");
        searches.query("history", Some(" Alpha ".into())).unwrap();
        searches.query("audit", Some("git status".into())).unwrap();
        searches.query("projects", Some("feature/login".into())).unwrap();
        assert_eq!(searches.query("history", None).unwrap(), " Alpha ");
        assert_eq!(searches.query("audit", None).unwrap(), "git status");
        assert_eq!(searches.query("projects", None).unwrap(), "feature/login");
        assert!(searches.query("cost", Some("bad".into())).is_err());
        searches.query("history", Some("\u{754c}".repeat(4096))).unwrap();
        assert!(searches.query("history", Some("x".repeat(4097))).is_err());
        assert_eq!(searches.query("history", None).unwrap(), "\u{754c}".repeat(4096));
        searches.query("history", Some(String::new())).unwrap();
        assert_eq!(searches.query("history", None).unwrap(), "");
        assert_eq!(searches.query("audit", None).unwrap(), "git status");
        assert_eq!(searches.query("projects", None).unwrap(), "feature/login");
        assert!(searches.query("projects", Some("x".repeat(4097))).is_err());
        assert_eq!(searches.query("projects", None).unwrap(), "feature/login");
        searches.query("projects", Some(String::new())).unwrap();
        assert_eq!(searches.query("projects", None).unwrap(), "");
        assert_eq!(searches.query("audit", None).unwrap(), "git status");
    }

    #[test]
    fn reopening_nest_keeps_its_tab_but_explicit_routes_and_zeca_off_win() {
        assert_eq!(super::restored_tab(None, Some("audit"), true), "audit");
        assert_eq!(
            super::restored_tab(Some("connections"), Some("audit"), true),
            "connections"
        );
        assert_eq!(super::restored_tab(None, Some("chat"), false), "flock");
        assert_eq!(super::restored_tab(None, Some("dictation"), false), "dictation");
        assert_eq!(
            super::restored_tab(Some("dictation"), Some("settings"), true),
            "dictation"
        );
        assert_eq!(
            super::restored_tab(None, Some("flock-profiles"), false),
            "flock-profiles"
        );
        assert_eq!(
            super::restored_tab(None, Some("dictation-statistics"), false),
            "dictation-statistics"
        );
        for tab in [
            "dictation-history",
            "dictation-commands",
            "dictation-dictionary",
            "dictation-snippets",
        ] {
            assert_eq!(super::restored_tab(Some(tab), None, false), tab);
        }
        assert_eq!(super::restored_tab(Some("<script>"), Some("cost"), true), "cost");
        assert_eq!(super::restored_tab(None, None, true), "flock");
        assert_eq!(super::restored_tab(None, Some("tasks"), true), "flock");
    }

    #[test]
    fn only_ready_nest_consumes_the_latest_request_without_acknowledging_a_tab() {
        let app = tauri::test::mock_builder()
            .build(tauri::test::mock_context(tauri::test::noop_assets()))
            .unwrap();
        let window = tauri::WindowBuilder::new(&app, "navigation-test")
            .build()
            .unwrap();
        let nest = window
            .add_child(
                tauri::WebviewBuilder::new(super::NEST, tauri::WebviewUrl::App("nest.html".into())),
                tauri::LogicalPosition::new(0., 0.),
                tauri::LogicalSize::new(480., 360.),
            )
            .unwrap();
        let settings = window
            .add_child(
                tauri::WebviewBuilder::new(super::SETTINGS, tauri::WebviewUrl::App("settings.html".into())),
                tauri::LogicalPosition::new(0., 0.),
                tauri::LogicalSize::new(480., 360.),
            )
            .unwrap();
        let acknowledged = super::NEST_TAB.lock().unwrap().clone();
        *super::NEST_REQUEST.lock().unwrap() = Some("chat".into());
        *super::NEST_REQUEST.lock().unwrap() = Some("dictation-conversation".into());
        assert!(super::nest_tab_ready(settings.clone()).is_err());
        assert_eq!(
            super::nest_tab_ready(nest.clone()).unwrap().as_deref(),
            Some("dictation-conversation")
        );
        assert!(super::nest_tab_ready(nest.clone()).unwrap().is_none());
        assert_eq!(
            *super::NEST_TAB.lock().unwrap(),
            acknowledged,
            "A request does not grant voice-page authority"
        );
    }

    #[test]
    fn minimized_nest_gives_back_approval_shortcuts_until_focused_again() {
        let mut state = (false, false);
        assert!(!super::update_nest_focus(&mut state, None, Some(true)));
        assert!(super::update_nest_focus(&mut state, Some(true), None));
        // Minimization revokes ownership without waiting for a focus-loss event.
        assert!(!super::update_nest_focus(&mut state, Some(false), None));
        assert!(!super::update_nest_focus(&mut state, None, Some(true)));
        assert!(super::update_nest_focus(&mut state, Some(true), None));
        // The focused native window may show Settings, which cannot answer a card.
        assert!(!super::update_nest_focus(&mut state, None, Some(false)));
        assert!(!super::update_nest_focus(&mut state, Some(true), None));
        assert!(super::update_nest_focus(&mut state, None, Some(true)));
        assert!(!super::update_nest_focus(&mut state, Some(false), Some(false)));
    }

    #[test]
    fn island_return_waits_for_saved_close_and_uses_the_final_route_once() {
        use tauri::Listener;
        let app = tauri::test::mock_app();
        let events = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        for name in ["open-chat", "open-now", "shortcut"] {
            let events = events.clone();
            app.listen_any(name, move |event| {
                events.lock().unwrap().push((name, event.payload().to_string()))
            });
        }
        let pending = std::sync::Mutex::new(None);
        super::finish_island_return(app.handle(), &pending, Some("chat"), true);
        assert!(events.lock().unwrap().is_empty());
        for (size, asked, last, zeca, expected) in [
            ("quick", Some("chat"), Some("chat"), true, ("open-chat", "null")),
            (
                "quick",
                Some("activity"),
                Some("activity"),
                true,
                ("open-now", "null"),
            ),
            (
                "quick",
                Some("connections"),
                Some("connections"),
                false,
                ("open-now", "null"),
            ),
            (
                "quick",
                Some("chat"),
                Some("audit"),
                true,
                ("shortcut", "\"open\""),
            ),
            (
                "quick",
                Some("settings"),
                Some("chat"),
                true,
                ("shortcut", "\"open\""),
            ),
            ("quick", None, Some("chat"), true, ("shortcut", "\"open\"")),
            (
                "quick",
                Some("chat"),
                Some("chat"),
                false,
                ("shortcut", "\"open\""),
            ),
            ("mini", Some("chat"), Some("chat"), true, ("shortcut", "\"fold\"")),
        ] {
            *pending.lock().unwrap() = Some((size.into(), asked.map(str::to_owned)));
            let before = events.lock().unwrap().len();
            super::finish_island_return(app.handle(), &pending, last, zeca);
            assert_eq!(
                events.lock().unwrap().last().unwrap(),
                &(expected.0, expected.1.into())
            );
            super::finish_island_return(app.handle(), &pending, last, zeca);
            assert_eq!(events.lock().unwrap().len(), before + 1);
        }
        // A failed close reopens the Nest and cancels its pending return.
        *pending.lock().unwrap() = Some(("quick".into(), Some("chat".into())));
        *pending.lock().unwrap() = None;
        let before = events.lock().unwrap().len();
        super::finish_island_return(app.handle(), &pending, Some("chat"), true);
        assert_eq!(events.lock().unwrap().len(), before);
    }

    #[test]
    fn connection_navigation_roundtrip_retains_only_navigation() {
        let input = serde_json::json!({
            "chosen": "github", "picked": "pr:me/vults#56", "repo": "me/vults",
            "tab": "prs", "detailOpen": true,
        });
        let place: super::ConnectionsPlace = serde_json::from_value(input.clone()).unwrap();
        place.validate().unwrap();
        assert!(!place.zeca);
        let mut expected = input;
        expected["zeca"] = serde_json::json!(false);
        assert_eq!(serde_json::to_value(&place).unwrap(), expected);
        let mut invalid = place;
        invalid.tab = Some("merge".into());
        assert!(invalid.validate().is_err());
        invalid.tab = None;
        invalid.repo = Some("x".repeat(4097));
        assert!(invalid.validate().is_err());
    }

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

#[tauri::command]
fn open_shortcut_settings<R: tauri::Runtime>(window: tauri::Webview<R>) -> Result<(), String> {
    settings_page(&window)?;
    #[cfg(target_os = "linux")]
    return vults_platform::shortcuts::open_settings();
    #[cfg(not(target_os = "linux"))]
    Err("KDE Shortcuts is unavailable on this platform.".into())
}

/// The desktop's global shortcuts (`platform::shortcuts::SHORTCUTS`). Next and previous go to
/// core; the rest go to the island, which decides whether a card is on screen to answer (a press
/// with nothing waiting does nothing). Allow and Deny go to the Nest while the user is in it.
#[cfg(target_os = "linux")]
fn listen_shortcuts(app: &AppHandle) {
    use tauri::Emitter;
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let emit = app.clone();
        let keys = app.clone();
        let words = app.clone();
        let result = vults_platform::shortcuts::listen(
            vults_brand::BUNDLE_ID,
            move |id| vults_core::i18n::dictation_shortcut(settings::lang(&words), id).map(str::to_owned),
            move |bound| {
                questions::cancel_capture(&keys);
                // Rebinding may not deliver the old key's release.
                if let Some(state) = keys.try_state::<dictation::State>() {
                    state.shortcuts_changed();
                }
                voice::cancel(&keys);
                runtime::cancel_voice_commands(&keys);
                for surface in [ISLAND, NEST] {
                    let _ = keys.emit_to(surface, "shortcut", "talk-cancel");
                }
                tracing::info!(count = bound.len(), "global shortcuts bound");
                let map: std::collections::BTreeMap<_, _> = bound.into_iter().collect();
                if let Ok(mut kept) = keys.state::<ShortcutKeys>().0.lock() {
                    kept.clone_from(&map);
                }
                // The island and the Nest show them on their cards.
                let _ = keys.emit("shortcut-keys", map);
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
                if id == "talk-cancel" {
                    if down {
                        dictation::key(&emit, id, true);
                        voice::cancel(&emit);
                        for surface in [ISLAND, NEST] {
                            let _ = emit.emit_to(surface, "shortcut", "talk-cancel");
                        }
                    }
                    return;
                }
                if matches!(id, "talk" | "talk-again" | "talk-command") {
                    dictation::key(&emit, id, down);
                    return;
                }
                // Ctrl+Alt+Space is also Open: Space-lock must keep the recording's focus.
                if id == "open" && dictation::holds_space(&emit) {
                    return;
                }
                if let Some(event) = vults_platform::shortcuts::island_event(id, down) {
                    let to = if matches!(id, "allow" | "deny") && NEST_HOLDS.load(Ordering::Relaxed) {
                        NEST
                    } else {
                        ISLAND
                    };
                    let _ = emit.emit_to(to, "shortcut", event);
                }
            },
        )
        .await;
        dictation::cancel(&app, true);
        voice::cancel(&app);
        if let Ok(mut keys) = app.state::<ShortcutKeys>().0.lock() {
            keys.clear();
        }
        let _ = app.emit(
            "shortcut-keys",
            std::collections::BTreeMap::<String, String>::new(),
        );
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
    if tab == Some(SETTINGS) {
        nest_window(app, None, None, full);
        return;
    }
    // An existing webview already owns its current tab; restore only after recreation.
    if tab.is_none() && app.get_webview(NEST).is_some() {
        nest_window(app, Some(""), None, full);
        return;
    }
    let last = NEST_TAB.lock().ok().and_then(|t| t.clone());
    let tab = restored_tab(tab, last.as_deref(), settings::zeca(app));
    if let Ok(mut pending) = NEST_REQUEST.lock() {
        *pending = Some(tab.to_owned());
    }
    if app.get_webview(NEST).is_some() {
        let _ = app.emit_to(NEST, "nest-tab", tab);
    }
    nest_window(app, Some(tab), None, full);
}

static NEST_TAB: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);
// Requested presentation is separate from the renderer's acknowledged tab and capture authority.
static NEST_REQUEST: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);
#[derive(Default)]
struct RecordQueries {
    history: String,
    audit: String,
    projects: String,
}

impl RecordQueries {
    fn query(&mut self, tab: &str, query: Option<String>) -> Result<String, String> {
        let kept = match tab {
            "history" => &mut self.history,
            "audit" => &mut self.audit,
            "projects" => &mut self.projects,
            _ => return Err("Unknown record search.".into()),
        };
        if let Some(query) = query {
            if query.chars().count() > 4096 {
                return Err("Record search is too long.".into());
            }
            *kept = query;
        }
        Ok(kept.clone())
    }
}

static RECORD_QUERIES: std::sync::Mutex<RecordQueries> = std::sync::Mutex::new(RecordQueries {
    history: String::new(),
    audit: String::new(),
    projects: String::new(),
});

/// Navigation only, kept until the app exits; no records or permissions are changed.
#[tauri::command]
fn nest_record_query<R: tauri::Runtime>(
    window: tauri::Webview<R>,
    tab: String,
    query: Option<String>,
) -> Result<String, String> {
    if window.label() != NEST {
        return Err("Record searches belong to the Nest.".into());
    }
    RECORD_QUERIES
        .lock()
        .map_err(|e| e.to_string())?
        .query(&tab, query)
}

/// Presentation only; core retains session selection and permission ownership.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct FlockChange {
    owner: String,
    step: u32,
    focused: bool,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct FlockPlace {
    query: String,
    only_needs: bool,
    #[serde(default)]
    status: Option<String>,
    detail: String,
    #[serde(default)]
    change: Option<FlockChange>,
}

impl FlockPlace {
    fn validate(&self) -> Result<(), String> {
        if self.query.chars().count() > 4096
            || !matches!(
                self.detail.as_str(),
                "session" | "diff" | "activity" | "context" | "zeca"
            )
            || self
                .status
                .as_deref()
                .is_some_and(|s| !matches!(s, "working" | "failed" | "quiet"))
            || self
                .change
                .as_ref()
                .is_some_and(|c| c.owner.is_empty() || c.owner.chars().count() > 4096 || c.step == 0)
        {
            return Err("Invalid flock navigation state.".into());
        }
        Ok(())
    }
}

static FLOCK_PLACE: std::sync::Mutex<Option<FlockPlace>> = std::sync::Mutex::new(None);

#[tauri::command]
fn nest_flock_place<R: tauri::Runtime>(
    window: tauri::Webview<R>,
    place: Option<FlockPlace>,
) -> Result<Option<FlockPlace>, String> {
    if window.label() != NEST {
        return Err("Flock navigation belongs to the Nest.".into());
    }
    if let Some(ref place) = place {
        place.validate()?;
    }
    let mut kept = FLOCK_PLACE.lock().map_err(|e| e.to_string())?;
    if let Some(place) = place {
        *kept = Some(place);
    }
    Ok(kept.clone())
}

/// Only the Nest may finish its saved close; never accept a caller-supplied window target.
#[tauri::command]
fn nest_close<R: tauri::Runtime>(window: tauri::Webview<R>) -> Result<(), String> {
    if window.label() != NEST {
        return Err("Only the Nest may finish closing its window.".into());
    }
    if let Some(nest) = window.app_handle().get_window(NEST_WINDOW) {
        // Destroy skips CloseRequested, which would ask the renderer to save a second time.
        nest.destroy().map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn valid_nest_tab(tab: &str) -> bool {
    if tab.starts_with("connection:") {
        let parts: Vec<_> = tab.split(':').collect();
        return tab.len() <= 4096 && parts.len() == 3 && parts.iter().all(|p| !p.is_empty());
    }
    matches!(
        tab,
        "flock"
            | "flock-profiles"
            | "memory"
            | "chat"
            | "activity"
            | "connections"
            | "dictation"
            | "dictation-history"
            | "dictation-commands"
            | "dictation-dictionary"
            | "dictation-snippets"
            | "dictation-meetings"
            | "dictation-conversation"
            | "dictation-statistics"
            | "week"
            | "projects"
            | "history"
            | "cost"
            | "traces"
            | "audit"
    )
}

fn restored_tab<'a>(asked: Option<&'a str>, last: Option<&'a str>, zeca: bool) -> &'a str {
    let tab = asked
        .filter(|t| valid_nest_tab(t))
        .or_else(|| last.filter(|t| valid_nest_tab(t)))
        .unwrap_or("flock");
    if matches!(tab, "chat" | "memory") && !zeca {
        "flock"
    } else {
        tab
    }
}

#[tauri::command]
fn nest_tab_ready<R: tauri::Runtime>(window: tauri::Webview<R>) -> Result<Option<String>, String> {
    if window.label() != NEST {
        return Err("Only the Nest may receive its pending navigation.".into());
    }
    Ok(NEST_REQUEST.lock().map_err(|e| e.to_string())?.take())
}

#[tauri::command]
fn nest_tab_set(window: tauri::Webview, tab: String) -> Result<(), String> {
    if window.label() != NEST || !valid_nest_tab(&tab) {
        return Err("Invalid Nest navigation state.".into());
    }
    #[cfg(target_os = "linux")]
    questions::depart(window.app_handle(), NEST);
    realtime::controls::depart(window.app_handle());
    *NEST_TAB.lock().map_err(|e| e.to_string())? = Some(tab);
    Ok(())
}

/// Brings the Nest's window forward, building it the first time, and shows one of its two
/// webviews: the Nest's (`nest`: Some, its tab or "" for where it was) or Settings' (`nest`: None,
/// at `section` when given). Only the shown one is in sight and takes the keys.
fn nest_window(app: &AppHandle, nest: Option<&str>, section: Option<&str>, full: Option<bool>) {
    #[cfg(target_os = "linux")]
    {
        questions::depart(app, ISLAND);
        questions::depart(app, NEST);
    }
    realtime::controls::depart(app);
    use tauri::{LogicalPosition, WebviewBuilder, window::WindowBuilder};
    // Reopening after a failed saved close cancels its earlier return request.
    if let Ok(mut pending) = NEST_RETURN.lock() {
        *pending = None;
    }
    away(app);
    let window = match app.get_window(NEST_WINDOW) {
        Some(w) => w,
        None => {
            if nest.is_none()
                && let Ok(mut pending) = NEST_REQUEST.lock()
            {
                *pending = None;
            }
            // The island grown larger (ADR 0021): no system title bar, the page's own frame with
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
                            let minimized = w.is_minimized().unwrap_or(false);
                            // Revoke shortcut ownership before the separate focus-loss event.
                            if minimized {
                                nest_holds(&app2, Some(false), None);
                            }
                            // The size the user picks, kept for the next time it opens. The opening
                            // size is no pick; full screen and maximized are the screen's (the
                            // full-screen flag misses the desktop's own keys, so the screen's size
                            // is checked too).
                            if !minimized
                                && !w.is_fullscreen().unwrap_or(true)
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
                            // Minimized, the Nest is out of sight: the island comes back (ADR 0021).
                            if minimized {
                                #[cfg(target_os = "linux")]
                                questions::depart(&app2, NEST);
                                meetings::cancel(&app2);
                                realtime::controls::depart(&app2);
                            }
                            let _ = app2.emit_to(ISLAND, "nest-open", !minimized);
                        }
                    }
                    tauri::WindowEvent::Focused(focused) => nest_holds(&app2, Some(*focused), None),
                    tauri::WindowEvent::Destroyed => {
                        #[cfg(target_os = "linux")]
                        questions::depart(&app2, NEST);
                        meetings::cancel(&app2);
                        realtime::controls::depart(&app2);
                        let _ = app2.emit_to(ISLAND, "nest-open", false);
                        nest_holds(&app2, Some(false), Some(false));
                        save_nest_size(&app2);
                        let last = NEST_TAB.lock().ok().and_then(|tab| tab.clone());
                        finish_island_return(&app2, &NEST_RETURN, last.as_deref(), settings::zeca(&app2));
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
    // One surface on screen at a time (ADR 0021): the island steps aside while the Nest is open.
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
    let _ = window.unminimize();
    let _ = window.show();
    let _ = window.set_focus();
    if hidden == SETTINGS {
        doctor::dictation::stop_mic(app);
    }
    if let Some(w) = app.get_webview(hidden) {
        set_shown(&w, false);
    }
    if let Some(w) = app.get_webview(shown) {
        set_shown(&w, true);
        let _ = w.set_focus();
    }
    nest_holds(app, None, Some(nest.is_some()));
}

/// Whether the Nest's window has the focus, and whether it shows the Nest's webview (not
/// Settings'). One lock: two events read and written apart could leave the island aside with
/// nobody in the Nest, and a card unseen.
static NEST_IN: std::sync::Mutex<(bool, bool)> = std::sync::Mutex::new((false, false));
/// Both: the user is in the Nest.
static NEST_HOLDS: AtomicBool = AtomicBool::new(false);

/// Keeps track of whether the user is in the Nest; the island hears each change. There, the Nest
/// answers the cards and takes Allow / Deny's keys, and the island stays aside; anywhere else a
/// card opens the island (ADR 0021).
fn nest_holds(app: &AppHandle, focused: Option<bool>, shown: Option<bool>) {
    if focused == Some(false) {
        doctor::dictation::stop_mic(app);
    }
    use tauri::Emitter;
    let Ok(mut state) = NEST_IN.lock() else {
        return;
    };
    let holds = update_nest_focus(&mut state, focused, shown);
    if NEST_HOLDS.swap(holds, Ordering::Relaxed) != holds {
        let _ = app.emit_to(ISLAND, "nest-holds", holds);
    }
}

/// A window's focus and the shown webview arrive separately; only both own approval shortcuts.
fn update_nest_focus(state: &mut (bool, bool), focused: Option<bool>, shown: Option<bool>) -> bool {
    if let Some(f) = focused {
        state.0 = f;
    }
    if let Some(s) = shown {
        state.1 = s;
    }
    state.0 && state.1
}

/// Whether the user is in the Nest now, for the island when it starts.
#[tauri::command]
fn nest_holds_cards() -> bool {
    NEST_HOLDS.load(Ordering::Relaxed)
}

/// The Nest's smallest size, logical pixels: its header and a tab still fit.
const NEST_MIN: (f64, f64) = (480.0, 360.0);
/// The Nest's size on a first open: large, as the prototype's Room is, within the screen.
const NEST_DEFAULT: (f64, f64) = (1180.0, 820.0);
/// The last size the user gave the Nest's window, saved when it closes.
static NEST_LAST: std::sync::Mutex<Option<(u32, u32)>> = std::sync::Mutex::new(None);

/// Navigation only: no pending merge or consent survives a destroyed webview.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ConnectionsPlace {
    chosen: Option<String>,
    picked: Option<String>,
    repo: Option<String>,
    tab: Option<String>,
    detail_open: bool,
    #[serde(default)]
    zeca: bool,
}

impl ConnectionsPlace {
    fn validate(&self) -> Result<(), String> {
        if [&self.chosen, &self.picked, &self.repo]
            .iter()
            .any(|s| s.as_ref().is_some_and(|s| s.len() > 4096))
            || self
                .tab
                .as_deref()
                .is_some_and(|t| !matches!(t, "inbox" | "prs" | "runs" | "rels"))
        {
            return Err("Invalid connection navigation state.".into());
        }
        Ok(())
    }
}

static CONNECTIONS_PLACE: std::sync::Mutex<Option<ConnectionsPlace>> = std::sync::Mutex::new(None);

#[tauri::command]
fn nest_connections_place() -> Result<Option<ConnectionsPlace>, String> {
    CONNECTIONS_PLACE
        .lock()
        .map(|p| p.clone())
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn nest_connections_place_set(window: tauri::Webview, place: ConnectionsPlace) -> Result<(), String> {
    if window.label() != NEST {
        return Err("Connection navigation belongs to the Nest.".into());
    }
    place.validate()?;
    *CONNECTIONS_PLACE.lock().map_err(|e| e.to_string())? = Some(place);
    Ok(())
}

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

/// Whether the Nest is open and in sight: the island asks when it starts (ADR 0021).
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

/// Quick/Mini becomes visible only after the Nest completes its saved close.
static NEST_RETURN: std::sync::Mutex<Option<(String, Option<String>)>> = std::sync::Mutex::new(None);

fn show_island_size<R: tauri::Runtime>(app: &AppHandle<R>, size: &str, tab: Option<&str>, zeca: bool) {
    use tauri::Emitter;
    if size == "quick" && tab == Some("chat") && zeca {
        let _ = app.emit_to(ISLAND, "open-chat", ());
    } else if size == "quick" && matches!(tab, Some("now" | "activity" | "connections")) {
        let _ = app.emit_to(ISLAND, "open-now", ());
    } else {
        let _ = app.emit_to(ISLAND, "shortcut", if size == "mini" { "fold" } else { "open" });
    }
}

fn finish_island_return<R: tauri::Runtime>(
    app: &AppHandle<R>,
    pending: &std::sync::Mutex<Option<(String, Option<String>)>>,
    last: Option<&str>,
    zeca: bool,
) {
    let Some((size, tab)) = pending.lock().ok().and_then(|mut value| value.take()) else {
        return;
    };
    // A second close-time save may have a newer tab; Settings and explicit global routes stay explicit.
    let tab = if tab.as_deref().is_some_and(valid_nest_tab) {
        last.or(tab.as_deref())
    } else {
        tab.as_deref()
    };
    show_island_size(app, &size, tab, zeca);
}

pub(crate) fn size_to(app: &AppHandle, size: &str, tab: Option<&str>) {
    match size {
        "mini" | "quick" => {
            if let Some(nest) = app.get_window(NEST_WINDOW) {
                if let Ok(mut pending) = NEST_RETURN.lock() {
                    *pending = Some((size.into(), tab.map(str::to_owned)));
                }
                if nest.close().is_err()
                    && let Ok(mut pending) = NEST_RETURN.lock()
                {
                    *pending = None;
                }
                return;
            }
            show_island_size(app, size, tab, settings::zeca(app));
        }
        "room" => open_nest(app, tab, Some(false)),
        "full" => open_nest(app, tab, Some(true)),
        _ => {}
    }
}

/// Opens Settings, at `section` when given (a page id of `ui/src/surfaces/settings/main.ts`): the
/// Nest's window, showing Settings' webview (ADR 0021).
pub(crate) fn open_settings(app: &AppHandle, section: Option<&str>) {
    // A page id is a plain word: nothing else reaches the URL or the event.
    let section = section.filter(|s| !s.is_empty() && s.chars().all(|c| c.is_ascii_lowercase()));
    nest_window(app, None, section, None);
}

#[tauri::command]
fn dictation_permission(app: AppHandle) -> String {
    #[cfg(target_os = "linux")]
    return dictation::permission::status(app);
    #[cfg(not(target_os = "linux"))]
    {
        let _ = app;
        "unavailable".into()
    }
}

#[tauri::command]
async fn retry_dictation_permission(app: AppHandle, window: tauri::Webview) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    return dictation::permission::retry(app, window).await;
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (app, window);
        Err("Dictation insertion is unavailable.".into())
    }
}

#[tauri::command]
fn dictation_status(app: AppHandle) -> Option<serde_json::Value> {
    #[cfg(target_os = "linux")]
    return app
        .try_state::<dictation::State>()
        .and_then(|s| s.status())
        .and_then(|status| serde_json::to_value(status).ok());
    #[cfg(not(target_os = "linux"))]
    {
        let _ = app;
        None
    }
}

#[cfg(target_os = "linux")]
fn question_surface(app: &AppHandle) -> &'static str {
    let nest = NEST_IN.lock().is_ok_and(|state| state.1);
    if nest && question_visible(app, NEST) {
        NEST
    } else if question_visible(app, ISLAND) {
        ISLAND
    } else {
        ""
    }
}
#[cfg(target_os = "linux")]
fn question_visible(app: &AppHandle, surface: &str) -> bool {
    match surface {
        NEST => {
            NEST_IN.lock().is_ok_and(|state| state.1)
                && app.get_window(NEST_WINDOW).is_some_and(|window| {
                    window.is_visible().unwrap_or(false) && !window.is_minimized().unwrap_or(true)
                })
        }
        ISLAND => {
            !app.get_window(NEST_WINDOW).is_some_and(|window| {
                window.is_visible().unwrap_or(false) && !window.is_minimized().unwrap_or(true)
            }) && app.get_webview(ISLAND).is_some()
                && app
                    .state::<settings::SettingsState>()
                    .0
                    .lock()
                    .is_ok_and(|s| s.presence != vults_core::Presence::Paused)
        }
        _ => false,
    }
}
