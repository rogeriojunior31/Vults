//! What the agents cost, and the user's daily budget (`docs/dev/plan-zeca.md` W9). Costs come from
//! the agents themselves: a task run's turns (the driver), and a watched Claude Code session's
//! statusLine. On a subscription they are what the tokens would cost at API prices.
//!
//! The budget is the user's own limit on what Vults starts: at 80 % a notification, and once it is
//! spent another, and no new session starts from here (a task's start, handoff, review, or a
//! `/continue`) until the next day. It never stops an agent, and never blocks or slows one Vults
//! only watches (rule 1).

use std::sync::Mutex;

use serde::Serialize;
use tauri::{AppHandle, Emitter};
use vults_protocol::AgentKind;
use vults_store::Store;
use vults_store::spend::Source;

/// The highest budget the Nest takes: $10,000 a day.
const MAX_CENTS: u32 = 1_000_000;

/// The day and the step (1: 80 %, 2: spent) last notified, so each is said once a day.
static TOLD: Mutex<Option<(String, u8)>> = Mutex::new(None);

fn open() -> Result<Store, String> {
    Store::open(&crate::history::dir()).map_err(|e| e.to_string())
}

pub fn today() -> String {
    crate::history::today().iso()
}

/// `subscription` when the agent's plan reports its usage windows (only a subscription has them);
/// empty when Vults can't tell.
pub fn plan(app: &AppHandle, agent: AgentKind) -> &'static str {
    if crate::usage::quota(app, agent).is_some() {
        "subscription"
    } else {
        ""
    }
}

/// Dollars as the Nest and the notifications write them.
fn money(usd: f64) -> String {
    format!("${usd:.2}")
}

fn budget(app: &AppHandle) -> Option<f64> {
    crate::settings::budget_cents(app).map(|c| f64::from(c) / 100.0)
}

/// How far into the budget: 0 below 80 %, 1 from 80 %, 2 spent.
fn step(spent: f64, budget: f64) -> u8 {
    if spent >= budget {
        2
    } else if spent >= budget * 0.8 {
        1
    } else {
        0
    }
}

/// A run's spend, from the driver: what it reported for a turn (`task:12`; a `/continue` is
/// `task:0`). Blocking.
pub fn run(app: &AppHandle, task: i64, roost: &str, plan: &str, usd: f64, tokens: u64) {
    let source = format!("task:{task}");
    let at = Source {
        day: &today(),
        source: &source,
        roost,
        plan,
    };
    let tokens = i64::try_from(tokens).unwrap_or(i64::MAX);
    if let Err(e) = open().and_then(|mut s| s.spend_add(&at, usd, tokens).map_err(|e| e.to_string())) {
        tracing::warn!(task, error = %e, "a run's spend not kept");
    }
    check(app);
}

/// A session's running total (Claude Code's): only what it grew by since Vults last saw it counts.
/// `roost` is the roost's key, or empty. Gives back what was added. Blocking.
pub fn session(app: &AppHandle, agent: AgentKind, id: &str, roost: &str, plan: &str, total: f64) -> f64 {
    let source = format!("{}:{id}", agent.name());
    let at = Source {
        day: &today(),
        source: &source,
        roost,
        plan,
    };
    let grew = match open().and_then(|mut s| s.spend_total(&at, total).map_err(|e| e.to_string())) {
        Ok(grew) => grew,
        Err(e) => {
            tracing::warn!(error = %e, "a session's spend not kept");
            0.0
        }
    };
    if grew > 0.0 {
        check(app);
    }
    grew
}

/// A watched session's cost from its statusLine, kept off the runtime loop. The roost is read from
/// its folder.
pub fn watched(
    app: &AppHandle,
    agent: AgentKind,
    id: String,
    cwd: Option<String>,
    plan: &'static str,
    total: f64,
) {
    let app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let roost = cwd
            .and_then(|c| crate::roosts::repo_of(std::path::Path::new(&c)))
            .map(|r| r.key())
            .unwrap_or_default();
        session(&app, agent, &id, &roost, plan, total);
    });
}

