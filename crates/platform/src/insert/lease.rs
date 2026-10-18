//! A short-lived text lease with its own X11 owner. Restoration and owner check share a connection.

use super::ServerGrab;
use crate::jump::x11::atom;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant};
use x11rb::connection::Connection;
use x11rb::protocol::{Event, xfixes, xproto};
use x11rb::protocol::{xfixes::ConnectionExt as _, xproto::ConnectionExt as _};
use x11rb::rust_connection::RustConnection;
use x11rb::wrapper::ConnectionExt as _;

const LIMIT: usize = 64 * 1024;

struct Text {
    current: String,
    previous: Option<String>,
    restore_at: Option<Instant>,
    restored: bool,
}

pub struct Lease {
    conn: Arc<RustConnection>,
    window: u32,
    selection: u32,
    live: Arc<AtomicBool>,
    text: Arc<Mutex<Text>>,
    pending: Option<(u32, std::sync::mpsc::SyncSender<()>)>,
}

impl std::fmt::Debug for Lease {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Lease(X11)")
    }
}

impl Drop for Lease {
    fn drop(&mut self) {
        self.live.store(false, Ordering::SeqCst);
    }
}

impl Lease {
    /// None means no clipboard was changed. The read closure must only read, never claim ownership.
    pub fn copy(text: &str, previous: impl FnOnce() -> Option<String>) -> Option<Self> {
        if !super::x11_available() {
            return None;
        }
        let mut lease = Self::prepare(text, previous)?;
        lease.claim().then_some(lease)
    }

    /// Prepare outside the cancellation lock; an unresponsive previous owner must not block Esc.
    pub fn prepare(text: &str, previous: impl FnOnce() -> Option<String>) -> Option<Self> {
        if !super::x11_available() || text.len() > LIMIT || text.contains('\0') {
            return None;
        }
        let (conn, screen) = x11rb::connect(None).ok()?;
        let root = conn.setup().roots.get(screen)?;
        let window = conn.generate_id().ok()?;
        conn.create_window(
            x11rb::COPY_DEPTH_FROM_PARENT,
            window,
            root.root,
            0,
            0,
            1,
            1,
            0,
            xproto::WindowClass::INPUT_OUTPUT,
            root.root_visual,
            &xproto::CreateWindowAux::new(),
        )
        .ok()?
        .check()
        .ok()?;
        let selection = atom(&conn, "CLIPBOARD").ok()?;
        let targets = atom(&conn, "TARGETS").ok()?;
        let utf8 = atom(&conn, "UTF8_STRING").ok()?;
        let plain = atom(&conn, "text/plain;charset=utf-8").ok()?;
        let hint = atom(&conn, "x-kde-passwordManagerHint").ok()?;
        conn.xfixes_query_version(2, 0).ok()?.reply().ok()?;
        conn.xfixes_select_selection_input(
            window,
            selection,
            xfixes::SelectionEventMask::SET_SELECTION_OWNER
                | xfixes::SelectionEventMask::SELECTION_WINDOW_DESTROY
                | xfixes::SelectionEventMask::SELECTION_CLIENT_CLOSE,
        )
        .ok()?
        .check()
        .ok()?;
        let owner = conn.get_selection_owner(selection).ok()?.reply().ok()?.owner;
        while conn.poll_for_event().ok()?.is_some() {}
        let previous = previous();
        if previous
            .as_ref()
            .is_some_and(|s| s.len() > LIMIT || s.contains('\0'))
        {
            return None;
        }
        let conn = Arc::new(conn);
        let live = Arc::new(AtomicBool::new(true));
        let text = Arc::new(Mutex::new(Text {
            current: text.into(),
            previous,
            restore_at: None,
            restored: false,
        }));
        // Start the serving thread before claiming the selection: a failed spawn must leave it alone.
        let (start, ready) = std::sync::mpsc::sync_channel(1);
        let serving = conn.clone();
        let active = live.clone();
        let contents = text.clone();
        std::thread::Builder::new()
            .name("dictation-clipboard".into())
            .spawn(move || {
                let serve = || -> Option<()> {
                    ready.recv().ok()?;
                    while active.load(Ordering::SeqCst) {
                        while let Some(event) = serving.poll_for_event().ok()? {
                            match event {
                                Event::SelectionClear(_) => return None,
                                Event::SelectionRequest(request) if request.selection == selection => {
                                    let text = contents.lock().ok()?;
                                    let property = if request.property == 0 {
                                        request.target
                                    } else {
                                        request.property
                                    };
                                    let accepted = if request.target == targets {
                                        let mut types = vec![targets, utf8, plain];
                                        if !text.restored {
                                            types.push(hint);
                                        }
                                        serving
                                            .change_property32(
                                                xproto::PropMode::REPLACE,
                                                request.requestor,
                                                property,
                                                xproto::AtomEnum::ATOM,
                                                &types,
                                            )
                                            .ok()?
                                            .check()
                                            .is_ok()
                                    } else if request.target == utf8
                                        || request.target == plain
                                        || (request.target == hint && !text.restored)
                                    {
                                        let bytes = if request.target == hint {
                                            b"secret".as_slice()
                                        } else {
                                            text.current.as_bytes()
                                        };
                                        serving
                                            .change_property8(
                                                xproto::PropMode::REPLACE,
                                                request.requestor,
                                                property,
                                                request.target,
                                                bytes,
                                            )
                                            .ok()?
                                            .check()
                                            .is_ok()
                                    } else {
                                        false
                                    };
                                    let reply = xproto::SelectionNotifyEvent {
                                        response_type: xproto::SELECTION_NOTIFY_EVENT,
                                        sequence: 0,
                                        time: request.time,
                                        requestor: request.requestor,
                                        selection,
                                        target: request.target,
                                        property: if accepted { property } else { 0 },
                                    };
                                    // A closed requestor must not stop serving the next app.
                                    let _ = serving.send_event(
                                        false,
                                        request.requestor,
                                        xproto::EventMask::NO_EVENT,
                                        reply,
                                    );
                                    let _ = serving.flush();
                                }
                                _ => {}
                            }
                        }
                        {
                            let mut text = contents.lock().ok()?;
                            if text.restore_at.is_some_and(|at| Instant::now() >= at) {
                                text.restore_at = None;
                                serving.grab_server().ok()?.check().ok()?;
                                let _grab = ServerGrab(&serving);
                                if serving.get_selection_owner(selection).ok()?.reply().ok()?.owner != window
                                {
                                    return None;
                                }
                                if let Some(previous) = text.previous.take() {
                                    text.current = previous;
                                    text.restored = true;
                                    serving
                                        .set_selection_owner(window, selection, x11rb::CURRENT_TIME)
                                        .ok()?
                                        .check()
                                        .ok()?;
                                }
                            }
                        }
                        std::thread::sleep(Duration::from_millis(5));
                    }
                    Some(())
                };
                let _ = serve();
                active.store(false, Ordering::SeqCst);
                if let Ok(mut text) = contents.lock() {
                    text.previous = None;
                    text.current.clear();
                }
                let _ = serving.destroy_window(window);
                let _ = serving.flush();
            })
            .ok()?;
        Some(Self {
            conn,
            window,
            selection,
            live,
            text,
            pending: Some((owner, start)),
        })
    }

