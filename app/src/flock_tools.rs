//! Zeca's API chat tools read the flock and GitHub. GitHub writes use typed requests and
//! human approval cards; no tool can answer an agent permission.

mod actions;
pub(crate) mod merging;
mod pull_requests;

use std::sync::Arc;

use serde_json::{Value, json};
use tauri::{AppHandle, Manager};
use vults_agents::driver::BoxFuture;
use vults_chat::tools::{Tool, Tools};
use vults_core::audit::{Act, Actor, Audit};
use vults_zeca::flock::{Flock, FlockReader, Session};

/// Where the tools read the flock from: the app's last view, or a test's.
pub trait Source: Send + Sync + 'static {
    /// The island's view now, as every window got it (JSON).
    fn view(&self) -> Option<Value>;
    /// The plans' usage windows (JSON).
    fn usage(&self) -> Value;
    fn github_snapshot_allowed(&self) -> bool {
        true
    }
    fn github_work_item_allowed(&self) -> bool {
        false
    }
    fn github_work_item(
        &self,
        _repository: String,
        _number: u32,
        _cursors: GithubCursors,
    ) -> BoxFuture<'static, Result<Value, String>> {
        Box::pin(async { Err("Live GitHub reads are unavailable.".into()) })
    }
    fn github_search_repositories(
        &self,
        _query: String,
        _cursor: Option<String>,
    ) -> BoxFuture<'static, Result<Value, String>> {
        Box::pin(async { Err("Live GitHub reads are unavailable.".into()) })
    }
    fn github_read(&self, _operation: String, _input: Value) -> BoxFuture<'static, Result<Value, String>> {
        Box::pin(async { Err("This GitHub read is unavailable.".into()) })
    }
    fn github_merge_prepare(
        &self,
        _action: vults_connectors::github::MergeControl,
    ) -> BoxFuture<'static, Result<Value, String>> {
        Box::pin(async { Err("GitHub merge controls are unavailable.".into()) })
    }
    fn github_merge_execute(
        &self,
        _action: vults_connectors::github::MergeControl,
        _preview: Value,
        _hash: String,
    ) -> BoxFuture<'static, Result<Value, String>> {
        Box::pin(async { Err("GitHub merge controls are unavailable.".into()) })
    }
    fn github_actions_prepare(
        &self,
        _action: vults_connectors::github::ActionsControl,
    ) -> BoxFuture<'static, Result<Value, String>> {
        Box::pin(async { Err("GitHub Actions controls are unavailable.".into()) })
    }
    fn github_actions_execute(
        &self,
        _action: vults_connectors::github::ActionsControl,
        _preview: Value,
        _hash: String,
    ) -> BoxFuture<'static, Result<Value, String>> {
        Box::pin(async { Err("GitHub Actions controls are unavailable.".into()) })
    }
    fn github_pull_request_prepare(
        &self,
        _action: vults_connectors::github::PullRequestWrite,
    ) -> BoxFuture<'static, Result<Value, String>> {
        Box::pin(async { Err("GitHub pull request changes are unavailable.".into()) })
    }
    fn github_pull_request_execute(
        &self,
        _action: vults_connectors::github::PullRequestWrite,
        _preview: Value,
        _hash: String,
    ) -> BoxFuture<'static, Result<Value, String>> {
        Box::pin(async { Err("GitHub pull request changes are unavailable.".into()) })
    }
    fn github_issue_write_prepare(
        &self,
        _action: vults_connectors::github::IssueWrite,
    ) -> BoxFuture<'static, Result<Value, String>> {
        Box::pin(async { Err("GitHub issue writes are unavailable.".into()) })
    }
    fn github_issue_write_execute(
        &self,
        _action: vults_connectors::github::IssueWrite,
        _preview: Value,
        _request_hash: String,
    ) -> BoxFuture<'static, Result<Value, String>> {
        Box::pin(async { Err("GitHub issue writes are unavailable.".into()) })
    }
}

#[derive(Default, Debug)]
pub struct GithubCursors {
    pub comment: Option<String>,
    pub file: Option<String>,
    pub review: Option<String>,
    pub check: Option<String>,
    pub thread: Option<String>,
}

#[derive(Clone)]
pub(super) struct GithubSource {
    pub(super) app: AppHandle,
    pub(super) grant: Option<(String, crate::specialist_github::Grant)>,
}
impl std::ops::Deref for GithubSource {
    type Target = AppHandle;
    fn deref(&self) -> &AppHandle {
        &self.app
    }
}
impl GithubSource {
    pub(super) fn companion(app: AppHandle) -> Self {
        Self { app, grant: None }
    }
    pub(crate) fn specialist(
        app: AppHandle,
        profile: String,
        grant: crate::specialist_github::Grant,
    ) -> Self {
        Self {
            app,
            grant: Some((profile, grant)),
        }
    }
    fn audience_allowed(&self, settings: &crate::settings::Settings) -> bool {
        match &self.grant {
            None => settings.zeca && settings.zeca_github_work_item,
            Some((profile, expected)) => crate::specialist_github::grant_active(settings, profile, expected),
        }
    }
    pub(super) fn subject(&self) -> &str {
        self.grant.as_ref().map_or("zeca", |(profile, _)| profile)
    }
}
impl Source for AppHandle {
    fn view(&self) -> Option<Value> {
        GithubSource::companion(self.clone()).view()
    }
    fn usage(&self) -> Value {
        GithubSource::companion(self.clone()).usage()
    }
    fn github_snapshot_allowed(&self) -> bool {
        GithubSource::companion(self.clone()).github_snapshot_allowed()
    }
    fn github_work_item_allowed(&self) -> bool {
        GithubSource::companion(self.clone()).github_work_item_allowed()
    }
    fn github_work_item(
        &self,
        _repository: String,
        _number: u32,
        _cursors: GithubCursors,
    ) -> BoxFuture<'static, Result<Value, String>> {
        GithubSource::companion(self.clone()).github_work_item(_repository, _number, _cursors)
    }
    fn github_search_repositories(
        &self,
        _query: String,
        _cursor: Option<String>,
    ) -> BoxFuture<'static, Result<Value, String>> {
        GithubSource::companion(self.clone()).github_search_repositories(_query, _cursor)
    }
    fn github_read(&self, _operation: String, _input: Value) -> BoxFuture<'static, Result<Value, String>> {
        GithubSource::companion(self.clone()).github_read(_operation, _input)
    }
    fn github_merge_prepare(
        &self,
        _action: vults_connectors::github::MergeControl,
    ) -> BoxFuture<'static, Result<Value, String>> {
        GithubSource::companion(self.clone()).github_merge_prepare(_action)
    }
    fn github_merge_execute(
        &self,
        _action: vults_connectors::github::MergeControl,
        _preview: Value,
        _hash: String,
    ) -> BoxFuture<'static, Result<Value, String>> {
        GithubSource::companion(self.clone()).github_merge_execute(_action, _preview, _hash)
    }
    fn github_actions_prepare(
        &self,
        _action: vults_connectors::github::ActionsControl,
    ) -> BoxFuture<'static, Result<Value, String>> {
        GithubSource::companion(self.clone()).github_actions_prepare(_action)
    }
    fn github_actions_execute(
        &self,
        _action: vults_connectors::github::ActionsControl,
        _preview: Value,
        _hash: String,
    ) -> BoxFuture<'static, Result<Value, String>> {
        GithubSource::companion(self.clone()).github_actions_execute(_action, _preview, _hash)
    }
    fn github_pull_request_prepare(
        &self,
        _action: vults_connectors::github::PullRequestWrite,
    ) -> BoxFuture<'static, Result<Value, String>> {
        GithubSource::companion(self.clone()).github_pull_request_prepare(_action)
    }
    fn github_pull_request_execute(
        &self,
        _action: vults_connectors::github::PullRequestWrite,
        _preview: Value,
        _hash: String,
    ) -> BoxFuture<'static, Result<Value, String>> {
        GithubSource::companion(self.clone()).github_pull_request_execute(_action, _preview, _hash)
    }
    fn github_issue_write_prepare(
        &self,
        _action: vults_connectors::github::IssueWrite,
    ) -> BoxFuture<'static, Result<Value, String>> {
        GithubSource::companion(self.clone()).github_issue_write_prepare(_action)
    }
    fn github_issue_write_execute(
        &self,
        _action: vults_connectors::github::IssueWrite,
        _preview: Value,
        _request_hash: String,
    ) -> BoxFuture<'static, Result<Value, String>> {
        GithubSource::companion(self.clone()).github_issue_write_execute(_action, _preview, _request_hash)
    }
}

struct GithubToolGate {
    app: GithubSource,
    gate: std::sync::Arc<crate::gate::NetGate>,
}

impl GithubToolGate {
    fn new(app: GithubSource) -> Self {
        Self {
            gate: crate::gate::shared(&app),
            app,
        }
    }
}

impl vults_connectors::Gate for GithubToolGate {
    fn allow(&self, connector: &str) -> bool {
        connector == "github"
            && self.app.github_work_item_allowed()
            && vults_connectors::Gate::allow(&*self.gate, connector)
    }

    fn done(&self, connector: &str, ok: bool, took: std::time::Duration) {
        vults_connectors::Gate::done(&*self.gate, connector, ok, took);
    }
}

