//! The weekly recap and the activity grid, summed from the local history (docs/dev/plan-activity.md).
//! Pure: the app reads the files and gives the entries, the dates and the UTC offset. Every number is
//! a count or a sum (ADR 0012); the words come from `i18n`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use vults_protocol::AgentKind;

use crate::i18n;
use crate::looks::Date;
use crate::turns::Turn;

const VERSION: u32 = 1;
/// The grid's weeks, the current one included: a year, as GitHub's.
pub const GRID_WEEKS: i64 = 53;

/// A finished turn, as the history keeps it (one line of `history.jsonl`).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub v: u32,
    /// When it ended, in Unix seconds.
    pub end: i64,
    /// The local day it ended (`2026-10-09`).
    pub day: String,
    pub secs: u64,
    pub agent: AgentKind,
    pub project: String,
    pub steps: u32,
    pub commands: u32,
    pub files: u32,
    pub added: u32,
    pub removed: u32,
    pub allowed: u32,
    pub denied: u32,
    pub answered: u32,
    pub questions: u32,
    pub failed: bool,
}

impl Entry {
    /// The turn, ended `turn.ago` before `now` (Unix seconds), on the local day of that instant.
    pub fn new(turn: &Turn, now: i64, offset: i32) -> Self {
        let end = now - i64::try_from(turn.ago.as_secs()).unwrap_or(0);
        Self {
            v: VERSION,
            end,
            day: Date::of(end, offset).iso(),
            secs: turn.secs,
            agent: turn.agent,
            project: turn.project.clone(),
            steps: turn.steps,
            commands: turn.commands,
            files: turn.files,
            added: turn.added,
            removed: turn.removed,
            allowed: turn.allowed,
            denied: turn.denied,
            answered: turn.answered,
            questions: turn.questions,
            failed: turn.failed,
        }
    }
}

/// One week of the history, Monday to Sunday.
#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct WeekView {
    /// Its Monday and Sunday (`2026-10-05`, `2026-10-11`).
    pub monday: String,
    pub sunday: String,
    /// Time with an agent at work: overlapping turns count once.
    pub active_secs: u64,
    pub turns: u32,
    pub steps: u32,
    pub commands: u32,
    pub files: u32,
    pub added: u32,
    pub removed: u32,
    pub allowed: u32,
    pub denied: u32,
    pub answered: u32,
    pub questions: u32,
    pub failed: u32,
    /// The agent and the project with the most turns; none in an empty week.
    #[cfg_attr(test, ts(as = "Option<crate::view::ts::AgentKind>"))]
    pub top_agent: Option<AgentKind>,
    pub top_project: Option<String>,
    /// The day with the most active time.
    pub busiest_day: Option<String>,
    pub longest_secs: u64,
    /// Active time per day, Monday first.
    pub days: Vec<u64>,
    /// The week in one sentence.
    pub headline: String,
}

/// A day of the grid.
#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct GridDay {
    pub day: String,
    pub turns: u32,
    pub secs: u64,
    /// 0 for no turn, then 1 to 4 by the quartiles of the year's active days (as GitHub's).
    pub level: u8,
}

/// A day's totals, as `days.json` keeps them.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DayTotal {
    pub turns: u32,
    pub secs: u64,
}

