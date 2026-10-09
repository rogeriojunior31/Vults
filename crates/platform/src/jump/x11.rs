//! EWMH: find the window by `_NET_WM_PID` among `_NET_CLIENT_LIST`, then ask the window manager
//! to activate it. Works in any X11 session and, on Wayland, for XWayland windows. Connects to
//! the app's own `DISPLAY`: the session's could be a forwarded one on another machine.

use x11rb::connection::Connection;
use x11rb::protocol::xproto::{AtomEnum, ClientMessageEvent, ConnectionExt, EventMask, Window};
use x11rb::rust_connection::RustConnection;

type Fallible<T> = Result<T, Box<dyn std::error::Error>>;

/// True when a window of `ancestors` was found and asked to come forward.
pub fn activate(ancestors: &[u32], folder: &str) -> bool {
    try_activate(ancestors, folder).unwrap_or(false)
}

fn atom(conn: &RustConnection, name: &str) -> Fallible<u32> {
    Ok(conn.intern_atom(false, name.as_bytes())?.reply()?.atom)
}

fn cardinal(conn: &RustConnection, window: Window, prop: u32) -> Option<u32> {
    let reply = conn
        .get_property(false, window, prop, AtomEnum::CARDINAL, 0, 1)
        .ok()?
        .reply()
        .ok()?;
    reply.value32()?.next()
}

/// `_NET_WM_NAME` (UTF-8), else the legacy `WM_NAME`.
fn title(conn: &RustConnection, window: Window, net_wm_name: u32, utf8: u32) -> String {
    let read = |prop: u32, kind: u32| {
        conn.get_property(false, window, prop, kind, 0, 1024)
            .ok()
            .and_then(|c| c.reply().ok())
            .map(|r| String::from_utf8_lossy(&r.value).into_owned())
            .filter(|t| !t.is_empty())
    };
    read(net_wm_name, utf8)
        .or_else(|| read(AtomEnum::WM_NAME.into(), AtomEnum::STRING.into()))
        .unwrap_or_default()
}

fn try_activate(ancestors: &[u32], folder: &str) -> Fallible<bool> {
    let (conn, screen) = x11rb::connect(None)?;
    let root = conn.setup().roots.get(screen).ok_or("no screen")?.root;
    let client_list = atom(&conn, "_NET_CLIENT_LIST")?;
    let wm_pid = atom(&conn, "_NET_WM_PID")?;
    let net_wm_name = atom(&conn, "_NET_WM_NAME")?;
    let utf8 = atom(&conn, "UTF8_STRING")?;
    let active = atom(&conn, "_NET_ACTIVE_WINDOW")?;
    let wm_desktop = atom(&conn, "_NET_WM_DESKTOP")?;
    let current_desktop = atom(&conn, "_NET_CURRENT_DESKTOP")?;

    let clients: Vec<Window> = conn
        .get_property(false, root, client_list, AtomEnum::WINDOW, 0, 4096)?
        .reply()?
        .value32()
        .map(|v| v.collect())
        .unwrap_or_default();
    // Every window's process in one round trip; a window gone meanwhile is skipped.
    let cookies = clients
        .iter()
        .map(|w| conn.get_property(false, *w, wm_pid, AtomEnum::CARDINAL, 0, 1))
        .collect::<Result<Vec<_>, _>>()?;
    let mut windows: Vec<(Window, u32, String)> = Vec::new();
    for (window, cookie) in clients.iter().zip(cookies) {
        let Ok(reply) = cookie.reply() else { continue };
        let Some(pid) = reply.value32().and_then(|mut v| v.next()) else {
            continue;
        };
        if ancestors.contains(&pid) {
            windows.push((*window, pid, title(&conn, *window, net_wm_name, utf8)));
        }
    }
    let Some(&target) = super::choose_window(ancestors, &windows, folder) else {
        return Ok(false);
    };

    let to_root = EventMask::SUBSTRUCTURE_REDIRECT | EventMask::SUBSTRUCTURE_NOTIFY;
    // Its workspace first, for the window managers that don't switch on their own.
    if let Some(desktop) = cardinal(&conn, target, wm_desktop).filter(|d| *d != u32::MAX) {
        let ev = ClientMessageEvent::new(32, root, current_desktop, [desktop, 0, 0, 0, 0]);
        conn.send_event(false, root, to_root, ev)?;
    }
    // Source 2: a pager acting for the user, which window managers honour over their
    // focus-stealing prevention. Timestamp 0: now.
    let ev = ClientMessageEvent::new(32, target, active, [2, 0, 0, 0, 0]);
    conn.send_event(false, root, to_root, ev)?;
    conn.flush()?;
    Ok(true)
}
