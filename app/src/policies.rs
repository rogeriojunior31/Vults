//! The user's policies (ADR 0014, `docs/dev/plan-zeca.md` W2), as Settings → Approvals writes them:
//! the text, seen as a diff against what stood before, accepted with a click. Each acceptance is a
//! consent (`vults_store::consents`) whose own text is the policy: nothing else holds it, so no
//! file can make a policy the user did not accept. One policy per folder (or everywhere); a newer
//! one revokes the older, and revoking takes it out of the core at once.
//!
//! A policy reaches the core only while its consent stands and was given to a diff that ends at
//! that very text (`valid_for`). Without the record, nothing answers.

use serde::Serialize;
use tauri::AppHandle;
use vults_core::policies::{self, Active, ParseError};
use vults_store::Store;
use vults_store::consents::{Consent, ConsentId};

/// The consents' kind for a policy.
const KIND: &str = "policy";
/// The subject of a policy that names no folder.
const EVERYWHERE: &str = "everywhere";

/// A standing policy, for Settings.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Row {
    pub consent: i64,
    /// Its folder, shown with `~`; none holds everywhere.
    pub project: Option<String>,
    pub text: String,
    /// When it was accepted, Unix seconds.
    pub at: i64,
}

/// What Settings shows before the click.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Preview {
    /// Against the policy that stands for the same folder, or against nothing; empty when the text
    /// is that policy already.
    pub diff: String,
    pub error: Option<ParseError>,
}

fn home() -> Option<String> {
    crate::paths::home().to_str().map(str::to_owned)
}

fn subject(text: &str) -> Result<String, ParseError> {
    let policy = policies::parse(text, home().as_deref())?;
    Ok(policy.project.unwrap_or_else(|| EVERYWHERE.to_owned()))
}

fn diff(before: &str, after: &str, subject: &str) -> String {
    vults_agent_config::unified(before, after, subject)
}

/// The newest policy consent of each subject, when it stands.
fn standing(all: &[Consent]) -> Vec<&Consent> {
    let mut seen = std::collections::HashSet::new();
    all.iter()
        .filter(|c| c.kind == KIND)
        // Newest first: the first of each subject is its latest word.
        .filter(|c| seen.insert(c.subject.as_str()))
        .filter(|c| c.revoked_at.is_none())
        .collect()
}

/// The policies that may answer: each standing consent whose diff ends at its text (against the
/// policy it replaced, or against nothing), that parses, and that holds where its subject says.
fn active_in(store: &Store) -> Vec<Active> {
    let Ok(all) = store.consents() else {
        return Vec::new();
    };
    let home = home();
    standing(&all)
        .into_iter()
        .filter_map(|c| {
            let before = all
                .iter()
                .find(|o| o.kind == KIND && o.subject == c.subject && o.id.0 < c.id.0)
                .map_or("", |o| o.text.as_str());
            let seen = [diff(before, &c.text, &c.subject), diff("", &c.text, &c.subject)];
            if !seen.iter().any(|d| store.valid_for(c.id, d).unwrap_or(false)) {
                tracing::warn!(
                    consent = c.id.0,
                    "a policy's consent does not match its diff: ignored"
                );
                return None;
            }
            let policy = policies::parse(&c.text, home.as_deref()).ok()?;
            let subject = policy.project.as_deref().unwrap_or(EVERYWHERE);
            (subject == c.subject).then_some(Active {
                consent: c.id.0,
                policy,
            })
        })
        .collect()
}

/// The policies for the core, at start-up.
pub fn active() -> Vec<Active> {
    match Store::open(&crate::history::dir()) {
        Ok(store) => active_in(&store),
        Err(e) => {
            tracing::warn!(error = %e, "policies not read: every permission asks");
            Vec::new()
        }
    }
}

fn open() -> Result<Store, String> {
    Store::open(&crate::history::dir()).map_err(|e| e.to_string())
}

/// Tells the core the policies that stand now; the store is closed before the wait.
async fn reload(app: &AppHandle, store: Store) -> Result<(), String> {
    let list = active_in(&store);
    drop(store);
    tracing::info!(count = list.len(), "policies changed");
    crate::runtime::set_policies(app, list).await
}

/// The standing policies.
#[tauri::command]
pub fn policies_list() -> Result<Vec<Row>, String> {
    let store = open()?;
    let all = store.consents().map_err(|e| e.to_string())?;
    Ok(standing(&all)
        .into_iter()
        .map(|c| Row {
            consent: c.id.0,
            project: (c.subject != EVERYWHERE).then(|| crate::paths::shown(std::path::Path::new(&c.subject))),
            text: c.text.clone(),
            at: c.at,
        })
        .collect())
}

