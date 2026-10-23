//! Explicit Chat references select read-only flock data, never an execution folder or a card.

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Target {
    Session { agent: String, id: String },
    Project { cwd: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Mention {
    pub token: String,
    pub target: Target,
}

pub fn validate(mentions: &[Mention]) -> Result<(), String> {
    if mentions.len() > 8
        || mentions.iter().any(|m| {
            if m.token.len() > 4096 {
                return true;
            }
            let valid_token = m
                .token
                .strip_prefix('@')
                .and_then(|s| serde_json::from_str::<String>(s).ok());
            valid_token.is_none_or(|s| s.is_empty() || s.len() > 4096)
                || match &m.target {
                    Target::Session { agent, id } => {
                        agent.is_empty() || agent.len() > 64 || id.is_empty() || id.len() > 4096
                    }
                    Target::Project { cwd } => cwd.len() > 4096 || !std::path::Path::new(cwd).is_absolute(),
                }
        })
    {
        return Err("Chat references are invalid or too large.".into());
    }
    Ok(())
}

impl Target {
    pub fn matches(&self, session: &Value) -> bool {
        match self {
            Self::Session { agent, id } => session["agent"] == *agent && session["id"] == *id,
            Self::Project { cwd } => session["cwd"] == *cwd,
        }
    }
}

/// Missing selections fail before a turn starts; edited-away tokens are ordinary draft text.
pub fn resolve(text: &str, mentions: Vec<Mention>, view: &Value) -> Result<Option<Vec<Target>>, String> {
    validate(&mentions)?;
    if text
        .match_indices("@\"")
        .any(|(i, _)| !mentions.iter().any(|m| text[i..].starts_with(&m.token)))
    {
        return Err("Choose flock references from the @ suggestions again before sending.".into());
    }
    let mut targets = Vec::new();
    for m in mentions.into_iter().filter(|m| text.contains(&m.token)) {
        if !view["sessions"]
            .as_array()
            .is_some_and(|sessions| sessions.iter().any(|s| m.target.matches(s)))
        {
            return Err("A referenced session or project is no longer in the flock. Remove its reference and choose again.".into());
        }
        if !targets.contains(&m.target) {
            targets.push(m.target);
        }
    }
    Ok((!targets.is_empty()).then_some(targets))
}

pub fn scoped(mut view: Value, targets: &[Target]) -> Value {
    if let Some(sessions) = view["sessions"].as_array_mut() {
        sessions.retain(|s| targets.iter().any(|t| t.matches(s)));
    }
    view
}

/// API tools refresh their data, while retaining this turn's explicit selection.
#[derive(Debug)]
pub struct Source {
    pub source: std::sync::Arc<dyn crate::flock_tools::Source>,
    pub targets: Vec<Target>,
}

impl crate::flock_tools::Source for Source {
    fn github_merge_prepare(
        &self,
        action: vults_connectors::github::MergeControl,
    ) -> vults_agents::driver::BoxFuture<'static, Result<Value, String>> {
        self.source.github_merge_prepare(action)
    }
    fn github_merge_execute(
        &self,
        action: vults_connectors::github::MergeControl,
        preview: Value,
        hash: String,
    ) -> vults_agents::driver::BoxFuture<'static, Result<Value, String>> {
        self.source.github_merge_execute(action, preview, hash)
    }
    fn github_actions_prepare(
        &self,
        action: vults_connectors::github::ActionsControl,
    ) -> vults_agents::driver::BoxFuture<'static, Result<Value, String>> {
        self.source.github_actions_prepare(action)
    }
    fn github_actions_execute(
        &self,
        action: vults_connectors::github::ActionsControl,
        preview: Value,
        hash: String,
    ) -> vults_agents::driver::BoxFuture<'static, Result<Value, String>> {
        self.source.github_actions_execute(action, preview, hash)
    }
    fn github_pull_request_prepare(
        &self,
        action: vults_connectors::github::PullRequestWrite,
    ) -> vults_agents::driver::BoxFuture<'static, Result<Value, String>> {
        self.source.github_pull_request_prepare(action)
    }
    fn github_pull_request_execute(
        &self,
        action: vults_connectors::github::PullRequestWrite,
        preview: Value,
        hash: String,
    ) -> vults_agents::driver::BoxFuture<'static, Result<Value, String>> {
        self.source.github_pull_request_execute(action, preview, hash)
    }
    fn github_issue_write_prepare(
        &self,
        action: vults_connectors::github::IssueWrite,
    ) -> vults_agents::driver::BoxFuture<'static, Result<Value, String>> {
        self.source.github_issue_write_prepare(action)
    }
    fn github_issue_write_execute(
        &self,
        action: vults_connectors::github::IssueWrite,
        preview: Value,
        request_hash: String,
    ) -> vults_agents::driver::BoxFuture<'static, Result<Value, String>> {
        self.source
            .github_issue_write_execute(action, preview, request_hash)
    }
    fn github_snapshot_allowed(&self) -> bool {
        self.source.github_snapshot_allowed()
    }
    fn github_work_item_allowed(&self) -> bool {
        self.source.github_work_item_allowed()
    }
    fn github_work_item(
        &self,
        repository: String,
        number: u32,
        cursors: crate::flock_tools::GithubCursors,
    ) -> vults_agents::driver::BoxFuture<'static, Result<Value, String>> {
        self.source.github_work_item(repository, number, cursors)
    }
    fn github_search_repositories(
        &self,
        query: String,
        cursor: Option<String>,
    ) -> vults_agents::driver::BoxFuture<'static, Result<Value, String>> {
        self.source.github_search_repositories(query, cursor)
    }
    fn github_read(
        &self,
        operation: String,
        input: Value,
    ) -> vults_agents::driver::BoxFuture<'static, Result<Value, String>> {
        self.source.github_read(operation, input)
    }
    fn view(&self) -> Option<Value> {
        self.source.view().map(|v| scoped(v, &self.targets))
    }
    fn usage(&self) -> Value {
        let view = self.view().unwrap_or_default();
        let mut windows = self.source.usage();
        if let Some(windows) = windows.as_array_mut() {
            windows.retain(|w| {
                view["sessions"]
                    .as_array()
                    .is_some_and(|sessions| sessions.iter().any(|s| s["agent"] == w["agent"]))
            });
        }
        windows
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn folder(path: &str) -> String {
        if cfg!(windows) {
            format!("C:{path}")
        } else {
            path.into()
        }
    }

    fn view() -> Value {
        json!({"sessions":[
            {"agent":"claude","id":"same","project":"site","cwd":folder("/one/site"),"step":"first"},
            {"agent":"codex","id":"same","project":"site","cwd":folder("/two/site"),"step":"second"},
            {"agent":"codex","id":"other","project":"site","cwd":folder("/one/site"),"step":"third"}
        ]})
    }

    #[test]
    fn references_use_identity_and_full_folder_not_a_shared_name() {
        let session = Target::Session {
            agent: "codex".into(),
            id: "same".into(),
        };
        let project = Target::Project {
            cwd: folder("/one/site"),
        };
        let selected = |target| Mention {
            token: "@\"site\"".into(),
            target,
        };
        let targets = resolve("Review @\"site\"", vec![selected(session.clone())], &view())
            .unwrap()
            .unwrap();
        assert_eq!(
            scoped(view(), &targets)["sessions"],
            json!([view()["sessions"][1]])
        );
        assert_eq!(
            scoped(view(), std::slice::from_ref(&project))["sessions"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
        assert_eq!(
            scoped(view(), &[session, project])["sessions"]
                .as_array()
                .unwrap()
                .len(),
            3
        );
        assert!(
            resolve(
                "Edited away",
                vec![selected(Target::Project { cwd: folder("/gone") })],
                &view()
            )
            .unwrap()
            .is_none()
        );
        assert!(
            resolve(
                "@\"site\"",
                vec![selected(Target::Project { cwd: folder("/gone") })],
                &view()
            )
            .is_err()
        );
        assert!(validate(&vec![selected(Target::Project { cwd: folder("/ok") }); 9]).is_err());
        assert!(
            validate(&[Mention {
                token: "@site".into(),
                target: Target::Project {
                    cwd: "relative".into()
                }
            }])
            .is_err()
        );
    }

    #[test]
    fn scoped_tools_cannot_fall_back_to_the_rest_of_the_flock() {
        use crate::flock_tools::Source as _;
        #[derive(Debug)]
        struct Fake;
        impl crate::flock_tools::Source for Fake {
            fn view(&self) -> Option<Value> {
                Some(view())
            }
            fn usage(&self) -> Value {
                json!([{"agent":"claude"},{"agent":"codex"}])
            }
        }
        let source = Source {
            source: std::sync::Arc::new(Fake),
            targets: vec![Target::Session {
                agent: "codex".into(),
                id: "same".into(),
            }],
        };
        assert_eq!(source.view().unwrap()["sessions"].as_array().unwrap().len(), 1);
        assert_eq!(source.usage(), json!([{"agent":"codex"}]));
        let empty = Source {
            targets: vec![Target::Project { cwd: folder("/gone") }],
            ..source
        };
        assert_eq!(empty.view().unwrap()["sessions"], json!([]));
        assert_eq!(empty.usage(), json!([]));
    }

    #[tokio::test]
    async fn api_tools_and_context_share_the_selection_even_on_a_missing_name() {
        use crate::flock_tools::Source as _;
        struct Fake;
        impl crate::flock_tools::Source for Fake {
            fn view(&self) -> Option<Value> {
                Some(view())
            }
            fn usage(&self) -> Value {
                json!([])
            }
        }
        let source = std::sync::Arc::new(Source {
            source: std::sync::Arc::new(Fake),
            targets: vec![Target::Session {
                agent: "codex".into(),
                id: "same".into(),
            }],
        });
        let tools = crate::flock_tools::all(source.clone());
        let list = tools
            .iter()
            .find(|t| t.name() == "flock_list")
            .unwrap()
            .call(&json!({}))
            .await
            .unwrap();
        assert!(list.contains("second") && !list.contains("first") && !list.contains("third"));
        let error = tools
            .iter()
            .find(|t| t.name() == "session_detail")
            .unwrap()
            .call(&json!({"session":"claude"}))
            .await
            .unwrap_err();
        assert!(!error.contains("first") && !error.contains("third"));
        let context = crate::context::volatile(source.as_ref(), &[]);
        assert!(context.contains("second") && !context.contains("first") && !context.contains("third"));
        assert_eq!(source.view().unwrap()["sessions"].as_array().unwrap().len(), 1);
        assert!(resolve("Pasted @\"site\"", vec![], &view()).is_err());
        assert!(resolve("No references", vec![], &view()).unwrap().is_none());
    }
}
