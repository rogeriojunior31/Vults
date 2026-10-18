use super::Mode;

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;

use futures_util::{FutureExt, StreamExt};
use tokio::sync::oneshot;
use zbus::zvariant::OwnedValue;
use zbus::{Connection, MatchRule, MessageStream, Proxy};

const PATH: &str = "/org/mpris/MediaPlayer2";
const PLAYER: &str = "org.mpris.MediaPlayer2.Player";
const TIMEOUT: Duration = Duration::from_millis(250);
// Restoration must finish before the next recording snapshots the same players.
static SERIAL: Mutex<()> = Mutex::new(());

/// Dropping this guard restores untouched players, including on cancellation or capture failure.
#[derive(Debug)]
pub struct Listening(Option<oneshot::Sender<()>>);

impl Listening {
    pub fn start(mode: Mode) -> Self {
        Self::start_at(mode, None)
    }

    fn start_at(mode: Mode, address: Option<String>) -> Self {
        if mode == Mode::Leave {
            return Self(None);
        }
        let (end, mut ended) = oneshot::channel();
        // Never delay microphone capture on a slow or unavailable session bus.
        let _ = std::thread::Builder::new()
            .name("dictation-music".into())
            .spawn(move || {
                let _serial = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
                if ended.try_recv() != Err(oneshot::error::TryRecvError::Empty) {
                    return;
                }
                let Ok(runtime) = tokio::runtime::Builder::new_current_thread().enable_all().build() else {
                    return;
                };
                runtime.block_on(async {
                    let connect = async {
                        let builder = match address.as_deref() {
                            Some(address) => zbus::connection::Builder::address(address)?,
                            None => zbus::connection::Builder::session()?,
                        };
                        builder.method_timeout(TIMEOUT).build().await
                    };
                    if let Ok(Ok(conn)) = tokio::time::timeout(TIMEOUT, connect).await {
                        let _ = hold(&conn, mode, ended).await;
                    }
                });
            });
        Self(Some(end))
    }
}

impl Drop for Listening {
    fn drop(&mut self) {
        if let Some(end) = self.0.take() {
            let _ = end.send(());
        }
    }
}

struct Changed {
    owner: String,
    volume: Option<f64>,
    touched: bool,
}

async fn player<'a>(conn: &'a Connection, owner: &str) -> zbus::Result<Proxy<'a>> {
    zbus::proxy::Builder::new(conn)
        .destination(owner.to_owned())?
        .path(PATH)?
        .interface(PLAYER)?
        .cache_properties(zbus::proxy::CacheProperties::No)
        .build()
        .await
}

async fn hold(conn: &Connection, mode: Mode, mut ended: oneshot::Receiver<()>) -> zbus::Result<()> {
    let rule = MatchRule::builder()
        .msg_type(zbus::message::Type::Signal)
        .interface("org.freedesktop.DBus.Properties")?
        .member("PropertiesChanged")?
        .path(PATH)?
        .build();
    let mut signals = MessageStream::for_match_rule(rule, conn, Some(256)).await?;
    let bus = zbus::fdo::DBusProxy::new(conn).await?;
    let mut changed = Vec::new();
    let mut stopped = false;
    for name in bus.list_names().await? {
        if ended.try_recv() != Err(oneshot::error::TryRecvError::Empty) {
            stopped = true;
            break;
        }
        if !name.starts_with("org.mpris.MediaPlayer2.") {
            continue;
        }
        let Ok(owner) = bus.get_name_owner(name.as_str().try_into()?).await else {
            continue;
        };
        // Unique owners prevent restoring a different process after a player restarts.
        let owner = owner.to_string();
        if changed.iter().any(|p: &Changed| p.owner == owner) {
            continue;
        }
        let Ok(proxy) = player(conn, &owner).await else {
            continue;
        };
        if proxy
            .get_property::<String>("PlaybackStatus")
            .await
            .ok()
            .as_deref()
            != Some("Playing")
        {
            continue;
        }
        let volume = match mode {
            Mode::Lower => {
                let Ok(volume) = proxy.get_property::<f64>("Volume").await else {
                    continue;
                };
                if !volume.is_finite() || volume <= 0.25 {
                    continue;
                }
                Some(volume)
            }
            Mode::Pause => None,
            Mode::Leave => return Ok(()),
        };
        // Remember before the write: a timed-out reply can still have changed the player.
        changed.push(Changed {
            owner,
            volume,
            touched: false,
        });
        match mode {
            Mode::Lower => {
                let _ = proxy.set_property("Volume", 0.25_f64).await;
            }
            Mode::Pause => {
                let _ = proxy.call_method("Pause", &()).await;
            }
            Mode::Leave => {}
        }
    }
    if !stopped {
        loop {
            tokio::select! {
                _ = &mut ended => break,
                signal = signals.next() => match signal {
                    Some(Ok(message)) => observe(&mut changed, &message),
                    _ => {
                        for player in &mut changed { player.touched = true; }
                        break;
                    },
                }
            }
        }
    }
    for index in 0..changed.len() {
        drain(&mut signals, &mut changed);
        if changed[index].touched {
            continue;
        }
        let Ok(proxy) = player(conn, &changed[index].owner).await else {
            continue;
        };
        let volume = changed[index].volume;
        let unchanged = if volume.is_some() {
            proxy.get_property::<f64>("Volume").await.ok() == Some(0.25)
        } else {
            proxy
                .get_property::<String>("PlaybackStatus")
                .await
                .ok()
                .as_deref()
                == Some("Paused")
        };
        // Reading the current state can also deliver earlier signals, including a change back.
        drain(&mut signals, &mut changed);
        if !unchanged || changed[index].touched {
            continue;
        }
        if let Some(volume) = volume {
            let _ = proxy.set_property("Volume", volume).await;
        } else {
            let _ = proxy.call_method("Play", &()).await;
        }
    }
    Ok(())
}

fn drain(signals: &mut MessageStream, changed: &mut [Changed]) {
    loop {
        match signals.next().now_or_never() {
            Some(Some(Ok(message))) => observe(changed, &message),
            None => return,
            _ => {
                for player in changed {
                    player.touched = true;
                }
                return;
            }
        }
    }
}

fn observe(players: &mut [Changed], message: &zbus::Message) {
    let header = message.header();
    let Some(sender) = header.sender() else { return };
    let Some(player) = players.iter_mut().find(|p| p.owner == sender.as_str()) else {
        return;
    };
    let Ok((interface, values, invalidated)) =
        message
            .body()
            .deserialize::<(String, HashMap<String, OwnedValue>, Vec<String>)>()
    else {
        return;
    };
    if interface != PLAYER {
        return;
    }
    let property = if player.volume.is_some() {
        "Volume"
    } else {
        "PlaybackStatus"
    };
    if invalidated.iter().any(|p| p == property || p == "Volume") {
        player.touched = true;
    }
    if let Some(value) = values.get("Volume") {
        player.touched |= player.volume.is_none() || f64::try_from(value).ok() != Some(0.25);
    }
    if player.volume.is_none()
        && let Some(value) = values.get("PlaybackStatus")
    {
        player.touched |= <&str>::try_from(value).ok() != Some("Paused");
    }
}

#[cfg(test)]
mod tests;
