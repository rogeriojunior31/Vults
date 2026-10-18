//! Literal dictation text rules. No IO or permission authority.

mod words;
pub use words::{Change, Kind, Processed, Rules, Segment, TOO_LARGE};

/// Only whole trailing phrases count; identifiers and longer sentences stay ordinary text.
pub fn cancels_dictation(text: &str) -> bool {
    let normalized: String = text
        .chars()
        .flat_map(char::to_lowercase)
        .filter_map(|c| match c {
            'á' => Some('a'), // check-english:allow
            'é' => Some('e'), // check-english:allow
            '\u{301}' => None,
            _ => Some(c),
        })
        .collect();
    let key = normalized.split_whitespace().collect::<Vec<_>>().join(" ");
    let key = key.trim_end_matches(|c| {
        matches!(
            c,
            '.' | ',' | '!' | '?' | ':' | ';' | '…' | '。' | '！' | '？' | '"' | '\'' | '”' | '’'
        )
    });
    ["cancel", "never mind", "cancela", "deixa pra la", "dejalo"]
        .iter()
        .any(|phrase| {
            key == *phrase
                || key
                    .strip_suffix(phrase)
                    .is_some_and(|prefix| prefix.ends_with(' '))
        })
}

pub fn spoken_code(text: &str) -> String {
    let mut out = String::new();
    let mut copied = 0;
    for (range, symbol) in code_changes(text) {
        out.push_str(&text[copied..range.start]);
        out.push(symbol);
        copied = range.end;
    }
    out.push_str(&text[copied..]);
    out
}

pub(super) fn code_changes(text: &str) -> Vec<(std::ops::Range<usize>, char)> {
    let mut position = 0;
    let tokens: Vec<(usize, &str)> = text
        .split_whitespace()
        .map(|word| {
            let start = position + text[position..].find(word).expect("token belongs to input");
            position = start + word.len();
            (start, word)
        })
        .collect();
    let mut out = Vec::new();
    let mut at = 1;
    while at < tokens.len() {
        let words = &tokens[at..];
        let Some((symbol, width)) = symbol(words) else {
            at += 1;
            continue;
        };
        let Some(&(right, word)) = words.get(width) else {
            break;
        };
        let (left, previous) = tokens[at - 1];
        let left_end = left + previous.len();
        // Preserve line breaks/control bytes so text conversion cannot weaken delivery checks.
        let spaces = text[left_end..tokens[at].0].chars().all(|c| c == ' ')
            && text[words[width - 1].0 + words[width - 1].1.len()..right]
                .chars()
                .all(|c| c == ' ')
            && (width == 1
                || text[words[0].0 + words[0].1.len()..words[1].0]
                    .chars()
                    .all(|c| c == ' '));
        if identifier(previous) && identifier(word) && spaces {
            out.push((left_end..right, symbol));
            at += width + 1;
        } else {
            at += width;
        }
    }
    out
}

fn symbol(words: &[(usize, &str)]) -> Option<(char, usize)> {
    let first = words.first()?.1.to_lowercase();
    let next = words.get(1).map(|(_, word)| word.to_lowercase());
    match (first.as_str(), next.as_deref()) {
        ("dois", Some("pontos")) | ("dos", Some("puntos")) => Some((':', 2)),
        ("guion" | "guión", Some("bajo")) => Some(('_', 2)), // check-english:allow
        ("ponto" | "dot" | "punto", _) => Some(('.', 1)),
        ("barra" | "slash", _) => Some(('/', 1)),
        ("underline" | "underscore", _) => Some(('_', 1)),
        ("traço" | "traco" | "dash" | "guion" | "guión", _) => Some(('-', 1)), // check-english:allow
        ("colon", _) => Some((':', 1)),
        _ => None,
    }
}

fn identifier(word: &str) -> bool {
    let key = word.to_lowercase();
    !matches!(key.as_str(),
        "ponto" | "dot" | "punto" | "barra" | "slash" | "underline" | "underscore" |
        "traço" | "traco" | "dash" | "guion" | "guión" | "dois" | "pontos" | "dos" | "puntos" | "bajo" | "colon") // check-english:allow
        && word.chars().all(|c| c.is_alphanumeric() || matches!(c, '_' | '.' | '/' | '-' | ':'))
        && word.chars().any(|c| c.is_alphanumeric() || c == '_')
}
