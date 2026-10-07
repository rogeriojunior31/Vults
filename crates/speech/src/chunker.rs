//! Cuts a streaming reply into sentences, so each one is spoken as soon as it is whole.
//! Ported from Patter's `sentence-chunker.ts` (MIT, see NOTICE), without its first-clause flush:
//! abbreviations, acronyms, decimals and websites do not end a sentence.

use std::sync::LazyLock;

use regex::Regex;

/// Shorter sentences are merged with the next one: a voice says a few words better in context.
const MIN_LEN: usize = 20;
const TERMINATORS: &str = ".!?\u{2026};";
/// Stand-ins while cutting: a period that ends nothing, and a cut. Never in the input.
const PRD: char = '\u{E000}';
const STOP: char = '\u{E001}';

/// Titles that take a period and come before a name (en, it, es, de, fr, pt), longest first.
const HONORIFICS: &[&str] = &[
    "Licda", "Sres", "Sras", "Srta", "Srtas", "Mtro", "Mtra", "Profa", "Enga", "Dott", "Geom", "Spett",
    "Gent", "Arch", "Capt", "Cmdr", "Dipl", "Mlle", "Mlles", "Mmes", "Prof", "Dres", "Mrs", "Sig", "Sgr",
    "Avv", "Ing", "Rag", "Egr", "Ill", "Gen", "Sen", "Rep", "Cpt", "Col", "Adm", "Sra", "Srs", "Dra", "Lic",
    "Arq", "Frl", "Mag", "Mme", "Mgr", "Eng", "Mr", "St", "Ms", "Dr", "Lt", "On", "Sr", "Hr", "Fr", "MM",
    "Pr", "Me",
];

fn re(pattern: &str) -> Regex {
    #[allow(clippy::expect_used)] // A constant pattern: the tests compile every one.
    Regex::new(pattern).expect("a valid pattern")
}

static PREFIXES: LazyLock<Regex> = LazyLock::new(|| re(&format!(r"\b({})[.]", HONORIFICS.join("|"))));
static WEBSITES: LazyLock<Regex> = LazyLock::new(|| re(r"[.](com|net|org|io|gov|edu|me|dev|rs)\b"));
static DECIMALS: LazyLock<Regex> = LazyLock::new(|| re(r"([0-9])[.]([0-9])"));
static DOTS: LazyLock<Regex> = LazyLock::new(|| re(r"\.{2,}"));
static INITIAL: LazyLock<Regex> = LazyLock::new(|| re(r"\s([A-Za-z])[.] "));
static ACRONYM_STARTER: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"([A-Z][.][A-Z][.](?:[A-Z][.])?) (Mr|Mrs|Ms|Dr|Prof|Capt|Cpt|Lt|He\s|She\s|It\s|They\s|Their\s|Our\s|We\s|But\s|However\s|That\s|This\s|Wherever)",
    )
});
static THREE_LETTERS: LazyLock<Regex> = LazyLock::new(|| re(r"([A-Za-z])[.]([A-Za-z])[.]([A-Za-z])[.]"));
static TWO_LETTERS: LazyLock<Regex> = LazyLock::new(|| re(r"([A-Za-z])[.]([A-Za-z])[.]"));
const SUFFIXES: &str = "Inc|Ltd|Jr|Sr|Co|etc|vs|No|Vol|pp|cf|ca|op|Mt|Ave|Blvd|Sq|fig|vol|ed";
static SUFFIX_STARTER: LazyLock<Regex> = LazyLock::new(|| {
    re(&format!(
        r" ({SUFFIXES})[.] (Mr|Mrs|Ms|Dr|Prof|He\s|She\s|It\s|They\s|We\s|But\s|However\s|That\s|This\s)"
    ))
});
static SUFFIX: LazyLock<Regex> = LazyLock::new(|| re(&format!(r" ({SUFFIXES})[.]")));
static LETTER: LazyLock<Regex> = LazyLock::new(|| re(r" ([A-Za-z])[.]"));

