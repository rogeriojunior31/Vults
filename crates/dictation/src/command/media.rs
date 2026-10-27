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
    if let Some(action) = playback(text) {
        return Some(action);
    }
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

// Match the whole request, so negation, named players and unrelated suffixes cannot run.
fn playback(text: &str) -> Option<MediaAction> {
    for (verbs, objects, action) in [
        (
            &["pause", "stop"][..],
            &[
                "what is playing",
                "what i am listening to",
                "what im listening to",
            ][..],
            MediaAction::Pause,
        ),
        (
            &["pausa", "pause", "pausar", "para", "pare", "parar"][..],
            &[
                "a musica",
                "a midia",
                "o que esta tocando",
                "o que estou ouvindo",
                "o que eu estou ouvindo",
                "o que estou escutando",
                "o que eu estou escutando",
            ][..],
            MediaAction::Pause,
        ),
        (
            &["pausa", "deten"][..],
            &["lo que esta sonando", "lo que estoy escuchando"][..],
            MediaAction::Pause,
        ),
        (
            &["resume", "continue"][..],
            &["what is playing", "what i was listening to"][..],
            MediaAction::Resume,
        ),
        (
            &["continua", "continue", "continuar", "retoma", "retome", "retomar"][..],
            &[
                "a musica",
                "a midia",
                "o que estava tocando",
                "o que estava ouvindo",
                "o que eu estava ouvindo",
                "o que estava escutando",
                "o que eu estava escutando",
            ][..],
            MediaAction::Resume,
        ),
        (
            &["reanuda", "continua"][..],
            &["lo que estaba sonando", "lo que estaba escuchando"][..],
            MediaAction::Resume,
        ),
    ] {
        if verbs.iter().any(|verb| {
            text.strip_prefix(verb)
                .and_then(|rest| rest.strip_prefix(' '))
                .is_some_and(|object| objects.contains(&object))
        }) {
            return Some(action);
        }
    }
    None
}
