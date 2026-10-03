//! The subscription's usage, as each CLI hands it out. Never read through a token of ours: only
//! what the CLI itself reports.

use serde::Serialize;
use serde_json::Value;
use vultures_ai_protocol::AgentKind;

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

/// From the result of `account/rateLimits/read` (`codex app-server`), shortest window first.
pub fn codex(result: &Value) -> Vec<Window> {
    let limits = &result["rateLimits"];
    let mut windows: Vec<Window> = ["primary", "secondary"]
        .iter()
        .filter_map(|slot| {
            let w = &limits[*slot];
            Some(Window {
                agent: AgentKind::Codex,
                minutes: u32::try_from(w["windowDurationMins"].as_u64()?).ok()?,
                used_percent: w["usedPercent"].as_u64()?.min(100) as u8,
                resets_at: w["resetsAt"].as_i64(),
            })
        })
        .collect();
    windows.sort_by_key(|w| w.minutes);
    windows
}

/// From Claude Code's statusLine input, relayed by the hook: `rate_limits.five_hour` and
/// `.seven_day`, each one there only while its window is open.
pub fn claude(payload: &Value) -> Vec<Window> {
    [("five_hour", 300), ("seven_day", 10080)]
        .iter()
        .filter_map(|(key, minutes)| {
            let w = &payload["rate_limits"][*key];
            Some(Window {
                agent: AgentKind::Claude,
                minutes: *minutes,
                used_percent: w["used_percentage"].as_f64()?.round().clamp(0.0, 100.0) as u8,
                resets_at: w["resets_at"].as_i64(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn claude_from_a_real_status_line() {
        // Claude Code 2.1.286: no usage before the session's first reply, then both windows.
        let mut lines = include_str!("../tests/fixtures/claude-statusline.jsonl").lines();
        let before: Value = serde_json::from_str(lines.next().unwrap()).unwrap();
        assert!(claude(&before).is_empty());
        let after: Value = serde_json::from_str(lines.next().unwrap()).unwrap();
        assert_eq!(
            claude(&after),
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
        assert_eq!(claude(&half)[0].used_percent, 42);
        assert_eq!(claude(&half)[0].resets_at, None);
    }

    #[test]
    fn codex_from_a_real_read() {
        // codex-cli 0.159.3, a "prolite" plan: the only window is the weekly one, in `primary`.
        let read: Value =
            serde_json::from_str(include_str!("../tests/fixtures/codex-rate-limits.json")).unwrap();
        assert_eq!(
            codex(&read["result"]),
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
        let got = codex(&result);
        assert_eq!(got.iter().map(|w| w.minutes).collect::<Vec<_>>(), [300, 10080]);
        assert_eq!(got[0].used_percent, 100);
        assert_eq!(got[0].resets_at, None);
        assert!(codex(&json!({ "rateLimits": null })).is_empty());
        assert!(codex(&json!({})).is_empty());
    }
}