/// The week starting on `monday`, from the entries of its days.
pub fn week(lang: i18n::Lang, entries: &[Entry], monday: Date) -> WeekView {
    let monday = monday.monday();
    let sunday = monday.plus(6);
    let mine: Vec<(&Entry, Date)> = entries
        .iter()
        .filter_map(|e| Date::parse(&e.day).map(|d| (e, d)))
        .filter(|(_, d)| *d >= monday && *d <= sunday)
        .collect();
    let sum = |f: fn(&Entry) -> u32| mine.iter().map(|(e, _)| f(e)).fold(0u32, u32::saturating_add);
    let mut days = vec![0u64; 7];
    let mut agents: BTreeMap<AgentKind, u32> = BTreeMap::new();
    let mut projects: BTreeMap<&str, u32> = BTreeMap::new();
    for (e, d) in &mine {
        days[usize::from(d.weekday())] += e.secs;
        *agents.entry(e.agent).or_default() += 1;
        if !e.project.is_empty() {
            *projects.entry(e.project.as_str()).or_default() += 1;
        }
    }
    // Most turns wins; a tie goes to the first by name, so the recap never flickers.
    fn top<K: Ord + Copy>(m: &BTreeMap<K, u32>) -> Option<K> {
        m.iter()
            .max_by(|a, b| a.1.cmp(b.1).then(b.0.cmp(a.0)))
            .map(|(k, _)| *k)
    }
    let busiest = (0..7)
        .filter(|&i| days[i] > 0)
        .max_by(|&a, &b| days[a].cmp(&days[b]).then(b.cmp(&a)));
    let mut view = WeekView {
        monday: monday.iso(),
        sunday: sunday.iso(),
        active_secs: merged_secs(mine.iter().map(|(e, _)| *e)),
        turns: u32::try_from(mine.len()).unwrap_or(u32::MAX),
        steps: sum(|e| e.steps),
        commands: sum(|e| e.commands),
        files: sum(|e| e.files),
        added: sum(|e| e.added),
        removed: sum(|e| e.removed),
        allowed: sum(|e| e.allowed),
        denied: sum(|e| e.denied),
        answered: sum(|e| e.answered),
        questions: sum(|e| e.questions),
        failed: sum(|e| u32::from(e.failed)),
        top_agent: top(&agents),
        top_project: top(&projects).map(str::to_string),
        busiest_day: busiest.map(|i| monday.plus(i as i64).iso()),
        longest_secs: mine.iter().map(|(e, _)| e.secs).max().unwrap_or(0),
        days,
        headline: String::new(),
    };
    view.headline = i18n::recap(lang, &view);
    view
}

/// The Mondays of the weeks that have a turn, newest first.
pub fn weeks_with_turns(entries: &[Entry]) -> Vec<Date> {
    let mut mondays: Vec<Date> = entries
        .iter()
        .filter_map(|e| Date::parse(&e.day))
        .map(Date::monday)
        .collect();
    mondays.sort_unstable_by(|a, b| b.cmp(a));
    mondays.dedup();
    mondays
}

/// Time covered by the turns, each from `end - secs` to `end`, overlaps counted once.
fn merged_secs<'a>(entries: impl Iterator<Item = &'a Entry>) -> u64 {
    let mut spans: Vec<(i64, i64)> = entries
        .map(|e| (e.end - i64::try_from(e.secs).unwrap_or(0), e.end))
        .filter(|(start, end)| end > start)
        .collect();
    spans.sort_unstable();
    let mut total = 0i64;
    let mut current: Option<(i64, i64)> = None;
    for (start, end) in spans {
        current = match current {
            Some((s, e)) if start <= e => Some((s, e.max(end))),
            Some((s, e)) => {
                total += e - s;
                Some((start, end))
            }
            None => Some((start, end)),
        };
    }
    if let Some((s, e)) = current {
        total += e - s;
    }
    u64::try_from(total).unwrap_or(0)
}

