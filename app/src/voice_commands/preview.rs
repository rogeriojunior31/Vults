//! Read-only interpretation: this module cannot produce effects or change command context.
use super::{Catalog, Command, Parsed, Preference, State, command};
use serde::Serialize;
use vults_dictation::command::names::{NameKind, Resolution};
use vults_platform::applications::Application;

#[derive(Debug, Serialize)]
pub(crate) struct Preview {
    pub status: &'static str,
    pub actions: Vec<Action>,
}
#[derive(Debug, Serialize)]
pub(crate) struct Action {
    pub action: &'static str,
    pub target: Option<String>,
    pub value: Option<u8>,
    pub status: &'static str,
    pub candidates: Vec<String>,
}

pub(crate) fn interpret(
    state: &State,
    text: &str,
    replacements: &[(String, String)],
    applications: Option<&[Application]>,
) -> Preview {
    if state.locked {
        return Preview {
            status: "locked",
            actions: vec![],
        };
    }
    let commands = match command::parse(text) {
        Parsed::Commands(commands) => commands,
        other => {
            return Preview {
                status: match other {
                    Parsed::Refused => "refused",
                    Parsed::InvalidVolume => "volume",
                    Parsed::Quoted => "quoted",
                    Parsed::Negated => "negated",
                    Parsed::AudioScopeRequired => "scope",
                    Parsed::UnsupportedMediaTarget => "media-target",
                    _ => "unknown",
                },
                actions: vec![],
            };
        }
    };
    let actions = commands
        .iter()
        .map(|command| {
            use command::{MediaAction as M, OutputAction as O};
            let action = match command {
                Command::Media(M::Pause) => "pause",
                Command::Media(M::Resume) => "resume",
                Command::Media(M::Next) => "next-track",
                Command::Media(M::Previous) => "previous-track",
                Command::Media(M::Volume(_)) => "player-volume",
                Command::Media(M::Quieter) => "player-quieter",
                Command::Media(M::Louder) => "player-louder",
                Command::Output(O::Mute(true)) => "computer-mute",
                Command::Output(O::Mute(false)) => "computer-unmute",
                Command::Output(O::Volume(_)) => "computer-volume",
                Command::Output(O::Quieter) => "computer-quieter",
                Command::Output(O::Louder) => "computer-louder",
                Command::Next => "next-session",
                Command::Previous => "previous-session",
                Command::Focus(_) => "focus",
                Command::Open(_) => "open",
                Command::Application(_) => "application",
                Command::Terminal(_) => "terminal",
                Command::Folder(_) => "folder",
                Command::SetPreference {
                    preference: Preference::Mute,
                    on: true,
                    ..
                } => "notifications-mute",
                Command::SetPreference {
                    preference: Preference::Mute,
                    on: false,
                    ..
                } => "notifications-unmute",
                Command::SetPreference {
                    preference: Preference::Pin,
                    on: true,
                    ..
                } => "pin",
                Command::SetPreference {
                    preference: Preference::Pin,
                    on: false,
                    ..
                } => "unpin",
                Command::SetPreference {
                    preference: Preference::Hide,
                    on: true,
                    ..
                } => "hide",
                Command::SetPreference {
                    preference: Preference::Hide,
                    on: false,
                    ..
                } => "unhide",
                Command::Waiting => "waiting",
                Command::Session(_) => "session",
                Command::Usage => "usage",
                Command::PullRequests => "pull-requests",
                Command::Repeat(_) => "repeat",
                Command::Undo => "undo",
            };
            let mut row = Action {
                action,
                target: None,
                value: match command {
                    Command::Media(M::Volume(value)) | Command::Output(O::Volume(value)) => Some(*value),
                    _ => None,
                },
                status: "recognized",
                candidates: vec![],
            };
            if matches!(command, Command::Repeat(_) | Command::Undo) {
                row.status = "context";
                return row;
            }
            if let Some(text) = super::target(command) {
                row.target = Some(text.into());
                let project = matches!(command, Command::SetPreference { .. });
                let apps = matches!(command, Command::Open(_) | Command::Application(_));
                if matches!(command, Command::Application(_)) && applications.is_none() {
                    row.status = "apps-off";
                    return row;
                }
                let catalog = Catalog::with_applications(
                    state,
                    project,
                    replacements,
                    if apps { applications.unwrap_or(&[]) } else { &[] },
                );
                match catalog.resolver.resolve(
                    text,
                    matches!(command, Command::Application(_)).then_some(NameKind::Application),
                ) {
                    Resolution::Found(name) => {
                        row.target = Some(name.name);
                    }
                    Resolution::Unknown => {
                        row.status = if apps && applications.is_none() {
                            "apps-off"
                        } else {
                            "missing"
                        }
                    }
                    Resolution::Ambiguous(names) => {
                        row.status = "ambiguous";
                        row.candidates = names.into_iter().map(|name| name.name).collect();
                    }
                }
            }
            row
        })
        .collect();
    Preview {
        status: "parsed",
        actions,
    }
}

