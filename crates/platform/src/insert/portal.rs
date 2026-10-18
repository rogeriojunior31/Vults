//! Keyboard-only portal session. A refused or revoked grant never triggers an automatic retry.
use super::{focus, kwin, plain_editor, process, wayland_lease};
use ashpd::desktop::{
    PersistMode, Session,
    remote_desktop::{DeviceType, KeyState, RemoteDesktop, SelectDevicesOptions},
};
use futures_util::{FutureExt, StreamExt};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicU64, Ordering},
    mpsc,
};
use std::time::Duration;
use tokio::sync::{Notify, mpsc as channel};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Permission {
    Off,
    Requesting,
    Ready,
    Temporary,
    Unavailable,
}
impl Permission {
    pub fn code(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Requesting => "requesting",
            Self::Ready => "ready",
            Self::Temporary => "temporary",
            Self::Unavailable => "unavailable",
        }
    }
    fn ready(self) -> bool {
        matches!(self, Self::Ready | Self::Temporary)
    }
}
struct Shared {
    epoch: AtomicU64,
    focus_epoch: AtomicU64,
    local_focus: AtomicBool,
    state: Mutex<Permission>,
    focus: Mutex<Option<Arc<kwin::Watch>>>,
    changed: Notify,
    notify: Box<dyn Fn(Permission) + Send + Sync>,
}
#[derive(Clone)]
pub struct Portal {
    shared: Arc<Shared>,
    send: channel::UnboundedSender<Command>,
}
impl std::fmt::Debug for Portal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Portal(Keyboard)")
    }
}

type Save = Box<dyn FnOnce(Option<&str>) -> bool + Send>;
enum Command {
    Request {
        epoch: u64,
        token: Option<String>,
        persist: bool,
        save: Save,
    },
    Close(Option<mpsc::SyncSender<()>>),
    Check {
        target: Target,
        reply: mpsc::SyncSender<bool>,
    },
    Paste {
        live: Arc<AtomicBool>,
        target: Target,
        lease: wayland_lease::Lease,
        reply: mpsc::SyncSender<bool>,
    },
}

