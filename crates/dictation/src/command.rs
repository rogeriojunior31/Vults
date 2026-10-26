//! Pure voice-command grammar. Names remain unresolved; no action or permission is executed.
mod media;
pub mod names;
pub use media::MediaAction;
mod phrases;

use unicode_normalization::{UnicodeNormalization, char::is_combining_mark};

/// Bound work before Unicode folding or building a batch.
pub const MAX_INPUT_BYTES: usize = 4096;
pub const MAX_COMMANDS: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Preference {
    Mute,
    Pin,
    Hide,
}

/// A description for the later name resolver and dispatcher, never a core Intent or capability.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Media(MediaAction),
    Next,
    Previous,
    Focus(String),
    Terminal(String),
    Folder(String),
    SetPreference {
        target: String,
        preference: Preference,
        on: bool,
    },
    Waiting,
    Session(String),
    Usage,
    PullRequests,
    Repeat(String),
    Undo,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Parsed {
    Commands(Vec<Command>),
    /// A permission/card answer was requested; none of this utterance may run.
    Refused,
    NotCommand,
}

/// Lexical key shared with later name matching: accent/case/punctuation/space folding only.
/// Politeness and verb rewriting belong to clauses, so this never strips words from a name.
pub fn normalize(text: &str) -> String {
    let mut out = String::new();
    let mut separated = false;
    for ch in text.nfd().flat_map(char::to_lowercase) {
        if is_combining_mark(ch) {
            continue;
        }
        if ch.is_alphanumeric() {
            if separated && !out.is_empty() {
                out.push(' ');
            }
            out.push(ch);
            separated = false;
        } else {
            separated = true;
        }
    }
    out
}

/// A batch is returned only when every explicit clause parses; refusal wins over unknown text.
pub fn parse(text: &str) -> Parsed {
    if text.len() > MAX_INPUT_BYTES || text.chars().any(|ch| ch.is_control() && !ch.is_whitespace()) {
        return Parsed::NotCommand;
    }
    let normalized = normalize(text);
    let words: Vec<_> = normalized.split_whitespace().collect();
    if words.is_empty() {
        return Parsed::NotCommand;
    }
    let mut clauses = Vec::new();
    let (mut start, mut at) = (0, 0);
    while at < words.len() {
        if let Some(connector) = phrases::CONNECTORS
            .iter()
            .find(|phrase| words[at..].starts_with(phrase))
        {
            clauses.push(prepare(&words[start..at].join(" ")));
            at += connector.len();
            start = at;
        } else {
            at += 1;
        }
    }
    clauses.push(prepare(&words[start..].join(" ")));
    if clauses.len() > MAX_COMMANDS {
        return Parsed::NotCommand;
    }
    if clauses.iter().any(|clause| refused(clause)) {
        return Parsed::Refused;
    }
    let commands: Option<Vec<_>> = clauses.iter().map(|clause| single(clause)).collect();
    commands.map_or(Parsed::NotCommand, |commands| {
        // Folding punctuation must not turn a signed volume into a different positive value.
        let signed = text.char_indices().any(|(index, ch)| {
            matches!(ch, '-' | '+' | '\u{2212}' | '\u{ff0b}' | '\u{ff0d}')
                && text[index + ch.len_utf8()..]
                    .trim_start()
                    .starts_with(|ch: char| ch.is_ascii_digit())
        });
        if signed
            && commands
                .iter()
                .any(|command| matches!(command, Command::Media(MediaAction::Volume(_))))
        {
            Parsed::NotCommand
        } else {
            Parsed::Commands(commands)
        }
    })
}

fn head<'a>(text: &'a str, phrase: &str) -> Option<&'a str> {
    text.strip_prefix(phrase).and_then(|rest| {
        if rest.is_empty() {
            Some(rest)
        } else {
            rest.strip_prefix(' ')
        }
    })
}
fn tail<'a>(text: &'a str, phrase: &str) -> Option<&'a str> {
    if phrase.is_empty() {
        return Some(text);
    }
    text.strip_suffix(phrase).and_then(|rest| {
        if rest.is_empty() {
            Some(rest)
        } else {
            rest.strip_suffix(' ')
        }
    })
}
fn prepare(text: &str) -> String {
    let mut text = text;
    while let Some(rest) = phrases::LEADING.iter().find_map(|phrase| head(text, phrase)) {
        text = rest;
    }
    while let Some(rest) = phrases::POLITE.iter().find_map(|phrase| tail(text, phrase)) {
        text = rest;
    }
    for (form, imperative) in phrases::VERBS {
        if let Some(rest) = head(text, form) {
            return format!("{imperative} {rest}").trim().to_owned();
        }
    }
    text.into()
}
fn refused(text: &str) -> bool {
    let text = ["always", "sempre", "siempre"]
        .iter()
        .find_map(|word| head(text, word))
        .unwrap_or(text);
    text.split_whitespace()
        .next()
        .is_some_and(|word| phrases::ANSWERS.contains(&word))
}
fn single(text: &str) -> Option<Command> {
    if let Some(action) = media::parse(text) {
        return Some(Command::Media(action));
    }
    if let Some((_, command)) = phrases::EXACT.iter().find(|(phrase, _)| text == *phrase) {
        return Some(command.clone());
    }
    for (prefix, suffix, kind) in phrases::TARGETS {
        if let Some(name) = head(text, prefix).and_then(|rest| tail(rest, suffix)) {
            return target(name).map(|target| kind.command(target));
        }
    }
    None
}
fn target(name: &str) -> Option<String> {
    let mut name = name;
    // The target may have one article and a project/session qualifier.
    for words in [
        &["the", "o", "a", "os", "as", "el", "la", "los", "las"][..],
        &["session", "sessao", "sesion", "project", "projeto", "proyecto"][..],
    ] {
        name = words.iter().find_map(|word| head(name, word)).unwrap_or(name);
    }
    (!name.is_empty()).then(|| name.to_owned())
}

#[derive(Clone, Copy)]
enum Target {
    Focus,
    Terminal,
    Folder,
    Session,
    Repeat,
    Set(Preference, bool),
}
impl Target {
    fn command(self, name: String) -> Command {
        match self {
            Self::Focus => Command::Focus(name),
            Self::Terminal => Command::Terminal(name),
            Self::Folder => Command::Folder(name),
            Self::Session => Command::Session(name),
            Self::Repeat => Command::Repeat(name),
            Self::Set(preference, on) => Command::SetPreference {
                target: name,
                preference,
                on,
            },
        }
    }
}