fn issue_write_repository(action: &vults_connectors::github::IssueWrite) -> &str {
    use vults_connectors::github::IssueWrite as Write;
    match action {
        Write::Create { repository, .. }
        | Write::Edit { repository, .. }
        | Write::Comment { repository, .. }
        | Write::EditComment { repository, .. }
        | Write::AddLabels { repository, .. }
        | Write::RemoveLabels { repository, .. }
        | Write::SetAssignees { repository, .. }
        | Write::Close { repository, .. }
        | Write::Reopen { repository, .. } => repository,
    }
}

fn issue_write_tool_name(action: &vults_connectors::github::IssueWrite) -> &'static str {
    use vults_connectors::github::IssueWrite as Write;
    match action {
        Write::Create { .. } => "github_issue_create",
        Write::Edit { .. } => "github_issue_edit",
        Write::Comment { .. } => "github_issue_comment",
        Write::EditComment { .. } => "github_issue_comment_edit",
        Write::AddLabels { .. } => "github_issue_add_labels",
        Write::RemoveLabels { .. } => "github_issue_remove_labels",
        Write::SetAssignees { .. } => "github_issue_set_assignees",
        Write::Close { .. } => "github_issue_close",
        Write::Reopen { .. } => "github_issue_reopen",
    }
}

fn audit_issue_write(
    subject: &str,
    action: &vults_connectors::github::IssueWrite,
    hash: &str,
    outcome: &str,
) {
    let repository = issue_write_repository(action);
    let number = match action {
        vults_connectors::github::IssueWrite::Create { .. } => "new issue".to_owned(),
        vults_connectors::github::IssueWrite::Edit { number, .. }
        | vults_connectors::github::IssueWrite::Comment { number, .. }
        | vults_connectors::github::IssueWrite::EditComment { number, .. }
        | vults_connectors::github::IssueWrite::AddLabels { number, .. }
        | vults_connectors::github::IssueWrite::RemoveLabels { number, .. }
        | vults_connectors::github::IssueWrite::SetAssignees { number, .. }
        | vults_connectors::github::IssueWrite::Close { number, .. }
        | vults_connectors::github::IssueWrite::Reopen { number, .. } => format!("issue #{number}"),
    };
    crate::audit::keep(Audit::new(
        Actor::System,
        Act::ExternalWrite,
        subject,
        repository,
        issue_write_tool_name(action),
        &format!("{number} · sha256:{hash} · {outcome}"),
    ));
}

struct IssueWriteAudit<'a> {
    subject: String,
    action: &'a vults_connectors::github::IssueWrite,
    hash: &'a str,
    outcome: &'static str,
}
impl Drop for IssueWriteAudit<'_> {
    fn drop(&mut self) {
        audit_issue_write(&self.subject, self.action, self.hash, self.outcome);
    }
}

impl Source for GithubSource {
    fn github_snapshot_allowed(&self) -> bool {
        self.state::<crate::settings::SettingsState>()
            .0
            .lock()
            .map(|s| s.zeca && s.zeca_github_snapshot)
            .unwrap_or(false)
    }
    fn github_work_item_allowed(&self) -> bool {
        self.state::<crate::settings::SettingsState>()
            .0
            .lock()
            .is_ok_and(|s| self.audience_allowed(&s) && s.connectors.get("github").copied().unwrap_or(false))
    }
    fn github_work_item(
        &self,
        repository: String,
        number: u32,
        cursors: GithubCursors,
    ) -> BoxFuture<'static, Result<Value, String>> {
        let app = self.clone();
        Box::pin(async move {
            if !app.github_work_item_allowed() {
                return Err("Turn on GitHub live reads for Zeca and the GitHub connector first.".into());
            }
            let request = crate::gate::connector_request(&app, "github").ok_or_else(|| {
                "The GitHub connector is off or its daily request limit is spent.".to_string()
            })?;
            let result = vults_connectors::github::work_item(
                &repository,
                number,
                cursors.comment.as_deref(),
                cursors.file.as_deref(),
                cursors.review.as_deref(),
                cursors.check.as_deref(),
                cursors.thread.as_deref(),
            )
            .await;
            request.finish(result.is_ok());
            if !app.github_work_item_allowed() {
                return Err("GitHub live reads were disabled while the request was running.".into());
            }
            result.map_err(|error| error.to_string())
        })
    }
    fn github_search_repositories(
        &self,
        query: String,
        cursor: Option<String>,
    ) -> BoxFuture<'static, Result<Value, String>> {
        let app = self.clone();
        Box::pin(async move {
            if !app.github_work_item_allowed() {
                return Err("Turn on GitHub live reads for Zeca and the GitHub connector first.".into());
            }
            let request = crate::gate::connector_request(&app, "github").ok_or_else(|| {
                "The GitHub connector is off or its daily request limit is spent.".to_string()
            })?;
            let result = vults_connectors::github::search_repositories(&query, cursor.as_deref()).await;
            request.finish(result.is_ok());
            if !app.github_work_item_allowed() {
                return Err("GitHub live reads were disabled while the request was running.".into());
            }
            result.map_err(|error| error.to_string())
        })
    }
    fn github_read(&self, operation: String, input: Value) -> BoxFuture<'static, Result<Value, String>> {
        let app = self.clone();
        Box::pin(async move {
            if !app.github_work_item_allowed() {
                return Err("Turn on GitHub live reads for Zeca and the GitHub connector first.".into());
            }
            if operation == "github_pull_request_merge_status" {
                let gate = GithubToolGate::new(app.clone());
                let result = vults_connectors::github::merge_status(
                    &gate,
                    input["repository"].as_str().unwrap_or_default(),
                    input["number"].as_u64().unwrap_or_default() as u32,
                    input.get("requestId").and_then(Value::as_str),
                )
                .await
                .map_err(|e| e.to_string())?;
                if !app.github_work_item_allowed() {
                    return Err("GitHub tools were disabled while the request was running.".into());
                }
                return Ok(result);
            }
            if operation == "github_actions_log" {
                let gate = GithubToolGate::new(app.clone());
                let result = vults_connectors::github::actions_job_log(
                    &gate,
                    input["repository"].as_str().unwrap_or_default(),
                    input["runId"].as_u64().unwrap_or_default(),
                    input["jobId"].as_u64().unwrap_or_default(),
                    input["attempt"].as_u64().unwrap_or_default() as u32,
                )
                .await
                .map_err(|error| error.to_string())?;
                if !app.github_work_item_allowed() {
                    return Err("GitHub tools were disabled while the request was running.".into());
                }
                return Ok(result);
            }
            let request = crate::gate::connector_request(&app, "github").ok_or_else(|| {
                "The GitHub connector is off or its daily request limit is spent.".to_string()
            })?;
            let repository = input["repository"].as_str().unwrap_or_default();
            let result = match operation.as_str() {
                "github_account" => vults_connectors::github::account().await,
                "github_repository" => vults_connectors::github::repository_details(repository).await,
                "github_pull_request_diff" => {
                    vults_connectors::github::pull_request_diff(
                        repository,
                        input["number"].as_u64().unwrap_or_default() as u32,
                    )
                    .await
                }
                "github_actions_run" => {
                    vults_connectors::github::actions_run(
                        repository,
                        input["runId"].as_u64().unwrap_or_default(),
                        input
                            .get("attempt")
                            .and_then(Value::as_u64)
                            .and_then(|value| u32::try_from(value).ok()),
                    )
                    .await
                }
                "github_actions_runs" => {
                    vults_connectors::github::actions_runs(
                        repository,
                        input["page"].as_u64().unwrap_or(1) as u32,
                    )
                    .await
                }
                "github_actions_jobs" => {
                    vults_connectors::github::actions_jobs(
                        repository,
                        input["runId"].as_u64().unwrap_or_default(),
                        input["attempt"].as_u64().unwrap_or_default() as u32,
                        input["page"].as_u64().unwrap_or(1) as u32,
                    )
                    .await
                }
                "github_branch_rules" => {
                    vults_connectors::github::branch_rules(
                        repository,
                        input["branch"].as_str().unwrap_or_default(),
                        input["page"].as_u64().unwrap_or(1) as u32,
                    )
                    .await
                }
                _ => Err(vults_connectors::Error::Other("Unsupported GitHub read.".into())),
            };
            request.finish(result.is_ok());
            if !app.github_work_item_allowed() {
                return Err("GitHub live reads were disabled while the request was running.".into());
            }
            result.map_err(|error| error.to_string())
        })
    }
    fn github_merge_prepare(
        &self,
        action: vults_connectors::github::MergeControl,
    ) -> BoxFuture<'static, Result<Value, String>> {
        merging::prepare(self.clone(), action)
    }
    fn github_merge_execute(
        &self,
        action: vults_connectors::github::MergeControl,
        preview: Value,
        hash: String,
    ) -> BoxFuture<'static, Result<Value, String>> {
        merging::execute(self.clone(), action, preview, hash)
    }
    fn github_actions_prepare(
        &self,
        action: vults_connectors::github::ActionsControl,
    ) -> BoxFuture<'static, Result<Value, String>> {
        actions::prepare(self.clone(), action)
    }
    fn github_actions_execute(
        &self,
        action: vults_connectors::github::ActionsControl,
        preview: Value,
        hash: String,
    ) -> BoxFuture<'static, Result<Value, String>> {
        actions::execute(self.clone(), action, preview, hash)
    }
    fn github_pull_request_prepare(
        &self,
        action: vults_connectors::github::PullRequestWrite,
    ) -> BoxFuture<'static, Result<Value, String>> {
        pull_requests::prepare(self.clone(), action)
    }
    fn github_pull_request_execute(
        &self,
        action: vults_connectors::github::PullRequestWrite,
        preview: Value,
        hash: String,
    ) -> BoxFuture<'static, Result<Value, String>> {
        pull_requests::execute(self.clone(), action, preview, hash)
    }
    fn github_issue_write_prepare(
        &self,
        action: vults_connectors::github::IssueWrite,
    ) -> BoxFuture<'static, Result<Value, String>> {
        let app = self.clone();
        Box::pin(async move {
            if !app.github_work_item_allowed() {
                return Err("Turn on GitHub tools for Zeca and the GitHub connector first.".into());
            }
            let gate = GithubToolGate::new(app.clone());
            let preview = vults_connectors::github::prepare_issue_write(&gate, &action)
                .await
                .map_err(|error| error.to_string())?;
            if !app.github_work_item_allowed() {
                return Err("GitHub tools were disabled while the proposal was being prepared.".into());
            }
            Ok(preview)
        })
    }
    fn github_issue_write_execute(
        &self,
        action: vults_connectors::github::IssueWrite,
        preview: Value,
        request_hash: String,
    ) -> BoxFuture<'static, Result<Value, String>> {
        let app = self.clone();
        Box::pin(async move {
            if !app.github_work_item_allowed() {
                audit_issue_write(app.subject(), &action, &request_hash, "denied");
                return Err("Turn on GitHub tools for Zeca and the GitHub connector first.".into());
            }
            let gate = GithubToolGate::new(app.clone());
            let mut audit = IssueWriteAudit {
                subject: app.subject().into(),
                action: &action,
                hash: &request_hash,
                outcome: "cancelled_or_uncertain",
            };
            let result =
                vults_connectors::github::execute_issue_write(&gate, &action, &preview, &request_hash)
                    .await
                    .map_err(|error| error.to_string());
            let outcome = if result.is_ok() {
                "verified"
            } else {
                "failed_or_uncertain"
            };
            audit.outcome = outcome;
            result
        })
    }
    fn view(&self) -> Option<Value> {
        let view = self.state::<crate::runtime::LastView>().get()?;
        serde_json::to_value(view).ok()
    }

    fn usage(&self) -> Value {
        serde_json::to_value(crate::usage::usage(self.app.clone())).unwrap_or_default()
    }
}

