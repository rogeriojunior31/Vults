//! Restore by changing this source's bytes, never by reclaiming the Wayland selection.
use std::io::Write;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
    mpsc,
};
use std::time::{Duration, Instant};
use wayland_client::globals::{GlobalListContents, registry_queue_init};
use wayland_client::protocol::{wl_registry, wl_seat};
use wayland_client::{Connection, Dispatch, QueueHandle, delegate_noop, event_created_child};
use wayland_protocols::ext::data_control::v1::client::{
    ext_data_control_device_v1 as device, ext_data_control_manager_v1 as manager,
    ext_data_control_offer_v1 as offer, ext_data_control_source_v1 as source,
};

const LIMIT: usize = 64 * 1024;
const TEXT: &str = "text/plain;charset=utf-8";
const HINT: &str = "x-kde-passwordManagerHint";

struct Text {
    current: String,
    previous: Option<String>,
    restore: Option<Instant>,
    temporary: bool,
}
struct Owner {
    send: mpsc::Sender<Command>,
    live: AtomicBool,
    text: Mutex<Text>,
}
impl Drop for Owner {
    fn drop(&mut self) {
        self.live.store(false, Ordering::SeqCst);
    }
}
#[derive(Clone)]
pub struct Lease {
    owner: Arc<Owner>,
}
impl std::fmt::Debug for Lease {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Lease(Wayland)")
    }
}

enum Command {
    Claim(mpsc::SyncSender<bool>),
    Check(mpsc::SyncSender<bool>),
}
struct State {
    owner: std::sync::Weak<Owner>,
    owned: bool,
    offers: Vec<offer::ExtDataControlOfferV1>,
    transfers: Vec<Transfer>,
}
impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for State {
    fn event(
        _: &mut Self,
        _: &wl_registry::WlRegistry,
        _: wl_registry::Event,
        _: &GlobalListContents,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}
delegate_noop!(State: ignore manager::ExtDataControlManagerV1);
delegate_noop!(State: ignore wl_seat::WlSeat);
delegate_noop!(State: ignore offer::ExtDataControlOfferV1);
impl Dispatch<device::ExtDataControlDeviceV1, ()> for State {
    fn event(
        state: &mut Self,
        _: &device::ExtDataControlDeviceV1,
        event: device::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            device::Event::DataOffer { id } => state.offers.push(id),
            device::Event::Selection { .. } | device::Event::PrimarySelection { .. } => {
                for offer in state.offers.drain(..) {
                    offer.destroy();
                }
            }
            device::Event::Finished => state.owned = false,
            _ => {}
        }
    }
    event_created_child!(State, device::ExtDataControlDeviceV1, [0 => (offer::ExtDataControlOfferV1, ())]);
}
struct Transfer {
    file: std::fs::File,
    bytes: Vec<u8>,
    offset: usize,
    deadline: Instant,
}
impl Dispatch<source::ExtDataControlSourceV1, ()> for State {
    fn event(
        state: &mut Self,
        _: &source::ExtDataControlSourceV1,
        event: source::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            source::Event::Cancelled => state.owned = false,
            source::Event::Send { mime_type, fd } => {
                let Some(owner) = state.owner.upgrade() else {
                    return;
                };
                let Ok(text) = owner.text.lock() else {
                    return;
                };
                let bytes = if mime_type == HINT {
                    b"secret".as_slice()
                } else {
                    text.current.as_bytes()
                };
                // The fd is nonblocking: a stalled consumer cannot stall cancellation or ownership checks.
                let flags = rustix::fs::fcntl_getfl(&fd).unwrap_or(rustix::fs::OFlags::empty());
                if rustix::fs::fcntl_setfl(&fd, flags | rustix::fs::OFlags::NONBLOCK).is_err() {
                    return;
                }
                if state.transfers.len() < 8 {
                    state.transfers.push(Transfer {
                        file: std::fs::File::from(fd),
                        bytes: bytes.into(),
                        offset: 0,
                        deadline: Instant::now() + Duration::from_secs(5),
                    });
                }
            }
            _ => {}
        }
    }
}

