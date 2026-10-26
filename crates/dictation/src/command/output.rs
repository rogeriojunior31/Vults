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
        "volume do computador ",
        "defina o volume do computador para ",
        "volumen del ordenador ",
    ]
    .iter()
    .find_map(|prefix| text.strip_prefix(prefix))
}
