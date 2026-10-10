//! The app's surfaces on Linux.
//!
//! Wayland gives an app neither its window position nor the global cursor. So each surface (the
//! island, the corner widget) is a wlr-layer-shell surface on the Overlay or Top layer, anchored to the edges its
//! [`LayerSpec`] names: anchored to the top edge alone, the compositor centers it like a panel
//! (KWin, Hyprland, Sway, niri; not GNOME). A surface keeps one fixed size and is never resized
//! or re-mapped (KWin stops showing a layer surface resized from the webview); only the rectangle
//! its page reports takes the mouse, through the input region, and everything else falls through
//! to the windows below. Without layer-shell (X11, GNOME, or `VULTS_NO_LAYER_SHELL`) it
//! stays a plain always-on-top window with the same region, unfocusable until the page asks for
//! the keyboard (and, on X11, on every workspace).
//!
//! The one exception to "never re-mapped": when a surface's output goes away (unplugged, turned
//! off) the compositor closes it, and only a new map brings it back.
//!
//! GTK lives on the main thread: every function here must be called there.

use std::cell::RefCell;
use std::collections::HashMap;

use gtk::prelude::*;
use gtk_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};

pub const NO_LAYER_SHELL_VAR: &str = "VULTS_NO_LAYER_SHELL";

/// A rectangle in logical pixels, relative to its window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

/// The screen edges a surface hangs from. One edge alone centers it along that edge; two
/// adjacent ones put it in their corner. Never two opposite edges: that stretches the surface.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Edges {
    pub top: bool,
    pub bottom: bool,
    pub left: bool,
    pub right: bool,
}

impl Edges {
    pub const TOP: Self = Self {
        top: true,
        bottom: false,
        left: false,
        right: false,
    };
}

/// How a surface sits on the screen. Its size is chosen once, before the first map; the edges
/// and margin can change later without a re-map.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LayerSpec {
    /// Tells the compositor's rules which surface this is.
    pub namespace: String,
    /// Logical size: fixed for the surface's whole life.
    pub width: i32,
    pub height: i32,
    pub edges: Edges,
    /// Distance from each anchored edge, in logical pixels.
    pub margin: i32,
    /// Whether it may take the keyboard from its first map; otherwise only when it asks
    /// ([`set_keyboard`]).
    pub keyboard: bool,
    /// Over full-screen windows too (the island: a card must be seen); otherwise on the Top
    /// layer, under them, like a panel.
    pub overlay: bool,
}

/// Turns the window into a layer surface when the session supports it; returns whether it did.
/// `label` names the surface's input region (the app's window label).
///
/// Must run before the window is first mapped: a surface's role can only be chosen then. That
/// is why the app creates its surfaces hidden; unrealizing a live window instead corrupts the
/// heap on exit.
pub fn init_layer(
    win: &gtk::ApplicationWindow,
    label: &str,
    spec: &LayerSpec,
    monitor: Option<&str>,
) -> bool {
    // Nothing takes the mouse until the page says where it draws.
    set_input_region(win, label, None);
    win.set_size_request(spec.width, spec.height);
    if std::env::var_os(NO_LAYER_SHELL_VAR).is_some() || !gtk_layer_shell::is_supported() {
        return false;
    }
    if win.is_realized() {
        win.hide();
        win.unrealize();
    }
    win.init_layer_shell();
    win.set_layer(if spec.overlay { Layer::Overlay } else { Layer::Top });
    win.set_namespace(&spec.namespace);
    let edges = spec.edges;
    for (edge, on) in [
        (Edge::Top, edges.top),
        (Edge::Bottom, edges.bottom),
        (Edge::Left, edges.left),
        (Edge::Right, edges.right),
    ] {
        win.set_anchor(edge, on);
        win.set_layer_shell_margin(edge, if on { spec.margin } else { 0 });
    }
    // Sit on the very edge without reserving space or being pushed by other panels.
    win.set_exclusive_zone(-1);
    win.set_keyboard_mode(if spec.keyboard {
        KeyboardMode::OnDemand
    } else {
        KeyboardMode::None
    });
    // Chosen before the first map, so starting up never moves the surface.
    place(win, label, monitor);
    // tauri.conf.json creates surfaces unfocusable for the X11 window; on a layer surface the
    // keyboard mode decides instead, and GTK must not refuse what it hands over.
    win.set_accept_focus(true);
    win.show_all();
    // The shape is reset when the surface is mapped.
    set_input_region(win, label, None);
    true
}

