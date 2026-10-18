use super::*;
use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::sync::Arc;

struct Bus(Child, String);
impl Bus {
    fn new() -> Self {
        let mut child = Command::new("dbus-daemon")
            .args(["--session", "--nofork", "--print-address=1"])
            .stdout(Stdio::piped())
            .spawn()
            .expect("private D-Bus daemon");
        let mut address = String::new();
        BufReader::new(child.stdout.take().unwrap())
            .read_line(&mut address)
            .unwrap();
        Self(child, address.trim().into())
    }
    async fn player(&self, name: &str, volume: f64, status: &str) -> (Connection, Arc<Mutex<Fake>>) {
        let state = Arc::new(Mutex::new(Fake {
            volume,
            status: status.into(),
            metadata_reads: 0,
            volumes: Vec::new(),
        }));
        let conn = zbus::connection::Builder::address(self.1.as_str())
            .unwrap()
            .name(format!("org.mpris.MediaPlayer2.{name}"))
            .unwrap()
            .serve_at(PATH, Player(state.clone()))
            .unwrap()
            .build()
            .await
            .unwrap();
        (conn, state)
    }
    async fn client(&self) -> Connection {
        zbus::connection::Builder::address(self.1.as_str())
            .unwrap()
            .method_timeout(TIMEOUT)
            .build()
            .await
            .unwrap()
    }
}
impl Drop for Bus {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
struct Fake {
    volume: f64,
    status: String,
    metadata_reads: usize,
    volumes: Vec<f64>,
}
struct Player(Arc<Mutex<Fake>>);
#[zbus::interface(name = "org.mpris.MediaPlayer2.Player")]
impl Player {
    #[zbus(property)]
    fn volume(&self) -> f64 {
        self.0.lock().unwrap().volume
    }
    #[zbus(property)]
    fn set_volume(&self, value: f64) {
        let mut state = self.0.lock().unwrap();
        state.volume = value;
        state.volumes.push(value);
    }
    #[zbus(property)]
    fn playback_status(&self) -> String {
        self.0.lock().unwrap().status.clone()
    }
    #[zbus(property)]
    fn metadata(&self) -> HashMap<String, OwnedValue> {
        self.0.lock().unwrap().metadata_reads += 1;
        HashMap::new()
    }
    async fn pause(&self, #[zbus(signal_emitter)] emitter: zbus::object_server::SignalEmitter<'_>) {
        self.0.lock().unwrap().status = "Paused".into();
        self.playback_status_changed(&emitter).await.unwrap();
    }
    async fn play(&self, #[zbus(signal_emitter)] emitter: zbus::object_server::SignalEmitter<'_>) {
        self.0.lock().unwrap().status = "Playing".into();
        self.playback_status_changed(&emitter).await.unwrap();
    }
}
async fn until(mut predicate: impl FnMut() -> bool) {
    tokio::time::timeout(Duration::from_secs(3), async {
        while !predicate() {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("music worker completed");
}

#[tokio::test]
async fn cancellation_restores_each_volume_but_preserves_quieter_and_paused_players() {
    let bus = Bus::new();
    let (_a, a) = bus.player("first", 0.8, "Playing").await;
    let (_b, b) = bus.player("second", 1.4, "Playing").await;
    let (_c, c) = bus.player("quiet", 0.1, "Playing").await;
    let (_d, d) = bus.player("paused", 0.7, "Paused").await;
    let guard = Listening::start_at(Mode::Lower, Some(bus.1.clone()));
    until(|| a.lock().unwrap().volume == 0.25 && b.lock().unwrap().volume == 0.25).await;
    drop(guard);
    until(|| a.lock().unwrap().volume == 0.8 && b.lock().unwrap().volume == 1.4).await;
    assert_eq!(c.lock().unwrap().volume, 0.1);
    assert_eq!(d.lock().unwrap().volume, 0.7);
    assert_eq!(d.lock().unwrap().status, "Paused");
    for state in [&a, &b, &c, &d] {
        assert_eq!(
            state.lock().unwrap().metadata_reads,
            0,
            "no GetAll/track metadata"
        );
    }
}

#[tokio::test]
async fn user_changes_even_back_to_our_volume_and_restarted_players_are_left_alone() {
    let bus = Bus::new();
    let (a_conn, a) = bus.player("changed", 0.8, "Playing").await;
    let (old, original) = bus.player("restart", 0.9, "Playing").await;
    let conn = bus.client().await;
    let (end, ended) = oneshot::channel();
    let task = tokio::spawn(async move { hold(&conn, Mode::Lower, ended).await });
    until(|| a.lock().unwrap().volume == 0.25 && original.lock().unwrap().volume == 0.25).await;
    let proxy = player(&a_conn, a_conn.unique_name().unwrap().as_str())
        .await
        .unwrap();
    proxy.set_property("Volume", 0.6_f64).await.unwrap();
    proxy.set_property("Volume", 0.25_f64).await.unwrap();
    old.close().await.unwrap();
    let (_replacement, new) = bus.player("restart", 0.4, "Playing").await;
    end.send(()).unwrap();
    task.await.unwrap().unwrap();
    assert_eq!(
        a.lock().unwrap().volume,
        0.25,
        "observed user volume wins even after returning to 25%"
    );
    assert_eq!(new.lock().unwrap().volume, 0.4, "a new owner is never restored");
}

#[tokio::test]
async fn pause_restores_only_untouched_playback_after_a_player_crashes() {
    let bus = Bus::new();
    let (a_conn, a) = bus.player("changed", 0.8, "Playing").await;
    let (_b_conn, b) = bus.player("restore", 0.5, "Playing").await;
    let (volume_conn, volume) = bus.player("volume_changed", 0.6, "Playing").await;
    let (crash, c) = bus.player("crash", 0.9, "Playing").await;
    let (_paused, paused) = bus.player("already_paused", 0.3, "Paused").await;
    let conn = bus.client().await;
    let (end, ended) = oneshot::channel();
    let task = tokio::spawn(async move { hold(&conn, Mode::Pause, ended).await });
    until(|| {
        a.lock().unwrap().status == "Paused"
            && b.lock().unwrap().status == "Paused"
            && c.lock().unwrap().status == "Paused"
            && volume.lock().unwrap().status == "Paused"
    })
    .await;
    let proxy = player(&a_conn, a_conn.unique_name().unwrap().as_str())
        .await
        .unwrap();
    proxy.call_method("Play", &()).await.unwrap();
    proxy.call_method("Pause", &()).await.unwrap();
    let volume_proxy = player(&volume_conn, volume_conn.unique_name().unwrap().as_str())
        .await
        .unwrap();
    volume_proxy.set_property("Volume", 0.7_f64).await.unwrap();
    crash.close().await.unwrap();
    end.send(()).unwrap();
    task.await.unwrap().unwrap();
    assert_eq!(a.lock().unwrap().status, "Paused");
    assert_eq!(b.lock().unwrap().status, "Playing");
    assert_eq!(paused.lock().unwrap().status, "Paused");
    assert_eq!(b.lock().unwrap().volume, 0.5);
    assert_eq!(
        volume.lock().unwrap().status,
        "Paused",
        "user volume changes prevent resuming playback"
    );
    assert_eq!(volume.lock().unwrap().volume, 0.7);
}

#[test]
fn leave_does_not_open_the_bus() {
    let guard = Listening::start_at(Mode::Leave, Some("invalid bus address".into()));
    assert!(guard.0.is_none());
}

#[tokio::test]
async fn rapid_recordings_restore_before_the_next_snapshot() {
    let bus = Bus::new();
    let (_conn, state) = bus.player("rapid", 0.8, "Playing").await;
    let first = Listening::start_at(Mode::Lower, Some(bus.1.clone()));
    until(|| state.lock().unwrap().volume == 0.25).await;
    drop(first);
    let second = Listening::start_at(Mode::Lower, Some(bus.1.clone()));
    until(|| state.lock().unwrap().volumes.len() >= 3).await;
    assert_eq!(state.lock().unwrap().volumes, [0.25, 0.8, 0.25]);
    drop(second);
    until(|| state.lock().unwrap().volume == 0.8).await;
}
