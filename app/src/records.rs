//! The Nest's records (`docs/dev/plan-zeca.md` W11): its History, Usage, Traces and Audit tabs. Each
//! only reads what this computer already keeps (`vults.sqlite`, the plans' usage), and only for the
//! Nest's window.

use serde::Serialize;
use tauri::AppHandle;
use vults_store::Store;

/// How much each tab shows: the newest first.
const LIMIT: u32 = 200;
/// The Usage tab's days, today included.
const DAYS: i64 = 14;

fn open() -> Result<Store, String> {
    Store::open(&crate::history::dir()).map_err(|e| e.to_string())
}

fn nest_only(window: &tauri::WebviewWindow) -> Result<(), String> {
    if window.label() == crate::NEST {
        Ok(())
    } else {
        Err("only the Nest can do this".into())
    }
}

async fn blocking<T: Send + 'static>(
    f: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|e| e.to_string())?
}

/// One finished turn, counts only (as Settings → Activity keeps it).
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Turn {
    /// Unix seconds.
    pub end: i64,
    pub agent: vults_protocol::AgentKind,
    pub project: String,
    pub secs: u64,
    pub steps: u32,
    pub commands: u32,
    pub files: u32,
    pub added: u32,
    pub removed: u32,
    pub failed: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct History {
    /// Settings → Activity's switch: off, nothing new is kept.
    pub on: bool,
    pub turns: Vec<Turn>,
}

#[tauri::command]
pub async fn nest_history(app: AppHandle, window: tauri::WebviewWindow) -> Result<History, String> {
    nest_only(&window)?;
    let on = crate::settings::history(&app);
    let turns = blocking(|| open()?.turns().map_err(|e| e.to_string())).await?;
    Ok(History {
        on,
        turns: turns
            .into_iter()
            .rev()
            .take(LIMIT as usize)
            .map(|e| Turn {
                end: e.end,
                agent: e.agent,
                project: e.project,
                secs: e.secs,
                steps: e.steps,
                commands: e.commands,
                files: e.files,
                added: e.added,
                removed: e.removed,
                failed: e.failed,
            })
            .collect(),
    })
}

/// One answer to a card (or an install): who, what, on what.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AuditLine {
    /// Unix seconds.
    pub at: i64,
    /// `human`, `phone`, `rule`, `policy`, `system`.
    pub actor: String,
    /// `allow`, `deny`, `always-allow`…
    pub act: String,
    pub agent: String,
    pub project: String,
    pub tool: String,
    pub target: String,
    /// A policy answered: the consent it was accepted under.
    pub consent: Option<i64>,
}

#[tauri::command]
pub async fn nest_audit(window: tauri::WebviewWindow) -> Result<Vec<AuditLine>, String> {
    nest_only(&window)?;
    let lines = blocking(|| open()?.audit(LIMIT).map_err(|e| e.to_string())).await?;
    Ok(lines
        .into_iter()
        .map(|l| AuditLine {
            at: l.at,
            actor: l.actor,
            act: l.act,
            agent: l.agent,
            project: l.project,
            tool: l.tool,
            target: l.target,
            consent: l.consent,
        })
        .collect())
}

/// One span of Zeca's traces: a chat turn, a tool it ran, an action.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Trace {
    pub start_ms: i64,
    pub ms: i64,
    /// `chat claude-opus-5-5`, `execute_tool Bash`.
    pub name: String,
    pub kind: String,
    /// `ok`, `error`, `stopped`, `denied`.
    pub status: String,
    /// It belongs to another span (a tool inside a turn).
    pub child: bool,
    pub input: Option<u64>,
    pub output: Option<u64>,
    pub cost_usd: Option<f64>,
}

#[tauri::command]
pub async fn nest_traces(window: tauri::WebviewWindow) -> Result<Vec<Trace>, String> {
    nest_only(&window)?;
    let spans = blocking(|| open()?.spans(LIMIT).map_err(|e| e.to_string())).await?;
    Ok(spans
        .into_iter()
        .map(|s| Trace {
            ms: (s.end_ms - s.start_ms).max(0),
            start_ms: s.start_ms,
            child: s.parent.is_some(),
            input: s.attrs["gen_ai.usage.input_tokens"].as_u64(),
            output: s.attrs["gen_ai.usage.output_tokens"].as_u64(),
            cost_usd: s.attrs["vults.cost_usd"].as_f64(),
            name: s.name,
            kind: s.kind,
            status: s.status,
        })
        .collect())
}

/// One day's spend.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DaySpend {
    pub day: String,
    pub usd: f64,
    pub tokens: i64,
}

/// One roost's spend, named as the Nest names it.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RoostSpend {
    pub name: String,
    pub today: f64,
    pub total: f64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Usage {
    /// The plans' windows (Claude Code's from its statusLine, Codex's when its meter is on).
    pub windows: Vec<vults_agents::usage::Window>,
    /// The last days with spend, oldest first.
    pub days: Vec<DaySpend>,
    pub roosts: Vec<RoostSpend>,
    pub budget: Option<f64>,
    /// Some of it was on a subscription: at API prices.
    pub subscription: bool,
}

/// A roost key as a name: `github:me/site` is `me/site`; a local repository, its folder's name.
fn roost_name(key: &str) -> String {
    if let Some(repo) = key.strip_prefix("github:") {
        return repo.to_string();
    }
    let path = key.split_once(':').map_or(key, |(_, p)| p);
    let path = std::path::Path::new(path);
    let path = if path.file_name().is_some_and(|n| n == ".git") {
        path.parent().unwrap_or(path)
    } else {
        path
    };
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| key.to_string())
}

#[tauri::command]
pub async fn nest_usage(app: AppHandle, window: tauri::WebviewWindow) -> Result<Usage, String> {
    nest_only(&window)?;
    let windows = crate::usage::usage(app.clone());
    let budget = crate::settings::budget_cents(&app).map(|c| f64::from(c) / 100.0);
    let today = crate::history::today();
    let (since, day) = (today.plus(-(DAYS - 1)).iso(), today.iso());
    let (days, roosts) = blocking(move || {
        let store = open()?;
        let days = store.spend_days(&since).map_err(|e| e.to_string())?;
        let roosts = store.spend_roosts(&day).map_err(|e| e.to_string())?;
        Ok((days, roosts))
    })
    .await?;
    Ok(Usage {
        windows,
        subscription: days.iter().any(|(_, d)| d.subscription) || roosts.iter().any(|r| r.subscription),
        days: days
            .into_iter()
            .map(|(day, d)| DaySpend {
                day,
                usd: d.usd,
                tokens: d.tokens,
            })
            .collect(),
        roosts: roosts
            .into_iter()
            .map(|r| RoostSpend {
                name: roost_name(&r.roost),
                today: r.today,
                total: r.total,
            })
            .collect(),
        budget,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_roost_key_reads_as_its_name() {
        assert_eq!(roost_name("github:me/site"), "me/site");
        assert_eq!(roost_name("local:/home/me/Projects/site/.git"), "site");
        assert_eq!(roost_name("local:/home/me/notes"), "notes");
    }
}
