//! GitHub through `gh`, the CLI the user is already logged into: we never see a token.
//!
//! One GraphQL query per poll (1 point of the 5000/hour budget) covers the user's open pull
//! requests (checks one by one, reviewers, branch, size), reviews requested from them, and the
//! checks on the default branch and the latest release of their recently pushed repositories.

mod merging;
pub use merging::{MergeControl, MergeMethod, execute_merge_control, merge_status, prepare_merge_control};

mod actions;
pub use actions::{
    ActionsControl, ActionsPhase, actions_run, execute_actions_control, prepare_actions_control,
};

mod pull_requests;
pub use pull_requests::{
    PullRequestWrite, ReviewComment, ReviewEvent, Side, execute_pull_request_write,
    prepare_pull_request_write,
};

use std::time::Duration;

use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::Command;

use crate::{
    Checks, Connector, Diff, Error, Event, Facts, Group, Level, Poll, Release, Review, Reviewer, Row, Run,
    RunState, Said, Snapshot, Story, StoryKind,
};

#[derive(Debug)]
pub struct GitHub;

const QUERY: &str = r#"query {
  viewer {
    login url
    pullRequests(first: 20, states: OPEN, orderBy: {field: UPDATED_AT, direction: DESC}) {
      nodes { number title url repository { nameWithOwner } reviewDecision
        headRefName additions deletions changedFiles updatedAt
        latestReviews(first: 5) { nodes { author { login } state } }
        reviewRequests(first: 5) { nodes { requestedReviewer { ... on User { login } } } }
        commits(last: 1) { nodes { commit { oid statusCheckRollup { state
          contexts(first: 30) { totalCount pageInfo { hasNextPage } nodes { __typename
          ... on CheckRun { name status conclusion } ... on StatusContext { context state } } } } } } } }
    }
    repositories(first: 10, ownerAffiliations: OWNER, orderBy: {field: PUSHED_AT, direction: DESC}) {
      nodes { nameWithOwner url isArchived defaultBranchRef { name target { ... on Commit {
        oid messageHeadline committedDate statusCheckRollup { state
          contexts(first: 30) { nodes { __typename
          ... on CheckRun { name status conclusion } ... on StatusContext { context state } } } } } } }
        releases(first: 1, orderBy: {field: CREATED_AT, direction: DESC}) {
          nodes { tagName name url publishedAt createdAt isPrerelease isDraft } } }
    }
  }
  search(query: "is:open is:pr review-requested:@me archived:false", type: ISSUE, first: 20) {
    nodes { ... on PullRequest { number title url repository { nameWithOwner }
      author { login } headRefName additions deletions changedFiles updatedAt } }
  }
}"#;

const WORK_ITEM_QUERY: &str = r#"query($owner: String!, $name: String!, $number: Int!, $commentsAfter: String, $filesAfter: String, $reviewsAfter: String, $checksAfter: String, $threadsAfter: String) {
  repository(owner: $owner, name: $name) {
    nameWithOwner url
    issueOrPullRequest(number: $number) {
      __typename
      ... on Issue {
        number title body url state createdAt updatedAt
        comments(first: 5, after: $commentsAfter) { totalCount pageInfo { hasNextPage endCursor } nodes { body createdAt url } }
      }
      ... on PullRequest {
        number title body url state createdAt updatedAt
        baseRefName headRefName headRefOid additions deletions changedFiles reviewDecision mergeStateStatus mergeable isDraft
        comments(first: 5, after: $commentsAfter) { totalCount pageInfo { hasNextPage endCursor } nodes { body createdAt url } }
        latestReviews(first: 5, after: $reviewsAfter) {
          totalCount pageInfo { hasNextPage endCursor }
          nodes { state body submittedAt author { ... on User { login } ... on Bot { login } } }
        }
        reviewThreads(first: 5, after: $threadsAfter) {
          totalCount pageInfo { hasNextPage endCursor }
          nodes { isResolved isOutdated path line comments(first: 3) {
            totalCount pageInfo { hasNextPage }
            nodes { body createdAt url author { ... on User { login } ... on Bot { login } } }
          } }
        }
        files(first: 10, after: $filesAfter) {
          totalCount pageInfo { hasNextPage endCursor }
          nodes { path additions deletions }
        }
        commits(last: 1) { nodes { commit { oid statusCheckRollup { state
          contexts(first: 10, after: $checksAfter) { totalCount pageInfo { hasNextPage endCursor } nodes { __typename
            ... on CheckRun { name status conclusion }
            ... on StatusContext { context state }
          } }
        } } } }
      }
    }
  }
}"#;

const REPOSITORY_SEARCH_QUERY: &str = r#"query($searchQuery: String!, $after: String) {
  viewer { login url }
  search(query: $searchQuery, type: REPOSITORY, first: 10, after: $after) {
    repositoryCount pageInfo { hasNextPage endCursor }
    nodes { ... on Repository {
      nameWithOwner url description isPrivate isArchived pushedAt
      defaultBranchRef { name }
    } }
  }
}"#;

const ACCOUNT_QUERY: &str = "query { viewer { login name url } }";

const MAX_WORK_ITEM_BYTES: usize = 1024 * 1024;

impl Connector for GitHub {
    fn id(&self) -> &'static str {
        "github"
    }

    fn interval(&self, last: &Snapshot) -> Duration {
        let running = last
            .values()
            .any(|v| matches!(v["ci"].as_str(), Some("PENDING" | "EXPECTED")));
        Duration::from_secs(if running { 60 } else { 300 })
    }

    fn poll(&self) -> Poll<'_> {
        Box::pin(async {
            let out = Command::new("gh")
                .args(["api", "graphql", "-f"])
                .arg(format!("query={QUERY}"))
                .kill_on_drop(true)
                .output();
            let out = tokio::time::timeout(Duration::from_secs(30), out)
                .await
                .map_err(|_| Error::Other("GitHub took too long to answer".into()))?
                .map_err(|e| match e.kind() {
                    std::io::ErrorKind::NotFound => {
                        Error::Unavailable("The GitHub CLI (gh) isn't installed".into())
                    }
                    _ => Error::Other(format!("can't run gh: {e}")),
                })?;
            answer(out.status.success(), &out.stdout, &out.stderr)
        })
    }

    fn diff(&self, before: &Snapshot, after: &Snapshot) -> Vec<Event> {
        diff(before, after)
    }

    fn board(&self, snapshot: &Snapshot) -> Option<Vec<Row>> {
        Some(board(snapshot))
    }

    fn identity(&self, snapshot: &Snapshot) -> Option<(String, String)> {
        let meta = snapshot.get("meta:account")?;
        Some((
            meta["login"].as_str()?.to_owned(),
            meta["host"].as_str()?.to_owned(),
        ))
    }

    fn watching(&self, snapshot: &Snapshot) -> usize {
        snapshot.keys().filter(|key| !key.starts_with("meta:")).count()
    }
}

/// Reads one issue or pull request with a fixed, labeled coverage limit.
pub async fn work_item(
    repository: &str,
    number: u32,
    comment_cursor: Option<&str>,
    file_cursor: Option<&str>,
    review_cursor: Option<&str>,
    check_cursor: Option<&str>,
    thread_cursor: Option<&str>,
) -> Result<Value, Error> {
    let (owner, name) = repository
        .split_once('/')
        .filter(|_| plain_repo(repository))
        .ok_or_else(|| Error::Other("Use a GitHub repository in owner/name form.".into()))?;
    if number == 0 || number > i32::MAX as u32 {
        return Err(Error::Other(
            "Issue or pull request number must be positive.".into(),
        ));
    }
    if comment_cursor.is_some_and(|cursor| cursor.len() > 512 || cursor.chars().any(char::is_control)) {
        return Err(Error::Other("The GitHub discussion cursor is invalid.".into()));
    }
    if file_cursor.is_some_and(|cursor| cursor.len() > 512 || cursor.chars().any(char::is_control)) {
        return Err(Error::Other("The GitHub file cursor is invalid.".into()));
    }
    for (cursor, label) in [
        (review_cursor, "review"),
        (check_cursor, "check"),
        (thread_cursor, "review thread"),
    ] {
        if cursor.is_some_and(|cursor| cursor.len() > 512 || cursor.chars().any(char::is_control)) {
            return Err(Error::Other(format!("The GitHub {label} cursor is invalid.")));
        }
    }

    let owner = owner.to_owned();
    let name = name.to_owned();
    let number = number.to_string();
    let mut variables = vec![
        ("owner", owner.as_str(), false),
        ("name", name.as_str(), false),
        ("number", number.as_str(), true),
    ];
    if let Some(cursor) = comment_cursor {
        variables.push(("commentsAfter", cursor, false));
    }
    if let Some(cursor) = file_cursor {
        variables.push(("filesAfter", cursor, false));
    }
    if let Some(cursor) = review_cursor {
        variables.push(("reviewsAfter", cursor, false));
    }
    if let Some(cursor) = check_cursor {
        variables.push(("checksAfter", cursor, false));
    }
    if let Some(cursor) = thread_cursor {
        variables.push(("threadsAfter", cursor, false));
    }
    let output = graphql(WORK_ITEM_QUERY, &variables, MAX_WORK_ITEM_BYTES).await?;
    work_item_answer(
        output.0,
        &output.1,
        &output.2,
        repository,
        std::time::SystemTime::now(),
    )
}

/// Searches only repositories visible to the account selected by `gh`.
pub async fn search_repositories(query: &str, cursor: Option<&str>) -> Result<Value, Error> {
    let query = query.trim();
    if query.is_empty() || query.chars().count() > 160 || query.chars().any(char::is_control) {
        return Err(Error::Other(
            "Use a search phrase between 1 and 160 characters, without control characters.".into(),
        ));
    }
    if cursor.is_some_and(|cursor| cursor.len() > 512 || cursor.chars().any(char::is_control)) {
        return Err(Error::Other("The GitHub search cursor is invalid.".into()));
    }
    let mut variables = vec![("searchQuery", query, false)];
    if let Some(cursor) = cursor {
        variables.push(("after", cursor, false));
    }
    let output = graphql(REPOSITORY_SEARCH_QUERY, &variables, 256 * 1024).await?;
    repository_search_answer(output.0, &output.1, &output.2, std::time::SystemTime::now())
}

/// Reads the identity selected by gh without making repository access assumptions.
pub async fn account() -> Result<Value, Error> {
    let output = graphql(ACCOUNT_QUERY, &[], 64 * 1024).await?;
    account_answer(output.0, &output.1, &output.2, std::time::SystemTime::now())
}

fn account_answer(
    success: bool,
    stdout: &[u8],
    stderr: &[u8],
    now: std::time::SystemTime,
) -> Result<Value, Error> {
    if !success {
        return Err(gh_error(false, stderr));
    }
    let data: Value = serde_json::from_slice(stdout).map_err(|_| gh_error(success, stderr))?;
    if data["errors"].as_array().is_some_and(|errors| !errors.is_empty()) {
        return Err(Error::Partial(
            "GitHub returned an incomplete account read.".into(),
        ));
    }
    let viewer = &data["data"]["viewer"];
    let login = viewer["login"]
        .as_str()
        .ok_or_else(|| Error::Partial("GitHub omitted the selected account.".into()))?;
    let url = viewer["url"]
        .as_str()
        .filter(|url| url.starts_with("https://"))
        .ok_or_else(|| Error::Partial("GitHub omitted the selected account URL.".into()))?;
    let host =
        github_host(url).ok_or_else(|| Error::Partial("GitHub returned an invalid account host.".into()))?;
    let fetched_at = now
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    Ok(
        json!({"source":{"provider":"github","mode":"live","account":login,"host":host,"url":url,"fetchedAt":fetched_at},"name":viewer["name"]}),
    )
}

/// Reads only metadata for one exact accessible repository.
pub async fn repository_details(repository: &str) -> Result<Value, Error> {
    let (owner, name) = checked_repository(repository)?;
    let value = gh_api_json(&format!("repos/{owner}/{name}"), 128 * 1024).await?;
    repository_answer(value, repository, std::time::SystemTime::now())
}

fn repository_answer(value: Value, repository: &str, now: std::time::SystemTime) -> Result<Value, Error> {
    if value["full_name"].as_str().is_none()
        || value["html_url"].as_str().is_none()
        || value["default_branch"].as_str().is_none()
        || value["private"].as_bool().is_none()
        || value["archived"].as_bool().is_none()
    {
        return Err(Error::Partial(
            "GitHub returned incomplete repository metadata.".into(),
        ));
    }
    let (description, truncated) = bounded_text(value["description"].as_str().unwrap_or_default(), 512);
    Ok(json!({
        "source":{"provider":"github","mode":"live","repository":value["full_name"],"url":value["html_url"],"fetchedAt":now.duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs()},
        "requestedRepository":repository,"repository":value["full_name"],"url":value["html_url"],"private":value["private"],"archived":value["archived"],
        "fork":value["fork"],"defaultBranch":value["default_branch"],"pushedAt":value["pushed_at"],
        "description":description,"descriptionTruncated":truncated
    }))
}

/// Reads the pull request's unified diff without fetching it as part of ordinary context.
pub async fn pull_request_diff(repository: &str, number: u32) -> Result<Value, Error> {
    let (owner, name) = checked_target(repository, number)?;
    let path = format!("repos/{owner}/{name}/pulls/{number}");
    let (success, stdout, stderr, oversized) =
        gh_api_partial(&path, Some("application/vnd.github.diff"), 256 * 1024).await?;
    diff_answer(
        success,
        &stdout,
        &stderr,
        repository,
        number,
        oversized,
        std::time::SystemTime::now(),
    )
}

fn diff_answer(
    success: bool,
    stdout: &[u8],
    stderr: &[u8],
    repository: &str,
    number: u32,
    oversized: bool,
    now: std::time::SystemTime,
) -> Result<Value, Error> {
    if !success {
        return Err(gh_error(false, stderr));
    }
    let diff = std::str::from_utf8(stdout)
        .map_err(|_| Error::Partial("GitHub returned a non-text pull request diff.".into()))?;
    let has_binary = diff
        .lines()
        .any(|line| line.starts_with("Binary files ") || line == "GIT binary patch");
    let (text, text_truncated) = bounded_text(diff, 48 * 1024);
    let truncated = text_truncated || oversized;
    let fetched_at = now
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    Ok(json!({
        "source": {"provider":"github", "mode":"live", "repository":repository, "url":format!("{}/pull/{number}",source(repository)["url"].as_str().unwrap_or_default()), "host":source(repository)["host"], "fetchedAt":fetched_at},
        "number":number, "diff":text, "truncated":truncated, "containsBinary":if oversized {Value::Null} else {json!(has_binary)}, "binaryObserved":has_binary,
        "coverage": if truncated { "partial-byte-limit" } else { "complete" }
    }))
}

/// Lists one bounded Actions run page. `page` is deliberately explicit; gh never auto-paginates.
pub async fn actions_runs(repository: &str, page: u32) -> Result<Value, Error> {
    let (owner, name) = checked_repository(repository)?;
    if !(1..=100).contains(&page) {
        return Err(Error::Other("Actions page must be between 1 and 100.".into()));
    }
    let path = format!("repos/{owner}/{name}/actions/runs?per_page=10&page={page}");
    let value = gh_api_json(&path, 256 * 1024).await?;
    actions_runs_answer(value, repository, page)
}

fn actions_runs_answer(value: Value, repository: &str, page: u32) -> Result<Value, Error> {
    let runs = value["workflow_runs"]
        .as_array()
        .ok_or_else(|| Error::Partial("GitHub omitted Actions run coverage.".into()))?;
    let total = value["total_count"]
        .as_u64()
        .ok_or_else(|| Error::Partial("GitHub omitted the Actions run count.".into()))?;
    if runs.len() > 10
        || runs.iter().any(|run| {
            run["id"].as_u64().is_none()
                || run["name"].as_str().is_none()
                || run["status"].as_str().is_none()
                || run["html_url"].as_str().is_none()
                || !run["run_attempt"]
                    .as_u64()
                    .is_some_and(|n| (1..=u32::MAX as u64).contains(&n))
                || !run["head_sha"].as_str().is_some_and(commit)
                || actions::phase(&run["status"], &run["conclusion"]).is_err()
        })
    {
        return Err(Error::Partial("GitHub returned malformed Actions runs.".into()));
    }
    let items: Vec<Value> = runs.iter().take(10).map(|run| json!({
        "id":run["id"], "name":run["name"], "workflow":run["workflow_id"], "event":run["event"],
        "status":run["status"], "phase":actions::phase(&run["status"],&run["conclusion"]).expect("validated phase"), "conclusion":run["conclusion"], "branch":run["head_branch"],
        "headSha":run["head_sha"], "attempt":run["run_attempt"], "createdAt":run["created_at"], "url":run["html_url"]
    })).collect();
    let has_more = u64::from(page) * 10 < total;
    Ok(
        json!({"source":source(repository), "runs":items, "total":total, "page":page, "hasMore":has_more, "nextPage":(has_more && page < 100).then_some(page + 1), "pageLimitReached":has_more && page == 100}),
    )
}

/// Reads one bounded page of jobs for an exact workflow run and attempt.
pub async fn actions_jobs(repository: &str, run_id: u64, attempt: u32, page: u32) -> Result<Value, Error> {
    let (owner, name) = checked_repository(repository)?;
    if run_id == 0 || attempt == 0 || !(1..=100).contains(&page) {
        return Err(Error::Other(
            "Run, attempt, and page must be positive; page is capped at 100.".into(),
        ));
    }
    let path =
        format!("repos/{owner}/{name}/actions/runs/{run_id}/attempts/{attempt}/jobs?per_page=10&page={page}");
    let value = gh_api_json(&path, 256 * 1024).await?;
    actions_jobs_answer(value, repository, run_id, attempt, page)
}

fn actions_jobs_answer(
    value: Value,
    repository: &str,
    run_id: u64,
    attempt: u32,
    page: u32,
) -> Result<Value, Error> {
    let jobs = value["jobs"]
        .as_array()
        .ok_or_else(|| Error::Partial("GitHub omitted Actions job coverage.".into()))?;
    let total = value["total_count"]
        .as_u64()
        .ok_or_else(|| Error::Partial("GitHub omitted the Actions job count.".into()))?;
    if jobs.len() > 10
        || jobs.iter().any(|job| {
            job["id"].as_u64().is_none()
                || job["run_id"].as_u64() != Some(run_id)
                || actions::phase(&job["status"], &job["conclusion"]).is_err()
                || job["name"].as_str().is_none()
                || job["status"].as_str().is_none()
                || job["html_url"].as_str().is_none()
                || !job["steps"].as_array().is_some_and(|steps| {
                    steps
                        .iter()
                        .all(|step| step["name"].is_string() && step["status"].is_string())
                })
        })
    {
        return Err(Error::Partial("GitHub returned malformed Actions jobs.".into()));
    }
    let items: Vec<Value> = jobs.iter().take(10).map(|job| json!({
        "id":job["id"], "name":job["name"], "status":job["status"], "phase":actions::phase(&job["status"],&job["conclusion"]).expect("validated phase"), "conclusion":job["conclusion"],
        "startedAt":job["started_at"], "completedAt":job["completed_at"], "url":job["html_url"],
        "steps":job["steps"].as_array().map(|steps| steps.iter().take(20).map(|step| json!({"name":step["name"],"status":step["status"],"conclusion":step["conclusion"]})).collect::<Vec<_>>()),
        "stepsTruncated":job["steps"].as_array().is_some_and(|steps| steps.len() > 20)
    })).collect();
    let has_more = u64::from(page) * 10 < total;
    Ok(
        json!({"source":source(repository), "runId":run_id, "attempt":attempt, "jobs":items, "total":total, "page":page, "hasMore":has_more, "nextPage":(has_more && page < 100).then_some(page + 1), "pageLimitReached":has_more && page == 100}),
    )
}

