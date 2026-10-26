use vults_dictation::command::{Command, Parsed, Preference, normalize, parse};

fn one(command: Command) -> Parsed {
    Parsed::Commands(vec![command])
}

#[test]
fn english_phrases_normalize_politeness_without_executing_or_resolving_names() {
    for (text, command) in [
        ("Uh, could you please OPEN API?!", Command::Open("api".into())),
        ("next session", Command::Next),
        ("previous", Command::Previous),
        ("go to the api terminal", Command::Terminal("api".into())),
        ("open the web folder, please", Command::Folder("web".into())),
        ("focus on the api", Command::Focus("api".into())),
        ("who is waiting on a card?", Command::Waiting),
        ("what is api doing?", Command::Session("api".into())),
        ("how much of my plan is left?", Command::Usage),
        ("how are my pull requests?", Command::PullRequests),
        ("same for web", Command::Repeat("web".into())),
        ("and docs too", Command::Repeat("docs".into())),
        ("undo", Command::Undo),
    ] {
        assert_eq!(parse(text), one(command), "{text}");
    }
}

#[test]
fn portuguese_infinitives_accents_articles_and_fillers_have_the_same_commands() {
    for (text, command) in [
        (
            "Ent\u{e3}o, voc\u{ea} pode, por favor, abrir o API?",
            Command::Open("api".into()),
        ),
        ("pr\u{f3}xima sess\u{e3}o", Command::Next),
        ("sess\u{e3}o anterior", Command::Previous),
        ("ir para o terminal do api", Command::Terminal("api".into())),
        ("abre a pasta do web", Command::Folder("web".into())),
        ("quem est\u{e1} esperando num card?", Command::Waiting),
        ("o que o api est\u{e1} fazendo?", Command::Session("api".into())),
        ("quanto falta do meu plano?", Command::Usage),
        ("como est\u{e3}o meus pull requests?", Command::PullRequests),
        ("o mesmo para o web", Command::Repeat("web".into())),
        ("e docs tamb\u{e9}m", Command::Repeat("docs".into())),
        ("desfazer", Command::Undo),
    ] {
        assert_eq!(parse(text), one(command), "{text}");
    }
}

#[test]
fn spanish_infinitives_punctuation_and_politeness_have_the_same_commands() {
    for (text, command) in [
        (
            "Pues, \u{bf}podr\u{ed}as abrir la sesi\u{f3}n API, por favor?",
            Command::Focus("api".into()),
        ),
        ("siguiente sesi\u{f3}n", Command::Next),
        ("sesi\u{f3}n anterior", Command::Previous),
        ("ir al terminal de api", Command::Terminal("api".into())),
        ("abrir la carpeta de web", Command::Folder("web".into())),
        (
            "\u{bf}qui\u{e9}n est\u{e1} esperando en una tarjeta?",
            Command::Waiting,
        ),
        ("qu\u{e9} hace api?", Command::Session("api".into())),
        ("cu\u{e1}nto queda de mi plan?", Command::Usage),
        ("c\u{f3}mo van mis pull requests?", Command::PullRequests),
        ("lo mismo para web", Command::Repeat("web".into())),
        ("y docs tambi\u{e9}n", Command::Repeat("docs".into())),
        ("deshacer", Command::Undo),
    ] {
        assert_eq!(parse(text), one(command), "{text}");
    }
}

#[test]
fn preference_commands_are_only_unresolved_descriptions() {
    for (phrase, preference, on) in [
        ("mute the web", Preference::Mute, true),
        ("silenciar o web", Preference::Mute, true),
        ("silencia web", Preference::Mute, true),
        ("unmute web", Preference::Mute, false),
        ("reativar o som do web", Preference::Mute, false),
        ("activar el sonido de web", Preference::Mute, false),
        ("pin web", Preference::Pin, true),
        ("fixar o web", Preference::Pin, true),
        ("fijar web", Preference::Pin, true),
        ("unpin web", Preference::Pin, false),
        ("desafixar o web", Preference::Pin, false),
        ("desfijar web", Preference::Pin, false),
        ("hide web", Preference::Hide, true),
        ("ocultar o web", Preference::Hide, true),
        ("oculta web", Preference::Hide, true),
        ("unhide web", Preference::Hide, false),
        ("mostrar o web", Preference::Hide, false),
        ("mostrar web", Preference::Hide, false),
    ] {
        assert_eq!(
            parse(phrase),
            one(Command::SetPreference {
                target: "web".into(),
                preference,
                on
            }),
            "{phrase}"
        );
    }
}

