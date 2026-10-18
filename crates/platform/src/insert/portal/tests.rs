use super::*;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, AtomicUsize};
use zbus::zvariant::{OwnedObjectPath, OwnedValue, Str};

type Options = HashMap<String, OwnedValue>;
#[derive(Default)]
pub(crate) struct Mock {
    pub persist: AtomicU32,
    pub starts: AtomicUsize,
    pub closes: AtomicUsize,
    pub deny: AtomicBool,
    pub hold: AtomicBool,
    pub keys: Mutex<Vec<(i32, u32)>>,
    pub path: Mutex<String>,
}
struct Desktop(Arc<Mock>);
struct SessionMock(Arc<Mock>);

fn string(options: &Options, name: &str) -> String {
    options
        .get(name)
        .and_then(|v| v.try_clone().ok())
        .and_then(|v| String::try_from(v).ok())
        .unwrap_or_default()
}
async fn response(
    conn: &zbus::Connection,
    sender: &str,
    options: &Options,
    code: u32,
    result: Options,
    emit: bool,
) -> zbus::fdo::Result<OwnedObjectPath> {
    let path = format!(
        "/org/freedesktop/portal/desktop/request/{}/{}",
        sender.trim_start_matches(':').replace('.', "_"),
        string(options, "handle_token")
    );
    if emit {
        conn.emit_signal(
            Some(sender),
            path.as_str(),
            "org.freedesktop.portal.Request",
            "Response",
            &(code, result),
        )
        .await?;
    }
    OwnedObjectPath::try_from(path).map_err(|e| zbus::fdo::Error::Failed(e.to_string()))
}
#[zbus::interface(name = "org.freedesktop.portal.RemoteDesktop")]
impl Desktop {
    #[zbus(property)]
    fn version(&self) -> u32 {
        2
    }
    async fn create_session(
        &self,
        options: Options,
        #[zbus(connection)] conn: &zbus::Connection,
        #[zbus(header)] header: zbus::message::Header<'_>,
    ) -> zbus::fdo::Result<OwnedObjectPath> {
        let sender = header.sender().expect("test sender").as_str();
        let path = format!(
            "/org/freedesktop/portal/desktop/session/{}/{}",
            sender.trim_start_matches(':').replace('.', "_"),
            string(&options, "session_handle_token")
        );
        *self.0.path.lock().expect("path") = path.clone();
        conn.object_server()
            .at(path.as_str(), SessionMock(self.0.clone()))
            .await?;
        response(
            conn,
            sender,
            &options,
            0,
            HashMap::from([("session_handle".into(), Str::from(path).into())]),
            true,
        )
        .await
    }
    async fn select_devices(
        &self,
        _session: OwnedObjectPath,
        options: Options,
        #[zbus(connection)] conn: &zbus::Connection,
        #[zbus(header)] header: zbus::message::Header<'_>,
    ) -> zbus::fdo::Result<OwnedObjectPath> {
        assert_eq!(
            options.get("types").and_then(|v| u32::try_from(v).ok()),
            Some(1),
            "keyboard only"
        );
        assert_eq!(
            options.get("persist_mode").and_then(|v| u32::try_from(v).ok()),
            Some(self.0.persist.load(Ordering::SeqCst))
        );
        assert!(
            string(&options, "restore_token")
                == if self.0.persist.load(Ordering::SeqCst) == 2 {
                    "old-test-token"
                } else {
                    ""
                }
        );
        response(
            conn,
            header.sender().expect("sender").as_str(),
            &options,
            0,
            Options::new(),
            true,
        )
        .await
    }
    async fn start(
        &self,
        _session: OwnedObjectPath,
        _window: &str,
        options: Options,
        #[zbus(connection)] conn: &zbus::Connection,
        #[zbus(header)] header: zbus::message::Header<'_>,
    ) -> zbus::fdo::Result<OwnedObjectPath> {
        self.0.starts.fetch_add(1, Ordering::SeqCst);
        response(
            conn,
            header.sender().expect("sender").as_str(),
            &options,
            u32::from(self.0.deny.load(Ordering::SeqCst)),
            HashMap::from([
                ("devices".into(), 1u32.into()),
                ("restore_token".into(), Str::from("rotated-test-token").into()),
            ]),
            !self.0.hold.load(Ordering::SeqCst),
        )
        .await
    }
    async fn notify_keyboard_keysym(
        &self,
        _session: OwnedObjectPath,
        _options: Options,
        symbol: i32,
        state: u32,
    ) {
        self.0.keys.lock().expect("keys").push((symbol, state));
    }
}
#[zbus::interface(name = "org.freedesktop.portal.Session")]
impl SessionMock {
    fn close(&self) {
        self.0.closes.fetch_add(1, Ordering::SeqCst);
    }
}
impl Mock {
    pub(crate) async fn server() -> (Arc<Self>, zbus::Connection) {
        let mock = Arc::new(Self::default());
        mock.persist.store(2, Ordering::SeqCst);
        let conn = zbus::connection::Builder::session()
            .expect("private bus")
            .name("org.freedesktop.portal.Desktop")
            .expect("name")
            .serve_at("/org/freedesktop/portal/desktop", Desktop(mock.clone()))
            .expect("mock desktop")
            .build()
            .await
            .expect("mock connection");
        (mock, conn)
    }
}

#[test]
fn disabling_and_retrying_revoke_old_targets_before_any_async_cleanup() {
    let portal = Portal::new(|_| {}).expect("worker");
    let epoch = portal.shared.epoch.load(Ordering::SeqCst);
    portal.shared.status(epoch, Permission::Ready);
    portal.disable();
    assert!(!portal.shared.valid(epoch));
    assert_eq!(portal.state(), Permission::Off);
    portal.shared.status(epoch, Permission::Ready);
    assert_eq!(
        portal.state(),
        Permission::Off,
        "late grants cannot re-enable a closed session"
    );
    assert!(portal.capture().is_none());
}
