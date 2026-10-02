//! The island on Linux.
//!
//! Wayland gives an app neither its window position nor the global cursor. So the island is a
//! wlr-layer-shell surface on the Overlay layer, anchored to the top edge: the compositor
//! centers it like a panel (KWin, Hyprland, Sway, niri; not GNOME). The surface keeps one fixed
//! size and is never resized or re-mapped (KWin stops showing a layer surface resized from
//! the webview); only the island's own rectangle takes the mouse, through the input region,
//! and everything else falls through to the windows below. Without layer-shell (X11, GNOME,
//! or `VULTURES_AI_NO_LAYER_SHELL`) it stays a plain always-on-top window with the same region,
//! unfocusable until the chat asks for the keyboard (and, on X11, on every workspace).

use gtk::prelude::*;
use gtk_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};

pub const NO_LAYER_SHELL_VAR: &str = "VULTURES_AI_NO_LAYER_SHELL";

/// A rectangle in logical pixels, relative to the island window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

/// Turns the window into a layer surface when the session supports it; returns whether it did.
///
/// Must run on the main thread before the window is first mapped: a surface's role can only
/// be chosen then. That is why the island is created hidden; unrealizing a live window instead
/// corrupts the heap on exit.
pub fn init_island(win: &gtk::ApplicationWindow, width: i32, height: i32) -> bool {
    // Nothing takes the mouse until the UI says where the island is.
    set_input_region(win, None);
    win.set_size_request(width, height);
    if std::env::var_os(NO_LAYER_SHELL_VAR).is_some() || !gtk_layer_shell::is_supported() {
        return false;
    }
    if win.is_realized() {
        win.hide();
        win.unrealize();
    }
    win.init_layer_shell();
    win.set_layer(Layer::Overlay);
    win.set_namespace(vultures_ai_brand::SLUG);
    win.set_anchor(Edge::Top, true);
    // Sit on the very top edge without reserving space or being pushed by other panels.
    win.set_exclusive_zone(-1);
    win.set_keyboard_mode(KeyboardMode::None);
    // tauri.conf.json creates the island unfocusable for the X11 window; on a layer surface
    // the keyboard mode decides instead, and GTK must not refuse what it hands over.
    win.set_accept_focus(true);
    win.show_all();
    // The shape is reset when the surface is mapped.
    set_input_region(win, None);
    true
}

/// Only `rect` takes the mouse; `None` lets every click through.
pub fn set_input_region(win: &gtk::ApplicationWindow, rect: Option<Rect>) {
    let region = match rect.filter(|r| r.width > 0 && r.height > 0) {
        Some(r) => {
            gtk::cairo::Region::create_rectangle(&gtk::cairo::RectangleInt::new(r.x, r.y, r.width, r.height))
        }
        None => gtk::cairo::Region::create(),
    };
    win.input_shape_combine_region(Some(&region));
}

/// Lets the island take the keyboard (the chat input) and gives it back. Never exclusive:
/// on-demand focus only follows a click, so typing elsewhere is never captured.
pub fn set_keyboard(win: &gtk::ApplicationWindow, on: bool) {
    if win.is_layer_window() {
        win.set_keyboard_mode(if on {
            KeyboardMode::OnDemand
        } else {
            KeyboardMode::None
        });
    } else {
        win.set_accept_focus(on);
    }
}