/// Fetches one job log using gh's authenticated CLI, with an output cap and secret redaction.
pub async fn actions_job_log(
    gate: &dyn crate::Gate,
    repository: &str,
    run_id: u64,
    job_id: u64,
    attempt: u32,
) -> Result<Value, Error> {
    checked_repository(repository)?;
    if run_id == 0 || job_id == 0 || attempt == 0 {
        return Err(Error::Other("Run, job, and attempt must be positive.".into()));
    }
    let path = format!("repos/{repository}/actions/jobs/{job_id}");
    let job = find_job_for_log(gate, &GhApi, repository, run_id, job_id, attempt).await?;
    let output = gated(gate, gh_api_partial(&format!("{path}/logs"), None, 256 * 1024)).await?;
    job_log_answer(
        repository,
        run_id,
        job_id,
        attempt,
        job["html_url"].clone(),
        output,
    )
}

fn job_log_answer(
    repository: &str,
    run_id: u64,
    job_id: u64,
    attempt: u32,
    url: Value,
    output: (bool, Vec<u8>, Vec<u8>, bool),
) -> Result<Value, Error> {
    let (success, stdout, stderr, oversized) = output;
    if !success {
        return Err(gh_error(false, &stderr));
    }
    let raw = String::from_utf8_lossy(&stdout);
    let safe = vults_core::redact::secrets(&raw);
    let (log, text_truncated) = bounded_text(&safe, 48 * 1024);
    let truncated = text_truncated || oversized;
    Ok(
        json!({"source":source(repository), "url":url, "runId":run_id, "jobId":job_id, "attempt":attempt, "log":log, "truncated":truncated, "coverage":if truncated {"partial-byte-limit"} else {"complete"}}),
    )
}

async fn find_job_for_log(
    gate: &dyn crate::Gate,
    api: &impl GithubApi,
    repository: &str,
    run_id: u64,
    job_id: u64,
    attempt: u32,
) -> Result<Value, Error> {
    for page in 1..=10 {
        let value = gated(gate,api.request(None,"GET",&format!("repos/{repository}/actions/runs/{run_id}/attempts/{attempt}/jobs?per_page=100&page={page}"),None)).await?;
        let jobs = value["jobs"]
            .as_array()
            .filter(|jobs| jobs.len() <= 100)
            .ok_or_else(|| Error::Partial("GitHub omitted the job page for this run attempt.".into()))?;
        let total = value["total_count"]
            .as_u64()
            .ok_or_else(|| Error::Partial("GitHub omitted the job count for this run attempt.".into()))?;
        if jobs.iter().any(|job| {
            !job["id"].is_u64() || job["run_id"].as_u64() != Some(run_id) || !job["html_url"].is_string()
        }) {
            return Err(Error::Partial(
                "GitHub returned malformed jobs for this run attempt.".into(),
            ));
        }
        if let Some(job) = jobs.iter().find(|job| job["id"].as_u64() == Some(job_id)) {
            return Ok(job.clone());
        }
        if jobs.len() < 100 || page * 100 >= total {
            return Err(Error::Other(
                "This GitHub job is not part of the requested run attempt.".into(),
            ));
        }
    }
    Err(Error::Partial(
        "The run-attempt job lookup reached its 1,000-item limit.".into(),
    ))
}

/// Reads the active branch rulesets/rules with an explicit REST page.
pub async fn branch_rules(repository: &str, branch: &str, page: u32) -> Result<Value, Error> {
    let (owner, name) = checked_repository(repository)?;
    if branch.is_empty()
        || branch.len() > 255
        || branch.chars().any(char::is_control)
        || branch.contains('*')
        || branch.contains('?')
        || !(1..=100).contains(&page)
    {
        return Err(Error::Other("The branch name is invalid.".into()));
    }
    let path = format!(
        "repos/{owner}/{name}/rules/branches/{}?per_page=100&page={page}",
        url_encode_path(branch)
    );
    let (success, response, stderr) = gh_api_included(&path, 256 * 1024).await?;
    if !success {
        return Err(gh_error(false, &stderr));
    }
    let (body, has_more) = response_body_and_next(&response)?;
    let rules: Value = serde_json::from_slice(body)
        .map_err(|_| Error::Partial("GitHub returned malformed branch rules.".into()))?;
    let items = rules
        .as_array()
        .ok_or_else(|| Error::Partial("GitHub omitted branch-rule coverage.".into()))?;
    Ok(
        json!({"source":source(repository),"branch":branch,"rules":items,"page":page,"hasMore":has_more,"nextPage":(has_more && page < 100).then_some(page+1),"pageLimitReached":has_more && page == 100,"coverage":"active-rules-for-branch"}),
    )
}

fn checked_target(repository: &str, number: u32) -> Result<(&str, &str), Error> {
    let (owner, name) = checked_repository(repository)?;
    if number == 0 || number > i32::MAX as u32 {
        return Err(Error::Other(
            "Issue or pull request number must be positive.".into(),
        ));
    }
    Ok((owner, name))
}

fn checked_repository(repository: &str) -> Result<(&str, &str), Error> {
    repository
        .split_once('/')
        .filter(|_| plain_repo(repository))
        .ok_or_else(|| Error::Other("Use a GitHub repository in owner/name form.".into()))
}

fn source(repository: &str) -> Value {
    let host = std::env::var("GH_HOST").unwrap_or_else(|_| "github.com".into());
    json!({"provider":"github", "mode":"live", "host":host, "repository":repository, "url":format!("https://{host}/{repository}"), "fetchedAt":unix_now()})
}

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
#[serde(tag = "operation", rename_all = "snake_case")]
pub enum IssueWrite {
    Create {
        repository: String,
        title: String,
        body: String,
    },
    Edit {
        repository: String,
        number: u32,
        title: Option<String>,
        body: Option<String>,
    },
    Comment {
        repository: String,
        number: u32,
        body: String,
    },
    EditComment {
        repository: String,
        number: u32,
        comment_id: u64,
        body: String,
    },
    AddLabels {
        repository: String,
        number: u32,
        labels: Vec<String>,
    },
    RemoveLabels {
        repository: String,
        number: u32,
        labels: Vec<String>,
    },
    SetAssignees {
        repository: String,
        number: u32,
        assignees: Vec<String>,
    },
    Close {
        repository: String,
        number: u32,
    },
    Reopen {
        repository: String,
        number: u32,
    },
}

impl IssueWrite {
    fn repository(&self) -> &str {
        match self {
            Self::Create { repository, .. }
            | Self::Edit { repository, .. }
            | Self::Comment { repository, .. }
            | Self::EditComment { repository, .. }
            | Self::AddLabels { repository, .. }
            | Self::RemoveLabels { repository, .. }
            | Self::SetAssignees { repository, .. }
            | Self::Close { repository, .. }
            | Self::Reopen { repository, .. } => repository,
        }
    }

    fn number(&self) -> Option<u32> {
        match self {
            Self::Create { .. } => None,
            Self::Edit { number, .. }
            | Self::Comment { number, .. }
            | Self::EditComment { number, .. }
            | Self::AddLabels { number, .. }
            | Self::RemoveLabels { number, .. }
            | Self::SetAssignees { number, .. }
            | Self::Close { number, .. }
            | Self::Reopen { number, .. } => Some(*number),
        }
    }

    pub fn validate(&self) -> Result<(), Error> {
        checked_repository(self.repository())?;
        if self
            .number()
            .is_some_and(|number| number == 0 || number > i32::MAX as u32)
        {
            return Err(Error::Other("Issue number must be positive.".into()));
        }
        let text = |value: &str, limit: usize, multiline: bool| {
            !value.trim().is_empty()
                && value.chars().count() <= limit
                && value
                    .chars()
                    .all(|c| !c.is_control() || (multiline && matches!(c, '\n' | '\r' | '\t')))
        };
        let list = |items: &[String], limit: usize, item_limit: usize, username: bool| {
            !items.is_empty()
                && items.len() <= limit
                && items.iter().collect::<std::collections::HashSet<_>>().len() == items.len()
                && items.iter().all(|item| {
                    !item.is_empty()
                        && item.len() <= item_limit
                        && (if username {
                            item.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
                        } else {
                            item.chars().all(|c| !c.is_control())
                        })
                })
        };
        match self {
            Self::Create { title, body, .. } => {
                if !text(title, 256, false) || body.len() > 10_000 || body.contains('\0') {
                    return Err(Error::Other("Issue title or body is invalid or too long.".into()));
                }
            }
            Self::Edit { title, body, .. } => {
                if title.is_none() && body.is_none()
                    || title.as_deref().is_some_and(|title| !text(title, 256, false))
                    || body
                        .as_deref()
                        .is_some_and(|body| body.len() > 10_000 || body.contains('\0'))
                {
                    return Err(Error::Other("Give a valid issue title or body to edit.".into()));
                }
            }
            Self::Comment { body, .. } | Self::EditComment { body, .. } => {
                if !text(body, 10_000, true) || matches!(self, Self::EditComment { comment_id: 0, .. }) {
                    return Err(Error::Other("Comment is empty, invalid or too long.".into()));
                }
            }
            Self::AddLabels { labels, .. } | Self::RemoveLabels { labels, .. } => {
                if !list(labels, 10, 50, false) {
                    return Err(Error::Other("Give one to ten distinct, valid labels.".into()));
                }
            }
            Self::SetAssignees { assignees, .. } => {
                if assignees.len() > 10
                    || assignees.iter().collect::<std::collections::HashSet<_>>().len() != assignees.len()
                    || assignees.iter().any(|name| {
                        name.is_empty()
                            || name.len() > 39
                            || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
                    })
                {
                    return Err(Error::Other("Give up to ten distinct GitHub usernames.".into()));
                }
            }
            Self::Close { .. } | Self::Reopen { .. } => {}
        }
        Ok(())
    }

    pub fn request_hash(&self) -> String {
        let bytes = serde_json::to_vec(self).expect("issue write is serializable");
        let digest = Sha256::digest(bytes);
        digest.iter().map(|byte| format!("{byte:02x}")).collect()
    }
}

struct Request<'a> {
    gate: &'a dyn crate::Gate,
    started: std::time::Instant,
    done: bool,
}

impl<'a> Request<'a> {
    fn begin(gate: &'a dyn crate::Gate) -> Result<Self, Error> {
        if !gate.allow("github") {
            return Err(Error::Other(
                "GitHub is off or its daily request budget is spent.".into(),
            ));
        }
        Ok(Self {
            gate,
            started: std::time::Instant::now(),
            done: false,
        })
    }

    fn finish(mut self, ok: bool) {
        self.gate.done("github", ok, self.started.elapsed());
        self.done = true;
    }
}

impl Drop for Request<'_> {
    fn drop(&mut self) {
        if !self.done {
            self.gate.done("github", false, self.started.elapsed());
        }
    }
}

async fn gated<T>(
    gate: &dyn crate::Gate,
    future: impl std::future::Future<Output = Result<T, Error>>,
) -> Result<T, Error> {
    let request = Request::begin(gate)?;
    let result = future.await;
    request.finish(result.is_ok());
    result
}

// Keep the HTTP contract observable so fixture tests exercise the actual write sequence.
trait GithubApi: Sync {
    fn request<'a>(
        &'a self,
        host: Option<&'a str>,
        method: &'a str,
        path: &'a str,
        body: Option<Value>,
    ) -> impl std::future::Future<Output = Result<Value, Error>> + Send + 'a;
}

struct GhApi;

impl GithubApi for GhApi {
    async fn request(
        &self,
        host: Option<&str>,
        method: &str,
        path: &str,
        body: Option<Value>,
    ) -> Result<Value, Error> {
        let mut command = Command::new("gh");
        command.args(["api", "--method", method, path]);
        if path.contains("/merge-async") {
            command.args(["-H", "X-GitHub-Api-Version: 2026-03-10"]);
        }
        if let Some(host) = host {
            command.args(["--hostname", host]);
        }
        let bytes = body
            .map(|value| serde_json::to_vec(&value))
            .transpose()
            .map_err(|error| Error::Other(error.to_string()))?;
        if bytes.is_some() {
            command.args(["--input", "-"]);
        }
        let (success, stdout, stderr, oversized) =
            run_bounded_partial_input(command, 512 * 1024, bytes.as_deref()).await?;
        if !success {
            return Err(gh_error(false, &stderr));
        }
        if oversized {
            return Err(Error::Partial(
                "GitHub returned more than the request limit.".into(),
            ));
        }
        if stdout.is_empty() {
            return Ok(Value::Null);
        }
        serde_json::from_slice(&stdout)
            .map_err(|_| Error::Partial("GitHub returned malformed request data.".into()))
    }
}

/// Captures the identity, repository permission and issue state that the user will review.
pub async fn prepare_issue_write(gate: &dyn crate::Gate, action: &IssueWrite) -> Result<Value, Error> {
    prepare_issue_write_using(gate, action, &GhApi).await
}

async fn prepare_issue_write_using(
    gate: &dyn crate::Gate,
    action: &IssueWrite,
    api: &impl GithubApi,
) -> Result<Value, Error> {
    action.validate()?;
    let inspected = inspect_issue_write(gate, action, api, None).await?;
    if duplicate_write(action, &inspected) {
        return Err(Error::Other(
            "This exact GitHub change is already present; inspect the issue before proposing it again."
                .into(),
        ));
    }
    validate_issue_targets(action, &inspected)?;
    Ok(inspected)
}

/// Rechecks identity, permission and target state before writing, then reads back the result.
pub async fn execute_issue_write(
    gate: &dyn crate::Gate,
    action: &IssueWrite,
    expected: &Value,
    expected_hash: &str,
) -> Result<Value, Error> {
    execute_issue_write_using(gate, action, expected, expected_hash, &GhApi).await
}

async fn execute_issue_write_using(
    gate: &dyn crate::Gate,
    action: &IssueWrite,
    expected: &Value,
    expected_hash: &str,
    api: &impl GithubApi,
) -> Result<Value, Error> {
    action.validate()?;
    if action.request_hash() != expected_hash {
        return Err(Error::Other(
            "The approved GitHub change no longer matches its request hash.".into(),
        ));
    }
    let host = expected["source"]["host"]
        .as_str()
        .filter(|host| github_host(&format!("https://{host}")).is_some())
        .ok_or_else(|| Error::Other("The approved GitHub host is missing. Review a new card.".into()))?;
    let before = inspect_issue_write(gate, action, api, Some(host)).await?;
    if before["binding"] != expected["binding"] {
        return Err(Error::Other(
            "The GitHub account, target or write permission changed after approval. Review it again.".into(),
        ));
    }
    if duplicate_write(action, &before) {
        return Err(Error::Other(
            "This GitHub change is already present; no duplicate write was sent.".into(),
        ));
    }
    validate_issue_targets(action, &before)?;
    let mutation = mutate_issue(gate, action, api, host).await;
    let after = inspect_issue_write(gate, action, api, Some(host)).await.map_err(|error| {
        Error::Other(format!("The write outcome is uncertain because verification failed ({error}). Inspect the GitHub target before proposing a retry."))
    })?;
    if before["binding"]["identity"] != after["binding"]["identity"]
        || before["binding"]["repositoryId"] != after["binding"]["repositoryId"]
        || (action.number().is_some() && before["issue"]["id"] != after["issue"]["id"])
    {
        return Err(Error::Other("The GitHub account or repository changed during the write. Its outcome is uncertain; inspect the target before retrying.".into()));
    }
    if !write_is_present(action, &after) {
        return Err(Error::Other("The complete GitHub change could not be verified. The target was reread; inspect it before proposing a retry.".into()));
    }
    let issue = if matches!(action, IssueWrite::Create { .. }) {
        after["related"]["matches"][0].clone()
    } else {
        let issue = &after["issue"];
        json!({"id":issue["id"],"number":issue["number"],"url":issue["url"],"title":issue["title"],"state":issue["state"],"labels":issue["labels"],"assignees":issue["assignees"]})
    };
    Ok(
        json!({"status":if mutation.is_ok() {"verified"} else {"verified_after_error"},"source":after["source"],"issue":issue}),
    )
}

struct WriteRepository {
    source: Value,
    binding: Value,
}

async fn write_repository(
    gate: &dyn crate::Gate,
    api: &impl GithubApi,
    repository_name: &str,
    host: Option<&str>,
) -> Result<WriteRepository, Error> {
    let (owner, name) = checked_repository(repository_name)?;
    let user = gated(gate, api.request(host, "GET", "user", None)).await?;
    let login = user["login"]
        .as_str()
        .ok_or_else(|| Error::Partial("GitHub omitted the active account.".into()))?;
    let host = user["html_url"]
        .as_str()
        .and_then(github_host)
        .filter(|value| host.is_none_or(|expected| expected == *value))
        .ok_or_else(|| Error::Partial("GitHub returned a different or invalid account host.".into()))?;
    if !user["id"].is_u64() {
        return Err(Error::Partial("GitHub omitted the account identity.".into()));
    }
    let repository = gated(
        gate,
        api.request(Some(host), "GET", &format!("repos/{owner}/{name}"), None),
    )
    .await?;
    if !repository["id"].is_u64()
        || !repository["html_url"]
            .as_str()
            .is_some_and(|url| url.eq_ignore_ascii_case(&format!("https://{host}/{}", repository_name)))
        || repository["permissions"]["push"].as_bool().is_none()
        || !repository["full_name"]
            .as_str()
            .is_some_and(|value| value.eq_ignore_ascii_case(repository_name))
    {
        return Err(Error::Partial(
            "GitHub did not return the exact repository and its write permissions.".into(),
        ));
    }
    if repository["permissions"]["push"] != true || repository["archived"] != false {
        return Err(Error::Auth(
            "The selected GitHub account cannot write to this repository, or it is archived.".into(),
        ));
    }
    let binding = json!({"identity":{"id":user["id"],"login":login,"host":host},"repositoryId":repository["id"],"repository":repository["full_name"],"url":repository["html_url"],"canWrite":true});
    Ok(WriteRepository {
        source: json!({"provider":"github","mode":"live","account":login,"host":host,"repository":repository["full_name"],"url":repository["html_url"],"fetchedAt":unix_now()}),
        binding,
    })
}

