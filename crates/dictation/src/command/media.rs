//! Local playback requests refer to the selected player, never to project notification mute.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaAction {
    Pause,
    Resume,
    Next,
    Previous,
    Volume(u8),
    Quieter,
    Louder,
}

pub(super) fn parse(text: &str) -> Option<MediaAction> {
    use MediaAction::*;
    for (phrases, action) in [
        (
            &[
                "pause media",
                "pause music",
                "stop the music",
                "pause the music",
                "pausar a midia",
                "pause a midia",
                "pare a midia",
                "pausar a musica",
                "pause a musica",
                "pare a musica",
                "pausa la musica",
                "deten la musica",
            ][..],
            Pause,
        ),
        (
            &[
                "resume media",
                "resume music",
                "play music",
                "retomar a midia",
                "retome a midia",
                "retomar a musica",
                "retome a musica",
                "reproduzir musica",
                "reanuda la musica",
            ][..],
            Resume,
        ),
        (
            &[
                "next track",
                "next song",
                "skip track",
                "proxima musica",
                "proxima faixa",
                "pule a musica",
                "siguiente cancion",
            ][..],
            Next,
        ),
        (
            &[
                "previous track",
                "previous song",
                "musica anterior",
                "faixa anterior",
                "cancion anterior",
            ][..],
            Previous,
        ),
        (
            &[
                "lower media volume",
                "lower music volume",
                "abaixe a musica",
                "baixar o volume da midia",
                "diminuir o volume da midia",
                "baja el volumen de la musica",
            ][..],
            Quieter,
        ),
        (
            &[
                "raise media volume",
                "raise music volume",
                "aumente a musica",
                "aumentar o volume da midia",
                "sube el volumen de la musica",
            ][..],
            Louder,
        ),
    ] {
        if phrases.contains(&text) {
            return Some(action);
        }
    }
    volume_argument(text).and_then(super::percentage).map(Volume)
}

pub(super) fn volume_argument(text: &str) -> Option<&str> {
    [
        "media volume ",
        "set media volume to ",
        "volume da midia ",
        "volume da musica ",
        "defina o volume da midia para ",
        "volumen de la musica ",
    ]
    .iter()
    .find_map(|prefix| text.strip_prefix(prefix))
}
