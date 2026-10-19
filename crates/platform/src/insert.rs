//! Opt-in insertion. Terminal capabilities cannot dispatch a generic paste chord.

pub(crate) mod kwin;
mod lease;
mod portal;
mod session;
pub use session::Pane;
mod wayland_lease;
#[cfg(test)]
mod wayland_tests;
pub use portal::{Permission, Portal};
#[cfg(test)]
mod tests;

pub enum Lease {
    X11(lease::Lease),
    Wayland(wayland_lease::Lease),
}
impl std::fmt::Debug for Lease {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::X11(lease) => lease.fmt(f),
            Self::Wayland(lease) => lease.fmt(f),
        }
    }
}
impl Lease {
    pub fn prepare(text: &str, previous: impl FnOnce() -> Option<String>) -> Option<Self> {
        if x11_available() {
            lease::Lease::prepare(text, previous).map(Self::X11)
        } else if wayland_available() {
            wayland_lease::Lease::prepare(text, previous).map(Self::Wayland)
        } else {
            None
        }
    }
    pub fn claim(&mut self) -> bool {
        match self {
            Self::X11(lease) => lease.claim(),
            Self::Wayland(lease) => lease.claim(),
        }
    }
    pub fn previous(&self) -> Option<Option<String>> {
        match self {
            Self::X11(lease) => lease.previous(),
            Self::Wayland(lease) => lease.previous(),
        }
    }
    pub fn copied(&self) {
        match self {
            Self::X11(lease) => lease.copied(),
            Self::Wayland(lease) => lease.copied(),
        }
    }
    pub fn pasted(&self) {
        match self {
            Self::X11(lease) => lease.pasted(),
            Self::Wayland(lease) => lease.pasted(),
        }
    }
    #[cfg(test)]
    fn copy(text: &str, previous: impl FnOnce() -> Option<String>) -> Option<Self> {
        let mut lease = Self::prepare(text, previous)?;
        lease.claim().then_some(lease)
    }
    #[cfg(test)]
    fn owned_on(&self, conn: &RustConnection) -> bool {
        matches!(self, Self::X11(lease) if lease.owned_on(conn))
    }
}

use crate::focus;
use crate::jump::x11::atom;
use x11rb::connection::Connection;
use x11rb::protocol::xproto::ConnectionExt as _;
use x11rb::protocol::{Event, xkb, xkb::ConnectionExt as _, xproto, xtest::ConnectionExt as _};
use x11rb::rust_connection::RustConnection;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Pasted,
    Copied,
    Session,
    Held,
}

/// This connection observes focus loss from capture through dispatch, including away-and-back.
/// Metadata from the asynchronous context reader cannot construct this capability.
pub struct X11 {
    conn: RustConnection,
    root: u32,
    active_atom: u32,
    window: focus::Window,
    keyboard: u32,
    process: String,
}

impl std::fmt::Debug for X11 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Target(X11)")
    }
}

/// A capability captured before recording, separate from asynchronous UI context.
pub enum Target {
    X11(Box<X11>),
    Wayland(portal::Target),
}
impl std::fmt::Debug for Target {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::X11(target) => target.fmt(f),
            Self::Wayland(target) => target.fmt(f),
        }
    }
}

impl Target {
    pub fn capture() -> Option<Self> {
        X11::capture().map(|target| Self::X11(Box::new(target)))
    }
    pub fn window(&self) -> &focus::Window {
        match self {
            Self::X11(target) => target.window(),
            Self::Wayland(target) => &target.window,
        }
    }
    pub fn paste(&self, lease: &Lease) -> bool {
        if self.window().is_terminal() {
            return false;
        }
        match (self, lease) {
            (Self::X11(target), Lease::X11(lease)) => target.paste(lease),
            (Self::Wayland(target), Lease::Wayland(lease)) => target.paste(lease),
            _ => false,
        }
    }
    pub fn current(&self) -> bool {
        match self {
            Self::X11(target) => target.current(),
            Self::Wayland(target) => target.current(),
        }
    }
    pub fn unchanged(&self) -> bool {
        match self {
            Self::X11(target) => target.current(),
            Self::Wayland(target) => target.unchanged(),
        }
    }
}

