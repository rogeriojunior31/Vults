#![allow(clippy::unwrap_used)]
use super::*;

#[test]
fn duplicates_use_unicode_case_keys_and_upserts_keep_replacement_order() {
    let dir = crate::temp("words-duplicates");
    let store = Store::open(&dir).unwrap();
    store.dictionary_word_add("  Vults  ").unwrap();
    assert!(matches!(
        store.dictionary_word_add("VULTS"),
        Err(Error::Invalid(_))
    ));
    for (word, duplicate) in [
        ("C\u{f3}dex", "C\u{d3}DEX"),
        ("Stra\u{df}e", "STRA\u{1e9e}E"),
        ("\u{3c3}", "\u{3c2}"),
    ] {
        store.dictionary_word_add(word).unwrap();
        assert!(matches!(
            store.dictionary_word_add(duplicate),
            Err(Error::Invalid(_))
        ));
    }
    let first = store
        .dictionary_replacement_set("cloud code", "Claude Code")
        .unwrap();
    let second = store
        .dictionary_replacement_set("second phrase", "second result")
        .unwrap();
    assert_eq!(
        store
            .dictionary_replacement_set(" CLOUD CODE ", "New spelling")
            .unwrap(),
        first
    );
    let literal = "  $1 \\ ${name} 'quoted'\nnext\tline  ";
    let snippet = store.snippet_add(" Signature ", literal).unwrap();
    assert!(matches!(
        store.snippet_add("SIGNATURE", "must not replace"),
        Err(Error::Invalid(_))
    ));
    drop(store);
    let saved = Store::open(&dir).unwrap().dictation_words().unwrap();
    assert!(saved.words.contains(&"Vults".into()));
    assert_eq!(
        saved.replacements.iter().map(|r| r.id).collect::<Vec<_>>(),
        [first, second]
    );
    assert_eq!(saved.replacements[0].from, "CLOUD CODE");
    assert_eq!(saved.replacements[0].to, "New spelling");
    assert_eq!(
        saved.snippets,
        [Snippet {
            id: snippet,
            trigger: "Signature".into(),
            text: literal.into()
        }]
    );
}

#[test]
fn upgrading_history_schema_keeps_history_and_clearing_history_keeps_your_words() {
    let dir = crate::temp("words-upgrade");
    std::fs::create_dir_all(&dir).unwrap();
    let conn = rusqlite::Connection::open(crate::path(&dir)).unwrap();
    for sql in &crate::MIGRATIONS[..21] {
        conn.execute_batch(sql).unwrap();
    }
    conn.execute("INSERT INTO dictations (at, heard, text, style, secs, words, outcome) VALUES (1, 'heard', 'written', 'verbatim', 1, 1, 'copied')", []).unwrap();
    conn.pragma_update(None, "user_version", 21).unwrap();
    drop(conn);
    let mut store = Store::open(&dir).unwrap();
    assert_eq!(store.schema_version().unwrap(), crate::MIGRATIONS.len() as i64);
    assert_eq!(store.dictations(500).unwrap()[0].1.text, "written");
    store.dictionary_word_add("Vults").unwrap();
    store
        .dictionary_replacement_set("cloud code", "Claude Code")
        .unwrap();
    store.snippet_add("signature", "Regards,\nAda").unwrap();
    let saved = store.dictation_words().unwrap();
    store.clear_all(&dir).unwrap();
    assert!(store.dictations(500).unwrap().is_empty());
    assert_eq!(store.dictation_words().unwrap(), saved);
    drop(store);
    assert_eq!(Store::open(&dir).unwrap().dictation_words().unwrap(), saved);
}

#[test]
fn delete_is_idempotent_and_old_ids_never_delete_a_later_entry() {
    let dir = crate::temp("words-delete");
    let mut store = Store::open(&dir).unwrap();
    store.dictionary_word_add("Vults").unwrap();
    let replacement = store.dictionary_replacement_set("source", "target").unwrap();
    let snippet = store.snippet_add("trigger", "text").unwrap();
    store.dictionary_word_delete(" VULTS ").unwrap();
    store.dictionary_replacement_delete(replacement).unwrap();
    store.snippet_delete(snippet).unwrap();
    let newer_replacement = store.dictionary_replacement_set("source", "new target").unwrap();
    let newer_snippet = store.snippet_add("trigger", "new text").unwrap();
    assert!(newer_replacement > replacement && newer_snippet > snippet);
    store.dictionary_word_delete("vults").unwrap();
    store.dictionary_replacement_delete(replacement).unwrap();
    store.snippet_delete(snippet).unwrap();
    let saved = store.dictation_words().unwrap();
    assert!(saved.words.is_empty());
    assert_eq!(saved.replacements[0].to, "new target");
    assert_eq!(saved.snippets[0].text, "new text");
}

#[test]
fn invalid_or_oversized_input_never_changes_a_saved_entry() {
    let dir = crate::temp("words-validation");
    let mut store = Store::open(&dir).unwrap();
    store.dictionary_replacement_set("source", "kept").unwrap();
    let before = store.dictation_words().unwrap();
    for bad in ["", "   ", "line\nbreak", "a\0b", &"x".repeat(1025)] {
        assert!(store.dictionary_word_add(bad).is_err());
        assert!(store.dictionary_replacement_set(bad, "text").is_err());
        assert!(store.snippet_add(bad, "text").is_err());
    }
    for bad in ["", " \n\t", "a\0b", &"x".repeat(65_537)] {
        assert!(store.dictionary_replacement_set("source", bad).is_err());
        assert!(store.snippet_add("trigger", bad).is_err());
    }
    assert!(store.dictionary_replacement_delete(0).is_err());
    assert!(store.snippet_delete(-1).is_err());
    assert_eq!(store.dictation_words().unwrap(), before);
    store.dictionary_word_add(&"x".repeat(1024)).unwrap();
    store.snippet_add("trigger", &"x".repeat(65_536)).unwrap();
}

#[test]
fn concurrent_adds_cannot_create_duplicate_words_or_triggers() {
    let dir = crate::temp("words-concurrent");
    drop(Store::open(&dir).unwrap());
    let ready = std::sync::Arc::new(std::sync::Barrier::new(2));
    let writers = ["Vults", "VULTS"].map(|word| {
        let (dir, ready) = (dir.clone(), ready.clone());
        std::thread::spawn(move || {
            let store = Store::open(&dir).unwrap();
            ready.wait();
            (
                store.dictionary_word_add(word),
                store.snippet_add(word, "literal text"),
            )
        })
    });
    let results = writers.map(|writer| writer.join().unwrap());
    assert_eq!(results.iter().filter(|(word, _)| word.is_ok()).count(), 1);
    assert_eq!(results.iter().filter(|(_, snippet)| snippet.is_ok()).count(), 1);
    let saved = Store::open(&dir).unwrap().dictation_words().unwrap();
    assert_eq!((saved.words.len(), saved.snippets.len()), (1, 1));
}
