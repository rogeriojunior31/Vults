//! Native clipboard and focus checks run only in a private compositor and D-Bus session.
use super::*;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

#[test]
#[ignore = "native Wayland smoke; starts a private headless KWin and D-Bus, never the user's session"]
fn private_wayland_lease_restores_without_reclaiming_and_focus_checkpoints_are_live() {
    if std::env::var_os("VULTS_WAYLAND_TEST_CHILD").is_none() {
        use std::os::unix::fs::PermissionsExt;
        let dir = std::env::temp_dir().join(format!("vults-wayland-smoke-{}", std::process::id()));
        std::fs::create_dir(&dir).expect("private runtime directory");
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)).expect("private permissions");
        let result = Command::new("dbus-run-session").arg("--").arg(std::env::current_exe().expect("test binary"))
            .args(["--exact", "insert::wayland_tests::private_wayland_lease_restores_without_reclaiming_and_focus_checkpoints_are_live", "--ignored", "--nocapture"])
            .env("VULTS_WAYLAND_TEST_CHILD", "1").env("XDG_RUNTIME_DIR", &dir)
            .env("XDG_CONFIG_HOME", dir.join("config")).env("XDG_DATA_HOME", dir.join("data"))
            .env("XDG_CACHE_HOME", dir.join("cache")).env("XDG_STATE_HOME", dir.join("state"))
            .env("WAYLAND_DISPLAY", "vults-wayland-test").env("XDG_SESSION_TYPE", "wayland").env("XDG_CURRENT_DESKTOP", "KDE")
            .env("QT_QPA_PLATFORM", "offscreen").env("KWIN_COMPOSE", "Q").env("LIBGL_ALWAYS_SOFTWARE", "1")
            .env("NO_AT_BRIDGE", "1").env("GTK_A11Y", "none").env("GTK_USE_PORTAL", "0").env("GDK_BACKEND", "wayland").env_remove("DISPLAY").status();
        let _ = std::fs::remove_dir_all(&dir);
        assert!(result.expect("private D-Bus child").success());
        return;
    }
    struct Server(std::process::Child);
    impl Drop for Server {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    let _server = Server(
        Command::new("kwin_wayland")
            .args([
                "--virtual",
                "--width",
                "640",
                "--height",
                "480",
                "--socket",
                "vults-wayland-test",
                "--no-lockscreen",
                "--no-global-shortcuts",
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("private KWin"),
    );
    let socket = std::path::PathBuf::from(std::env::var_os("XDG_RUNTIME_DIR").expect("private runtime"))
        .join("vults-wayland-test");
    until(|| socket.exists());
    let mut clipboard = crate::clipboard::Clipboard::new().expect("native private clipboard");
    let original = "previous text ".repeat(2400);
    clipboard.copy(&original).expect("prior text");
    let transcript = "dictation \u{4e2d}\u{6587}\n".repeat(1800);
    let mut lease = Lease::prepare(&transcript, || clipboard.text()).expect("prepare");
    assert!(lease.claim());
    clipboard
        .confirm(&transcript)
        .expect("independent read, larger than pipe capacity");
    lease.pasted();
    until(|| clipboard.text().is_some_and(|text| text == original));
    let mut first = Lease::prepare("same transcript", || clipboard.text()).expect("new lease");
    assert!(first.claim());
    clipboard.confirm("same transcript").expect("new copy");
    clipboard
        .copy("same transcript")
        .expect("user copy with equal contents");
    first.pasted();
    std::thread::sleep(Duration::from_millis(800));
    clipboard
        .confirm("same transcript")
        .expect("a newer equal copy survives restoration");
    let mut first = Lease::prepare("first", || clipboard.text()).expect("overlap first");
    assert!(first.claim());
    first.pasted();
    let mut second = Lease::prepare("second", || first.previous().unwrap_or_else(|| clipboard.text()))
        .expect("overlap second");
    assert!(second.claim());
    second.pasted();
    until(|| clipboard.text().is_some_and(|text| text == "same transcript"));
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime");
    runtime.block_on(async {
        use gtk::prelude::*;
        use std::sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        };
        let (mock, connection) = portal::tests::Mock::server().await;
        gtk::glib::set_prgname(Some("kwrite"));
        gtk::init().expect("private GTK");
        let application = gtk::Application::new(Some("org.kde.kwrite"), Default::default());
        application
            .register(None::<&gtk::gio::Cancellable>)
            .expect("private application");
        let window = gtk::ApplicationWindow::new(&application);
        window.set_title("Private dictation test");
        window.show_all();
        for _ in 0..30 {
            pump();
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        focus_private_window(&connection, "Private dictation test").await;
        let watch = kwin::Watch::start().await.expect("native focus monitor");
        let first = watch.checkpoint().await.expect("first compositor checkpoint");
        assert!(
            first.window.as_ref().is_some_and(|w| plain_editor(&w.class)),
            "the private editor is recognized: {:?}",
            first.window.as_ref().map(|w| &w.class)
        );
        let styles = crate::focus::Tracker::start()
            .await
            .expect("read-only style tracker without a portal grant");
        let frozen = styles.snapshot().expect("key-down application context");
        assert_eq!(Some(&frozen), first.window.as_ref());
        assert_eq!(
            mock.starts.load(Ordering::SeqCst),
            0,
            "Style context never requests keyboard permission"
        );
        let other = gtk::ApplicationWindow::new(&application);
        other.set_title("Second private editor");
        other.show_all();
        for _ in 0..30 {
            pump();
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        focus_private_window(&connection, "Second private editor").await;
        let changed = watch.checkpoint().await.expect("changed focus");
        assert!(
            changed.sequence > first.sequence,
            "same-app windows still change the focus epoch"
        );
        settle(|| {
            styles.snapshot().as_ref().map(|w| &w.identity) == changed.window.as_ref().map(|w| &w.identity)
        })
        .await;
        assert_ne!(styles.snapshot().unwrap().identity, frozen.identity);
        assert_eq!(
            Some(&frozen),
            first.window.as_ref(),
            "The recording keeps its original context"
        );
        styles.close().await;
        assert!(
            watch.checkpoint().await.is_some(),
            "Closing the style tracker cannot unload the insertion tracker"
        );
        other.close();
        for _ in 0..30 {
            pump();
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        focus_private_window(&connection, "Private dictation test").await;
        let returned = watch.checkpoint().await.expect("returned focus");
        assert!(
            returned.sequence > changed.sequence,
            "away-and-back cannot reuse a target"
        );
        watch.close().await;
        drop(watch);

        let saves = Arc::new(AtomicUsize::new(0));
        let granted = Portal::new(|_| {}).expect("portal actor");
        let saved = saves.clone();
        granted.request(Some("old-test-token".into()), true, move |token| {
            assert!(token == Some("rotated-test-token"));
            saved.fetch_add(1, Ordering::SeqCst);
            true
        });
        settle(|| granted.state() == Permission::Ready).await;
        assert_eq!(mock.starts.load(Ordering::SeqCst), 1);
        assert_eq!(saves.load(Ordering::SeqCst), 1);
        let target = Arc::new(granted.capture().expect("ready focused editor"));
        let current = target.clone();
        assert!(
            tokio::task::spawn_blocking(move || current.current() && current.unchanged())
                .await
                .expect("read-only delivery checkpoint")
        );
        let mut lease = Lease::prepare("portal dictation", || clipboard.text()).expect("portal lease");
        assert!(lease.claim());
        clipboard.confirm("portal dictation").expect("portal clipboard");
        let lease = Arc::new(lease);
        for _ in 0..2 {
            let target = target.clone();
            let lease = lease.clone();
            assert!(
                tokio::task::spawn_blocking(move || target.paste(&lease))
                    .await
                    .expect("paste worker")
            );
        }
        assert_eq!(
            mock.starts.load(Ordering::SeqCst),
            1,
            "dictations reuse the open grant"
        );
        assert_eq!(
            *mock.keys.lock().expect("keys"),
            [
                (0xffe3, 1),
                (0x76, 1),
                (0x76, 0),
                (0xffe3, 0),
                (0xffe3, 1),
                (0x76, 1),
                (0x76, 0),
                (0xffe3, 0)
            ]
        );
        let path = mock.path.lock().expect("session path").clone();
        connection
            .emit_signal(
                None::<&str>,
                path.as_str(),
                "org.freedesktop.portal.Session",
                "Closed",
                &(std::collections::HashMap::<String, zbus::zvariant::OwnedValue>::new(),),
            )
            .await
            .expect("revoke grant");
        settle(|| granted.state() == Permission::Unavailable).await;
        assert!(granted.capture().is_none());
        let old_target = target.clone();
        let old_lease = lease.clone();
        assert!(
            !tokio::task::spawn_blocking(move || old_target.paste(&old_lease))
                .await
                .expect("revoked paste")
        );
        assert_eq!(
            mock.starts.load(Ordering::SeqCst),
            1,
            "revocation never reopens a dialog"
        );
        mock.deny.store(true, Ordering::SeqCst);
        granted.request(Some("old-test-token".into()), true, |_| {
            panic!("a refusal cannot save a token")
        });
        settle(|| mock.starts.load(Ordering::SeqCst) == 2 && granted.state() == Permission::Unavailable)
            .await;
        for _ in 0..10 {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert_eq!(
            mock.starts.load(Ordering::SeqCst),
            2,
            "a refusal is not retried automatically"
        );
        mock.deny.store(false, Ordering::SeqCst);
        mock.hold.store(true, Ordering::SeqCst);
        let closed = mock.closes.load(Ordering::SeqCst);
        granted.request(Some("old-test-token".into()), true, |_| {
            panic!("cancelled consent cannot save a token")
        });
        settle(|| mock.starts.load(Ordering::SeqCst) == 3).await;
        granted.disable();
        settle(|| mock.closes.load(Ordering::SeqCst) > closed).await;
        assert_eq!(granted.state(), Permission::Off);
        assert!(granted.capture().is_none());
        mock.hold.store(false, Ordering::SeqCst);
        mock.persist.store(0, Ordering::SeqCst);
        granted.request(None, false, |_| {
            panic!("a nonpersistent grant must not save a token")
        });
        settle(|| granted.state() == Permission::Temporary).await;
        let target = Arc::new(granted.capture().expect("temporary grant editor"));
        granted.focus_changed(true);
        let lease = lease.clone();
        assert!(
            !tokio::task::spawn_blocking(move || target.paste(&lease))
                .await
                .expect("local focus invalidation")
        );
        granted.disable();
        window.close();
        pump();
    });
}
fn until(mut check: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if check() {
            return;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    panic!("native condition did not settle before the deadline");
}

fn pump() {
    while gtk::events_pending() {
        gtk::main_iteration_do(false);
    }
}
async fn settle(mut check: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        pump();
        if check() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("portal state did not settle before the deadline");
}

async fn focus_private_window(conn: &zbus::Connection, caption: &str) {
    assert!(
        std::env::var_os("VULTS_WAYLAND_TEST_CHILD").is_some(),
        "private compositor only"
    );
    let code = format!(
        "var windows = workspace.windowList(); for (var i = 0; i < windows.length; i++) {{ if (windows[i].pid === {} && windows[i].caption === {}) workspace.activeWindow = windows[i]; }}",
        std::process::id(),
        serde_json::to_string(caption).expect("test caption")
    );
    let name = "vults-private-test-focus";
    let file = crate::jump::kwin::write_script(
        &crate::jump::kwin::script_dir().expect("private dir"),
        name,
        &code,
    )
    .expect("test script");
    let proxy = zbus::Proxy::new(conn, "org.kde.KWin", "/Scripting", "org.kde.kwin.Scripting")
        .await
        .expect("private KWin");
    let id: i32 = proxy
        .call("loadScript", &(file.0.to_str().expect("path"), name))
        .await
        .expect("load private script");
    let path = format!("/Scripting/Script{id}");
    let script = zbus::Proxy::new(conn, "org.kde.KWin", path, "org.kde.kwin.Script")
        .await
        .expect("script proxy");
    script.call::<_, _, ()>("run", &()).await.expect("private focus");
    tokio::time::sleep(Duration::from_millis(20)).await;
    let _: bool = proxy
        .call("unloadScript", &(name,))
        .await
        .expect("unload private test script");
}
