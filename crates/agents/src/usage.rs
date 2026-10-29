//! The subscription's usage, as each CLI hands it out. Never read through a token of ours: only
//! what the CLI itself reports.

use serde::Serialize;
use serde_json::Value;
use vults_protocol::AgentKind;

/// One rate-limit window ("5 hours", "a week") and how much of it is used.
#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
pub struct Window {
    pub agent: AgentKind,
    /// The window's length. Codex's slots say nothing about it: on some plans the first slot is
    /// the weekly window and there is no 5-hour one.
    pub minutes: u32,
    pub used_percent: u8,
    /// Epoch seconds.
    pub resets_at: Option<i64>,
}

/// A garbled payload (a ratio sent as a percentage, milliseconds sent as seconds) shows nothing
/// rather than a wrong gauge: a little over 100 is real overuse, far over it is not a percentage.
const MAX_PERCENT: f64 = 200.0;
/// No plan's window resets further out than this; a later time is not epoch seconds.
const MAX_RESET_AHEAD: i64 = 400 * 24 * 3600;

/// `now` is epoch seconds, to judge the reset time.
fn window(agent: AgentKind, minutes: u32, used: f64, resets_at: Option<i64>, now: i64) -> Option<Window> {
    if !(0.0..=MAX_PERCENT).contains(&used) {
        return None;
    }
    Some(Window {
        agent,
        minutes,
        used_percent: used.round().min(100.0) as u8,
        resets_at: resets_at.filter(|t| *t <= now + MAX_RESET_AHEAD),
    })
}

/// From the result of `account/rateLimits/read` (`codex app-server`), shortest window first.
/// `now` is epoch seconds.
pub fn codex(result: &Value, now: i64) -> Vec<Window> {
    // New servers identify the authoritative quota by bucket. Do not substitute another
    // model's allowance when a legacy top-level snapshot disagrees with it.
    let limits = result
        .get("rateLimitsByLimitId")
        .and_then(|all| all.get("codex"))
        .unwrap_or(&result["rateLimits"]);
    let mut windows: Vec<Window> = ["primary", "secondary"]
        .iter()
        .filter_map(|slot| {
            let w = &limits[*slot];
            window(
                AgentKind::Codex,
                u32::try_from(w["windowDurationMins"].as_u64()?).ok()?,
                w["usedPercent"].as_f64()?,
                w["resetsAt"].as_i64(),
                now,
            )
        })
        .collect();
    windows.sort_by_key(|w| w.minutes);
    windows
}

/// The session's cost so far, in US dollars, from Claude Code's statusLine input: what its tokens
/// would cost at API prices, which a subscription does not charge.
pub fn claude_cost(payload: &Value) -> Option<f64> {
    payload["cost"]["total_cost_usd"]
        .as_f64()
        .filter(|usd| usd.is_finite() && *usd >= 0.0)
}

/// The session's running time, including resumed runs, as the CLI reports it.
pub fn claude_elapsed(payload: &Value) -> Option<u64> {
    payload["cost"]["total_duration_ms"].as_u64()
}

/// The current context occupancy, not cumulative tokens or a subscription limit.
pub fn claude_context(payload: &Value) -> Option<f64> {
    payload["context_window"]["used_percentage"]
        .as_f64()
        .filter(|used| (0.0..=100.0).contains(used))
}

/// From Claude Code's statusLine input, relayed by the hook: `rate_limits.five_hour` and
/// `.seven_day`, each one there only while its window is open. `now` is epoch seconds.
pub fn claude(payload: &Value, now: i64) -> Vec<Window> {
    [("five_hour", 300), ("seven_day", 10080)]
        .iter()
        .filter_map(|(key, minutes)| {
            let w = &payload["rate_limits"][*key];
            window(
                AgentKind::Claude,
                *minutes,
                w["used_percentage"].as_f64()?,
                w["resets_at"].as_i64(),
                now,
            )
        })
        .collect()
}

/// Claude Code's read-only `get_usage` control response. Unlike statusLine, percentages are
/// named `utilization` and reset times are RFC 3339. Missing windows remain unknown.
pub fn claude_account(payload: &Value, now: i64) -> Vec<Window> {
    if payload["rate_limits_available"] != true {
        return Vec::new();
    }
    [("five_hour", 300), ("seven_day", 10080)]
        .into_iter()
        .filter_map(|(key, minutes)| {
            let value = &payload["rate_limits"][key];
            window(
                AgentKind::Claude,
                minutes,
                value["utilization"].as_f64()?,
                value["resets_at"].as_str().and_then(reset_seconds),
                now,
            )
        })
        .collect()
}

