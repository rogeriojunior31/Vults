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

/// Play/pause or skip on that player.
pub async fn control(player: &str, what: Control) -> Result<(), String> {
    let conn = Connection::session().await.map_err(text)?;
    let proxy = Proxy::new(&conn, player.to_string(), PATH, PLAYER)
        .await
        .map_err(text)?;
    let method = match what {
        Control::PlayPause => "PlayPause",
        Control::Next => "Next",
        Control::Previous => "Previous",
    };
    proxy.call_method(method, &()).await.map_err(text)?;
    Ok(())
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
    use std::sync::{Arc, Mutex};

    use zbus::interface;
    use zbus::object_server::SignalEmitter;
    use zbus::zvariant::{OwnedValue, Value};

    use super::*;

    /// A silent player on the user's session bus, as Spotify or a browser would show up.
    struct Fake {
        playing: Arc<Mutex<bool>>,
    }

    #[interface(name = "org.mpris.MediaPlayer2.Player")]
    impl Fake {
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
        // CI has no session bus: nothing to test there.
        let Ok(bus) = Connection::session().await else {
            return;
        };
        let name = format!("{PREFIX}vultures_test_{}", std::process::id());
        let playing = Arc::new(Mutex::new(true));
        bus.object_server()
            .at(
                PATH,
                Fake {
                    playing: playing.clone(),
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
    }
}