/// Whether the window became a layer surface (`init_layer`); otherwise the app places it.
pub fn is_layer(win: &gtk::ApplicationWindow) -> bool {
    win.is_layer_window()
}

/// Moves a mapped layer surface to other edges, without a re-map (its size stays). `keep_clear`
/// keeps it off the panels' reserved space (beside a panel); otherwise it sits on the very edge.
pub fn set_edges(win: &gtk::ApplicationWindow, edges: Edges, margin: i32, keep_clear: bool) {
    if !win.is_layer_window() {
        return;
    }
    for (edge, on) in [
        (Edge::Top, edges.top),
        (Edge::Bottom, edges.bottom),
        (Edge::Left, edges.left),
        (Edge::Right, edges.right),
    ] {
        win.set_anchor(edge, on);
        win.set_layer_shell_margin(edge, if on { margin } else { 0 });
    }
    win.set_exclusive_zone(if keep_clear { 0 } else { -1 });
}

impl Edges {
    /// The corner by a panel on `side`, where its tray usually is: the right end of a
    /// horizontal panel, the bottom of a vertical one.
    pub fn by_panel(side: Side) -> Self {
        Self {
            top: side == Side::Top,
            bottom: side != Side::Top,
            left: side == Side::Left,
            right: side != Side::Left,
        }
    }
}

/// The screen edge a panel sits on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Top,
    Bottom,
    Left,
    Right,
}