#[test]
fn compound_commands_are_atomic_and_only_split_explicit_whole_connectors() {
    for connector in [
        "and then",
        "e depois",
        "e ent\u{e3}o",
        "y luego",
        "y despu\u{e9}s",
    ] {
        assert_eq!(
            parse(&format!("open api {connector} mute web")),
            Parsed::Commands(vec![
                Command::Open("api".into()),
                Command::SetPreference {
                    target: "web".into(),
                    preference: Preference::Mute,
                    on: true
                }
            ])
        );
        for text in [
            format!("next {connector} write a poem"),
            format!("write a poem {connector} next"),
            format!("next {connector} write a poem {connector} previous"),
            format!("{connector} next"),
            format!("next {connector}"),
        ] {
            assert_eq!(parse(&text), Parsed::NotCommand, "{text}");
        }
    }
    assert_eq!(parse("open andthen"), one(Command::Open("andthen".into())));
    assert_eq!(parse("open research and then development"), Parsed::NotCommand);
    assert_eq!(
        parse("next session and then please undo and then previous"),
        Parsed::Commands(vec![Command::Next, Command::Undo, Command::Previous])
    );
}

#[test]
fn permission_answers_are_refused_in_every_part_without_partial_commands() {
    for answer in [
        "approve",
        "allow it",
        "always approve",
        "deny",
        "yes",
        "no",
        "answer the card",
        "release",
        "aprovar",
        "permite",
        "sim",
        "n\u{e3}o",
        "sempre permitir",
        "negar",
        "responder ao card",
        "aprueba",
        "permitir",
        "s\u{ed}",
        "denegar",
        "siempre aprobar",
        "responde a la tarjeta",
    ] {
        assert_eq!(parse(answer), Parsed::Refused, "{answer}");
        assert_eq!(parse(&format!("please {answer}")), Parsed::Refused);
        assert_eq!(parse(&format!("next and then {answer}")), Parsed::Refused);
        assert_eq!(parse(&format!("{answer} and then next")), Parsed::Refused);
        assert_eq!(
            parse(&format!("next and then {answer} and then previous")),
            Parsed::Refused
        );
        assert_eq!(parse(&format!("write a poem and then {answer}")), Parsed::Refused);
    }
    assert_eq!(parse("open approve"), one(Command::Open("approve".into())));
    assert_eq!(
        parse("mute yes"),
        one(Command::SetPreference {
            target: "yes".into(),
            preference: Preference::Mute,
            on: true
        })
    );
}

#[test]
fn unknown_empty_missing_and_control_bearing_inputs_never_produce_partial_actions() {
    for text in [
        "",
        " \n\t ",
        "!!!",
        "please",
        "open",
        "mute",
        "unpin",
        "what is doing",
        "same for",
        "and too",
        "opening api",
        "next session write a poem",
        "write next session",
        "open api\0",
        "next\u{1b}",
        "go to the terminal",
        "please do not mute web",
        "do not open api",
        "openapi",
    ] {
        assert_eq!(parse(text), Parsed::NotCommand, "{text:?}");
    }
    assert_eq!(parse(&"x".repeat(4097)), Parsed::NotCommand);
    assert_eq!(parse(&["next"; 9].join(" and then ")), Parsed::NotCommand);
    assert_eq!(
        parse(&["next"; 8].join(" and then ")),
        Parsed::Commands(vec![Command::Next; 8])
    );
}

#[test]
fn lexical_normalization_is_idempotent_and_does_not_strip_name_words() {
    for (input, expected) in [
        ("  \u{c1}PI, GATEWAY?!  ", "api gateway"),
        ("Cafe\u{301}", "cafe"),
        ("API\u{a0}WEB", "api web"),
        ("please", "please"),
        ("abrir", "abrir"),
        ("API-gateway/lib.rs", "api gateway lib rs"),
        ("世界", "世界"),
    ] {
        assert_eq!(normalize(input), expected);
        assert_eq!(normalize(&normalize(input)), expected);
    }
    assert_eq!(parse("open please api"), one(Command::Open("please api".into())));
    assert_eq!(parse("open cafe\u{301}"), one(Command::Open("cafe".into())));
}