/// Flock reads and typed GitHub tools backed by the application.
pub fn all(source: Arc<dyn Source>) -> Tools {
    let mut tools: Tools = vec![
        Arc::new(FlockList(source.clone())),
        Arc::new(SessionDetail(source.clone())),
        Arc::new(PendingAsks(source.clone())),
        Arc::new(Usage(source.clone())),
        Arc::new(GithubSnapshot(source.clone())),
        Arc::new(GithubWorkItem(source.clone())),
        Arc::new(GithubRepositorySearch(source.clone())),
        Arc::new(GithubRead {
            source: source.clone(),
            operation: "github_account",
        }),
        Arc::new(GithubRead {
            source: source.clone(),
            operation: "github_repository",
        }),
        Arc::new(GithubRead {
            source: source.clone(),
            operation: "github_pull_request_diff",
        }),
        Arc::new(GithubRead {
            source: source.clone(),
            operation: "github_pull_request_merge_status",
        }),
        Arc::new(GithubRead {
            source: source.clone(),
            operation: "github_actions_run",
        }),
        Arc::new(GithubRead {
            source: source.clone(),
            operation: "github_actions_runs",
        }),
        Arc::new(GithubRead {
            source: source.clone(),
            operation: "github_actions_jobs",
        }),
        Arc::new(GithubRead {
            source: source.clone(),
            operation: "github_actions_log",
        }),
        Arc::new(GithubRead {
            source: source.clone(),
            operation: "github_branch_rules",
        }),
        Arc::new(GithubIssueWrite {
            source: source.clone(),
            operation: "github_issue_create",
        }),
        Arc::new(GithubIssueWrite {
            source: source.clone(),
            operation: "github_issue_edit",
        }),
        Arc::new(GithubIssueWrite {
            source: source.clone(),
            operation: "github_issue_comment",
        }),
        Arc::new(GithubIssueWrite {
            source: source.clone(),
            operation: "github_issue_comment_edit",
        }),
        Arc::new(GithubIssueWrite {
            source: source.clone(),
            operation: "github_issue_add_labels",
        }),
        Arc::new(GithubIssueWrite {
            source: source.clone(),
            operation: "github_issue_remove_labels",
        }),
        Arc::new(GithubIssueWrite {
            source: source.clone(),
            operation: "github_issue_set_assignees",
        }),
        Arc::new(GithubIssueWrite {
            source: source.clone(),
            operation: "github_issue_close",
        }),
        Arc::new(GithubIssueWrite {
            source: source.clone(),
            operation: "github_issue_reopen",
        }),
    ];
    tools.extend(pull_requests::tools(source.clone()));
    tools.extend(actions::tools(source.clone()));
    tools.extend(merging::tools(source));
    tools
}

/// Realtime currently admits five tools; keep the slower live GitHub lookup in regular chat.
pub fn realtime(source: Arc<dyn Source>) -> Tools {
    let mut tools = all(source);
    tools.retain(|tool| !tool.name().starts_with("github_") || tool.name() == "github_snapshot");
    tools
}

/// Zeca's snapshot of the flock from the view. Every agent-supplied text (a card's target, what a
/// session is doing, its steps) is redacted here: the core's view keeps them as the agent sent
/// them, and these go to the provider. They stay local agents' words, not outside content (Z8): an
/// agent's own fetches are its session's business, and the chat's cards are not the agents'.
pub fn flock(view: &Value) -> Flock {
    let text = |v: &Value| v.as_str().map(str::to_string);
    let approval = &view["approval"];
    let sessions = view["sessions"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or_default()
        .iter()
        .map(|s| {
            let agent = text(&s["agent_name"])
                .or_else(|| text(&s["agent"]))
                .unwrap_or_default();
            // `waiting`: a card waits on it, in front or queued (`card` is only the one in front).
            let waiting = (s["waiting"] == true || s["card"] == true).then(|| {
                // Ids are unique per agent: the front card is the one of this agent's session.
                if approval["session"] == s["id"] && approval["agent"] == s["agent"] {
                    (
                        text(&approval["tool"]).unwrap_or_default(),
                        vults_core::redact::secrets(approval["target"].as_str().unwrap_or_default()),
                    )
                } else {
                    // Another card than the one in front: the island does not show its target.
                    (String::new(), String::new())
                }
            });
            Session {
                agent,
                project: text(&s["project"]).unwrap_or_default(),
                status: text(&s["status"]).unwrap_or_default(),
                doing: s["step"].as_str().map(vults_core::redact::secrets),
                steps: s["steps"]
                    .as_array()
                    .map(Vec::as_slice)
                    .unwrap_or_default()
                    .iter()
                    .filter_map(|v| v.as_str().map(vults_core::redact::secrets))
                    .collect(),
                waiting,
            }
        })
        .collect();
    Flock { sessions }
}

fn reader(source: &dyn Source) -> FlockReader {
    FlockReader::new(source.view().map(|v| flock(&v)).unwrap_or_default())
}

/// One session as a line of the tools' answers.
fn brief(s: &Session) -> Value {
    json!({
        "agent": s.agent,
        "project": s.project,
        "status": s.status,
        "doing": s.doing,
        "waiting_on": s.waiting.as_ref().map(|(tool, target)| {
            if tool.is_empty() { "a card".to_string() } else { format!("{tool} · {target}") }
        }),
    })
}

fn detail(s: &Session) -> Value {
    let mut value = brief(s);
    value["steps"] = json!(s.steps);
    value
}

/// Voice commands already resolved an exact identity; do not guess again from its display label.
pub fn session_detail_exact(view: &Value, key: &vults_core::SessionKey) -> Option<Value> {
    let session = view["sessions"]
        .as_array()?
        .iter()
        .find(|s| s["id"].as_str() == Some(&key.session_id) && s["agent"] == json!(key.agent))?;
    let mut selected = view.clone();
    selected["sessions"] = json!([session]);
    let flock = flock(&selected);
    flock.sessions.first().map(detail)
}

const NO_INPUT: fn() -> Value = || json!({ "type": "object", "properties": {} });

#[derive(Debug)]
struct FlockList(Arc<dyn Source>);
#[derive(Debug)]
struct SessionDetail(Arc<dyn Source>);
#[derive(Debug)]
struct PendingAsks(Arc<dyn Source>);
#[derive(Debug)]
struct Usage(Arc<dyn Source>);
#[derive(Debug)]
struct GithubSnapshot(Arc<dyn Source>);
#[derive(Debug)]
struct GithubWorkItem(Arc<dyn Source>);
#[derive(Debug)]
struct GithubRepositorySearch(Arc<dyn Source>);
#[derive(Debug)]
struct GithubRead {
    source: Arc<dyn Source>,
    operation: &'static str,
}
#[derive(Debug)]
struct GithubIssueWrite {
    source: Arc<dyn Source>,
    operation: &'static str,
}

impl std::fmt::Debug for dyn Source {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Source")
    }
}

fn answer(v: Value) -> BoxFuture<'static, Result<String, String>> {
    Box::pin(async move { Ok(v.to_string()) })
}

/// An error the model reads as one (`is_error`), with what it can do instead.
fn failed(v: Value) -> BoxFuture<'static, Result<String, String>> {
    Box::pin(async move { Err(v.to_string()) })
}

