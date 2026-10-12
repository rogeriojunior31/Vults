//! GitHub through `gh`, the CLI the user is already logged into: we never see a token.
//!
//! One GraphQL query per poll (1 point of the 5000/hour budget) covers the user's open pull
//! requests (checks one by one, reviewers, branch, size), reviews requested from them, and the
//! checks on the default branch and the latest release of their recently pushed repositories.

use std::time::Duration;

use serde_json::{Value, json};
use tokio::process::Command;

use crate::{
    Checks, Connector, Diff, Error, Event, Facts, Group, Level, Poll, Release, Review, Reviewer, Row, Run,
    RunState, Said, Snapshot, Story, StoryKind,
};

#[derive(Debug)]
pub struct GitHub;

const QUERY: &str = r#"query {
  viewer {
    pullRequests(first: 20, states: OPEN, orderBy: {field: UPDATED_AT, direction: DESC}) {
      nodes { number title url repository { nameWithOwner } reviewDecision
        headRefName additions deletions changedFiles updatedAt
        latestReviews(first: 5) { nodes { author { login } state } }
        reviewRequests(first: 5) { nodes { requestedReviewer { ... on User { login } } } }
        commits(last: 1) { nodes { commit { oid statusCheckRollup { state
          contexts(first: 30) { nodes { __typename
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
}

/// What `gh api graphql` printed, as a snapshot or the reason there is none.
fn answer(success: bool, stdout: &[u8], stderr: &[u8]) -> Result<Snapshot, Error> {
    // A GraphQL answer can carry `errors` beside `data` (one SAML-protected org, one deleted
    // repository): gh then exits 1 but still prints the answer. The rest of the data is good.
    // A whole list missing is not partial: its items would all come back as news next time.
    if let Ok(data) = serde_json::from_slice::<Value>(stdout)
        && data["data"]["viewer"]["pullRequests"]["nodes"].is_array()
        && data["data"]["viewer"]["repositories"]["nodes"].is_array()
        && data["data"]["search"]["nodes"].is_array()
    {
        return Ok(snapshot(&data));
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

/// Opens an issue in `repo` (`owner/name`) on the user's login, on their click (a task's **Open an
/// issue**): its address.
pub async fn create_issue(repo: &str, title: &str, body: &str) -> Result<String, Error> {
    if !plain_repo(repo) {
        return Err(Error::Other("not a GitHub repository".into()));
    }
    // Arguments, never a shell: the title and body are the user's text.
    let out = Command::new("gh")
        .args([
            "issue", "create", "--repo", repo, "--title", title, "--body", body,
        ])
        .stdin(std::process::Stdio::null())
        .kill_on_drop(true)
        .output();
    let out = tokio::time::timeout(Duration::from_secs(30), out)
        .await
        .map_err(|_| Error::Other("GitHub took too long to answer".into()))?
        .map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => Error::Unavailable("The GitHub CLI (gh) isn't installed".into()),
            _ => Error::Other(format!("can't run gh: {e}")),
        })?;
    issue_answer(repo, out.status.success(), &out.stdout, &out.stderr)
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

/// Squash-merges the user's pull request `pr:owner/repo#12` (a row of the card) through their
/// login and deletes its branch, on their confirmed click. Only while its head is still `head`,
/// the commit the card showed green: GitHub refuses a pull request pushed to since.
pub async fn merge(item: &str, head: &str) -> Result<(), Error> {
    let (repo, number) = pull_request(item).ok_or_else(|| Error::Other("not a pull request".into()))?;
    if !commit(head) {
        return Err(Error::Other("not a commit".into()));
    }
    let out = Command::new("gh")
        .args([
            "pr",
            "merge",
            number,
            &format!("--repo={repo}"),
            &format!("--match-head-commit={head}"),
            "--squash",
            "--delete-branch",
        ])
        // Never a question in a terminal nobody sees.
        .env("GH_PROMPT_DISABLED", "1")
        .stdin(std::process::Stdio::null())
        .kill_on_drop(true)
        .output();
    let out = tokio::time::timeout(Duration::from_secs(60), out)
        .await
        .map_err(|_| Error::Other("GitHub took too long to answer".into()))?
        .map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => Error::Unavailable("The GitHub CLI (gh) isn't installed".into()),
            _ => Error::Other(format!("can't run gh: {e}")),
        })?;
    if out.status.success() {
        return Ok(());
    }
    match gh_error(false, &out.stderr) {
        // Nothing tries a merge again on its own: the user does, later.
        Error::RateLimited { .. } => Err(Error::Other(
            "GitHub's rate limit is spent; try again later".into(),
        )),
        e => Err(e),
    }
}

/// A commit's id as GitHub gives it: hex, never a flag.
fn commit(head: &str) -> bool {
    (7..=64).contains(&head.len()) && head.chars().all(|c| c.is_ascii_hexdigit())
}

/// `pr:owner/repo#12` → (`owner/repo`, `12`): only a plain repository and a number reach `gh`.
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
    matches!(repo.split_once('/'), Some((o, n)) if ok(o) && ok(n) && !n.contains('/') && o != "." && o != "..")
}

/// The issue's address `gh` printed last, only if it is this repository's: GitHub's names know
/// no case, so `MyOrg/Site` is `myorg/site`.
fn issue_answer(repo: &str, success: bool, stdout: &[u8], stderr: &[u8]) -> Result<String, Error> {
    let prefix = format!("https://github.com/{repo}/issues/").to_ascii_lowercase();
    String::from_utf8_lossy(stdout)
        .lines()
        .map(str::trim)
        .rfind(|l| {
            l.to_ascii_lowercase()
                .strip_prefix(&prefix)
                .is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()))
        })
        .map(str::to_owned)
        .ok_or_else(|| gh_error(success, stderr))
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
                    Some(oid) => format!(
                        "gh pr merge {n} --repo {r} --squash --delete-branch --match-head-commit {oid}"
                    ),
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
    fn an_issue_answer_is_this_repositorys_address_only() {
        assert!(plain_repo("me/site") && plain_repo("my-org/site.rs"));
        assert!(
            !plain_repo("me") && !plain_repo("me/site/x") && !plain_repo("../x") && !plain_repo("me/s ite")
        );
        let out = b"Creating issue in me/site\n\nhttps://github.com/me/site/issues/12\n";
        assert_eq!(
            issue_answer("me/site", true, out, b"").ok().as_deref(),
            Some("https://github.com/me/site/issues/12")
        );
        let cased = b"https://github.com/MyOrg/Site/issues/7\n";
        assert_eq!(
            issue_answer("myorg/site", true, cased, b"").ok().as_deref(),
            Some("https://github.com/MyOrg/Site/issues/7")
        );
        let elsewhere = b"https://github.com/evil/x/issues/1\n";
        assert!(issue_answer("me/site", true, elsewhere, b"").is_err());
        assert!(issue_answer("me/site", false, b"", b"HTTP 403").is_err());
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

    fn answer(pr_ci: &str, review: Value, branch_ci: &str, oid: &str, requested: bool) -> Value {
        json!({ "data": {
            "viewer": {
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
            Some("gh pr merge 9 --repo me/app --squash --delete-branch --match-head-commit abc1234")
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
    fn a_partial_answer_still_yields_a_snapshot() {
        let got = super::answer(
            false,
            PARTIAL.as_bytes(),
            b"gh: Resource protected by organization SAML enforcement.\n",
        )
        .expect("the data beside the errors is used");
        assert_eq!(got.keys().collect::<Vec<_>>(), ["pr:me/app#12"]);
        assert_eq!(got["pr:me/app#12"]["ci"], "SUCCESS");
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