/// The diff a click would accept, or why the text is not a policy.
#[tauri::command]
pub fn policy_preview(text: String) -> Result<Preview, String> {
    if text.len() > policies::MAX_TEXT {
        return Err("A policy is at most 16 KB.".into());
    }
    let subject = match subject(&text) {
        Ok(s) => s,
        Err(e) => {
            return Ok(Preview {
                diff: String::new(),
                error: Some(e),
            });
        }
    };
    let store = open()?;
    let all = store.consents().map_err(|e| e.to_string())?;
    let before = standing(&all)
        .into_iter()
        .find(|c| c.subject == subject)
        .map_or(String::new(), |c| c.text.clone());
    Ok(Preview {
        diff: diff(&before, &text, &subject),
        error: None,
    })
}

/// The click: records the consent to `text` after the user saw `shown`, and revokes the policy it
/// replaces. Refused when the diff is no longer what was shown (another window accepted first).
#[tauri::command]
pub async fn policy_accept(app: AppHandle, text: String, shown: String) -> Result<(), String> {
    if text.len() > policies::MAX_TEXT {
        return Err("A policy is at most 16 KB.".into());
    }
    let subject = subject(&text).map_err(|_| "This is not a policy yet.".to_string())?;
    let mut store = open()?;
    let all = store.consents().map_err(|e| e.to_string())?;
    let old = standing(&all).into_iter().find(|c| c.subject == subject).cloned();
    let now = diff(old.as_ref().map_or("", |c| c.text.as_str()), &text, &subject);
    if now.is_empty() {
        return Err("This policy is already the one in place.".into());
    }
    if now != shown {
        return Err("The policy changed since you looked: review it again.".into());
    }
    let id = store
        .consent(KIND, &subject, &now, &text)
        .map_err(|e| e.to_string())?;
    if let Some(old) = old {
        store.revoke(old.id).map_err(|e| e.to_string())?;
    }
    tracing::info!(consent = id.0, "policy accepted");
    reload(&app, store).await
}

/// Takes a policy back: from now on its cards ask again.
#[tauri::command]
pub async fn policy_revoke(app: AppHandle, consent: i64) -> Result<(), String> {
    let mut store = open()?;
    let all = store.consents().map_err(|e| e.to_string())?;
    if !all.iter().any(|c| c.id.0 == consent && c.kind == KIND) {
        return Err("No such policy.".into());
    }
    store.revoke(ConsentId(consent)).map_err(|e| e.to_string())?;
    tracing::info!(consent, "policy revoked");
    reload(&app, store).await
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    fn store(name: &str) -> Store {
        let dir = std::env::temp_dir().join(format!("vults-policies-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        Store::open(&dir).unwrap()
    }

    const TEXT: &str = "project /srv/site\nallow command git status\n";

    #[test]
    fn no_policy_answers_without_the_consent_record() {
        let mut s = store("record");
        assert!(active_in(&s).is_empty(), "nothing accepted, nothing answers");
        // Accepted as Settings does: the diff against nothing, then the click.
        let shown = diff("", TEXT, "/srv/site");
        let id = s.consent(KIND, "/srv/site", &shown, TEXT).unwrap();
        let list = active_in(&s);
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].consent, id.0);
        // Revoked: gone.
        s.revoke(id).unwrap();
        assert!(active_in(&s).is_empty());
        // A consent to another diff than the one ending at its text: never active.
        s.consent(KIND, "/srv/site", "some other diff", TEXT).unwrap();
        assert!(active_in(&s).is_empty());
        // A consent filed under another folder than the text names: never active.
        let elsewhere = diff("", TEXT, "/srv/other");
        s.consent(KIND, "/srv/other", &elsewhere, TEXT).unwrap();
        assert!(active_in(&s).is_empty());
        // Another kind of consent is no policy.
        s.consent("connector", "/srv/site", &shown, TEXT).unwrap();
        assert!(active_in(&s).is_empty());
    }

    #[test]
    fn a_newer_policy_replaces_the_older_and_its_diff_is_against_it() {
        let mut s = store("replace");
        let first = s
            .consent(KIND, "/srv/site", &diff("", TEXT, "/srv/site"), TEXT)
            .unwrap();
        let next = "project /srv/site\nallow command git status\ndeny external\n";
        let shown = diff(TEXT, next, "/srv/site");
        assert!(shown.contains("+deny external"));
        let second = s.consent(KIND, "/srv/site", &shown, next).unwrap();
        s.revoke(first).unwrap();
        let list = active_in(&s);
        assert_eq!(list.iter().map(|a| a.consent).collect::<Vec<_>>(), [second.0]);
        assert_eq!(list[0].policy.lines.len(), 2);
        // Revoking the newest leaves nothing: the older one was revoked when it was replaced.
        s.revoke(second).unwrap();
        assert!(active_in(&s).is_empty());
    }
}