impl Lease {
    pub fn prepare(text: &str, previous: impl FnOnce() -> Option<String>) -> Option<Self> {
        if text.len() > LIMIT || text.contains('\0') {
            return None;
        }
        let conn = Connection::connect_to_env().ok()?;
        let (globals, mut queue) = registry_queue_init::<State>(&conn).ok()?;
        let qh = queue.handle();
        // Multiple seats need an explicit seat identity; never guess whose clipboard to replace.
        if globals
            .contents()
            .with_list(|list| list.iter().filter(|g| g.interface == "wl_seat").count())
            != 1
        {
            return None;
        }
        let manager: manager::ExtDataControlManagerV1 = globals.bind(&qh, 1..=1, ()).ok()?;
        let seat: wl_seat::WlSeat = globals.bind(&qh, 1..=1, ()).ok()?;
        let device = manager.get_data_device(&seat, &qh, ());
        let source = manager.create_data_source(&qh, ());
        for mime in [TEXT, "text/plain", "UTF8_STRING", HINT] {
            source.offer(mime.into());
        }
        let previous = previous();
        if previous
            .as_ref()
            .is_some_and(|s| s.len() > LIMIT || s.contains('\0'))
        {
            return None;
        }
        let (send, receive) = mpsc::channel();
        let owner = Arc::new(Owner {
            send,
            live: AtomicBool::new(true),
            text: Mutex::new(Text {
                current: text.into(),
                previous,
                restore: None,
                temporary: true,
            }),
        });
        let weak = Arc::downgrade(&owner);
        std::thread::Builder::new()
            .name("dictation-wayland-clipboard".into())
            .spawn(move || {
                let mut state = State {
                    owner: weak.clone(),
                    owned: false,
                    offers: Vec::new(),
                    transfers: Vec::new(),
                };
                let mut claimed = false;
                let mut run = || -> Option<()> {
                    loop {
                        let owner = weak.upgrade()?;
                        if !owner.live.load(Ordering::SeqCst) {
                            return None;
                        }
                        while let Ok(command) = receive.try_recv() {
                            match command {
                                Command::Claim(reply) => {
                                    if claimed {
                                        let _ = reply.send(false);
                                        continue;
                                    }
                                    claimed = true;
                                    state.owned = true;
                                    device.set_selection(Some(&source));
                                    let valid = queue.roundtrip(&mut state).is_ok() && state.owned;
                                    let _ = reply.send(valid);
                                }
                                Command::Check(reply) => {
                                    let valid = queue.roundtrip(&mut state).is_ok() && state.owned;
                                    let _ = reply.send(valid);
                                }
                            }
                        }
                        queue.dispatch_pending(&mut state).ok()?;
                        if claimed && !state.owned {
                            return None;
                        }
                        state.transfers.retain_mut(|transfer| {
                            while transfer.offset < transfer.bytes.len() && Instant::now() < transfer.deadline
                            {
                                match transfer.file.write(&transfer.bytes[transfer.offset..]) {
                                    Ok(0) => return false,
                                    Ok(count) => transfer.offset += count,
                                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                                        return true;
                                    }
                                    Err(_) => return false,
                                }
                            }
                            false
                        });
                        {
                            let mut text = owner.text.lock().ok()?;
                            if state.owned && text.restore.is_some_and(|at| Instant::now() >= at) {
                                text.restore = None;
                                if let Some(previous) = text.previous.take() {
                                    text.current = previous;
                                    text.temporary = false;
                                }
                                // No set_selection here: even an ownership change still in flight wins.
                            }
                        }
                        conn.flush().ok()?;
                        drop(owner);
                        if let Some(guard) = queue.prepare_read() {
                            let mut fds = [rustix::event::PollFd::new(&conn, rustix::event::PollFlags::IN)];
                            let timeout = rustix::event::Timespec {
                                tv_sec: 0,
                                tv_nsec: 10_000_000,
                            };
                            if rustix::event::poll(&mut fds, Some(&timeout)).ok()? > 0 {
                                guard.read().ok()?;
                            }
                        }
                    }
                };
                let _ = run();
                if let Some(owner) = weak.upgrade() {
                    owner.live.store(false, Ordering::SeqCst);
                    if let Ok(mut text) = owner.text.lock() {
                        text.previous = None;
                        text.current.clear();
                    }
                }
            })
            .ok()?;
        Some(Self { owner })
    }
    fn ask(&self, claim: bool) -> bool {
        if !self.owner.live.load(Ordering::SeqCst) {
            return false;
        }
        let (send, receive) = mpsc::sync_channel(1);
        if self
            .owner
            .send
            .send(if claim {
                Command::Claim(send)
            } else {
                Command::Check(send)
            })
            .is_err()
        {
            return false;
        }
        receive.recv_timeout(Duration::from_millis(300)).unwrap_or(false)
    }
    pub fn claim(&self) -> bool {
        self.ask(true)
    }
    pub fn current(&self) -> bool {
        self.ask(false)
    }
    pub fn previous(&self) -> Option<Option<String>> {
        if !self.current() {
            return None;
        }
        let text = self.owner.text.lock().ok()?;
        Some(if text.temporary {
            text.previous.clone()
        } else {
            Some(text.current.clone())
        })
    }
    pub fn copied(&self) {
        if let Ok(mut text) = self.owner.text.lock() {
            text.previous = None;
            text.restore = None;
            text.temporary = false;
        }
    }
    pub fn pasted(&self) {
        if let Ok(mut text) = self.owner.text.lock() {
            text.restore = Some(Instant::now() + Duration::from_millis(700));
        }
    }
}
