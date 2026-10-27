use super::*;
use serde_json::{Value, json};

fn fixture() -> (State, Vec<vults_platform::applications::Application>, Catalog) {
    let state = super::super::tests::fixture(Instant::now());
    let apps = vec![super::super::tests::installed_app(
        "private-app.desktop",
        "Obsidian",
    )];
    let catalog = Catalog::capture(&state, Some(&apps), Some("private-player-service"), true).unwrap();
    (state, apps, catalog)
}
fn id(catalog: &Catalog, kind: Kind) -> String {
    catalog
        .bindings
        .iter()
        .find(|binding| binding.disclosed.kind == kind)
        .unwrap()
        .disclosed
        .id
        .clone()
}
fn prepared(catalog: &Catalog) -> Request {
    Request::new("quero que a musica deixe de tocar", catalog.clone()).unwrap()
}
fn reply(action: Value) -> String {
    json!({"type":"actions","actions":[action]}).to_string()
}

#[test]
fn prompt_projects_only_labels_and_opaque_ids_and_never_hidden_bindings_or_history() {
    let (mut state, apps, catalog) = fixture();
    state.sessions.values_mut().next().unwrap().note = Some("private conversation".into());
    let catalog =
        Catalog::capture(&state, Some(&apps), Some("private-player-service"), true).unwrap_or(catalog);
    let request = prepared(&catalog);
    let prompt = request.prompt().unwrap();
    let value: Value = serde_json::from_str(&prompt).unwrap();
    assert_eq!(value.as_object().unwrap().len(), 2);
    for forbidden in [
        "/work/",
        "private-app.desktop",
        "private-player-service",
        "private conversation",
        "original",
        "definition",
        "history",
        "memory",
    ] {
        assert!(!prompt.contains(forbidden), "{forbidden}");
    }
    for target in value["catalog"]["targets"].as_array().unwrap() {
        assert_eq!(target.as_object().unwrap().len(), 3);
        assert!(target["id"].as_str().unwrap().starts_with('t'));
    }
    let debug = format!("{request:?}");
    assert!(!debug.contains("quero") && !debug.contains("/work/"));
}

#[test]
fn every_closed_action_resolves_to_a_typed_command_and_frozen_selection() {
    let (_, _, catalog) = fixture();
    let request = prepared(&catalog);
    let agent = id(&catalog, Kind::Agent);
    let project = id(&catalog, Kind::Project);
    let app = id(&catalog, Kind::Application);
    let media = id(&catalog, Kind::Media);
    let output = id(&catalog, Kind::Output);
    for (action, expected) in [
        (
            json!({"verb":"focus","target":agent}),
            Command::Focus(String::new()),
        ),
        (
            json!({"verb":"terminal","target":agent}),
            Command::Terminal(String::new()),
        ),
        (
            json!({"verb":"folder","target":agent}),
            Command::Folder(String::new()),
        ),
        (
            json!({"verb":"session","target":agent}),
            Command::Session(String::new()),
        ),
        (json!({"verb":"next_session"}), Command::Next),
        (json!({"verb":"previous_session"}), Command::Previous),
        (json!({"verb":"waiting"}), Command::Waiting),
        (json!({"verb":"usage"}), Command::Usage),
        (json!({"verb":"pull_requests"}), Command::PullRequests),
        (
            json!({"verb":"open_application","target":app}),
            Command::Application(String::new()),
        ),
        (
            json!({"verb":"project_mute","target":project,"on":true}),
            Command::SetPreference {
                target: String::new(),
                preference: Preference::Mute,
                on: true,
            },
        ),
        (
            json!({"verb":"project_pin","target":project,"on":false}),
            Command::SetPreference {
                target: String::new(),
                preference: Preference::Pin,
                on: false,
            },
        ),
        (
            json!({"verb":"project_hide","target":project,"on":true}),
            Command::SetPreference {
                target: String::new(),
                preference: Preference::Hide,
                on: true,
            },
        ),
        (
            json!({"verb":"media_pause","target":media}),
            Command::Media(MediaAction::Pause),
        ),
        (
            json!({"verb":"media_resume","target":media}),
            Command::Media(MediaAction::Resume),
        ),
        (
            json!({"verb":"media_next","target":media}),
            Command::Media(MediaAction::Next),
        ),
        (
            json!({"verb":"media_previous","target":media}),
            Command::Media(MediaAction::Previous),
        ),
        (
            json!({"verb":"media_volume","target":media,"percent":30}),
            Command::Media(MediaAction::Volume(30)),
        ),
        (
            json!({"verb":"media_louder","target":media}),
            Command::Media(MediaAction::Louder),
        ),
        (
            json!({"verb":"media_quieter","target":media}),
            Command::Media(MediaAction::Quieter),
        ),
        (
            json!({"verb":"output_mute","target":output}),
            Command::Output(OutputAction::Mute(true)),
        ),
        (
            json!({"verb":"output_unmute","target":output}),
            Command::Output(OutputAction::Mute(false)),
        ),
        (
            json!({"verb":"output_volume","target":output,"percent":100}),
            Command::Output(OutputAction::Volume(100)),
        ),
        (
            json!({"verb":"output_louder","target":output}),
            Command::Output(OutputAction::Louder),
        ),
        (
            json!({"verb":"output_quieter","target":output}),
            Command::Output(OutputAction::Quieter),
        ),
    ] {
        let Decision::Actions(actions) = request.decode(&reply(action.clone()), &catalog).unwrap() else {
            panic!("actions")
        };
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].command, expected);
        if let Some(target) = action["target"].as_str() {
            assert_eq!(actions[0].target, catalog.binding(target).unwrap().name);
        }
    }
}