/// The text cut into sentences of at least [`MIN_LEN`] characters (the last may be shorter).
fn split(text: &str) -> Vec<String> {
    let t: String = text
        .chars()
        .map(|c| if c == '\n' { ' ' } else { c })
        .filter(|c| *c != PRD && *c != STOP)
        .collect();
    let p = PRD.to_string();
    let t = PREFIXES.replace_all(&t, format!("${{1}}{p}"));
    let t = WEBSITES.replace_all(&t, format!("{p}${{1}}"));
    let t = DECIMALS.replace_all(&t, format!("${{1}}{p}${{2}}"));
    let t = DOTS.replace_all(&t, |c: &regex::Captures| p.repeat(c[0].len()));
    let t = t.replace("Ph.D.", &format!("Ph{p}D{p}"));
    let t = INITIAL.replace_all(&t, format!(" ${{1}}{p} "));
    let t = ACRONYM_STARTER.replace_all(&t, format!("${{1}}{STOP} ${{2}}"));
    let t = THREE_LETTERS.replace_all(&t, format!("${{1}}{p}${{2}}{p}${{3}}{p}"));
    let t = TWO_LETTERS.replace_all(&t, format!("${{1}}{p}${{2}}{p}"));
    let t = SUFFIX_STARTER.replace_all(&t, format!(" ${{1}}.{STOP} ${{2}}"));
    let t = SUFFIX.replace_all(&t, format!(" ${{1}}{p}"));
    let t = LETTER.replace_all(&t, format!(" ${{1}}{p}"));
    // A cut after each terminator, or after the quote that closes on it, when a space or the end
    // follows: mid-stream, "example." may still become "example.com".
    let mut marked = String::with_capacity(t.len() + 16);
    let mut chars = t.chars().peekable();
    while let Some(c) = chars.next() {
        marked.push(c);
        if TERMINATORS.contains(c) {
            if let Some(&q @ ('"' | '\u{201d}')) = chars.peek() {
                marked.push(q);
                chars.next();
            }
            if chars.peek().is_none_or(|n| n.is_whitespace()) {
                marked.push(STOP);
            }
        }
    }
    let marked = marked.replace(PRD, ".");
    let mut out = Vec::new();
    let mut buff = String::new();
    for part in marked.split(STOP) {
        let s = part.trim();
        if s.is_empty() {
            continue;
        }
        buff.push(' ');
        buff.push_str(s);
        if buff.chars().count() > MIN_LEN {
            out.push(buff.trim_start().to_string());
            buff.clear();
        }
    }
    if !buff.is_empty() {
        out.push(buff.trim_start().to_string());
    }
    out
}

/// Takes the reply as it streams; gives back the sentences that are whole.
#[derive(Debug, Default)]
pub struct Chunker {
    buffer: String,
}

impl Chunker {
    pub fn push(&mut self, text: &str) -> Vec<String> {
        self.buffer.push_str(text);
        if self.buffer.chars().count() < MIN_LEN {
            return self.short_flush();
        }
        let mut sentences = split(&self.buffer);
        if sentences.len() <= 1 {
            return Vec::new();
        }
        let mut last = sentences.pop().unwrap_or_default();
        // The cut trims the text: a space still due before the next word is kept.
        if self.buffer.ends_with(char::is_whitespace) {
            last.push(' ');
        }
        self.buffer = last;
        sentences.retain(|s| !s.is_empty());
        sentences
    }

    /// A short reply that is already whole ("Done.") goes at once, unless its period may belong
    /// to a number, an acronym or a title still to be followed by a name.
    fn short_flush(&mut self) -> Vec<String> {
        let stripped = self.buffer.trim_end();
        let Some(last) = stripped.chars().last() else {
            return Vec::new();
        };
        if !TERMINATORS.contains(last) || stripped.chars().filter(|c| TERMINATORS.contains(*c)).count() != 1 {
            return Vec::new();
        }
        let before = stripped.chars().rev().nth(1);
        if before.is_some_and(|c| c.is_ascii_digit()) {
            return Vec::new();
        }
        if last == '.' {
            let word = stripped
                .trim_end_matches(|c| TERMINATORS.contains(c))
                .split_whitespace()
                .last()
                .unwrap_or("");
            let acronym = (1..=3).contains(&word.len()) && word.chars().all(|c| c.is_ascii_uppercase());
            if acronym || HONORIFICS.contains(&word) {
                return Vec::new();
            }
        }
        let out = stripped.trim_start().to_string();
        self.buffer.clear();
        vec![out]
    }

