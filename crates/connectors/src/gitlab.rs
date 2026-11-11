//! Read-only GitLab.com snapshots through the user's authenticated CLI.
use crate::{
    Checks, Connector, Diff, Error, Event, Facts, Group, Level, Poll, Release, Review, Reviewer, Row, Run,
    RunState, Said, Snapshot, Story, StoryKind,
};
use serde_json::{Value, json};
use std::{process::Stdio, time::Duration};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    process::Command,
};

#[derive(Debug)]
pub struct GitLab;
const LIMIT: usize = 50;
const MAX_BYTES: usize = 1024 * 1024;
const INVALID: &str = "GitLab returned incomplete or invalid data. The previous snapshot is kept.";
const AUTH: &str = "Sign in to GitLab.com with glab auth login --hostname gitlab.com.";
const FAILED: &str = "Could not read GitLab. Check glab and try again.";
const QUERY: &str = r#"query VultsReadOnly {
  currentUser {
    id username
    contributedProjects(first: 10, sort: LATEST_ACTIVITY_DESC) {
      nodes { fullPath archived
        repository { rootRef lastCommit { sha title committedDate
          latestPipeline { ...PipelineDetails } } }
        releases(first: 1, sort: RELEASED_AT_DESC) { nodes { name tagName releasedAt upcomingRelease } }
      }
    }
    authoredMergeRequests(first: 50, state: opened, sort: UPDATED_DESC) {
      pageInfo { hasNextPage }
      nodes { ...Request }
    }
    reviewRequestedMergeRequests(first: 50, state: opened, sort: UPDATED_DESC) {
      pageInfo { hasNextPage }
      nodes { ...Request }
    }
  }
}
fragment Request on MergeRequest {
  iid title webUrl sourceBranch diffHeadSha approved approvalsRequired updatedAt
  diffStatsSummary { additions deletions fileCount }
  reviewers(first: 20) {
    pageInfo { hasNextPage }
    nodes { username mergeRequestInteraction { approved reviewState } }
  }
  approvedBy(first: 20) { pageInfo { hasNextPage } nodes { username } }
  author { username }
  project { fullPath }
  headPipeline { ...PipelineDetails }
}
fragment PipelineDetails on Pipeline {
  status sha
  jobs(first: 30, retried: false) {
    pageInfo { hasNextPage }
    nodes { name status }
  }
}"#;

impl Connector for GitLab {
    fn id(&self) -> &'static str {
        "gitlab"
    }
    fn interval(&self, last: &Snapshot) -> Duration {
        Duration::from_secs(if last.values().any(|row| row["ci"] == "running") {
            60
        } else {
            300
        })
    }
    fn poll(&self) -> Poll<'_> {
        Box::pin(poll())
    }
    fn diff(&self, before: &Snapshot, after: &Snapshot) -> Vec<Event> {
        diff(before, after)
    }
    fn board(&self, snapshot: &Snapshot) -> Option<Vec<Row>> {
        Some(board(snapshot))
    }
    fn identity(&self, snapshot: &Snapshot) -> Option<(String, String)> {
        Some((
            snapshot.get("meta:account")?["login"].as_str()?.into(),
            "gitlab.com".into(),
        ))
    }
    fn watching(&self, snapshot: &Snapshot) -> usize {
        snapshot
            .iter()
            .filter_map(|(key, row)| {
                let number = row["number"].as_u64();
                Some((
                    row["repo"].as_str()?,
                    number,
                    if number.is_some() {
                        "mr"
                    } else {
                        key.split_once(':')?.0
                    },
                ))
            })
            .collect::<std::collections::BTreeSet<_>>()
            .len()
    }
}

