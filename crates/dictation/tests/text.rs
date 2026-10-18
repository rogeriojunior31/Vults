use vults_dictation::text::spoken_code;

#[test]
fn spoken_identifiers_cover_english_portuguese_and_spanish() {
    for (heard, written) in [
        ("src slash main dot rs", "src/main.rs"),
        ("user underscore name dash old colon value", "user_name-old:value"),
        ("src barra main ponto rs", "src/main.rs"),
        (
            "user underline name traço old dois pontos value", // check-english:allow
            "user_name-old:value",
        ),
        ("user traco old", "user-old"),
        ("src barra main punto rs", "src/main.rs"),
        (
            "usuario guion bajo nombre guion viejo dos puntos valor",
            "usuario_nombre-viejo:valor",
        ),
        ("usuario guión bajo nombre guión viejo", "usuario_nombre-viejo"), // check-english:allow
        ("src SLASH Main DOT RS", "src/Main.RS"),
        ("3 dot 12", "3.12"),
        ("配置 slash main dot rs", "配置/main.rs"),
        ("open src/main dot rs now", "open src/main.rs now"),
        ("  src  slash main dot rs.  ", "  src/main.rs.  "),
    ] {
        assert_eq!(spoken_code(heard), written, "{heard}");
        assert_eq!(spoken_code(written), written, "Converted text is stable");
    }
}

#[test]
fn punctuation_requires_identifier_neighbors_and_preserves_other_bytes() {
    for text in [
        "",
        "dot",
        "slash src",
        "main dot",
        "src dot slash main",
        "src dois pontos",
        "foo dot --",
        "(foo) dot bar",
        "foo, dot bar",
        "my.dot.file",
        "Hello world.",
        "foo\nslash bar",
        "foo slash\r\nbar",
        "foo\tslash bar",
        "foo dois\npontos bar",
        "foo\0 slash bar",
        "foo slash $HOME",
        "foo slash `whoami`",
        "foo slash $(id)",
    ] {
        assert_eq!(spoken_code(text), text, "{text:?}");
    }
    assert_eq!(
        spoken_code("src slash main\nnext dot file\t"),
        "src/main\nnext.file\t"
    );
}

#[test]
fn voice_cancel_matches_only_whole_trailing_phrases() {
    use vults_dictation::text::cancels_dictation;
    for text in [
        "cancel",
        "CANCEL?!",
        "  never   mind.  ",
        "write this, never mind",
        "cancela",
        "escreva isto, cancela!",
        "deixa pra lá", // check-english:allow
        "deixa pra la",
        "DEIXA PRA LA\u{301}",
        "texto, deixa pra lá.", // check-english:allow
        "déjalo",               // check-english:allow
        "texto, déjalo.",       // check-english:allow
        "DE\u{301}JALO",        // check-english:allow
    ] {
        assert!(cancels_dictation(text), "{text:?}");
    }
    for text in [
        "",
        "...",
        "cancel the meeting at noon",
        "cancela a tarefa amanha",
        "never mind the meeting",
        "cancel now please",
        "uncancel",
        "cancellation",
        "cancelled",
        "mycancel",
        "pre-cancel",
        "path/cancel",
        "file.cancel",
        ".cancel",
        "cancel()",
        "cancel_by_voice",
        "never mindfully",
        "dejalo para manana",
        "\"cancel\"",
        "'cancel'",
        "hello, never mind, then keep this",
    ] {
        assert!(!cancels_dictation(text), "{text:?}");
    }
}