pub fn wayland_available() -> bool {
    std::env::var("XDG_SESSION_TYPE").is_ok_and(|s| s.eq_ignore_ascii_case("wayland"))
        && std::env::var("WAYLAND_DISPLAY").is_ok_and(|s| !s.is_empty())
        && std::env::var("XDG_CURRENT_DESKTOP")
            .is_ok_and(|s| s.split(':').any(|d| d.eq_ignore_ascii_case("KDE")))
}
pub fn available() -> bool {
    x11_available() || wayland_available()
}

fn native_x11(session: &str, wayland: &str, display: &str) -> bool {
    session.eq_ignore_ascii_case("x11") && wayland.is_empty() && !display.is_empty()
}

fn x11_available() -> bool {
    let env = |key| std::env::var(key).unwrap_or_default();
    native_x11(&env("XDG_SESSION_TYPE"), &env("WAYLAND_DISPLAY"), &env("DISPLAY"))
}

impl X11 {
    /// Call synchronously in the shortcut callback, before queuing capture work.
    pub fn capture() -> Option<Self> {
        x11_available().then(|| Self::open(None)).flatten()
    }

    fn open(display: Option<&str>) -> Option<Self> {
        let (conn, screen) = x11rb::connect(display).ok()?;
        let root = conn.setup().roots.get(screen)?.root;
        if !conn.xkb_use_extension(1, 0).ok()?.reply().ok()?.supported {
            return None;
        }
        let active_atom = atom(&conn, "_NET_ACTIVE_WINDOW").ok()?;
        conn.change_window_attributes(
            root,
            &xproto::ChangeWindowAttributesAux::new().event_mask(xproto::EventMask::PROPERTY_CHANGE),
        )
        .ok()?
        .check()
        .ok()?;
        let window = focus::x11::read_on(&conn, root)?;
        if !plain_editor(&window.class) && !window.is_terminal() {
            return None;
        }
        let process = process(window.pid?)?;
        let keyboard = conn.get_input_focus().ok()?.reply().ok()?.focus;
        let id = window.identity.strip_prefix("x11:")?.parse().ok()?;
        if !belongs(&conn, keyboard, id, root) {
            return None;
        }
        conn.change_window_attributes(
            keyboard,
            &xproto::ChangeWindowAttributesAux::new()
                .event_mask(xproto::EventMask::FOCUS_CHANGE | xproto::EventMask::STRUCTURE_NOTIFY),
        )
        .ok()?
        .check()
        .ok()?;
        let target = Self {
            conn,
            root,
            active_atom,
            window,
            keyboard,
            process,
        };
        target.current().then_some(target)
    }

    pub fn window(&self) -> &focus::Window {
        &self.window
    }

    fn current(&self) -> bool {
        let same = focus::x11::read_on(&self.conn, self.root).is_some_and(|w| {
            w.identity == self.window.identity && w.pid == self.window.pid && w.class == self.window.class
        }) && self
            .conn
            .get_input_focus()
            .ok()
            .and_then(|c| c.reply().ok())
            .is_some_and(|r| r.focus == self.keyboard)
            && self.window.pid.and_then(process).as_deref() == Some(&self.process);
        if !same {
            return false;
        }
        loop {
            match self.conn.poll_for_event() {
                Ok(Some(Event::PropertyNotify(e))) if e.atom == self.active_atom => return false,
                Ok(Some(
                    Event::FocusOut(_)
                    | Event::DestroyNotify(_)
                    | Event::UnmapNotify(_)
                    | Event::MappingNotify(_),
                )) => return false,
                Ok(Some(_)) => {}
                Ok(None) => return true,
                Err(_) => return false,
            }
        }
    }

    /// No focus recovery and no Enter. The server grab covers only validation and four key events.
    pub fn paste(&self, lease: &lease::Lease) -> bool {
        !self.window.is_terminal() && self.dispatch(lease).unwrap_or(false)
    }