async fn poll() -> Result<Snapshot, Error> {
    let mut command = Command::new("glab");
    configure(&mut command);
    let result = tokio::time::timeout(Duration::from_secs(30), read(&mut command))
        .await
        .map_err(|_| Error::Other("GitLab did not respond in time.".into()))??;
    answer(result.0, &result.1, &result.2)
}
fn configure(command: &mut Command) {
    command
        .args([
            "api",
            "graphql",
            "--hostname",
            "gitlab.com",
            "--method",
            "POST",
            "--input",
            "-",
        ])
        .env("GITLAB_HOST", "gitlab.com")
        .env("GITLAB_API_HOST", "gitlab.com")
        .env("GLAB_API_PROTOCOL", "https")
        .env("GLAB_SKIP_TLS_VERIFY", "false")
        .env("GLAB_ENABLE_CI_AUTOLOGIN", "false")
        .env("GLAB_CHECK_UPDATE", "false")
        .env("GLAB_COLOR", "never")
        .env("GLAB_DEBUG", "false")
        .env("GLAB_DEBUG_HTTP", "false")
        .env("GITLAB_SUBFOLDER", "/")
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("NO_COLOR", "1")
        .env_remove("GITLAB_REPO")
        .current_dir(std::env::temp_dir())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    #[cfg(windows)]
    command.creation_flags(0x0800_0000);
}
async fn read(command: &mut Command) -> Result<(bool, Vec<u8>, Vec<u8>), Error> {
    let mut child = command.spawn().map_err(|error| match error.kind() {
        std::io::ErrorKind::NotFound => Error::Unavailable("The GitLab CLI (glab) is not installed.".into()),
        _ => Error::Other(FAILED.into()),
    })?;
    let mut input = child.stdin.take().ok_or_else(|| Error::Other(FAILED.into()))?;
    input
        .write_all(&serde_json::to_vec(&json!({"query":QUERY})).map_err(|_| Error::Other(FAILED.into()))?)
        .await
        .map_err(|_| Error::Other(FAILED.into()))?;
    drop(input);
    let stdout = child.stdout.take().ok_or_else(|| Error::Other(FAILED.into()))?;
    let stderr = child.stderr.take().ok_or_else(|| Error::Other(FAILED.into()))?;
    async fn bounded(stream: impl tokio::io::AsyncRead + Unpin, limit: usize) -> Result<Vec<u8>, Error> {
        let mut bytes = Vec::new();
        stream
            .take((limit + 1) as u64)
            .read_to_end(&mut bytes)
            .await
            .map_err(|_| Error::Other(FAILED.into()))?;
        if bytes.len() > limit {
            return Err(Error::Partial(INVALID.into()));
        }
        Ok(bytes)
    }
    let capture = async { tokio::try_join!(bounded(stdout, MAX_BYTES), bounded(stderr, 16384)) };
    let ((out, err), status) = tokio::try_join!(capture, async {
        child.wait().await.map_err(|_| Error::Other(FAILED.into()))
    })?;
    Ok((status.success(), out, err))
}
fn answer(success: bool, stdout: &[u8], stderr: &[u8]) -> Result<Snapshot, Error> {
    if !success {
        // Classify locally; CLI diagnostics can contain account data and must never reach a view.
        let error = String::from_utf8_lossy(stderr).to_ascii_lowercase();
        return Err(if error.contains("429") || error.contains("rate limit") {
            Error::RateLimited {
                retry_after: Duration::from_secs(900),
            }
        } else if [
            "401",
            "403",
            "auth login",
            "not logged",
            "unauthorized",
            "forbidden",
        ]
        .iter()
        .any(|key| error.contains(key))
        {
            Error::Auth(AUTH.into())
        } else {
            Error::Other(FAILED.into())
        });
    }
    if stdout.len() > MAX_BYTES {
        return Err(Error::Partial(INVALID.into()));
    }
    let response: Value = serde_json::from_slice(stdout).map_err(|_| Error::Partial(INVALID.into()))?;
    if response
        .get("errors")
        .is_some_and(|errors| errors.as_array().is_none_or(|errors| !errors.is_empty()))
    {
        return Err(Error::Partial(INVALID.into()));
    }
    if !response["data"].is_object() || response["data"].get("currentUser").is_none() {
        return Err(Error::Partial(INVALID.into()));
    }
    let user = &response["data"]["currentUser"];
    if user.is_null() {
        return Err(Error::Auth(AUTH.into()));
    }
    let login = text(&user["username"], 255)
        .filter(|value| {
            value
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
        })
        .ok_or_else(|| Error::Partial(INVALID.into()))?;
    let id = user["id"]
        .as_str()
        .and_then(|s| s.strip_prefix("gid://gitlab/User/"))
        .and_then(|s| s.parse::<u64>().ok())
        .filter(|id| *id > 0 && *id <= i64::MAX as u64)
        .ok_or_else(invalid)?;
    let namespace = format!("{login}@{id}");
    let mut snapshot = Snapshot::from([("meta:account".into(), json!({"login":login,"id":id,"schema":3}))]);
    for (field, prefix) in [
        ("authoredMergeRequests", "mr"),
        ("reviewRequestedMergeRequests", "review"),
    ] {
        let group = &user[field];
        match group["pageInfo"]["hasNextPage"].as_bool() {
            Some(false) => {}
            Some(true) => {
                return Err(Error::Partial(
                    "GitLab returned more than 50 items in a group. The previous snapshot is kept.".into(),
                ));
            }
            None => return Err(Error::Partial(INVALID.into())),
        }
        let rows = group["nodes"]
            .as_array()
            .filter(|rows| rows.len() <= LIMIT)
            .ok_or_else(|| Error::Partial(INVALID.into()))?;
        for (rank, row) in rows.iter().enumerate() {
            let repo = text(&row["project"]["fullPath"], 1024)
                .filter(|repo| project(repo))
                .ok_or_else(|| Error::Partial(INVALID.into()))?;
            let number = row["iid"]
                .as_str()
                .and_then(|id| id.parse::<u32>().ok())
                .filter(|id| *id > 0)
                .ok_or_else(|| Error::Partial(INVALID.into()))?;
            let title = text(&row["title"], 4096).ok_or_else(|| Error::Partial(INVALID.into()))?;
            let url = format!("https://gitlab.com/{repo}/-/merge_requests/{number}");
            if row["webUrl"] != url {
                return Err(Error::Partial(INVALID.into()));
            }
            let head = row["diffHeadSha"].as_str().filter(|sha| commit(sha));
            let pipeline = &row["headPipeline"];
            let ci = if head.is_some() && pipeline["sha"].as_str() == head {
                pipeline_state(pipeline["status"].as_str())
            } else {
                None
            };
            let approved = row["approved"]
                .as_bool()
                .ok_or_else(|| Error::Partial(INVALID.into()))?;
            let required = match row.get("approvalsRequired") {
                Some(Value::Null) => None,
                Some(value) => Some(value.as_u64().ok_or_else(|| Error::Partial(INVALID.into()))?),
                None => return Err(Error::Partial(INVALID.into())),
            };
            let approved = approved && required.is_some_and(|count| count > 0);
            let reviewers = reviewers(&row["reviewers"], &row["approvedBy"])?;
            let changes = reviewers.iter().any(|r| r["said"] == "changes");
            let stats = &row["diffStatsSummary"];
            let size = if stats.is_null() {
                Value::Null
            } else {
                json!({"added":count(&stats["additions"])?,"removed":count(&stats["deletions"])?,"files":count(&stats["fileCount"]) ?})
            };
            let runs = if head.is_some() && pipeline["sha"].as_str() == head {
                runs(pipeline)?
            } else {
                Vec::new()
            };
            let item = json!({"repo":repo,"number":number,"title":title,"url":url,"branch":text(&row["sourceBranch"],1024),
                "author":text(&row["author"]["username"],255),"oid":head,"ci":ci,"approved":approved,"changes":changes,"rank":rank,
                "size":size,"reviewers":reviewers,"runs":runs,"updated":row["updatedAt"]});
            if snapshot
                .insert(format!("{prefix}:{namespace}:{repo}!{number}:item"), item)
                .is_some()
            {
                return Err(Error::Partial(INVALID.into()));
            }
        }
    }
    projects(&mut snapshot, &user["contributedProjects"], &namespace)?;
    Ok(snapshot)
}
fn invalid() -> Error {
    Error::Partial(INVALID.into())
}
fn count(value: &Value) -> Result<u32, Error> {
    value.as_u64().and_then(|n| n.try_into().ok()).ok_or_else(invalid)
}
fn connection(value: &Value, limit: usize) -> Result<&Vec<Value>, Error> {
    if value["pageInfo"]["hasNextPage"] != false {
        return Err(invalid());
    }
    value["nodes"]
        .as_array()
        .filter(|rows| rows.len() <= limit)
        .ok_or_else(invalid)
}
fn reviewers(value: &Value, approvals: &Value) -> Result<Vec<Value>, Error> {
    let mut seen = std::collections::BTreeSet::new();
    let mut result = connection(value, 20)?
        .iter()
        .map(|v| {
            let login = text(&v["username"], 255).ok_or_else(invalid)?;
            if !seen.insert(login) {
                return Err(invalid());
            }
            let interaction = &v["mergeRequestInteraction"];
            let approved = interaction["approved"].as_bool().ok_or_else(invalid)?;
            let said = match interaction["reviewState"].as_str() {
                Some("REQUESTED_CHANGES") => "changes",
                Some("APPROVED") if approved => "approved",
                None if approved => "approved",
                None => "waiting",
                Some("UNREVIEWED" | "REVIEW_STARTED" | "UNAPPROVED") if !approved => "waiting",
                Some("REVIEWED") if !approved => "commented",
                _ => return Err(invalid()),
            };
            Ok(json!({"login":login,"said":said}))
        })
        .collect::<Result<Vec<_>, Error>>()?;
    let mut approved_seen = std::collections::BTreeSet::new();
    for user in connection(approvals, 20)? {
        let login = text(&user["username"], 255).ok_or_else(invalid)?;
        if !approved_seen.insert(login) {
            return Err(invalid());
        }
        if let Some(existing) = result.iter_mut().find(|r| r["login"] == login) {
            if existing["said"] != "changes" {
                existing["said"] = json!("approved");
            }
        } else {
            result.push(json!({"login":login,"said":"approved"}));
        }
    }
    Ok(result)
}
fn runs(pipeline: &Value) -> Result<Vec<Value>, Error> {
    if pipeline.is_null() {
        return Ok(Vec::new());
    }
    connection(&pipeline["jobs"], 30)?
        .iter()
        .map(|v| {
            let name = text(&v["name"], 1024).ok_or_else(invalid)?;
            let state = match v["status"].as_str() {
                Some("SUCCESS") => "passed",
                Some("FAILED") => "failed",
                Some("RUNNING" | "PREPARING" | "CANCELING") => "running",
                Some(
                    "CREATED"
                    | "PENDING"
                    | "WAITING_FOR_RESOURCE"
                    | "WAITING_FOR_CALLBACK"
                    | "MANUAL"
                    | "SCHEDULED",
                ) => "waiting",
                Some("SKIPPED" | "CANCELED") => "skipped",
                _ => return Err(invalid()),
            };
            Ok(json!({"name":name,"state":state}))
        })
        .collect()
}
fn projects(snapshot: &mut Snapshot, value: &Value, login: &str) -> Result<(), Error> {
    let nodes = value["nodes"]
        .as_array()
        .filter(|v| v.len() <= 10)
        .ok_or_else(invalid)?;
    let mut seen = std::collections::BTreeSet::new();
    for (rank, p) in nodes.iter().enumerate() {
        let repo = text(&p["fullPath"], 1024)
            .filter(|p| project(p))
            .ok_or_else(invalid)?;
        if !seen.insert(repo) {
            return Err(invalid());
        }
        if p["archived"].as_bool().ok_or_else(invalid)? {
            continue;
        }
        let repository = &p["repository"];
        if !repository.is_null() && !repository.is_object() {
            return Err(invalid());
        }
        if let Some(branch) = repository["rootRef"].as_str() {
            let c = &repository["lastCommit"];
            let sha = c["sha"].as_str().filter(|s| commit(s)).ok_or_else(invalid)?;
            let pipeline = &c["latestPipeline"];
            if pipeline["sha"] == sha {
                let ci = pipeline_state(pipeline["status"].as_str());
                snapshot.insert(format!("branch:{login}:{repo}:item"),json!({"repo":repo,"branch":branch,"oid":sha,
                    "title":text(&c["title"],4096).ok_or_else(invalid)?,"url":format!("https://gitlab.com/{repo}/-/commit/{sha}"),
                    "ci":ci,"runs":runs(pipeline)?,"updated":c["committedDate"],"rank":rank}));
            }
        }
        let releases = p["releases"]["nodes"]
            .as_array()
            .filter(|v| v.len() <= 1)
            .ok_or_else(invalid)?;
        if let Some(r) = releases.first() {
            let tag = text(&r["tagName"], 255).ok_or_else(invalid)?;
            let upcoming = r["upcomingRelease"].as_bool().ok_or_else(invalid)?;
            let encoded: String = tag
                .bytes()
                .map(|b| {
                    if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
                        char::from(b).to_string()
                    } else {
                        format!("%{b:02X}")
                    }
                })
                .collect();
            snapshot.insert(format!("release:{login}:{repo}:item"),json!({"repo":repo,"tag":tag,"title":text(&r["name"],4096).unwrap_or(tag),
                "url":format!("https://gitlab.com/{repo}/-/releases/{encoded}"),"upcoming":upcoming,"updated":r["releasedAt"],"rank":rank}));
        }
    }
    Ok(())
}
fn text(value: &Value, limit: usize) -> Option<&str> {
    value
        .as_str()
        .filter(|s| !s.is_empty() && s.len() <= limit && !s.chars().any(char::is_control))
}
fn project(value: &str) -> bool {
    value.contains('/')
        && value.split('/').all(|part| {
            !part.is_empty()
                && part != "."
                && part != ".."
                && part
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
        })
}
fn commit(value: &str) -> bool {
    matches!(value.len(), 40 | 64) && value.bytes().all(|c| c.is_ascii_hexdigit())
}
fn pipeline_state(state: Option<&str>) -> Option<&'static str> {
    match state {
        Some("SUCCESS") => Some("passing"),
        Some("FAILED") => Some("failing"),
        Some(
            "CREATED" | "WAITING_FOR_RESOURCE" | "PREPARING" | "WAITING_FOR_CALLBACK" | "PENDING" | "RUNNING",
        ) => Some("running"),
        _ => None,
    }
}
fn board(snapshot: &Snapshot) -> Vec<Row> {
    let mut rows: Vec<_> = snapshot
        .iter()
        .filter_map(|(key, item)| {
            let (kind, _) = key.split_once(':')?;
            let short = item["repo"].as_str()?.rsplit('/').next()?;
            let name = match kind {
                "mr" | "review" => format!("{short}!{}", item["number"].as_u64()?),
                _ => short.into(),
            };
            let group = match kind {
                "mr" => Group::Yours,
                "review" => Group::ToReview,
                "branch" => Group::Branches,
                "release" => Group::Releases,
                _ => return None,
            };
            Some((
                group,
                item["rank"].as_u64().unwrap_or(u64::MAX),
                Row {
                    item: key.clone(),
                    group,
                    name,
                    title: item["title"].as_str()?.into(),
                    checks: match item["ci"].as_str() {
                        Some("passing") => Some(Checks::Passing),
                        Some("failing") => Some(Checks::Failing),
                        Some("running") => Some(Checks::Running),
                        _ => None,
                    },
                    review: if item["changes"] == true {
                        Some(Review::Changes)
                    } else {
                        (kind == "mr" && item["approved"] == true).then_some(Review::Approved)
                    },
                    url: item["url"].as_str().map(str::to_owned),
                    // This connector reads only. A passing pipeline never offers a merge action.
                    head: None,
                    facts: Facts {
                        repo: item["repo"].as_str().map(str::to_owned),
                        branch: item["branch"].as_str().map(str::to_owned),
                        author: item["author"].as_str().map(str::to_owned),
                        diff: item["size"].is_object().then(|| Diff {
                            added: item["size"]["added"].as_u64().unwrap_or(0) as u32,
                            removed: item["size"]["removed"].as_u64().unwrap_or(0) as u32,
                            files: item["size"]["files"].as_u64().unwrap_or(0) as u32,
                        }),
                        updated: item["updated"].as_str().and_then(crate::timestamp::parse),
                        reviewers: item["reviewers"]
                            .as_array()
                            .into_iter()
                            .flatten()
                            .filter_map(|r| {
                                Some(Reviewer {
                                    login: r["login"].as_str()?.into(),
                                    said: match r["said"].as_str()? {
                                        "approved" => Said::Approved,
                                        "changes" => Said::Changes,
                                        "commented" => Said::Commented,
                                        "waiting" => Said::Waiting,
                                        _ => return None,
                                    },
                                })
                            })
                            .collect(),
                        runs: item["runs"]
                            .as_array()
                            .into_iter()
                            .flatten()
                            .filter_map(|r| {
                                Some(Run {
                                    name: r["name"].as_str()?.into(),
                                    state: match r["state"].as_str()? {
                                        "passed" => RunState::Passed,
                                        "failed" => RunState::Failed,
                                        "running" => RunState::Running,
                                        "waiting" => RunState::Waiting,
                                        "skipped" => RunState::Skipped,
                                        _ => return None,
                                    },
                                })
                            })
                            .collect(),
                        release: (kind == "release" && item["upcoming"] == false).then_some(Release::Latest),
                        ..Facts::default()
                    },
                },
            ))
        })
        .collect();
    rows.sort_by_key(|(group, rank, _)| (*group, *rank));
    rows.into_iter().map(|(_, _, row)| row).collect()
}
fn diff(before: &Snapshot, after: &Snapshot) -> Vec<Event> {
    if before
        .get("meta:account")
        .is_some_and(|account| Some(account) != after.get("meta:account"))
    {
        return Vec::new();
    }
    let mut events = Vec::new();
    for (key, item) in after {
        let Some((kind, _)) = key.split_once(':') else {
            continue;
        };
        let name = if kind == "branch" {
            item["repo"].as_str().unwrap_or_default().to_owned()
        } else {
            format!("{}!{}", item["repo"].as_str().unwrap_or_default(), item["number"])
        };
        let previous = before.get(key);
        let mut push = |news: &str, topic: Option<&str>, level, story_kind: StoryKind| {
            events.push(Event {
                connector: "gitlab".into(),
                key: format!("{key}:{news}"),
                topic: topic.map(|topic| format!("{key}:{topic}")),
                level,
                title: format!(
                    "{} · {name}",
                    match story_kind {
                        StoryKind::ReviewRequested => "Review requested",
                        StoryKind::ChecksFailed => "Checks failed",
                        StoryKind::ChecksPassed => "Checks passed",
                        StoryKind::Approved => "Approved",
                        StoryKind::ChangesRequested => "Changes requested",
                    }
                ),
                detail: item["title"].as_str().unwrap_or_default().into(),
                url: item["url"].as_str().map(str::to_owned),
                story: Some(Story {
                    kind: story_kind,
                    name: name.clone(),
                    branch: (kind == "branch").then(|| item["branch"].as_str().unwrap_or_default().into()),
                }),
            })
        };
        if kind == "review" && previous.is_none() {
            push("requested", None, Level::Info, StoryKind::ReviewRequested);
        }
        if kind != "mr" && kind != "branch" {
            continue;
        }
        if previous.is_some_and(|was| was["oid"] != item["oid"] || was["ci"] != item["ci"])
            || previous.is_none()
        {
            let oid = item["oid"].as_str().unwrap_or_default();
            match item["ci"].as_str() {
                Some("failing") => push(
                    &format!("ci-failed:{oid}"),
                    Some("ci"),
                    Level::Error,
                    StoryKind::ChecksFailed,
                ),
                Some("passing") => push(
                    &format!("ci-passed:{oid}"),
                    Some("ci"),
                    Level::Ok,
                    StoryKind::ChecksPassed,
                ),
                _ => {}
            }
        }
        if item["changes"] == true && previous.is_none_or(|was| was["changes"] != true) {
            push(
                "changes",
                Some("review"),
                Level::Warn,
                StoryKind::ChangesRequested,
            );
        } else if item["changes"] != true
            && item["approved"] == true
            && previous.is_none_or(|was| was["approved"] != true || was["changes"] == true)
        {
            push("approved", Some("review"), Level::Ok, StoryKind::Approved);
        }
    }
    events
}

#[cfg(test)]
#[path = "gitlab_tests.rs"]
mod tests;