/// Whether `asked` names this session: its project, or its agent by any of the names people use
/// (`claude code` for `claude`, `qwen code` for `qwen`).
fn named(s: &Session, asked: &str) -> bool {
    let asked = asked.trim().to_lowercase();
    let agent = s.agent.to_lowercase();
    s.project.to_lowercase() == asked
        || agent == asked
        || asked
            .split_whitespace()
            .next()
            .is_some_and(|first| first == agent)
}

impl Tool for FlockList {
    fn name(&self) -> &str {
        "flock_list"
    }
    fn description(&self) -> &str {
        "The agent sessions on the user's island now (Claude Code, Codex, Antigravity and others): \
         each one's agent, project, status, what it is doing and the card it waits on, if any."
    }
    fn schema(&self) -> Value {
        NO_INPUT()
    }
    fn call<'a>(&'a self, _: &'a Value) -> BoxFuture<'a, Result<String, String>> {
        let r = reader(self.0.as_ref());
        answer(json!({ "sessions": r.sessions().iter().map(brief).collect::<Vec<_>>() }))
    }
}

impl Tool for SessionDetail {
    fn name(&self) -> &str {
        "session_detail"
    }
    fn description(&self) -> &str {
        "One session's latest steps, by its project name (or its agent's name when only one runs)."
    }
    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": { "session": { "type": "string", "description": "The project, or the agent" } },
            "required": ["session"],
        })
    }
    fn call<'a>(&'a self, input: &'a Value) -> BoxFuture<'a, Result<String, String>> {
        let r = reader(self.0.as_ref());
        let wanted = input["session"].as_str().unwrap_or_default();
        let found: Vec<&Session> = r.sessions().iter().filter(|s| named(s, wanted)).collect();
        match found.as_slice() {
            [s] => answer(detail(s)),
            [] => failed(json!({
                "error": format!("No session named {wanted}."),
                "sessions": r.sessions().iter().map(brief).collect::<Vec<_>>(),
            })),
            many => failed(json!({
                "error": "More than one session goes by that name: name the project.",
                "sessions": many.iter().map(|s| brief(s)).collect::<Vec<_>>(),
            })),
        }
    }
}

impl Tool for PendingAsks {
    fn name(&self) -> &str {
        "pending_asks"
    }
    fn description(&self) -> &str {
        "The sessions waiting on the user's answer to a card. Only the user answers them, on the island."
    }
    fn schema(&self) -> Value {
        NO_INPUT()
    }
    fn call<'a>(&'a self, _: &'a Value) -> BoxFuture<'a, Result<String, String>> {
        answer(
            cached_read(&self.0.view().unwrap_or_default(), Value::Null, "pending_asks")
                .expect("Known local read"),
        )
    }
}

impl Tool for Usage {
    fn name(&self) -> &str {
        "usage"
    }
    fn description(&self) -> &str {
        "How much of each plan's rate-limit windows is used (Claude Code, Codex), and when they reset."
    }
    fn schema(&self) -> Value {
        NO_INPUT()
    }
    fn call<'a>(&'a self, _: &'a Value) -> BoxFuture<'a, Result<String, String>> {
        answer(json!({ "windows": self.0.usage() }))
    }
}

/// What is open, for the model: releases left out, and of each row's checks only those not
/// passed, so twenty pull requests of thirty checks each stay a short answer.
fn open_rows(rows: &Value) -> Value {
    let rows = rows
        .as_array()
        .into_iter()
        .flatten()
        .filter(|r| r["group"] != "releases");
    Value::Array(
        rows.map(|r| {
            let mut r = r.clone();
            if let Some(runs) = r["facts"]["runs"].as_array_mut() {
                runs.retain(|c| c["state"] != "passed" && c["state"] != "skipped");
            }
            r
        })
        .collect(),
    )
}

/// Synchronous local reads let command admission and cancellation share one short lock.
/// No request is made: both values are already cached by their existing owners.
pub fn cached_read(view: &Value, usage: Value, name: &str) -> Option<Value> {
    Some(match name {
        "pending_asks" => {
            json!({"waiting":FlockReader::new(flock(view)).waiting().map(brief).collect::<Vec<_>>()})
        }
        "usage" => json!({"windows":usage}),
        "github_snapshot" => {
            let board = view["boards"]
                .as_array()?
                .iter()
                .find(|board| board["connector"] == "github")?;
            json!({"source":{"provider":"github","mode":"cached_snapshot"},"rows":open_rows(&board["rows"])})
        }
        _ => return None,
    })
}

impl Tool for GithubSnapshot {
    fn name(&self) -> &str {
        "github_snapshot"
    }
    fn description(&self) -> &str {
        "The user's GitHub card as the island shows it: their pull requests and reviews, with checks. The source says this is a cached snapshot, not a live read."
    }
    fn schema(&self) -> Value {
        NO_INPUT()
    }
    fn call<'a>(&'a self, _: &'a Value) -> BoxFuture<'a, Result<String, String>> {
        if !self.0.github_snapshot_allowed() {
            return failed(json!({"error":"The user disabled the built-in GitHub snapshot tool."}));
        }
        match cached_read(&self.0.view().unwrap_or_default(), Value::Null, "github_snapshot") {
            Some(value) => Box::pin(async move { github_result(value) }),
            None => failed(json!({"error":"The GitHub connector is off, or has nothing yet."})),
        }
    }

    /// Titles and names come from GitHub: outside content (Z8).
    fn reads_outside(&self) -> bool {
        true
    }

    fn activity_mode(&self) -> Option<&'static str> {
        Some("cached_snapshot")
    }
}

impl Tool for GithubWorkItem {
    fn name(&self) -> &str {
        "github_work_item"
    }
    fn description(&self) -> &str {
        "Reads one GitHub issue or pull request by exact owner/repository and number. Returns bounded pages of discussion, changed files, reviews, review threads and checks. For each collection, pass its nextCursor in the matching cursor field while hasMore is true. Each call is a separate GitHub request. Cite source.repository, source.url and source.fetchedAt; check coverage and truncation before treating the result as complete."
    }
    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "repository": { "type": "string", "pattern": "^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$", "description": "Exact repository in owner/name form." },
                "number": { "type": "integer", "minimum": 1, "maximum": 2147483647, "description": "Issue or pull request number." },
                "commentCursor": { "type": "string", "maxLength": 512, "description": "discussion.nextCursor from a previous page, when discussion.hasMore is true." },
                "fileCursor": { "type": "string", "maxLength": 512, "description": "pullRequest.files.nextCursor from a previous page, when pullRequest.files.hasMore is true." },
                "reviewCursor": { "type": "string", "maxLength": 512, "description": "pullRequest.reviews.nextCursor from a previous page." },
                "checkCursor": { "type": "string", "maxLength": 512, "description": "pullRequest.checks.nextCursor from a previous page." },
                "threadCursor": { "type": "string", "maxLength": 512, "description": "pullRequest.reviewThreads.nextCursor from a previous page." }
            },
            "required": ["repository", "number"],
            "additionalProperties": false
        })
    }
    fn call<'a>(&'a self, input: &'a Value) -> BoxFuture<'a, Result<String, String>> {
        if !self.0.github_work_item_allowed() {
            return failed(
                json!({ "error": "Live GitHub issue and pull request reads are off, or the GitHub connector is off." }),
            );
        }
        let Some(repository) = input["repository"].as_str().map(str::to_owned) else {
            return failed(json!({ "error": "Give the exact GitHub repository as owner/name." }));
        };
        let Some(number) = input["number"]
            .as_u64()
            .and_then(|value| u32::try_from(value).ok())
        else {
            return failed(json!({ "error": "Give a positive issue or pull request number." }));
        };
        let comment_cursor = match input.get("commentCursor") {
            None | Some(Value::Null) => None,
            Some(Value::String(cursor)) => Some(cursor.clone()),
            _ => return failed(json!({ "error": "The discussion cursor must be a string." })),
        };
        let file_cursor = match input.get("fileCursor") {
            None | Some(Value::Null) => None,
            Some(Value::String(cursor)) => Some(cursor.clone()),
            _ => return failed(json!({ "error": "The changed-file cursor must be a string." })),
        };
        let mut collection_cursors = Vec::new();
        for (key, label) in [
            ("reviewCursor", "review"),
            ("checkCursor", "check"),
            ("threadCursor", "review thread"),
        ] {
            match input.get(key) {
                None | Some(Value::Null) => collection_cursors.push(None),
                Some(Value::String(cursor)) => collection_cursors.push(Some(cursor.clone())),
                _ => return failed(json!({"error":format!("The {label} cursor must be a string.")})),
            }
        }
        let result = self.0.github_work_item(
            repository,
            number,
            GithubCursors {
                comment: comment_cursor,
                file: file_cursor,
                review: collection_cursors.remove(0),
                check: collection_cursors.remove(0),
                thread: collection_cursors.remove(0),
            },
        );
        Box::pin(async move {
            let value = result.await?;
            github_result(value)
        })
    }
    fn realtime_timeout(&self) -> std::time::Duration {
        std::time::Duration::from_secs(30)
    }
    fn reads_outside(&self) -> bool {
        true
    }
    fn activity_mode(&self) -> Option<&'static str> {
        Some("live")
    }
}

