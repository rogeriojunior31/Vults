//! Local Your words data. The Nest owns edits; each recording takes a read-only snapshot.

use std::path::Path;

use serde::{Deserialize, Serialize};
use tauri::{Manager, Webview};
use vults_store::{Store, words::Words};

#[derive(Debug, Deserialize)]
#[serde(tag = "action", rename_all = "camelCase", deny_unknown_fields)]
pub(crate) enum Change {
    AddWord { word: String },
    DeleteWord { word: String },
    SetReplacement { from: String, to: String },
    DeleteReplacement { id: i64 },
    AddSnippet { trigger: String, text: String },
    DeleteSnippet { id: i64 },
}

fn page<R: tauri::Runtime>(window: &Webview<R>) -> Result<(), String> {
    if window.label() != crate::NEST {
        return Err("Your words is managed in the Nest.".into());
    }
    Ok(())
}

fn error(error: vults_store::Error) -> String {
    match error {
        vults_store::Error::Invalid(message) => message.into(),
        // SQLite diagnostics can contain the user's phrases or snippets.
        _ => "Could not access Your words.".into(),
    }
}

fn load(dir: &Path) -> Result<Words, String> {
    Store::open(dir)
        .and_then(|mut store| store.dictation_words())
        .map_err(error)
}

pub(crate) fn rules(dir: &Path) -> Result<vults_dictation::text::Rules, String> {
    let saved = load(dir)?;
    Ok(vults_dictation::text::Rules {
        words: saved.words,
        replacements: saved.replacements.into_iter().map(|r| (r.from, r.to)).collect(),
        snippets: saved.snippets.into_iter().map(|s| (s.trigger, s.text)).collect(),
    })
}

fn change(dir: &Path, change: Change) -> Result<(), String> {
    let mut store = Store::open(dir).map_err(error)?;
    match change {
        Change::AddWord { word } => store.dictionary_word_add(&word),
        Change::DeleteWord { word } => store.dictionary_word_delete(&word),
        Change::SetReplacement { from, to } => store.dictionary_replacement_set(&from, &to).map(|_| ()),
        Change::DeleteReplacement { id } => store.dictionary_replacement_delete(id),
        Change::AddSnippet { trigger, text } => store.snippet_add(&trigger, &text).map(|_| ()),
        Change::DeleteSnippet { id } => store.snippet_delete(id),
    }
    .map_err(error)
}

#[derive(Debug, Serialize)]
pub(crate) struct Preview {
    text: String,
    cancelled: bool,
    segments: Vec<PreviewSegment>,
}

#[derive(Debug, Serialize)]
struct PreviewSegment {
    text: String,
    kind: Option<&'static str>,
    rule: Option<String>,
}

fn preview(
    dir: &Path,
    text: &str,
    terminal: bool,
    spoken_code: bool,
    cancel: bool,
) -> Result<Preview, String> {
    if text.len() > 256 * 1024 {
        return Err(vults_dictation::text::TOO_LARGE.into());
    }
    if cancel && vults_dictation::text::cancels_dictation(text) {
        return Ok(Preview {
            text: String::new(),
            cancelled: true,
            segments: Vec::new(),
        });
    }
    let processed = rules(dir)?
        .apply(text, terminal && spoken_code)
        .map_err(str::to_owned)?;
    let output = processed.text();
    let segments = processed
        .segments
        .into_iter()
        .map(|s| {
            let (kind, rule) = match s.change {
                Some(c) => (
                    Some(match c.kind {
                        vults_dictation::text::Kind::Snippet => "snippet",
                        vults_dictation::text::Kind::Replacement => "replacement",
                        vults_dictation::text::Kind::Spelling => "spelling",
                        vults_dictation::text::Kind::SpokenCode => "spokenCode",
                    }),
                    Some(c.rule),
                ),
                None => (None, None),
            };
            PreviewSegment {
                text: s.text,
                kind,
                rule,
            }
        })
        .collect();
    Ok(Preview {
        text: output,
        cancelled: false,
        segments,
    })
}

#[tauri::command]
pub async fn dictation_try<R: tauri::Runtime>(
    window: Webview<R>,
    text: String,
    terminal: bool,
) -> Result<Preview, String> {
    page(&window)?;
    let (code, cancel) = window
        .state::<crate::settings::SettingsState>()
        .0
        .lock()
        .map(|s| (s.dictation_spoken_code, s.dictation_cancel_by_voice))
        .map_err(|_| "Settings are busy.".to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        preview(&crate::history::dir(), &text, terminal, code, cancel)
    })
    .await
    .map_err(|_| "Could not test this sentence.".to_string())?
}

#[tauri::command]
pub async fn dictation_words<R: tauri::Runtime>(window: Webview<R>) -> Result<Words, String> {
    page(&window)?;
    tauri::async_runtime::spawn_blocking(|| load(&crate::history::dir()))
        .await
        .map_err(|_| "Could not access Your words.".to_string())?
}

