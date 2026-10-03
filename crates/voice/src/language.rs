//! What the user speaks: Brazilian Portuguese or English. Whisper's own detection weighs about a
//! hundred languages and, on a short question, takes Portuguese for English (a name or a loanword
//! is enough) or for Spanish; told the language, it transcribes the same clips right. Auto only
//! ever chooses between these two.
//!
//! No initial prompt: measured on Brazilian Portuguese speech, a sample sentence made the smaller
//! models invent whole phrases that were never said.

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    /// Portuguese or English, whichever the recording sounds more like.
    #[default]
    Auto,
    /// Brazilian Portuguese.
    Pt,
    En,
}

/// Whisper's code for a language that was settled.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Spoken {
    Pt,
    En,
}

impl Spoken {
    pub(crate) fn code(self) -> &'static str {
        match self {
            Spoken::Pt => "pt",
            Spoken::En => "en",
        }
    }

    /// The more likely of the two, from whisper's language probabilities.
    pub(crate) fn likelier(pt: f32, en: f32) -> Self {
        if pt >= en { Spoken::Pt } else { Spoken::En }
    }
}

impl Language {
    pub(crate) fn settled(self) -> Option<Spoken> {
        match self {
            Language::Auto => None,
            Language::Pt => Some(Spoken::Pt),
            Language::En => Some(Spoken::En),
        }
    }

    /// The desktop's language as a default: `pt_BR.UTF-8` speaks Portuguese, `en_US` English,
    /// anything else either.
    pub fn from_locale(locale: &str) -> Self {
        let lang = locale.split(['_', '.', '-', '@']).next().unwrap_or_default();
        match lang.to_ascii_lowercase().as_str() {
            "pt" => Language::Pt,
            "en" => Language::En,
            _ => Language::Auto,
        }
    }

    /// From `LC_ALL`, `LC_MESSAGES` or `LANG`, the first one set.
    pub fn of_desktop() -> Self {
        ["LC_ALL", "LC_MESSAGES", "LANG"]
            .iter()
            .filter_map(|v| std::env::var(v).ok())
            .find(|v| !v.is_empty() && v != "C" && v != "POSIX")
            .map_or(Language::Auto, |v| Self::from_locale(&v))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_desktop_language_is_the_default() {
        assert_eq!(Language::from_locale("pt_BR.UTF-8"), Language::Pt);
        assert_eq!(Language::from_locale("pt-BR"), Language::Pt);
        assert_eq!(Language::from_locale("en_US.UTF-8"), Language::En);
        assert_eq!(Language::from_locale("de_DE.UTF-8"), Language::Auto);
        assert_eq!(Language::from_locale(""), Language::Auto);
    }

    #[test]
    fn auto_only_chooses_between_the_two() {
        assert_eq!(Spoken::likelier(0.6, 0.1), Spoken::Pt);
        assert_eq!(Spoken::likelier(0.05, 0.7), Spoken::En);
        assert_eq!(Language::Auto.settled(), None);
        assert_eq!(Language::Pt.settled().map(Spoken::code), Some("pt"));
    }

    #[test]
    fn the_wire_names_are_short() {
        assert_eq!(serde_json::to_string(&Language::Pt).unwrap(), "\"pt\"");
        assert_eq!(
            serde_json::from_str::<Language>("\"auto\"").unwrap(),
            Language::Auto
        );
    }
}