async fn inspect_issue_write(
    gate: &dyn crate::Gate,
    action: &IssueWrite,
    api: &impl GithubApi,
    host: Option<&str>,
) -> Result<Value, Error> {
    let (owner, name) = checked_repository(action.repository())?;
    let context = write_repository(gate, api, action.repository(), host).await?;
    let host = context.source["host"].as_str().expect("validated host");
    let mut binding = context.binding;
    let mut issue = Value::Null;
    let mut related = Value::Null;
    if let Some(number) = action.number() {
        let value = gated(
            gate,
            api.request(
                Some(host),
                "GET",
                &format!("repos/{owner}/{name}/issues/{number}"),
                None,
            ),
        )
        .await?;
        if value["number"].as_u64() != Some(u64::from(number))
            || !value["id"].is_u64()
            || !value["title"].is_string()
            || !matches!(value["state"].as_str(), Some("open" | "closed"))
            || !value["updated_at"].is_string()
            || !value["html_url"].as_str().is_some_and(|url| {
                url.eq_ignore_ascii_case(&format!("https://{host}/{}/issues/{number}", action.repository()))
            })
            || !(value["body"].is_null() || value["body"].is_string())
            || value["pull_request"].is_object()
        {
            return Err(Error::Partial(
                "GitHub did not return a complete issue (pull requests are not writable here).".into(),
            ));
        }
        let names = |key: &str, field: &str| -> Result<Vec<String>, Error> {
            let items = value[key]
                .as_array()
                .ok_or_else(|| Error::Partial(format!("GitHub omitted issue {key}.")))?;
            let mut names: Vec<String> = items
                .iter()
                .map(|item| {
                    item[field]
                        .as_str()
                        .map(str::to_owned)
                        .ok_or_else(|| Error::Partial(format!("GitHub returned malformed issue {key}.")))
                })
                .collect::<Result<_, _>>()?;
            names.sort_unstable_by_key(|name| name.to_ascii_lowercase());
            Ok(names)
        };
        issue = json!({"id":value["id"],"number":number,"url":value["html_url"],"title":value["title"],"body":value["body"].as_str().unwrap_or_default(),"state":value["state"],"updatedAt":value["updated_at"],"labels":names("labels","name")?,"assignees":names("assignees","login")?});
        if let IssueWrite::Comment { body, .. } = action {
            let comments = issue_collection(
                gate,
                api,
                host,
                &format!("repos/{owner}/{name}/issues/{number}/comments?per_page=100"),
            )
            .await?;
            if comments
                .iter()
                .any(|item| !item["id"].is_u64() || !item["body"].is_string())
            {
                return Err(Error::Partial(
                    "GitHub returned malformed comment coverage.".into(),
                ));
            }
            related = json!({"comments":comments.iter().map(|item| json!({"id":item["id"],"body":if item["body"]==*body {body.as_str()} else {""}})).collect::<Vec<_>>()});
        }
        if let IssueWrite::EditComment { comment_id, .. } = action {
            let comment = gated(
                gate,
                api.request(
                    Some(host),
                    "GET",
                    &format!("repos/{owner}/{name}/issues/comments/{comment_id}"),
                    None,
                ),
            )
            .await?;
            let expected_path = format!("/repos/{owner}/{name}/issues/{number}");
            if comment["id"].as_u64() != Some(*comment_id)
                || !comment["body"].is_string()
                || !comment["updated_at"].is_string()
                || !comment["issue_url"].as_str().is_some_and(|url| {
                    url.to_ascii_lowercase()
                        .ends_with(&expected_path.to_ascii_lowercase())
                })
            {
                return Err(Error::Partial(
                    "GitHub did not return the exact comment for this issue.".into(),
                ));
            }
            related["comment"] =
                json!({"id":comment["id"],"body":comment["body"],"updatedAt":comment["updated_at"]});
        }
        binding["issue"] = issue.clone();
        binding["related"] = related.clone();
    } else if let IssueWrite::Create { title, body, .. } = action {
        let items = issue_collection(
            gate,
            api,
            host,
            &format!("repos/{owner}/{name}/issues?state=all&per_page=100&sort=created&direction=asc"),
        )
        .await?;
        if items.iter().any(|item| {
            !item["id"].is_u64()
                || !item["number"].is_u64()
                || !item["title"].is_string()
                || !item["html_url"].is_string()
                || !(item["body"].is_null() || item["body"].is_string())
        }) {
            return Err(Error::Partial(
                "GitHub returned malformed duplicate-check coverage.".into(),
            ));
        }
        binding["existingIssueIds"] = json!(items.iter().map(|item| &item["id"]).collect::<Vec<_>>());
        related = json!({"complete":true,"matches":items.iter().filter(|item| item["pull_request"].is_null() && item["title"]==*title && item["body"].as_str().unwrap_or_default()==body).map(|item| json!({"id":item["id"],"number":item["number"],"url":item["html_url"]})).collect::<Vec<_>>()});
        binding["related"] = related.clone();
    }
    Ok(json!({"source":context.source,"binding":binding,"issue":issue,"related":related}))
}

async fn issue_collection(
    gate: &dyn crate::Gate,
    api: &impl GithubApi,
    host: &str,
    path: &str,
) -> Result<Vec<Value>, Error> {
    let mut kept = Vec::new();
    for page in 1..=10 {
        let value = gated(
            gate,
            api.request(Some(host), "GET", &format!("{path}&page={page}"), None),
        )
        .await?;
        let items = value
            .as_array()
            .filter(|items| items.len() <= 100)
            .ok_or_else(|| Error::Partial("GitHub omitted duplicate-check coverage.".into()))?;
        kept.extend(items.iter().cloned());
        if items.len() < 100 {
            return Ok(kept);
        }
    }
    Err(Error::Partial("The duplicate check reached its 1,000-item limit. Use GitHub to perform this change, then refresh Vults.".into()))
}

fn issue_write_requests(action: &IssueWrite) -> Vec<(&'static str, String, Option<Value>)> {
    let root = format!("repos/{}/issues", action.repository());
    let issue = format!("{root}/{}", action.number().unwrap_or_default());
    let (method, path, body) = match action {
        IssueWrite::Create { title, body, .. } => ("POST", root, json!({"title":title,"body":body})),
        IssueWrite::Edit { title, body, .. } => {
            let mut value = serde_json::Map::new();
            if let Some(title) = title {
                value.insert("title".into(), json!(title));
            }
            if let Some(body) = body {
                value.insert("body".into(), json!(body));
            }
            ("PATCH", issue, Value::Object(value))
        }
        IssueWrite::Comment { body, .. } => ("POST", format!("{issue}/comments"), json!({"body":body})),
        IssueWrite::EditComment { comment_id, body, .. } => (
            "PATCH",
            format!("{root}/comments/{comment_id}"),
            json!({"body":body}),
        ),
        IssueWrite::AddLabels { labels, .. } => ("POST", format!("{issue}/labels"), json!({"labels":labels})),
        IssueWrite::RemoveLabels { labels, .. } => {
            return labels
                .iter()
                .map(|label| {
                    (
                        "DELETE",
                        format!("{issue}/labels/{}", url_encode_path(label)),
                        None,
                    )
                })
                .collect();
        }
        IssueWrite::SetAssignees { assignees, .. } => ("PATCH", issue, json!({"assignees":assignees})),
        IssueWrite::Close { .. } => ("PATCH", issue, json!({"state":"closed"})),
        IssueWrite::Reopen { .. } => ("PATCH", issue, json!({"state":"open"})),
    };
    vec![(method, path, Some(body))]
}

async fn mutate_issue(
    gate: &dyn crate::Gate,
    action: &IssueWrite,
    api: &impl GithubApi,
    host: &str,
) -> Result<(), Error> {
    for (method, path, body) in issue_write_requests(action) {
        gated(gate, api.request(Some(host), method, &path, body)).await?;
    }
    Ok(())
}

fn duplicate_write(action: &IssueWrite, inspected: &Value) -> bool {
    let issue = &inspected["issue"];
    match action {
        IssueWrite::Create { .. } => inspected["related"]["matches"]
            .as_array()
            .is_some_and(|matches| !matches.is_empty()),
        IssueWrite::Edit { title, body, .. } => {
            title.as_ref().is_none_or(|value| issue["title"] == *value)
                && body.as_ref().is_none_or(|value| issue["body"] == *value)
        }
        IssueWrite::Comment { body, .. } => inspected["related"]["comments"]
            .as_array()
            .is_some_and(|comments| comments.iter().any(|comment| comment["body"] == *body)),
        IssueWrite::EditComment { body, .. } => inspected["related"]["comment"]["body"] == *body,
        IssueWrite::AddLabels { labels, .. } => labels.iter().all(|label| {
            issue["labels"].as_array().is_some_and(|current| {
                current.iter().any(|value| {
                    value
                        .as_str()
                        .is_some_and(|value| value.eq_ignore_ascii_case(label))
                })
            })
        }),
        IssueWrite::RemoveLabels { labels, .. } => labels.iter().all(|label| {
            issue["labels"].as_array().is_some_and(|current| {
                !current.iter().any(|value| {
                    value
                        .as_str()
                        .is_some_and(|value| value.eq_ignore_ascii_case(label))
                })
            })
        }),
        IssueWrite::SetAssignees { assignees, .. } => {
            let Some(current) = issue["assignees"].as_array() else {
                return false;
            };
            current.len() == assignees.len()
                && assignees.iter().all(|name| {
                    current.iter().any(|value| {
                        value
                            .as_str()
                            .is_some_and(|value| value.eq_ignore_ascii_case(name))
                    })
                })
        }
        IssueWrite::Close { .. } => issue["state"] == "closed",
        IssueWrite::Reopen { .. } => issue["state"] == "open",
    }
}

fn validate_issue_targets(action: &IssueWrite, inspected: &Value) -> Result<(), Error> {
    if let IssueWrite::RemoveLabels { labels, .. } = action {
        for label in labels {
            if !inspected["issue"]["labels"].as_array().is_some_and(|current| {
                current.iter().any(|value| {
                    value
                        .as_str()
                        .is_some_and(|value| value.eq_ignore_ascii_case(label))
                })
            }) {
                return Err(Error::Other("One or more labels are no longer on this issue. Review the target before removing labels.".into()));
            }
        }
    }
    Ok(())
}

fn write_is_present(action: &IssueWrite, inspected: &Value) -> bool {
    match action {
        IssueWrite::Create { .. } => inspected["related"]["matches"]
            .as_array()
            .is_some_and(|matches| matches.len() == 1),
        IssueWrite::Comment { body, .. } => inspected["related"]["comments"]
            .as_array()
            .is_some_and(|comments| comments.iter().filter(|comment| comment["body"] == *body).count() == 1),
        _ => duplicate_write(action, inspected),
    }
}

fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn url_encode_path(branch: &str) -> String {
    branch
        .bytes()
        .map(|byte| {
            if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
                (byte as char).to_string()
            } else {
                format!("%{byte:02X}")
            }
        })
        .collect()
}

async fn gh_api_json(path: &str, limit: usize) -> Result<Value, Error> {
    let (success, stdout, stderr) = gh_api(path, None, limit).await?;
    if !success {
        return Err(gh_error(false, &stderr));
    }
    serde_json::from_slice(&stdout).map_err(|_| Error::Partial("GitHub returned malformed JSON.".into()))
}

async fn gh_api_included(path: &str, limit: usize) -> Result<(bool, Vec<u8>, Vec<u8>), Error> {
    let mut command = Command::new("gh");
    command.args(["api", "-i", path]);
    run_bounded(command, limit).await
}

fn response_body_and_next(response: &[u8]) -> Result<(&[u8], bool), Error> {
    let split = response
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|index| (index, 4))
        .or_else(|| {
            response
                .windows(2)
                .position(|window| window == b"\n\n")
                .map(|index| (index, 2))
        })
        .ok_or_else(|| Error::Partial("GitHub omitted the response headers.".into()))?;
    let (head, width) = split;
    let headers = String::from_utf8_lossy(&response[..head]).to_ascii_lowercase();
    let has_more = headers
        .lines()
        .any(|line| line.starts_with("link:") && line.contains("rel=\"next\""));
    Ok((&response[head + width..], has_more))
}

async fn gh_api(path: &str, accept: Option<&str>, limit: usize) -> Result<(bool, Vec<u8>, Vec<u8>), Error> {
    let mut command = Command::new("gh");
    command.args(["api", path]);
    if let Some(accept) = accept {
        command.args(["-H", &format!("Accept: {accept}")]);
    }
    run_bounded(command, limit).await
}

async fn gh_api_partial(
    path: &str,
    accept: Option<&str>,
    limit: usize,
) -> Result<(bool, Vec<u8>, Vec<u8>, bool), Error> {
    let mut command = Command::new("gh");
    command.args(["api", path]);
    if let Some(accept) = accept {
        command.args(["-H", &format!("Accept: {accept}")]);
    }
    run_bounded_partial(command, limit).await
}

async fn run_bounded(command: Command, limit: usize) -> Result<(bool, Vec<u8>, Vec<u8>), Error> {
    let (success, stdout, stderr, oversized) = run_bounded_partial(command, limit).await?;
    if oversized {
        return Err(Error::Other("GitHub returned more than the read limit.".into()));
    }
    Ok((success, stdout, stderr))
}

async fn run_bounded_partial(
    command: Command,
    limit: usize,
) -> Result<(bool, Vec<u8>, Vec<u8>, bool), Error> {
    run_bounded_partial_input(command, limit, None).await
}

async fn run_bounded_partial_input(
    mut command: Command,
    limit: usize,
    input: Option<&[u8]>,
) -> Result<(bool, Vec<u8>, Vec<u8>, bool), Error> {
    let output = tokio::time::timeout(Duration::from_secs(30), async {
        command
            .kill_on_drop(true)
            .stdin(if input.is_some() {
                std::process::Stdio::piped()
            } else {
                std::process::Stdio::null()
            })
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());
        let mut child = command.spawn().map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => Error::Unavailable("The GitHub CLI (gh) isn't installed".into()),
            _ => Error::Other(format!("can't run gh: {e}")),
        })?;
        let mut stdout = child
            .stdout
            .take()
            .ok_or_else(|| Error::Other("GitHub output was unavailable.".into()))?;
        let mut stderr = child
            .stderr
            .take()
            .ok_or_else(|| Error::Other("GitHub error output was unavailable.".into()))?;
        if let Some(input) = input {
            let mut stdin = child
                .stdin
                .take()
                .ok_or_else(|| Error::Other("GitHub input was unavailable.".into()))?;
            stdin
                .write_all(input)
                .await
                .map_err(|e| Error::Other(format!("can't send the GitHub request: {e}")))?;
        }
        let out_read = read_limited(&mut stdout, limit);
        let err_read = read_limited(&mut stderr, 16 * 1024);
        tokio::pin!(out_read, err_read);
        let (out, err) = tokio::select! {
            result = &mut out_read => {
                let out = result.map_err(|e| Error::Other(format!("can't read gh output: {e}")))?;
                if out.1 { let _ = child.start_kill(); }
                let err = err_read.await.map_err(|e| Error::Other(format!("can't read gh error output: {e}")))?;
                (out, err)
            }
            result = &mut err_read => {
                let err = result.map_err(|e| Error::Other(format!("can't read gh error output: {e}")))?;
                if err.1 { let _ = child.start_kill(); }
                let out = out_read.await.map_err(|e| Error::Other(format!("can't read gh output: {e}")))?;
                if out.1 { let _ = child.start_kill(); }
                (out, err)
            }
        };
        let status = child
            .wait()
            .await
            .map_err(|e| Error::Other(format!("can't wait for gh: {e}")))?;
        Ok::<_, Error>((status.success() || out.1, out.0, err.0, out.1 || err.1))
    })
    .await
    .map_err(|_| Error::Other("GitHub took too long to answer.".into()))??;
    Ok(output)
}

async fn graphql(
    query: &str,
    variables: &[(&str, &str, bool)],
    max_output: usize,
) -> Result<(bool, Vec<u8>, Vec<u8>), Error> {
    let mut command = Command::new("gh");
    command
        .args(["api", "graphql", "-f"])
        .arg(format!("query={query}"));
    for (name, value, typed) in variables {
        command
            .arg(if *typed { "-F" } else { "-f" })
            .arg(format!("{name}={value}"));
    }
    let output = run_bounded_partial(command, max_output).await?;
    if output.3 {
        return Err(Error::Other("GitHub returned more than the read limit.".into()));
    }
    Ok((output.0, output.1, output.2))
}

fn repository_search_answer(
    success: bool,
    stdout: &[u8],
    stderr: &[u8],
    now: std::time::SystemTime,
) -> Result<Value, Error> {
    if !success {
        return Err(gh_error(false, stderr));
    }
    let data: Value = serde_json::from_slice(stdout).map_err(|_| gh_error(success, stderr))?;
    if data["errors"].as_array().is_some_and(|errors| !errors.is_empty()) {
        return Err(Error::Partial(
            "GitHub returned an incomplete repository search.".into(),
        ));
    }
    let search = &data["data"]["search"];
    let viewer = &data["data"]["viewer"];
    let nodes = search["nodes"]
        .as_array()
        .ok_or_else(|| Error::Partial("GitHub omitted repository search coverage.".into()))?;
    let login = viewer["login"]
        .as_str()
        .ok_or_else(|| Error::Partial("GitHub omitted the account used for repository search.".into()))?;
    let host = viewer["url"]
        .as_str()
        .and_then(github_host)
        .ok_or_else(|| Error::Partial("GitHub omitted the account host for repository search.".into()))?;
    if search["pageInfo"]["hasNextPage"].is_null() {
        return Err(Error::Partial(
            "GitHub omitted repository search coverage.".into(),
        ));
    }
    let has_more = search["pageInfo"]["hasNextPage"] == true;
    let next_cursor = if has_more {
        Some(
            search["pageInfo"]["endCursor"]
                .as_str()
                .filter(|cursor| !cursor.is_empty() && cursor.len() <= 512)
                .ok_or_else(|| Error::Partial("GitHub omitted the next search cursor.".into()))?,
        )
    } else {
        None
    };
    if !search["repositoryCount"].is_u64()
        || nodes.iter().any(|repo| {
            !repo["nameWithOwner"].is_string()
                || !repo["url"].is_string()
                || !repo["isPrivate"].is_boolean()
                || !repo["isArchived"].is_boolean()
                || (!repo["defaultBranchRef"].is_null()
                    && repo["defaultBranchRef"]["name"].as_str().is_none())
        })
    {
        return Err(Error::Partial(
            "GitHub returned malformed repository search results.".into(),
        ));
    }
    let fetched_at = now
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let repositories: Vec<Value> = nodes
        .iter()
        .take(10)
        .map(|repo| {
            let (description, truncated) =
                bounded_text(repo["description"].as_str().unwrap_or_default(), 512);
            json!({
                "repository": repo["nameWithOwner"], "url": repo["url"],
                "description": description, "descriptionTruncated": truncated,
                "private": repo["isPrivate"], "archived": repo["isArchived"],
                "pushedAt": repo["pushedAt"], "defaultBranch": repo["defaultBranchRef"]["name"]
            })
        })
        .collect();
    Ok(json!({
        "source": { "provider": "github", "mode":"live", "account": login, "host": host, "fetchedAt": fetched_at },
        "repositories": repositories,
        "total": search["repositoryCount"],
        "hasMore": has_more,
        "nextCursor": next_cursor
    }))
}

