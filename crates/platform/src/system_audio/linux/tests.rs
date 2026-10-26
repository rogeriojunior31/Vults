use super::*;
use serde_json::json;
use std::sync::{
    Mutex,
    atomic::{AtomicBool, Ordering},
};

struct Fake(Mutex<State>);
struct State {
    name: String,
    index: u32,
    cookie: String,
    serial: String,
    port: String,
    volume: Vec<u32>,
    muted: bool,
    writes: Vec<Vec<String>>,
    ignore: bool,
}
impl Fake {
    fn new() -> Self {
        Self(Mutex::new(State {
            name: "speakers".into(),
            index: 12,
            cookie: "1234:abcd".into(),
            serial: "42".into(),
            port: "speaker".into(),
            volume: vec![32768, 16384],
            muted: false,
            writes: vec![],
            ignore: false,
        }))
    }
}
impl Io for Fake {
    async fn request(&self, args: &[String]) -> Result<Vec<u8>, Error> {
        let mut s = self.0.lock().unwrap();
        match args[0].as_str() {
            "info" => Ok(serde_json::to_vec(&json!({"is_local":"yes","cookie":s.cookie,
                "server_string":"unix:/run/user/fixture/pulse/native","default_sink_name":s.name}))
            .unwrap()),
            "list" => {
                assert_eq!(args, ["list", "sinks"]);
                Ok(serde_json::to_vec(&json!([{"name":s.name,"index":s.index,"channel_map":"front-left,front-right",
                    "mute":s.muted,"volume":{"front-left":{"value":s.volume[0]},"front-right":{"value":s.volume[1]}},
                    "properties":{"object.serial":s.serial},"active_port":s.port}])).unwrap())
            }
            "set-sink-mute" | "set-sink-volume" => {
                assert_eq!(args[1], s.index.to_string());
                s.writes.push(args.to_vec());
                if !s.ignore {
                    if args[0] == "set-sink-mute" {
                        s.muted = args[2] == "1";
                    } else {
                        s.volume = args[2..].iter().map(|arg| arg.parse().unwrap()).collect();
                    }
                }
                Ok(vec![])
            }
            other => panic!("Unexpected native command {other}"),
        }
    }
}

#[tokio::test]
async fn absolute_and_relative_volume_preserve_balance_and_never_unmute_or_amplify() {
    let io = Fake::new();
    let identity = snapshot(&io, &|| true).await.unwrap().identity;
    io.0.lock().unwrap().muted = true;
    let out = apply(&io, &identity, Action::Volume(60), &|| true).await.unwrap();
    assert_eq!(
        out,
        Status {
            muted: true,
            percent: 60
        }
    );
    let volume = io.0.lock().unwrap().volume.clone();
    assert_eq!(volume, vec![39321, 19661]);
    assert_eq!(
        apply(&io, &identity, Action::Louder, &|| true)
            .await
            .unwrap()
            .percent,
        70
    );
    assert_eq!(
        apply(&io, &identity, Action::Quieter, &|| true)
            .await
            .unwrap()
            .percent,
        60
    );
    assert_eq!(
        apply(&io, &identity, Action::Volume(0), &|| true)
            .await
            .unwrap()
            .percent,
        0
    );
    assert_eq!(
        apply(&io, &identity, Action::Louder, &|| true)
            .await
            .unwrap()
            .percent,
        10
    );
    io.0.lock().unwrap().volume = vec![131072, 65536];
    assert_eq!(
        apply(&io, &identity, Action::Mute(true), &|| true)
            .await
            .unwrap()
            .percent,
        200
    );
    assert_eq!(io.0.lock().unwrap().volume, vec![131072, 65536]);
    assert_eq!(
        apply(&io, &identity, Action::Quieter, &|| true)
            .await
            .unwrap()
            .percent,
        100
    );
    io.0.lock().unwrap().volume = vec![131072, 65536];
    assert_eq!(
        apply(&io, &identity, Action::Louder, &|| true)
            .await
            .unwrap()
            .percent,
        100
    );
    assert!(io.0.lock().unwrap().volume.iter().all(|v| *v <= 65536));
    assert_eq!(
        apply(&io, &identity, Action::Volume(101), &|| true).await,
        Err(Error::Invalid)
    );
    for _ in 0..2 {
        assert!(
            apply(&io, &identity, Action::Mute(true), &|| true)
                .await
                .unwrap()
                .muted
        );
    }
    assert!(
        !apply(&io, &identity, Action::Mute(false), &|| true)
            .await
            .unwrap()
            .muted
    );
    assert!(
        io.0.lock()
            .unwrap()
            .writes
            .iter()
            .all(|args| args[1] == "12" && !args.iter().any(|arg| arg.contains('@')))
    );
}

