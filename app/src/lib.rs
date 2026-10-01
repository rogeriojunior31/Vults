//! The Tauri shell: wires the hook server, the core and the UI together, and executes effects.
//! Domain rules live in `core`; this file only moves data and talks to the OS.

mod installer;
mod runtime;

use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

pub const ISLAND: &str = "island";
const SETTINGS: &str = "settings";

pub fn run() {
    tauri::Builder::default()
        // The socket is removed and rebound on start, so a second instance would steal it.
        .plugin(tauri_plugin_single_instance::init(|app, _, _| open_settings(app)))
        .invoke_handler(tauri::generate_handler![
            runtime::current_view,
            runtime::decide,
            runtime::layout,
            installer::install_status,
            installer::install_preview,
            installer::install_apply,
        ])
        .setup(|app| {
            let handle = app.handle().clone();
            init_island(&handle);
            installer::ensure_hook_exe(&handle);
            tray(&handle)?;
            runtime::start(handle);
            Ok(())
        })
        .run(tauri::generate_context!())
        .unwrap_or_else(|err| eprintln!("{}: {err}", vultures_ai_brand::NAME));
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

    let setup = MenuItem::with_id(app, "setup", "Set up agents…", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&setup, &quit])?;
    let mut tray = TrayIconBuilder::with_id("main")
        .tooltip(vultures_ai_brand::NAME)
        .menu(&menu)
        .on_menu_event(|app, event| match event.id().as_ref() {
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