async fn read_limited(
    reader: &mut (impl tokio::io::AsyncRead + Unpin),
    limit: usize,
) -> std::io::Result<(Vec<u8>, bool)> {
    let mut kept = Vec::with_capacity(limit.min(64 * 1024));
    let mut buffer = [0; 8192];
    let mut overflow = false;
    loop {
        let read = reader.read(&mut buffer).await?;
        if read == 0 {
            return Ok((kept, overflow));
        }
        let room = limit.saturating_sub(kept.len());
        let keep = room.min(read);
        kept.extend_from_slice(&buffer[..keep]);
        overflow |= keep < read;
        if overflow {
            return Ok((kept, true));
        }
    }
}

fn work_item_answer(
    success: bool,
    stdout: &[u8],
    stderr: &[u8],
    repository: &str,
    now: std::time::SystemTime,
) -> Result<Value, Error> {
    if !success {
        return Err(gh_error(false, stderr));
    }
    let data: Value = serde_json::from_slice(stdout).map_err(|_| gh_error(success, stderr))?;
    if data["errors"].as_array().is_some_and(|errors| !errors.is_empty()) {
        return Err(Error::Partial(
            "GitHub returned a partial work item; check repository access and try again.".into(),
        ));
    }
    let repo = &data["data"]["repository"];
    let item = &repo["issueOrPullRequest"];
    if repo.is_null() || item.is_null() {
        return Err(Error::Other(
            "GitHub could not find that issue or pull request, or your account cannot read it.".into(),
        ));
    }
    let discussion = &item["comments"];
    if !discussion["nodes"]
        .as_array()
        .is_some_and(|nodes| nodes.iter().all(|node| node["body"].is_string()))
        || !discussion["totalCount"].is_u64()
        || discussion["pageInfo"]["hasNextPage"].as_bool().is_none()
        || item["title"].as_str().is_none()
        || item["url"].as_str().is_none()
        || item["state"].as_str().is_none()
        || repo["nameWithOwner"].as_str().is_none()
        || repo["url"].as_str().is_none()
    {
        return Err(Error::Partial(
            "GitHub omitted discussion coverage; the result is incomplete.".into(),
        ));
    }
    let comments_more = discussion["pageInfo"]["hasNextPage"] == true;
    let comments_cursor = if comments_more {
        Some(
            discussion["pageInfo"]["endCursor"]
                .as_str()
                .filter(|cursor| !cursor.is_empty() && cursor.len() <= 512)
                .ok_or_else(|| Error::Partial("GitHub omitted the next discussion cursor.".into()))?,
        )
    } else {
        None
    };
    let kind = match item["__typename"].as_str() {
        Some("Issue") => "issue",
        Some("PullRequest") => "pullRequest",
        _ => return Err(Error::Partial("GitHub returned an unsupported work item.".into())),
    };
    let (body, body_truncated) = bounded_text(item["body"].as_str().unwrap_or_default(), 3 * 1024);
    let comments: Vec<Value> = discussion["nodes"]
        .as_array()
        .into_iter()
        .flatten()
        .take(5)
        .map(|comment| {
            let (body, truncated) = bounded_text(comment["body"].as_str().unwrap_or_default(), 400);
            json!({ "id":comment["url"].as_str().and_then(|url|url.rsplit_once("#issuecomment-")).and_then(|(_,id)|id.parse::<u64>().ok()), "body": body, "truncated": truncated, "createdAt": comment["createdAt"], "url": comment["url"] })
        })
        .collect();
    let fetched_at = now
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let mut result = json!({
        "source": { "provider": "github", "mode":"live", "repository": repo["nameWithOwner"], "url": repo["url"], "fetchedAt": fetched_at },
        "requestedRepository": repository,
        "kind": kind,
        "number": item["number"], "title": item["title"], "body": body,
        "bodyTruncated": body_truncated, "url": item["url"], "state": item["state"],
        "createdAt": item["createdAt"], "updatedAt": item["updatedAt"],
        "discussion": { "comments": comments, "total": discussion["totalCount"],
            "hasMore": comments_more, "nextCursor": comments_cursor },
    });
    if kind == "pullRequest" {
        let files = item["files"]["nodes"]
            .as_array()
            .ok_or_else(|| Error::Partial("GitHub omitted pull request file coverage.".into()))?;
        let reviews = item["latestReviews"]["nodes"]
            .as_array()
            .ok_or_else(|| Error::Partial("GitHub omitted pull request review coverage.".into()))?;
        if item["files"]["pageInfo"]["hasNextPage"].is_null()
            || item["latestReviews"]["pageInfo"]["hasNextPage"].is_null()
            || item["reviewThreads"]["nodes"].as_array().is_none()
            || item["reviewThreads"]["pageInfo"]["hasNextPage"].is_null()
        {
            return Err(Error::Partial(
                "GitHub returned incomplete pull request coverage.".into(),
            ));
        }
        if !item["files"]["totalCount"].is_u64()
            || files.iter().any(|file| {
                file["path"].as_str().is_none() || !file["additions"].is_u64() || !file["deletions"].is_u64()
            })
            || !item["latestReviews"]["totalCount"].is_u64()
            || reviews.iter().any(|review| review["state"].as_str().is_none())
            || !item["reviewThreads"]["totalCount"].is_u64()
        {
            return Err(Error::Partial(
                "GitHub returned malformed pull request files, reviews, or threads.".into(),
            ));
        }
        let files_more = item["files"]["pageInfo"]["hasNextPage"] == true;
        let files_cursor = if files_more {
            Some(
                item["files"]["pageInfo"]["endCursor"]
                    .as_str()
                    .filter(|cursor| !cursor.is_empty() && cursor.len() <= 512)
                    .ok_or_else(|| Error::Partial("GitHub omitted the next file cursor.".into()))?,
            )
        } else {
            None
        };
        let file_items: Vec<Value> = files.iter().take(10).map(|file| {
            let (path, truncated) = bounded_text(file["path"].as_str().unwrap_or_default(), 200);
            json!({ "path": path, "pathTruncated": truncated, "additions": file["additions"], "deletions": file["deletions"] })
        }).collect();
        let review_items: Vec<Value> = reviews
            .iter()
            .take(5)
            .map(|review| {
                let (body, truncated) = bounded_text(review["body"].as_str().unwrap_or_default(), 400);
                json!({ "author": review["author"]["login"], "state": review["state"], "body": body,
                "bodyTruncated": truncated, "submittedAt": review["submittedAt"] })
            })
            .collect();
        let reviews_more = item["latestReviews"]["pageInfo"]["hasNextPage"] == true;
        let reviews_cursor = next_cursor(&item["latestReviews"]["pageInfo"], reviews_more, "review")?;
        let threads = item["reviewThreads"]["nodes"]
            .as_array()
            .expect("validated above");
        if threads.iter().any(|thread| {
            thread["path"].as_str().is_none()
                || thread["comments"]["nodes"].as_array().is_none()
                || thread["comments"]["pageInfo"]["hasNextPage"].as_bool().is_none()
                || !thread["comments"]["totalCount"].is_u64()
        }) {
            return Err(Error::Partial(
                "GitHub returned malformed pull request review threads.".into(),
            ));
        }
        let threads_more = item["reviewThreads"]["pageInfo"]["hasNextPage"] == true;
        let threads_cursor = next_cursor(&item["reviewThreads"]["pageInfo"], threads_more, "review thread")?;
        let thread_items: Vec<Value> = threads.iter().take(5).map(|thread| {
            let comments = thread["comments"]["nodes"].as_array().cloned().unwrap_or_default();
            let items: Vec<Value> = comments.iter().take(3).map(|comment| {
                let (body, truncated) = bounded_text(comment["body"].as_str().unwrap_or_default(), 400);
                json!({"author":comment["author"]["login"], "body":body, "truncated":truncated, "createdAt":comment["createdAt"], "url":comment["url"]})
            }).collect();
            json!({"path":thread["path"], "line":thread["line"], "resolved":thread["isResolved"], "outdated":thread["isOutdated"], "comments":{"items":items,"total":thread["comments"]["totalCount"],"hasMore":thread["comments"]["pageInfo"]["hasNextPage"] == true}})
        }).collect();
        if !item["commits"]["nodes"]
            .as_array()
            .is_some_and(|commits| commits.len() == 1 && commits[0]["commit"]["oid"].is_string())
        {
            return Err(Error::Partial(
                "GitHub omitted the pull request head commit for check coverage.".into(),
            ));
        }
        let rollup = &item["commits"]["nodes"][0]["commit"]["statusCheckRollup"];
        if !rollup.is_null()
            && (rollup["contexts"]["nodes"].as_array().is_none()
                || rollup["contexts"]["pageInfo"]["hasNextPage"].is_null())
        {
            return Err(Error::Partial(
                "GitHub omitted pull request check coverage.".into(),
            ));
        }
        if !rollup.is_null() {
            let checks = rollup["contexts"]["nodes"].as_array().expect("validated above");
            if !rollup["contexts"]["totalCount"].is_u64() || runs(rollup).len() != checks.len() {
                return Err(Error::Partial(
                    "GitHub returned malformed pull request check coverage.".into(),
                ));
            }
        }
        let check_items = runs(rollup)
            .into_iter()
            .take(10)
            .map(|check| {
                let (name, truncated) = bounded_text(check["name"].as_str().unwrap_or_default(), 120);
                json!({ "name": name, "nameTruncated": truncated, "state": check["state"] })
            })
            .collect::<Vec<_>>();
        let check_more = rollup["contexts"]["pageInfo"]["hasNextPage"] == true;
        let check_cursor = if rollup.is_null() {
            None
        } else {
            next_cursor(&rollup["contexts"]["pageInfo"], check_more, "check")?
        };
        result["pullRequest"] = json!({
            "base": item["baseRefName"], "head": item["headRefName"], "headSha": item["headRefOid"],
            "additions": item["additions"], "deletions": item["deletions"], "changedFiles": item["changedFiles"],
            "reviewDecision": item["reviewDecision"],
            "mergeStateStatus": item["mergeStateStatus"], "mergeable": item["mergeable"], "draft": item["isDraft"],
            "files": { "items": file_items, "total": item["files"]["totalCount"], "hasMore": files_more, "nextCursor": files_cursor },
            "reviews": { "items": review_items, "total": item["latestReviews"]["totalCount"], "hasMore": reviews_more, "nextCursor": reviews_cursor },
            "reviewThreads": { "items": thread_items, "total": item["reviewThreads"]["totalCount"], "hasMore": threads_more, "nextCursor": threads_cursor },
            "checks": { "state": rollup["state"], "items": check_items, "hasMore": check_more, "nextCursor": check_cursor },
        });
        result["pullRequest"]["checks"]["availability"] = json!(if rollup.is_null() {
            "not-provided"
        } else {
            "available"
        });
    }
    Ok(result)
}

fn bounded_text(text: &str, max: usize) -> (String, bool) {
    if text.len() <= max {
        return (text.to_owned(), false);
    }
    let mut end = max;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    (text[..end].to_owned(), true)
}

fn next_cursor(page_info: &Value, has_more: bool, label: &str) -> Result<Option<String>, Error> {
    if !has_more {
        return Ok(None);
    }
    page_info["endCursor"]
        .as_str()
        .filter(|cursor| !cursor.is_empty() && cursor.len() <= 512)
        .map(str::to_owned)
        .map(Some)
        .ok_or_else(|| Error::Partial(format!("GitHub omitted the next {label} cursor.")))
}

/// What `gh api graphql` printed, as a complete snapshot or the reason there is none.
fn answer(success: bool, stdout: &[u8], stderr: &[u8]) -> Result<Snapshot, Error> {
    if let Ok(data) = serde_json::from_slice::<Value>(stdout) {
        let complete = data["data"]["viewer"]["pullRequests"]["nodes"].is_array()
            && data["data"]["viewer"]["repositories"]["nodes"].is_array()
            && data["data"]["search"]["nodes"].is_array();
        if complete {
            if data["errors"].as_array().is_some_and(|errors| !errors.is_empty()) {
                return Err(Error::Partial(
                    "GitHub returned partial data. Some repositories or fields may be inaccessible; the previous complete snapshot is kept.".into(),
                ));
            }
            return Ok(snapshot(&data));
        }
    }
    Err(gh_error(success, stderr))
}

/// Why `gh` gave no usable answer.
fn gh_error(success: bool, stderr: &[u8]) -> Error {
    let said = String::from_utf8_lossy(stderr);
    let lower = said.to_lowercase();
    if !success {
        // "HTTP 401", never a bare 401: a pull request's number can be one.
        return if lower.contains("rate limit") {
            Error::RateLimited {
                retry_after: Duration::from_secs(15 * 60),
            }
        } else if lower.contains("gh auth login")
            || lower.contains("http 401")
            || lower.contains("not logged")
        {
            Error::Auth("gh isn't logged in: run `gh auth login` in a terminal".into())
        } else {
            Error::Other(first_line(&said))
        };
    }
    Error::Other("unexpected answer from GitHub".into())
}

/// The user's contribution calendar: the last year, by day, as GitHub's profile draws it.
const CALENDAR: &str = r#"query {
  viewer { contributionsCollection { contributionCalendar {
    totalContributions
    weeks { contributionDays { date contributionCount contributionLevel } }
  } } }
}"#;

/// The user's GitHub contribution calendar (Settings → Activity, *GitHub*).
#[derive(serde::Serialize, Clone, Debug, PartialEq, Eq)]
pub struct Calendar {
    pub total: u32,
    pub days: Vec<CalendarDay>,
}

#[derive(serde::Serialize, Clone, Debug, PartialEq, Eq)]
pub struct CalendarDay {
    /// `2026-10-09`.
    pub day: String,
    pub count: u32,
    /// GitHub's own level: 0 for none, then its four quartiles.
    pub level: u8,
}

/// Asks `gh` for the calendar: one query (1 point of the hourly budget), on the user's login.
pub async fn calendar() -> Result<Calendar, Error> {
    let out = Command::new("gh")
        .args(["api", "graphql", "-f"])
        .arg(format!("query={CALENDAR}"))
        .kill_on_drop(true)
        .output();
    let out = tokio::time::timeout(Duration::from_secs(30), out)
        .await
        .map_err(|_| Error::Other("GitHub took too long to answer".into()))?
        .map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => Error::Unavailable("The GitHub CLI (gh) isn't installed".into()),
            _ => Error::Other(format!("can't run gh: {e}")),
        })?;
    calendar_answer(out.status.success(), &out.stdout, &out.stderr)
}

/// What `gh` said first, as it said it, without its leading `X ` mark.
fn first_line(said: &str) -> String {
    let line = said
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .unwrap_or("gh failed");
    line.strip_prefix("X ").unwrap_or(line).to_string()
}

/// A commit's id as GitHub gives it: hex, never a flag.
fn commit(head: &str) -> bool {
    (7..=64).contains(&head.len()) && head.chars().all(|c| c.is_ascii_hexdigit())
}

/// `pr:owner/repo#12` → (`owner/repo`, `12`): only a plain repository and a number reach `gh`.
#[cfg(test)]
fn pull_request(item: &str) -> Option<(&str, &str)> {
    let (repo, number) = item.strip_prefix("pr:")?.rsplit_once('#')?;
    let digits = !number.is_empty() && number.len() <= 10 && number.chars().all(|c| c.is_ascii_digit());
    // GitHub's names never start with a dash: none can pass for a flag.
    (plain_repo(repo) && !repo.starts_with('-') && digits && !number.starts_with('0'))
        .then_some((repo, number))
}

/// `owner/name`, as GitHub spells them.
fn plain_repo(repo: &str) -> bool {
    let ok = |p: &str| {
        !p.is_empty()
            && p.chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
    };
    matches!(repo.split_once('/'), Some((o, n)) if ok(o) && ok(n) && !n.contains('/') && ![".", ".."].contains(&o) && ![".", ".."].contains(&n))
}

fn calendar_answer(success: bool, stdout: &[u8], stderr: &[u8]) -> Result<Calendar, Error> {
    let data: Value = serde_json::from_slice(stdout).map_err(|_| gh_error(success, stderr))?;
    let cal = &data["data"]["viewer"]["contributionsCollection"]["contributionCalendar"];
    let weeks = cal["weeks"].as_array().ok_or_else(|| gh_error(success, stderr))?;
    let days = weeks
        .iter()
        .filter_map(|w| w["contributionDays"].as_array())
        .flatten()
        .filter_map(|d| {
            Some(CalendarDay {
                day: d["date"].as_str()?.to_string(),
                count: u32::try_from(d["contributionCount"].as_u64()?).ok()?,
                level: match d["contributionLevel"].as_str()? {
                    "FIRST_QUARTILE" => 1,
                    "SECOND_QUARTILE" => 2,
                    "THIRD_QUARTILE" => 3,
                    "FOURTH_QUARTILE" => 4,
                    _ => 0,
                },
            })
        })
        .collect();
    Ok(Calendar {
        total: cal["totalContributions"]
            .as_u64()
            .and_then(|t| u32::try_from(t).ok())
            .unwrap_or(0),
        days,
    })
}

/// The GraphQL answer as items keyed by what they are.
fn snapshot(data: &Value) -> Snapshot {
    let mut s = Snapshot::new();
    let d = &data["data"];
    if let (Some(login), Some(host)) = (
        d["viewer"]["login"].as_str(),
        d["viewer"]["url"].as_str().and_then(github_host),
    ) {
        s.insert("meta:account".into(), json!({"login": login, "host": host}));
    }
    // `rank`: the order GitHub gave (latest update, latest push), which the card keeps.
    for (rank, pr) in d["viewer"]["pullRequests"]["nodes"]
        .as_array()
        .into_iter()
        .flatten()
        .enumerate()
    {
        // A node GitHub could not resolve (a partial answer) is null: skip it.
        let Some(repo) = pr["repository"]["nameWithOwner"].as_str() else {
            continue;
        };
        let commit = &pr["commits"]["nodes"][0]["commit"];
        // Who answered, then who is still asked: one line each, an answer first.
        let mut reviewers: Vec<Value> = nodes(&pr["latestReviews"])
            .filter_map(|r| Some(json!({ "login": r["author"]["login"].as_str()?, "state": r["state"] })))
            .collect();
        for asked in nodes(&pr["reviewRequests"]) {
            let who = &asked["requestedReviewer"];
            // Users only: a team's fields need `read:org`, and a token without it fails the query.
            if let Some(login) = who["login"].as_str()
                && !reviewers.iter().any(|r| r["login"] == login)
            {
                reviewers.push(json!({ "login": login, "state": "WAITING" }));
            }
        }
        s.insert(
            format!("pr:{repo}#{}", pr["number"]),
            json!({
                "title": pr["title"], "url": pr["url"], "oid": commit["oid"],
                "ci": commit["statusCheckRollup"]["state"], "review": pr["reviewDecision"],
                "rank": rank, "branch": pr["headRefName"], "added": pr["additions"],
                "removed": pr["deletions"], "files": pr["changedFiles"], "updated": pr["updatedAt"],
                "reviewers": reviewers, "runs": runs(&commit["statusCheckRollup"]),
            }),
        );
    }
    for (rank, pr) in d["search"]["nodes"].as_array().into_iter().flatten().enumerate() {
        let Some(repo) = pr["repository"]["nameWithOwner"].as_str() else {
            continue;
        };
        s.insert(
            format!("review:{repo}#{}", pr["number"]),
            json!({
                "title": pr["title"], "url": pr["url"], "rank": rank, "author": pr["author"]["login"],
                "branch": pr["headRefName"], "added": pr["additions"], "removed": pr["deletions"],
                "files": pr["changedFiles"], "updated": pr["updatedAt"],
            }),
        );
    }
    for (rank, r) in d["viewer"]["repositories"]["nodes"]
        .as_array()
        .into_iter()
        .flatten()
        .enumerate()
    {
        if r["isArchived"] == true {
            continue;
        }
        let repo = r["nameWithOwner"].as_str().unwrap_or_default();
        if let Some(rel) = r["releases"]["nodes"][0].as_object() {
            s.insert(
                format!("release:{repo}"),
                json!({
                    "tag": rel["tagName"], "title": rel["name"], "url": rel["url"],
                    // A draft has no publish date yet.
                    "updated": if rel["publishedAt"].is_null() { &rel["createdAt"] } else { &rel["publishedAt"] },
                    "pre": rel["isPrerelease"], "draft": rel["isDraft"], "rank": rank,
                }),
            );
        }
        let commit = &r["defaultBranchRef"]["target"];
        if commit["oid"].is_null() {
            continue;
        }
        s.insert(
            format!("branch:{repo}"),
            json!({
                "branch": r["defaultBranchRef"]["name"],
                "oid": commit["oid"],
                "headline": commit["messageHeadline"],
                "ci": commit["statusCheckRollup"]["state"],
                "url": format!("{}/commit/{}", r["url"].as_str().unwrap_or_default(), commit["oid"].as_str().unwrap_or_default()),
                "rank": rank, "updated": commit["committedDate"], "runs": runs(&commit["statusCheckRollup"]),
            }),
        );
    }
    s
}

