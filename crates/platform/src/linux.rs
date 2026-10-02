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
//!
//! The one exception to "never re-mapped": when the island's output goes away (unplugged,
//! turned off) the compositor closes the surface, and only a new map brings the island back.

use std::cell::Cell;

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
pub fn init_island(win: &gtk::ApplicationWindow, width: i32, height: i32, monitor: Option<&str>) -> bool {
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
    // Chosen before the first map, so starting up never moves the surface.
    place_island(win, monitor);
    // tauri.conf.json creates the island unfocusable for the X11 window; on a layer surface
    // the keyboard mode decides instead, and GTK must not refuse what it hands over.
    win.set_accept_focus(true);
    win.show_all();
    // The shape is reset when the surface is mapped.
    set_input_region(win, None);
    true
}

thread_local! {
    /// The last region the UI asked for, put back after a re-map (a map resets the shape).
    /// GTK lives on the main thread, so this does too.
    static REGION: Cell<Option<Rect>> = const { Cell::new(None) };
}

/// Only `rect` takes the mouse; `None` lets every click through.
pub fn set_input_region(win: &gtk::ApplicationWindow, rect: Option<Rect>) {
    REGION.set(rect);
    apply_region(win, rect);
}

fn apply_region(win: &gtk::ApplicationWindow, rect: Option<Rect>) {
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

/// How the settings name a monitor: GTK 3 has no connector name ("DP-1"), so maker and model.
/// Two identical screens share a name; the first one connected wins.
fn monitor_name(m: &gtk::gdk::Monitor) -> String {
    let parts = [m.manufacturer(), m.model()];
    let name = parts.iter().flatten().map(|s| s.trim()).filter(|s| !s.is_empty());
    name.collect::<Vec<_>>().join(" ")
}

fn monitors() -> Vec<gtk::gdk::Monitor> {
    let Some(display) = gtk::gdk::Display::default() else {
        return Vec::new();
    };
    (0..display.n_monitors())
        .filter_map(|i| display.monitor(i))
        .collect()
}

/// The connected monitors: the name `place_island` takes, and a short label (the model; makers
/// are long: "Samsung Electric Company").
pub fn monitor_names() -> Vec<(String, String)> {
    let mut list: Vec<(String, String)> = Vec::new();
    for m in monitors() {
        let name = monitor_name(&m);
        if name.is_empty() || list.iter().any(|(n, _)| *n == name) {
            continue;
        }
        let label = m.model().map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
        list.push((name.clone(), label.unwrap_or(name)));
    }
    list
}

/// Puts the island on the monitor named `wanted` while it is connected; otherwise, or with
/// `None`, the compositor chooses (usually the focused output). A layer surface that changes
/// output is re-mapped by gtk-layer-shell, so the input region is put back afterwards.
pub fn place_island(win: &gtk::ApplicationWindow, wanted: Option<&str>) {
    if !win.is_layer_window() {
        return;
    }
    let target = wanted.and_then(|w| monitors().into_iter().find(|m| monitor_name(m) == w));
    if win.monitor() == target {
        return;
    }
    match &target {
        Some(m) => win.set_monitor(m),
        None => {
            use gtk::glib::translate::ToGlibPtr;
            let gtk_win: &gtk::Window = win.upcast_ref();
            // SAFETY: a live GtkWindow from the main thread; NULL is the documented "let the
            // compositor decide".
            unsafe {
                gtk_layer_shell_sys::gtk_layer_set_monitor(gtk_win.to_glib_none().0, std::ptr::null_mut())
            };
        }
    }
    restore_region(win);
}

/// Maps the island again after the compositor closed its surface (its output went away).
/// Waits a moment first, so GTK has dropped the monitor that left before one is chosen.
pub fn revive_island(win: &gtk::ApplicationWindow, wanted: Option<String>) {
    if !win.is_layer_window() {
        return;
    }
    let win = win.clone();
    gtk::glib::timeout_add_local_once(std::time::Duration::from_millis(250), move || {
        place_island(&win, wanted.as_deref());
        win.hide();
        win.show_all();
        restore_region(&win);
    });
}

fn restore_region(win: &gtk::ApplicationWindow) {
    apply_region(win, REGION.get());
    // The new surface may only exist once GTK has run: set it again on the next turn.
    let win = win.clone();
    gtk::glib::idle_add_local_once(move || apply_region(&win, REGION.get()));
}

/// Calls `changed` on the main thread whenever a monitor is plugged in or removed.
pub fn on_monitors_changed(changed: impl Fn() + Clone + 'static) {
    let Some(display) = gtk::gdk::Display::default() else {
        return;
    };
    let added = changed.clone();
    display.connect_monitor_added(move |_, _| added());
    display.connect_monitor_removed(move |_, _| changed());
}

/// Calls `crossed(true)` when the pointer comes onto the island's surface and `crossed(false)`
/// when it leaves. The page cannot tell by itself: leaving through the input region's edge,
/// WebKitGTK keeps the last position inside the page, so `pointerleave` never fires; and its
/// pointer events reach the page by another road than ours, so mixing the two loses the order.
/// GTK's crossings come in order: on Linux they are the only word on where the pointer is.
pub fn on_pointer_crossing(win: &gtk::ApplicationWindow, crossed: impl Fn(bool) + Clone + 'static) {
    use gtk::gdk::{EventMask, NotifyType};
    win.add_events(EventMask::ENTER_NOTIFY_MASK | EventMask::LEAVE_NOTIFY_MASK);
    let entered = crossed.clone();
    // Into or out of the webview's own child window is not crossing the surface.
    win.connect_enter_notify_event(move |_, event| {
        if event.detail() != NotifyType::Inferior {
            entered(true);
        }
        gtk::glib::Propagation::Proceed
    });
    win.connect_leave_notify_event(move |_, event| {
        if event.detail() != NotifyType::Inferior {
            crossed(false);
        }
        gtk::glib::Propagation::Proceed
    });
}
