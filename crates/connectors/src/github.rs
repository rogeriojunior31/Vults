//! GitHub through `gh`, the CLI the user is already logged into: we never see a token.
//!
//! One GraphQL query per poll (1 point of the 5000/hour budget) covers the user's open pull
//! requests (checks and review decision), reviews requested from them, and the checks on the
//! default branch of their recently pushed repositories.

use std::time::Duration;

use serde_json::{Value, json};
use tokio::process::Command;

use crate::{Checks, Connector, Error, Event, Group, Level, Poll, Review, Row, Snapshot, Story, StoryKind};

#[derive(Debug)]
pub struct GitHub;

const QUERY: &str = r#"query {
  viewer {
    pullRequests(first: 20, states: OPEN, orderBy: {field: UPDATED_AT, direction: DESC}) {
      nodes { number title url repository { nameWithOwner } reviewDecision
        commits(last: 1) { nodes { commit { oid statusCheckRollup { state } } } } }
    }
    repositories(first: 10, ownerAffiliations: OWNER, orderBy: {field: PUSHED_AT, direction: DESC}) {
      nodes { nameWithOwner url isArchived defaultBranchRef { name target { ... on Commit {
        oid messageHeadline statusCheckRollup { state } } } } }
    }
  }
  search(query: "is:open is:pr review-requested:@me archived:false", type: ISSUE, first: 20) {
    nodes { ... on PullRequest { number title url repository { nameWithOwner } } }
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
    let stderr = String::from_utf8_lossy(stderr).to_lowercase();
    if !success {
        return if stderr.contains("rate limit") {
            Error::RateLimited {
                retry_after: Duration::from_secs(15 * 60),
            }
        } else if stderr.contains("gh auth login") || stderr.contains("401") || stderr.contains("not logged")
        {
            Error::Auth("gh isn't logged in: run `gh auth login` in a terminal".into())
        } else {
            Error::Other(stderr.lines().next().unwrap_or("gh failed").to_string())
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
        s.insert(
            format!("pr:{repo}#{}", pr["number"]),
            json!({
                "title": pr["title"], "url": pr["url"], "oid": commit["oid"],
                "ci": commit["statusCheckRollup"]["state"], "review": pr["reviewDecision"],
                "rank": rank,
            }),
        );
    }
    for (rank, pr) in d["search"]["nodes"].as_array().into_iter().flatten().enumerate() {
        let Some(repo) = pr["repository"]["nameWithOwner"].as_str() else {
            continue;
        };
        s.insert(
            format!("review:{repo}#{}", pr["number"]),
            json!({ "title": pr["title"], "url": pr["url"], "rank": rank }),
        );
    }
    for (rank, r) in d["viewer"]["repositories"]["nodes"]
        .as_array()
        .into_iter()
        .flatten()
        .enumerate()
    {
        let commit = &r["defaultBranchRef"]["target"];
        if r["isArchived"] == true || commit["oid"].is_null() {
            continue;
        }
        let repo = r["nameWithOwner"].as_str().unwrap_or_default();
        s.insert(
            format!("branch:{repo}"),
            json!({
                "branch": r["defaultBranchRef"]["name"],
                "oid": commit["oid"],
                "headline": commit["messageHeadline"],
                "ci": commit["statusCheckRollup"]["state"],
                "url": format!("{}/commit/{}", r["url"].as_str().unwrap_or_default(), commit["oid"].as_str().unwrap_or_default()),
                "rank": rank,
            }),
        );
    }
    s
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
                _ => return None,
            };
            let row = Row {
                item: key.clone(),
                group,
                name: short,
                title,
                checks,
                review,
                url: v["url"].as_str().map(str::to_string),
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
    use super::*;

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
