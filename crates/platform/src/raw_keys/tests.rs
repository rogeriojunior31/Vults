use super::*;
use std::sync::mpsc;

fn filter(bindings: &[&str]) -> Option<Filter> {
    let symbols = [
        0xff1b, 0, 0x20, 0, 0xffe1, 0, 0xffe3, 0, 0xffe9, 0, 0x76, 0x56, 0x61, 0x41, 0x62, 0x42, 0xffc9, 0,
        0xffeb, 0,
    ];
    let mut held = [0; 32];
    for code in [11, 12] {
        held[code / 8] |= 1 << (code % 8);
    }
    Filter::new(
        8,
        2,
        &symbols,
        &[10, 11, 12, 17],
        held,
        &bindings.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
    )
}

#[test]
fn symbols_classify_actions_without_retaining_typed_text() {
    for (symbol, expected) in [
        (0xff1b, Some(Key::Escape)),
        (0x20, Some(Key::Space)),
        (0xff80, Some(Key::Space)),
        (0x61, Some(Key::Other)),
        (0xffbe, Some(Key::Other)),
        (0xff0d, Some(Key::Other)),
        (0xffe1, None),
        (0xffe4, None),
        (0xff7f, None),
        (0xfe03, None),
        (0xfe11, None),
        (0, None),
    ] {
        assert_eq!(classify(symbol), expected);
    }
    assert!(!supports("wayland", "wayland-1", ":0"));
    assert!(!supports("x11", "wayland-1", ":0"));
    assert!(!supports("x11", "", ""));
    assert!(supports("x11", "", ":0"));
}

#[test]
fn actual_shortcut_chords_and_repeats_do_not_cancel_their_recording() {
    let mut f = filter(&["Ctrl+Alt+V", "CTRL+ALT+B"]).expect("filter");
    assert_eq!(f.press(13, false), None);
    f.release(13);
    assert_eq!(f.press(13, false), None, "second talk tap belongs to the portal");
    assert_eq!(
        f.press(15, false),
        None,
        "Copy again cannot cancel active dictation"
    );
    assert_eq!(f.press(9, true), None);
    assert_eq!(f.press(10, false), None);
    assert_eq!(f.press(14, false), Some(Key::Other));
    assert_eq!(f.press(9, false), Some(Key::Space));
    assert_eq!(f.press(8, false), Some(Key::Escape));
    f.release(11);
    f.release(12);
    f.release(13);
    assert_eq!(
        f.press(13, false),
        Some(Key::Other),
        "a bare letter is not the shortcut"
    );
    assert_eq!(f.press(u32::MAX, false), None);
    f.release(u32::MAX);
    let mut f = filter(&["Ctrl+Alt+F12"]).expect("remapped filter");
    assert_eq!(f.press(16, false), None);
    assert_eq!(f.press(13, false), Some(Key::Other), "no hardcoded talk letter");
    let mut f = filter(&["Ctrl+Alt+Esc"]).expect("escape binding");
    assert_eq!(f.press(8, false), None);
    f.release(11);
    f.release(8);
    assert_eq!(f.press(8, false), Some(Key::Escape));
    assert!(filter(&["Unrecognized+V"]).is_none());
    assert!(filter(&["Ctrl+Alt+UnknownKey"]).is_none());
    assert_eq!(keyval("Space"), Some(0x20));
    assert_eq!(keyval("F12"), Some(0xffc9));
    assert_eq!(keyval("V"), Some(0x76));
    assert!(uses_space("Ctrl+Alt+Space"));
    assert!(!uses_space("Ctrl+Alt+F2"));
    assert!(!uses_space("Ctrl++"));
}

struct FakeConnection {
    next: mpsc::Receiver<Result<Key, ()>>,
    closed: mpsc::Sender<()>,
}
impl FakeConnection {
    fn next(&self) -> Result<Option<Key>, ()> {
        match self.next.try_recv() {
            Ok(Ok(key)) => Ok(Some(key)),
            Ok(Err(())) | Err(mpsc::TryRecvError::Disconnected) => Err(()),
            Err(mpsc::TryRecvError::Empty) => Ok(None),
        }
    }
}
impl Drop for FakeConnection {
    fn drop(&mut self) {
        let _ = self.closed.send(());
    }
}

