//! Local usage totals survive transcript retention; daily app attribution starts at migration 24.

use serde::{Deserialize, Serialize};

use super::{Day, Store};
use crate::Error;
use vults_core::looks::Date;

pub(crate) const TABLE: &str = "CREATE TABLE dictation_app_days (
    day TEXT NOT NULL, app TEXT NOT NULL, words INTEGER NOT NULL CHECK(words > 0),
    PRIMARY KEY(day, app));";

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Period {
    Week,
    Month,
    All,
}

#[derive(Debug, Serialize)]
pub struct Usage {
    pub words: i64,
    pub secs: f64,
    pub count: i64,
    pub saved_minutes: Option<f64>,
    pub wpm: Option<f64>,
}

impl Usage {
    fn of(day: Day) -> Self {
        Self {
            words: day.words,
            secs: day.secs,
            count: day.count,
            saved_minutes: (day.count > 0).then(|| (day.words as f64 / 45.0 - day.secs / 60.0).max(0.0)),
            wpm: (day.count > 0).then(|| {
                if day.secs < 1.0 {
                    0.0
                } else {
                    day.words as f64 * 60.0 / day.secs
                }
            }),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct DailyUsage {
    pub day: String,
    pub words: i64,
    pub count: i64,
}
#[derive(Debug, Serialize)]
pub struct AppUsage {
    pub app: Option<String>,
    pub words: i64,
}
#[derive(Debug, Serialize)]
pub struct Statistics {
    pub period: Period,
    pub from: Option<String>,
    pub through: String,
    pub usage: Usage,
    pub days: Vec<DailyUsage>,
    pub chart_limited: bool,
    pub apps: Vec<AppUsage>,
    pub apps_complete: bool,
}

impl Store {
    pub fn dictation_usage(&self, from: Date, through: Date) -> Result<Usage, Error> {
        Ok(Usage::of(self.conn.query_row(
            "SELECT COALESCE(SUM(words),0), COALESCE(SUM(secs),0), COALESCE(SUM(count),0)
             FROM dictation_days WHERE day >= ?1 AND day <= ?2",
            [from.iso(), through.iso()],
            |r| {
                Ok(Day {
                    words: r.get(0)?,
                    secs: r.get(1)?,
                    count: r.get(2)?,
                })
            },
        )?))
    }

    pub fn dictation_statistics(&self, period: Period, today: Date) -> Result<Statistics, Error> {
        let snapshot = self.conn.unchecked_transaction()?;
        let days = self.dictation_days()?;
        let from = match period {
            Period::Week => Some(today.plus(-6)),
            Period::Month => Some(today.plus(-29)),
            Period::All => None,
        };
        let through = if period == Period::All {
            days.keys()
                .next_back()
                .and_then(|d| Date::parse(d))
                .unwrap_or(today)
                .max(today)
        } else {
            today
        };
        let usage = self.dictation_usage(from.unwrap_or(Date::new(1970, 1, 1)), through)?;
        let chart_from = from
            .or_else(|| days.keys().next().and_then(|d| Date::parse(d)))
            .unwrap_or(through.plus(-6));
        // ponytail: all-time totals stay exact; a daily chart is bounded to the latest 90 calendar days.
        let chart_start = chart_from.max(through.plus(-89));
        let series = (0..=through.days() - chart_start.days())
            .map(|i| {
                let day = chart_start.plus(i).iso();
                let value = days.get(&day);
                DailyUsage {
                    day,
                    words: value.map_or(0, |v| v.words),
                    count: value.map_or(0, |v| v.count),
                }
            })
            .collect();
        let mut apps = if period == Period::All {
            self.dictation_apps()?
        } else {
            let mut stmt = self.conn.prepare(
                "SELECT app, SUM(words) FROM dictation_app_days WHERE day >= ?1 AND day <= ?2 GROUP BY app",
            )?;
            stmt.query_map([from.expect("bounded period").iso(), through.iso()], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })?
            .collect::<Result<_, _>>()?
        };
        let attributed: i64 = apps.values().sum();
        if period == Period::All && usage.words > attributed {
            *apps.entry(String::new()).or_default() += usage.words - attributed;
        }
        let mut apps: Vec<_> = apps
            .into_iter()
            .map(|(app, words)| AppUsage {
                app: (!app.is_empty()).then_some(app),
                words,
            })
            .collect();
        apps.sort_by(|a, b| b.words.cmp(&a.words).then_with(|| a.app.cmp(&b.app)));
        apps.truncate(8);
        let apps_complete = period == Period::All || attributed == usage.words;
        let result = Statistics {
            period,
            from: from.map(Date::iso),
            through: through.iso(),
            usage,
            days: series,
            chart_limited: chart_start > chart_from,
            apps,
            apps_complete,
        };
        snapshot.commit()?;
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn figures_use_measured_duration_and_floor_savings_and_short_audio() {
        for (words, secs, count, saved, wpm) in [
            (0, 0.0, 0, None, None),
            (45, 30.0, 1, Some(0.5), Some(90.0)),
            (45, 120.0, 1, Some(0.0), Some(22.5)),
            (1, 0.5, 1, Some(1.0 / 45.0 - 0.5 / 60.0), Some(0.0)),
        ] {
            let usage = Usage::of(Day { words, secs, count });
            assert_eq!(usage.saved_minutes, saved);
            assert_eq!(usage.wpm, wpm);
        }
    }

    #[test]
    fn periods_use_usage_totals_after_erasure_and_report_legacy_app_gaps() {
        let mut store = Store::open(&crate::temp("dictation-statistics")).unwrap();
        let today = Date::new(2026, 10, 18);
        let mut entry = super::super::tests::sample(1_792_281_600);
        for (ago, words, app) in [
            (0, 90, Some("Code")),
            (6, 45, None),
            (7, 30, Some("Kate")),
            (29, 60, Some("Code")),
            (30, 75, Some("Kate")),
        ] {
            entry.words = words;
            entry.app = app.map(str::to_owned);
            store
                .dictation_add(
                    &entry,
                    today.plus(-ago),
                    super::super::Retention::NotKept,
                    entry.at,
                )
                .unwrap();
        }
        let week = store.dictation_statistics(Period::Week, today).unwrap();
        assert_eq!((week.usage.words, week.days.len()), (135, 7));
        assert!(week.apps_complete);
        assert_eq!(week.apps[0].app.as_deref(), Some("Code"));
        assert_eq!(week.apps[1].app, None);
        assert_eq!(
            store
                .dictation_statistics(Period::Month, today)
                .unwrap()
                .usage
                .words,
            225
        );
        let all = store.dictation_statistics(Period::All, today).unwrap();
        assert_eq!(all.usage.words, 300);
        assert!(store.dictations(1).unwrap().is_empty());
        store
            .conn
            .execute(
                "DELETE FROM dictation_app_days WHERE day = ?1",
                [today.plus(-6).iso()],
            )
            .unwrap();
        assert!(
            !store
                .dictation_statistics(Period::Week, today)
                .unwrap()
                .apps_complete
        );
        assert!(
            store
                .dictation_statistics(Period::All, today)
                .unwrap()
                .apps_complete
        );
        store
            .conn
            .execute("INSERT INTO dictation_days VALUES ('2025-01-01', 10, 1, 1)", [])
            .unwrap();
        let all = store.dictation_statistics(Period::All, today).unwrap();
        assert!(all.chart_limited);
        assert_eq!((all.days.len(), all.usage.words), (90, 310));
        let week_words = store
            .dictation_usage(today.monday(), today.monday().plus(6))
            .unwrap();
        assert_eq!(week_words.words, 135);
    }
}