/// The grid's days, from the Monday 52 weeks before `today`'s week to `today`, with their levels.
pub fn grid(days: &BTreeMap<String, DayTotal>, today: Date) -> Vec<GridDay> {
    let first = today.monday().plus(-7 * (GRID_WEEKS - 1));
    let totals: Vec<(Date, DayTotal)> = (0..=today.days() - first.days())
        .map(|i| {
            let d = first.plus(i);
            (d, days.get(&d.iso()).copied().unwrap_or_default())
        })
        .collect();
    let mut active: Vec<u64> = totals.iter().map(|(_, t)| t.secs).filter(|s| *s > 0).collect();
    active.sort_unstable();
    let at = |q: usize| {
        active
            .get((active.len() * q / 4).min(active.len().saturating_sub(1)))
            .copied()
            .unwrap_or(0)
    };
    let (q1, q2, q3) = (at(1), at(2), at(3));
    totals
        .into_iter()
        .map(|(d, t)| GridDay {
            day: d.iso(),
            turns: t.turns,
            secs: t.secs,
            level: match t.secs {
                0 if t.turns == 0 => 0,
                s if s <= q1 => 1,
                s if s <= q2 => 2,
                s if s <= q3 => 3,
                _ => 4,
            },
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(day: &str, end: i64, secs: u64, agent: AgentKind, project: &str) -> Entry {
        Entry {
            v: 1,
            end,
            day: day.into(),
            secs,
            agent,
            project: project.into(),
            steps: 2,
            commands: 1,
            files: 1,
            added: 5,
            removed: 2,
            allowed: 1,
            denied: 0,
            answered: 0,
            questions: 0,
            failed: false,
        }
    }

    /// 2026-10-06 (a Tuesday) 10:00 UTC.
    const TUE: i64 = 1_791_280_800;

    #[test]
    fn overlapping_turns_count_once() {
        let es = [
            entry("2026-10-06", TUE, 3600, AgentKind::Claude, "site"),
            // Inside the first.
            entry("2026-10-06", TUE - 600, 600, AgentKind::Codex, "api"),
            // Half over its end.
            entry("2026-10-06", TUE + 1800, 3600, AgentKind::Claude, "site"),
            // Apart.
            entry("2026-10-06", TUE + 10_000, 100, AgentKind::Claude, "site"),
        ];
        let w = week(i18n::Lang::En, &es, Date::new(2026, 10, 6));
        assert_eq!(w.active_secs, 3600 + 1800 + 100);
        assert_eq!(w.turns, 4);
        assert_eq!((w.added, w.removed, w.commands), (20, 8, 4));
        assert_eq!(w.top_agent, Some(AgentKind::Claude));
        assert_eq!(w.top_project.as_deref(), Some("site"));
        assert_eq!(w.busiest_day.as_deref(), Some("2026-10-06"));
        assert_eq!(w.longest_secs, 3600);
        assert_eq!(w.days[1], 3600 + 600 + 3600 + 100, "per day, as the turns ran");
        assert_eq!(
            (w.monday.as_str(), w.sunday.as_str()),
            ("2026-10-05", "2026-10-11")
        );
        assert_eq!(w.headline, "4 turns, 1 h 31 min with your agents, most on site.");
    }

    #[test]
    fn the_week_is_monday_to_sunday_by_local_day() {
        let es = [
            entry("2026-10-04", 0, 60, AgentKind::Claude, "a"), // Sunday before
            entry("2026-10-05", 0, 60, AgentKind::Claude, "a"), // Monday
            entry("2026-10-11", 0, 60, AgentKind::Claude, "a"), // Sunday
            entry("2026-10-12", 0, 60, AgentKind::Claude, "a"), // Monday after
        ];
        let w = week(i18n::Lang::En, &es, Date::new(2026, 10, 8));
        assert_eq!(w.turns, 2);
        assert_eq!(w.days[0] + w.days[6], 120);
    }

    #[test]
    fn an_empty_week_says_so() {
        let w = week(i18n::Lang::En, &[], Date::new(2026, 10, 5));
        assert_eq!(
            (w.turns, w.active_secs, w.top_agent, w.busiest_day),
            (0, 0, None, None)
        );
        assert_eq!(w.headline, "No agent turns that week.");
    }

    #[test]
    fn the_weeks_with_turns_newest_first() {
        let es = [
            entry("2026-09-29", 0, 1, AgentKind::Claude, ""),
            entry("2026-10-07", 0, 1, AgentKind::Claude, ""),
            entry("2026-10-06", 0, 1, AgentKind::Claude, ""),
            entry("not a day", 0, 1, AgentKind::Claude, ""),
        ];
        assert_eq!(
            weeks_with_turns(&es),
            [Date::new(2026, 10, 5), Date::new(2026, 9, 28)]
        );
    }

    #[test]
    fn the_grid_is_a_year_to_today_with_quartile_levels() {
        let today = Date::new(2026, 10, 9);
        let mut days = BTreeMap::new();
        for (i, secs) in [100u64, 200, 300, 400, 500, 600, 700, 800].iter().enumerate() {
            days.insert(
                today.plus(-(i as i64)).iso(),
                DayTotal {
                    turns: 1,
                    secs: *secs,
                },
            );
        }
        // Older than the grid: left out.
        days.insert(today.plus(-400).iso(), DayTotal { turns: 9, secs: 9999 });
        let g = grid(&days, today);
        assert_eq!(
            g.first().unwrap().day,
            "2025-10-06",
            "a Monday, 52 weeks before this one"
        );
        assert_eq!(g.last().unwrap().day, "2026-10-09");
        assert_eq!(g.len(), 52 * 7 + 5);
        let level = |d: Date| g.iter().find(|x| x.day == d.iso()).unwrap().level;
        assert_eq!(level(today.plus(-20)), 0);
        assert_eq!(level(today), 1, "100 s, the least");
        assert_eq!(level(today.plus(-7)), 4, "800 s, the most");
        assert_eq!(level(today.plus(-3)), 2, "400 s, the second quartile");
        assert!(g.iter().all(|d| d.level <= 4));
    }
}