impl Tool for GithubRepositorySearch {
    fn name(&self) -> &str {
        "github_search_repositories"
    }
    fn description(&self) -> &str {
        "Searches repositories visible to the user's selected GitHub account. Use a short repository name, owner, or topic phrase, then read a selected issue or pull request by exact owner/name and number. Returns one page of at most ten matches. If hasMore is true, pass nextCursor as cursor to fetch the next page; every page is a separate GitHub request. Cite source.account, source.host, source.fetchedAt and each selected repository's URL."
    }
    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "query": { "type": "string", "minLength": 1, "maxLength": 160, "description": "Repository name, owner, or topic phrase." },
                "cursor": { "type": "string", "maxLength": 512, "description": "nextCursor from the previous page, when hasMore is true." }
            },
            "required": ["query"],
            "additionalProperties": false
        })
    }
    fn call<'a>(&'a self, input: &'a Value) -> BoxFuture<'a, Result<String, String>> {
        if !self.0.github_work_item_allowed() {
            return failed(json!({ "error": "Live GitHub reads are off, or the GitHub connector is off." }));
        }
        let Some(query) = input["query"].as_str().map(str::to_owned) else {
            return failed(json!({ "error": "Give a repository name, owner, or topic phrase to search." }));
        };
        let cursor = match input.get("cursor") {
            None | Some(Value::Null) => None,
            Some(Value::String(cursor)) => Some(cursor.clone()),
            _ => return failed(json!({ "error": "The repository search cursor must be a string." })),
        };
        let result = self.0.github_search_repositories(query, cursor);
        Box::pin(async move {
            let value = result.await?;
            github_result(value)
        })
    }
    fn realtime_timeout(&self) -> std::time::Duration {
        std::time::Duration::from_secs(30)
    }
    fn reads_outside(&self) -> bool {
        true
    }
    fn activity_mode(&self) -> Option<&'static str> {
        Some("live")
    }
}

impl Tool for GithubRead {
    fn name(&self) -> &str {
        self.operation
    }
    fn description(&self) -> &str {
        match self.operation {
            "github_account" => {
                "Reads the account login and host selected by gh, with a profile URL and retrieval time. Cite source.url and source.fetchedAt."
            }
            "github_repository" => {
                "Reads metadata for one exact accessible repository, including its visibility, archive state, default branch and source URL. Cite source.repository, source.url and source.fetchedAt."
            }
            "github_pull_request_diff" => {
                "Reads only the unified diff for one exact pull request. Cite source.repository, source.url and source.fetchedAt. The diff is capped at 48 KiB; check truncated and containsBinary before relying on full file coverage."
            }
            "github_pull_request_merge_status" => {
                "Reads the exact PR's merged, queued, pending, open or closed state without submitting a merge. Optionally supply the requestId UUID returned by an asynchronous merge to read its result. A queued or pending result is never merged. Does not clear durable uncertain receipts or resend a control."
            }
            "github_actions_run" => {
                "Reads one exact workflow run or historical attempt. Omit attempt to inspect the latest observed attempt after a control. Returns commit SHA, workflow, observed attempt and distinct queued/running/waiting/cancelled/completed phase. Never interpret an accepted control as a verified new attempt."
            }
            "github_actions_runs" => {
                "Lists one page of ten recent workflow runs for an exact repository. Cite source.repository, source.url and source.fetchedAt. Pass nextPage as page while hasMore is true."
            }
            "github_actions_jobs" => {
                "Lists one page of ten jobs for an exact workflow run and attempt. Cite source.repository, source.url and source.fetchedAt. Pass nextPage as page while hasMore is true."
            }
            "github_actions_log" => {
                "Reads one exact Actions job log for a repository, run, job and attempt. Cite source.repository, source.url and source.fetchedAt. Output is capped and secrets are redacted; check truncated."
            }
            _ => {
                "Reads active rules that apply to an exact branch, including organization-level rulesets. Cite source.repository, source.url and source.fetchedAt. Returns at most 100 rules per page; check hasMore and pass nextPage as page."
            }
        }
    }
    fn schema(&self) -> Value {
        let (properties, required): (Value, &[&str]) = match self.operation {
            "github_account" => (json!({}), &[]),
            "github_repository" => (
                json!({"repository":{"type":"string","pattern":"^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$"}}),
                &["repository"],
            ),
            "github_pull_request_diff" => (
                json!({"repository":{"type":"string","pattern":"^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$"},"number":{"type":"integer","minimum":1,"maximum":2147483647}}),
                &["repository", "number"],
            ),
            "github_pull_request_merge_status" => (
                json!({"repository":{"type":"string"},"number":{"type":"integer","minimum":1,"maximum":2147483647},"requestId":{"type":"string","pattern":"^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$"}}),
                &["repository", "number"],
            ),
            "github_actions_run" => (
                json!({"repository":{"type":"string","pattern":"^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$"},"runId":{"type":"integer","minimum":1},"attempt":{"type":"integer","minimum":1,"maximum":4294967295u64}}),
                &["repository", "runId"],
            ),
            "github_actions_runs" => (
                json!({"repository":{"type":"string","pattern":"^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$"},"page":{"type":"integer","minimum":1,"maximum":100,"default":1}}),
                &["repository"],
            ),
            "github_actions_jobs" => (
                json!({"repository":{"type":"string","pattern":"^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$"},"runId":{"type":"integer","minimum":1},"attempt":{"type":"integer","minimum":1},"page":{"type":"integer","minimum":1,"maximum":100,"default":1}}),
                &["repository", "runId", "attempt"],
            ),
            "github_actions_log" => (
                json!({"repository":{"type":"string","pattern":"^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$"},"runId":{"type":"integer","minimum":1},"jobId":{"type":"integer","minimum":1},"attempt":{"type":"integer","minimum":1}}),
                &["repository", "runId", "jobId", "attempt"],
            ),
            _ => (
                json!({"repository":{"type":"string","pattern":"^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$"},"branch":{"type":"string","minLength":1,"maxLength":255},"page":{"type":"integer","minimum":1,"maximum":100,"default":1}}),
                &["repository", "branch"],
            ),
        };
        json!({"type":"object","properties":properties,"required":required,"additionalProperties":false})
    }
    fn call<'a>(&'a self, input: &'a Value) -> BoxFuture<'a, Result<String, String>> {
        if !self.source.github_work_item_allowed() {
            return failed(json!({"error":"Live GitHub reads are off, or the GitHub connector is off."}));
        }
        if self.operation != "github_account" && input["repository"].as_str().is_none() {
            return failed(json!({"error":"Give the exact GitHub repository as owner/name."}));
        }
        let positive = |key: &str| input[key].as_u64().is_some_and(|n| n > 0);
        let valid = match self.operation {
            "github_account" => true,
            "github_repository" => input["repository"].as_str().is_some_and(|name| !name.is_empty()),
            "github_pull_request_diff" => input["number"]
                .as_u64()
                .is_some_and(|n| (1..=i32::MAX as u64).contains(&n)),
            "github_pull_request_merge_status" => {
                input["number"]
                    .as_u64()
                    .is_some_and(|n| (1..=i32::MAX as u64).contains(&n))
                    && input.get("requestId").is_none_or(|v| {
                        v.as_str().is_some_and(|s| {
                            s.len() == 36
                                && s.bytes().enumerate().all(|(i, c)| {
                                    if [8, 13, 18, 23].contains(&i) {
                                        c == b'-'
                                    } else {
                                        c.is_ascii_hexdigit()
                                    }
                                })
                        })
                    })
            }
            "github_actions_run" => {
                positive("runId")
                    && input.get("attempt").is_none_or(|value| {
                        value.as_u64().is_some_and(|n| (1..=u32::MAX as u64).contains(&n))
                    })
            }
            "github_actions_runs" => input.get("page").is_none_or(|_| positive_page(input, "page")),
            "github_actions_jobs" => {
                positive("runId")
                    && input["attempt"]
                        .as_u64()
                        .is_some_and(|n| (1..=u32::MAX as u64).contains(&n))
                    && input.get("page").is_none_or(|_| positive_page(input, "page"))
            }
            "github_actions_log" => {
                positive("runId")
                    && positive("jobId")
                    && input["attempt"]
                        .as_u64()
                        .is_some_and(|n| (1..=u32::MAX as u64).contains(&n))
            }
            _ => {
                input["branch"].as_str().is_some_and(|s| !s.is_empty())
                    && input.get("page").is_none_or(|_| positive_page(input, "page"))
            }
        };
        if !valid {
            return failed(json!({"error":"The GitHub read arguments are missing or invalid."}));
        }
        let result = self.source.github_read(self.operation.to_string(), input.clone());
        Box::pin(async move {
            let value = result.await?;
            github_result(value)
        })
    }
    fn realtime_timeout(&self) -> std::time::Duration {
        std::time::Duration::from_secs(30)
    }
    fn reads_outside(&self) -> bool {
        true
    }
    fn activity_mode(&self) -> Option<&'static str> {
        Some("live")
    }
}

impl Tool for GithubIssueWrite {
    fn name(&self) -> &str {
        self.operation
    }

    fn description(&self) -> &str {
        match self.operation {
            "github_issue_create" => {
                "Proposes creating an issue in an exact repository. The user must approve the complete title and body."
            }
            "github_issue_edit" => {
                "Proposes editing an issue title or body. The user must approve the exact change."
            }
            "github_issue_comment" => {
                "Proposes posting an issue comment. The user must approve the exact comment text."
            }
            "github_issue_comment_edit" => {
                "Proposes editing one existing issue comment by its exact ID. The user must approve the exact replacement text."
            }
            "github_issue_add_labels" => {
                "Proposes adding labels to an issue. The user must approve the exact labels."
            }
            "github_issue_remove_labels" => {
                "Proposes removing labels from an issue. The user must approve the exact labels."
            }
            "github_issue_set_assignees" => {
                "Proposes replacing an issue's assignees, including clearing them with an empty list. The user must approve the exact usernames."
            }
            "github_issue_close" => {
                "Proposes closing an issue. The user must approve the exact repository and issue."
            }
            _ => "Proposes reopening an issue. The user must approve the exact repository and issue.",
        }
    }

