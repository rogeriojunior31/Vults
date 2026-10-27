//! MPRIS over the session bus. No polling: the players' PropertiesChanged and the bus's
//! NameOwnerChanged signals wake the watcher, and it reads every player again (a handful of
//! calls, only when something changed).

use std::collections::HashMap;
use std::time::Duration;

use futures_util::{FutureExt, StreamExt};
use zbus::fdo::DBusProxy;
use zbus::message::Type;
use zbus::zvariant::OwnedValue;
use zbus::{Connection, MatchRule, MessageStream, Proxy};

use crate::{Control, NowPlaying, Player, Status, pick};

const PREFIX: &str = "org.mpris.MediaPlayer2.";
const PATH: &str = "/org/mpris/MediaPlayer2";
const PLAYER: &str = "org.mpris.MediaPlayer2.Player";
/// A track change sends several signals at once: read the players once they settled.
const SETTLE: Duration = Duration::from_millis(150);

/// Calls `on_change` with what is playing now, and again each time it changes. Runs until the
/// bus goes away; drop the future to stop.
pub async fn watch(on_change: impl Fn(Option<NowPlaying>)) -> Result<(), String> {
    let conn = Connection::session().await.map_err(text)?;
    let changed = MatchRule::builder()
        .msg_type(Type::Signal)
        .interface("org.freedesktop.DBus.Properties")
        .map_err(text)?
        .member("PropertiesChanged")
        .map_err(text)?
        .path(PATH)
        .map_err(text)?
        .build();
    let owners = MatchRule::builder()
        .msg_type(Type::Signal)
        .interface("org.freedesktop.DBus")
        .map_err(text)?
        .member("NameOwnerChanged")
        .map_err(text)?
        .arg0ns("org.mpris.MediaPlayer2")
        .map_err(text)?
        .build();
    let changed = MessageStream::for_match_rule(changed, &conn, Some(64))
        .await
        .map_err(text)?;
    let owners = MessageStream::for_match_rule(owners, &conn, Some(64))
        .await
        .map_err(text)?;
    let mut signals = futures_util::stream::select(changed, owners);

    let mut shown: Option<NowPlaying> = None;
    let mut first = true;
    loop {
        let now = pick(&players(&conn).await, shown.as_ref().map(|n| n.player.as_str()));
        if first || now != shown {
            on_change(now.clone());
            shown = now;
            first = false;
        }
        if signals.next().await.is_none() {
            return Err("the session bus went away".into());
        }
        tokio::time::sleep(SETTLE).await;
        // Everything that came meanwhile is covered by the one read above.
        while let Some(Some(_)) = signals.next().now_or_never() {}
    }
}

/// A unique process identity captured before an asynchronous command can be delayed.
#[derive(Debug)]
pub struct Target {
    conn: Connection,
    owner: String,
    service: String,
}

pub async fn pin(player: &str) -> Result<Target, String> {
    if !player.starts_with(PREFIX) {
        return Err("The target is not an MPRIS player.".into());
    }
    let conn = Connection::session().await.map_err(text)?;
    let bus = DBusProxy::new(&conn).await.map_err(text)?;
    let owner = bus
        .get_name_owner(player.try_into().map_err(text)?)
        .await
        .map_err(text)?
        .to_string();
    Ok(Target {
        conn,
        owner,
        service: player.into(),
    })
}

pub async fn control(player: &str, what: Control) -> Result<(), String> {
    what.validate()?;
    pin(player).await?.control(what, || true).await
}

impl Target {
    /// Read every requested capability before the first effect; execution still rechecks it.
    pub async fn validate_batch(
        &self,
        actions: &[Control],
        allowed: impl Fn() -> bool,
    ) -> Result<(), String> {
        for action in actions {
            self.prepare(*action, &allowed).await?;
        }
        self.check_owner(&allowed).await
    }