/// Says the budget's news once a day per step. Blocking.
fn check(app: &AppHandle) {
    let Some(budget) = budget(app) else {
        return;
    };
    let day = today();
    let Ok(spent) = open().and_then(|s| s.spend_day(&day).map_err(|e| e.to_string())) else {
        return;
    };
    let now = step(spent.usd, budget);
    let news = {
        let Ok(mut told) = TOLD.lock() else {
            return;
        };
        let before = told.as_ref().filter(|(d, _)| *d == day).map_or(0, |(_, s)| *s);
        if now > before {
            *told = Some((day, now));
        }
        now > before
    };
    if news {
        tracing::info!(step = now, "daily budget");
        let (title, body) = vults_core::i18n::budget(
            crate::settings::lang(app),
            now == 2,
            &money(spent.usd),
            &money(budget),
        );
        crate::notify::plain(app, title, body);
        let _ = app.emit("tasks-changed", ());
    }
}

/// Before Vults starts a session: refused once today's budget is spent. A day Vults can't read
/// starts (the user clicked; the ledger's fault is not theirs). Blocking.
pub fn refuse(app: &AppHandle) -> Result<(), String> {
    let Some(budget) = budget(app) else {
        return Ok(());
    };
    match open().and_then(|s| s.spend_day(&today()).map_err(|e| e.to_string())) {
        Ok(spent) if step(spent.usd, budget) == 2 => {
            Err("Today's budget is spent: Vults starts no new session until tomorrow.".into())
        }
        Ok(_) => Ok(()),
        Err(e) => {
            tracing::warn!(error = %e, "today's spend not read: the start goes on");
            Ok(())
        }
    }
}

/// Today's spend and the budget, for the Tasks tab.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Today {
    pub usd: f64,
    pub tokens: i64,
    pub budget: Option<f64>,
    /// Some of it was on a subscription: "at API prices".
    pub subscription: bool,
}

#[tauri::command]
pub async fn spend_today(app: AppHandle) -> Result<Today, String> {
    let budget = budget(&app);
    tauri::async_runtime::spawn_blocking(move || {
        let day = open()?.spend_day(&today()).map_err(|e| e.to_string())?;
        Ok(Today {
            usd: day.usd,
            tokens: day.tokens,
            budget,
            subscription: day.subscription,
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

/// The daily budget in cents, or none for no budget; from the Nest only.
#[tauri::command]
pub async fn budget_set(
    app: AppHandle,
    window: tauri::WebviewWindow,
    cents: Option<u32>,
) -> Result<(), String> {
    if window.label() != crate::NEST {
        return Err("only the Nest can do this".into());
    }
    let cents = cents.filter(|c| *c > 0);
    if cents.is_some_and(|c| c > MAX_CENTS) {
        return Err("That budget is too high.".into());
    }
    crate::settings::edit(&app, |s| s.budget_daily_cents = cents)?;
    tracing::info!(cents, "daily budget");
    // A budget raised (or removed) may be told again; one lowered past today's spend says so now.
    if let Ok(mut told) = TOLD.lock() {
        *told = None;
    }
    let a = app.clone();
    let _ = tauri::async_runtime::spawn_blocking(move || check(&a)).await;
    let _ = app.emit("tasks-changed", ());
    Ok(())
}

/// Each roost's costs, by its key.
pub fn roosts() -> std::collections::HashMap<String, vults_store::spend::Roost> {
    open()
        .and_then(|s| s.spend_roosts(&today()).map_err(|e| e.to_string()))
        .unwrap_or_default()
        .into_iter()
        .map(|r| (r.roost.clone(), r))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_budget_warns_at_80_percent_and_is_spent_at_100() {
        assert_eq!(step(3.99, 5.0), 0);
        assert_eq!(step(4.0, 5.0), 1);
        assert_eq!(step(4.99, 5.0), 1);
        assert_eq!(step(5.0, 5.0), 2);
        assert_eq!(step(7.0, 5.0), 2);
        assert_eq!(money(4.1), "$4.10");
    }
}
