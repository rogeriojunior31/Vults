use super::*;

#[test]
fn terminal_payloads_are_literal_bounded_and_never_submit() {
    for text in ["", "yes\n", "ok\r", "a\tb", "\u{1b}[201~", "\u{7f}", "\u{2028}"] {
        assert!(send_args("%3", 42, 43, text).is_none());
    }
    assert!(send_args("%3; run-shell bad", 42, 43, "hello").is_none());
    assert!(send_args("%3", 42, 43, &"a".repeat(8193)).is_none());
    let text = "-- ; $(bad) `bad` \\n \u{4e2d}";
    let args = send_args("%3", 42, 43, text).expect("literal text");
    assert_eq!(&args[..4], ["if-shell", "-F", "-t", "%3"]);
    assert!(!args[5].contains(text));
    let encoded = args[5]
        .strip_prefix("send-keys -H -t %3 ")
        .expect("hex path")
        .strip_suffix(" ; display-message -p vults-delivered")
        .expect("ack");
    let decoded: Vec<_> = encoded
        .split(' ')
        .map(|b| u8::from_str_radix(b, 16).expect("hex"))
        .collect();
    assert_eq!(decoded, text.as_bytes());
}

#[test]
fn direct_wezterm_metadata_is_not_a_terminal_input_capability() {
    let window = Window {
        class: "org.wezfurlong.wezterm".into(),
        title: "agent".into(),
        pid: Some(std::process::id()),
        identity: "test-only".into(),
    };
    let terminal = Terminal {
        pid: Some(std::process::id()),
        env: [
            ("WEZTERM_PANE".into(), "17".into()),
            (
                "WEZTERM_UNIX_SOCKET".into(),
                "/run/user/1000/wezterm/gui-sock-42".into(),
            ),
        ]
        .into(),
        ..Default::default()
    };
    // WezTerm's copy/search overlay shares its underlying pane id; metadata cannot prove its input field.
    assert!(Pane::capture(&window, &terminal).is_none());
}

#[test]
fn kitty_context_requires_one_focused_pane_and_its_process_identity() {
    let mut tree = serde_json::json!([{"is_focused":true,"tabs":[{"is_focused":true,"windows":[
        {"id":7,"pid":42,"is_focused":true}
    ]}]}]);
    assert_eq!(kitty_pane(&tree, 7), Some(42));
    assert_eq!(kitty_pane(&tree, 8), None);
    tree[0]["is_focused"] = false.into();
    assert_eq!(kitty_pane(&tree, 7), None);
    tree[0]["is_focused"] = true.into();
    tree[0]["tabs"][0]["windows"]
        .as_array_mut()
        .expect("panes")
        .push(serde_json::json!({"id":8,"is_focused":true,"foreground_processes":[{"pid":42}]}));
    assert_eq!(kitty_pane(&tree, 7), None);
    assert_eq!(kitty_pane(&serde_json::Value::Null, 7), None);
}

