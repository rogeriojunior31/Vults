//! The corner widget (ADR 0008): a second layer surface, fixed in the screen corner the user
//! picks, with the birds that matter most and how many work or need you. It lives only while it
//! is chosen (one webview at rest otherwise): built when the setting turns it on, destroyed when
//! it turns it off. It draws from the island's view and never answers a card: a click only brings
//! the island up.

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

use crate::WIDGET;

/// Logical size, fixed for the surface's life; the page draws exactly this
/// (`ui/src/surfaces/widget/render.ts`).
pub const WIDGET_SIZE: (i32, i32) = (212, 44);

/// Distance from the corner's two edges, in logical pixels.
const MARGIN: i32 = 12;

/// The screen corner the widget sits in.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Corner {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

impl Corner {
    fn top(self) -> bool {
        matches!(self, Corner::TopLeft | Corner::TopRight)
    }

    fn left(self) -> bool {
        matches!(self, Corner::TopLeft | Corner::BottomLeft)
    }

    /// The two screen edges that make the corner.
    #[cfg(target_os = "linux")]
    pub fn edges(self) -> vults_platform::linux::Edges {
        vults_platform::linux::Edges {
            top: self.top(),
            bottom: !self.top(),
            left: self.left(),
            right: !self.left(),
        }
    }
}

/// How the widget sits on the screen in `corner`.
#[cfg(target_os = "linux")]
pub fn layer_spec(corner: Corner) -> vults_platform::linux::LayerSpec {
    let (width, height) = WIDGET_SIZE;
    vults_platform::linux::LayerSpec {
        namespace: format!("{}-widget", vults_brand::SLUG),
        width,
        height,
        edges: corner.edges(),
        margin: MARGIN,
        keyboard: false,
        // Under full-screen windows: a video or a game hides it, as it hides a panel.
        overlay: false,
    }
}

pub fn corner(app: &AppHandle) -> Option<Corner> {
    let state = app.state::<crate::settings::SettingsState>();
    state.0.lock().ok().and_then(|s| s.widget)
}

/// Builds, moves or removes the widget as the setting says. From any thread.
pub fn apply(app: &AppHandle) {
    let corner = corner(app);
    let app = app.clone();
    let main = app.clone();
    let _ = main.run_on_main_thread(move || match (corner, app.get_webview_window(WIDGET)) {
        (None, Some(win)) => {
            tracing::info!("corner widget off");
            // Not `close`: surfaces refuse it (a closed surface is a lost monitor, brought back).
            let _ = win.destroy();
        }
        (Some(corner), Some(win)) => place(&win, corner),
        (Some(corner), None) => build(&app, corner),
        (None, None) => {}
    });
}

/// Main thread. Hidden until it is a layer surface: the role is chosen before the first map.
fn build(app: &AppHandle, corner: Corner) {
    let (width, height) = WIDGET_SIZE;
    let built = WebviewWindowBuilder::new(app, WIDGET, WebviewUrl::App("widget.html".into()))
        .title(format!("{} widget", vults_brand::NAME))
        .inner_size(f64::from(width), f64::from(height))
        .resizable(false)
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .focused(false)
        .focusable(false)
        .visible_on_all_workspaces(true)
        .visible(false)
        .maximizable(false)
        .minimizable(false)
        .closable(false)
        .build();
    let win = match built {
        Ok(win) => win,
        Err(e) => {
            tracing::warn!("corner widget not built: {e}");
            return;
        }
    };
    tracing::info!(?corner, "corner widget on");
    #[cfg(target_os = "linux")]
    if let Ok(gtk) = win.gtk_window() {
        use vults_platform::linux::{init_layer, set_edges};
        let monitor = crate::settings::monitor(app);
        if init_layer(&gtk, WIDGET, &layer_spec(corner), monitor.as_deref()) {
            // Clear of the panels' space: a corner widget never sits under a panel.
            set_edges(&gtk, corner.edges(), MARGIN, true);
            return;
        }
    }
    place_by_hand(&win, corner);
    let _ = win.show();
}

/// Main thread. Another corner: the surface moves without a re-map (its size stays).
fn place(win: &tauri::WebviewWindow, corner: Corner) {
    #[cfg(target_os = "linux")]
    if let Ok(gtk) = win.gtk_window()
        && vults_platform::linux::is_layer(&gtk)
    {
        vults_platform::linux::set_edges(&gtk, corner.edges(), MARGIN, true);
        return;
    }
    place_by_hand(win, corner);
}

/// Without a compositor to anchor it: the corner of its monitor's work area.
fn place_by_hand(win: &tauri::WebviewWindow, corner: Corner) {
    let (width, height) = WIDGET_SIZE;
    if let Ok(Some(monitor)) = win.current_monitor() {
        let scale = monitor.scale_factor();
        let area = monitor.work_area();
        let size = area.size.to_logical::<f64>(scale);
        let origin = area.position.to_logical::<f64>(scale);
        let margin = f64::from(MARGIN);
        let x = if corner.left() {
            origin.x + margin
        } else {
            origin.x + size.width - f64::from(width) - margin
        };
        let y = if corner.top() {
            origin.y + margin
        } else {
            origin.y + size.height - f64::from(height) - margin
        };
        let _ = win.set_position(tauri::LogicalPosition::new(x, y));
    }
}

/// Settings: a corner, or `None` for no widget. It comes, moves or goes at once.
#[tauri::command]
pub fn set_widget(app: AppHandle, corner: Option<Corner>) -> Result<(), String> {
    crate::settings::edit(&app, |s| s.widget = corner)?;
    apply(&app);
    let _ = app.emit("settings", serde_json::json!({ "widget": corner }));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn corners_read_as_settings_write_them() {
        for (word, corner) in [
            ("top-left", Corner::TopLeft),
            ("top-right", Corner::TopRight),
            ("bottom-left", Corner::BottomLeft),
            ("bottom-right", Corner::BottomRight),
        ] {
            assert_eq!(
                serde_json::from_str::<Corner>(&format!("\"{word}\"")).unwrap(),
                corner
            );
        }
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn a_corner_is_two_adjacent_edges() {
        for corner in [
            Corner::TopLeft,
            Corner::TopRight,
            Corner::BottomLeft,
            Corner::BottomRight,
        ] {
            let e = corner.edges();
            // Two opposite edges would stretch the surface.
            assert!(e.top != e.bottom && e.left != e.right, "{corner:?}");
        }
        let e = Corner::BottomRight.edges();
        assert!(e.bottom && e.right);
    }
}