#[test]
fn media_scope_preserves_negation_card_refusal_and_project_notifications() {
    use vults_dictation::command::MediaAction;
    for (text, action) in [
        ("Pause the music, please.", MediaAction::Pause),
        ("pare a m\u{ed}dia", MediaAction::Pause),
        ("retome a m\u{fa}sica", MediaAction::Resume),
        ("pausa la m\u{fa}sica", MediaAction::Pause),
        ("next track", MediaAction::Next),
        ("faixa anterior", MediaAction::Previous),
        ("volume da m\u{ed}dia 30 por cento", MediaAction::Volume(30)),
        ("volumen de la musica 100", MediaAction::Volume(100)),
        ("abaixe a m\u{fa}sica", MediaAction::Quieter),
        ("raise media volume", MediaAction::Louder),
    ] {
        assert_eq!(parse(text), one(Command::Media(action)), "{text}");
    }
    for text in [
        "media volume 101",
        "media volume 999999",
        "media volume -10",
        "media volume +10",
        "media volume 40.5",
        "media volume \u{2212} 10",
        "do not pause music",
        "what does pause media mean",
    ] {
        assert_eq!(
            parse(text),
            if text.starts_with("media volume") {
                Parsed::InvalidVolume
            } else {
                Parsed::NotCommand
            },
            "{text}"
        );
    }
    assert_eq!(
        parse("mute web"),
        one(Command::SetPreference {
            target: "web".into(),
            preference: Preference::Mute,
            on: true
        })
    );
    assert_eq!(parse("pause media and then approve"), Parsed::Refused);
}

#[test]
fn opening_an_app_or_project_uses_an_explicit_namespace_without_touching_the_name() {
    for (text, expected) in [
        ("open Obsidian", Command::Open("obsidian".into())),
        ("abrir aplicativo Firefox", Command::Application("firefox".into())),
        (
            "abrir la aplicacion Firefox",
            Command::Application("firefox".into()),
        ),
        ("open app Session", Command::Application("session".into())),
        ("open app Project", Command::Application("project".into())),
        ("abrir projeto Firefox", Command::Focus("firefox".into())),
        ("focus Firefox", Command::Focus("firefox".into())),
        ("open a session api", Command::Focus("api".into())),
        ("abrir sess\u{e3}o api", Command::Focus("api".into())),
        ("abrir proyecto api", Command::Focus("api".into())),
        (
            "open the application Firefox",
            Command::Application("firefox".into()),
        ),
    ] {
        assert_eq!(parse(text), one(expected), "{text}");
    }
    assert_eq!(parse("open app"), Parsed::NotCommand);
    assert_eq!(parse("open app Firefox and then approve"), Parsed::Refused);
}

#[test]
fn computer_audio_has_an_explicit_scope_and_preserves_project_and_player_commands() {
    use vults_dictation::command::{MediaAction, OutputAction};
    for (text, action) in [
        ("mute the computer", OutputAction::Mute(true)),
        ("silenciar o computador", OutputAction::Mute(true)),
        ("reativar o som do computador", OutputAction::Mute(false)),
        ("desilenciar el ordenador", OutputAction::Mute(false)),
        ("volume do computador 40 por cento", OutputAction::Volume(40)),
        ("volumen del ordenador 100 por ciento", OutputAction::Volume(100)),
        ("raise computer volume", OutputAction::Louder),
        ("abaixe o volume do computador", OutputAction::Quieter),
    ] {
        assert_eq!(parse(text), one(Command::Output(action)), "{text}");
    }
    for text in [
        "computer volume -10",
        "computer volume +10",
        "computer volume 40.5",
        "computer volume 101",
        "computer volume 99999",
        "lower the volume",
        "aumente o som",
        "do not mute the computer",
    ] {
        assert_eq!(
            parse(text),
            if text.starts_with("computer volume") {
                Parsed::InvalidVolume
            } else {
                Parsed::NotCommand
            },
            "{text}"
        );
    }
    assert_eq!(parse("pause media"), one(Command::Media(MediaAction::Pause)));
    assert_eq!(
        parse("mute project computer"),
        one(Command::SetPreference {
            target: "computer".into(),
            preference: Preference::Mute,
            on: true
        })
    );
    assert_eq!(parse("mute computer and then approve"), Parsed::Refused);
}

#[test]
fn mixed_audio_examples_keep_their_spoken_order_in_each_supported_grammar() {
    use vults_dictation::command::{MediaAction, OutputAction};
    for text in [
        "pause media and then computer volume 40 and then resume music",
        "pause a midia e depois volume do computador 40 e depois retome a musica",
        "pausa la musica y luego volumen del ordenador 40 y luego reanuda la musica",
    ] {
        assert_eq!(
            parse(text),
            Parsed::Commands(vec![
                Command::Media(MediaAction::Pause),
                Command::Output(OutputAction::Volume(40)),
                Command::Media(MediaAction::Resume),
            ]),
            "{text}"
        );
    }
}
