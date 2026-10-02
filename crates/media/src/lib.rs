//! What is playing on the computer, and its play/pause and skip controls, for the island. On
//! Linux that is every MPRIS player on the session bus (Spotify, browsers, mpv, …). Only read
//! while the user has the setting on; nothing leaves the machine.

use serde::{Deserialize, Serialize};

#[cfg(target_os = "linux")]
mod mpris;
#[cfg(target_os = "linux")]
pub use mpris::{control, watch};

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
pub struct NowPlaying {
    pub title: String,
    pub artist: Option<String>,
    pub playing: bool,
    /// The player's bus name: the controls go to the one on screen.
    #[serde(skip)]
    pub player: String,
}

#[derive(Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Control {
    PlayPause,
    Next,
    Previous,
}

/// One player as read from the bus.
#[derive(Clone, Debug)]
pub struct Player {
    pub name: String,
    pub status: Status,
    pub title: String,
    pub artist: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Playing,
    Paused,
    Stopped,
}

/// The player to show: one that plays (the one already shown first), else the one shown a
/// moment ago if it is only paused, so its play button still works. A tab paused hours ago
/// never comes back on its own.
pub fn pick(players: &[Player], shown: Option<&str>) -> Option<NowPlaying> {
    let playing = |p: &&Player| p.status == Status::Playing && !p.title.is_empty();
    let chosen = players
        .iter()
        .filter(playing)
        .find(|p| Some(p.name.as_str()) == shown)
        .or_else(|| players.iter().find(playing))
        .or_else(|| {
            players
                .iter()
                .find(|p| Some(p.name.as_str()) == shown && p.status == Status::Paused && !p.title.is_empty())
        })?;
    let artist = chosen
        .artist
        .as_deref()
        .map(clean_artist)
        .filter(|a| !a.is_empty());
    Some(NowPlaying {
        title: clean_title(&chosen.title, artist.as_deref()),
        artist,
        playing: chosen.status == Status::Playing,
        player: chosen.name.clone(),
    })
}

/// `Song (Official Video) [4K]` → `Song`; `Artist - Song` → `Song` when the artist is known;
/// `Song - Remastered 2011` → `Song`.
pub fn clean_title(raw: &str, artist: Option<&str>) -> String {
    let mut out = String::new();
    let mut depth = 0u32;
    for c in raw.chars() {
        match c {
            '(' | '[' => depth += 1,
            ')' | ']' => depth = depth.saturating_sub(1),
            _ if depth == 0 => out.push(c),
            _ => {}
        }
    }
    let title = out.split_whitespace().collect::<Vec<_>>().join(" ");
    let title = match (title.split_once(" - "), artist) {
        (Some((head, tail)), Some(a)) if head.eq_ignore_ascii_case(a) => tail.to_string(),
        (Some((head, _)), _) => head.to_string(),
        (None, _) => title,
    };
    let title = title.trim().to_string();
    // Nothing left (a title made only of brackets): the raw one beats an empty line.
    if title.is_empty() {
        raw.trim().to_string()
    } else {
        title
    }
}

/// `Artist feat. Guest` → `Artist`.
pub fn clean_artist(raw: &str) -> String {
    let lower = raw.to_lowercase();
    let cut = [" feat. ", " ft. ", " featuring "]
        .iter()
        .filter_map(|m| lower.find(m))
        .min()
        .unwrap_or(raw.len());
    // `to_lowercase` can change byte lengths; only cut on a boundary of the original.
    let cut = if raw.is_char_boundary(cut) { cut } else { raw.len() };
    raw[..cut].trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn player(name: &str, status: Status, title: &str) -> Player {
        Player {
            name: name.into(),
            status,
            title: title.into(),
            artist: Some("Band".into()),
        }
    }

    #[test]
    fn a_playing_player_wins_and_the_shown_one_keeps_its_place() {
        let firefox = player("org.mpris.MediaPlayer2.firefox", Status::Playing, "Clip");
        let spotify = player("org.mpris.MediaPlayer2.spotify", Status::Playing, "Song");
        let both = [firefox.clone(), spotify.clone()];
        assert_eq!(pick(&both, None).unwrap().title, "Clip");
        assert_eq!(pick(&both, Some(&spotify.name)).unwrap().title, "Song");
    }

    #[test]
    fn a_pause_keeps_the_shown_player_but_never_brings_back_an_old_one() {
        let paused = player("org.mpris.MediaPlayer2.spotify", Status::Paused, "Song");
        let shown = pick(std::slice::from_ref(&paused), Some(&paused.name)).unwrap();
        assert!(!shown.playing);
        assert_eq!(pick(std::slice::from_ref(&paused), None), None);
        let stopped = player("org.mpris.MediaPlayer2.mpv", Status::Stopped, "Film");
        assert_eq!(pick(std::slice::from_ref(&stopped), Some(&stopped.name)), None);
    }

    #[test]
    fn titles_and_artists_lose_their_noise() {
        assert_eq!(clean_title("Song (Official Video) [4K]", None), "Song");
        assert_eq!(clean_title("Band - Song (Lyrics)", Some("Band")), "Song");
        assert_eq!(clean_title("Song - Remastered 2011", Some("Band")), "Song");
        assert_eq!(clean_title("(Intro)", None), "(Intro)");
        assert_eq!(clean_artist("Band feat. Guest"), "Band");
        assert_eq!(clean_artist("Band Ft. Guest"), "Band");
        assert_eq!(clean_artist("Band"), "Band");
    }
}
