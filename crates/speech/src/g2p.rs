//! Text to the phonemes Kokoro reads. English through misaki-rs (MIT, built without espeak);
//! Portuguese through the system's espeak-ng, as a separate program.

use std::path::PathBuf;

use crate::{Lang, espeak};

pub struct G2p {
    en: misaki_rs::G2P,
    /// Where espeak-ng is; tests point it at a stand-in. None looks it up on the PATH each time,
    /// so one installed while the app runs is used at once.
    espeak: Option<PathBuf>,
}

impl std::fmt::Debug for G2p {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("G2p")
    }
}

impl G2p {
    /// Reads misaki's lexicon (embedded): about a second, on the speaker's thread.
    pub fn new(espeak: Option<PathBuf>) -> Self {
        // Words misaki does not know (names like "Zeca") go to espeak-ng too, when it is there.
        let fallback = Unknown(espeak.clone());
        Self {
            en: misaki_rs::G2P::with_fallback(misaki_rs::Language::EnglishUS, Some(Box::new(fallback))),
            espeak,
        }
    }

    fn espeak(&self) -> Option<PathBuf> {
        self.espeak.clone().or_else(espeak::find)
    }

    pub fn phonemes(&self, text: &str, lang: Lang) -> Result<String, String> {
        match lang {
            Lang::En => self
                .en
                .g2p(text)
                .map(|(p, _)| misaki_letters(&p))
                .map_err(|e| format!("can't read the sentence: {e}")),
            Lang::Pt => {
                let bin = self.espeak().ok_or(espeak::MISSING)?;
                espeak::ipa(&bin, "pt-br", text)
            }
        }
    }
}

/// English words misaki has no entry for, through espeak-ng when it is installed. Without it misaki
/// gets nothing back for them and says what it can.
struct Unknown(Option<PathBuf>);

impl misaki_rs::Fallback for Unknown {
    fn phonemize(&self, word: &str) -> Result<String, misaki_rs::fallback::FallbackError> {
        let error = |error: String| misaki_rs::fallback::FallbackError::Espeak {
            word: word.into(),
            error,
        };
        let bin = self
            .0
            .clone()
            .or_else(espeak::find)
            .ok_or_else(|| error(espeak::MISSING.into()))?;
        let p = espeak::ipa(&bin, "en-us", word).map_err(error)?;
        // misaki's American conventions: an r is turned (ɹ), and no length marks.
        Ok(p.replace('r', "\u{279}").replace('\u{2d0}', ""))
    }
}

/// misaki joins a diphthong's letters with a zero-width joiner; Kokoro's vocabulary has one
/// letter for each.
fn misaki_letters(p: &str) -> String {
    let mut p = p.to_string();
    for (joined, letter) in [
        ("a\u{200d}\u{26a}", "I"),
        ("a\u{200d}\u{28a}", "W"),
        ("e\u{200d}\u{26a}", "A"),
        ("o\u{200d}\u{28a}", "O"),
        ("\u{254}\u{200d}\u{26a}", "Y"),
        ("\u{259}\u{200d}\u{28a}", "Q"),
        ("d\u{200d}\u{292}", "\u{2a4}"),
        ("t\u{200d}\u{283}", "\u{2a7}"),
    ] {
        p = p.replace(joined, letter);
    }
    p.replace('\u{200d}', "")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .replace(" .", ".")
        .replace(" ,", ",")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn misakis_joined_letters_become_kokoros() {
        assert_eq!(
            misaki_letters("h\u{259}l\u{2c8}o\u{200d}\u{28a} ."),
            "h\u{259}l\u{2c8}O."
        );
        assert_eq!(
            misaki_letters("t\u{200d}\u{283}e\u{200d}\u{26a}n  ,"),
            "\u{2a7}An,"
        );
    }

    #[test]
    fn portuguese_without_espeak_is_not_spoken() {
        let g = G2p::new(Some(PathBuf::from("/nonexistent/espeak-ng")));
        assert!(g.phonemes("Oi, eu sou o Zeca.", Lang::Pt).is_err());
        // English needs no espeak: the lexicon has the words.
        let en = g.phonemes("I finished the review.", Lang::En).expect("english");
        assert!(en.contains('\u{2c8}') && !en.contains('\u{200d}'), "{en}");
    }
}
