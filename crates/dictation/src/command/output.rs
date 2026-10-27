//! Explicit computer-output scope; an unspecified sound request must not pick a player or device.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutputAction {
    Mute(bool),
    Volume(u8),
    Louder,
    Quieter,
}

pub(super) fn parse(text: &str) -> Option<OutputAction> {
    use OutputAction::*;
    for (phrases, action) in [
        (
            &[
                "mute computer",
                "mute the computer",
                "mute computer audio",
                "mute system audio",
                "mute o som do computador",
                "mute o computador",
                "mute computador",
                "mute el ordenador",
                "mute el equipo",
            ][..],
            Mute(true),
        ),
        (
            &[
                "unmute computer",
                "unmute the computer",
                "unmute computer audio",
                "unmute system audio",
                "unmute computador",
                "unmute o computador",
                "unmute el ordenador",
                "unmute el equipo",
            ][..],
            Mute(false),
        ),
        (
            &[
                "raise computer volume",
                "raise system volume",
                "increase computer volume",
                "increase system volume",
                "turn up computer volume",
                "turn up system volume",
                "aumenta o volume do computador",
                "subir el volumen del ordenador",
                "aumentar o volume do computador",
                "aumente o volume do computador",
                "sube el volumen del ordenador",
            ][..],
            Louder,
        ),
        (
            &[
                "lower computer volume",
                "lower system volume",
                "decrease computer volume",
                "decrease system volume",
                "turn down computer volume",
                "turn down system volume",
                "abaixa o volume do computador",
                "reduza o volume do computador",
                "reduzir o volume do computador",
                "bajar el volumen del ordenador",
                "diminuir o volume do computador",
                "abaixe o volume do computador",
                "baja el volumen del ordenador",
            ][..],
            Quieter,
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
        "computer volume ",
        "set computer volume to ",
        "system volume ",
        "set system volume to ",
        "ajuste o volume do computador para ",
        "pon el volumen del ordenador a ",
        "volume do computador ",
        "defina o volume do computador para ",
        "volumen del ordenador ",
    ]
    .iter()
    .find_map(|prefix| text.strip_prefix(prefix))
}

// Unscoped volume requests must not guess between player and computer audio or reach Chat.
pub(super) fn needs_scope(text: &str) -> bool {
    [
        "lower volume",
        "lower the volume",
        "raise volume",
        "raise the volume",
        "increase volume",
        "increase the volume",
        "decrease volume",
        "decrease the volume",
        "turn up volume",
        "turn up the volume",
        "turn down volume",
        "turn down the volume",
        "abaixar o volume",
        "reduzir o volume",
        "abaixa o volume",
        "abaixe o volume",
        "baixar o volume",
        "diminua o volume",
        "diminuir o volume",
        "reduza o volume",
        "aumenta o volume",
        "aumente o volume",
        "aumentar o volume",
        "aumente o som",
        "abaixe o som",
        "baja el volumen",
        "bajar el volumen",
        "sube el volumen",
        "subir el volumen",
    ]
    .contains(&text)
        || ["volume ", "set volume to ", "volume para ", "pon el volumen a "]
            .iter()
            .filter_map(|prefix| text.strip_prefix(prefix))
            .any(|value| {
                let value = super::volume_number(value);
                !value.is_empty()
                    && value
                        .split_whitespace()
                        .all(|part| part.bytes().all(|ch| ch.is_ascii_digit()))
            })
}
