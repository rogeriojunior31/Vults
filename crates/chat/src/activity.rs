//! Bounded, display-only turn activity; restoring it never dispatches a tool or a decision.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Event {
    Reasoning {
        text: String,
    },
    Start {
        id: String,
        tool: String,
        detail: String,
    },
    End {
        id: String,
        ok: bool,
    },
    Decision {
        tool: String,
        target: String,
        answer: String,
        by: String,
    },
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Activity {
    pub reasoning: String,
    pub tools: Vec<Tool>,
    pub decisions: Vec<Decision>,
    pub truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub id: String,
    pub tool: String,
    pub detail: String,
    pub ok: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Decision {
    pub tool: String,
    pub target: String,
    pub answer: String,
    pub by: String,
}

fn bounded(text: &str, limit: usize) -> String {
    text.chars().take(limit).collect()
}

impl Activity {
    pub fn apply(&mut self, event: &Event) {
        match event {
            Event::Reasoning { text } => {
                let remaining = 32_768usize.saturating_sub(self.reasoning.chars().count());
                self.reasoning.push_str(&bounded(text, remaining));
                self.truncated |= text.chars().count() > remaining;
            }
            Event::Start { id, tool, detail } => {
                if self.tools.iter().any(|t| t.id == *id) {
                    return;
                }
                if self.tools.len() == 128 {
                    self.truncated = true;
                    return;
                }
                self.truncated |= detail.chars().count() > 16_384;
                self.tools.push(Tool {
                    id: bounded(id, 512),
                    tool: bounded(tool, 512),
                    detail: bounded(detail, 16_384),
                    ok: None,
                });
            }
            Event::End { id, ok } => {
                if let Some(t) = self.tools.iter_mut().find(|t| t.id == *id) {
                    t.ok = Some(*ok);
                }
            }
            Event::Decision {
                tool,
                target,
                answer,
                by,
            } => {
                if self.decisions.len() == 128 {
                    self.truncated = true;
                    return;
                }
                self.truncated |= target.chars().count() > 16_384;
                self.decisions.push(Decision {
                    tool: bounded(tool, 512),
                    target: bounded(target, 16_384),
                    answer: bounded(answer, 32),
                    by: bounded(by, 32),
                });
            }
        }
    }
    /// Like traces and the journal, persistent supporting details omit recognized secrets.
    pub fn for_history(&self) -> Self {
        let mut saved = self.clone();
        saved.reasoning = vults_core::redact::secrets(&saved.reasoning);
        for tool in &mut saved.tools {
            tool.detail = vults_core::redact::secrets(&tool.detail);
            tool.tool = vults_core::redact::secrets(&tool.tool);
        }
        for decision in &mut saved.decisions {
            decision.target = vults_core::redact::secrets(&decision.target);
            decision.tool = vults_core::redact::secrets(&decision.tool);
        }
        saved
    }

    pub fn is_empty(&self) -> bool {
        self.reasoning.is_empty() && self.tools.is_empty() && self.decisions.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bounded_history_keeps_completion_and_never_replays_actions() {
        let mut activity = Activity::default();
        activity.apply(&Event::Reasoning {
            text: "\u{00e9}".repeat(40_000),
        });
        assert_eq!(activity.reasoning.chars().count(), 32_768);
        for n in 0..130 {
            activity.apply(&Event::Start {
                id: n.to_string(),
                tool: "Read".into(),
                detail: "file.rs".into(),
            });
        }
        activity.apply(&Event::End {
            id: "0".into(),
            ok: false,
        });
        assert_eq!(activity.tools.len(), 128);
        assert_eq!(activity.tools[0].ok, Some(false));
        assert!(activity.truncated);
        activity.apply(&Event::Decision {
            tool: "Bash".into(),
            target: "API_TOKEN=private command".into(),
            answer: "allow".into(),
            by: "human".into(),
        });
        let saved = activity.for_history();
        assert_eq!(saved.decisions[0].target, "API_TOKEN=[redacted] command");
        assert_eq!(activity.decisions[0].target, "API_TOKEN=private command");
        let restored: Activity = serde_json::from_str(&serde_json::to_string(&activity).unwrap()).unwrap();
        assert_eq!(restored.tools[0].ok, Some(false));
    }
}