#[test]
fn dropping_revoking_or_losing_the_connection_stops_observation() {
    for reason in ["drop", "revoke", "disconnect"] {
        let (send, receive) = mpsc::channel();
        let (closed, ended) = mpsc::channel();
        let (emit, events) = mpsc::channel();
        let connection = FakeConnection {
            next: receive,
            closed,
        };
        let active = Arc::new(AtomicBool::new(true));
        let live = Arc::new(AtomicBool::new(true));
        let listener = Listener {
            active: active.clone(),
        };
        let watching = live.clone();
        let flag = active.clone();
        let thread = std::thread::spawn(move || {
            pump(
                || connection.next(),
                &flag,
                || watching.load(Ordering::SeqCst),
                |key| {
                    let _ = emit.send(key);
                },
            );
            flag.store(false, Ordering::SeqCst);
        });
        send.send(Ok(Key::Space)).expect("key");
        assert_eq!(
            events.recv_timeout(Duration::from_secs(1)).expect("event"),
            Key::Space
        );
        match reason {
            "drop" => drop(listener),
            "revoke" => live.store(false, Ordering::SeqCst),
            _ => {
                send.send(Err(())).expect("disconnect");
            }
        }
        ended
            .recv_timeout(Duration::from_secs(1))
            .expect("connection closed");
        thread.join().expect("listener thread");
        assert!(!active.load(Ordering::SeqCst));
        assert!(events.try_recv().is_err());
    }
}

#[test]
#[ignore = "native XI2 smoke; requires Xvfb and injects keys only into its private server"]
fn private_x11_server_delivers_raw_chords_without_grabbing_the_foreground() {
    use std::io::{BufRead, BufReader};
    use std::process::{Child, Command, Stdio};
    use x11rb::protocol::{xproto, xtest::ConnectionExt as _};
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
    BufReader::new(server.0.stdout.take().expect("display number"))
        .read_line(&mut number)
        .expect("read display");
    let display = format!(":{}", number.trim().parse::<u32>().expect("numeric display"));
    let bindings = ["Ctrl+Alt+V".to_owned(), "Ctrl+Alt+B".to_owned()];
    let input = Input::open(&bindings, Some(&display)).expect("XI 2.1 listener");
    let (send, receive) = mpsc::channel();
    let listener = start(
        input,
        || true,
        move |key| {
            let _ = send.send(key);
        },
    )
    .expect("start listener");
    let active = listener.active();
    let (injector, screen) = x11rb::connect(Some(&display)).expect("test client");
    let root = &injector.setup().roots[screen];
    let window = injector.generate_id().expect("window id");
    injector
        .create_window(
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
        .expect("window")
        .check()
        .expect("create window");
    injector.map_window(window).expect("map");
    injector
        .set_input_focus(xproto::InputFocus::PARENT, window, x11rb::CURRENT_TIME)
        .expect("focus");
    let min = injector.setup().min_keycode;
    let max = injector.setup().max_keycode;
    let map = injector
        .get_keyboard_mapping(min, max - min + 1)
        .expect("keymap")
        .reply()
        .expect("keymap reply");
    let code = |symbol| {
        map.keysyms
            .chunks(usize::from(map.keysyms_per_keycode))
            .position(|row| row.contains(&symbol))
            .map(|i| min + u8::try_from(i).expect("keycode"))
            .expect("symbol in map")
    };
    let press = |symbol, down| {
        injector
            .xtest_fake_input(
                if down {
                    xproto::KEY_PRESS_EVENT
                } else {
                    xproto::KEY_RELEASE_EVENT
                },
                code(symbol),
                0,
                root.root,
                0,
                0,
                0,
            )
            .expect("synthetic key");
    };
    press(0xffe3, true);
    press(0xffe9, true);
    for symbol in [0x76, 0x62, 0x61, 0x20, 0xff1b] {
        press(symbol, true);
        press(symbol, false);
    }
    press(0xffe9, false);
    press(0xffe3, false);
    injector.flush().expect("flush");
    for expected in [Key::Other, Key::Space, Key::Escape] {
        assert_eq!(
            receive.recv_timeout(Duration::from_secs(2)).expect("raw key"),
            expected
        );
    }
    let mut foreground = BTreeSet::new();
    let until = std::time::Instant::now() + Duration::from_secs(1);
    while std::time::Instant::now() < until && foreground.len() < 3 {
        match injector.poll_for_event().expect("foreground event") {
            Some(Event::KeyPress(e)) if [code(0x61), code(0x20), code(0xff1b)].contains(&e.detail) => {
                foreground.insert(e.detail);
            }
            Some(_) => {}
            None => std::thread::sleep(Duration::from_millis(5)),
        }
    }
    assert_eq!(
        foreground.len(),
        3,
        "raw observation never consumes the foreground's keys"
    );
    drop(listener);
    assert!(!active.load(Ordering::SeqCst));
    assert!(
        matches!(
            receive.recv_timeout(Duration::from_secs(1)),
            Err(mpsc::RecvTimeoutError::Disconnected)
        ),
        "stopping closes the native listener and callback"
    );
}