#[derive(Clone)]
pub struct Target {
    portal: Portal,
    epoch: u64,
    sequence: u64,
    local_focus: u64,
    pub(super) window: focus::Window,
    process: String,
}
impl std::fmt::Debug for Target {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Target(Wayland)")
    }
}
impl Target {
    pub(super) fn unchanged(&self) -> bool {
        self.valid()
            && self.portal.shared.focus.lock().is_ok_and(|focus| {
                focus.as_ref().is_some_and(|watch| {
                    let snapshot = watch.snapshot();
                    snapshot.sequence == self.sequence && snapshot.window.as_ref() == Some(&self.window)
                })
            })
    }
    pub(super) fn current(&self) -> bool {
        if !self.valid() {
            return false;
        }
        let (reply, receive) = mpsc::sync_channel(1);
        if self
            .portal
            .send
            .send(Command::Check {
                target: self.clone(),
                reply,
            })
            .is_err()
        {
            return false;
        }
        receive.recv_timeout(Duration::from_millis(400)).unwrap_or(false) && self.valid()
    }
    fn valid(&self) -> bool {
        self.portal.shared.valid(self.epoch)
            && self.portal.state().ready()
            && !self.portal.shared.local_focus.load(Ordering::SeqCst)
            && self.portal.shared.focus_epoch.load(Ordering::SeqCst) == self.local_focus
    }
    pub(super) fn paste(&self, lease: &wayland_lease::Lease) -> bool {
        if self.window.is_terminal() || !self.valid() {
            return false;
        }
        let (reply, receive) = mpsc::sync_channel(1);
        let live = Arc::new(AtomicBool::new(true));
        if self
            .portal
            .send
            .send(Command::Paste {
                live: live.clone(),
                target: self.clone(),
                lease: lease.clone(),
                reply,
            })
            .is_err()
        {
            return false;
        }
        let result = receive.recv_timeout(Duration::from_secs(2)).unwrap_or(false);
        live.store(false, Ordering::SeqCst);
        result
    }
}
impl Shared {
    fn valid(&self, epoch: u64) -> bool {
        self.epoch.load(Ordering::SeqCst) == epoch
    }
    fn status(&self, epoch: u64, state: Permission) {
        let Ok(mut current) = self.state.lock() else {
            return;
        };
        if !self.valid(epoch) {
            return;
        }
        *current = state;
        (self.notify)(state);
    }
    fn clear(&self) {
        if let Ok(mut focus) = self.focus.lock() {
            *focus = None;
        }
    }
}
impl Portal {
    pub fn new(notify: impl Fn(Permission) + Send + Sync + 'static) -> Result<Self, String> {
        let shared = Arc::new(Shared {
            epoch: AtomicU64::new(0),
            focus_epoch: AtomicU64::new(0),
            local_focus: AtomicBool::new(false),
            state: Mutex::new(Permission::Off),
            focus: Mutex::default(),
            changed: Notify::new(),
            notify: Box::new(notify),
        });
        let (send, receive) = channel::unbounded_channel();
        let actor = shared.clone();
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|_| "The keyboard portal worker is unavailable.")?;
        std::thread::Builder::new()
            .name("dictation-keyboard-portal".into())
            .spawn(move || runtime.block_on(run(actor, receive)))
            .map_err(|_| "The keyboard portal worker is unavailable.")?;
        Ok(Self { shared, send })
    }
    pub fn state(&self) -> Permission {
        self.shared
            .state
            .lock()
            .map(|s| *s)
            .unwrap_or(Permission::Unavailable)
    }
    /// Whether this active keyboard session also owns its verified KWin focus watch.
    pub fn focus_ready(&self) -> bool {
        self.state().ready() && self.shared.focus.lock().is_ok_and(|focus| focus.is_some())
    }
    pub fn request(
        &self,
        token: Option<String>,
        persist: bool,
        save: impl FnOnce(Option<&str>) -> bool + Send + 'static,
    ) {
        if !super::wayland_available() {
            self.unavailable();
            return;
        }
        let epoch = self.shared.epoch.fetch_add(1, Ordering::SeqCst) + 1;
        self.shared.clear();
        self.shared.status(epoch, Permission::Requesting);
        self.shared.changed.notify_waiters();
        if self
            .send
            .send(Command::Request {
                epoch,
                token,
                persist,
                save: Box::new(save),
            })
            .is_err()
        {
            self.shared.status(epoch, Permission::Unavailable);
        }
    }
    /// Invalidate immediately, including while the desktop's permission dialog is open.
    pub fn disable(&self) {
        let epoch = self.shared.epoch.fetch_add(1, Ordering::SeqCst) + 1;
        self.shared.clear();
        self.shared.status(epoch, Permission::Off);
        self.shared.changed.notify_waiters();
        let _ = self.send.send(Command::Close(None));
    }
    pub fn shutdown(&self) {
        self.disable();
        let (send, receive) = mpsc::sync_channel(1);
        if self.send.send(Command::Close(Some(send))).is_ok() {
            let _ = receive.recv_timeout(Duration::from_secs(3));
        }
    }
    pub fn unavailable(&self) {
        self.shared
            .status(self.shared.epoch.load(Ordering::SeqCst), Permission::Unavailable);
    }
    pub fn focus_changed(&self, focused: bool) {
        self.shared.local_focus.store(focused, Ordering::SeqCst);
        self.shared.focus_epoch.fetch_add(1, Ordering::SeqCst);
    }
    pub fn capture(&self) -> Option<super::Target> {
        let epoch = self.shared.epoch.load(Ordering::SeqCst);
        let local_focus = self.shared.focus_epoch.load(Ordering::SeqCst);
        if self.shared.local_focus.load(Ordering::SeqCst) || !self.state().ready() {
            return None;
        }
        let snapshot = self.shared.focus.lock().ok()?.as_ref()?.snapshot();
        let window = snapshot.window?;
        if !plain_editor(&window.class) && !window.is_terminal() {
            return None;
        }
        let process = process(window.pid?)?;
        (self.shared.valid(epoch) && self.shared.focus_epoch.load(Ordering::SeqCst) == local_focus).then(
            || {
                super::Target::Wayland(Target {
                    portal: self.clone(),
                    epoch,
                    local_focus,
                    sequence: snapshot.sequence,
                    window,
                    process,
                })
            },
        )
    }
}

