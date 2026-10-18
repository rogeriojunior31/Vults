use super::*;
use std::time::{Duration, Instant};
use x11rb::wrapper::ConnectionExt as _;

#[test]
fn only_native_x11_and_plain_editors_are_eligible() {
    assert!(native_x11("x11", "", ":0"));
    for (session, wayland, display) in [
        ("", "", ":0"),
        ("wayland", "", ":0"),
        ("x11", "wayland-1", ":0"),
        ("x11", "", ""),
    ] {
        assert!(!native_x11(session, wayland, display));
    }
    for class in [
        "KWrite",
        "org.kde.kwrite",
        "org.gnome.TextEditor",
        "mousepad",
        "Leafpad",
    ] {
        assert!(plain_editor(class));
    }
    for class in [
        "konsole", "kitty", "XTerm", "code", "Kate", "gedit", "firefox", "", "Unknown",
    ] {
        assert!(
            !plain_editor(class),
            "unknown fields and terminal hosts stay copy-only: {class}"
        );
    }
    assert!(process(std::process::id()).is_some());
    assert!(process(u32::MAX).is_none());
}

#[test]
#[ignore = "native insertion smoke; starts a private Xvfb and child process, never the user's display"]
fn private_x11_pastes_restores_and_preserves_new_copies_and_focus_changes() {
    if std::env::var_os("VULTS_INSERT_TEST_CHILD").is_none() {
        use std::io::{BufRead, BufReader};
        use std::process::{Child, Command, Stdio};
        struct Server(Child);
        impl Drop for Server {
            fn drop(&mut self) {
                let _ = self.0.kill();
                let _ = self.0.wait();
            }
        }
        let mut server = Server(
            Command::new("Xvfb")
                .args([
                    "-displayfd",
                    "1",
                    "-screen",
                    "0",
                    "640x480x24",
                    "-nolisten",
                    "tcp",
                ])
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .spawn()
                .expect("Xvfb"),
        );
        let mut number = String::new();
        BufReader::new(server.0.stdout.take().expect("display"))
            .read_line(&mut number)
            .expect("number");
        let display = format!(":{}", number.trim().parse::<u32>().expect("numeric display"));
        let status = Command::new(std::env::current_exe().expect("test executable"))
            .args([
                "--exact",
                "insert::tests::private_x11_pastes_restores_and_preserves_new_copies_and_focus_changes",
                "--ignored",
                "--nocapture",
            ])
            .env("VULTS_INSERT_TEST_CHILD", &display)
            .env("DISPLAY", &display)
            .env("XDG_SESSION_TYPE", "x11")
            .env_remove("WAYLAND_DISPLAY")
            .status()
            .expect("isolated test child");
        assert!(status.success());
        return;
    }
    assert_eq!(
        std::env::var("DISPLAY").ok(),
        std::env::var("VULTS_INSERT_TEST_CHILD").ok()
    );
    let (conn, screen) = x11rb::connect(None).expect("private display");
    let root = &conn.setup().roots[screen];
    let active = atom(&conn, "_NET_ACTIVE_WINDOW").expect("active atom");
    let pid = atom(&conn, "_NET_WM_PID").expect("pid atom");
    let window = conn.generate_id().expect("window");
    conn.create_window(
        x11rb::COPY_DEPTH_FROM_PARENT,
        window,
        root.root,
        0,
        0,
        100,
        100,
        0,
        xproto::WindowClass::INPUT_OUTPUT,
        root.root_visual,
        &xproto::CreateWindowAux::new()
            .event_mask(xproto::EventMask::KEY_PRESS | xproto::EventMask::KEY_RELEASE),
    )
    .expect("create")
    .check()
    .expect("window");
    conn.map_window(window).expect("map");
    let class = |name: &str| {
        conn.change_property8(
            xproto::PropMode::REPLACE,
            window,
            xproto::AtomEnum::WM_CLASS,
            xproto::AtomEnum::STRING,
            format!("editor\0{name}\0").as_bytes(),
        )
        .expect("class")
        .check()
        .expect("class property")
    };
    let activate = |id| {
        conn.change_property32(
            xproto::PropMode::REPLACE,
            root.root,
            active,
            xproto::AtomEnum::WINDOW,
            &[id],
        )
        .expect("active")
        .check()
        .expect("active property");
        conn.set_input_focus(xproto::InputFocus::PARENT, id, x11rb::CURRENT_TIME)
            .expect("focus")
            .check()
            .expect("focus reply");
    };
    class("KWrite");
    conn.change_property32(
        xproto::PropMode::REPLACE,
        window,
        pid,
        xproto::AtomEnum::CARDINAL,
        &[std::process::id()],
    )
    .expect("pid")
    .check()
    .expect("pid property");
    activate(window);
    let mut clipboard = arboard::Clipboard::new().expect("private clipboard");
    clipboard.set_text("original text").expect("prior copy");
    conn.xkb_use_extension(1, 0)
        .expect("XKB")
        .reply()
        .expect("XKB reply");
    let min = conn.setup().min_keycode;
    let keyboard = conn
        .get_keyboard_mapping(min, conn.setup().max_keycode - min + 1)
        .expect("lock mapping")
        .reply()
        .expect("mapping reply");
    let num = min
        + u8::try_from(
            keyboard
                .keysyms
                .chunks(usize::from(keyboard.keysyms_per_keycode))
                .position(|row| row.first() == Some(&0xff7f))
                .expect("NumLock"),
        )
        .expect("NumLock keycode");
    for kind in [xproto::KEY_PRESS_EVENT, xproto::KEY_RELEASE_EVENT] {
        conn.xtest_fake_input(kind, num, 0, root.root, 0, 0, 0)
            .expect("NumLock input")
            .check()
            .expect("NumLock checked");
    }
    assert!(
        u16::from(
            conn.xkb_get_state(xkb::ID::USE_CORE_KBD.into())
                .expect("NumLock state")
                .reply()
                .expect("state reply")
                .locked_mods
        ) != 0
    );
    while conn.poll_for_event().expect("drain NumLock").is_some() {}
    let prepared =
        Lease::prepare("cancelled before claiming", || clipboard.get_text().ok()).expect("prepare only");
    assert!(!prepared.owned_on(&conn));
    assert!(clipboard.get_text().expect("untouched clipboard") == "original text");
    drop(prepared);
    assert!(clipboard.get_text().expect("cancelled preparation") == "original text");
    let target = Target::capture().expect("capture on keydown");
    let lease =
        Lease::copy("spoken\nUnicode: \u{4e2d}\u{6587}", || clipboard.get_text().ok()).expect("lease");
    assert!(clipboard.get_text().expect("independent read") == "spoken\nUnicode: \u{4e2d}\u{6587}");
    assert_eq!(format!("{target:?} {lease:?}"), "Target(X11) Lease(X11)");
    assert!(target.paste(&lease));
    let mut events = Vec::new();
    let until = Instant::now() + Duration::from_secs(1);
    while Instant::now() < until && events.len() < 4 {
        match conn.poll_for_event().expect("events") {
            Some(Event::KeyPress(e)) => events.push((e.detail, true, e.state)),
            Some(Event::KeyRelease(e)) => events.push((e.detail, false, e.state)),
            Some(_) => {}
            None => std::thread::sleep(Duration::from_millis(5)),
        }
    }
    assert_eq!(events.len(), 4, "the foreground receives only Ctrl+V down and up");
    let symbols = conn
        .get_keyboard_mapping(
            conn.setup().min_keycode,
            conn.setup().max_keycode - conn.setup().min_keycode + 1,
        )
        .expect("dispatched symbols")
        .reply()
        .expect("symbols reply");
    let symbol = |code: u8| {
        symbols.keysyms
            [usize::from(code - conn.setup().min_keycode) * usize::from(symbols.keysyms_per_keycode)]
    };
    assert_eq!(symbol(events[0].0), 0xffe3);
    assert_eq!(symbol(events[1].0), 0x76, "V, never Enter");
    assert_eq!(events[0].0, events[3].0);
    assert_eq!(events[1].0, events[2].0);
    assert!(events[1].2.contains(xproto::KeyButMask::CONTROL));
    assert!(
        conn.query_keymap()
            .expect("keys")
            .reply()
            .expect("keys reply")
            .keys
            .iter()
            .all(|b| *b == 0)
    );
    lease.pasted();
    wait_for(|| clipboard.get_text().is_ok_and(|t| t == "original text"));

    // Copying the exact transcript is a new owner too: a text-only comparison would lose it.
    let lease = Lease::copy("same text", || clipboard.get_text().ok()).expect("same-text lease");
    clipboard.set_text("same text").expect("manual same-text copy");
    lease.pasted();
    std::thread::sleep(Duration::from_millis(780));
    assert!(clipboard.get_text().expect("new copy") == "same text");
    assert!(!lease.owned_on(&conn));

    // The previous reader raced a same-owner copy; do not restore that stale snapshot later.
    let raced = Lease::copy("discarded transcript", || {
        let before = clipboard.get_text().ok();
        clipboard.set_text("newer copy during read").expect("racing copy");
        before
    });
    assert!(raced.is_none());
    assert!(clipboard.get_text().expect("raced text") == "newer copy during read");

    clipboard.set_text("baseline").expect("baseline");
    let first = Lease::copy("first dictation", || clipboard.get_text().ok()).expect("first lease");
    first.pasted();
    let second = Lease::copy("second dictation", || {
        first.previous().unwrap_or_else(|| clipboard.get_text().ok())
    })
    .expect("overlapping lease");
    drop(first);
    assert!(clipboard.get_text().expect("second text") == "second dictation");
    second.pasted();
    wait_for(|| clipboard.get_text().is_ok_and(|t| t == "baseline"));

    let target = Target::capture().expect("focus target");
    let lease = Lease::copy("clipboard fallback", || clipboard.get_text().ok()).expect("fallback lease");
    activate(root.root);
    activate(window);
    assert!(
        !target.paste(&lease),
        "away-and-back is not the original uninterrupted focus"
    );
    lease.copied();
    assert!(lease.previous() == Some(Some("clipboard fallback".into())));
    std::thread::sleep(Duration::from_millis(780));
    assert!(clipboard.get_text().expect("fallback stays") == "clipboard fallback");
    let target = Target::capture().expect("fresh target");
    clipboard
        .set_text("new clipboard before paste")
        .expect("new clipboard");
    assert!(!target.paste(&lease), "never paste another copy's text");
    assert!(conn.poll_for_event().expect("no synthetic keys").is_none());

    let target = Target::capture().expect("held-key target");
    let held_lease = Lease::copy("held key fallback", || clipboard.get_text().ok()).expect("held lease");
    let min = conn.setup().min_keycode;
    let map = conn
        .get_keyboard_mapping(min, conn.setup().max_keycode - min + 1)
        .expect("keymap")
        .reply()
        .expect("keymap reply");
    let ctrl = min
        + u8::try_from(
            map.keysyms
                .chunks(usize::from(map.keysyms_per_keycode))
                .position(|row| row.first() == Some(&0xffe3))
                .expect("Control key"),
        )
        .expect("keycode");
    conn.xtest_fake_input(xproto::KEY_PRESS_EVENT, ctrl, 0, root.root, 0, 0, 0)
        .expect("hold")
        .check()
        .expect("hold checked");
    assert!(
        !target.paste(&held_lease),
        "a held key blocks injection rather than releasing it"
    );
    let held = conn
        .query_keymap()
        .expect("held keys")
        .reply()
        .expect("held reply");
    assert!(held.keys[usize::from(ctrl / 8)] & (1 << (ctrl % 8)) != 0);
    conn.xtest_fake_input(xproto::KEY_RELEASE_EVENT, ctrl, 0, root.root, 0, 0, 0)
        .expect("release")
        .check()
        .expect("release checked");
    while conn.poll_for_event().expect("drain held-key events").is_some() {}
    assert!(
        Lease::copy(&"x".repeat(65 * 1024), || None).is_none(),
        "oversized text stays on the ordinary copy path"
    );
    assert!(Lease::copy("invalid\0text", || None).is_none());

    class("Konsole");
    let terminal = Target::capture().expect("terminal focus capability");
    let lease = Lease::copy("must not paste", || None).expect("lease");
    assert!(
        !terminal.paste(&lease),
        "a terminal capability cannot dispatch a generic paste chord"
    );
    class("Code");
    assert!(
        Target::capture().is_none(),
        "an editor's title cannot prove its focused field"
    );
    class("KWrite");
    let target = Target::capture().expect("closing target");
    let lease = Lease::copy("closed target fallback", || clipboard.get_text().ok()).expect("closed lease");
    conn.destroy_window(window)
        .expect("close")
        .check()
        .expect("closed");
    assert!(!target.paste(&lease));
    assert!(lease.owned_on(&conn));
}

fn wait_for(mut condition: impl FnMut() -> bool) {
    let until = Instant::now() + Duration::from_secs(3);
    while Instant::now() < until {
        if condition() {
            return;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    panic!("clipboard condition was not reached before the deadline");
}