#[tauri::command]
pub async fn dictation_words_change<R: tauri::Runtime>(
    window: Webview<R>,
    edit: Change,
) -> Result<(), String> {
    page(&window)?;
    tauri::async_runtime::spawn_blocking(move || change(&crate::history::dir(), edit))
        .await
        .map_err(|_| "Could not access Your words.".to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_nest_work_page_may_read_or_edit() {
        let app = tauri::test::mock_app();
        for label in [crate::NEST, crate::SETTINGS, crate::ISLAND, "widget"] {
            let view = tauri::WebviewWindowBuilder::new(&app, label, tauri::WebviewUrl::default())
                .build()
                .unwrap();
            assert_eq!(page(view.as_ref()).is_ok(), label == crate::NEST);
            if label != crate::NEST {
                assert!(tauri::async_runtime::block_on(dictation_words(view.as_ref().clone())).is_err());
                assert!(
                    tauri::async_runtime::block_on(dictation_try(
                        view.as_ref().clone(),
                        "ignored".into(),
                        false
                    ))
                    .is_err()
                );
                assert!(
                    tauri::async_runtime::block_on(dictation_words_change(
                        view.as_ref().clone(),
                        Change::AddWord {
                            word: "Ignored".into()
                        }
                    ))
                    .is_err()
                );
            }
        }
    }

    #[test]
    fn commands_validate_changes_round_trip_and_keep_database_details_private() {
        let dir = std::env::temp_dir().join(format!("vults-your-words-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let edit = |value| change(&dir, serde_json::from_value(value).unwrap());
        edit(serde_json::json!({ "action": "addWord", "word": "Vults" })).unwrap();
        assert!(edit(serde_json::json!({ "action": "addWord", "word": "vults" })).is_err());
        edit(serde_json::json!({ "action": "setReplacement", "from": "cloud code", "to": "Claude Code" }))
            .unwrap();
        edit(serde_json::json!({ "action": "addSnippet", "trigger": "signature", "text": "Regards,\nAda" }))
            .unwrap();
        let captured = rules(&dir).unwrap();
        let tried = preview(&dir, "cloud code signature source slash main", true, true, true).unwrap();
        assert_eq!(tried.text, "Claude Code Regards,\nAda source/main");
        assert!(tried.segments.iter().any(|s| s.kind == Some("snippet")));
        assert!(preview(&dir, "cancel", false, true, true).unwrap().cancelled);
        assert_eq!(
            preview(&dir, "cancel", false, true, false).unwrap().text,
            "cancel"
        );
        assert_eq!(
            preview(&dir, "source slash main", false, true, true)
                .unwrap()
                .text,
            "source slash main"
        );
        assert!(preview(&dir, &"x".repeat(256 * 1024 + 1), false, true, true).is_err());
        edit(serde_json::json!({ "action": "setReplacement", "from": "cloud code", "to": "Changed later" }))
            .unwrap();
        assert_eq!(captured.apply("cloud code", false).unwrap().text(), "Claude Code");
        assert_eq!(
            rules(&dir).unwrap().apply("cloud code", false).unwrap().text(),
            "Changed later"
        );
        let saved = load(&dir).unwrap();
        assert_eq!(saved.words, ["Vults"]);
        assert_eq!(saved.replacements[0].to, "Changed later");
        assert_eq!(saved.snippets[0].text, "Regards,\nAda");
        edit(serde_json::json!({ "action": "deleteWord", "word": "VULTS" })).unwrap();
        edit(serde_json::json!({ "action": "deleteReplacement", "id": saved.replacements[0].id })).unwrap();
        edit(serde_json::json!({ "action": "deleteSnippet", "id": saved.snippets[0].id })).unwrap();
        let empty = load(&dir).unwrap();
        assert!(empty.words.is_empty() && empty.replacements.is_empty() && empty.snippets.is_empty());
        assert!(
            serde_json::from_value::<Change>(serde_json::json!({ "action": "clearEverything" })).is_err()
        );
        assert!(
            serde_json::from_value::<Change>(
                serde_json::json!({ "action": "addWord", "word": "A", "run": true })
            )
            .is_err()
        );
        assert!(edit(serde_json::json!({ "action": "deleteSnippet", "id": -1 })).is_err());
        std::fs::remove_dir_all(&dir).unwrap();
        std::fs::write(&dir, "not a directory").unwrap();
        assert_eq!(load(&dir).unwrap_err(), "Could not access Your words.");
        assert_eq!(
            error(vults_store::Error::Invalid("Invalid phrase.")),
            "Invalid phrase."
        );
        std::fs::remove_file(&dir).unwrap();
    }
}