async fn close(session: &Session<RemoteDesktop>) {
    let _ = tokio::time::timeout(Duration::from_millis(500), session.close()).await;
}
async fn run(shared: Arc<Shared>, mut receive: channel::UnboundedReceiver<Command>) {
    let mut pending = None;
    loop {
        let Some(command) = (match pending.take() {
            Some(command) => Some(command),
            None => receive.recv().await,
        }) else {
            break;
        };
        let Command::Request {
            epoch,
            token,
            persist,
            save,
        } = command
        else {
            match command {
                Command::Check { reply, .. } => {
                    let _ = reply.send(false);
                }
                Command::Paste { reply, .. } => {
                    let _ = reply.send(false);
                }
                Command::Close(Some(reply)) => {
                    let _ = reply.send(());
                }
                _ => {}
            }
            continue;
        };
        if !shared.valid(epoch) {
            continue;
        }
        let Ok(Ok(portal)) = tokio::time::timeout(Duration::from_secs(2), async {
            RemoteDesktop::with_connection(zbus::Connection::session().await?).await
        })
        .await
        else {
            shared.status(epoch, Permission::Unavailable);
            continue;
        };
        if !shared.valid(epoch) {
            continue;
        }
        let session = tokio::select! {
            _ = shared.changed.notified() => continue,
            result = tokio::time::timeout(Duration::from_secs(2), portal.create_session(Default::default())) => result,
        };
        let Ok(Ok(session)) = session else {
            shared.status(epoch, Permission::Unavailable);
            continue;
        };
        if !shared.valid(epoch) {
            close(&session).await;
            continue;
        }
        let Ok(closed) = session.receive_closed().await else {
            close(&session).await;
            shared.clear();
            shared.status(epoch, Permission::Unavailable);
            continue;
        };
        futures_util::pin_mut!(closed);
        let configured = async {
            let devices = SelectDevicesOptions::default()
                .set_devices(Some(DeviceType::Keyboard.into()))
                .set_persist_mode(if persist {
                    PersistMode::ExplicitlyRevoked
                } else {
                    PersistMode::DoNot
                })
                .set_restore_token(token.as_deref());
            portal
                .select_devices(&session, devices)
                .await
                .ok()?
                .response()
                .ok()?;
            if !shared.valid(epoch) {
                return None;
            }
            let selected = portal
                .start(&session, None, Default::default())
                .await
                .ok()?
                .response()
                .ok()?;
            if selected.devices() != DeviceType::Keyboard {
                return None;
            }
            Some(selected)
        };
        let selected = tokio::select! {
            _ = closed.next() => None,
            _ = shared.changed.notified() => None,
            result = tokio::time::timeout(Duration::from_secs(120), configured) => result.ok().flatten(),
        };
        let Some(selected) = selected.filter(|_| shared.valid(epoch)) else {
            close(&session).await;
            shared.status(epoch, Permission::Unavailable);
            continue;
        };
        let watch = kwin::Watch::start().await;
        let Some(watch) = watch else {
            close(&session).await;
            shared.status(epoch, Permission::Unavailable);
            continue;
        };
        if !shared.valid(epoch) {
            watch.close().await;
            close(&session).await;
            continue;
        }
        if closed.next().now_or_never().is_some() {
            watch.close().await;
            close(&session).await;
            shared.status(epoch, Permission::Unavailable);
            continue;
        }
        let watch = Arc::new(watch);
        let saved = persist && save(selected.restore_token());
        drop(selected);
        if !shared.valid(epoch) {
            watch.close().await;
            close(&session).await;
            continue;
        }
        if let Ok(mut focus) = shared.focus.lock() {
            *focus = Some(watch.clone());
        }
        shared.status(
            epoch,
            if persist && saved {
                Permission::Ready
            } else {
                Permission::Temporary
            },
        );
        while shared.valid(epoch) && shared.state.lock().is_ok_and(|s| s.ready()) {
            let command = tokio::select! {
                _ = shared.changed.notified() => break,
                _ = closed.next() => break,
                command = receive.recv() => command,
            };
            match command {
                Some(Command::Check { target, reply }) => {
                    let _ = reply.send(current(&watch, &target).await);
                }
                Some(Command::Paste {
                    live,
                    target,
                    lease,
                    reply,
                }) => {
                    let result = paste(&portal, &session, &watch, &target, &lease, &live).await;
                    let _ = reply.send(result);
                    if !result && !shared.valid(epoch) {
                        break;
                    }
                }
                Some(command @ Command::Request { .. }) => {
                    pending = Some(command);
                    break;
                }
                Some(command @ Command::Close(Some(_))) => {
                    pending = Some(command);
                    break;
                }
                Some(Command::Close(None)) | None => break,
            }
        }
        shared.clear();
        watch.close().await;
        close(&session).await;
        shared.status(epoch, Permission::Unavailable);
    }
}

async fn paste(
    portal: &RemoteDesktop,
    session: &Session<RemoteDesktop>,
    watch: &kwin::Watch,
    target: &Target,
    lease: &wayland_lease::Lease,
    live: &AtomicBool,
) -> bool {
    if target.window.is_terminal() || !live.load(Ordering::SeqCst) || !lease.current() {
        return false;
    }
    if !current(watch, target).await {
        return false;
    }
    let notify = |symbol, state| async move {
        tokio::time::timeout(
            Duration::from_millis(100),
            portal.notify_keyboard_keysym(session, symbol, state, Default::default()),
        )
        .await
        .is_ok_and(|r| r.is_ok())
    };
    if !live.load(Ordering::SeqCst) || !target.valid() {
        return false;
    }
    let ctrl = notify(0xffe3, KeyState::Pressed).await;
    let v = ctrl && live.load(Ordering::SeqCst) && target.valid() && notify(0x76, KeyState::Pressed).await;
    let release_v = notify(0x76, KeyState::Released).await;
    let release_ctrl = notify(0xffe3, KeyState::Released).await;
    if !ctrl || !v || !release_v || !release_ctrl {
        target.portal.shared.status(target.epoch, Permission::Unavailable);
        // Close a faulty virtual keyboard too, so a timed-out press cannot remain held.
        close(session).await;
        return false;
    }
    true
}

async fn current(watch: &kwin::Watch, target: &Target) -> bool {
    let Some(checkpoint) = watch.checkpoint().await else {
        return false;
    };
    checkpoint.sequence == target.sequence
        && checkpoint.window.as_ref() == Some(&target.window)
        && target.window.pid.and_then(process).as_deref() == Some(&target.process)
        && target.valid()
}

#[cfg(test)]
pub(super) mod tests;
