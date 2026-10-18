use vults_dictation::word_count;

#[test]
fn words_follow_unicode_boundaries_without_counting_punctuation_or_emoji() {
    for (text, count) in [
        ("", 0),
        (" \n\t", 0),
        ("... 🙂 —", 0),
        ("can't", 1),
        ("alpha\u{a0}beta\ngamma", 3),
        ("na\u{ef}ve cafe\u{301}", 2),
        ("你好世界", 4),
        ("hello世界 foo_bar 32.3", 5),
    ] {
        assert_eq!(word_count(text), count, "{text:?}");
    }
}
