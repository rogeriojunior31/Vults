use super::*;
use std::time::{Duration, Instant};

#[test]
#[ignore = "native WezTerm smoke; requires wezterm/wezterm-gui and Xvfb, uses private servers only"]
fn private_wezterm_navigation_selects_the_exact_server_and_never_discovers_a_replacement() {
    struct Child(std::process::Child);
    impl Drop for Child {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    if std::env::var_os("VULTS_WEZTERM_TEST_CHILD").is_none() {
        use std::io::{BufRead, BufReader};
        use std::os::unix::fs::DirBuilderExt;
        let mut display = Child(
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
        let directory = std::env::temp_dir().join(format!("vults-wezterm-nav-{}", std::process::id()));
        std::fs::DirBuilder::new()
            .mode(0o700)
            .create(&directory)
            .expect("private runtime directory");
        let status = Command::new("dbus-run-session").arg("--").arg(std::env::current_exe().expect("test binary"))
            .args(["--exact", "jump::wezterm_tests::private_wezterm_navigation_selects_the_exact_server_and_never_discovers_a_replacement", "--ignored", "--nocapture"])
            .env("VULTS_WEZTERM_TEST_CHILD", "1")
            .env("DISPLAY", format!(":{}", number.trim().parse::<u32>().expect("display")))
            .env("XDG_SESSION_TYPE", "x11").env_remove("WAYLAND_DISPLAY")
            .env("XDG_RUNTIME_DIR", &directory).env("XDG_CONFIG_HOME", directory.join("config"))
            .env("XDG_CACHE_HOME", directory.join("cache")).env("XDG_DATA_HOME", directory.join("data"))
            .env("LIBGL_ALWAYS_SOFTWARE", "1").status();
        let _ = std::fs::remove_dir_all(directory);
        assert!(status.expect("isolated test child").success());
        return;
    }
    let start = || {
        Child(
            Command::new("wezterm-gui")
                .args([
                    "--skip-config",
                    "--config",
                    "check_for_updates=false",
                    "--config",
                    "enable_wayland=false",
                    "start",
                    "--always-new-process",
                    "--no-auto-connect",
                    "--",
                    "/usr/bin/env",
                    "-i",
                    "PATH=/usr/bin:/bin",
                    "cat",
                ])
                .env_remove("WEZTERM_UNIX_SOCKET")
                .env_remove("WEZTERM_PANE")
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .expect("private WezTerm"),
        )
    };
    let runtime = std::path::PathBuf::from(std::env::var_os("XDG_RUNTIME_DIR").expect("private runtime"));
    let socket = |pid| runtime.join("wezterm").join(format!("gui-sock-{pid}"));
    let mut first = start();
    let first_socket = socket(first.0.id());
    until(|| first_socket.exists());
    let second = start();
    let second_socket = socket(second.0.id());
    until(|| second_socket.exists());
    let first_socket = first_socket.to_str().expect("socket");
    let second_socket = second_socket.to_str().expect("socket");
    let cli = |socket: &str, args: &[&str]| {
        let output = Command::new("wezterm")
            .args(["--skip-config", "cli", "--no-auto-start"])
            .args(args)
            .env("WEZTERM_UNIX_SOCKET", socket)
            .env_remove("WEZTERM_PANE")
            .stdin(Stdio::null())
            .output()
            .expect("private CLI");
        assert!(
            output.status.success(),
            "private WezTerm CLI: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).expect("UTF-8")
    };
    let panes = |socket: &str| {
        serde_json::from_str::<serde_json::Value>(&cli(socket, &["list", "--format", "json"]))
            .expect("pane list")
    };
    for socket in [first_socket, second_socket] {
        let list = panes(socket);
        assert_eq!(list.as_array().expect("panes").len(), 1);
        assert_eq!(list[0]["pane_id"], 0);
        cli(
            socket,
            &[
                "split-pane",
                "--pane-id",
                "0",
                "--",
                "/usr/bin/env",
                "-i",
                "PATH=/usr/bin:/bin",
                "cat",
            ],
        );
    }
    let active = |socket: &str| {
        let list = panes(socket);
        let row = list
            .as_array()
            .expect("panes")
            .iter()
            .find(|pane| pane["is_active"] == true)
            .expect("active pane");
        row["pane_id"].as_u64().expect("pane id")
    };
    assert_eq!(active(first_socket), 1);
    assert_eq!(active(second_socket), 1);
    let target: BTreeMap<_, _> = [
        ("WEZTERM_PANE".into(), "0".into()),
        ("WEZTERM_UNIX_SOCKET".into(), first_socket.into()),
    ]
    .into();
    assert!(multiplexer(&target, &|| true));
    assert_eq!(active(first_socket), 0);
    assert_eq!(
        active(second_socket),
        1,
        "identical pane ids in another instance must not move it"
    );
    let no_instance: BTreeMap<_, _> = [("WEZTERM_PANE".into(), "0".into())].into();
    assert!(!multiplexer(&no_instance, &|| true));
    assert_eq!(active(second_socket), 1);
    first.0.kill().expect("stop first GUI");
    first.0.wait().expect("first GUI stopped");
    let _ = std::fs::remove_file(first_socket);
    assert!(!multiplexer(&target, &|| true));
    assert_eq!(
        active(second_socket),
        1,
        "a vanished server must not fall back to the other instance"
    );
}

fn until(mut check: impl FnMut() -> bool) {
    let started = Instant::now();
    while !check() {
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "private native condition did not settle"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}
