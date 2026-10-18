use vults_dictation::text::{Kind, Rules};

fn rules() -> Rules {
    Rules {
        words: vec!["Vults".into(), "Codex".into(), "Claude Code".into()],
        replacements: vec![
            ("cloud code".into(), "Claude Code".into()),
            ("c\u{f3}dex".into(), "Codex".into()),
        ],
        snippets: vec![("my email".into(), "ana@example.com".into())],
    }
}

#[test]
fn whole_words_case_and_separators_cover_the_text_contract() {
    for (heard, expected) in [
        ("", ""),
        ("unchanged words.", "unchanged words."),
        ("vults", "Vults"),
        ("Use VULTS, please.", "Use Vults, please."),
        ("vults123 prevults vults", "vults123 prevults Vults"),
        ("claude-code", "Claude Code"),
        ("cloud_code", "Claude Code"),
        ("cloud/code", "Claude Code"),
        ("cloud...code", "Claude Code"),
        ("cloud  code", "Claude Code"),
        ("my email", "ana@example.com"),
        ("MY-EMAIL and my_email", "ana@example.com and ana@example.com"),
        ("c\u{f3}dex", "Codex"),
        ("C\u{d3}DEX!", "Codex!"),
        ("  vults  ", "  Vults  "),
        ("prefixcloud code suffix", "prefixcloud code suffix"),
        ("cloud codex", "cloud Codex"),
    ] {
        let out = rules().apply(heard, false).unwrap();
        assert_eq!(out.text(), expected, "{heard:?}");
        assert_eq!(out.words_changed, heard != expected);
        assert!(!out.code_changed);
    }
}

#[test]
fn snippets_are_opaque_even_to_later_snippets_spelling_and_spoken_code() {
    let mut rules = rules();
    rules.snippets = vec![
        (
            "signature".into(),
            "  cloud code\nsrc slash main dot rs $1 ⟦0⟧  ".into(),
        ),
        (
            "cloud code".into(),
            "do not expand inside the first snippet".into(),
        ),
    ];
    let out = rules
        .apply("signature, cloud code, ⟦0⟧, src slash main dot rs", true)
        .unwrap();
    assert_eq!(
        out.text(),
        "  cloud code\nsrc slash main dot rs $1 ⟦0⟧  , do not expand inside the first snippet, ⟦0⟧, src/main.rs"
    );
    let first = &out.segments[0];
    assert_eq!(first.change.as_ref().unwrap().kind, Kind::Snippet);
    assert_eq!(first.change.as_ref().unwrap().heard, "signature");
    assert!(out.code_changed && out.words_changed);
    let identity = Rules {
        snippets: vec![("keep".into(), "keep".into())],
        replacements: vec![("keep".into(), "changed".into())],
        ..Default::default()
    };
    assert_eq!(identity.apply("keep", false).unwrap().text(), "keep");
}

#[test]
fn replacements_are_literal_ordered_and_can_match_across_previous_edit_boundaries() {
    let rules = Rules {
        replacements: vec![
            ("alpha".into(), "beta".into()),
            ("beta gamma".into(), "delta".into()),
            ("delta".into(), "$1 \\ ${name} (.*)".into()),
        ],
        ..Default::default()
    };
    assert_eq!(
        rules.apply("alpha gamma", false).unwrap().text(),
        "$1 \\ ${name} (.*)"
    );
    let backwards = Rules {
        replacements: rules.replacements.into_iter().rev().collect(),
        ..Default::default()
    };
    assert_eq!(
        backwards.apply("alpha gamma", false).unwrap().text(),
        "beta gamma"
    );
    let literal = Rules {
        replacements: vec![("c++".into(), "C++ $1".into()), ("(test)".into(), "[ok]".into())],
        ..Default::default()
    };
    assert_eq!(
        literal.apply("c++, (test)", false).unwrap().text(),
        "C++ $1, [ok]"
    );
}

#[test]
fn dictionary_only_accepts_plain_words_and_keeps_combining_marks_inside_boundaries() {
    let rules = Rules {
        words: vec![
            "A".into(),
            "x!".into(),
            "C++".into(),
            "a_b".into(),
            "foo.bar".into(),
            "D'Artagnan".into(),
            "API".into(),
            "cafe".into(),
            "Stra\u{df}e".into(),
            "\u{3a3}".repeat(2),
            "中文".into(),
        ],
        ..Default::default()
    };
    assert_eq!(
        rules
            .apply(
                "a X! c++ A_B FOO.BAR d artagnan api cafe\u{301} STRASSE 中文",
                false
            )
            .unwrap()
            .text(),
        "a X! c++ A_B FOO.BAR D'Artagnan API cafe\u{301} Stra\u{df}e 中文"
    );
    assert_eq!(
        rules.apply("aPi", false).unwrap().segments[0]
            .change
            .as_ref()
            .unwrap()
            .kind,
        Kind::Spelling
    );
    assert!(rules.apply("API", false).unwrap().segments[0].change.is_none());
}

#[test]
fn controls_are_barriers_and_never_disappear_from_a_match() {
    for barrier in ['\n', '\r', '\t', '\0', '\u{1b}'] {
        let text = format!("cloud{barrier}code {barrier} vults");
        assert_eq!(
            rules().apply(&text, false).unwrap().text(),
            format!("cloud{barrier}code {barrier} Vults")
        );
    }
    assert_eq!(
        rules().apply("cloud\u{a0}code", false).unwrap().text(),
        "Claude Code"
    );
    assert_eq!(rules().apply("cloud\ncode", true).unwrap().text(), "cloud\ncode");
}

#[test]
fn output_growth_is_bounded_and_pure_input_is_not_modified() {
    let rules = Rules {
        replacements: vec![("x".into(), "y ".repeat(65_536)), ("y".into(), "z ".repeat(128))],
        ..Default::default()
    };
    assert!(rules.apply("x", false).is_err());
    assert!(
        Rules::default()
            .apply(&"x".repeat(256 * 1024 + 1), false)
            .is_err()
    );
    assert_eq!(Rules::default().apply("⟦0⟧", false).unwrap().text(), "⟦0⟧");
}

#[test]
fn spoken_code_preserves_the_other_change_annotations() {
    let rules = Rules {
        replacements: vec![("source".into(), "src".into())],
        ..Default::default()
    };
    let out = rules.apply("source slash main", true).unwrap();
    assert_eq!(out.text(), "src/main");
    assert_eq!(out.segments[0].text, "src");
    assert_eq!(out.segments[0].change.as_ref().unwrap().kind, Kind::Replacement);
    assert_eq!(out.segments[0].change.as_ref().unwrap().heard, "source");
    assert_eq!(out.segments[1].text, "/");
    assert_eq!(out.segments[1].change.as_ref().unwrap().kind, Kind::SpokenCode);
    assert!(out.segments[2].change.is_none());
}