    async fn prepare(
        &self,
        what: Control,
        allowed: &impl Fn() -> bool,
    ) -> Result<(Proxy<'_>, Option<f64>), String> {
        what.validate()?;
        if !allowed() {
            return Err("Media command cancelled.".into());
        }
        let proxy: Proxy<'_> = zbus::proxy::Builder::new(&self.conn)
            .destination(self.owner.clone())
            .map_err(text)?
            .path(PATH)
            .map_err(text)?
            .interface(PLAYER)
            .map_err(text)?
            .cache_properties(zbus::proxy::CacheProperties::No)
            .build()
            .await
            .map_err(text)?;
        let capability = match what {
            Control::Play => "CanPlay",
            Control::Pause => "CanPause",
            Control::Next => "CanGoNext",
            Control::Previous => "CanGoPrevious",
            _ => "CanControl",
        };
        if what != Control::PlayPause
            && (!proxy.get_property::<bool>("CanControl").await.map_err(text)?
                || !proxy.get_property::<bool>(capability).await.map_err(text)?)
        {
            return Err("This player does not support the requested control.".into());
        }
        let volume = match what {
            Control::Volume(percent) => Some(f64::from(percent) / 100.0),
            Control::VolumeUp | Control::VolumeDown => {
                let current = proxy.get_property::<f64>("Volume").await.map_err(text)?;
                if !current.is_finite() || current < 0.0 {
                    return Err("Invalid player volume.".into());
                }
                let step = if what == Control::VolumeUp { 0.1 } else { -0.1 };
                Some((current + step).clamp(0.0, 1.0))
            }
            _ => None,
        };
        Ok((proxy, volume))
    }

    async fn check_owner(&self, allowed: &impl Fn() -> bool) -> Result<(), String> {
        let bus = DBusProxy::new(&self.conn).await.map_err(text)?;
        let owner = bus
            .get_name_owner(self.service.as_str().try_into().map_err(text)?)
            .await
            .map_err(text)?;
        if owner.as_str() != self.owner || !allowed() {
            return Err("Media command cancelled or player changed.".into());
        }
        Ok(())
    }

    /// Recheck cancellation/consent after property reads and immediately before a native write.
    pub async fn control(&self, what: Control, allowed: impl Fn() -> bool) -> Result<(), String> {
        let (proxy, volume) = self.prepare(what, &allowed).await?;
        self.check_owner(&allowed).await?;
        if let Some(volume) = volume {
            proxy.set_property("Volume", volume).await.map_err(text)?;
        } else {
            proxy
                .call_method(what.method().expect("Playback has a method"), &())
                .await
                .map_err(text)?;
        }
        Ok(())
    }
}

async fn players(conn: &Connection) -> Vec<Player> {
    let Ok(bus) = DBusProxy::new(conn).await else {
        return Vec::new();
    };
    let names = bus.list_names().await.unwrap_or_default();
    let mut out = Vec::new();
    for name in names
        .iter()
        .map(|n| n.to_string())
        .filter(|n| n.starts_with(PREFIX))
    {
        // A player that does not answer (closing, busy) is left out of this read.
        if let Some(p) = read(conn, &name).await {
            out.push(p);
        }
    }
    out
}

async fn read(conn: &Connection, name: &str) -> Option<Player> {
    let proxy = Proxy::new(conn, name.to_string(), PATH, PLAYER).await.ok()?;
    let status: String = proxy.get_property("PlaybackStatus").await.ok()?;
    let meta: HashMap<String, OwnedValue> = proxy.get_property("Metadata").await.unwrap_or_default();
    let title = meta
        .get("xesam:title")
        .and_then(|v| String::try_from(v.try_clone().ok()?).ok())
        .unwrap_or_default();
    let artist = meta
        .get("xesam:artist")
        .and_then(|v| Vec::<String>::try_from(v.try_clone().ok()?).ok())
        .map(|a| a.join(", "))
        .filter(|a| !a.is_empty());
    Some(Player {
        name: name.to_string(),
        status: match status.as_str() {
            "Playing" => Status::Playing,
            "Paused" => Status::Paused,
            _ => Status::Stopped,
        },
        title,
        artist,
    })
}

