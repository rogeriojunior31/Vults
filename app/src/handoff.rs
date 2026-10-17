//! `/continue` in the chat (`docs/dev/plan-zeca.md` W7): words for a watched session that has
//! stopped, resumed here (`continuation::continue_session`). Typed by the user, sent to no model of Zeca's.

use std::path::PathBuf;

use serde::Serialize;
use tauri::AppHandle;

/// What `/continue` says back, as `/remember` does: an English sentence the window translates, and
/// its detail.
#[derive(Serialize, Debug, PartialEq)]
pub struct Said {
    pub said: &'static str,
    pub detail: Option<String>,
}

fn said((said, detail): crate::continuation::Said) -> Said {
    Said { said, detail }
}

/// A `/continue` typed in the chat; none for any other text.
#[tauri::command]
pub async fn continue_command(
    app: AppHandle,
    window: tauri::Webview,
    text: String,
    folder: Option<String>,
) -> Result<Option<Said>, String> {
    if ![crate::ISLAND, crate::NEST].contains(&window.label()) {
        return Err("only the chat can do this".into());
    }
    let trimmed = text.trim();
    let Some(rest) = trimmed
        .strip_prefix("/continue")
        .filter(|r| r.is_empty() || r.starts_with(char::is_whitespace))
    else {
        return Ok(None);
    };
    let words = rest.trim().to_string();
    if words == "stop" {
        return Ok(Some(said(crate::continuation::stop_continuation(app).await)));
    }
    if words.is_empty() {
        return Ok(Some(said((
            "Say what to tell it: /continue followed by your words.",
            None,
        ))));
    }
    let Some(cwd) = folder.map(PathBuf::from).filter(|d| d.is_absolute()) else {
        return Ok(Some(said((
            "This conversation has no folder: pick one first.",
            None,
        ))));
    };
    Ok(Some(said(
        crate::continuation::continue_session(app, cwd.to_string_lossy().into_owned(), words).await,
    )))
}
