use super::Window;
use crate::jump::x11::{atom, cardinal, title};
use x11rb::connection::Connection;
use x11rb::protocol::xproto::{AtomEnum, ConnectionExt};

pub(super) fn read() -> Option<Window> {
    let (conn, screen) = x11rb::connect(None).ok()?;
    let root = conn.setup().roots.get(screen)?.root;
    read_on(&conn, root)
}

pub(crate) fn read_on(conn: &x11rb::rust_connection::RustConnection, root: u32) -> Option<Window> {
    let active = atom(conn, "_NET_ACTIVE_WINDOW").ok()?;
    let foreground = || {
        conn.get_property(false, root, active, AtomEnum::WINDOW, 0, 1)
            .ok()?
            .reply()
            .ok()?
            .value32()?
            .next()
            .filter(|id| *id != 0)
    };
    let id = foreground()?;
    let class = conn
        .get_property(false, id, AtomEnum::WM_CLASS, AtomEnum::STRING, 0, 256)
        .ok()?
        .reply()
        .ok()?;
    let class = class_name(&class.value)?;
    let pid = cardinal(conn, id, atom(conn, "_NET_WM_PID").ok()?).filter(|p| *p > 1);
    let title = title(
        conn,
        id,
        atom(conn, "_NET_WM_NAME").ok()?,
        atom(conn, "UTF8_STRING").ok()?,
    );
    (foreground()? == id).then(|| Window {
        class,
        title,
        pid,
        identity: format!("x11:{id}"),
    })
}

fn class_name(bytes: &[u8]) -> Option<String> {
    let mut parts = bytes.split(|b| *b == 0);
    let instance = parts.next()?;
    let class = parts.next()?;
    let name = if class.is_empty() { instance } else { class };
    (!name.is_empty()).then(|| String::from_utf8_lossy(name).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn wm_class_uses_the_class_not_the_instance() {
        assert_eq!(class_name(b"some title\0Konsole\0"), Some("Konsole".into()));
        assert_eq!(class_name(b"kitty\0\0"), Some("kitty".into()));
        assert_eq!(class_name(b"malformed"), None);
        assert_eq!(class_name(b"\0\0"), None);
    }
}
