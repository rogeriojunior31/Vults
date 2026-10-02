//! User-facing text. Whole sentences per language, never assembled from fragments, so a
//! translation can reorder words freely. pt-BR comes after 1.0.

use serde::{Deserialize, Serialize};

use crate::Activity;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Lang {
    #[default]
    En,
}

/// `Editing main.rs`, or just `Editing`.
pub fn step(lang: Lang, activity: Activity, tool: &str, detail: Option<&str>) -> String {
    let Lang::En = lang;
    let verb = match activity {
        Activity::Read => "Reading",
        Activity::Search => "Searching",
        Activity::Edit => "Editing",
        Activity::Run => "Running",
        Activity::Web => "Browsing",
        Activity::Plan => "Planning",
        Activity::Subagent => "Delegating",
        Activity::Think => "Thinking",
        Activity::Work => tool,
    };
    match detail {
        Some(d) => format!("{verb} {d}"),
        None => verb.to_string(),
    }
}

/// A step one of the user's Always rules allowed, without a card: `Running cargo test · always
/// allowed`.
pub fn ruled_step(lang: Lang, step: &str) -> String {
    let Lang::En = lang;
    format!("{step} · always allowed")
}