fn github_host(profile_url: &str) -> Option<&str> {
    let authority = profile_url.strip_prefix("https://")?.split('/').next()?;
    (!authority.is_empty()
        && authority
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b':')))
    .then_some(authority)
}

/// A connection's nodes, none when GitHub left it out.
fn nodes(connection: &Value) -> impl Iterator<Item = &Value> {
    connection["nodes"].as_array().into_iter().flatten()
}

/// A commit's checks as `{name, state}`, `state` already one of ours: Actions' runs and the older
/// commit statuses say it differently.
fn runs(rollup: &Value) -> Vec<Value> {
    nodes(&rollup["contexts"])
        .filter_map(|c| {
            let (name, state) = match c["__typename"].as_str()? {
                "CheckRun" => (
                    c["name"].as_str()?,
                    match (c["status"].as_str()?, c["conclusion"].as_str()) {
                        ("COMPLETED", Some("SUCCESS")) => "passed",
                        ("COMPLETED", Some("NEUTRAL" | "SKIPPED" | "STALE")) => "skipped",
                        ("COMPLETED", _) => "failed",
                        ("IN_PROGRESS", _) => "running",
                        _ => "waiting",
                    },
                ),
                "StatusContext" => (
                    c["context"].as_str()?,
                    match c["state"].as_str()? {
                        "SUCCESS" => "passed",
                        "FAILURE" | "ERROR" => "failed",
                        "PENDING" => "running",
                        _ => "waiting",
                    },
                ),
                _ => return None,
            };
            Some(json!({ "name": name, "state": state }))
        })
        .collect()
}

/// `2026-10-12T21:50:34Z` → Unix seconds; GitHub always answers in UTC.
fn unix(at: &str) -> Option<u64> {
    let b = at.as_bytes();
    if b.len() != 20
        || b[4] != b'-'
        || b[7] != b'-'
        || b[10] != b'T'
        || b[13] != b':'
        || b[16] != b':'
        || b[19] != b'Z'
    {
        return None;
    }
    let n = |r: std::ops::Range<usize>| at.get(r)?.parse::<i64>().ok();
    let (y, m, d) = (n(0..4)?, n(5..7)?, n(8..10)?);
    let (hh, mm, ss) = (n(11..13)?, n(14..16)?, n(17..19)?);
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) || hh > 23 || mm > 59 || ss > 60 {
        return None;
    }
    // Days from the civil date (Howard Hinnant's algorithm).
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * ((m + 9) % 12) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    u64::try_from(days * 86_400 + hh * 3600 + mm * 60 + ss).ok()
}

/// A branch or tag that can stand in a copied command as it is: nothing a shell reads as more.
fn plain_ref(name: &str) -> bool {
    (1..=100).contains(&name.len())
        && !name.starts_with('-')
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '/' | '-' | '+'))
}

/// What the detail shows of an item, and a `gh` command to look closer. The command is only
/// text to copy: it is built from checked names, so pasting it runs nothing else.
fn facts(kind: &str, name: &str, v: &Value, checks: Option<Checks>, review: Option<Review>) -> Facts {
    let text = |k: &str| v[k].as_str().filter(|s| !s.is_empty()).map(str::to_string);
    let count = |k: &str| v[k].as_u64().and_then(|n| u32::try_from(n).ok());
    let (repo, number) = name.split_once('#').map_or((name, None), |(r, n)| (r, Some(n)));
    let repo = plain_repo(repo).then(|| repo.to_string());
    let number = number.filter(|n| !n.is_empty() && n.len() <= 10 && n.chars().all(|c| c.is_ascii_digit()));
    let branch = text("branch");
    let command = repo.as_deref().and_then(|r| match (kind, number) {
        ("pr", Some(n)) => Some(match (checks, review) {
            (Some(Checks::Failing), _) => format!("gh pr checks {n} --repo {r}"),
            (Some(Checks::Running), _) => format!("gh pr checks {n} --repo {r} --watch"),
            // Pinned like the card's own merge: never a commit pushed since it showed green.
            (Some(Checks::Passing), Some(Review::Approved)) => {
                match v["oid"].as_str().filter(|o| commit(o)) {
                    Some(oid) => format!("gh pr merge {n} --repo {r} --squash --match-head-commit {oid}"),
                    None => format!("gh pr view {n} --repo {r}"),
                }
            }
            _ => format!("gh pr view {n} --repo {r}"),
        }),
        ("review", Some(n)) => Some(format!("gh pr diff {n} --repo {r}")),
        ("branch", _) => {
            let on = branch
                .as_deref()
                .filter(|b| plain_ref(b))
                .map(|b| format!(" --branch {b}"));
            let failed = if checks == Some(Checks::Failing) {
                " --status failure"
            } else {
                ""
            };
            Some(format!(
                "gh run list --repo {r}{}{failed}",
                on.unwrap_or_default()
            ))
        }
        ("release", _) => text("tag")
            .filter(|t| plain_ref(t))
            .map(|t| format!("gh release view {t} --repo {r}")),
        _ => None,
    });
    Facts {
        repo,
        branch,
        diff: match (count("added"), count("removed"), count("files")) {
            (Some(added), Some(removed), Some(files)) => Some(Diff {
                added,
                removed,
                files,
            }),
            _ => None,
        },
        author: text("author"),
        reviewers: v["reviewers"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|r| {
                Some(Reviewer {
                    login: r["login"].as_str()?.to_string(),
                    said: match r["state"].as_str()? {
                        "APPROVED" => Said::Approved,
                        "CHANGES_REQUESTED" => Said::Changes,
                        "WAITING" => Said::Waiting,
                        _ => Said::Commented,
                    },
                })
            })
            .collect(),
        updated: v["updated"].as_str().and_then(unix),
        runs: v["runs"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|r| {
                Some(Run {
                    name: r["name"].as_str()?.to_string(),
                    state: match r["state"].as_str()? {
                        "passed" => RunState::Passed,
                        "failed" => RunState::Failed,
                        "running" => RunState::Running,
                        "skipped" => RunState::Skipped,
                        _ => RunState::Waiting,
                    },
                })
            })
            .collect(),
        release: (kind == "release").then(|| {
            if v["draft"] == true {
                Release::Draft
            } else if v["pre"] == true {
                Release::Prerelease
            } else {
                Release::Latest
            }
        }),
        command,
    }
}

/// The card: open pull requests with their checks and review, reviews waiting for the user, and
/// the default branches that have checks.
fn board(snapshot: &Snapshot) -> Vec<Row> {
    let mut rows: Vec<(Group, u64, Row)> = snapshot
        .iter()
        .filter_map(|(key, v)| {
            let (kind, name) = key.split_once(':')?;
            let text = |k: &str| v[k].as_str().unwrap_or_default().to_string();
            let checks = match v["ci"].as_str() {
                Some("SUCCESS") => Some(Checks::Passing),
                Some("FAILURE" | "ERROR") => Some(Checks::Failing),
                Some("PENDING" | "EXPECTED") => Some(Checks::Running),
                _ => None,
            };
            // `owner/repo#12` → `repo#12`: the card is narrow, the link has the rest.
            let short = name.rsplit_once('/').map_or(name, |(_, n)| n).to_string();
            let (group, title, review) = match kind {
                "pr" => (
                    Group::Yours,
                    text("title"),
                    match v["review"].as_str() {
                        Some("APPROVED") => Some(Review::Approved),
                        Some("CHANGES_REQUESTED") => Some(Review::Changes),
                        _ => None,
                    },
                ),
                "review" => (Group::ToReview, text("title"), None),
                // Kept without checks too, so its alerts stay (a `[skip ci]` push after a
                // failure); the card's view leaves such rows out.
                "branch" => (
                    Group::Branches,
                    format!("{} · {}", text("branch"), text("headline")),
                    None,
                ),
                "release" => (
                    Group::Releases,
                    Some(text("title"))
                        .filter(|t| !t.is_empty())
                        .unwrap_or_else(|| text("tag")),
                    None,
                ),
                _ => return None,
            };
            // `vults v0.1.7`: a release's line names its tag.
            let short = if kind == "release" {
                format!("{short} {}", text("tag"))
            } else {
                short
            };
            let row = Row {
                item: key.clone(),
                group,
                name: short,
                title,
                checks,
                review,
                url: v["url"].as_str().map(str::to_string),
                head: (kind == "pr")
                    .then(|| v["oid"].as_str().map(str::to_string))
                    .flatten(),
                facts: facts(kind, name, v, checks, review),
            };
            Some((group, v["rank"].as_u64().unwrap_or(u64::MAX), row))
        })
        .collect();
    rows.sort_by_key(|(group, rank, _)| (*group, *rank));
    rows.into_iter().map(|(_, _, row)| row).collect()
}

