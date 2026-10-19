use vults_dictation::{
    cleanup::{Language, Style},
    styles::*,
};

#[test]
fn identities_are_exact_and_share_only_reviewed_desktop_aliases() {
    for (class, expected) in [
        (" org.kde.konsole.desktop ", "konsole"),
        ("Code", "code"),
        ("org.mozilla.Thunderbird", "thunderbird"),
        ("firefox", "firefox"),
        ("org.example.Thunderbird", "org.example.thunderbird"),
    ] {
        assert_eq!(app_id(class).as_deref(), Some(expected));
    }
    for invalid in ["", " ", "mail\nchat", "bad\0app", &"x".repeat(1025)] {
        assert_eq!(app_id(invalid), None);
    }
}

#[test]
fn resolution_table_obeys_opt_in_categories_overrides_session_and_unknown_context() {
    let prefs = Preferences {
        default: Style::Prompt,
        write_in: Some(Language::Spanish),
        sessions_english: true,
        ..Default::default()
    };
    for (class, terminal, session, style, language) in [
        (Some("org.kde.konsole"), false, false, Style::Verbatim, None),
        (Some("future-terminal"), true, false, Style::Verbatim, None),
        (Some("Code.desktop"), false, false, Style::Verbatim, None),
        (Some("md.obsidian.Obsidian"), false, false, Style::Verbatim, None),
        (
            Some("org.mozilla.Thunderbird"),
            false,
            false,
            Style::Polished,
            Some(Language::Spanish),
        ),
        (
            Some("org.gnome.Evolution"),
            false,
            false,
            Style::Polished,
            Some(Language::Spanish),
        ),
        (
            Some("Slack"),
            false,
            false,
            Style::Natural,
            Some(Language::Spanish),
        ),
        (
            Some("org.signal.Signal"),
            false,
            false,
            Style::Natural,
            Some(Language::Spanish),
        ),
        (
            Some("Firefox"),
            false,
            false,
            Style::Prompt,
            Some(Language::Spanish),
        ),
        (
            Some("my-new-app"),
            false,
            false,
            Style::Prompt,
            Some(Language::Spanish),
        ),
        (
            Some("Konsole"),
            true,
            true,
            Style::Prompt,
            Some(Language::English),
        ),
        (None, false, true, Style::Verbatim, None),
        (Some(""), false, false, Style::Verbatim, None),
    ] {
        assert_eq!(
            resolve(true, class, terminal, session, &prefs),
            Resolved {
                style,
                write_in: language
            },
            "{class:?}"
        );
        assert_eq!(
            resolve(false, class, terminal, session, &prefs),
            Resolved {
                style: Style::Verbatim,
                write_in: None
            }
        );
    }
    let mut changed = prefs.clone();
    changed.apps.insert("konsole".into(), Style::Polished);
    changed.apps.insert("thunderbird".into(), Style::Verbatim);
    assert_eq!(
        resolve(true, Some("org.kde.konsole"), true, false, &changed).style,
        Style::Polished
    );
    assert_eq!(
        resolve(true, Some("thunderbird"), false, false, &changed),
        Resolved {
            style: Style::Verbatim,
            write_in: None
        }
    );
    assert_eq!(
        resolve(true, Some("konsole"), true, true, &changed).style,
        Style::Prompt
    );
    changed.sessions_english = false;
    assert_eq!(
        resolve(true, Some("konsole"), true, true, &changed).write_in,
        Some(Language::Spanish)
    );
    changed.write_in = None;
    assert_eq!(
        resolve(true, Some("konsole"), true, true, &changed).write_in,
        None
    );
    assert_eq!(
        resolve(true, Some("firefox"), false, false, &Preferences::default()),
        Resolved {
            style: Style::Natural,
            write_in: None
        }
    );
}

#[test]
fn a_captured_resolution_cannot_change_with_later_preferences_or_focus() {
    let mut prefs = Preferences::default();
    let captured = resolve(true, Some("Thunderbird"), false, false, &prefs);
    prefs.apps.insert("thunderbird".into(), Style::Prompt);
    prefs.write_in = Some(Language::English);
    assert_eq!(
        captured,
        Resolved {
            style: Style::Polished,
            write_in: None
        }
    );
    assert_ne!(captured, resolve(true, Some("Thunderbird"), false, false, &prefs));
    assert_eq!(
        resolve(true, Some("Konsole"), true, false, &prefs).style,
        Style::Verbatim
    );
    for style in [Style::Verbatim, Style::Natural, Style::Polished, Style::Prompt] {
        assert_eq!(Style::parse(style.id()), Some(style));
    }
    assert_eq!(Style::parse("execute"), None);
}
