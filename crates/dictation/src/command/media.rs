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
    for prefix in [
        "media volume ",
        "set media volume to ",
        "volume da midia ",
        "volume da musica ",
        "defina o volume da midia para ",
        "volumen de la musica ",
    ] {
        if let Some(value) = text.strip_prefix(prefix) {
            let value = value
                .strip_suffix(" percent")
                .or_else(|| value.strip_suffix(" por cento"))
                .or_else(|| value.strip_suffix(" por ciento"))
                .unwrap_or(value);
            return value.parse::<u8>().ok().filter(|value| *value <= 100).map(Volume);
        }
    }
    None
}
