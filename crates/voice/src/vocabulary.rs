//! Per-recording Whisper context. The tokenizer measures the complete seed plus whole entries.

use std::collections::HashSet;

pub const TOKENS: usize = 224;

pub(crate) fn build<T>(
    seed: Option<&str>,
    words: &[String],
    mut tokenize: impl FnMut(&str) -> Result<Vec<T>, String>,
) -> Result<Option<Vec<T>>, String> {
    let mut text = seed.unwrap_or_default().to_string();
    let mut tokens = tokenize(&text)?;
    if tokens.len() > TOKENS {
        return Err("The voice vocabulary seed exceeds its token limit.".into());
    }
    let mut seen = HashSet::new();
    let mut added = false;
    for word in words {
        let word = word.trim();
        if word.is_empty()
            || word.len() > 1024
            || word.chars().any(char::is_control)
            || !seen.insert(word.to_lowercase().to_uppercase())
        {
            continue;
        }
        let candidate = if text.is_empty() {
            word.to_string()
        } else {
            format!("{text}{}{word}", if added { ", " } else { " " })
        };
        let next = tokenize(&candidate)?;
        if next.len() > TOKENS {
            break;
        }
        text = candidate;
        tokens = next;
        added = true;
    }
    Ok(added.then_some(tokens))
}

/// Only measured model/language pairs may receive dictionary context in production.
pub fn enabled(model: &str, language: Option<&str>) -> bool {
    // The offline 40-sentence/silence comparison is recorded with the fixtures.
    model == "turbo" && language == Some("en")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_measured_pair_is_enabled() {
        for model in ["base", "small", "turbo", "unknown"] {
            for language in [None, Some("pt"), Some("en"), Some("es"), Some("auto")] {
                assert_eq!(
                    enabled(model, language),
                    model == "turbo" && language == Some("en")
                );
            }
        }
    }

    #[test]
    fn seed_and_whole_entries_share_one_budget_without_controls_or_duplicates() {
        let tokenize = |text: &str| Ok(text.chars().collect::<Vec<_>>());
        let words = [" Vults ", "VULTS", "bad\nentry", "", "Rust"].map(str::to_owned);
        let actual = build(Some("Coding"), &words, tokenize)
            .unwrap()
            .unwrap()
            .into_iter()
            .collect::<String>();
        assert_eq!(actual, "Coding Vults, Rust");
        let seed = "x".repeat(TOKENS - 6);
        let actual = build(Some(&seed), &["Vults".into(), "More".into()], tokenize)
            .unwrap()
            .unwrap();
        assert_eq!(actual.len(), TOKENS);
        assert!(actual.into_iter().collect::<String>().ends_with(" Vults"));
        assert!(build(Some("Coding"), &[], tokenize).unwrap().is_none());
        assert_eq!(
            build(None, &["Zeca".into()], tokenize)
                .unwrap()
                .unwrap()
                .into_iter()
                .collect::<String>(),
            "Zeca"
        );
        assert!(
            build(None, &["bad\0entry".into(), "x".repeat(1025)], tokenize)
                .unwrap()
                .is_none()
        );
        assert_eq!(
            build(None, &["Stra\u{df}e".into(), "STRA\u{1e9e}E".into()], tokenize)
                .unwrap()
                .unwrap()
                .into_iter()
                .collect::<String>(),
            "Stra\u{df}e"
        );
        assert!(build(Some(&"x".repeat(TOKENS + 1)), &[], tokenize).is_err());
        assert!(build::<char>(None, &["Vults".into()], |_| Err("Tokenizer failed".into())).is_err());
    }
}

#[cfg(test)]
mod evaluation;