    fn schema(&self) -> Value {
        let repo = json!({"type":"string","pattern":"^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$","maxLength":200});
        let number = json!({"type":"integer","minimum":1,"maximum":2147483647});
        let text = |max| json!({"type":"string","maxLength":max});
        let names = |max, minimum| json!({"type":"array","minItems":minimum,"maxItems":max,"uniqueItems":true,"items":{"type":"string","minLength":1,"maxLength":50}});
        let (properties, required) = match self.operation {
            "github_issue_create" => (
                json!({"repository":repo,"title":text(256),"body":text(10000)}),
                json!(["repository", "title", "body"]),
            ),
            "github_issue_edit" => (
                json!({"repository":repo,"number":number,"title":text(256),"body":text(10000)}),
                json!(["repository", "number"]),
            ),
            "github_issue_comment" => (
                json!({"repository":repo,"number":number,"body":text(10000)}),
                json!(["repository", "number", "body"]),
            ),
            "github_issue_comment_edit" => (
                json!({"repository":repo,"number":number,"commentId":{"type":"integer","minimum":1},"body":text(10000)}),
                json!(["repository", "number", "commentId", "body"]),
            ),
            "github_issue_add_labels" | "github_issue_remove_labels" => (
                json!({"repository":repo,"number":number,"labels":names(10,1)}),
                json!(["repository", "number", "labels"]),
            ),
            "github_issue_set_assignees" => (
                json!({"repository":repo,"number":number,"assignees":{"type":"array","maxItems":10,"uniqueItems":true,"items":{"type":"string","minLength":1,"maxLength":39}}}),
                json!(["repository", "number", "assignees"]),
            ),
            _ => (
                json!({"repository":repo,"number":number}),
                json!(["repository", "number"]),
            ),
        };
        json!({"type":"object","properties":properties,"required":required,"additionalProperties":false})
    }

    fn call<'a>(&'a self, _input: &'a Value) -> BoxFuture<'a, Result<String, String>> {
        failed(json!({"error":"GitHub issue changes require the explicit approval card."}))
    }

    fn approval<'a>(
        &'a self,
        input: &'a Value,
    ) -> Option<BoxFuture<'a, Result<vults_chat::tools::ToolApproval, String>>> {
        let action = match parse_issue_write(self.operation, input) {
            Ok(action) => action,
            Err(error) => return Some(Box::pin(async move { Err(error) })),
        };
        Some(Box::pin(async move {
            let preview = self.source.github_issue_write_prepare(action.clone()).await?;
            let hash = action.request_hash();
            let number = input["number"]
                .as_u64()
                .map(|n| format!("issue #{n}"))
                .unwrap_or_else(|| "new issue".into());
            let target = format!(
                "{} · {number} · {} · sha256:{hash}",
                issue_write_repository(&action),
                self.operation
            );
            let description = format!(
                "Review the exact GitHub change for {} ({number}). This request will be checked again before it runs.",
                issue_write_repository(&action)
            );
            let issue = &preview["issue"];
            let full = serde_json::to_string_pretty(&json!({"operation":self.operation,"request":action,"requestHash":hash,"source":preview["source"],"current":{"title":issue["title"],"state":issue["state"],"url":issue["url"],"labels":issue["labels"],"assignees":issue["assignees"]}})).map_err(|error|error.to_string())?;
            Ok(vults_chat::tools::ToolApproval {
                target,
                description,
                full,
                context: json!({"preview":preview,"requestHash":hash}),
            })
        }))
    }

    fn call_approved<'a>(
        &'a self,
        input: &'a Value,
        context: &'a Value,
    ) -> BoxFuture<'a, Result<String, String>> {
        let action = match parse_issue_write(self.operation, input) {
            Ok(action) => action,
            Err(error) => return Box::pin(async move { Err(error) }),
        };
        let hash = action.request_hash();
        let approved_hash = context["requestHash"].as_str().unwrap_or_default();
        if hash != approved_hash {
            return Box::pin(async {
                Err("The GitHub request changed after approval. Review a new card.".into())
            });
        }
        let source = self.source.clone();
        let preview = context["preview"].clone();
        Box::pin(async move {
            let result = source.github_issue_write_execute(action, preview, hash).await?;
            github_result(result)
        })
    }

    fn reads_outside(&self) -> bool {
        true
    }
    fn activity_mode(&self) -> Option<&'static str> {
        Some("live")
    }
}

fn parse_issue_write(operation: &str, input: &Value) -> Result<vults_connectors::github::IssueWrite, String> {
    use vults_connectors::github::IssueWrite as W;
    let allowed: &[&str] = match operation {
        "github_issue_create" => &["repository", "title", "body"],
        "github_issue_edit" => &["repository", "number", "title", "body"],
        "github_issue_comment" => &["repository", "number", "body"],
        "github_issue_comment_edit" => &["repository", "number", "commentId", "body"],
        "github_issue_add_labels" | "github_issue_remove_labels" => &["repository", "number", "labels"],
        "github_issue_set_assignees" => &["repository", "number", "assignees"],
        "github_issue_close" | "github_issue_reopen" => &["repository", "number"],
        _ => return Err("Unsupported GitHub issue write.".into()),
    };
    let object = input
        .as_object()
        .ok_or("GitHub issue write arguments must be an object.")?;
    if object.keys().any(|key| !allowed.contains(&key.as_str())) {
        return Err("GitHub issue write arguments contain an unsupported field.".into());
    }
    let repository = input["repository"]
        .as_str()
        .ok_or("Give the exact GitHub repository as owner/name.")?
        .to_owned();
    let number = || {
        input["number"]
            .as_u64()
            .and_then(|n| u32::try_from(n).ok())
            .filter(|n| *n > 0)
            .ok_or_else(|| "Give a positive issue number.".to_owned())
    };
    let list = |key: &'static str| -> Result<Vec<String>, String> {
        input[key]
            .as_array()
            .ok_or_else(|| format!("Give {key} as a list."))?
            .iter()
            .map(|v| {
                v.as_str()
                    .map(str::to_owned)
                    .ok_or_else(|| format!("Every {key} entry must be text."))
            })
            .collect()
    };
    let action = match operation {
        "github_issue_create" => W::Create {
            repository,
            title: input["title"].as_str().ok_or("Give an issue title.")?.to_owned(),
            body: input["body"].as_str().ok_or("Give the issue body.")?.to_owned(),
        },
        "github_issue_edit" => {
            let optional_text = |key| -> Result<Option<String>, String> {
                match input.get(key) {
                    None => Ok(None),
                    Some(Value::String(value)) => Ok(Some(value.clone())),
                    _ => Err(format!("The {key} field must be text.")),
                }
            };
            W::Edit {
                repository,
                number: number()?,
                title: optional_text("title")?,
                body: optional_text("body")?,
            }
        }
        "github_issue_comment" => W::Comment {
            repository,
            number: number()?,
            body: input["body"].as_str().ok_or("Give the comment body.")?.to_owned(),
        },
        "github_issue_comment_edit" => W::EditComment {
            repository,
            number: number()?,
            comment_id: input["commentId"]
                .as_u64()
                .filter(|value| *value > 0)
                .ok_or("Give a positive issue comment ID.")?,
            body: input["body"].as_str().ok_or("Give the comment body.")?.to_owned(),
        },
        "github_issue_add_labels" => W::AddLabels {
            repository,
            number: number()?,
            labels: list("labels")?,
        },
        "github_issue_remove_labels" => W::RemoveLabels {
            repository,
            number: number()?,
            labels: list("labels")?,
        },
        "github_issue_set_assignees" => W::SetAssignees {
            repository,
            number: number()?,
            assignees: list("assignees")?,
        },
        "github_issue_close" => W::Close {
            repository,
            number: number()?,
        },
        "github_issue_reopen" => W::Reopen {
            repository,
            number: number()?,
        },
        _ => return Err("Unsupported GitHub issue write.".into()),
    };
    action.validate().map_err(|error| error.to_string())?;
    Ok(action)
}

// Preserve a valid JSON envelope and its coverage flags at the chat transport limit.
fn github_result(mut value: Value) -> Result<String, String> {
    loop {
        let text = serde_json::to_string(&value).map_err(|error| error.to_string())?;
        if text.len() <= vults_chat::tools::MAX_RESULT {
            return Ok(text);
        }
        let field = ["log", "diff"]
            .into_iter()
            .find(|key| value[*key].as_str().is_some_and(|text| !text.is_empty()));
        let Some(field) = field else {
            return Err("GitHub returned more context than the chat can hold. Request a narrower page or a separate diff or log.".into());
        };
        let text = value[field].as_str().expect("text field selected");
        let mut end = text.len() / 2;
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        value[field] = json!(&text[..end]);
        value["truncated"] = json!(true);
        value["coverage"] = json!("partial-byte-limit");
    }
}