/// The edge of the Plasma panel that holds the system tray, from Plasma's own file
/// (`plasma-org.kde.plasma.desktop-appletsrc`). Plasma writes `location` as 3 top, 4 bottom,
/// 5 left, 6 right.
pub fn plasma_tray_side(appletsrc: &str) -> Option<Side> {
    use std::collections::HashMap;
    let mut location: HashMap<&str, &str> = HashMap::new();
    let mut with_tray: Vec<&str> = Vec::new();
    let mut section: Vec<&str> = Vec::new();
    for line in appletsrc.lines().map(str::trim) {
        if let Some(header) = line.strip_prefix('[') {
            section = header.trim_end_matches(']').split("][").collect();
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        match (section.as_slice(), key.trim()) {
            (["Containments", id], "location") => {
                location.insert(id, value.trim());
            }
            (["Containments", id, "Applets", _], "plugin") if value.trim() == "org.kde.plasma.systemtray" => {
                with_tray.push(id);
            }
            _ => {}
        }
    }
    with_tray.iter().find_map(|id| match location.get(id).copied()? {
        "3" => Some(Side::Top),
        "4" => Some(Side::Bottom),
        "5" => Some(Side::Left),
        "6" => Some(Side::Right),
        _ => None,
    })
}

/// The side of the panel holding the tray, as Plasma saved it; `None` off Plasma.
pub fn plasma_panel_side() -> Option<Side> {
    let config = std::env::var_os("XDG_CONFIG_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::home_dir().map(|h| h.join(".config")))?;
    let text = std::fs::read_to_string(config.join("plasma-org.kde.plasma.desktop-appletsrc")).ok()?;
    plasma_tray_side(&text)
}

/// The desktop's "animations" switch (GTK follows KDE's and GNOME's); off means reduced motion.
pub fn animations_enabled() -> bool {
    gtk::Settings::default().is_none_or(|s| s.is_gtk_enable_animations())
}

thread_local! {
    /// The last region each surface's page asked for, by label, put back after a re-map (a map
    /// resets the shape).
    static REGIONS: RefCell<HashMap<String, Option<Rect>>> = RefCell::new(HashMap::new());
}

fn region(label: &str) -> Option<Rect> {
    REGIONS.with_borrow(|r| r.get(label).copied().flatten())
}

/// Only `rect` takes the mouse; `None` lets every click through.
pub fn set_input_region(win: &gtk::ApplicationWindow, label: &str, rect: Option<Rect>) {
    REGIONS.with_borrow_mut(|r| r.insert(label.to_string(), rect));
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

/// Lets a surface take the keyboard (the island's chat input) and gives it back. Never exclusive:
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

/// The connected monitors: the name `place` takes, and a short label (the model; makers are
/// long: "Samsung Electric Company").
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

/// Puts the surface on the monitor named `wanted` while it is connected; otherwise, or with
/// `None`, the compositor chooses (usually the focused output). A layer surface that changes
/// output is re-mapped by gtk-layer-shell, so its input region is put back afterwards.
pub fn place(win: &gtk::ApplicationWindow, label: &str, wanted: Option<&str>) {
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
    restore_region(win, label);
}

/// Maps the surface again after the compositor closed it (its output went away). Waits a moment
/// first, so GTK has dropped the monitor that left before one is chosen.
pub fn revive(win: &gtk::ApplicationWindow, label: &str, wanted: Option<String>) {
    if !win.is_layer_window() {
        return;
    }
    let win = win.clone();
    let label = label.to_string();
    gtk::glib::timeout_add_local_once(std::time::Duration::from_millis(250), move || {
        place(&win, &label, wanted.as_deref());
        win.hide();
        win.show_all();
        restore_region(&win, &label);
    });
}

fn restore_region(win: &gtk::ApplicationWindow, label: &str) {
    apply_region(win, region(label));
    // The new surface may only exist once GTK has run: set it again on the next turn.
    let win = win.clone();
    let label = label.to_string();
    gtk::glib::idle_add_local_once(move || apply_region(&win, region(&label)));
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

/// Calls `crossed(true)` when the pointer comes onto a surface and `crossed(false)`
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

/// Today in the user's time zone, as (year, month, day): GLib reads it (TZ, /etc/localtime), so
/// no time-zone crate is needed. Safe from any thread.
/// The local UTC offset at `unix`, in seconds (`crate::utc_offset`).
pub fn utc_offset(unix: i64) -> Option<i32> {
    let t = gtk::glib::DateTime::from_unix_local(unix).ok()?;
    i32::try_from(t.utc_offset().as_seconds()).ok()
}

pub fn today() -> Option<(i32, u8, u8)> {
    let now = gtk::glib::DateTime::now_local().ok()?;
    Some((
        now.year(),
        u8::try_from(now.month()).ok()?,
        u8::try_from(now.day_of_month()).ok()?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_utc_offset_is_a_real_zone_offset() {
        let offset = utc_offset(1_791_547_200).expect("glib reads the local zone");
        assert!(offset.abs() <= 14 * 3600 && offset % 900 == 0, "{offset}");
    }

    const PLASMA: &str = "[Containments][23]
formfactor=2
location=3
plugin=org.kde.panel

[Containments][23][Applets][24]
plugin=org.kde.plasma.kickoff

[Containments][23][Applets][28]
location=4
plugin=org.kde.plasma.systemtray

[Containments][43]
location=0
plugin=org.kde.plasma.folder
";

    #[test]
    fn the_tray_side_is_the_panel_holding_it() {
        // The tray applet's own `location` is not the panel's.
        assert_eq!(plasma_tray_side(PLASMA), Some(Side::Top));
        let bottom = PLASMA.replace("location=3", "location=4");
        assert_eq!(plasma_tray_side(&bottom), Some(Side::Bottom));
        let left = PLASMA.replace("location=3", "location=5");
        assert_eq!(plasma_tray_side(&left), Some(Side::Left));
        assert_eq!(
            plasma_tray_side("[Containments][1]\nlocation=4\nplugin=org.kde.panel\n"),
            None
        );
        assert_eq!(plasma_tray_side(""), None);
    }

    #[test]
    fn the_corner_is_by_the_tray() {
        let corner = |side| {
            let e = Edges::by_panel(side);
            (e.top, e.bottom, e.left, e.right)
        };
        assert_eq!(corner(Side::Top), (true, false, false, true));
        assert_eq!(corner(Side::Bottom), (false, true, false, true));
        assert_eq!(corner(Side::Left), (false, true, true, false));
        assert_eq!(corner(Side::Right), (false, true, false, true));
    }
}