    /// The end of the reply: whatever is left is the last sentence.
    pub fn flush(&mut self) -> Option<String> {
        let rest = std::mem::take(&mut self.buffer);
        let rest = rest.trim();
        (!rest.is_empty()).then(|| rest.to_string())
    }
}

/// A sentence longer than this is cut again: Kokoro takes at most 510 phonemes at once.
pub const MAX_CHARS: usize = 220;

/// Pieces of at most [`MAX_CHARS`], cut after a comma when one is near the end, else between words.
pub fn pieces(sentence: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = sentence.trim();
    while rest.chars().count() > MAX_CHARS {
        let limit = rest.char_indices().nth(MAX_CHARS).map_or(rest.len(), |(i, _)| i);
        let head = &rest[..limit];
        let cut = head
            .rfind([',', ';', ':'])
            .filter(|&i| i > limit / 2)
            .map(|i| i + 1)
            .or_else(|| head.rfind(' '))
            .filter(|&i| i > 0)
            .unwrap_or(limit);
        out.push(rest[..cut].trim().to_string());
        rest = rest[cut..].trim_start();
    }
    if !rest.is_empty() {
        out.push(rest.to_string());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Fed in small pieces, as a model streams.
    fn stream(text: &str, size: usize) -> Vec<String> {
        let mut c = Chunker::default();
        let chars: Vec<char> = text.chars().collect();
        let mut out = Vec::new();
        for piece in chars.chunks(size) {
            out.extend(c.push(&piece.iter().collect::<String>()));
        }
        out.extend(c.flush());
        out
    }

    #[test]
    fn a_reply_comes_out_sentence_by_sentence() {
        let text = "I read the file and found the bug. It was in the parser! Shall I fix it now?";
        for size in [1, 3, 7, 100] {
            assert_eq!(
                stream(text, size),
                [
                    "I read the file and found the bug.",
                    "It was in the parser!",
                    "Shall I fix it now?"
                ],
                "{size}"
            );
        }
    }

    #[test]
    fn periods_that_end_nothing_do_not_cut() {
        let out = stream(
            "Dr. Silva measured 3.14 seconds on example.com, e.g. on the U.S. servers. Then it stopped.",
            4,
        );
        assert_eq!(
            out,
            [
                "Dr. Silva measured 3.14 seconds on example.com, e.g. on the U.S. servers.",
                "Then it stopped."
            ]
        );
        assert_eq!(stream("Wait... what happened here? Nothing.", 2).len(), 2);
    }

    #[test]
    fn short_sentences_wait_for_company_and_a_short_reply_goes_at_once() {
        // Whole, the short ones ride with the next; one character at a time, each goes when done.
        let text = "Yes. I did it. All tests pass now.";
        assert_eq!(stream(text, 100), [text]);
        assert_eq!(stream(text, 1), ["Yes.", "I did it.", "All tests pass now."]);
        let mut c = Chunker::default();
        assert_eq!(c.push("Done."), ["Done."]);
        // A title or a number may go on.
        assert!(c.push("Mr.").is_empty());
        assert!(Chunker::default().push("Version 2.").is_empty());
    }

    #[test]
    fn spaces_between_pieces_survive_a_cut() {
        let out = stream("The build is green again. Next ", 100);
        let mut c = Chunker::default();
        let mut got = c.push("The build is green again. Next ");
        got.extend(c.push("step is the docs."));
        got.extend(c.flush());
        assert_eq!(got, ["The build is green again.", "Next step is the docs."]);
        assert_eq!(out, ["The build is green again.", "Next"]);
    }

    #[test]
    fn long_sentences_are_cut_for_the_voice() {
        let long = format!(
            "{}, and then {}",
            "word ".repeat(30).trim(),
            "more ".repeat(40).trim()
        );
        let parts = pieces(&long);
        assert!(parts.len() >= 2);
        assert!(
            parts
                .iter()
                .all(|p| p.chars().count() <= MAX_CHARS && !p.is_empty())
        );
        assert_eq!(
            parts.join(" ").split_whitespace().count(),
            long.split_whitespace().count()
        );
        assert_eq!(pieces("Short one."), ["Short one."]);
        assert!(pieces("  ").is_empty());
    }
}