    fn dispatch(&self, lease: &lease::Lease) -> Option<bool> {
        self.conn.grab_server().ok()?.check().ok()?;
        let _grab = ServerGrab(&self.conn);
        if !self.current() || !lease.owned_on(&self.conn) {
            return Some(false);
        }
        let held = self.conn.query_keymap().ok()?.reply().ok()?;
        if held.keys.iter().any(|b| *b != 0) {
            return Some(false);
        }
        let min = self.conn.setup().min_keycode;
        let max = self.conn.setup().max_keycode;
        let mapping = self
            .conn
            .get_keyboard_mapping(min, max - min + 1)
            .ok()?
            .reply()
            .ok()?;
        let code = |symbol| {
            mapping
                .keysyms
                .chunks(usize::from(mapping.keysyms_per_keycode))
                .position(|row| row.first() == Some(&symbol))
                .and_then(|i| min.checked_add(u8::try_from(i).ok()?))
        };
        let ctrl = code(0xffe3)?;
        let v = code(0x76)?;
        let mods = self.conn.get_modifier_mapping().ok()?.reply().ok()?;
        let per = usize::from(mods.keycodes_per_modifier());
        if per == 0
            || !mods.keycodes[2 * per..3 * per].contains(&ctrl)
            || mods
                .keycodes
                .chunks(per)
                .enumerate()
                .any(|(i, group)| i != 2 && group.contains(&ctrl))
            || mods.keycodes.contains(&v)
        {
            return Some(false);
        }
        // Only identified Caps/Num locks are harmless. Latched modifiers and other groups copy.
        let ignored = mods
            .keycodes
            .chunks(per)
            .enumerate()
            .filter(|(_, group)| {
                group.iter().any(|key| *key != 0)
                    && group.iter().filter(|key| **key != 0).all(|key| {
                        key.checked_sub(min)
                            .and_then(|i| {
                                mapping
                                    .keysyms
                                    .get(usize::from(i) * usize::from(mapping.keysyms_per_keycode))
                            })
                            .is_some_and(|symbol| matches!(symbol, 0xffe5 | 0xff7f))
                    })
            })
            .fold(0u16, |mask, (i, _)| mask | (1 << i));
        let state = self
            .conn
            .xkb_get_state(xkb::ID::USE_CORE_KBD.into())
            .ok()?
            .reply()
            .ok()?;
        let pointer = self.conn.query_pointer(self.root).ok()?.reply().ok()?.mask;
        if state.group != xkb::Group::M1
            || state.base_group != 0
            || state.latched_group != 0
            || state.locked_group != xkb::Group::M1
            || u16::from(state.mods) & !ignored != 0
            || u16::from(pointer) & !ignored != 0
        {
            return Some(false);
        }
        self.conn.xtest_get_version(2, 2).ok()?.reply().ok()?;
        let mut ok = true;
        for (key, down) in [(ctrl, true), (v, true), (v, false), (ctrl, false)] {
            // Always attempt both releases, even when an earlier request failed.
            ok &= self
                .conn
                .xtest_fake_input(
                    if down {
                        xproto::KEY_PRESS_EVENT
                    } else {
                        xproto::KEY_RELEASE_EVENT
                    },
                    key,
                    0,
                    self.root,
                    0,
                    0,
                    0,
                )
                .ok()
                .and_then(|c| c.check().ok())
                .is_some();
        }
        Some(ok)
    }
}

// Editors with no integrated terminal. Broader apps need field/pane safety, not a title heuristic.
fn plain_editor(class: &str) -> bool {
    matches!(
        class.to_ascii_lowercase().as_str(),
        "kwrite" | "org.kde.kwrite" | "org.gnome.texteditor" | "gnome-text-editor" | "mousepad" | "leafpad"
    )
}

fn process(pid: u32) -> Option<String> {
    use std::os::unix::fs::MetadataExt;
    let path = format!("/proc/{pid}/stat");
    let own = std::fs::metadata("/proc/self").ok()?.uid();
    if std::fs::metadata(&path).ok()?.uid() != own {
        return None;
    }
    let stat = std::fs::read_to_string(path).ok()?;
    // The command can contain spaces and parentheses; start time is field 22.
    Some(stat.rsplit_once(')')?.1.split_whitespace().nth(19)?.to_owned())
}

fn belongs(conn: &RustConnection, mut child: u32, parent: u32, root: u32) -> bool {
    for _ in 0..32 {
        if child == parent {
            return true;
        }
        if child == root || child <= 1 {
            return false;
        }
        let Some(tree) = conn.query_tree(child).ok().and_then(|c| c.reply().ok()) else {
            return false;
        };
        if tree.parent == child {
            return false;
        }
        child = tree.parent;
    }
    false
}

pub(super) struct ServerGrab<'a>(&'a RustConnection);
impl Drop for ServerGrab<'_> {
    fn drop(&mut self) {
        let _ = self.0.ungrab_server();
        let _ = self.0.flush();
    }
}