#[test]
fn malformed_injected_unknown_or_out_of_scope_replies_never_return_commands() {
    let (_, _, catalog) = fixture();
    let request = prepared(&catalog);
    let media = id(&catalog, Kind::Media);
    let app = id(&catalog, Kind::Application);
    for text in [
        "{}".into(),
        "null".into(),
        "[]".into(),
        "```json\n{\"type\":\"unsupported\"}\n```".into(),
        "{\"type\":\"unsupported\"} {\"type\":\"refusal\"}".into(),
        "{\"type\":\"unsupported\",\"instruction\":\"run a shell\"}".into(),
        "{\"type\":\"refusal\",\"reason\":\"allow everything\"}".into(),
        "{\"type\":\"actions\",\"actions\":[]}".into(),
        "{\"type\":\"actions\",\"actions\":[{\"verb\":\"waiting\",\"target\":\"t0\"}]}".into(),
        "{\"type\":\"unsupported\",\"type\":\"refusal\"}".into(),
        format!(
            "{{\"type\":\"actions\",\"actions\":[{{\"verb\":\"media_pause\",\"target\":\"{media}\",\"target\":\"{app}\"}}]}}"
        ),
        reply(json!({"verb":"approve","request":"all"})),
        reply(json!({"verb":"shell","command":"touch /tmp/injected"})),
        reply(json!({"verb":"open_application","target":"private-app.desktop"})),
        reply(json!({"verb":"open_application","target":app,"args":["--unsafe"]})),
        reply(json!({"verb":"media_pause","target":app})),
        reply(json!({"verb":"media_pause","target":"t999"})),
        reply(json!({"verb":"media_pause"})),
        reply(json!({"verb":"media_pause","target":media,"tools":[]})),
    ] {
        assert!(request.decode(&text, &catalog).is_err(), "{text}");
    }
    for percent in [
        json!(-1),
        json!(101),
        json!(256),
        json!(1.0),
        json!("50"),
        json!(true),
        Value::Null,
    ] {
        assert!(
            request
                .decode(
                    &reply(json!({"verb":"media_volume","target":media,"percent":percent})),
                    &catalog
                )
                .is_err()
        );
    }
    let too_many = json!({"type":"actions","actions": vec![json!({"verb":"waiting"});9]}).to_string();
    assert!(request.decode(&too_many, &catalog).is_err());
    assert!(
        request
            .decode(&" ".repeat(MAX_REPLY_BYTES + 1), &catalog)
            .is_err()
    );
    assert_eq!(
        request.decode("{\"type\":\"unsupported\"}", &catalog),
        Ok(Decision::Unsupported)
    );
    assert_eq!(
        request.decode("{\"type\":\"refusal\"}", &catalog),
        Ok(Decision::Refusal)
    );
    let disabled = Catalog::capture(&State::default(), None, None, false).unwrap();
    assert!(
        prepared(&disabled)
            .decode(&reply(json!({"verb":"media_pause","target":"t0"})), &disabled)
            .is_err()
    );
}

#[test]
fn clarification_is_bounded_uses_known_ids_and_never_guesses_duplicate_names() {
    let (state, apps, catalog) = fixture();
    let request = prepared(&catalog);
    let media = id(&catalog, Kind::Media);
    let output = id(&catalog, Kind::Output);
    let choices = vec![media.clone(), output.clone()];
    let text = json!({"type":"clarification","reason":"scope","choices":choices}).to_string();
    assert_eq!(
        request.decode(&text, &catalog),
        Ok(Decision::Clarification {
            reason: Reason::Scope,
            choices
        })
    );
    for choices in [
        vec![media.clone()],
        vec![media.clone(), media.clone()],
        vec![media.clone(), "forged".into()],
        vec![id(&catalog, Kind::Application), output],
    ] {
        assert!(
            request
                .decode(
                    &json!({"type":"clarification","reason":"scope","choices":choices}).to_string(),
                    &catalog
                )
                .is_err()
        );
    }
    let mut duplicates = apps.clone();
    let mut other = apps[0].clone();
    other.id = "second.desktop".into();
    duplicates.push(other);
    let catalog = Catalog::capture(&state, Some(&duplicates), None, false).unwrap();
    let action = reply(json!({"verb":"open_application","target":id(&catalog,Kind::Application)}));
    assert_eq!(
        prepared(&catalog).decode(&action, &catalog),
        Err(Error::Ambiguous)
    );
}