fn diff(before: &Snapshot, after: &Snapshot) -> Vec<Event> {
    let mut out = Vec::new();
    for (key, now) in after {
        let was = before.get(key);
        let text = |v: &Value, k: &str| v[k].as_str().unwrap_or_default().to_string();
        let url = now["url"].as_str().map(str::to_string);
        let (kind, name) = key.split_once(':').unwrap_or((key, ""));
        // `topic`: the story a news belongs to (`ci`, `review`); a newer one retires the older.
        let mut push =
            |news: &str, topic: Option<&str>, level: Level, title: String, detail: String, story: Story| {
                out.push(Event {
                    connector: "github".into(),
                    key: format!("{key}:{news}"),
                    topic: topic.map(|t| format!("{key}:{t}")),
                    level,
                    title,
                    detail,
                    url: url.clone(),
                    story: Some(story),
                });
            };
        match kind {
            "review" if was.is_none() => {
                push(
                    "requested",
                    None,
                    Level::Info,
                    format!("Review requested · {name}"),
                    text(now, "title"),
                    Story {
                        kind: StoryKind::ReviewRequested,
                        name: name.to_string(),
                        branch: None,
                    },
                );
            }
            "pr" => {
                // A push whose checks finished between two polls leaves `ci` as it was: the
                // commit says it is news. A snapshot saved before we kept the commit has no
                // `oid`, and reads as the same commit so an upgrade replays nothing.
                let same_commit = was.is_none_or(|w| w.get("oid").is_none() || w["oid"] == now["oid"]);
                let ci = now["ci"].as_str();
                if !same_commit || ci != was.and_then(|w| w["ci"].as_str()) {
                    match ci {
                        Some("FAILURE" | "ERROR") => push(
                            &format!("ci-failed:{}", text(now, "oid")),
                            Some("ci"),
                            Level::Error,
                            format!("Checks failed · {name}"),
                            text(now, "title"),
                            Story {
                                kind: StoryKind::ChecksFailed,
                                name: name.to_string(),
                                branch: None,
                            },
                        ),
                        Some("SUCCESS") if was.is_some() => push(
                            &format!("ci-passed:{}", text(now, "oid")),
                            Some("ci"),
                            Level::Ok,
                            format!("Checks passed · {name}"),
                            text(now, "title"),
                            Story {
                                kind: StoryKind::ChecksPassed,
                                name: name.to_string(),
                                branch: None,
                            },
                        ),
                        _ => {}
                    }
                }
                let review = now["review"].as_str();
                if review != was.and_then(|w| w["review"].as_str()) {
                    match review {
                        Some("APPROVED") => push(
                            "approved",
                            Some("review"),
                            Level::Ok,
                            format!("Approved · {name}"),
                            text(now, "title"),
                            Story {
                                kind: StoryKind::Approved,
                                name: name.to_string(),
                                branch: None,
                            },
                        ),
                        Some("CHANGES_REQUESTED") => push(
                            "changes",
                            Some("review"),
                            Level::Warn,
                            format!("Changes requested · {name}"),
                            text(now, "title"),
                            Story {
                                kind: StoryKind::ChangesRequested,
                                name: name.to_string(),
                                branch: None,
                            },
                        ),
                        _ => {}
                    }
                }
            }
            "branch" => {
                let same_commit = was.and_then(|w| w["oid"].as_str()) == now["oid"].as_str();
                let ci = now["ci"].as_str();
                let ci_changed = !same_commit || ci != was.and_then(|w| w["ci"].as_str());
                let branch = text(now, "branch");
                let was_red = matches!(was.and_then(|w| w["ci"].as_str()), Some("FAILURE" | "ERROR"));
                match ci {
                    Some("FAILURE" | "ERROR") if ci_changed => push(
                        &format!("ci-failed:{}", text(now, "oid")),
                        Some("ci"),
                        Level::Error,
                        format!("Checks failed on {branch} · {name}"),
                        text(now, "headline"),
                        Story {
                            kind: StoryKind::ChecksFailed,
                            name: name.to_string(),
                            branch: Some(branch.clone()),
                        },
                    ),
                    // Passing is news when we watched it run (or fail) on this commit, or when it
                    // fixes a failure: that retires the failure's alert.
                    Some("SUCCESS") if ci_changed && (same_commit || was_red) => push(
                        &format!("ci-passed:{}", text(now, "oid")),
                        Some("ci"),
                        Level::Ok,
                        format!("Checks passed on {branch} · {name}"),
                        text(now, "headline"),
                        Story {
                            kind: StoryKind::ChecksPassed,
                            name: name.to_string(),
                            branch: Some(branch.clone()),
                        },
                    ),
                    _ => {}
                }
            }
            _ => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{IssueWrite as W, duplicate_write, write_is_present};
    use serde_json::json;

    #[test]
    fn issue_writes_are_validated_typed_and_bound_to_the_exact_content() {
        let writes = [
            W::Create {
                repository: "org/repo".into(),
                title: "Bug".into(),
                body: "Details".into(),
            },
            W::Edit {
                repository: "org/repo".into(),
                number: 12,
                title: Some("New title".into()),
                body: None,
            },
            W::Comment {
                repository: "org/repo".into(),
                number: 12,
                body: "Reviewed".into(),
            },
            W::AddLabels {
                repository: "org/repo".into(),
                number: 12,
                labels: vec!["bug".into()],
            },
            W::RemoveLabels {
                repository: "org/repo".into(),
                number: 12,
                labels: vec!["triage".into()],
            },
            W::SetAssignees {
                repository: "org/repo".into(),
                number: 12,
                assignees: vec![],
            },
            W::Close {
                repository: "org/repo".into(),
                number: 12,
            },
            W::Reopen {
                repository: "org/repo".into(),
                number: 12,
            },
        ];
        for write in &writes {
            write.validate().unwrap();
            assert_eq!(write.request_hash(), write.request_hash());
        }
        assert_ne!(
            writes[0].request_hash(),
            W::Create {
                repository: "org/repo".into(),
                title: "Bug".into(),
                body: "Changed".into()
            }
            .request_hash()
        );
        assert!(
            W::Close {
                repository: "org/repo".into(),
                number: 0
            }
            .validate()
            .is_err()
        );
        assert!(
            W::Comment {
                repository: "org/repo".into(),
                number: 1,
                body: "\n".into()
            }
            .validate()
            .is_err()
        );
        assert!(
            W::AddLabels {
                repository: "org/repo".into(),
                number: 1,
                labels: vec!["bug".into(), "bug".into()]
            }
            .validate()
            .is_err()
        );
    }

    #[test]
    fn issue_writes_detect_duplicates_and_verify_readback_by_operation() {
        let current = json!({
            "issue":{"title":"Current","body":"Body","state":"closed","labels":["bug"],"assignees":[]},
            "related":{"matches":[{"number":12}],"comments":[{"body":"Exact comment"}]}
        });
        assert!(duplicate_write(
            &W::Create {
                repository: "org/repo".into(),
                title: "x".into(),
                body: "y".into()
            },
            &current
        ));
        assert!(duplicate_write(
            &W::Comment {
                repository: "org/repo".into(),
                number: 12,
                body: "Exact comment".into()
            },
            &current
        ));
        assert!(write_is_present(
            &W::Close {
                repository: "org/repo".into(),
                number: 12
            },
            &current
        ));
        assert!(write_is_present(
            &W::AddLabels {
                repository: "org/repo".into(),
                number: 12,
                labels: vec!["bug".into()]
            },
            &current
        ));
        assert!(!write_is_present(
            &W::Reopen {
                repository: "org/repo".into(),
                number: 12
            },
            &current
        ));
    }

    #[test]
    fn gh_says_why_as_it_said_it() {
        let e = super::gh_error(
            false,
            b"X Pull request me/app#401 is not mergeable: the base branch policy prohibits the merge.\n",
        );
        assert!(
            matches!(&e, super::Error::Other(m) if m == "Pull request me/app#401 is not mergeable: the base branch policy prohibits the merge."),
            "{e}"
        );
        assert!(matches!(
            super::gh_error(false, b"HTTP 401: Bad credentials"),
            super::Error::Auth(_)
        ));
        assert!(super::commit("0123456789abcdef0123456789abcdef01234567"));
        assert!(!super::commit("--admin"));
        assert!(!super::commit("abc"));
    }

    #[test]
    fn only_a_plain_pull_request_is_merged() {
        assert_eq!(super::pull_request("pr:me/app#12"), Some(("me/app", "12")));
        for bad in [
            "review:me/app#12",
            "branch:me/app",
            "pr:me/app#",
            "pr:me/app#012",
            "pr:me/app#12a",
            "pr:me/app#-1",
            "pr:me#12",
            "pr:me/a/b#12",
            "pr:../app#12",
            "pr:me/app#12 --admin",
            "pr:--admin/app#1",
        ] {
            assert_eq!(super::pull_request(bad), None, "{bad}");
        }
    }

    use super::*;

    #[test]
    fn action_read_sources_are_explicitly_live() {
        let source = source("team/app");
        assert_eq!(source["provider"], "github");
        assert_eq!(source["mode"], "live");
        assert_eq!(source["repository"], "team/app");
        assert_eq!(source["url"], "https://github.com/team/app");
        assert!(source["fetchedAt"].as_u64().is_some());
    }

    #[test]
    fn repository_names_are_plain() {
        assert!(plain_repo("me/site") && plain_repo("my-org/site.rs"));
        assert!(
            !plain_repo("me") && !plain_repo("me/site/x") && !plain_repo("../x") && !plain_repo("me/s ite")
        );
    }

    #[test]
    fn news_says_what_it_is_for_the_app_to_translate() {
        let before: Snapshot = [(
            "branch:me/app".to_string(),
            serde_json::json!({ "branch": "main", "ci": "SUCCESS", "oid": "a1", "headline": "Fix" }),
        )]
        .into();
        let after: Snapshot = [(
            "branch:me/app".to_string(),
            serde_json::json!({ "branch": "main", "ci": "FAILURE", "oid": "b2", "headline": "Break" }),
        )]
        .into();
        let events = diff(&before, &after);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].title, "Checks failed on main · me/app");
        assert_eq!(
            events[0].story,
            Some(Story {
                kind: StoryKind::ChecksFailed,
                name: "me/app".into(),
                branch: Some("main".into())
            })
        );
    }

    #[test]
    fn the_calendar_reads_days_and_github_levels() {
        // The shape `gh api graphql` prints for CALENDAR, made up (no one's real calendar).
        let out = br#"{"data":{"viewer":{"contributionsCollection":{"contributionCalendar":{
          "totalContributions": 12,
          "weeks": [
            {"contributionDays": [
              {"date":"2026-10-04","contributionCount":0,"contributionLevel":"NONE"},
              {"date":"2026-10-05","contributionCount":3,"contributionLevel":"FIRST_QUARTILE"}]},
            {"contributionDays": [
              {"date":"2026-10-11","contributionCount":9,"contributionLevel":"FOURTH_QUARTILE"},
              {"date":"broken"}]}
          ]}}}}}"#;
        let c = calendar_answer(true, out, b"").expect("a calendar");
        assert_eq!(c.total, 12);
        assert_eq!(
            c.days,
            [
                CalendarDay {
                    day: "2026-10-04".into(),
                    count: 0,
                    level: 0
                },
                CalendarDay {
                    day: "2026-10-05".into(),
                    count: 3,
                    level: 1
                },
                CalendarDay {
                    day: "2026-10-11".into(),
                    count: 9,
                    level: 4
                },
            ]
        );
    }

    #[test]
    fn a_calendar_that_cannot_come_says_why() {
        assert!(matches!(
            calendar_answer(
                false,
                b"",
                b"To get started with GitHub CLI, please run:  gh auth login"
            ),
            Err(Error::Auth(_))
        ));
        assert!(matches!(
            calendar_answer(false, b"", b"API rate limit exceeded"),
            Err(Error::RateLimited { .. })
        ));
        assert!(matches!(calendar_answer(true, b"{}", b""), Err(Error::Other(_))));
    }

    fn work_item_fixture(kind: &str) -> Value {
        let base = json!({
            "__typename": kind, "number": 12, "title": "A request", "body": "Description",
            "url": "https://github.example.com/team/app/issues/12", "state": "OPEN",
            "createdAt": "2026-10-20T10:00:00Z", "updatedAt": "2026-10-21T10:00:00Z",
            "comments": { "totalCount": 1, "pageInfo": { "hasNextPage": false }, "nodes": [
                { "body": "A comment", "createdAt": "2026-10-21T09:00:00Z", "url": "https://github.example.com/team/app/issues/12#issuecomment-1" }
            ] }
        });
        let mut item = base;
        if kind == "PullRequest" {
            item["baseRefName"] = json!("main");
            item["headRefName"] = json!("feature");
            item["headRefOid"] = json!("0123456789abcdef");
            item["additions"] = json!(5);
            item["deletions"] = json!(2);
            item["changedFiles"] = json!(1);
            item["reviewDecision"] = json!("APPROVED");
            item["mergeStateStatus"] = json!("CLEAN");
            item["mergeable"] = json!("MERGEABLE");
            item["isDraft"] = json!(false);
            item["files"] = json!({ "totalCount": 1, "pageInfo": { "hasNextPage": false }, "nodes": [
                { "path": "src/main.rs", "additions": 5, "deletions": 2 }
            ] });
            item["latestReviews"] = json!({ "totalCount": 1, "pageInfo": { "hasNextPage": false, "endCursor": null }, "nodes": [
                { "state": "APPROVED", "body": "Looks good", "submittedAt": "2026-10-21T10:00:00Z", "author": { "login": "reviewer" } }
            ] });
            item["reviewThreads"] =
                json!({"totalCount":0,"pageInfo":{"hasNextPage":false,"endCursor":null},"nodes":[]});
            item["commits"] = json!({ "nodes": [{ "commit": { "oid": "0123456789abcdef", "statusCheckRollup": {
                "state": "SUCCESS", "contexts": { "totalCount": 1, "pageInfo": { "hasNextPage": false, "endCursor": null }, "nodes": [
                    { "__typename": "CheckRun", "name": "ci", "status": "COMPLETED", "conclusion": "SUCCESS" }
                ] }
            } } }] });
        }
        json!({ "data": { "repository": { "nameWithOwner": "team/app", "url": "https://github.example.com/team/app", "issueOrPullRequest": item } } })
    }

    #[test]
    fn the_last_check_page_uses_page_info_instead_of_the_global_total() {
        let mut value = work_item_fixture("PullRequest");
        value["data"]["repository"]["issueOrPullRequest"]["commits"]["nodes"][0]["commit"]["statusCheckRollup"]
            ["contexts"]["totalCount"] = json!(31);
        let answer = super::work_item_answer(
            true,
            &serde_json::to_vec(&value).unwrap(),
            b"",
            "org/repo",
            std::time::SystemTime::now(),
        )
        .unwrap();
        assert_eq!(answer["pullRequest"]["checks"]["hasMore"], false);
        assert!(answer["pullRequest"]["checks"]["nextCursor"].is_null());
    }

    #[test]
    fn work_item_read_returns_source_and_explicit_issue_discussion_coverage() {
        let bytes = serde_json::to_vec(&work_item_fixture("Issue")).unwrap();
        let result = work_item_answer(true, &bytes, b"", "team/app", std::time::UNIX_EPOCH).unwrap();
        assert_eq!(result["kind"], "issue");
        assert_eq!(result["source"]["mode"], "live");
        assert_eq!(result["source"]["repository"], "team/app");
        assert_eq!(result["source"]["fetchedAt"], 0);
        assert_eq!(result["source"]["mode"], "live");
        assert_eq!(result["discussion"]["comments"][0]["body"], "A comment");
        assert_eq!(result["discussion"]["hasMore"], false);
        assert_eq!(result["discussion"]["nextCursor"], Value::Null);
    }

    #[test]
    fn work_item_read_returns_a_cursor_for_the_next_discussion_page() {
        let mut response = work_item_fixture("Issue");
        response["data"]["repository"]["issueOrPullRequest"]["comments"]["pageInfo"] =
            json!({ "hasNextPage": true, "endCursor": "discussion-page-2" });
        let result = work_item_answer(
            true,
            &serde_json::to_vec(&response).unwrap(),
            b"",
            "team/app",
            std::time::UNIX_EPOCH,
        )
        .unwrap();
        assert_eq!(result["discussion"]["hasMore"], true);
        assert_eq!(result["discussion"]["nextCursor"], "discussion-page-2");
    }

    #[test]
    fn work_item_read_bounds_long_text_and_pr_collections() {
        let mut response = work_item_fixture("PullRequest");
        response["data"]["repository"]["issueOrPullRequest"]["body"] = json!("é".repeat(4000)); // check-english:allow (multi-byte truncation fixture)
        response["data"]["repository"]["issueOrPullRequest"]["files"]["totalCount"] = json!(30);
        response["data"]["repository"]["issueOrPullRequest"]["files"]["pageInfo"] =
            json!({ "hasNextPage": true, "endCursor": "files-page-2" });
        response["data"]["repository"]["issueOrPullRequest"]["latestReviews"]["pageInfo"] =
            json!({ "hasNextPage": true, "endCursor": "reviews-page-2" });
        response["data"]["repository"]["issueOrPullRequest"]["reviewThreads"]["totalCount"] = json!(6);
        response["data"]["repository"]["issueOrPullRequest"]["reviewThreads"]["pageInfo"] =
            json!({ "hasNextPage": true, "endCursor": "threads-page-2" });
        response["data"]["repository"]["issueOrPullRequest"]["reviewThreads"]["nodes"] = json!([{
            "path":"src/main.rs","line":4,"isResolved":false,"isOutdated":false,
            "comments":{"totalCount":1,"pageInfo":{"hasNextPage":false},"nodes":[
                {"body":"Please add a check","createdAt":"2026-10-21T10:00:00Z","url":"https://github.com/team/app/pull/12#discussion_r1","author":{"login":"reviewer"}}
            ]}
        }]);
        response["data"]["repository"]["issueOrPullRequest"]["commits"]["nodes"][0]["commit"]["statusCheckRollup"]
            ["contexts"]["pageInfo"] = json!({ "hasNextPage": true, "endCursor": "checks-page-2" });
        for n in 0..25 {
            response["data"]["repository"]["issueOrPullRequest"]["files"]["nodes"]
                .as_array_mut()
                .unwrap()
                .push(json!({ "path": format!("file-{n}.rs"), "additions": 1, "deletions": 0 }));
        }
        let bytes = serde_json::to_vec(&response).unwrap();
        let result = work_item_answer(true, &bytes, b"", "team/app", std::time::UNIX_EPOCH).unwrap();
        assert_eq!(result["kind"], "pullRequest");
        assert_eq!(result["bodyTruncated"], true);
        assert_eq!(result["pullRequest"]["headSha"], "0123456789abcdef");
        assert_eq!(
            result["pullRequest"]["files"]["items"].as_array().unwrap().len(),
            10
        );
        assert_eq!(result["pullRequest"]["files"]["hasMore"], true);
        assert_eq!(result["pullRequest"]["files"]["nextCursor"], "files-page-2");
        assert_eq!(result["pullRequest"]["reviews"]["items"][0]["author"], "reviewer");
        assert_eq!(result["pullRequest"]["reviews"]["nextCursor"], "reviews-page-2");
        assert_eq!(
            result["pullRequest"]["reviewThreads"]["items"][0]["comments"]["items"][0]["body"],
            "Please add a check"
        );
        assert_eq!(
            result["pullRequest"]["reviewThreads"]["nextCursor"],
            "threads-page-2"
        );
        assert_eq!(result["pullRequest"]["checks"]["items"][0]["state"], "passed");
        assert_eq!(result["pullRequest"]["checks"]["nextCursor"], "checks-page-2");
    }

    #[test]
    fn work_item_read_never_turns_partial_or_missing_access_into_absence() {
        let mut response = work_item_fixture("Issue");
        response["errors"] = json!([{ "type": "FORBIDDEN", "message": "SAML" }]);
        assert!(matches!(
            work_item_answer(
                true,
                &serde_json::to_vec(&response).unwrap(),
                b"",
                "team/app",
                std::time::UNIX_EPOCH
            ),
            Err(Error::Partial(_))
        ));
        let missing = json!({ "data": { "repository": null } });
        assert!(matches!(
            work_item_answer(true, &serde_json::to_vec(&missing).unwrap(), b"", "team/app", std::time::UNIX_EPOCH),
            Err(Error::Other(message)) if message.contains("cannot read")
        ));
    }

    #[tokio::test]
    async fn invalid_work_item_targets_are_refused_before_running_gh() {
        assert!(
            work_item("--repo/evil", 1, None, None, None, None, None)
                .await
                .is_err()
        );
        assert!(
            work_item("team/app", 0, None, None, None, None, None)
                .await
                .is_err()
        );
        assert!(
            work_item("team/app", 1, Some(&"x".repeat(513)), None, None, None, None)
                .await
                .is_err()
        );
        assert!(
            work_item("team/app", 1, None, Some(&"x".repeat(513)), None, None, None)
                .await
                .is_err()
        );
        assert!(
            work_item("team/app", 1, None, None, None, Some("\n"), None)
                .await
                .is_err()
        );
        assert!(branch_rules("team/app", "main*", 1).await.is_err());
    }

    #[test]
    fn repository_search_returns_bounded_matches_and_source_time() {
        let nodes: Vec<Value> = (0..12)
            .map(|n| {
                json!({
                    "nameWithOwner": format!("team/app-{n}"),
                    "url": format!("https://github.com/team/app-{n}"),
                    "description": "A repository",
                    "isPrivate": n % 2 == 0,
                    "isArchived": false,
                    "pushedAt": "2026-10-20T10:00:00Z",
                    "defaultBranchRef": { "name": "main" }
                })
            })
            .collect();
        let response = json!({ "data": {
            "viewer": { "login": "maintainer", "url": "https://github.com/maintainer" },
            "search": { "repositoryCount": 12, "pageInfo": { "hasNextPage": true, "endCursor": "cursor-page-2" }, "nodes": nodes }
        } });
        let result = repository_search_answer(
            true,
            &serde_json::to_vec(&response).unwrap(),
            b"",
            std::time::UNIX_EPOCH,
        )
        .unwrap();
        assert_eq!(result["repositories"].as_array().unwrap().len(), 10);
        assert_eq!(result["repositories"][0]["repository"], "team/app-0");
        assert_eq!(result["total"], 12);
        assert_eq!(result["hasMore"], true);
        assert_eq!(result["nextCursor"], "cursor-page-2");
        assert_eq!(result["source"]["fetchedAt"], 0);
        assert_eq!(result["source"]["account"], "maintainer");
        assert_eq!(result["source"]["host"], "github.com");
    }

    #[test]
    fn account_and_repository_reads_keep_identity_and_access_provenance() {
        let account = account_answer(
            true,
            br#"{"data":{"viewer":{"login":"luna","name":"Luna","url":"https://github.example.com/luna"}}}"#,
            b"",
            std::time::UNIX_EPOCH,
        )
        .unwrap();
        assert_eq!(account["source"]["account"], "luna");
        assert_eq!(account["source"]["mode"], "live");
        assert_eq!(account["source"]["host"], "github.example.com");
        assert_eq!(account["source"]["fetchedAt"], 0);
        let repository = repository_answer(
            json!({
                "full_name":"team/app","html_url":"https://github.example.com/team/app","private":true,
                "archived":false,"fork":false,"default_branch":"main","pushed_at":"2026-10-20T10:00:00Z",
                "description":"Internal project"
            }),
            "team/app",
            std::time::UNIX_EPOCH,
        )
        .unwrap();
        assert_eq!(repository["private"], true);
        assert_eq!(repository["defaultBranch"], "main");
        assert_eq!(repository["source"]["url"], "https://github.example.com/team/app");
        assert_eq!(repository["source"]["mode"], "live");
        assert!(
            repository_answer(json!({"full_name":"team/app"}), "team/app", std::time::UNIX_EPOCH).is_err()
        );
        assert!(account_answer(true, br#"{"data":{"viewer":null}}"#, b"", std::time::UNIX_EPOCH).is_err());
    }

    #[test]
    fn diff_fixture_marks_binary_and_bounded_coverage() {
        let result = diff_answer(
            true,
            b"diff --git a/image.png b/image.png\nBinary files a/image.png and b/image.png differ\n",
            b"",
            "team/app",
            12,
            false,
            std::time::UNIX_EPOCH,
        )
        .unwrap();
        assert_eq!(result["containsBinary"], true);
        assert_eq!(result["source"]["fetchedAt"], 0);
        assert_eq!(result["source"]["mode"], "live");
        let oversized = diff_answer(
            true,
            &vec![b'x'; 300 * 1024],
            b"",
            "team/app",
            12,
            true,
            std::time::UNIX_EPOCH,
        )
        .unwrap();
        assert_eq!(oversized["truncated"], true);
        assert_eq!(oversized["containsBinary"], Value::Null);
        assert_eq!(oversized["coverage"], "partial-byte-limit");
    }

    #[test]
    fn branch_rules_pagination_uses_the_link_header() {
        let response = b"HTTP/2 200\r\nLink: <https://api.github.com/page=2>; rel=\"next\", <https://api.github.com/page=4>; rel=\"last\"\r\n\r\n[]";
        let (body, more) = response_body_and_next(response).unwrap();
        assert_eq!(body, b"[]");
        assert!(more);
        assert!(!response_body_and_next(b"HTTP/2 200\r\n\r\n[]").unwrap().1);
        assert!(response_body_and_next(b"[]").is_err());
    }

    #[test]
    fn repository_search_rejects_partial_and_missing_coverage() {
        let response = json!({
            "errors": [{ "type": "FORBIDDEN" }],
            "data": {
                "viewer": { "login": "maintainer", "url": "https://github.com/maintainer" },
                "search": { "repositoryCount": 0, "pageInfo": { "hasNextPage": false }, "nodes": [] }
            }
        });
        assert!(matches!(
            repository_search_answer(
                true,
                &serde_json::to_vec(&response).unwrap(),
                b"",
                std::time::UNIX_EPOCH
            ),
            Err(Error::Partial(_))
        ));
        assert!(matches!(
            repository_search_answer(
                true,
                br#"{"data":{"viewer":{"login":"x","url":"https://github.com/x"},"search":{"nodes":[]}}}"#,
                b"",
                std::time::UNIX_EPOCH
            ),
            Err(Error::Partial(_))
        ));
        let no_cursor = json!({ "data": {
            "viewer": { "login": "x", "url": "https://github.com/x" },
            "search": { "repositoryCount": 11, "pageInfo": { "hasNextPage": true }, "nodes": [] }
        } });
        assert!(matches!(
            repository_search_answer(
                true,
                &serde_json::to_vec(&no_cursor).unwrap(),
                b"",
                std::time::UNIX_EPOCH
            ),
            Err(Error::Partial(_))
        ));
    }

    #[tokio::test]
    async fn invalid_repository_search_is_refused_before_running_gh() {
        assert!(search_repositories(" ", None).await.is_err());
        assert!(search_repositories(&"x".repeat(161), None).await.is_err());
        assert!(search_repositories("team\nrepo", None).await.is_err());
        assert!(
            search_repositories("vults", Some(&"x".repeat(513)))
                .await
                .is_err()
        );
    }

    fn answer(pr_ci: &str, review: Value, branch_ci: &str, oid: &str, requested: bool) -> Value {
        json!({ "data": {
            "viewer": {
                "login": "maintainer", "url": "https://github.example.com/maintainer",
                "pullRequests": { "nodes": [ {
                    "number": 12, "title": "Add the flock", "url": "https://github.com/me/app/pull/12",
                    "repository": { "nameWithOwner": "me/app" }, "reviewDecision": review,
                    "commits": { "nodes": [ { "commit": { "oid": "p1", "statusCheckRollup": { "state": pr_ci } } } ] }
                } ] },
                "repositories": { "nodes": [ {
                    "nameWithOwner": "me/app", "url": "https://github.com/me/app", "isArchived": false,
                    "defaultBranchRef": { "name": "main", "target": {
                        "oid": oid, "messageHeadline": "Fix landing", "statusCheckRollup": { "state": branch_ci } } }
                } ] }
            },
            "search": { "nodes": if requested { json!([ {
                "number": 7, "title": "Bump deps", "url": "https://github.com/team/lib/pull/7",
                "repository": { "nameWithOwner": "team/lib" } } ]) } else { json!([]) } }
        }})
    }

    /// The same answer with the pull request on another commit.
    fn pushed(mut answer: Value, oid: &str) -> Value {
        answer["data"]["viewer"]["pullRequests"]["nodes"][0]["commits"]["nodes"][0]["commit"]["oid"] =
            json!(oid);
        answer
    }

    fn keys(events: &[Event]) -> Vec<&str> {
        events.iter().map(|e| e.key.as_str()).collect()
    }

    #[test]
    fn nothing_changed_means_no_news() {
        let a = snapshot(&answer("PENDING", Value::Null, "SUCCESS", "abc", false));
        assert!(diff(&a, &a).is_empty());
    }

    #[test]
    fn account_label_comes_from_the_same_snapshot_without_becoming_a_watched_item() {
        let s = snapshot(&answer("SUCCESS", Value::Null, "SUCCESS", "abc", false));
        assert_eq!(
            GitHub.identity(&s),
            Some(("maintainer".into(), "github.example.com".into()))
        );
        assert_eq!(GitHub.watching(&s), 2);
        assert_eq!(s.len(), 3, "two work items and one metadata entry");
        assert!(
            diff(&Snapshot::new(), &s).is_empty(),
            "identity is not connector news"
        );
        assert!(board(&s).iter().all(|row| row.name != "maintainer"));
    }

    #[test]
    fn github_profile_host_accepts_only_an_https_authority() {
        assert_eq!(github_host("https://github.com/maintainer"), Some("github.com"));
        assert_eq!(github_host("http://github.com/maintainer"), None);
        assert_eq!(github_host("https://user@github.com/profile"), None);
        assert_eq!(github_host("https:///profile"), None);
    }

    #[test]
    fn pull_request_news() {
        let before = snapshot(&answer("PENDING", Value::Null, "SUCCESS", "abc", false));
        let after = snapshot(&answer(
            "FAILURE",
            json!("CHANGES_REQUESTED"),
            "SUCCESS",
            "abc",
            true,
        ));
        let news = diff(&before, &after);
        assert_eq!(
            keys(&news),
            [
                "pr:me/app#12:ci-failed:p1",
                "pr:me/app#12:changes",
                "review:team/lib#7:requested"
            ]
        );
        assert_eq!(news[0].level, Level::Error);
        assert_eq!(news[0].title, "Checks failed · me/app#12");
        assert_eq!(news[0].url.as_deref(), Some("https://github.com/me/app/pull/12"));

        let fixed = snapshot(&answer("SUCCESS", json!("APPROVED"), "SUCCESS", "abc", true));
        assert_eq!(
            keys(&diff(&after, &fixed)),
            ["pr:me/app#12:ci-passed:p1", "pr:me/app#12:approved"]
        );
    }

    #[test]
    fn a_push_whose_checks_finished_between_polls_is_news() {
        let green = snapshot(&answer("SUCCESS", Value::Null, "SUCCESS", "abc", false));
        let green_again = snapshot(&pushed(
            answer("SUCCESS", Value::Null, "SUCCESS", "abc", false),
            "p2",
        ));
        assert_eq!(keys(&diff(&green, &green_again)), ["pr:me/app#12:ci-passed:p2"]);
        let red = snapshot(&pushed(
            answer("FAILURE", Value::Null, "SUCCESS", "abc", false),
            "p3",
        ));
        let red_again = snapshot(&pushed(
            answer("FAILURE", Value::Null, "SUCCESS", "abc", false),
            "p4",
        ));
        assert_eq!(keys(&diff(&red, &red_again)), ["pr:me/app#12:ci-failed:p4"]);
        // Still running on the new commit: the next poll tells.
        let running = snapshot(&pushed(
            answer("PENDING", Value::Null, "SUCCESS", "abc", false),
            "p5",
        ));
        assert!(diff(&green, &running).is_empty());
    }

    #[test]
    fn a_snapshot_from_before_the_commit_was_kept_replays_nothing() {
        let mut old = snapshot(&answer("SUCCESS", Value::Null, "SUCCESS", "abc", false));
        old.get_mut("pr:me/app#12")
            .unwrap()
            .as_object_mut()
            .unwrap()
            .remove("oid");
        let now = snapshot(&answer("SUCCESS", Value::Null, "SUCCESS", "abc", false));
        assert!(diff(&old, &now).is_empty());
    }

    #[test]
    fn polls_sooner_while_checks_run() {
        let every = |a: Value| GitHub.interval(&snapshot(&a)).as_secs();
        assert_eq!(every(answer("PENDING", Value::Null, "SUCCESS", "abc", false)), 60);
        assert_eq!(
            every(answer("SUCCESS", Value::Null, "EXPECTED", "abc", false)),
            60
        );
        assert_eq!(every(answer("SUCCESS", Value::Null, "FAILURE", "abc", true)), 300);
        assert_eq!(GitHub.interval(&Snapshot::new()).as_secs(), 300);
    }

    #[test]
    fn default_branch_news() {
        let running = snapshot(&answer("PENDING", Value::Null, "PENDING", "def", false));
        let before = snapshot(&answer("PENDING", Value::Null, "SUCCESS", "abc", false));
        // A new commit still running: no news yet.
        assert!(diff(&before, &running).is_empty());
        let passed = snapshot(&answer("PENDING", Value::Null, "SUCCESS", "def", false));
        let news = diff(&running, &passed);
        assert_eq!(keys(&news), ["branch:me/app:ci-passed:def"]);
        assert_eq!(news[0].title, "Checks passed on main · me/app");
        assert_eq!(
            news[0].url.as_deref(),
            Some("https://github.com/me/app/commit/def")
        );
        // A new commit that fails straight away is news even without seeing it run.
        let failed = snapshot(&answer("PENDING", Value::Null, "FAILURE", "ghi", false));
        assert_eq!(keys(&diff(&passed, &failed)), ["branch:me/app:ci-failed:ghi"]);
        // Fixed by a newer commit whose checks finished between polls: the pass retires the failure.
        let fixed = snapshot(&answer("PENDING", Value::Null, "SUCCESS", "jkl", false));
        assert_eq!(keys(&diff(&failed, &fixed)), ["branch:me/app:ci-passed:jkl"]);
    }

    #[test]
    fn news_of_one_story_shares_a_topic() {
        let before = snapshot(&answer("PENDING", Value::Null, "PENDING", "abc", false));
        let after = snapshot(&answer("FAILURE", json!("APPROVED"), "FAILURE", "abc", true));
        let topics: Vec<_> = diff(&before, &after).into_iter().map(|e| e.topic).collect();
        assert_eq!(
            topics,
            [
                Some("branch:me/app:ci".to_string()),
                Some("pr:me/app#12:ci".to_string()),
                Some("pr:me/app#12:review".to_string()),
                None,
            ]
        );
    }

    #[test]
    fn the_card_lists_pull_requests_reviews_and_branches_in_github_order() {
        let mut a = answer("FAILURE", json!("APPROVED"), "PENDING", "abc", true);
        // A second pull request, updated less recently, and a repository without checks.
        let mut older = a["data"]["viewer"]["pullRequests"]["nodes"][0].clone();
        older["number"] = json!(3);
        older["title"] = json!("Older");
        older["reviewDecision"] = json!("CHANGES_REQUESTED");
        older["commits"]["nodes"][0]["commit"]["statusCheckRollup"] = Value::Null;
        a["data"]["viewer"]["pullRequests"]["nodes"]
            .as_array_mut()
            .unwrap()
            .push(older);
        let mut bare = a["data"]["viewer"]["repositories"]["nodes"][0].clone();
        bare["nameWithOwner"] = json!("me/notes");
        bare["defaultBranchRef"]["target"]["statusCheckRollup"] = Value::Null;
        a["data"]["viewer"]["repositories"]["nodes"]
            .as_array_mut()
            .unwrap()
            .push(bare);

        let rows = board(&snapshot(&a));
        let shown: Vec<_> = rows
            .iter()
            .map(|r| (r.group, r.name.as_str(), r.title.as_str(), r.checks, r.review))
            .collect();
        assert_eq!(
            shown,
            [
                (
                    Group::Yours,
                    "app#12",
                    "Add the flock",
                    Some(Checks::Failing),
                    Some(Review::Approved)
                ),
                (Group::Yours, "app#3", "Older", None, Some(Review::Changes)),
                (Group::ToReview, "lib#7", "Bump deps", None, None),
                (
                    Group::Branches,
                    "app",
                    "main · Fix landing",
                    Some(Checks::Running),
                    None
                ),
                (Group::Branches, "notes", "main · Fix landing", None, None),
            ]
        );
        assert_eq!(rows[0].item, "pr:me/app#12");
        assert_eq!(rows[0].url.as_deref(), Some("https://github.com/me/app/pull/12"));
        assert_eq!(
            rows[3].url.as_deref(),
            Some("https://github.com/me/app/commit/abc")
        );
        assert_eq!(
            GitHub.board(&Snapshot::new()),
            Some(vec![]),
            "nothing open is an empty card"
        );
    }

    #[test]
    fn the_detail_reads_branch_size_reviewers_checks_and_a_command() {
        let mut a = answer("FAILURE", json!("CHANGES_REQUESTED"), "SUCCESS", "abc", true);
        let pr = &mut a["data"]["viewer"]["pullRequests"]["nodes"][0];
        pr["headRefName"] = json!("fix/refresh");
        pr["additions"] = json!(120);
        pr["deletions"] = json!(40);
        pr["changedFiles"] = json!(6);
        pr["updatedAt"] = json!("2026-10-12T21:50:34Z");
        pr["latestReviews"] =
            json!({ "nodes": [ { "author": { "login": "ana" }, "state": "CHANGES_REQUESTED" } ] });
        // Ana asked again stays one line; a team (no login) is left out.
        pr["reviewRequests"] = json!({ "nodes": [
            { "requestedReviewer": { "login": "ana" } }, { "requestedReviewer": {} },
            { "requestedReviewer": { "login": "core" } } ] });
        pr["commits"]["nodes"][0]["commit"]["statusCheckRollup"]["contexts"] = json!({ "nodes": [
            { "__typename": "CheckRun", "name": "test", "status": "COMPLETED", "conclusion": "SUCCESS" },
            { "__typename": "CheckRun", "name": "visual", "status": "COMPLETED", "conclusion": "FAILURE" },
            { "__typename": "CheckRun", "name": "docs", "status": "COMPLETED", "conclusion": "SKIPPED" },
            { "__typename": "CheckRun", "name": "deploy", "status": "QUEUED", "conclusion": null },
            { "__typename": "StatusContext", "context": "ci/legacy", "state": "PENDING" } ] });
        a["data"]["viewer"]["repositories"]["nodes"][0]["releases"] = json!({ "nodes": [ {
            "tagName": "v0.1.7", "name": "", "url": "https://github.com/me/app/releases/tag/v0.1.7",
            "publishedAt": null, "createdAt": "2026-10-11T04:36:11Z", "isPrerelease": false, "isDraft": true } ] });
        a["data"]["search"]["nodes"][0]["author"] = json!({ "login": "bob" });

        let rows = board(&snapshot(&a));
        let f = &rows[0].facts;
        assert_eq!(f.repo.as_deref(), Some("me/app"));
        assert_eq!(f.branch.as_deref(), Some("fix/refresh"));
        assert_eq!(
            f.diff,
            Some(Diff {
                added: 120,
                removed: 40,
                files: 6
            })
        );
        assert_eq!(f.updated, Some(1_791_841_834));
        let said: Vec<_> = f.reviewers.iter().map(|r| (r.login.as_str(), r.said)).collect();
        assert_eq!(said, [("ana", Said::Changes), ("core", Said::Waiting)]);
        let runs: Vec<_> = f.runs.iter().map(|r| (r.name.as_str(), r.state)).collect();
        assert_eq!(
            runs,
            [
                ("test", RunState::Passed),
                ("visual", RunState::Failed),
                ("docs", RunState::Skipped),
                ("deploy", RunState::Waiting),
                ("ci/legacy", RunState::Running),
            ]
        );
        assert_eq!(f.command.as_deref(), Some("gh pr checks 12 --repo me/app"));

        let review = rows.iter().find(|r| r.group == Group::ToReview).unwrap();
        assert_eq!(review.facts.author.as_deref(), Some("bob"));
        assert_eq!(
            review.facts.command.as_deref(),
            Some("gh pr diff 7 --repo team/lib")
        );
        let branch = rows.iter().find(|r| r.group == Group::Branches).unwrap();
        assert_eq!(
            branch.facts.command.as_deref(),
            Some("gh run list --repo me/app --branch main")
        );
        // A release without a name goes by its tag; a draft dates from its creation.
        let release = rows.iter().find(|r| r.group == Group::Releases).unwrap();
        assert_eq!(
            (release.name.as_str(), release.title.as_str()),
            ("app v0.1.7", "v0.1.7")
        );
        assert_eq!(release.facts.release, Some(Release::Draft));
        assert_eq!(release.facts.updated, Some(1_791_693_371));
        assert_eq!(
            release.facts.command.as_deref(),
            Some("gh release view v0.1.7 --repo me/app")
        );
        assert!(
            diff(&Snapshot::new(), &snapshot(&a))
                .iter()
                .all(|e| !e.key.starts_with("release:"))
        );
    }

    #[test]
    fn a_copied_command_names_nothing_a_shell_reads_as_more() {
        let v = json!({ "branch": "main;rm -rf ~", "tag": "$(boom)" });
        let branch = facts("branch", "me/app", &v, Some(Checks::Failing), None);
        assert_eq!(
            branch.command.as_deref(),
            Some("gh run list --repo me/app --status failure")
        );
        assert_eq!(facts("release", "me/app", &v, None, None).command, None);
        assert_eq!(facts("pr", "me/a b#1", &v, None, None).command, None);
        assert!(plain_ref("release/v1.2.3+build") && !plain_ref("-x") && !plain_ref("a b"));
        // A merge to copy takes only the commit the card showed green.
        let ready = |oid: &str| {
            facts(
                "pr",
                "me/app#9",
                &json!({ "oid": oid }),
                Some(Checks::Passing),
                Some(Review::Approved),
            )
            .command
        };
        assert_eq!(
            ready("abc1234").as_deref(),
            Some("gh pr merge 9 --repo me/app --squash --match-head-commit abc1234")
        );
        assert_eq!(ready("--admin").as_deref(), Some("gh pr view 9 --repo me/app"));
    }

    #[test]
    fn github_dates_read_as_unix_seconds() {
        assert_eq!(unix("1970-01-01T00:00:00Z"), Some(0));
        assert_eq!(unix("2000-03-01T00:00:00Z"), Some(951_868_800));
        assert_eq!(unix("2026-10-12T21:50:34Z"), Some(1_791_841_834));
        for bad in [
            "",
            "2026-10-12",
            "2026-13-01T00:00:00Z",
            "2026-10-12T21:50:34+02:00",
        ] {
            assert_eq!(unix(bad), None, "{bad}");
        }
    }

    #[test]
    fn the_rank_is_no_news() {
        let before = snapshot(&answer("SUCCESS", Value::Null, "SUCCESS", "abc", true));
        let mut after = before.clone();
        for v in after.values_mut() {
            v["rank"] = json!(9);
        }
        assert!(diff(&before, &after).is_empty());
    }

    /// An answer with `errors` beside `data`, written in our query's shape after what `gh api
    /// graphql` really did for a repository that does not exist: exit 1, the whole answer on
    /// stdout, `gh: <first message>` on stderr.
    const PARTIAL: &str = r#"{"data":{"viewer":{"pullRequests":{"nodes":[null,{"number":12,"title":"Add the flock","url":"https://github.com/me/app/pull/12","repository":{"nameWithOwner":"me/app"},"reviewDecision":null,"commits":{"nodes":[{"commit":{"oid":"p1","statusCheckRollup":{"state":"SUCCESS"}}}]}}]},"repositories":{"nodes":[]}},"search":{"nodes":[null]}},"errors":[{"type":"FORBIDDEN","path":["viewer","pullRequests","nodes",0],"message":"Resource protected by organization SAML enforcement."}]}"#;

    #[test]
    fn a_partial_answer_is_rejected_as_incomplete() {
        let error = super::answer(
            false,
            PARTIAL.as_bytes(),
            b"gh: Resource protected by organization SAML enforcement.\n",
        )
        .unwrap_err();
        assert!(
            matches!(error, Error::Partial(message) if message.contains("previous complete snapshot is kept"))
        );
    }

    #[test]
    fn an_answer_without_data_is_an_error() {
        let limited =
            r#"{"data":null,"errors":[{"type":"RATE_LIMITED","message":"API rate limit exceeded"}]}"#;
        assert!(matches!(
            super::answer(
                false,
                limited.as_bytes(),
                b"gh: API rate limit exceeded for user\n"
            ),
            Err(Error::RateLimited { .. })
        ));
        assert!(matches!(
            super::answer(
                false,
                b"",
                b"To get started with GitHub CLI, please run:  gh auth login\n"
            ),
            Err(Error::Auth(_))
        ));
        // A whole list missing (a resolver timeout) keeps the last snapshot.
        let thin = r#"{"data":{"viewer":{"pullRequests":{"nodes":[]},"repositories":null},"search":null},"errors":[{"message":"timeout"}]}"#;
        assert!(matches!(
            super::answer(false, thin.as_bytes(), b"gh: timeout\n"),
            Err(Error::Other(_))
        ));
        assert!(matches!(
            super::answer(true, b"<html>", b""),
            Err(Error::Other(_))
        ));
    }
}

#[cfg(test)]
mod issue_write_tests {
    use super::*;
    use std::collections::VecDeque;
    use std::sync::Mutex;

    struct Step {
        method: &'static str,
        path: String,
        body: Option<Value>,
        host: Option<&'static str>,
        result: Result<Value, Error>,
    }
    struct FakeApi(Mutex<VecDeque<Step>>);
    impl FakeApi {
        fn new(steps: Vec<Step>) -> Self {
            Self(Mutex::new(steps.into()))
        }
        fn exhausted(&self) {
            assert!(self.0.lock().unwrap().is_empty(), "expected request was not sent");
        }
    }
    impl GithubApi for FakeApi {
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
                .expect("unexpected GitHub request or retry");
            assert_eq!(
                (host, method, path, body),
                (step.host, step.method, step.path.as_str(), step.body)
            );
            step.result
        }
    }
    struct Budget(Mutex<usize>);
    impl crate::Gate for Budget {
        fn allow(&self, _: &str) -> bool {
            let mut left = self.0.lock().unwrap();
            if *left == 0 {
                false
            } else {
                *left -= 1;
                true
            }
        }
        fn done(&self, _: &str, _: bool, _: Duration) {}
    }
    fn gate() -> Budget {
        Budget(Mutex::new(100))
    }
    fn issue() -> Value {
        json!({"id":70,"number":7,"title":"Title","body":null,"state":"open","updated_at":"2026-10-23T10:00:00Z","html_url":"https://github.com/org/repo/issues/7","labels":[],"assignees":[]})
    }
    fn inspection(value: Value, initial: bool) -> Vec<Step> {
        vec![
            Step {
                host: if initial { None } else { Some("github.com") },
                method: "GET",
                path: "user".into(),
                body: None,
                result: Ok(json!({"id":1,"login":"person","html_url":"https://github.com/person"})),
            },
            Step {
                host: Some("github.com"),
                method: "GET",
                path: "repos/org/repo".into(),
                body: None,
                result: Ok(
                    json!({"id":20,"full_name":"org/repo","html_url":"https://github.com/org/repo","archived":false,"permissions":{"push":true}}),
                ),
            },
            Step {
                host: Some("github.com"),
                method: "GET",
                path: "repos/org/repo/issues/7".into(),
                body: None,
                result: Ok(value),
            },
        ]
    }
    fn close() -> IssueWrite {
        IssueWrite::Close {
            repository: "org/repo".into(),
            number: 7,
        }
    }
    async fn preview(action: &IssueWrite, value: Value) -> Value {
        let api = FakeApi::new(inspection(value, true));
        let preview = prepare_issue_write_using(&gate(), action, &api).await.unwrap();
        api.exhausted();
        preview
    }
    fn mutation(method: &'static str, path: &str, body: Option<Value>, result: Result<Value, Error>) -> Step {
        Step {
            host: Some("github.com"),
            method,
            path: path.into(),
            body,
            result,
        }
    }

    #[test]
    fn every_issue_operation_uses_its_exact_http_contract() {
        let repo = "org/repo".to_owned();
        let actions = [
            (
                IssueWrite::Create {
                    repository: repo.clone(),
                    title: "T".into(),
                    body: "B".into(),
                },
                (
                    "POST",
                    "repos/org/repo/issues",
                    Some(json!({"title":"T","body":"B"})),
                ),
            ),
            (
                IssueWrite::Edit {
                    repository: repo.clone(),
                    number: 7,
                    title: None,
                    body: Some("".into()),
                },
                ("PATCH", "repos/org/repo/issues/7", Some(json!({"body":""}))),
            ),
            (
                IssueWrite::Comment {
                    repository: repo.clone(),
                    number: 7,
                    body: "C".into(),
                },
                (
                    "POST",
                    "repos/org/repo/issues/7/comments",
                    Some(json!({"body":"C"})),
                ),
            ),
            (
                IssueWrite::EditComment {
                    repository: repo.clone(),
                    number: 7,
                    comment_id: 55,
                    body: "C2".into(),
                },
                (
                    "PATCH",
                    "repos/org/repo/issues/comments/55",
                    Some(json!({"body":"C2"})),
                ),
            ),
            (
                IssueWrite::AddLabels {
                    repository: repo.clone(),
                    number: 7,
                    labels: vec!["bug".into()],
                },
                (
                    "POST",
                    "repos/org/repo/issues/7/labels",
                    Some(json!({"labels":["bug"]})),
                ),
            ),
            (
                IssueWrite::RemoveLabels {
                    repository: repo.clone(),
                    number: 7,
                    labels: vec!["area/ui".into()],
                },
                ("DELETE", "repos/org/repo/issues/7/labels/area%2Fui", None),
            ),
            (
                IssueWrite::SetAssignees {
                    repository: repo.clone(),
                    number: 7,
                    assignees: vec![],
                },
                ("PATCH", "repos/org/repo/issues/7", Some(json!({"assignees":[]}))),
            ),
            (
                close(),
                (
                    "PATCH",
                    "repos/org/repo/issues/7",
                    Some(json!({"state":"closed"})),
                ),
            ),
            (
                IssueWrite::Reopen {
                    repository: repo,
                    number: 7,
                },
                ("PATCH", "repos/org/repo/issues/7", Some(json!({"state":"open"}))),
            ),
        ];
        for (action, (method, path, body)) in actions {
            let requests = issue_write_requests(&action);
            assert_eq!(requests, vec![(method, path.to_owned(), body)], "{action:?}");
        }
    }

    #[tokio::test]
    async fn every_issue_write_verifies_the_requested_remote_state() {
        let repo = "org/repo".to_owned();
        let actions = [
            IssueWrite::Create {
                repository: repo.clone(),
                title: "New".into(),
                body: "".into(),
            },
            IssueWrite::Edit {
                repository: repo.clone(),
                number: 7,
                title: None,
                body: Some("New".into()),
            },
            IssueWrite::Comment {
                repository: repo.clone(),
                number: 7,
                body: "New".into(),
            },
            IssueWrite::AddLabels {
                repository: repo.clone(),
                number: 7,
                labels: vec!["bug".into()],
            },
            IssueWrite::RemoveLabels {
                repository: repo.clone(),
                number: 7,
                labels: vec!["bug".into()],
            },
            IssueWrite::SetAssignees {
                repository: repo.clone(),
                number: 7,
                assignees: vec!["Alice".into()],
            },
            IssueWrite::SetAssignees {
                repository: repo.clone(),
                number: 7,
                assignees: vec![],
            },
            close(),
            IssueWrite::Reopen {
                repository: repo,
                number: 7,
            },
        ];
        for action in actions {
            let mut before = issue();
            let mut after = issue();
            match &action {
                IssueWrite::Edit { .. } => after["body"] = json!("New"),
                IssueWrite::AddLabels { .. } => after["labels"] = json!([{"name":"bug"}]),
                IssueWrite::RemoveLabels { .. } => before["labels"] = json!([{"name":"bug"}]),
                IssueWrite::SetAssignees { assignees, .. } => {
                    before["assignees"] = json!([{"login":"Other"}]);
                    after["assignees"] = json!(
                        assignees
                            .iter()
                            .map(|name| json!({"login":name.to_lowercase()}))
                            .collect::<Vec<_>>()
                    );
                }
                IssueWrite::Close { .. } => after["state"] = json!("closed"),
                IssueWrite::Reopen { .. } => before["state"] = json!("closed"),
                _ => {}
            }
            let reads = |value: Value, initial: bool, complete: bool| {
                let mut steps = inspection(value, initial);
                match &action {
                    IssueWrite::Create { .. } => {
                        let mut created = issue();
                        created["title"] = json!("New");
                        steps[2].path =
                            "repos/org/repo/issues?state=all&per_page=100&sort=created&direction=asc&page=1"
                                .into();
                        steps[2].result = Ok(if complete { json!([created]) } else { json!([]) });
                    }
                    IssueWrite::Comment { .. } => steps.push(mutation(
                        "GET",
                        "repos/org/repo/issues/7/comments?per_page=100&page=1",
                        None,
                        Ok(if complete {
                            json!([{"id":55,"body":"New"}])
                        } else {
                            json!([])
                        }),
                    )),
                    _ => {}
                }
                steps
            };
            let api = FakeApi::new(reads(before.clone(), true, false));
            let expected = prepare_issue_write_using(&gate(), &action, &api).await.unwrap();
            api.exhausted();
            let mut steps = reads(before, false, false);
            steps.extend(
                issue_write_requests(&action)
                    .into_iter()
                    .map(|(method, path, body)| mutation(method, &path, body, Ok(json!({})))),
            );
            steps.extend(reads(after, false, true));
            let api = FakeApi::new(steps);
            let result = execute_issue_write_using(&gate(), &action, &expected, &action.request_hash(), &api)
                .await
                .unwrap();
            assert_eq!(result["status"], "verified", "{action:?}");
            assert!(!result["issue"].is_null());
            api.exhausted();
        }
    }

    #[tokio::test]
    async fn stale_identity_target_permissions_or_content_send_no_mutation() {
        let action = close();
        let expected = preview(&action, issue()).await;
        for changed in [
            "account",
            "repository",
            "target",
            "permission",
            "archived",
            "malformed",
        ] {
            let mut steps = inspection(issue(), false);
            match changed {
                "account" => steps[0].result.as_mut().unwrap()["id"] = json!(2),
                "repository" => steps[1].result.as_mut().unwrap()["id"] = json!(21),
                "target" => steps[2].result.as_mut().unwrap()["updated_at"] = json!("later"),
                "permission" => {
                    steps[1].result.as_mut().unwrap()["permissions"]["push"] = json!(false);
                    steps.truncate(2);
                }
                "archived" => {
                    steps[1].result.as_mut().unwrap()["archived"] = json!(true);
                    steps.truncate(2);
                }
                _ => {
                    steps[2].result.as_mut().unwrap()["labels"] = json!([{}]);
                }
            }
            let api = FakeApi::new(steps);
            assert!(
                execute_issue_write_using(&gate(), &action, &expected, &action.request_hash(), &api)
                    .await
                    .is_err(),
                "{changed}"
            );
            api.exhausted();
        }
        let api = FakeApi::new(vec![]);
        assert!(
            execute_issue_write_using(&gate(), &action, &expected, "edited-after-approval", &api)
                .await
                .is_err()
        );
        api.exhausted();
    }

    #[tokio::test]
    async fn successful_or_timed_out_writes_are_verified_without_repetition() {
        let action = close();
        let expected = preview(&action, issue()).await;
        for (failure, completed) in [(false, true), (true, true), (true, false)] {
            let mut steps = inspection(issue(), false);
            steps.push(mutation(
                "PATCH",
                "repos/org/repo/issues/7",
                Some(json!({"state":"closed"})),
                if failure {
                    Err(Error::Other("GitHub took too long to answer.".into()))
                } else {
                    Ok(json!({}))
                },
            ));
            let mut after = issue();
            if completed {
                after["state"] = json!("closed");
            }
            steps.extend(inspection(after, false));
            let api = FakeApi::new(steps);
            let result =
                execute_issue_write_using(&gate(), &action, &expected, &action.request_hash(), &api).await;
            if completed {
                assert_eq!(
                    result.unwrap()["status"],
                    if failure {
                        "verified_after_error"
                    } else {
                        "verified"
                    }
                );
            } else {
                assert!(result.unwrap_err().to_string().contains("could not be verified"));
            }
            api.exhausted();
        }
    }

    #[tokio::test]
    async fn partial_label_removal_and_failed_verification_are_uncertain_without_retry() {
        let action = IssueWrite::RemoveLabels {
            repository: "org/repo".into(),
            number: 7,
            labels: vec!["one".into(), "two".into()],
        };
        let mut initial = issue();
        initial["labels"] = json!([{"name":"one"},{"name":"two"}]);
        let expected = preview(&action, initial.clone()).await;
        let mut steps = inspection(initial, false);
        steps.push(mutation(
            "DELETE",
            "repos/org/repo/issues/7/labels/one",
            None,
            Ok(json!([])),
        ));
        steps.push(mutation(
            "DELETE",
            "repos/org/repo/issues/7/labels/two",
            None,
            Err(Error::Other("network lost".into())),
        ));
        let mut after = issue();
        after["labels"] = json!([{"name":"two"}]);
        steps.extend(inspection(after, false));
        let api = FakeApi::new(steps);
        assert!(
            execute_issue_write_using(&gate(), &action, &expected, &action.request_hash(), &api)
                .await
                .is_err()
        );
        api.exhausted();
        let action = close();
        let expected = preview(&action, issue()).await;
        let mut steps = inspection(issue(), false);
        steps.push(mutation(
            "PATCH",
            "repos/org/repo/issues/7",
            Some(json!({"state":"closed"})),
            Ok(json!({})),
        ));
        steps.push(mutation("GET", "user", None, Err(Error::Other("offline".into()))));
        let api = FakeApi::new(steps);
        assert!(
            execute_issue_write_using(&gate(), &action, &expected, &action.request_hash(), &api)
                .await
                .unwrap_err()
                .to_string()
                .contains("uncertain")
        );
        api.exhausted();
    }

    #[tokio::test]
    async fn duplicate_empty_body_and_unordered_assignees_are_not_written() {
        let action = IssueWrite::Edit {
            repository: "org/repo".into(),
            number: 7,
            title: None,
            body: Some("".into()),
        };
        let api = FakeApi::new(inspection(issue(), true));
        assert!(
            prepare_issue_write_using(&gate(), &action, &api)
                .await
                .unwrap_err()
                .to_string()
                .contains("already present")
        );
        api.exhausted();
        let action = IssueWrite::SetAssignees {
            repository: "org/repo".into(),
            number: 7,
            assignees: vec!["Bob".into(), "alice".into()],
        };
        let mut value = issue();
        value["assignees"] = json!([{"login":"Alice"},{"login":"bob"}]);
        let api = FakeApi::new(inspection(value, true));
        assert!(
            prepare_issue_write_using(&gate(), &action, &api)
                .await
                .unwrap_err()
                .to_string()
                .contains("already present")
        );
        api.exhausted();
    }

    #[tokio::test]
    async fn editing_a_comment_binds_its_issue_and_rejects_a_changed_body() {
        let action = IssueWrite::EditComment {
            repository: "org/repo".into(),
            number: 7,
            comment_id: 55,
            body: "replacement".into(),
        };
        let comment = json!({"id":55,"body":"original","updated_at":"today","issue_url":"https://api.github.com/repos/org/repo/issues/7"});
        let mut steps = inspection(issue(), true);
        steps.push(mutation(
            "GET",
            "repos/org/repo/issues/comments/55",
            None,
            Ok(comment.clone()),
        ));
        let api = FakeApi::new(steps);
        let expected = prepare_issue_write_using(&gate(), &action, &api).await.unwrap();
        api.exhausted();
        for (field, value) in [
            ("body", json!("someone edited it")),
            (
                "issue_url",
                json!("https://api.github.com/repos/org/repo/issues/8"),
            ),
        ] {
            let mut changed = comment.clone();
            changed[field] = value;
            let mut steps = inspection(issue(), false);
            steps.push(mutation(
                "GET",
                "repos/org/repo/issues/comments/55",
                None,
                Ok(changed),
            ));
            let api = FakeApi::new(steps);
            assert!(
                execute_issue_write_using(&gate(), &action, &expected, &action.request_hash(), &api)
                    .await
                    .is_err()
            );
            api.exhausted();
        }
        let mut steps = inspection(issue(), false);
        steps.push(mutation(
            "GET",
            "repos/org/repo/issues/comments/55",
            None,
            Ok(comment.clone()),
        ));
        steps.push(mutation(
            "PATCH",
            "repos/org/repo/issues/comments/55",
            Some(json!({"body":"replacement"})),
            Ok(json!({})),
        ));
        let mut changed = comment;
        changed["body"] = json!("replacement");
        changed["updated_at"] = json!("later");
        steps.extend(inspection(issue(), false));
        steps.push(mutation(
            "GET",
            "repos/org/repo/issues/comments/55",
            None,
            Ok(changed),
        ));
        let api = FakeApi::new(steps);
        assert_eq!(
            execute_issue_write_using(&gate(), &action, &expected, &action.request_hash(), &api)
                .await
                .unwrap()["status"],
            "verified"
        );
        api.exhausted();
    }

    #[tokio::test]
    async fn job_logs_only_follow_a_job_in_the_exact_run_attempt() {
        let job = json!({"id":55,"run_id":12,"html_url":"https://github.com/org/repo/actions/runs/12"});
        let path = "repos/org/repo/actions/runs/12/attempts/2/jobs?per_page=100&page=1";
        for found in [true, false] {
            let mut changed = job.clone();
            if !found {
                changed["id"] = json!(56);
            }
            let api = FakeApi::new(vec![Step {
                host: None,
                method: "GET",
                path: path.into(),
                body: None,
                result: Ok(json!({"total_count":1,"jobs":[changed]})),
            }]);
            assert_eq!(
                find_job_for_log(&gate(), &api, "org/repo", 12, 55, 2)
                    .await
                    .is_ok(),
                found
            );
            api.exhausted();
        }
    }

    #[tokio::test]
    async fn revoked_budget_never_calls_the_transport() {
        let api = FakeApi::new(vec![]);
        assert!(
            prepare_issue_write_using(&Budget(Mutex::new(0)), &close(), &api)
                .await
                .is_err()
        );
        api.exhausted();
        let expected = preview(&close(), issue()).await;
        let api = FakeApi::new(inspection(issue(), false));
        assert!(
            execute_issue_write_using(
                &Budget(Mutex::new(3)),
                &close(),
                &expected,
                &close().request_hash(),
                &api
            )
            .await
            .is_err()
        );
        api.exhausted();
    }

    #[tokio::test]
    async fn duplicate_scans_follow_pages_and_stop_at_the_explicit_limit() {
        let page = json!(
            (1..=100)
                .map(|id| json!({"id":id,"body":"other"}))
                .collect::<Vec<_>>()
        );
        let base = "repos/org/repo/issues/7/comments?per_page=100";
        let api = FakeApi::new(vec![
            mutation("GET", &format!("{base}&page=1"), None, Ok(page.clone())),
            mutation("GET", &format!("{base}&page=2"), None, Ok(json!([]))),
        ]);
        assert_eq!(
            issue_collection(&gate(), &api, "github.com", base)
                .await
                .unwrap()
                .len(),
            100
        );
        api.exhausted();
        let api = FakeApi::new(
            (1..=10)
                .map(|p| mutation("GET", &format!("{base}&page={p}"), None, Ok(page.clone())))
                .collect(),
        );
        assert!(
            issue_collection(&gate(), &api, "github.com", base)
                .await
                .unwrap_err()
                .to_string()
                .contains("1,000-item")
        );
        api.exhausted();
    }
}

