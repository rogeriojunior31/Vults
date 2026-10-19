//! Pure text-filter prompts and conservative reply checks. No model, IO, tools or execution.
mod prompt;

use crate::text::{Kind, Processed};
use std::{collections::BTreeMap, ops::Range};

pub const MAX_INPUT_BYTES: usize = 256 * 1024;
pub const MAX_REPLY_BYTES: usize = 1024 * 1024;
pub const MAX_SPELLINGS: usize = 200;
const MAX_PROTECTED: usize = 4096;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Style {
    #[default]
    Verbatim,
    Natural,
    Polished,
    Prompt,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Language {
    #[serde(rename = "en")]
    English,
    #[serde(rename = "pt-BR")]
    BrazilianPortuguese,
    #[serde(rename = "es")]
    Spanish,
    #[serde(rename = "zh")]
    SimplifiedChinese,
}
impl Style {
    pub fn id(self) -> &'static str {
        match self {
            Self::Verbatim => "verbatim",
            Self::Natural => "natural",
            Self::Polished => "polished",
            Self::Prompt => "prompt",
        }
    }
    pub fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "verbatim" => Self::Verbatim,
            "natural" => Self::Natural,
            "polished" => Self::Polished,
            "prompt" => Self::Prompt,
            _ => return None,
        })
    }
}
impl Language {
    pub fn code(self) -> &'static str {
        match self {
            Self::English => "en",
            Self::BrazilianPortuguese => "pt-BR",
            Self::Spanish => "es",
            Self::SimplifiedChinese => "zh",
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::English => "English",
            Self::BrazilianPortuguese => "Brazilian Portuguese",
            Self::Spanish => "Spanish",
            Self::SimplifiedChinese => "Simplified Chinese",
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Rules {
    pub hesitations: bool,
    pub self_corrections: bool,
    pub technical_terms: bool,
}
impl Default for Rules {
    fn default() -> Self {
        Self {
            hesitations: true,
            self_corrections: true,
            technical_terms: true,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Options {
    pub style: Style,
    pub rules: Rules,
    pub app: String,
    pub spellings: Vec<String>,
    pub write_in: Option<Language>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    System,
    User,
    Assistant,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Message {
    pub role: Role,
    pub text: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BuildError {
    TooLarge,
    Controls,
    Metadata,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rejection {
    Empty,
    TooLarge,
    Controls,
    Marks,
    Length,
    Question,
}

#[derive(Debug)]
pub struct Request {
    original: String,
    masked: String,
    protected: BTreeMap<String, String>,
    messages: Vec<Message>,
    style: Style,
    translated: bool,
}

fn controls(text: &str) -> bool {
    text.chars().any(|c| c.is_control() && !matches!(c, '\n' | '\t'))
}

/// Marker-shaped literal text also participates in validation; only generated marks are restored.
fn marks(text: &str) -> Vec<Range<usize>> {
    let mut out = Vec::new();
    let mut start = None;
    for (at, c) in text.char_indices() {
        match c {
            '⟦' => start = Some(at),
            '⟧' => {
                if let Some(start) = start.take() {
                    out.push(start..at + c.len_utf8());
                }
            }
            _ => {}
        }
    }
    out
}

impl Request {
    /// Verbatim and empty input never need a provider. Protected local edits stay opaque.
    pub fn build(input: &Processed, options: &Options) -> Result<Option<Self>, BuildError> {
        if options.style == Style::Verbatim {
            return Ok(None);
        }
        let bytes = input
            .segments
            .iter()
            .try_fold(0usize, |n, segment| n.checked_add(segment.text.len()));
        if bytes.is_none_or(|n| n > MAX_INPUT_BYTES) {
            return Err(BuildError::TooLarge);
        }
        let original = input.text();
        if original.trim().is_empty() {
            return Ok(None);
        }
        if controls(&original) {
            return Err(BuildError::Controls);
        }
        if options.app.len() > 1024 || options.app.chars().any(char::is_control) {
            return Err(BuildError::Metadata);
        }
        let mut masked = String::new();
        let mut protected = BTreeMap::new();
        let mut next = 0;
        for segment in &input.segments {
            if segment.change.as_ref().is_some_and(|c| {
                matches!(
                    c.kind,
                    Kind::Snippet | Kind::Replacement | Kind::Spelling | Kind::SpokenCode
                )
            }) {
                if protected.len() == MAX_PROTECTED {
                    return Err(BuildError::TooLarge);
                }
                // Literal marker text can never collide with a generated identity.
                let mark = loop {
                    let candidate = format!("⟦{next}⟧");
                    next += 1;
                    if !original.contains(&candidate) {
                        break candidate;
                    }
                };
                masked.push_str(&mark);
                protected.insert(mark, segment.text.clone());
            } else {
                masked.push_str(&segment.text);
            }
        }
        let messages = prompt::messages(&masked, options);
        Ok(Some(Self {
            original,
            masked,
            protected,
            messages,
            style: options.style,
            translated: options.write_in.is_some(),
        }))
    }

    pub fn messages(&self) -> &[Message] {
        &self.messages
    }
    pub fn original(&self) -> &str {
        &self.original
    }

    /// Invalid output is an error so the orchestrator can retain `original()` without guessing.
    pub fn validate(&self, reply: &str) -> Result<String, Rejection> {
        if reply.len() > MAX_REPLY_BYTES {
            return Err(Rejection::TooLarge);
        }
        if reply.trim().is_empty() {
            return Err(Rejection::Empty);
        }
        if controls(reply) {
            return Err(Rejection::Controls);
        }
        let reply_marks = marks(reply);
        if marks(&self.masked)
            .iter()
            .map(|r| &self.masked[r.clone()])
            .ne(reply_marks.iter().map(|r| &reply[r.clone()]))
        {
            return Err(Rejection::Marks);
        }
        let input_len = self.masked.chars().count();
        let reply_len = reply.chars().count();
        let max = if self.style == Style::Prompt {
            input_len * 8 + 600
        } else {
            input_len * 4 + 200
        };
        if reply_len * 100 < input_len * 15 || reply_len > max {
            return Err(Rejection::Length);
        }
        if self.style != Style::Prompt && !self.translated {
            let input_words = crate::word_count(&self.masked);
            let reply_words = crate::word_count(reply);
            if reply_words * 100 < input_words * 35 || reply_words * 10 > input_words * 14 + 40 {
                return Err(Rejection::Length);
            }
        }
        let question_count = |text: &str| text.chars().filter(|c| matches!(c, '?' | '？' | '؟')).count();
        if question_count(reply) < question_count(&self.masked) {
            return Err(Rejection::Question);
        }
        let mut result = String::new();
        let mut at = 0;
        for range in reply_marks {
            result.push_str(&reply[at..range.start]);
            let marker = &reply[range.clone()];
            result.push_str(self.protected.get(marker).map_or(marker, String::as_str));
            at = range.end;
        }
        result.push_str(&reply[at..]);
        if result.len() > MAX_REPLY_BYTES {
            return Err(Rejection::TooLarge);
        }
        Ok(result)
    }
}