    /// Claim under the caller's cancellation guard. This does not read the previous owner.
    pub fn claim(&mut self) -> bool {
        self.claim_inner().unwrap_or(false)
    }

    fn claim_inner(&mut self) -> Option<bool> {
        let (owner, start) = self.pending.take()?;
        self.conn.grab_server().ok()?.check().ok()?;
        let _grab = ServerGrab(&self.conn);
        if self
            .conn
            .get_selection_owner(self.selection)
            .ok()?
            .reply()
            .ok()?
            .owner
            != owner
        {
            return Some(false);
        }
        // An equal text copied by the same owner is still a new copy.
        while let Some(event) = self.conn.poll_for_event().ok()? {
            if matches!(event, Event::XfixesSelectionNotify(_)) {
                return Some(false);
            }
        }
        self.conn
            .set_selection_owner(self.window, self.selection, x11rb::CURRENT_TIME)
            .ok()?
            .check()
            .ok()?;
        let claimed = self
            .conn
            .get_selection_owner(self.selection)
            .ok()?
            .reply()
            .ok()?
            .owner
            == self.window;
        drop(_grab);
        if !claimed {
            return Some(false);
        }
        start.send(()).ok()?;
        Some(true)
    }

    pub(super) fn owned_on(&self, conn: &RustConnection) -> bool {
        self.live.load(Ordering::SeqCst)
            && conn
                .get_selection_owner(self.selection)
                .ok()
                .and_then(|c| c.reply().ok())
                .is_some_and(|r| r.owner == self.window)
    }

    /// Overlapping dictations inherit the original text, never the older transcript.
    pub fn previous(&self) -> Option<Option<String>> {
        if !self.owned_on(&self.conn) {
            return None;
        }
        let text = self.text.lock().ok()?;
        Some(if text.restored {
            Some(text.current.clone())
        } else {
            text.previous.clone()
        })
    }

    /// A fallback is a permanent copy, not a lease awaiting restoration.
    pub fn copied(&self) {
        if let Ok(mut text) = self.text.lock() {
            text.previous = None;
            text.restore_at = None;
            text.restored = true;
        }
    }

    pub fn pasted(&self) {
        if let Ok(mut text) = self.text.lock() {
            text.restore_at = Some(Instant::now() + Duration::from_millis(700));
        }
    }
}