pub(crate) fn reset_seconds(raw: &str) -> Option<i64> {
    let value: toml::value::Datetime = raw.parse().ok()?;
    let date = value.date?;
    let time = value.time?;
    let day = vults_core::looks::Date::new(i32::from(date.year), date.month, date.day);
    if vults_core::looks::Date::from_days(day.days()) != day {
        return None;
    }
    let offset = match value.offset? {
        toml::value::Offset::Z => 0,
        toml::value::Offset::Custom { minutes } => i64::from(minutes) * 60,
    };
    Some(
        day.days() * 86400
            + i64::from(time.hour) * 3600
            + i64::from(time.minute) * 60
            + i64::from(time.second?)
            - offset,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// 2026-10-02, when the fixtures were recorded.
    const NOW: i64 = 1790900000;

    #[test]
    fn native_claude_allowance_requires_explicit_availability_and_preserves_units() {
        let report = json!({"rate_limits_available":true,"rate_limits":{
            "five_hour":{"utilization":0,"resets_at":null},
            "seven_day":{"utilization":12.4,"resets_at":"2026-10-10T13:30:00.123+01:30"},
            "seven_day_opus":{"utilization":99}
        }});
        let got = claude_account(&report, NOW);
        assert_eq!(
            got.len(),
            2,
            "Do not mix a model-scoped allowance into the general plan"
        );
        assert_eq!((got[0].used_percent, got[0].resets_at), (0, None));
        assert_eq!(got[1].used_percent, 12);
        assert_eq!(got[1].resets_at, reset_seconds("2026-10-10T12:00:00Z"));
        assert!(got[1].resets_at.is_some());
        assert!(
            claude_account(
                &json!({"rate_limits_available":false,"rate_limits":report["rate_limits"]}),
                NOW
            )
            .is_empty()
        );
        for invalid in [json!(null), json!(-1), json!("12"), json!(250)] {
            assert!(claude_account(&json!({"rate_limits_available":true,"rate_limits":{"five_hour":{"utilization":invalid}}}),NOW).is_empty());
        }
        for invalid in ["2026-02-30T12:00:00Z", "2026-10-10T12:00:00", "not a date"] {
            assert_eq!(reset_seconds(invalid), None);
        }
        let bucket = json!({"primary":{"usedPercent":0,"windowDurationMins":300}});
        let codex = codex(
            &json!({"rateLimitsByLimitId":{"codex":bucket},"rateLimits":{"primary":{"usedPercent":99,"windowDurationMins":300}}}),
            NOW,
        );
        assert_eq!(
            codex[0].used_percent, 0,
            "The keyed bucket outranks the legacy copy"
        );
    }

    #[test]
    fn implausible_values_show_nothing() {
        let line = |pct: f64, reset: i64| {
            let five = json!({ "used_percentage": pct, "resets_at": reset });
            claude(&json!({ "rate_limits": { "five_hour": five } }), NOW)
        };
        // A little past 100 is real overuse: the gauge is full.
        assert_eq!(line(150.0, NOW + 60)[0].used_percent, 100);
        assert!(line(250.0, NOW + 60).is_empty());
        assert!(line(-1.0, NOW + 60).is_empty());
        // Milliseconds where seconds belong: the window stays, its reset time goes.
        let w = &line(10.0, NOW * 1000)[0];
        assert_eq!((w.used_percent, w.resets_at), (10, None));
        // A reset already past is only stale, not wrong.
        assert_eq!(line(10.0, NOW - 60)[0].resets_at, Some(NOW - 60));
    }

    #[test]
    fn claude_session_metrics_keep_unknown_distinct_from_zero() {
        let start: Value =
            serde_json::from_str(include_str!("../tests/fixtures/claude-statusline-start.json")).unwrap();
        assert_eq!(claude_elapsed(&start), Some(705));
        assert_eq!(claude_context(&start), None);
        let report = |ms: Value, used: Value| json!({ "cost": { "total_duration_ms": ms }, "context_window": { "used_percentage": used } });
        let zero = report(json!(0), json!(0));
        assert_eq!(claude_elapsed(&zero), Some(0));
        assert_eq!(claude_context(&zero), Some(0.0));
        assert_eq!(claude_context(&report(json!(42), json!(37.5))), Some(37.5));
        for bad in [json!(null), json!(-1), json!(101), json!("42")] {
            assert_eq!(claude_context(&report(json!(42), bad)), None);
        }
        for bad in [json!(null), json!(-1), json!(1.5), json!("42")] {
            assert_eq!(claude_elapsed(&report(bad, json!(42))), None);
        }
    }

    #[test]
    fn claude_from_a_real_status_line() {
        // Claude Code 2.1.286: no usage before the session's first reply, then both windows.
        let mut lines = include_str!("../tests/fixtures/claude-statusline.jsonl").lines();
        let before: Value = serde_json::from_str(lines.next().unwrap()).unwrap();
        assert!(claude(&before, NOW).is_empty());
        let after: Value = serde_json::from_str(lines.next().unwrap()).unwrap();
        assert_eq!(
            claude(&after, NOW),
            vec![
                Window {
                    agent: AgentKind::Claude,
                    minutes: 300,
                    used_percent: 23,
                    resets_at: Some(1790993400)
                },
                Window {
                    agent: AgentKind::Claude,
                    minutes: 10080,
                    used_percent: 12,
                    resets_at: Some(1791493200)
                },
            ]
        );
        let half = json!({ "rate_limits": { "seven_day": { "used_percentage": 41.6 } } });
        assert_eq!(claude(&half, NOW)[0].used_percent, 42);
        assert_eq!(claude(&half, NOW)[0].resets_at, None);
    }

    #[test]
    fn codex_from_a_real_read() {
        // codex-cli 0.159.3, a "prolite" plan: the only window is the weekly one, in `primary`.
        let read: Value =
            serde_json::from_str(include_str!("../tests/fixtures/codex-rate-limits.json")).unwrap();
        assert_eq!(
            codex(&read["result"], NOW),
            vec![Window {
                agent: AgentKind::Codex,
                minutes: 10080,
                used_percent: 0,
                resets_at: Some(1791580268),
            }]
        );
    }

    #[test]
    fn codex_windows_go_by_length_not_slot() {
        let result = json!({ "rateLimits": {
            "primary": { "usedPercent": 41, "windowDurationMins": 10080, "resetsAt": 2 },
            "secondary": { "usedPercent": 130, "windowDurationMins": 300, "resetsAt": null },
        }});
        let got = codex(&result, NOW);
        assert_eq!(got.iter().map(|w| w.minutes).collect::<Vec<_>>(), [300, 10080]);
        assert_eq!(got[0].used_percent, 100);
        assert_eq!(got[0].resets_at, None);
        assert!(codex(&json!({ "rateLimits": null }), NOW).is_empty());
        assert!(codex(&json!({}), NOW).is_empty());
    }
}
