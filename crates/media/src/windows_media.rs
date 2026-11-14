//! Windows system media sessions, read only while the user enables Now Playing.
use crate::{Control, NowPlaying, Player, Status, pick};
use std::time::Duration;
use windows::Media::Control::{
    GlobalSystemMediaTransportControlsSession as Session,
    GlobalSystemMediaTransportControlsSessionManager as Manager,
    GlobalSystemMediaTransportControlsSessionPlaybackStatus as Playback,
};

fn text(error: windows::core::Error) -> String {
    error.to_string()
}

async fn manager() -> Result<Manager, String> {
    tokio::time::timeout(Duration::from_secs(3), Manager::RequestAsync().map_err(text)?)
        .await
        .map_err(|_| "The Windows media service did not respond.".to_string())?
        .map_err(text)
}

async fn player(session: &Session) -> Result<Player, String> {
    let status = match session.GetPlaybackInfo().and_then(|info| info.PlaybackStatus()) {
        Ok(Playback::Playing) => Status::Playing,
        Ok(Playback::Paused) => Status::Paused,
        _ => Status::Stopped,
    };
    let properties = tokio::time::timeout(
        Duration::from_secs(2),
        session.TryGetMediaPropertiesAsync().map_err(text)?,
    )
    .await
    .map_err(|_| "The media player did not respond.".to_string())?
    .map_err(text)?;
    Ok(Player {
        name: session.SourceAppUserModelId().map_err(text)?.to_string(),
        title: properties.Title().map_err(text)?.to_string(),
        artist: Some(properties.Artist().map_err(text)?.to_string()),
        status,
    })
}

async fn players(manager: &Manager) -> Result<Vec<Player>, String> {
    let sessions: Vec<_> = manager.GetSessions().map_err(text)?.into_iter().collect();
    let mut players = Vec::new();
    for session in sessions {
        // One player that hangs or fails must not hide the others.
        if let Ok(player) = player(&session).await {
            players.push(player);
        }
    }
    Ok(players)
}

/// Reads in a row that fail before the media service is asked for again.
const RETRIES: u32 = 3;

/// Only a service that never answers at the start is an error: a failed read later keeps the song
/// on screen and tries again, and the service is asked for again when it keeps failing (it
/// restarts with Explorer and with updates).
pub async fn watch(on_change: impl Fn(Option<NowPlaying>)) -> Result<(), String> {
    let mut manager = manager().await?;
    let mut shown: Option<NowPlaying> = None;
    let mut first = true;
    let mut failures = 0;
    loop {
        match players(&manager).await {
            Ok(players) => {
                failures = 0;
                let now = pick(&players, shown.as_ref().map(|now| now.player.as_str()));
                if first || now != shown {
                    on_change(now.clone());
                    shown = now;
                    first = false;
                }
            }
            Err(_) => {
                failures += 1;
                if failures >= RETRIES
                    && let Ok(again) = self::manager().await
                {
                    manager = again;
                    failures = 0;
                }
            }
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}

/// Keep the exact session object: a restarted player cannot inherit a delayed command.
#[derive(Debug)]
pub struct Target {
    manager: Manager,
    session: Session,
}

pub async fn pin(player: &str) -> Result<Target, String> {
    let manager = manager().await?;
    let mut matching = manager
        .GetSessions()
        .map_err(text)?
        .into_iter()
        .filter(|session| session.SourceAppUserModelId().is_ok_and(|id| id == player));
    let session = matching
        .next()
        .ok_or("The media player is no longer available.")?;
    if matching.next().is_some() {
        return Err("The media player has multiple sessions.".into());
    }
    Ok(Target { manager, session })
}

impl Target {
    pub fn present(&self) -> bool {
        self.manager.GetSessions().is_ok_and(|sessions| {
            let Ok(id) = self.session.SourceAppUserModelId() else {
                return false;
            };
            let matching: Vec<_> = sessions
                .into_iter()
                .filter(|session| session.SourceAppUserModelId().is_ok_and(|source| source == id))
                .collect();
            matching.len() == 1 && matching[0] == self.session
        })
    }

    fn prepare(&self, action: Control, allowed: &impl Fn() -> bool) -> Result<(), String> {
        action.validate()?;
        if !allowed() || !self.present() {
            return Err("Media command cancelled or player changed.".into());
        }
        let controls = self
            .session
            .GetPlaybackInfo()
            .and_then(|info| info.Controls())
            .map_err(text)?;
        let supported = match action {
            Control::PlayPause => controls.IsPlayPauseToggleEnabled(),
            Control::Play => controls.IsPlayEnabled(),
            Control::Pause => controls.IsPauseEnabled(),
            Control::Next => controls.IsNextEnabled(),
            Control::Previous => controls.IsPreviousEnabled(),
            Control::Volume(_) | Control::VolumeUp | Control::VolumeDown => {
                return Err("This player does not support the requested control.".into());
            }
        }
        .map_err(text)?;
        if !supported {
            return Err("This player does not support the requested control.".into());
        }
        Ok(())
    }

    pub async fn validate_batch(
        &self,
        actions: &[Control],
        allowed: impl Fn() -> bool,
    ) -> Result<(), String> {
        if !allowed() || !self.present() {
            return Err("Media command cancelled or player changed.".into());
        }
        for action in actions {
            self.prepare(*action, &allowed)?;
        }
        Ok(())
    }

    pub async fn control(&self, action: Control, allowed: impl Fn() -> bool) -> Result<(), String> {
        self.prepare(action, &allowed)?;
        if !allowed() {
            return Err("Media command cancelled.".into());
        }
        let request = match action {
            Control::PlayPause => self.session.TryTogglePlayPauseAsync(),
            Control::Play => self.session.TryPlayAsync(),
            Control::Pause => self.session.TryPauseAsync(),
            Control::Next => self.session.TrySkipNextAsync(),
            Control::Previous => self.session.TrySkipPreviousAsync(),
            _ => return Err("This player does not support the requested control.".into()),
        }
        .map_err(text)?;
        if tokio::time::timeout(Duration::from_secs(3), request)
            .await
            .map_err(|_| "The media player did not respond.".to_string())?
            .map_err(text)?
        {
            Ok(())
        } else {
            Err("The media player refused the requested control.".into())
        }
    }
}

pub async fn control(player: &str, action: Control) -> Result<(), String> {
    pin(player).await?.control(action, || true).await
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    #[ignore = "requires the Windows system media service in an interactive session"]
    async fn native_media_service_can_be_read_and_an_absent_player_cannot_be_controlled() {
        let manager = super::manager().await.unwrap();
        super::players(&manager).await.unwrap();
        assert!(super::pin("vults.nonexistent.test.player").await.is_err());
    }
}