fn text(e: impl std::fmt::Display) -> String {
    e.to_string()
}

#[cfg(test)]
mod tests {
    use std::sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    };

    use zbus::interface;
    use zbus::object_server::SignalEmitter;
    use zbus::zvariant::{OwnedValue, Value};

    use super::*;

    /// A silent player on a private session bus, as Spotify or a browser would show up.
    struct Fake {
        playing: Arc<Mutex<bool>>,
        volume: Arc<Mutex<f64>>,
        controllable: Arc<AtomicBool>,
    }

    #[interface(name = "org.mpris.MediaPlayer2.Player")]
    impl Fake {
        fn play(&self) {
            *self.playing.lock().unwrap() = true;
        }

        fn pause(&self) {
            *self.playing.lock().unwrap() = false;
        }

        #[zbus(property)]
        fn can_control(&self) -> bool {
            self.controllable.load(Ordering::SeqCst)
        }

        #[zbus(property)]
        fn can_play(&self) -> bool {
            self.can_control()
        }

        #[zbus(property)]
        fn can_pause(&self) -> bool {
            self.can_control()
        }

        #[zbus(property)]
        fn can_go_next(&self) -> bool {
            false
        }

        #[zbus(property)]
        fn volume(&self) -> f64 {
            *self.volume.lock().unwrap()
        }

        #[zbus(property)]
        fn set_volume(&mut self, volume: f64) {
            *self.volume.lock().unwrap() = volume;
        }

        async fn play_pause(&self, #[zbus(signal_emitter)] emitter: SignalEmitter<'_>) {
            {
                let mut p = self.playing.lock().unwrap_or_else(|e| e.into_inner());
                *p = !*p;
            }
            // As a real player does: the watcher wakes on this, it never polls.
            let _ = self.playback_status_changed(&emitter).await;
        }

        #[zbus(property)]
        fn playback_status(&self) -> String {
            let p = *self.playing.lock().unwrap_or_else(|e| e.into_inner());
            if p { "Playing" } else { "Paused" }.into()
        }

        #[zbus(property)]
        fn metadata(&self) -> HashMap<String, OwnedValue> {
            let mut m = HashMap::new();
            let value = |v: Value<'_>| v.try_to_owned().unwrap_or_else(|_| OwnedValue::from(0u8));
            m.insert("xesam:title".into(), value(Value::from("Song (Official Video)")));
            m.insert(
                "xesam:artist".into(),
                value(Value::from(vec!["Band feat. Guest"])),
            );
            m
        }
    }

    #[tokio::test]
    async fn a_player_on_the_bus_is_seen_and_obeys() {
        // Real desktop players must not compete with this fixture or see its control calls.
        if std::env::var_os("VULTS_MEDIA_TEST_CHILD").is_none() {
            let status = std::process::Command::new("dbus-run-session")
                .arg("--")
                .arg(std::env::current_exe().expect("test binary"))
                .args([
                    "--exact",
                    "mpris::tests::a_player_on_the_bus_is_seen_and_obeys",
                    "--nocapture",
                ])
                .env("VULTS_MEDIA_TEST_CHILD", "1")
                .status()
                .expect("private D-Bus session");
            assert!(status.success());
            return;
        }
        let bus = Connection::session().await.expect("private session bus");
        let name = format!("{PREFIX}vultures_test_{}", std::process::id());
        let playing = Arc::new(Mutex::new(true));
        let volume = Arc::new(Mutex::new(0.75));
        let controllable = Arc::new(AtomicBool::new(true));
        bus.object_server()
            .at(
                PATH,
                Fake {
                    playing: playing.clone(),
                    volume: volume.clone(),
                    controllable: controllable.clone(),
                },
            )
            .await
            .expect("serve the fake player");
        bus.request_name(name.as_str()).await.expect("own the name");

        let seen = Arc::new(Mutex::new(None));
        let sink = seen.clone();
        let watcher = tokio::spawn(async move {
            let _ = watch(move |n| *sink.lock().unwrap_or_else(|e| e.into_inner()) = n).await;
        });
        tokio::time::sleep(Duration::from_millis(500)).await;
        let now = seen.lock().unwrap_or_else(|e| e.into_inner()).clone();
        let now = now.expect("the fake player is playing");
        assert_eq!(
            (now.title.as_str(), now.artist.as_deref(), now.playing),
            ("Song", Some("Band"), true)
        );
        assert_eq!(now.player, name);

        control(&name, Control::PlayPause)
            .await
            .expect("play/pause reaches it");
        assert!(!*playing.lock().unwrap_or_else(|e| e.into_inner()));
        tokio::time::sleep(Duration::from_millis(500)).await;
        let after = seen.lock().unwrap_or_else(|e| e.into_inner()).clone();
        assert_eq!(
            after.map(|n| n.playing),
            Some(false),
            "the pause reached the watcher"
        );
        watcher.abort();
        assert!(control("org.example.Unrelated", Control::Play).await.is_err());
        for _ in 0..2 {
            control(&name, Control::Play).await.expect("explicit play");
            assert!(*playing.lock().unwrap());
        }
        for _ in 0..2 {
            control(&name, Control::Pause).await.expect("explicit pause");
            assert!(!*playing.lock().unwrap());
        }
        control(&name, Control::Volume(30))
            .await
            .expect("bounded player volume");
        assert_eq!(*volume.lock().unwrap(), 0.3);
        assert!(control(&name, Control::Volume(101)).await.is_err());
        assert_eq!(*volume.lock().unwrap(), 0.3);
        control(&name, Control::VolumeUp)
            .await
            .expect("relative volume up");
        assert!((*volume.lock().unwrap() - 0.4).abs() < 1e-9);
        control(&name, Control::VolumeDown)
            .await
            .expect("relative volume down");
        assert!((*volume.lock().unwrap() - 0.3).abs() < 1e-9);
        let target = pin(&name).await.expect("capture player identity");
        target
            .validate_batch(&[Control::Play, Control::Volume(40)], || true)
            .await
            .expect("supported batch preflight");
        assert!(!*playing.lock().unwrap(), "preflight must not play");
        assert!((*volume.lock().unwrap() - 0.3).abs() < 1e-9);
        assert!(
            target
                .validate_batch(&[Control::Play, Control::Next], || true)
                .await
                .is_err()
        );
        assert!(
            !*playing.lock().unwrap(),
            "unsupported suffix cannot execute its prefix"
        );
        assert!(
            target
                .validate_batch(&[Control::Play, Control::Volume(101)], || true)
                .await
                .is_err()
        );
        assert!(target.validate_batch(&[Control::Play], || false).await.is_err());
        *volume.lock().unwrap() = 2.0;
        target
            .control(Control::VolumeDown, || true)
            .await
            .expect("clamp after measured step");
        assert_eq!(*volume.lock().unwrap(), 1.0);
        *volume.lock().unwrap() = 0.3;
        let checks = AtomicUsize::new(0);
        assert!(
            target
                .control(Control::Play, || checks.fetch_add(1, Ordering::SeqCst) == 0)
                .await
                .is_err()
        );
        assert!(
            !*playing.lock().unwrap(),
            "cancellation after property reads prevents execution"
        );
        controllable.store(false, Ordering::SeqCst);
        assert!(control(&name, Control::Play).await.is_err());
        assert!(control(&name, Control::Pause).await.is_err());
        assert!(control(&name, Control::Volume(40)).await.is_err());
        assert!(!*playing.lock().unwrap());
        assert!((*volume.lock().unwrap() - 0.3).abs() < 1e-9);
        controllable.store(true, Ordering::SeqCst);
        bus.release_name(name.as_str()).await.expect("player departs");
        assert!(target.validate_batch(&[Control::Play], || true).await.is_err());
        assert!(target.control(Control::Play, || true).await.is_err());
        assert!(!*playing.lock().unwrap());
    }
}