#[cfg(test)]
mod actions_log_tests {
    use super::*;
    #[test]
    fn log_redaction_and_byte_coverage_survive_the_return_envelope() {
        let output = (
            true,
            b"export GITHUB_TOKEN=ghp_secret\nTOKEN=abc123 deploy\n".to_vec(),
            vec![],
            false,
        );
        let result = job_log_answer(
            "org/repo",
            12,
            55,
            1,
            json!("https://github.com/org/repo/actions/runs/12"),
            output,
        )
        .unwrap();
        assert!(!result["log"].as_str().unwrap().contains("ghp_secret"));
        assert!(!result["log"].as_str().unwrap().contains("abc123"));
        assert_eq!(result["truncated"], false);
        let result = job_log_answer(
            "org/repo",
            12,
            55,
            1,
            Value::Null,
            (true, "word ".repeat(14000).into_bytes(), vec![], false),
        )
        .unwrap();
        assert_eq!(result["truncated"], true);
        assert!(result["log"].as_str().unwrap().len() <= 48 * 1024);
        assert!(
            job_log_answer(
                "org/repo",
                12,
                55,
                1,
                Value::Null,
                (false, vec![], b"gh: logs unavailable (HTTP 404)".to_vec(), false)
            )
            .is_err()
        );
    }
}
