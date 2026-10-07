//! The languages Zeca speaks, and a guess of which one a sentence is in.

use serde::Serialize;

#[derive(Serialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum Lang {
    En,
    Pt,
}

impl Lang {
    /// From a language code (`pt`, `pt-BR`, `en`); any other is spoken as English.
    pub fn from_code(code: &str) -> Self {
        if code.get(..2).is_some_and(|c| c.eq_ignore_ascii_case("pt")) {
            Self::Pt
        } else {
            Self::En
        }
    }

    pub fn code(self) -> &'static str {
        match self {
            Self::En => "en",
            Self::Pt => "pt",
        }
    }
}

/// Short words that only one of the two languages uses often.
const EN: &[&str] = &[
    "the", "is", "are", "was", "were", "to", "of", "and", "you", "it", "this", "that", "for", "with", "not",
    "be", "have", "has", "i", "on", "what", "can", "will", "file", "it's", "i'll", "there", "here", "your",
    "from",
];
const PT: &[&str] = &[
    "o", "os", "as", "de", "do", "da", "dos", "das", "que", "um", "uma", "em", "na", "para", "com", "por",
    "mais", "isso", "isto", "este", "esta", "eu", "foi", "como", "mas", "voce", "ja", "nao", "e", "arquivo",
    "aqui", "seu", "sua", "tem",
];

/// English or Portuguese, by the words used; None when the sentence does not say (a name, a
/// number, "OK.").
pub fn detect(sentence: &str) -> Option<Lang> {
    let mut en = 0;
    let mut pt = 0;
    for word in sentence
        .split(|c: char| !(c.is_alphabetic() || c == '\''))
        .filter(|w| !w.is_empty())
    {
        let lower = word.to_lowercase();
        // Accents that English does not write weigh as much as a word.
        if lower
            .chars()
            .any(|c| "\u{e3}\u{f5}\u{e7}\u{e1}\u{e9}\u{ed}\u{f3}\u{fa}\u{e2}\u{ea}\u{f4}\u{e0}".contains(c))
        {
            pt += 1;
        }
        let plain: String = lower.chars().map(unaccent).collect();
        if EN.contains(&lower.as_str()) {
            en += 1;
        } else if PT.contains(&plain.as_str()) {
            pt += 1;
        }
    }
    match en.cmp(&pt) {
        std::cmp::Ordering::Greater => Some(Lang::En),
        std::cmp::Ordering::Less => Some(Lang::Pt),
        std::cmp::Ordering::Equal => None,
    }
}

fn unaccent(c: char) -> char {
    match c {
        '\u{e1}' | '\u{e0}' | '\u{e2}' | '\u{e3}' => 'a',
        '\u{e9}' | '\u{ea}' => 'e',
        '\u{ed}' => 'i',
        '\u{f3}' | '\u{f4}' | '\u{f5}' => 'o',
        '\u{fa}' => 'u',
        '\u{e7}' => 'c',
        c => c,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_sentence_says_its_language() {
        assert_eq!(detect("I finished reviewing the pull request."), Some(Lang::En));
        assert_eq!(
            detect("Terminei de revisar o pull request e encontrei tr\u{ea}s problemas."),
            Some(Lang::Pt)
        );
        assert_eq!(detect("N\u{e3}o achei o arquivo."), Some(Lang::Pt));
        assert_eq!(detect("OK."), None);
        assert_eq!(detect("Zeca, 42."), None);
    }

    #[test]
    fn codes_become_languages() {
        assert_eq!(Lang::from_code("pt-BR"), Lang::Pt);
        assert_eq!(Lang::from_code("pt"), Lang::Pt);
        assert_eq!(Lang::from_code("es"), Lang::En);
        assert_eq!(Lang::Pt.code(), "pt");
    }
}
