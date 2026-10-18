//! Background text copying without moving focus. Keep the X11 owner alive until app exit.

use std::io::Read;
use wl_clipboard_rs::{copy, paste};

pub enum Clipboard {
    Wayland,
    X11(arboard::Clipboard),
}

impl std::fmt::Debug for Clipboard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Wayland => "Clipboard(Wayland)",
            Self::X11(_) => "Clipboard(X11)",
        })
    }
}

impl Clipboard {
    pub fn new() -> Result<Self, String> {
        let wayland = std::env::var_os("WAYLAND_DISPLAY").is_some_and(|s| !s.is_empty())
            || std::env::var("XDG_SESSION_TYPE").is_ok_and(|s| s.eq_ignore_ascii_case("wayland"));
        if wayland {
            // An XWayland fallback could confirm its own copy while native Wayland stays unchanged.
            wl_clipboard_rs::utils::is_primary_selection_supported()
                .map_err(|_| "This Wayland desktop does not provide background clipboard access.")?;
            Ok(Self::Wayland)
        } else {
            arboard::Clipboard::new()
                .map(Self::X11)
                .map_err(|_| "The X11 clipboard is unavailable.".into())
        }
    }

    /// A successful setter alone is not proof that the compositor accepted the selection.
    pub fn copy(&mut self, text: &str) -> Result<(), String> {
        if text.contains('\0') {
            return Err("Dictation text contains a NUL character.".into());
        }
        match self {
            Self::Wayland => copy::Options::new()
                .copy(copy::Source::Bytes(text.as_bytes().into()), copy::MimeType::Text)
                .map_err(|_| "The Wayland clipboard refused the text.".to_string())?,
            Self::X11(clipboard) => clipboard
                .set_text(text)
                .map_err(|_| "The X11 clipboard refused the text.".to_string())?,
        }
        self.confirm(text)
    }

    pub fn text(&mut self) -> Option<String> {
        match self {
            Self::X11(clipboard) => clipboard.get_text().ok(),
            Self::Wayland => {
                let (pipe, _) = paste::get_contents(
                    paste::ClipboardType::Regular,
                    paste::Seat::Unspecified,
                    paste::MimeType::Text,
                )
                .ok()?;
                let mut bytes = Vec::new();
                pipe.take(65_537).read_to_end(&mut bytes).ok()?;
                String::from_utf8(bytes).ok()
            }
        }
    }

    pub fn confirm(&mut self, text: &str) -> Result<(), String> {
        let matches = match self {
            Self::Wayland => {
                let (pipe, _) = paste::get_contents(
                    paste::ClipboardType::Regular,
                    paste::Seat::Unspecified,
                    paste::MimeType::Text,
                )
                .map_err(|_| "The Wayland clipboard did not confirm the text.")?;
                let mut received = Vec::new();
                pipe.take(text.len() as u64 + 1)
                    .read_to_end(&mut received)
                    .map_err(|_| "The Wayland clipboard could not be read back.")?;
                received == text.as_bytes()
            }
            Self::X11(clipboard) => {
                clipboard
                    .get_text()
                    .map_err(|_| "The X11 clipboard did not confirm the text.")?
                    == text
            }
        };
        if !matches {
            return Err("The clipboard changed before the copy could be confirmed.".into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::process::{Command, Stdio};

    fn pasted() -> Vec<u8> {
        let output = Command::new("wl-paste")
            .arg("--no-newline")
            .output()
            .expect("wl-paste");
        assert!(
            output.status.success(),
            "native smoke requires an existing text clipboard"
        );
        output.stdout
    }

    struct Restore(Vec<u8>);
    const FIRST: &str = "vults background clipboard smoke: first";
    const SECOND: &str = "vults background clipboard smoke: second\nUnicode: \u{4e2d}\u{6587}";

    impl Drop for Restore {
        fn drop(&mut self) {
            let current = pasted();
            if current != FIRST.as_bytes() && current != SECOND.as_bytes() {
                return;
            }
            let mut child = Command::new("wl-copy")
                .args(["--type", "text/plain;charset=utf-8"])
                .stdin(Stdio::piped())
                .spawn()
                .expect("wl-copy");
            child
                .stdin
                .take()
                .expect("stdin")
                .write_all(&self.0)
                .expect("restore text");
            assert!(child.wait().expect("restore status").success());
        }
    }

    #[test]
    #[ignore = "native Wayland smoke; requires wl-copy/wl-paste and temporarily replaces clipboard text"]
    fn background_copy_reaches_an_independent_wayland_client() {
        let mut clipboard = Clipboard::new().expect("native clipboard");
        assert!(matches!(clipboard, Clipboard::Wayland));
        let _restore = Restore(pasted());
        clipboard.copy(FIRST).expect("first copy");
        assert!(
            pasted() == FIRST.as_bytes(),
            "the independent client must receive the first copy"
        );
        clipboard.copy(SECOND).expect("second copy");
        assert!(
            pasted() == SECOND.as_bytes(),
            "the independent client must receive the replacement copy"
        );
        assert!(clipboard.copy("invalid\0text").is_err());
        assert!(
            pasted() == SECOND.as_bytes(),
            "invalid input must not replace the clipboard"
        );
    }
}
