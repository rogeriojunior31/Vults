//! Local transcript filtering, shared by preview and final decoding. No model or IO.

const RUN: usize = 6;
const UNIT: usize = 60;

pub(crate) fn measured(text: &str, cut_repeats: bool) -> crate::Transcript {
    let mut loops_cut = 0;
    let mut text = text.to_owned();
    // Only known sound annotations: brackets also belong to code and spoken explanations.
    for marker in [
        "[BLANK_AUDIO]",
        "[blank_audio]",
        "[silence]",
        "[SILENCE]",
        "[MUSIC]",
        "[music]",
        "[Music]",
        "[MUSIC PLAYING]",
        "(music)",
        "(Music)",
        "[laughs]",
        "(laughs)",
        "[applause]",
        "(applause)",
        "♪",
        "♫",
    ] {
        text = text.replace(marker, " ");
    }
    if cut_repeats {
        let words: Vec<&str> = text.split_whitespace().collect();
        let keys: Vec<String> = words
            .iter()
            .map(|word| {
                word.trim_matches(['.', ',', '!', '?', ':', ';', '"', '\'', '“', '”', '‘', '’'])
                    .to_lowercase()
            })
            .collect();
        let mut kept = Vec::new();
        let mut at = 0;
        while at < words.len() {
            let mut consumed = 1;
            let mut width = 1;
            let mut chars = 0;
            for len in 1..=(words.len() - at) / RUN {
                chars += words[at + len - 1].chars().count() + usize::from(len > 1);
                if chars > UNIT || keys[at + len - 1].is_empty() {
                    break;
                }
                let repeats = repeats(&keys, at, len);
                if repeats >= RUN {
                    width = len;
                    consumed = len * repeats;
                    loops_cut += 1;
                    break;
                }
            }
            kept.extend_from_slice(&words[at..at + width]);
            at += consumed;
        }
        let chars: Vec<char> = kept.join(" ").chars().collect();
        let mut collapsed = String::new();
        let mut at = 0;
        while at < chars.len() {
            let mut width = 1;
            let mut consumed = 1;
            for len in 2..=UNIT.min((chars.len() - at) / RUN) {
                let unit = &chars[at..at + len];
                // Preserve elongated letters, numbers and punctuation used in technical speech.
                if !unit.iter().any(|c| c.is_alphabetic()) || unit.iter().all(|c| *c == unit[0]) {
                    continue;
                }
                let repeats = repeats(&chars, at, len);
                if repeats >= RUN {
                    width = len;
                    consumed = len * repeats;
                    loops_cut += 1;
                    break;
                }
            }
            collapsed.extend(&chars[at..at + width]);
            at += consumed;
        }
        text = collapsed;
    }
    let mut text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    for punctuation in ['.', ',', '?', '!'] {
        text = text.replace(&format!(" {punctuation}"), &punctuation.to_string());
    }
    let key = text
        .trim_matches(|c: char| c.is_whitespace() || matches!(c, '.' | ',' | '!' | '?' | '。' | '！' | '？'))
        .to_lowercase();
    let text = if !text.chars().any(char::is_alphanumeric)
        || matches!(
            key.as_str(),
            "thank you"
                | "thanks for watching"
                | "thank you for watching"
                | "obrigado"
                | "obrigada"
                | "obrigado por assistir"
                | "obrigada por assistir"
                | "gracias"
                | "gracias por ver"
                | "gracias por mirar"
                | "谢谢"
                | "谢谢观看"
                | "感谢观看"
        ) {
        String::new()
    } else {
        text
    };
    crate::Transcript { text, loops_cut }
}

#[cfg(test)]
fn filter(text: &str, cut_repeats: bool) -> String {
    measured(text, cut_repeats).text
}

fn repeats<T: PartialEq>(items: &[T], at: usize, len: usize) -> usize {
    let unit = &items[at..at + len];
    1 + items[at + len..]
        .chunks_exact(len)
        .take_while(|next| *next == unit)
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_repetitive_runs_without_counting_other_cleanup() {
        assert_eq!(
            measured("go go go go go go and stop stop stop stop stop stop", true).loops_cut,
            2
        );
        assert_eq!(measured("你好你好你好你好你好你好", true).loops_cut, 1);
        assert_eq!(measured("[music] Thank you.", true).loops_cut, 0);
        assert_eq!(measured("go go go go go go", false).loops_cut, 0);
    }

    #[test]
    fn disabling_loop_removal_preserves_repetitions_but_still_cleans_silence() {
        for input in [
            "go back go back go back go back go back go back",
            "你好你好你好你好你好你好",
        ] {
            assert_eq!(filter(input, false), input);
            assert_ne!(filter(input, true), input);
        }
        for enabled in [false, true] {
            assert_eq!(filter("[music] Thank you.", enabled), "");
            assert_eq!(filter("[BLANK_AUDIO]", enabled), "");
            assert_eq!(filter("Hello (music) there [laughs].", enabled), "Hello there.");
            assert_eq!(
                filter("Thank you for fixing the build.", enabled),
                "Thank you for fixing the build."
            );
        }
    }

    #[test]
    fn loops_are_collapsed_with_unicode_and_bounded_units() {
        for line in include_str!("../tests/fixtures/transcript-loops.tsv").lines() {
            let (input, expected) = line.split_once('\t').expect("input and expected transcript");
            assert_eq!(filter(input, true), expected, "{input}");
            assert_eq!(filter(expected, true), expected, "filter is idempotent");
        }
        let reported = include_str!("../tests/fixtures/reported-music-loop.txt").trim();
        assert_eq!(
            filter(reported, true),
            reported,
            "reported loops below six remain"
        );
        for len in [60, 61] {
            let unit: String = (0..len).map(|i| char::from_u32(0x4e00 + i).unwrap()).collect();
            let input = unit.repeat(RUN);
            assert_eq!(filter(&input, true), if len == 60 { unit } else { input });
        }
        assert_eq!(filter(&"repeat ".repeat(20_000), true), "repeat");
    }

    #[test]
    fn silence_phrases_match_only_the_whole_result() {
        for input in [
            "Thank you.",
            " THANK YOU! ",
            "Obrigado.",
            "Obrigada!",
            "Gracias.",
            "Thanks for watching.",
            "[BLANK_AUDIO]",
            "♪ ♫",
            "...",
            "谢谢。",
            "感谢观看！",
            "[music] Thank you.",
            "Thank you. Thank you. Thank you. Thank you. Thank you. Thank you.",
        ] {
            assert_eq!(filter(input, true), "", "{input}");
        }
        for input in [
            "Thank you for fixing the build.",
            "Say Obrigado in Portuguese.",
            "Gracias por revisar el cambio.",
            "谢谢你修复构建。",
            "The thanks for watching label is wrong.",
        ] {
            assert_eq!(filter(input, true), input);
        }
    }

    #[test]
    fn ordinary_repetitions_and_technical_syntax_survive() {
        for input in [
            "no, no, no, no, no",
            "go back go back go back go back go back",
            "noooooooo",
            "Use 1000000 and 0x000000.",
            "Keep ====== and ------.",
            "Call foo(bar) and values[index].",
            "An unmatched (parenthesis is still text.",
            "C C++ C C++ C C++",
            "Use https://example.com/a_(b).",
        ] {
            assert_eq!(filter(input, true), input);
        }
        assert_eq!(filter("Hello (music) there [laughs].", true), "Hello there.");
        assert_eq!(filter("  Keep\nthese\twords . ", true), "Keep these words.");
    }
}