#[tokio::test]
async fn changed_output_server_port_or_index_is_never_retargeted() {
    for field in 0..5 {
        let io = Fake::new();
        let identity = snapshot(&io, &|| true).await.unwrap().identity;
        {
            let mut state = io.0.lock().unwrap();
            match field {
                0 => state.name = "headphones".into(),
                1 => state.index += 1,
                2 => state.cookie = "9876:abcd".into(),
                3 => state.serial = "43".into(),
                _ => state.port = "headphone".into(),
            }
        }
        assert_eq!(
            apply(&io, &identity, Action::Mute(true), &|| true).await,
            Err(Error::Changed)
        );
        assert!(io.0.lock().unwrap().writes.is_empty());
    }
}

#[tokio::test]
async fn cancellation_and_unconfirmed_writes_never_report_success() {
    let io = Fake::new();
    let identity = snapshot(&io, &|| true).await.unwrap().identity;
    let calls = std::sync::atomic::AtomicUsize::new(0);
    assert_eq!(
        apply(&io, &identity, Action::Mute(true), &|| calls
            .fetch_add(1, Ordering::SeqCst)
            < 3)
        .await,
        Err(Error::Cancelled)
    );
    assert!(io.0.lock().unwrap().writes.is_empty());
    let allowed = AtomicBool::new(false);
    assert_eq!(
        snapshot(&io, &|| allowed.load(Ordering::SeqCst))
            .await
            .unwrap_err(),
        Error::Cancelled
    );
    io.0.lock().unwrap().ignore = true;
    assert_eq!(
        apply(&io, &identity, Action::Mute(true), &|| true).await,
        Err(Error::Failed)
    );
}

#[test]
fn local_endpoint_and_channel_metadata_fail_closed() {
    for server in [
        "tcp:example.com",
        "unix:relative",
        "unix:/tmp/native tcp:example.com",
        "unix:/tmp/socket;exit",
    ] {
        assert_eq!(local_server(server), Err(Error::Unavailable));
    }
    assert!(local_server("unix:/run/user/1000/pulse/native").is_ok());
    assert!(server(br#"{"is_local":"no"}"#).is_err());
    let fresh = || Server {
        cookie: "1234:abcd".into(),
        server: "unix:/fixture/native".into(),
        default: "speakers".into(),
    };
    let valid = json!([{"name":"speakers","index":12,"channel_map":"front-left,front-right","mute":false,
        "volume":{"front-left":{"value":32768},"front-right":{"value":16384}}}]);
    assert!(sink(&serde_json::to_vec(&valid).unwrap(), fresh()).is_ok());
    for field in 0..6 {
        let mut invalid = valid.clone();
        match field {
            0 => invalid[0]["channel_map"] = json!("front-left,front-left"),
            1 => invalid[0]["volume"]["front-right"] = Value::Null,
            2 => invalid[0]["index"] = json!(u32::MAX),
            3 => invalid[0]["mute"] = Value::Null,
            4 => invalid[0]["active_port"] = json!(42),
            _ => invalid[0]["properties"] = json!({"object.serial":42}),
        }
        assert!(sink(&serde_json::to_vec(&invalid).unwrap(), fresh()).is_err());
    }
    for volume in [vec![0, 0], vec![u32::MAX, 0], vec![1234, 5678]] {
        assert!(volumes(&volume, 100).iter().all(|v| *v <= 65536));
    }
}

#[tokio::test]
async fn native_process_arguments_output_and_lifetime_are_bounded_without_audio_access() {
    use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
    struct Directory(PathBuf);
    impl Drop for Directory {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir =
        Directory(std::env::temp_dir().join(format!("vults-audio-runner-{}-{stamp}", std::process::id())));
    std::fs::DirBuilder::new().mode(0o700).create(&dir.0).unwrap();
    let program = dir.0.join("fixture");
    let backend = Backend {
        program: program.clone(),
        server: "unix:/fixture/native".into(),
    };
    let prefix = "#!/bin/sh\n[ \"$1\" = --format=json ] && [ \"$2\" = --server ] && [ \"$3\" = unix:/fixture/native ] && [ \"$4\" = info ] || exit 17\n";
    let write = |body: &str| {
        std::fs::write(&program, format!("{prefix}{body}\n")).unwrap();
        std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o700)).unwrap();
    };
    write("printf '{}'");
    assert_eq!(backend.request(&["info".into()]).await.unwrap(), b"{}");
    write("exit 8");
    assert_eq!(backend.request(&["info".into()]).await, Err(Error::Failed));
    write(&format!(
        "i=0; while [ \"$i\" -lt 129 ]; do printf '%s' '{}'; i=$((i+1)); done",
        "x".repeat(8192)
    ));
    assert_eq!(backend.request(&["info".into()]).await, Err(Error::Invalid));
    write("exec /bin/sleep 4");
    assert_eq!(backend.request(&["info".into()]).await, Err(Error::Failed));
    std::fs::remove_file(program).unwrap();
    assert_eq!(backend.request(&["info".into()]).await, Err(Error::Unavailable));
}
