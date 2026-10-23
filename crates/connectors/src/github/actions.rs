//! Actions controls use exact run attempts and a durable admission before a non-idempotent POST.

use super::{
    Digest, GhApi, GithubApi, Request, Sha256, bounded_text, checked_repository, commit, gated, gh_api_json,
    github_host, write_repository,
};
use crate::{Error, Gate};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ActionsPhase {
    Queued,
    Running,
    Waiting,
    Cancelled,
    Completed,
}

pub(super) fn phase(status: &Value, conclusion: &Value) -> Result<ActionsPhase, Error> {
    match status.as_str() {
        Some("queued" | "requested" | "pending") if conclusion.is_null() => Ok(ActionsPhase::Queued),
        Some("in_progress") if conclusion.is_null() => Ok(ActionsPhase::Running),
        Some("waiting") if conclusion.is_null() => Ok(ActionsPhase::Waiting),
        Some("completed") if conclusion == "cancelled" => Ok(ActionsPhase::Cancelled),
        Some("completed") if conclusion.as_str().is_some_and(|s| !s.is_empty()) => {
            Ok(ActionsPhase::Completed)
        }
        _ => Err(Error::Partial(
            "GitHub returned an incomplete or unknown Actions state.".into(),
        )),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(
    tag = "operation",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum ActionsControl {
    RerunFailed {
        repository: String,
        run_id: u64,
        attempt: u32,
        head_sha: String,
    },
    RerunRun {
        repository: String,
        run_id: u64,
        attempt: u32,
        head_sha: String,
    },
    RerunJob {
        repository: String,
        run_id: u64,
        attempt: u32,
        head_sha: String,
        job_id: u64,
    },
    Cancel {
        repository: String,
        run_id: u64,
        attempt: u32,
        head_sha: String,
    },
}
impl ActionsControl {
    fn target(&self) -> (&str, u64, u32, &str) {
        match self {
            Self::RerunFailed {
                repository,
                run_id,
                attempt,
                head_sha,
            }
            | Self::RerunRun {
                repository,
                run_id,
                attempt,
                head_sha,
            }
            | Self::RerunJob {
                repository,
                run_id,
                attempt,
                head_sha,
                ..
            }
            | Self::Cancel {
                repository,
                run_id,
                attempt,
                head_sha,
            } => (repository, *run_id, *attempt, head_sha),
        }
    }
    pub fn repository(&self) -> &str {
        self.target().0
    }
    pub fn run_id(&self) -> u64 {
        self.target().1
    }
    pub fn attempt(&self) -> u32 {
        self.target().2
    }
    pub fn head_sha(&self) -> &str {
        self.target().3
    }
    pub fn tool_name(&self) -> &'static str {
        match self {
            Self::RerunFailed { .. } => "github_actions_rerun_failed",
            Self::RerunRun { .. } => "github_actions_rerun_run",
            Self::RerunJob { .. } => "github_actions_rerun_job",
            Self::Cancel { .. } => "github_actions_cancel",
        }
    }
    pub fn request_hash(&self) -> String {
        Sha256::digest(serde_json::to_vec(self).expect("serializable control"))
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect()
    }
    pub fn validate(&self) -> Result<(), Error> {
        checked_repository(self.repository())?;
        if self.run_id() == 0
            || self.attempt() == 0
            || self.attempt() == u32::MAX
            || self.head_sha().len() != 40
            || !commit(self.head_sha())
            || self.head_sha().bytes().any(|c| c.is_ascii_uppercase())
            || matches!(self, Self::RerunJob { job_id: 0, .. })
        {
            return Err(Error::Other(
                "Give a positive run, attempt and job ID and the exact lowercase 40-digit commit SHA.".into(),
            ));
        }
        Ok(())
    }
}

/// An exact historical attempt, or the latest run when no attempt was selected.
pub async fn actions_run(repository: &str, run_id: u64, attempt: Option<u32>) -> Result<Value, Error> {
    checked_repository(repository)?;
    if run_id == 0 || attempt == Some(0) {
        return Err(Error::Other("Give a positive workflow run and attempt.".into()));
    }
    let path = format!(
        "repos/{repository}/actions/runs/{run_id}{}",
        attempt.map(|n| format!("/attempts/{n}")).unwrap_or_default()
    );
    let value = gh_api_json(&path, 512 * 1024).await?;
    let host = value["html_url"]
        .as_str()
        .and_then(github_host)
        .ok_or_else(|| Error::Partial("GitHub omitted the run host.".into()))?;
    let run = parse_run(&value, repository, host, run_id, None)?;
    if attempt.is_some_and(|n| run["attempt"].as_u64() != Some(u64::from(n))) {
        return Err(Error::Partial("GitHub returned a different run attempt.".into()));
    }
    Ok(
        json!({"source":{"provider":"github","mode":"live","host":host,"repository":repository,"url":run["url"],"fetchedAt":super::unix_now()},"run":run,"latestRequested":attempt.is_none()}),
    )
}

pub async fn prepare_actions_control(gate: &dyn Gate, action: &ActionsControl) -> Result<Value, Error> {
    prepare_using(gate, action, &GhApi).await
}

/// `admit` must durably reserve this run attempt after the human card, before sending the POST.
pub async fn execute_actions_control<F, Fut>(
    gate: &dyn Gate,
    action: &ActionsControl,
    expected: &Value,
    hash: &str,
    admit: F,
) -> Result<Value, Error>
where
    F: FnOnce() -> Fut + Send,
    Fut: std::future::Future<Output = Result<(), Error>> + Send,
{
    execute_using(gate, action, expected, hash, &GhApi, admit).await
}

async fn prepare_using(
    gate: &dyn Gate,
    action: &ActionsControl,
    api: &impl GithubApi,
) -> Result<Value, Error> {
    action.validate()?;
    inspect(gate, action, api, None).await
}

async fn execute_using<F, Fut>(
    gate: &dyn Gate,
    action: &ActionsControl,
    expected: &Value,
    hash: &str,
    api: &impl GithubApi,
    admit: F,
) -> Result<Value, Error>
where
    F: FnOnce() -> Fut + Send,
    Fut: std::future::Future<Output = Result<(), Error>> + Send,
{
    action.validate()?;
    if action.request_hash() != hash {
        return Err(Error::Other(
            "The Actions request changed after approval. Review a new card.".into(),
        ));
    }
    let host = expected["source"]["host"]
        .as_str()
        .filter(|host| github_host(&format!("https://{host}")).is_some())
        .ok_or_else(|| Error::Other("The approved GitHub host is missing. Review a new card.".into()))?;
    let before = inspect(gate, action, api, Some(host)).await?;
    if before["binding"] != expected["binding"] {
        return Err(Error::Other("The GitHub account, run, attempt, commit or job scope changed after approval. Review a new card.".into()));
    }
    let request = Request::begin(gate)?;
    admit().await?;
    let (path, body) = mutation(action);
    let mutation = api.request(Some(host), "POST", &path, body).await;
    request.finish(mutation.is_ok());
    let accepted = mutation.is_ok();
    let rejected = mutation.as_ref().err().is_some_and(definitely_rejected);
    let error = mutation
        .as_ref()
        .err()
        .map(|error| bounded_text(&vults_core::redact::secrets(&error.to_string()), 512).0);
    let after = match read_context(gate, api, action.repository(), action.run_id(), Some(host)).await {
        Ok(after) => after,
        Err(read_error) => {
            return Ok(
                json!({"status":if rejected{"rejected"}else{"uncertain"},"verified":false,"controlAccepted":accepted,"source":before["source"],"observedRun":before["run"],"observationFresh":false,"error":error,"message":format!("The Actions result could not be checked ({read_error}). Inspect GitHub before retrying.")}),
            );
        }
    };
    if ["identity", "repositoryId", "repository", "url", "canWrite"]
        .iter()
        .any(|key| before["binding"][*key] != after["binding"][*key])
        || !same_run(&before["run"], &after["run"])
    {
        return Ok(result(
            "uncertain",
            false,
            accepted,
            &after,
            error,
            "The account or run identity changed during this control. Inspect GitHub before retrying.",
        ));
    }
    if rejected {
        return Ok(result(
            "rejected",
            false,
            false,
            &after,
            error,
            "GitHub rejected this control. A fresh proposal is required after correcting the error.",
        ));
    }
    let observed = after["run"]["attempt"].as_u64().expect("validated attempt");
    if matches!(action, ActionsControl::Cancel { .. }) {
        if observed != u64::from(action.attempt()) {
            return Ok(result(
                "uncertain",
                false,
                accepted,
                &after,
                error,
                "A different attempt appeared during cancellation. Inspect GitHub before retrying.",
            ));
        }
        return Ok(match after["run"]["phase"].as_str() {
            Some("cancelled") => result(
                "verified",
                true,
                accepted,
                &after,
                error,
                "The exact run attempt is now cancelled.",
            ),
            Some("completed") => result(
                "completed_without_cancellation",
                false,
                accepted,
                &after,
                error,
                "The run completed; cancellation was not confirmed.",
            ),
            _ if accepted => result(
                "accepted_pending_verification",
                false,
                true,
                &after,
                error,
                "GitHub accepted the cancellation request, but this attempt is still active. Inspect the latest run; do not resend.",
            ),
            _ => result(
                "uncertain",
                false,
                false,
                &after,
                error,
                "The cancellation outcome is uncertain. Inspect the latest run; do not resend.",
            ),
        });
    }
    if accepted && observed == u64::from(action.attempt()) + 1 {
        return Ok(result(
            "verified",
            true,
            true,
            &after,
            error,
            "A new attempt for the exact commit was observed. Its phase is reported separately.",
        ));
    }
    if accepted && observed == u64::from(action.attempt()) {
        return Ok(result(
            "accepted_pending_verification",
            false,
            true,
            &after,
            error,
            "GitHub accepted the rerun, but a new attempt is not visible yet. Inspect the latest run; do not resend.",
        ));
    }
    Ok(result(
        "uncertain",
        false,
        accepted,
        &after,
        error,
        "The exact rerun scope could not be confirmed. Inspect the observed run and do not resend this control.",
    ))
}

fn result(
    status: &str,
    verified: bool,
    accepted: bool,
    context: &Value,
    error: Option<String>,
    message: &str,
) -> Value {
    json!({"status":status,"verified":verified,"controlAccepted":accepted,"source":context["source"],"observedRun":context["run"],"observationFresh":true,"error":error,"message":message})
}
fn same_run(before: &Value, after: &Value) -> bool {
    ["id", "headSha", "workflowId", "headRepositoryId", "branch"]
        .iter()
        .all(|key| before[*key] == after[*key])
}
pub(super) fn definitely_rejected(error: &Error) -> bool {
    match error {
        Error::Auth(_) | Error::Unavailable(_) | Error::RateLimited { .. } => true,
        Error::Other(message) => message
            .rsplit_once("(HTTP ")
            .and_then(|(_, tail)| tail.strip_suffix(')'))
            .and_then(|code| code.parse::<u16>().ok())
            .is_some_and(|code| (400..500).contains(&code) && !matches!(code, 408 | 409)),
        _ => false,
    }
}

async fn read_context(
    gate: &dyn Gate,
    api: &impl GithubApi,
    repository: &str,
    run_id: u64,
    host: Option<&str>,
) -> Result<Value, Error> {
    let context = write_repository(gate, api, repository, host).await?;
    let host = context.source["host"].as_str().expect("validated host");
    let value = gated(
        gate,
        api.request(
            Some(host),
            "GET",
            &format!("repos/{repository}/actions/runs/{run_id}"),
            None,
        ),
    )
    .await?;
    let run = parse_run(
        &value,
        repository,
        host,
        run_id,
        Some(&context.binding["repositoryId"]),
    )?;
    Ok(json!({"source":context.source,"binding":context.binding,"run":run}))
}

async fn inspect(
    gate: &dyn Gate,
    action: &ActionsControl,
    api: &impl GithubApi,
    host: Option<&str>,
) -> Result<Value, Error> {
    let mut current = read_context(gate, api, action.repository(), action.run_id(), host).await?;
    let run = &current["run"];
    if run["attempt"].as_u64() != Some(u64::from(action.attempt())) || run["headSha"] != action.head_sha() {
        return Err(Error::Other("The current Actions attempt or commit does not match the request. Read the latest run and propose a new control.".into()));
    }
    let terminal = matches!(run["phase"].as_str(), Some("completed" | "cancelled"));
    if matches!(action, ActionsControl::Cancel { .. }) == terminal {
        return Err(Error::Other(
            if terminal {
                "This run attempt has already finished; there is no active execution to cancel."
            } else {
                "This run attempt is still active; a second rerun must not be submitted."
            }
            .into(),
        ));
    }
    let mut scope = match action {
        ActionsControl::Cancel { .. } => json!({"kind":"entire-run-cancellation"}),
        ActionsControl::RerunRun { .. } => json!({"kind":"entire-workflow-rerun"}),
        _ => {
            let jobs = jobs(
                gate,
                api,
                action,
                current["source"]["host"].as_str().expect("host"),
            )
            .await?;
            let selected: Vec<_> = match action {
                ActionsControl::RerunJob { job_id, .. } => {
                    let job = jobs
                        .iter()
                        .find(|job| job["id"].as_u64() == Some(*job_id))
                        .ok_or_else(|| {
                            Error::Other("This job does not belong to the exact run attempt.".into())
                        })?;
                    if !failed(job) {
                        return Err(Error::Other("The selected job has not failed. Choose a failed job or explicitly rerun the whole workflow.".into()));
                    }
                    vec![job.clone()]
                }
                _ => jobs.into_iter().filter(failed).collect(),
            };
            if selected.is_empty() {
                return Err(Error::Other("This attempt has no failed jobs to rerun.".into()));
            }
            json!({"kind":if matches!(action,ActionsControl::RerunJob{..}){"selected-failed-job-and-dependents"}else{"failed-jobs-and-dependents"},"failedJobs":selected,"dependentJobs":"included-by-github","dependencyCoverage":"not-enumerated-by-the-api"})
        }
    };
    scope["runId"] = json!(action.run_id());
    scope["attempt"] = json!(action.attempt());
    scope["headSha"] = json!(action.head_sha());
    let mut binding = current["binding"].clone();
    binding["run"] = json!({"id":run["id"],"attempt":run["attempt"],"headSha":run["headSha"],"workflowId":run["workflowId"],"headRepositoryId":run["headRepositoryId"],"branch":run["branch"]});
    binding["scope"] = scope.clone();
    current["binding"] = binding;
    current["scope"] = scope;
    Ok(current)
}
fn failed(job: &Value) -> bool {
    matches!(
        job["conclusion"].as_str(),
        Some("failure" | "timed_out" | "startup_failure")
    )
}

fn parse_run(
    value: &Value,
    repository: &str,
    host: &str,
    id: u64,
    repository_id: Option<&Value>,
) -> Result<Value, Error> {
    let attempt = value["run_attempt"]
        .as_u64()
        .filter(|n| *n > 0 && *n <= u32::MAX as u64)
        .ok_or_else(|| Error::Partial("GitHub omitted the workflow run attempt.".into()))?;
    let url = value["html_url"]
        .as_str()
        .ok_or_else(|| Error::Partial("GitHub omitted the workflow run URL.".into()))?;
    let base = format!("https://{host}/{repository}/actions/runs/{id}");
    if value["id"].as_u64() != Some(id)
        || value["workflow_id"].as_u64().is_none_or(|n| n == 0)
        || !value["name"].is_string()
        || !value["head_sha"].as_str().is_some_and(commit)
        || !value["repository"]["id"].as_u64().is_some_and(|n| n > 0)
        || repository_id.is_some_and(|expected| *expected != value["repository"]["id"])
        || !value["repository"]["full_name"]
            .as_str()
            .is_some_and(|name| name.eq_ignore_ascii_case(repository))
        || !(url.eq_ignore_ascii_case(&base)
            || url.eq_ignore_ascii_case(&format!("{base}/attempts/{attempt}")))
    {
        return Err(Error::Partial(
            "GitHub returned a different or incomplete workflow run or repository.".into(),
        ));
    }
    let state = phase(&value["status"], &value["conclusion"])?;
    let (name, truncated) = bounded_text(value["name"].as_str().expect("name"), 256);
    Ok(
        json!({"id":id,"workflowId":value["workflow_id"],"name":name,"nameTruncated":truncated,"attempt":attempt,"headSha":value["head_sha"],"headRepositoryId":value["head_repository"]["id"],"headRepository":value["head_repository"]["full_name"],"branch":value["head_branch"],"phase":state,"status":value["status"],"conclusion":value["conclusion"],"url":url,"createdAt":value["created_at"],"updatedAt":value["updated_at"]}),
    )
}

async fn jobs(
    gate: &dyn Gate,
    api: &impl GithubApi,
    action: &ActionsControl,
    host: &str,
) -> Result<Vec<Value>, Error> {
    let mut kept = Vec::new();
    let mut ids = std::collections::HashSet::new();
    let mut total = None;
    for page in 1..=10 {
        let path = format!(
            "repos/{}/actions/runs/{}/attempts/{}/jobs?per_page=100&page={page}",
            action.repository(),
            action.run_id(),
            action.attempt()
        );
        let value = gated(gate, api.request(Some(host), "GET", &path, None)).await?;
        let count = value["total_count"]
            .as_u64()
            .filter(|n| *n <= 1000)
            .ok_or_else(|| {
                Error::Partial("GitHub omitted job coverage or the attempt exceeds 1,000 jobs.".into())
            })?;
        if total.is_some_and(|n| n != count) {
            return Err(Error::Partial(
                "Job coverage changed while this attempt was being read.".into(),
            ));
        }
        total = Some(count);
        let values = value["jobs"]
            .as_array()
            .filter(|jobs| jobs.len() <= 100)
            .ok_or_else(|| Error::Partial("GitHub omitted the run-attempt job page.".into()))?;
        for job in values {
            let id = job["id"]
                .as_u64()
                .filter(|n| *n > 0)
                .ok_or_else(|| Error::Partial("GitHub omitted a job identity.".into()))?;
            if !ids.insert(id)
                || job["run_id"].as_u64() != Some(action.run_id())
                || job
                    .get("run_attempt")
                    .is_some_and(|n| n.as_u64() != Some(u64::from(action.attempt())))
                || job.get("head_sha").is_some_and(|sha| sha != action.head_sha())
                || !job["name"].is_string()
            {
                return Err(Error::Partial(
                    "GitHub returned duplicate jobs or jobs outside the exact run attempt.".into(),
                ));
            }
            let state = phase(&job["status"], &job["conclusion"])?;
            let (name, truncated) = bounded_text(job["name"].as_str().expect("name"), 256);
            kept.push(json!({"id":id,"name":name,"nameTruncated":truncated,"phase":state,"conclusion":job["conclusion"]}));
        }
        if kept.len() as u64 == count {
            return Ok(kept);
        }
        if values.len() < 100 || kept.len() as u64 > count {
            return Err(Error::Partial("GitHub returned an incomplete job page.".into()));
        }
    }
    Err(Error::Partial(
        "The run-attempt job lookup reached its 1,000-job limit.".into(),
    ))
}
fn mutation(action: &ActionsControl) -> (String, Option<Value>) {
    let root = format!("repos/{}/actions", action.repository());
    let path = match action {
        ActionsControl::RerunFailed { .. } => format!("{root}/runs/{}/rerun-failed-jobs", action.run_id()),
        ActionsControl::RerunRun { .. } => format!("{root}/runs/{}/rerun", action.run_id()),
        ActionsControl::RerunJob { job_id, .. } => format!("{root}/jobs/{job_id}/rerun"),
        ActionsControl::Cancel { .. } => return (format!("{root}/runs/{}/cancel", action.run_id()), None),
    };
    (path, Some(json!({"enable_debug_logging":false})))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{collections::VecDeque, sync::Mutex, time::Duration};
    struct Step {
        method: &'static str,
        path: String,
        body: Option<Value>,
        host: Option<&'static str>,
        result: Result<Value, Error>,
    }
    struct Fake(Mutex<VecDeque<Step>>);
    impl Fake {
        fn new(steps: Vec<Step>) -> Self {
            Self(Mutex::new(steps.into()))
        }
        fn exhausted(&self) {
            assert!(
                self.0.lock().unwrap().is_empty(),
                "an expected request was not sent"
            );
        }
    }
    impl GithubApi for Fake {
        async fn request(
            &self,
            host: Option<&str>,
            method: &str,
            path: &str,
            body: Option<Value>,
        ) -> Result<Value, Error> {
            let step = self
                .0
                .lock()
                .unwrap()
                .pop_front()
                .expect("unexpected GitHub request or duplicate POST");
            assert_eq!(
                (host, method, path, body),
                (step.host, step.method, step.path.as_str(), step.body)
            );
            step.result
        }
    }
    struct Budget(Mutex<usize>);
    impl Gate for Budget {
        fn allow(&self, _: &str) -> bool {
            let mut n = self.0.lock().unwrap();
            if *n == 0 {
                false
            } else {
                *n -= 1;
                true
            }
        }
        fn done(&self, _: &str, _: bool, _: Duration) {}
    }
    fn gate() -> Budget {
        Budget(Mutex::new(100))
    }
    fn get(path: &str, result: Value) -> Step {
        Step {
            method: "GET",
            path: path.into(),
            body: None,
            host: Some("github.com"),
            result: Ok(result),
        }
    }
    fn run(attempt: u32, status: &str, conclusion: Value) -> Value {
        json!({"id":12,"workflow_id":3,"name":"CI","run_attempt":attempt,"head_sha":"b".repeat(40),"head_branch":"main","status":status,"conclusion":conclusion,"html_url":"https://github.com/org/repo/actions/runs/12","repository":{"id":20,"full_name":"org/repo"},"head_repository":{"id":20,"full_name":"org/repo"}})
    }
    fn job(id: u64, name: &str, conclusion: &str) -> Value {
        json!({"id":id,"run_id":12,"run_attempt":1,"name":name,"head_sha":"b".repeat(40),"status":"completed","conclusion":conclusion})
    }
    fn action(kind: &str) -> ActionsControl {
        serde_json::from_value(
            json!({"operation":kind,"repository":"org/repo","runId":12,"attempt":1,"headSha":"b".repeat(40)}),
        )
        .unwrap()
    }
    fn job_action() -> ActionsControl {
        ActionsControl::RerunJob {
            repository: "org/repo".into(),
            run_id: 12,
            attempt: 1,
            head_sha: "b".repeat(40),
            job_id: 55,
        }
    }
    fn context(value: Value, initial: bool) -> Vec<Step> {
        let mut user = get(
            "user",
            json!({"id":1,"login":"person","html_url":"https://github.com/person"}),
        );
        if initial {
            user.host = None;
        }
        vec![
            user,
            get(
                "repos/org/repo",
                json!({"id":20,"full_name":"org/repo","html_url":"https://github.com/org/repo","permissions":{"push":true},"archived":false}),
            ),
            get("repos/org/repo/actions/runs/12", value),
        ]
    }
    fn inspection(action: &ActionsControl, value: Value, initial: bool) -> Vec<Step> {
        let mut steps = context(value, initial);
        if matches!(
            action,
            ActionsControl::RerunFailed { .. } | ActionsControl::RerunJob { .. }
        ) {
            steps.push(get(
                "repos/org/repo/actions/runs/12/attempts/1/jobs?per_page=100&page=1",
                json!({"total_count":2,"jobs":[job(55,"build","failure"),job(56,"lint","success")]}),
            ));
        }
        steps
    }
    async fn preview(action: &ActionsControl, value: Value) -> Value {
        let api = Fake::new(inspection(action, value, true));
        let value = prepare_using(&gate(), action, &api).await.unwrap();
        api.exhausted();
        value
    }
    fn post(action: &ActionsControl, result: Result<Value, Error>) -> Step {
        let (path, body) = mutation(action);
        Step {
            host: Some("github.com"),
            method: "POST",
            path,
            body,
            result,
        }
    }

    #[test]
    fn exact_control_endpoints_and_debug_logging_are_fixed() {
        for (action, path, body) in [
            (
                action("rerun_failed"),
                "repos/org/repo/actions/runs/12/rerun-failed-jobs",
                Some(json!({"enable_debug_logging":false})),
            ),
            (
                action("rerun_run"),
                "repos/org/repo/actions/runs/12/rerun",
                Some(json!({"enable_debug_logging":false})),
            ),
            (
                job_action(),
                "repos/org/repo/actions/jobs/55/rerun",
                Some(json!({"enable_debug_logging":false})),
            ),
            (action("cancel"), "repos/org/repo/actions/runs/12/cancel", None),
        ] {
            assert_eq!(mutation(&action), (path.into(), body));
        }
        assert_ne!(
            action("rerun_run").request_hash(),
            action("rerun_failed").request_hash()
        );
    }
    #[tokio::test]
    async fn every_control_is_sent_once_and_reports_the_observed_phase() {
        for action in [
            action("rerun_failed"),
            action("rerun_run"),
            job_action(),
            action("cancel"),
        ] {
            let cancel = matches!(action, ActionsControl::Cancel { .. });
            let initial = if cancel {
                run(1, "in_progress", Value::Null)
            } else {
                run(1, "completed", json!("failure"))
            };
            let expected = preview(&action, initial.clone()).await;
            if matches!(
                action,
                ActionsControl::RerunFailed { .. } | ActionsControl::RerunJob { .. }
            ) {
                assert_eq!(expected["scope"]["failedJobs"].as_array().unwrap().len(), 1);
                assert_eq!(expected["scope"]["dependentJobs"], "included-by-github");
            }
            let mut steps = inspection(&action, initial, false);
            steps.push(post(&action, Ok(Value::Null)));
            steps.extend(context(
                if cancel {
                    run(1, "completed", json!("cancelled"))
                } else {
                    run(2, "queued", Value::Null)
                },
                false,
            ));
            let api = Fake::new(steps);
            let admitted = std::sync::atomic::AtomicUsize::new(0);
            let result = execute_using(
                &gate(),
                &action,
                &expected,
                &action.request_hash(),
                &api,
                || async {
                    admitted.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    Ok(())
                },
            )
            .await
            .unwrap();
            assert_eq!(result["status"], "verified");
            assert_eq!(
                result["observedRun"]["phase"],
                if cancel { "cancelled" } else { "queued" }
            );
            assert_eq!(admitted.load(std::sync::atomic::Ordering::SeqCst), 1);
            api.exhausted();
        }
    }
    #[tokio::test]
    async fn accepted_is_distinct_from_a_verified_new_attempt_or_cancellation() {
        for (action, initial) in [
            (action("rerun_run"), run(1, "completed", json!("failure"))),
            (action("cancel"), run(1, "in_progress", Value::Null)),
        ] {
            let expected = preview(&action, initial.clone()).await;
            let mut steps = inspection(&action, initial.clone(), false);
            steps.push(post(&action, Ok(Value::Null)));
            steps.extend(context(initial, false));
            let api = Fake::new(steps);
            let result = execute_using(
                &gate(),
                &action,
                &expected,
                &action.request_hash(),
                &api,
                || async { Ok(()) },
            )
            .await
            .unwrap();
            assert_eq!(result["status"], "accepted_pending_verification");
            assert_eq!(result["verified"], false);
            assert_eq!(result["controlAccepted"], true);
            api.exhausted();
        }
    }
    #[tokio::test]
    async fn wrong_repo_attempt_commit_identity_and_scope_send_no_post() {
        let action = job_action();
        let initial = run(1, "completed", json!("failure"));
        let expected = preview(&action, initial.clone()).await;
        for changed in ["repository", "attempt", "commit", "account", "scope"] {
            let mut value = initial.clone();
            match changed {
                "repository" => value["repository"]["id"] = json!(21),
                "attempt" => value["run_attempt"] = json!(2),
                "commit" => value["head_sha"] = json!("c".repeat(40)),
                _ => {}
            }
            let mut steps = inspection(&action, value, false);
            if matches!(changed, "repository" | "attempt" | "commit") {
                steps.truncate(3);
            }
            if changed == "account" {
                steps[0].result.as_mut().unwrap()["id"] = json!(2);
            }
            if changed == "scope" {
                steps[3].result.as_mut().unwrap()["jobs"][0]["name"] = json!("different scope");
            }
            let api = Fake::new(steps);
            assert!(
                execute_using(
                    &gate(),
                    &action,
                    &expected,
                    &action.request_hash(),
                    &api,
                    || async { panic!("must not reserve stale control") }
                )
                .await
                .is_err(),
                "{changed}"
            );
            api.exhausted();
        }
        let api = Fake::new(vec![]);
        assert!(
            execute_using(&gate(), &action, &expected, "edited-hash", &api, || async {
                panic!("must not admit edited action")
            })
            .await
            .is_err()
        );
        api.exhausted();
    }
    #[tokio::test]
    async fn finished_cancellation_and_active_rerun_are_refused_without_admission() {
        for (action, value) in [
            (action("cancel"), run(1, "completed", json!("success"))),
            (action("rerun_run"), run(1, "in_progress", Value::Null)),
        ] {
            let api = Fake::new(context(value, true));
            assert!(prepare_using(&gate(), &action, &api).await.is_err());
            api.exhausted();
        }
        let action = action("cancel");
        let expected = preview(&action, run(1, "queued", Value::Null)).await;
        let mut steps = inspection(&action, run(1, "in_progress", Value::Null), false);
        steps.push(post(&action, Ok(Value::Null)));
        steps.extend(context(run(1, "completed", json!("success")), false));
        let api = Fake::new(steps);
        let result = execute_using(
            &gate(),
            &action,
            &expected,
            &action.request_hash(),
            &api,
            || async { Ok(()) },
        )
        .await
        .unwrap();
        assert_eq!(result["status"], "completed_without_cancellation");
        assert_eq!(result["observedRun"]["phase"], "completed");
        api.exhausted();
    }
    #[tokio::test]
    async fn timeout_rejection_and_unavailable_verification_never_repeat_a_post() {
        let action = action("rerun_run");
        let initial = run(1, "completed", json!("failure"));
        let expected = preview(&action, initial.clone()).await;
        for (error, after, status) in [
            (
                "GitHub took too long to answer",
                run(1, "completed", json!("failure")),
                "uncertain",
            ),
            (
                "GitHub took too long to answer",
                run(2, "in_progress", Value::Null),
                "uncertain",
            ),
            (
                "gh: forbidden (HTTP 403)",
                run(1, "completed", json!("failure")),
                "rejected",
            ),
        ] {
            let mut steps = inspection(&action, initial.clone(), false);
            steps.push(post(&action, Err(Error::Other(error.into()))));
            steps.extend(context(after, false));
            let api = Fake::new(steps);
            assert_eq!(
                execute_using(
                    &gate(),
                    &action,
                    &expected,
                    &action.request_hash(),
                    &api,
                    || async { Ok(()) }
                )
                .await
                .unwrap()["status"],
                status
            );
            api.exhausted();
        }
        let mut steps = inspection(&action, initial, false);
        steps.push(post(&action, Ok(Value::Null)));
        let mut failed = get("user", Value::Null);
        failed.result = Err(Error::Other("offline".into()));
        steps.push(failed);
        let api = Fake::new(steps);
        let result = execute_using(
            &gate(),
            &action,
            &expected,
            &action.request_hash(),
            &api,
            || async { Ok(()) },
        )
        .await
        .unwrap();
        assert_eq!(result["status"], "uncertain");
        assert_eq!(result["observationFresh"], false);
        api.exhausted();
    }
    #[tokio::test]
    async fn duplicate_admission_and_exhausted_gate_do_not_send_a_post() {
        let action = action("rerun_run");
        let initial = run(1, "completed", json!("failure"));
        let expected = preview(&action, initial.clone()).await;
        let api = Fake::new(inspection(&action, initial.clone(), false));
        assert!(
            execute_using(
                &gate(),
                &action,
                &expected,
                &action.request_hash(),
                &api,
                || async { Err(Error::Other("duplicate reservation".into())) }
            )
            .await
            .is_err()
        );
        api.exhausted();
        let api = Fake::new(inspection(&action, initial, false));
        assert!(
            execute_using(
                &Budget(Mutex::new(3)),
                &action,
                &expected,
                &action.request_hash(),
                &api,
                || async { panic!("budget must be admitted first") }
            )
            .await
            .is_err()
        );
        api.exhausted();
    }
    #[tokio::test]
    async fn invalid_job_coverage_and_wrong_job_fail_closed() {
        let action = job_action();
        let initial = run(1, "completed", json!("failure"));
        for change in ["run", "attempt", "id", "success", "coverage", "missing"] {
            let mut steps = inspection(&action, initial.clone(), true);
            let value = steps[3].result.as_mut().unwrap();
            match change {
                "run" => value["jobs"][0]["run_id"] = json!(99),
                "attempt" => value["jobs"][0]["run_attempt"] = json!(2),
                "id" => value["jobs"][0]["id"] = json!(99),
                "success" => value["jobs"][0]["conclusion"] = json!("success"),
                "coverage" => value["total_count"] = json!(3),
                _ => {
                    value.as_object_mut().unwrap().remove("jobs");
                }
            }
            let api = Fake::new(steps);
            assert!(prepare_using(&gate(), &action, &api).await.is_err(), "{change}");
            api.exhausted();
        }
    }
    #[tokio::test]
    async fn permission_loss_and_excess_attempt_advancement_never_report_success() {
        let action = action("rerun_run");
        let initial = run(1, "completed", json!("success"));
        let expected = preview(&action, initial.clone()).await;
        let mut denied = context(initial.clone(), false);
        denied.truncate(2);
        denied[1].result.as_mut().unwrap()["permissions"]["push"] = json!(false);
        let api = Fake::new(denied);
        assert!(
            execute_using(
                &gate(),
                &action,
                &expected,
                &action.request_hash(),
                &api,
                || async { panic!("permission must precede admission") }
            )
            .await
            .is_err()
        );
        api.exhausted();
        for (remaining, attempt) in [(4, 2), (100, 3)] {
            let mut steps = inspection(&action, initial.clone(), false);
            steps.push(post(&action, Ok(Value::Null)));
            if remaining > 4 {
                steps.extend(context(run(attempt, "queued", Value::Null), false));
            }
            let api = Fake::new(steps);
            let value = execute_using(
                &Budget(Mutex::new(remaining)),
                &action,
                &expected,
                &action.request_hash(),
                &api,
                || async { Ok(()) },
            )
            .await
            .unwrap();
            assert_eq!(value["status"], "uncertain");
            assert_eq!(value["verified"], false);
            api.exhausted();
        }
    }
    #[tokio::test]
    async fn failed_job_scope_follows_pages_and_rejects_duplicate_ids() {
        let action = action("rerun_failed");
        for duplicate in [false, true] {
            let mut steps = context(run(1, "completed", json!("failure")), true);
            let page: Vec<_> = (1..=100).map(|id| job(id, "job", "failure")).collect();
            steps.push(get(
                "repos/org/repo/actions/runs/12/attempts/1/jobs?per_page=100&page=1",
                json!({"total_count":101,"jobs":page}),
            ));
            steps.push(get(
                "repos/org/repo/actions/runs/12/attempts/1/jobs?per_page=100&page=2",
                json!({"total_count":101,"jobs":[job(if duplicate{1}else{101},"last","failure")]}),
            ));
            let api = Fake::new(steps);
            let result = prepare_using(&gate(), &action, &api).await;
            if duplicate {
                assert!(result.is_err());
            } else {
                assert_eq!(
                    result.unwrap()["scope"]["failedJobs"].as_array().unwrap().len(),
                    101
                );
            }
            api.exhausted();
        }
    }
    #[tokio::test]
    async fn dropping_an_inflight_control_never_reads_back_or_resends() {
        struct Hanging(Fake);
        impl GithubApi for Hanging {
            async fn request(
                &self,
                host: Option<&str>,
                method: &str,
                path: &str,
                body: Option<Value>,
            ) -> Result<Value, Error> {
                let value = self.0.request(host, method, path, body).await;
                if method == "POST" {
                    std::future::pending().await
                } else {
                    value
                }
            }
        }
        let action = action("rerun_run");
        let initial = run(1, "completed", json!("failure"));
        let expected = preview(&action, initial.clone()).await;
        let mut steps = inspection(&action, initial, false);
        steps.push(post(&action, Ok(Value::Null)));
        let api = Hanging(Fake::new(steps));
        let admitted = std::sync::atomic::AtomicUsize::new(0);
        let hash = action.request_hash();
        let gate = gate();
        let pending = execute_using(&gate, &action, &expected, &hash, &api, || async {
            admitted.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok(())
        });
        assert!(
            tokio::time::timeout(Duration::from_millis(10), pending)
                .await
                .is_err()
        );
        assert_eq!(admitted.load(std::sync::atomic::Ordering::SeqCst), 1);
        api.0.exhausted();
    }
    #[test]
    fn controls_reject_noncanonical_commit_ids_before_network_access() {
        for sha in [
            "abcdef0".to_string(),
            "a".repeat(64),
            "B".repeat(40),
            "z".repeat(40),
        ] {
            let action = ActionsControl::RerunRun {
                repository: "org/repo".into(),
                run_id: 12,
                attempt: 1,
                head_sha: sha,
            };
            assert!(action.validate().is_err());
        }
        assert!(action("rerun_run").validate().is_ok());
    }
    #[test]
    fn phases_are_explicit_and_unknown_or_contradictory_states_are_not_success() {
        assert_eq!(
            phase(&json!("queued"), &Value::Null).unwrap(),
            ActionsPhase::Queued
        );
        assert_eq!(
            phase(&json!("in_progress"), &Value::Null).unwrap(),
            ActionsPhase::Running
        );
        assert_eq!(
            phase(&json!("completed"), &json!("cancelled")).unwrap(),
            ActionsPhase::Cancelled
        );
        assert_eq!(
            phase(&json!("completed"), &json!("failure")).unwrap(),
            ActionsPhase::Completed
        );
        assert!(phase(&json!("completed"), &Value::Null).is_err());
        assert!(phase(&json!("in_progress"), &json!("failure")).is_err());
        assert!(phase(&json!("unknown"), &Value::Null).is_err());
    }
}