type CheckSources = (Vec<(String, String)>, Option<Vec<Application>>);

fn read_sources(
    parsed: &Parsed,
    replacements: impl FnOnce() -> Result<Vec<(String, String)>, String>,
    applications: impl FnOnce() -> Result<Option<Vec<Application>>, String>,
) -> Result<CheckSources, String> {
    let Parsed::Commands(commands) = parsed else {
        return Ok((vec![], None));
    };
    let apps = if commands
        .iter()
        .any(|command| matches!(command, Command::Open(_) | Command::Application(_)))
    {
        applications()?
    } else {
        None
    };
    // Rejected/non-target phrases and disabled explicit app queries need no dictionary access.
    let names = commands.iter().any(|command| {
        super::target(command).is_some() && (!matches!(command, Command::Application(_)) || apps.is_some())
    });
    let replacements = if names { replacements()? } else { vec![] };
    Ok((replacements, apps))
}

#[tauri::command]
pub(crate) async fn voice_command_check(window: tauri::Webview, text: String) -> Result<Preview, String> {
    use tauri::Manager;
    if !matches!(window.label(), "nest" | "settings") {
        return Err("Open Dictation to check a command.".into());
    }
    if text.len() > command::MAX_INPUT_BYTES || text.chars().any(|ch| ch.is_control() && !ch.is_whitespace())
    {
        return Err("The command phrase is invalid or too long.".into());
    }
    let app = window.app_handle().clone();
    let parsed = command::parse(&text);
    let (replacements, applications) = tauri::async_runtime::spawn_blocking(move || {
        read_sources(
            &parsed,
            || crate::dictation_words::rules(&crate::history::dir()).map(|rules| rules.replacements),
            || {
                #[cfg(target_os = "linux")]
                {
                    let enabled = app
                        .state::<crate::settings::SettingsState>()
                        .0
                        .lock()
                        .map_err(|_| "Settings are busy.")?
                        .dictation_command_apps;
                    Ok(enabled.then(vults_platform::applications::installed))
                }
                #[cfg(not(target_os = "linux"))]
                {
                    let _ = app;
                    Ok(None)
                }
            },
        )
    })
    .await
    .map_err(|_| "Could not check the command phrase.")??;
    crate::runtime::voice_command_preview(window.app_handle(), text, replacements, applications).await
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preview_and_dispatcher_share_the_enabled_open_target_scope() {
        let now = std::time::Instant::now();
        let state = super::super::tests::fixture(now);
        let before = serde_json::to_value(state.view()).unwrap();
        let preview = interpret(&state, "open api", &[], None);
        let mut dispatched = state.clone();
        let actual = super::super::Commands::new(now).execute_with_applications(
            &mut dispatched,
            "open api",
            &[],
            serde_json::json!([]),
            now,
            None,
        );
        assert_eq!(actual.feedback, super::super::Feedback::Done);
        assert_eq!(
            preview.actions[0].status, "recognized",
            "Disabled app matching must not invalidate a known project"
        );
        assert_eq!(serde_json::to_value(state.view()).unwrap(), before);
        let apps = [super::super::tests::installed_app("api.desktop", "api")];
        let ambiguous = interpret(&state, "open api", &[], Some(&apps));
        assert_eq!(ambiguous.actions[0].status, "ambiguous");
        assert_eq!(ambiguous.actions[0].candidates.len(), 2);
        let actual = super::super::Commands::new(now).execute_with_applications(
            &mut dispatched,
            "open api",
            &[],
            serde_json::json!([]),
            now,
            Some(&apps),
        );
        assert_eq!(actual.feedback, super::super::Feedback::WhichOne);
        assert_eq!(
            interpret(&state, "open app api", &[], None).actions[0].status,
            "apps-off"
        );
    }

    #[test]
    fn preview_reads_only_the_sources_required_by_the_complete_phrase() {
        let state = State::default();
        for phrase in [
            "pause media",
            "computer volume 40",
            "next session",
            "who is waiting?",
            "undo",
            "approve",
            "do not pause music",
            "say \"open app Firefox\"",
            "media volume 120",
            "lower volume",
            "pause media in Firefox",
            "pause media and then approve",
            "write a poem",
        ] {
            let (aliases, apps) = read_sources(
                &command::parse(phrase),
                || panic!("Unexpected dictionary read for {phrase}"),
                || panic!("Unexpected app/settings read for {phrase}"),
            )
            .unwrap();
            assert!(aliases.is_empty() && apps.is_none());
            let result = interpret(&state, phrase, &aliases, apps.as_deref());
            assert!(!result.status.is_empty());
        }
        let (aliases, apps) = read_sources(
            &command::parse("open app Firefox"),
            || panic!("Disabled explicit app matching cannot need aliases"),
            || Ok(None),
        )
        .unwrap();
        assert_eq!(
            interpret(&state, "open app Firefox", &aliases, apps.as_deref()).actions[0].status,
            "apps-off"
        );
        let error = read_sources(
            &command::parse("open project api"),
            || Err("Dictionary unavailable".into()),
            || panic!("Project scope never enumerates apps"),
        )
        .unwrap_err();
        assert_eq!(error, "Dictionary unavailable");
        let calls = std::cell::RefCell::new(Vec::new());
        let app = super::super::tests::installed_app("firefox.desktop", "Firefox");
        let (aliases, apps) = read_sources(
            &command::parse("open app Firefox"),
            || {
                calls.borrow_mut().push("aliases");
                Ok(vec![])
            },
            || {
                calls.borrow_mut().push("apps");
                Ok(Some(vec![app]))
            },
        )
        .unwrap();
        assert_eq!(*calls.borrow(), ["apps", "aliases"]);
        assert_eq!(
            interpret(&state, "open app Firefox", &aliases, apps.as_deref()).actions[0].status,
            "recognized"
        );
        let (aliases, apps) = read_sources(
            &command::parse("open project backend and then pause media"),
            || Ok(vec![("backend".into(), "api".into())]),
            || panic!("A mixed project/media batch needs no app catalog"),
        )
        .unwrap();
        assert_eq!(aliases, [("backend".into(), "api".into())]);
        assert!(apps.is_none());
    }

    #[test]
    fn preview_uses_the_live_grammar_and_resolver_without_changing_state() {
        let state = super::super::tests::fixture(std::time::Instant::now());
        let before = serde_json::to_value(state.view()).unwrap();
        for (phrase, status) in [
            ("do not pause music", "negated"),
            ("approve", "refused"),
            ("lower volume", "scope"),
            ("pause media in Firefox", "media-target"),
            ("media volume 120", "volume"),
            ("write a poem", "unknown"),
        ] {
            assert_eq!(interpret(&state, phrase, &[], None).status, status, "{phrase}");
        }
        let result = interpret(&state, "pause media and then open project api", &[], None);
        assert_eq!(result.actions.len(), 2);
        assert_eq!(result.actions[0].action, "pause");
        assert_eq!(result.actions[1].status, "recognized");
        assert!(result.actions[1].target.as_ref().unwrap().contains("api"));
        assert_eq!(
            interpret(&state, "same for web", &[], None).actions[0].status,
            "context"
        );
        let application = Application {
            id: "api.desktop".into(),
            name: "api".into(),
            aliases: vec![],
            source: std::path::PathBuf::from("/fixture/api.desktop"),
            definition: std::sync::Arc::from([]),
        };
        let apps = [application];
        let ambiguous = interpret(&state, "open api", &[], Some(&apps));
        assert_eq!(ambiguous.actions[0].status, "ambiguous");
        assert_eq!(ambiguous.actions[0].candidates.len(), 2);
        let explicit = interpret(&state, "open app api", &[], Some(&apps));
        assert_eq!(explicit.actions[0].status, "recognized");
        assert_eq!(explicit.actions[0].target.as_deref(), Some("api / api.desktop"));
        assert_eq!(
            interpret(&state, "open app api", &[], None).actions[0].status,
            "apps-off"
        );
        let aliased = interpret(
            &state,
            "open project backend",
            &[("backend".into(), "api".into())],
            None,
        );
        assert_eq!(aliased.actions[0].status, "recognized");
        assert_eq!(serde_json::to_value(state.view()).unwrap(), before);
    }
}
