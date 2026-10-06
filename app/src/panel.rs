//! Where the island lives: at the top of the screen (*Island*), or by the panel's tray (*Panel*),
//! drawn only when opened. Same window either way (ADR 0008): only its edges move, never its
//! size, and a card still opens it on its own (ADR 0009).

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};

use crate::ISLAND;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Presence {
    #[default]
    Island,
    Panel,
}

/// Which edge the island's shape hangs from inside its window: the top, or (by a panel at the
/// bottom or the side) the bottom.
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Dock {
    Top,
    Bottom,
}

/// What the island needs to know to draw itself there.
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct Place {
    pub presence: Presence,
    pub dock: Dock,
}

/// Space between the island and the panel's corner, in logical pixels.
const MARGIN: i32 = 8;

pub fn presence(app: &AppHandle) -> Presence {
    let state = app.state::<crate::settings::SettingsState>();
    state.0.lock().map(|s| s.presence).unwrap_or_default()
}

/// The side of the panel holding the tray: Plasma says; elsewhere the bottom, the most common.
#[cfg(target_os = "linux")]
fn panel_side() -> vultures_ai_platform::linux::Side {
    vultures_ai_platform::linux::plasma_panel_side().unwrap_or(vultures_ai_platform::linux::Side::Bottom)
}

pub fn place(app: &AppHandle) -> Place {
    let presence = presence(app);
    #[cfg(target_os = "linux")]
    let dock = match (presence, panel_side()) {
        (Presence::Panel, vultures_ai_platform::linux::Side::Top) | (Presence::Island, _) => Dock::Top,
        (Presence::Panel, _) => Dock::Bottom,
    };
    #[cfg(not(target_os = "linux"))]
    let dock = match presence {
        Presence::Island => Dock::Top,
        Presence::Panel => Dock::Bottom,
    };
    Place { presence, dock }
}

/// Moves the island where the setting says, and tells it. Cheap when nothing changed: the
/// panel is read again each time, so a panel moved since is followed. From any thread.
pub fn apply(app: &AppHandle) {
    let place = place(app);
    let _ = app.emit_to(ISLAND, "place", place);
    let Some(win) = app.get_webview_window(ISLAND) else {
        return;
    };
    let _ = app.run_on_main_thread(move || {
        #[cfg(target_os = "linux")]
        if let Ok(gtk) = win.gtk_window() {
            use vultures_ai_platform::linux::{Edges, is_layer, set_edges};
            if is_layer(&gtk) {
                match place.presence {
                    Presence::Island => set_edges(&gtk, Edges::TOP, 0, false),
                    Presence::Panel => set_edges(&gtk, Edges::by_panel(panel_side()), MARGIN, true),
                }
                return;
            }
        }
        // No layer shell: the window is placed by hand, as at start.
        match place.presence {
            Presence::Island => crate::runtime::place_top_center(&win),
            Presence::Panel => place_corner(&win, place.dock),
        }
    });
}

/// Without a compositor to anchor it: the bottom-right (or top-right) corner of its monitor's
/// work area, so the panel stays clear.
fn place_corner(win: &tauri::WebviewWindow, dock: Dock) {
    let (width, height) = crate::runtime::ISLAND_SIZE;
    if let Ok(Some(monitor)) = win.current_monitor() {
        let scale = monitor.scale_factor();
        let area = monitor.work_area();
        let size = area.size.to_logical::<f64>(scale);
        let origin = area.position.to_logical::<f64>(scale);
        let margin = f64::from(MARGIN);
        let x = origin.x + size.width - f64::from(width) - margin;
        let y = match dock {
            Dock::Top => origin.y + margin,
            Dock::Bottom => origin.y + size.height - f64::from(height) - margin,
        };
        let _ = win.set_position(tauri::LogicalPosition::new(x, y));
    }
}

/// The setting: saved, and the island moves at once (no restart).
#[tauri::command]
pub fn set_presence(app: AppHandle, presence: Presence) -> Result<(), String> {
    crate::settings::edit(&app, |s| s.presence = presence)?;
    tracing::info!(?presence, "where the island lives");
    apply(&app);
    Ok(())
}

/// For the island on load.
#[tauri::command]
pub fn island_place(app: AppHandle) -> Place {
    place(&app)
}