#[test]
#[ignore = "native tmux smoke; private server and PTY, never the user's terminal"]
fn private_tmux_confirms_literal_bytes_and_rejects_changed_panes_and_processes() {
    use std::os::unix::fs::DirBuilderExt;
    let kitty = std::env::var_os("VULTS_TEST_KITTY").is_some();
    if kitty && std::env::var_os("VULTS_KITTY_TEST_CHILD").is_none() {
        use std::io::{BufRead, BufReader};
        struct Display(std::process::Child);
        impl Drop for Display {
            fn drop(&mut self) {
                let _ = self.0.kill();
                let _ = self.0.wait();
            }
        }
        let mut display = Display(
            Command::new("Xvfb")
                .args([
                    "-displayfd",
                    "1",
                    "-screen",
                    "0",
                    "800x600x24",
                    "-nolisten",
                    "tcp",
                ])
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .spawn()
                .expect("private Xvfb"),
        );
        let mut number = String::new();
        BufReader::new(display.0.stdout.take().expect("display number"))
            .read_line(&mut number)
            .expect("display number");
        let status = Command::new(std::env::current_exe().expect("test binary"))
            .args(["--exact", "insert::session::tests::private_tmux_confirms_literal_bytes_and_rejects_changed_panes_and_processes", "--ignored", "--nocapture"])
            .env("VULTS_KITTY_TEST_CHILD", "1").env("DISPLAY", format!(":{}", number.trim().parse::<u32>().expect("number")))
            .env("XDG_SESSION_TYPE", "x11").env("LIBGL_ALWAYS_SOFTWARE", "1").env_remove("WAYLAND_DISPLAY")
            .status().expect("isolated Kitty test");
        assert!(status.success());
        return;
    }
    let directory = std::env::temp_dir().join(format!(
        "vults-pane-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    std::fs::DirBuilder::new()
        .mode(0o700)
        .create(&directory)
        .expect("private directory");
    struct Fixture {
        directory: std::path::PathBuf,
        socket: String,
        client: Option<std::process::Child>,
    }
    impl Fixture {
        fn command(&self, args: &[&str]) -> String {
            let output = Command::new("tmux")
                .env("SHELL", "/bin/sh")
                .env_remove("BASH_ENV")
                .args(["-f", "/dev/null", "-S", &self.socket])
                .args(args)
                .stdin(Stdio::null())
                .output()
                .expect("private tmux");
            assert!(output.status.success(), "private tmux command failed");
            String::from_utf8(output.stdout).expect("UTF-8")
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = Command::new("tmux")
                .args(["-S", &self.socket, "kill-server"])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
            if let Some(child) = &mut self.client {
                let _ = child.kill();
                let _ = child.wait();
            }
            let _ = std::fs::remove_dir_all(&self.directory);
        }
    }
    let mut fixture = Fixture {
        socket: directory.join("server").to_str().expect("path").into(),
        directory,
        client: None,
    };
    fixture.command(&[
        "new-session",
        "-d",
        "-s",
        "dictation",
        "/usr/bin/env",
        "-i",
        "PATH=/usr/bin:/bin",
        "cat",
    ]);
    let pane_id = fixture
        .command(&["display-message", "-p", "#{pane_id}"])
        .trim()
        .to_owned();
    let pid = fixture
        .command(&["display-message", "-p", "#{pane_pid}"])
        .trim()
        .parse()
        .expect("pane PID");
    let kitty_socket = fixture.directory.join("kitty");
    let mut client = if kitty {
        let mut client = Command::new("kitty");
        client
            .args([
                "--config",
                "NONE",
                "--override",
                "allow_remote_control=socket-only",
                "--override",
                "shell_integration=disabled",
                "--override",
                "sync_to_monitor=no",
                "--listen-on",
                &format!("unix:{}", kitty_socket.display()),
                "tmux",
                "-S",
                &fixture.socket,
                "attach-session",
                "-t",
                "dictation",
            ])
            .env("XDG_CONFIG_HOME", fixture.directory.join("config"))
            .env("XDG_CACHE_HOME", fixture.directory.join("cache"));
        client
    } else {
        let mut client = Command::new("script");
        client.args([
            "-q",
            "-c",
            &format!("tmux -S {} attach-session -t dictation", fixture.socket),
            "/dev/null",
        ]);
        client
    };
    fixture.client = Some(
        client
            .env("TERM", "xterm-256color")
            .env("SHELL", "/bin/sh")
            .env_remove("BASH_ENV")
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("private client PTY"),
    );
    let socket = Socket::open(&fixture.socket).expect("owned socket");
    let started = Instant::now();
    while clients(&socket).is_none_or(|rows| rows.is_empty()) {
        assert!(started.elapsed() < Duration::from_secs(3), "client attached");
        std::thread::sleep(Duration::from_millis(10));
    }
    let window = Window {
        class: if kitty { "kitty" } else { "XTerm" }.into(),
        title: "not used".into(),
        pid: Some(fixture.client.as_ref().expect("client").id()),
        identity: "test-only".into(),
    };
    let mut terminal = Terminal {
        pid: Some(pid),
        env: [
            ("TMUX".into(), format!("{},1,0", fixture.socket)),
            ("TMUX_PANE".into(), pane_id.clone()),
        ]
        .into(),
        ..Terminal::default()
    };
    if kitty {
        use x11rb::{
            connection::Connection,
            protocol::xproto::{AtomEnum, ConnectionExt, InputFocus},
        };
        let (conn, screen) = x11rb::connect(None).expect("private X11");
        let root = conn.setup().roots[screen].root;
        let atom = crate::jump::x11::atom(&conn, "_NET_WM_PID").expect("PID atom");
        let mut focused = false;
        let started = Instant::now();
        while !focused {
            assert!(
                started.elapsed() < Duration::from_secs(3),
                "Kitty mapped its private window"
            );
            for child in conn
                .query_tree(root)
                .expect("tree")
                .reply()
                .expect("tree")
                .children
            {
                let pid = conn
                    .get_property(false, child, atom, AtomEnum::CARDINAL, 0, 1)
                    .ok()
                    .and_then(|r| r.reply().ok())
                    .and_then(|r| r.value32().and_then(|mut v| v.next()));
                if pid == window.pid {
                    conn.set_input_focus(InputFocus::PARENT, child, x11rb::CURRENT_TIME)
                        .expect("focus");
                    conn.flush().expect("flush");
                    focused = true;
                    break;
                }
            }
            if !focused {
                std::thread::sleep(Duration::from_millis(10));
            }
        }
        let socket = Socket::open(kitty_socket.to_str().expect("path")).expect("Kitty socket");
        let mut command = Command::new("kitten");
        command.args([
            "@",
            "--to",
            &format!("unix:{}", socket.path),
            "--use-password",
            "never",
            "ls",
        ]);
        let diagnostic = command.output().expect("Kitty probe");
        assert!(
            diagnostic.status.success(),
            "private Kitty query: {}",
            String::from_utf8_lossy(&diagnostic.stderr)
        );
        let tree: serde_json::Value = serde_json::from_slice(&diagnostic.stdout).expect("tree");
        let id = tree[0]["tabs"][0]["windows"][0]["id"]
            .as_u64()
            .expect("Kitty pane ID");
        terminal
            .env
            .insert("KITTY_LISTEN_ON".into(), format!("unix:{}", socket.path));
        terminal.env.insert("KITTY_WINDOW_ID".into(), id.to_string());
        let client = clients(&Socket::open(&fixture.socket).expect("socket")).expect("clients")[0].client;
        while !kitty_focus(&socket, id, client) {
            assert!(
                started.elapsed() < Duration::from_secs(3),
                "Kitty reports actual focus and foreground client"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
    }
    let pane = Pane::capture(&window, &terminal).expect("exact attached pane");
    assert!(pane.current(&window));
    if let Some((socket, id)) = &pane.kitty {
        let mut launch = Command::new("kitten");
        launch.args([
            "@",
            "--to",
            &format!("unix:{}", socket.path),
            "--use-password",
            "never",
            "launch",
            "--type=window",
            "/usr/bin/env",
            "-i",
            "PATH=/usr/bin:/bin",
            "cat",
        ]);
        assert!(run(launch).is_some(), "second private Kitty pane");
        assert!(
            !pane.current(&window),
            "another outer pane in the same Kitty window must not match"
        );
        let mut focus = Command::new("kitten");
        focus.args([
            "@",
            "--to",
            &format!("unix:{}", socket.path),
            "--use-password",
            "never",
            "focus-window",
            "--match",
            &format!("id:{id}"),
        ]);
        assert!(run(focus).is_some(), "return to the captured private pane");
        assert!(pane.current(&window));
    }
    let text = "hello ; $(no) `no` \\n \u{4e2d}";
    assert!(pane.send(text));
    let screen = fixture.command(&["capture-pane", "-p", "-t", &pane_id]);
    assert_eq!(
        screen.trim_end(),
        text,
        "UTF-8 text was echoed without Enter or command interpretation"
    );
    fixture.command(&["copy-mode", "-t", &pane_id]);
    assert!(!pane.current(&window));
    assert!(!pane.send("blocked"));
    fixture.command(&["send-keys", "-X", "-t", &pane_id, "cancel"]);
    fixture.command(&[
        "split-window",
        "-h",
        "-t",
        &pane_id,
        "/usr/bin/env",
        "-i",
        "PATH=/usr/bin:/bin",
        "cat",
    ]);
    assert!(!pane.current(&window));
    assert!(!pane.send("blocked"));
    fixture.command(&["select-pane", "-t", &pane_id]);
    assert!(pane.current(&window));
    let mut unknown = window.clone();
    unknown.class = "konsole".into();
    assert!(
        Pane::capture(&unknown, &terminal).is_none(),
        "shared terminal PID is insufficient"
    );
    let mut stale = terminal;
    stale.pid = Some(u32::MAX);
    assert!(Pane::capture(&window, &stale).is_none());
    fixture.command(&["detach-client", "-s", "dictation"]);
    assert!(!pane.current(&window));
    let args = send_args(&pane.id, pane.pane_pid, pane.client, "blocked").expect("payload");
    assert_eq!(
        run(tmux(&pane.socket, &args)).as_deref(),
        Some(""),
        "the server also rejects a detached client"
    );
}