fn positive_page(input: &Value, key: &str) -> bool {
    input[key].as_u64().is_some_and(|page| (1..=100).contains(&page))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fake(Value);

    impl Source for Fake {
        fn view(&self) -> Option<Value> {
            Some(self.0.clone())
        }
        fn usage(&self) -> Value {
            json!([{ "agent": "claude", "minutes": 300, "used_percent": 42, "resets_at": null }])
        }
    }

    #[test]
    fn github_results_remain_valid_json_with_explicit_transport_coverage() {
        let value = json!({"source":{"repository":"org/repo"},"log":"\u{1}".repeat(30000),"truncated":false,"coverage":"complete"});
        let text = github_result(value).unwrap();
        assert!(text.len() <= vults_chat::tools::MAX_RESULT);
        let value: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(value["truncated"], true);
        assert_eq!(value["coverage"], "partial-byte-limit");
        assert_eq!(value["source"]["repository"], "org/repo");
        assert!(github_result(json!({"items":"x".repeat(30000)})).is_err());
        assert!(
            parse_issue_write(
                "github_issue_edit",
                &json!({"repository":"org/repo","number":7,"title":12,"body":"valid"})
            )
            .is_err()
        );
        assert!(
            parse_issue_write(
                "github_issue_close",
                &json!({"repository":"org/repo","number":7,"extra":"ignored"})
            )
            .is_err()
        );
    }

    #[tokio::test]
    async fn a_write_changed_after_approval_never_reaches_the_service() {
        struct Writes(std::sync::atomic::AtomicUsize);
        impl Source for Writes {
            fn view(&self) -> Option<Value> {
                None
            }
            fn usage(&self) -> Value {
                Value::Null
            }
            fn github_issue_write_prepare(
                &self,
                _: vults_connectors::github::IssueWrite,
            ) -> BoxFuture<'static, Result<Value, String>> {
                Box::pin(async {
                    Ok(
                        json!({"source":{"account":"person","host":"github.com"},"issue":{"title":"Issue","state":"open"},"related":{"comments":"huge history".repeat(5000)}}),
                    )
                })
            }
            fn github_issue_write_execute(
                &self,
                _: vults_connectors::github::IssueWrite,
                _: Value,
                _: String,
            ) -> BoxFuture<'static, Result<Value, String>> {
                self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Box::pin(async { Ok(json!({"status":"verified"})) })
            }
        }
        let source = Arc::new(Writes(std::sync::atomic::AtomicUsize::new(0)));
        let tools = all(source.clone());
        let tool = tool(&tools, "github_issue_comment");
        let input = json!({"repository":"org/repo","number":7,"body":"exact approved content"});
        let prepared = tool.approval(&input).unwrap().await.unwrap();
        assert!(prepared.full.contains("exact approved content"));
        assert!(!prepared.full.contains("huge history"));
        assert!(
            tool.call_approved(
                &json!({"repository":"org/repo","number":7,"body":"changed"}),
                &prepared.context
            )
            .await
            .is_err()
        );
        assert_eq!(source.0.load(std::sync::atomic::Ordering::SeqCst), 0);
        tool.call_approved(&input, &prepared.context).await.unwrap();
        assert_eq!(source.0.load(std::sync::atomic::Ordering::SeqCst), 1);
    }

    fn view() -> Value {
        json!({
            "sessions": [
                { "id": "a", "agent": "claude", "project": "vults", "status": "approval", "card": true,
                  "waiting": true, "step": null, "steps": ["Read lib.rs"] },
                { "id": "b", "agent": "codex", "project": "site", "status": "working", "card": false,
                  "waiting": false, "step": "Running cargo test",
                  "steps": ["Edit main.rs", "Running export GITHUB_TOKEN=ghp_secret", "Running cargo test"] },
                // A card queued behind the one in front: `waiting`, not `card`.
                { "id": "c", "agent": "claude", "project": "docs", "status": "approval", "card": false,
                  "waiting": true, "step": null, "steps": [] },
                { "id": "d", "agent": "other", "agent_name": "antigravity", "project": "notes", "status": "idle",
                  "card": false, "waiting": false, "step": null, "steps": [] },
            ],
            "approval": { "session": "a", "agent": "claude", "tool": "Bash", "target": "Bash · TOKEN=abc123 ./deploy.sh" },
            "boards": [{ "connector": "github", "rows": [{ "name": "vults#12", "title": "Nest" }] }],
        })
    }

    fn tool(tools: &Tools, name: &str) -> Arc<dyn Tool> {
        tools.iter().find(|t| t.name() == name).unwrap().clone()
    }

    async fn call(tools: &Tools, name: &str, input: Value) -> Value {
        serde_json::from_str(&tool(tools, name).call(&input).await.unwrap()).unwrap()
    }

    /// A call the model reads as an error.
    async fn error(tools: &Tools, name: &str, input: Value) -> Value {
        serde_json::from_str(&tool(tools, name).call(&input).await.unwrap_err()).unwrap()
    }

    #[tokio::test]
    async fn what_is_codex_doing_is_answered_from_the_island() {
        let tools = all(Arc::new(Fake(view())));
        let list = call(&tools, "flock_list", json!({})).await;
        let codex = &list["sessions"][1];
        assert_eq!(
            (codex["agent"].as_str(), codex["doing"].as_str()),
            (Some("codex"), Some("Running cargo test"))
        );
        let detail = call(&tools, "session_detail", json!({ "session": "Codex" })).await;
        let steps = detail["steps"].as_array().unwrap();
        assert_eq!(steps.len(), 3);
        // A secret in a step never leaves.
        assert!(!detail.to_string().contains("ghp_secret"), "{detail}");
        // Two sessions go by "claude": the project is asked for.
        let two = error(&tools, "session_detail", json!({ "session": "claude code" })).await;
        assert!(two["error"].as_str().unwrap().contains("More than one"));
        // Another tool goes by its own name, said the way people say it.
        assert_eq!(
            call(&tools, "session_detail", json!({ "session": "Antigravity IDE" })).await["project"],
            "notes"
        );
        let none = error(&tools, "session_detail", json!({ "session": "aider" })).await;
        assert!(none["error"].is_string());
    }

    #[tokio::test]
    async fn the_cards_waiting_name_what_they_ask_with_secrets_redacted() {
        let tools = all(Arc::new(Fake(view())));
        let asks = call(&tools, "pending_asks", json!({})).await;
        let waiting = asks["waiting"].as_array().unwrap();
        assert_eq!(waiting.len(), 2);
        let front = waiting[0]["waiting_on"].as_str().unwrap();
        assert!(
            front.starts_with("Bash · ") && !front.contains("abc123"),
            "{front}"
        );
        // A card not in front: the island doesn't show its target, nor does the tool.
        assert_eq!(waiting[1]["waiting_on"], "a card");
    }

    #[test]
    fn the_model_gets_what_is_open_and_only_the_checks_that_did_not_pass() {
        let rows = json!([
            { "name": "app#1", "group": "yours", "facts": { "runs": [
                { "name": "test", "state": "passed" }, { "name": "visual", "state": "failed" },
                { "name": "docs", "state": "skipped" } ] } },
            { "name": "app v1", "group": "releases", "facts": { "runs": [] } },
        ]);
        let open = open_rows(&rows);
        assert_eq!(open.as_array().unwrap().len(), 1);
        assert_eq!(
            open[0]["facts"]["runs"],
            json!([{ "name": "visual", "state": "failed" }])
        );
    }

    #[tokio::test]
    async fn usage_and_github_come_through_and_github_is_outside_content() {
        let tools = all(Arc::new(Fake(view())));
        assert_eq!(
            call(&tools, "usage", json!({})).await["windows"][0]["used_percent"],
            42
        );
        assert_eq!(
            call(&tools, "github_snapshot", json!({})).await["rows"][0]["name"],
            "vults#12"
        );
        assert_eq!(
            call(&tools, "github_snapshot", json!({})).await["source"]["mode"],
            "cached_snapshot"
        );
        assert!(tool(&tools, "github_snapshot").reads_outside());
        assert!(!tool(&tools, "flock_list").reads_outside());
        let off = all(Arc::new(Fake(json!({ "sessions": [] }))));
        assert!(error(&off, "github_snapshot", json!({})).await["error"].is_string());
        assert_eq!(call(&off, "flock_list", json!({})).await["sessions"], json!([]));
        // No view yet (the app just started): an empty flock, not a failure.
        struct Nothing;
        impl Source for Nothing {
            fn view(&self) -> Option<Value> {
                None
            }
            fn usage(&self) -> Value {
                json!([])
            }
        }
        let none = all(Arc::new(Nothing));
        assert_eq!(call(&none, "pending_asks", json!({})).await["waiting"], json!([]));
    }

    #[tokio::test]
    async fn live_github_work_item_requires_its_switch_and_returns_outside_content() {
        struct Live(std::sync::atomic::AtomicBool, std::sync::atomic::AtomicUsize);
        impl Source for Live {
            fn view(&self) -> Option<Value> {
                Some(view())
            }
            fn usage(&self) -> Value {
                Value::Null
            }
            fn github_work_item_allowed(&self) -> bool {
                self.0.load(std::sync::atomic::Ordering::SeqCst)
            }
            fn github_work_item(
                &self,
                repository: String,
                number: u32,
                cursors: GithubCursors,
            ) -> BoxFuture<'static, Result<Value, String>> {
                self.1.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Box::pin(async move {
                    Ok(json!({
                        "repository":repository,"number":number,"commentCursor":cursors.comment,"fileCursor":cursors.file,
                        "reviewCursor":cursors.review,"checkCursor":cursors.check,"threadCursor":cursors.thread
                    }))
                })
            }
            fn github_search_repositories(
                &self,
                query: String,
                cursor: Option<String>,
            ) -> BoxFuture<'static, Result<Value, String>> {
                self.1.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Box::pin(async move { Ok(json!({ "query": query, "cursor": cursor, "repositories": [] })) })
            }
            fn github_read(
                &self,
                operation: String,
                input: Value,
            ) -> BoxFuture<'static, Result<Value, String>> {
                self.1.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Box::pin(async move { Ok(json!({"operation":operation,"input":input})) })
            }
        }
        let source = Arc::new(Live(
            std::sync::atomic::AtomicBool::new(false),
            std::sync::atomic::AtomicUsize::new(0),
        ));
        let tools = all(source.clone());
        let work_item = tool(&tools, "github_work_item");
        let repository_search = tool(&tools, "github_search_repositories");
        let actions = tool(&tools, "github_actions_jobs");
        let account = tool(&tools, "github_account");
        let issue_writes = [
            "github_issue_create",
            "github_issue_edit",
            "github_issue_comment",
            "github_issue_comment_edit",
            "github_issue_add_labels",
            "github_issue_remove_labels",
            "github_issue_set_assignees",
            "github_issue_close",
            "github_issue_reopen",
        ];
        for name in issue_writes {
            let write = tool(&tools, name);
            assert!(write.reads_outside());
            assert_eq!(write.activity_mode(), Some("live"));
            assert_eq!(write.schema()["additionalProperties"], false);
            assert!(
                realtime(Arc::new(Fake(view())))
                    .iter()
                    .all(|tool| tool.name() != name)
            );
        }
        assert!(
            error(
                &tools,
                "github_issue_close",
                json!({"repository":"org/repo","number":7})
            )
            .await["error"]
                .as_str()
                .unwrap()
                .contains("approval card")
        );
        for github_tool in tools.iter().filter(|tool| tool.name().starts_with("github_")) {
            assert!(github_tool.reads_outside(), "{}", github_tool.name());
            assert_eq!(
                github_tool.activity_mode(),
                Some(if github_tool.name() == "github_snapshot" {
                    "cached_snapshot"
                } else {
                    "live"
                })
            );
        }
        assert!(work_item.reads_outside());
        assert_eq!(work_item.schema()["required"], json!(["repository", "number"]));
        assert!(repository_search.reads_outside());
        assert_eq!(repository_search.schema()["required"], json!(["query"]));
        assert_eq!(
            actions.schema()["required"],
            json!(["repository", "runId", "attempt"])
        );
        assert!(actions.reads_outside());
        assert_eq!(account.schema()["required"], json!([]));
        assert!(account.reads_outside());
        assert!(
            !realtime(Arc::new(Fake(view())))
                .iter()
                .any(|tool| tool.name() == "github_account")
        );
        assert!(
            !realtime(Arc::new(Fake(view())))
                .iter()
                .any(|tool| tool.name().starts_with("github_actions_"))
        );
        assert!(
            error(
                &tools,
                "github_work_item",
                json!({ "repository": "team/app", "number": 12 })
            )
            .await["error"]
                .is_string()
        );
        assert_eq!(source.1.load(std::sync::atomic::Ordering::SeqCst), 0);
        assert!(
            error(&tools, "github_search_repositories", json!({ "query": "vults" })).await["error"]
                .is_string()
        );
        assert_eq!(source.1.load(std::sync::atomic::Ordering::SeqCst), 0);
        source.0.store(true, std::sync::atomic::Ordering::SeqCst);
        assert_eq!(
            call(
                &tools,
                "github_actions_jobs",
                json!({"repository":"team/app","runId":42,"attempt":1})
            )
            .await["operation"],
            "github_actions_jobs"
        );
        assert_eq!(
            call(
                &tools,
                "github_work_item",
                json!({ "repository": "team/app", "number": 12 })
            )
            .await,
            json!({ "repository": "team/app", "number": 12, "commentCursor": null, "fileCursor": null, "reviewCursor":null,"checkCursor":null,"threadCursor":null })
        );
        assert_eq!(
            call(
                &tools,
                "github_work_item",
                json!({ "repository": "team/app", "number": 12, "commentCursor": "discussion-page-2" })
            )
            .await,
            json!({ "repository": "team/app", "number": 12, "commentCursor": "discussion-page-2", "fileCursor": null,"reviewCursor":null,"checkCursor":null,"threadCursor":null })
        );
        assert_eq!(
            call(
                &tools,
                "github_work_item",
                json!({ "repository": "team/app", "number": 12, "fileCursor": "files-page-2" })
            )
            .await,
            json!({ "repository": "team/app", "number": 12, "commentCursor": null, "fileCursor": "files-page-2","reviewCursor":null,"checkCursor":null,"threadCursor":null })
        );
        assert_eq!(
            call(&tools, "github_search_repositories", json!({ "query": "vults" })).await,
            json!({ "query": "vults", "cursor": null, "repositories": [] })
        );
        assert_eq!(
            call(
                &tools,
                "github_search_repositories",
                json!({ "query": "vults", "cursor": "cursor-page-2" })
            )
            .await,
            json!({ "query": "vults", "cursor": "cursor-page-2", "repositories": [] })
        );
        source.0.store(false, std::sync::atomic::Ordering::SeqCst);
        assert!(
            error(
                &tools,
                "github_work_item",
                json!({ "repository": "team/app", "number": 12 })
            )
            .await["error"]
                .is_string()
        );
        assert!(
            error(&tools, "github_search_repositories", json!({ "query": "vults" })).await["error"]
                .is_string()
        );
        assert_eq!(source.1.load(std::sync::atomic::Ordering::SeqCst), 6);
    }

    #[test]
    fn the_snapshot_reads_the_cores_own_view() {
        use vults_core::{AgentEvent, AgentUpdate, Ask, Input, RequestId, SessionKey, State, Step, Terminal};
        let mut state = State::default();
        let now = std::time::Instant::now();
        let update = |session: &str, agent, event| {
            Input::Agent(AgentUpdate {
                session: SessionKey {
                    agent,
                    session_id: session.into(),
                },
                cwd: Some(format!("/home/me/{session}")),
                terminal: Terminal::default(),
                agent_id: None,
                call_id: None,
                event,
                received: None,
            })
        };
        let codex = vults_core::AgentKind::Codex;
        let claude = vults_core::AgentKind::Claude;
        vults_core::reduce(
            &mut state,
            update(
                "site",
                codex,
                AgentEvent::ToolStarted(Step {
                    activity: vults_core::Activity::Run,
                    tool: "Bash".into(),
                    detail: Some("cargo test".into()),
                }),
            ),
            now,
        );
        vults_core::reduce(
            &mut state,
            update(
                "vults",
                claude,
                AgentEvent::PermissionRequested {
                    request: RequestId("r".into()),
                    tool: "Bash".into(),
                    target: "Bash · TOKEN=abc123 ./deploy.sh".into(),
                    ask: Ask::default(),
                },
            ),
            now,
        );
        // A second card, queued behind the first.
        vults_core::reduce(
            &mut state,
            update(
                "docs",
                claude,
                AgentEvent::PermissionRequested {
                    request: RequestId("r2".into()),
                    tool: "Edit".into(),
                    target: "Edit · README.md".into(),
                    ask: Ask::default(),
                },
            ),
            now,
        );
        let flock = flock(&serde_json::to_value(state.view()).unwrap());
        let queued = flock.sessions.iter().find(|s| s.project == "docs").unwrap();
        assert!(queued.waiting.is_some(), "a queued card waits too: {queued:?}");
        let codex = flock.sessions.iter().find(|s| s.agent == "codex").unwrap();
        assert_eq!(codex.project, "site");
        assert!(
            codex.doing.as_deref().is_some_and(|d| d.contains("cargo test")),
            "{codex:?}"
        );
        let claude = flock.sessions.iter().find(|s| s.project == "vults").unwrap();
        let (tool, target) = claude.waiting.clone().unwrap();
        assert_eq!(tool, "Bash");
        assert!(
            target.contains("deploy.sh") && !target.contains("abc123"),
            "{target}"
        );
    }
    #[tokio::test]
    async fn snapshot_switch_is_checked_on_every_call_through_scoped_sources() {
        struct Switched(std::sync::atomic::AtomicBool);
        impl Source for Switched {
            fn view(&self) -> Option<Value> {
                Some(view())
            }
            fn usage(&self) -> Value {
                Value::Null
            }
            fn github_snapshot_allowed(&self) -> bool {
                self.0.load(std::sync::atomic::Ordering::SeqCst)
            }
        }
        let source = Arc::new(Switched(std::sync::atomic::AtomicBool::new(true)));
        let scoped = Arc::new(crate::mentions::Source {
            source: source.clone(),
            targets: Vec::new(),
        });
        let tools = all(scoped);
        assert!(call(&tools, "github_snapshot", json!({})).await["rows"].is_array());
        source.0.store(false, std::sync::atomic::Ordering::SeqCst);
        assert!(
            error(&tools, "github_snapshot", json!({})).await["error"]
                .as_str()
                .unwrap()
                .contains("disabled")
        );
        assert!(call(&tools, "flock_list", json!({})).await["sessions"].is_array());
        source.0.store(true, std::sync::atomic::Ordering::SeqCst);
        assert!(call(&tools, "github_snapshot", json!({})).await["rows"].is_array());
    }
}