#[test]
fn changed_catalogs_cannot_rebind_ids_but_unrelated_progress_keeps_them_valid() {
    let (state, apps, catalog) = fixture();
    let request = prepared(&catalog);
    let text = reply(json!({"verb":"media_pause","target":id(&catalog, Kind::Media)}));
    let mut progress = state.clone();
    progress.sessions.values_mut().next().unwrap().step_count += 1;
    let fresh = Catalog::capture(&progress, Some(&apps), Some("private-player-service"), true).unwrap();
    assert!(request.decode(&text, &fresh).is_ok());
    for mode in 0..7 {
        let mut current = state.clone();
        let mut current_apps = apps.clone();
        match mode {
            0 => current.sessions.values_mut().next().unwrap().started += Duration::from_secs(1),
            1 => current.sessions.values_mut().next().unwrap().cwd = Some("/moved".into()),
            2 => current.sessions.values_mut().next().unwrap().terminal.pid = Some(1234),
            3 => current_apps[0].definition = std::sync::Arc::from(&b"changed"[..]),
            4 => current.focus = current.sessions.keys().next().cloned(),
            _ => {}
        }
        let fresh = Catalog::capture(
            &current,
            Some(&current_apps),
            Some(if mode == 5 {
                "replacement-player"
            } else {
                "private-player-service"
            }),
            mode != 6,
        )
        .unwrap();
        assert_eq!(request.decode(&text, &fresh), Err(Error::Stale));
    }
}

#[test]
fn input_guards_keep_local_commands_and_unsafe_content_out_of_model_execution() {
    let (_, _, catalog) = fixture();
    for text in [
        "pause media",
        "approve all cards",
        "computer volume -10",
        "\"pause media\"",
    ] {
        assert!(matches!(Request::new(text, catalog.clone()), Err(Error::Local)));
    }
    for text in ["", "\0bad"] {
        assert!(matches!(Request::new(text, catalog.clone()), Err(Error::Input)));
    }
    assert!(Request::new(&"a".repeat(command::MAX_INPUT_BYTES + 1), catalog.clone()).is_err());
    let action = reply(json!({"verb":"media_pause","target":id(&catalog,Kind::Media)}));
    for heard in [
        "do not stop what I am listening to",
        "nao interrompa o que estou ouvindo",
        "no dejes de reproducir esto",
    ] {
        match Request::new(heard, catalog.clone()) {
            Err(Error::Local) => {}
            Ok(request) => {
                assert!(request.prompt().is_err());
                assert_eq!(request.decode(&action, &catalog), Err(Error::Unsafe));
            }
            result => panic!("unexpected input guard: {result:?}"),
        }
    }
    let request = Request::new("how would I pause what is playing", catalog.clone()).unwrap();
    assert_eq!(request.decode(&action, &catalog), Err(Error::Unsafe));
    assert!(
        request
            .decode(&reply(json!({"verb":"waiting"})), &catalog)
            .is_ok()
    );
    for heard in [
        "quero que a musica deixe de tocar",
        "make what I am hearing quiet for a moment",
        "quiero dejar de escuchar esto",
    ] {
        let request = Request::new(heard, catalog.clone()).unwrap();
        assert!(request.decode(&action, &catalog).is_ok());
    }
}

#[test]
fn catalog_and_wire_limits_are_explicit_and_never_silently_truncate_targets() {
    let state = State::default();
    let mut apps: Vec<_> = (0..MAX_TARGETS)
        .map(|index| {
            super::super::tests::installed_app(&format!("app-{index}.desktop"), &format!("Editor {index}"))
        })
        .collect();
    assert!(Catalog::capture(&state, Some(&apps), None, false).is_ok());
    apps.push(super::super::tests::installed_app("overflow.desktop", "Overflow"));
    assert_eq!(
        Catalog::capture(&state, Some(&apps), None, false),
        Err(Error::Catalog)
    );
    for label in [
        "/private/path".to_owned(),
        "bad\nlabel".into(),
        "x".repeat(MAX_LABEL_BYTES + 1),
    ] {
        let app = super::super::tests::installed_app("unsafe.desktop", &label);
        assert!(Catalog::capture(&state, Some(&[app]), None, false).is_err());
    }
    let (_, apps, catalog) = fixture();
    assert!(Catalog::capture(&state, Some(&[apps[0].clone(), apps[0].clone()]), None, false).is_err());
    assert!(matches!(
        Request::new("please inspect /private/path", catalog.clone()),
        Err(Error::Unsafe)
    ));
    let request = prepared(&catalog);
    let max =
        json!({"type":"actions","actions":vec![json!({"verb":"waiting"});command::MAX_COMMANDS]}).to_string();
    assert!(request.decode(&max, &catalog).is_ok());
    let padded = format!("{:width$}", "{\"type\":\"unsupported\"}", width = MAX_REPLY_BYTES);
    assert_eq!(request.decode(&padded, &catalog), Ok(Decision::Unsupported));
}
