//! Offers inside an opened Chat. These are view data, never speech, requests or decisions.

use crate::{
    SessionRef, SessionView, Status,
    i18n::{self, Lang},
};
use serde::Serialize;

#[derive(Serialize, Clone, Debug, PartialEq)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct ChatView {
    pub greeting: String,
    pub caption: String,
    pub suggestions: Vec<ChatSuggestion>,
    pub followups: Vec<ChatFollowup>,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct ChatSuggestion {
    pub id: String,
    pub text: String,
    pub session: Option<SessionRef>,
}

#[derive(Serialize, Clone, Copy, Debug, PartialEq)]
#[serde(rename_all = "kebab-case")]
#[cfg_attr(test, derive(ts_rs::TS))]
pub enum ChatFollowup {
    Explain,
    Next,
    Flock,
}

pub(crate) fn view(sessions: &[SessionView], hour: Option<u8>, lang: Lang) -> ChatView {
    use i18n::ChatText as T;
    let waiting = sessions
        .iter()
        .find(|s| s.card)
        .or_else(|| sessions.iter().find(|s| s.waiting));
    let failed = sessions.iter().find(|s| s.status == Status::Failed);
    let working = sessions
        .iter()
        .filter(|s| matches!(s.status, Status::Working | Status::Thinking))
        .count();
    let greeting = i18n::chat(
        lang,
        match hour {
            Some(5..=11) => T::Morning,
            Some(12..=17) => T::Afternoon,
            Some(0..=4 | 18..=23) => T::Evening,
            _ => T::Greeting,
        },
        "",
        0,
    );
    let caption = if let Some(s) = waiting {
        i18n::chat(lang, T::Waiting, project(s), 0)
    } else if let Some(s) = failed {
        i18n::chat(lang, T::Failed, project(s), 0)
    } else if working > 0 {
        i18n::chat(lang, T::Working, "", working)
    } else {
        i18n::chat(lang, T::Quiet, "", 0)
    };
    let mut suggestions = Vec::new();
    let mut offer = |id: &str, text, s: &SessionView| {
        suggestions.push(ChatSuggestion {
            id: id.into(),
            text: i18n::chat(lang, text, project(s), 0),
            session: Some(SessionRef {
                agent: s.agent,
                id: s.id.clone(),
            }),
        })
    };
    if let Some(s) = waiting {
        offer("waiting", T::AskWaiting, s);
    }
    if let Some(s) = failed {
        offer("failed", T::AskFailed, s);
    }
    if waiting.is_none()
        && failed.is_none()
        && let Some(s) = sessions
            .iter()
            .find(|s| matches!(s.status, Status::Working | Status::Thinking))
            .or_else(|| sessions.first())
    {
        offer("session", T::AskSession, s);
    }
    if suggestions.is_empty() {
        suggestions.push(ChatSuggestion {
            id: "help".into(),
            text: i18n::chat(lang, T::AskHelp, "", 0),
            session: None,
        });
    }
    suggestions.push(ChatSuggestion {
        id: "recent".into(),
        text: i18n::chat(lang, T::AskRecent, "", 0),
        session: None,
    });
    let mut followups = vec![ChatFollowup::Explain, ChatFollowup::Next];
    if !sessions.is_empty() {
        followups.push(ChatFollowup::Flock);
    }
    ChatView {
        greeting,
        caption,
        suggestions,
        followups,
    }
}

fn project(session: &SessionView) -> &str {
    if session.project.trim().is_empty() {
        session.agent_name.as_deref().unwrap_or(session.agent.name())
    } else {
        &session.project
    }
}
