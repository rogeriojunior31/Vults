//! Typed pull request changes, bound to a human-reviewed repository, refs and commit.

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::{
    Digest, GhApi, GithubApi, Sha256, bounded_text, checked_repository, commit, gated, github_host,
    plain_ref, url_encode_path, write_repository,
};
use crate::{Error, Gate};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum Side {
    Left,
    Right,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ReviewComment {
    pub path: String,
    pub line: u32,
    pub side: Side,
    pub body: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ReviewEvent {
    Comment,
    Approve,
    RequestChanges,
}

impl ReviewEvent {
    fn state(self) -> &'static str {
        match self {
            Self::Comment => "COMMENTED",
            Self::Approve => "APPROVED",
            Self::RequestChanges => "CHANGES_REQUESTED",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub enum PullRequestWrite {
    Create {
        repository: String,
        base: String,
        head: String,
        title: String,
        body: String,
        draft: bool,
    },
    Edit {
        repository: String,
        number: u32,
        title: Option<String>,
        body: Option<String>,
        base: Option<String>,
    },
    Ready {
        repository: String,
        number: u32,
    },
    Draft {
        repository: String,
        number: u32,
    },
    RequestReviewers {
        repository: String,
        number: u32,
        reviewers: Vec<String>,
        #[serde(rename = "teamReviewers")]
        team_reviewers: Vec<String>,
    },
    SubmitReview {
        repository: String,
        number: u32,
        event: ReviewEvent,
        body: String,
        comments: Vec<ReviewComment>,
    },
}

impl PullRequestWrite {
    pub fn repository(&self) -> &str {
        match self {
            Self::Create { repository, .. }
            | Self::Edit { repository, .. }
            | Self::Ready { repository, .. }
            | Self::Draft { repository, .. }
            | Self::RequestReviewers { repository, .. }
            | Self::SubmitReview { repository, .. } => repository,
        }
    }
    pub fn number(&self) -> Option<u32> {
        match self {
            Self::Create { .. } => None,
            Self::Edit { number, .. }
            | Self::Ready { number, .. }
            | Self::Draft { number, .. }
            | Self::RequestReviewers { number, .. }
            | Self::SubmitReview { number, .. } => Some(*number),
        }
    }
    pub fn tool_name(&self) -> &'static str {
        match self {
            Self::Create { .. } => "github_pull_request_create",
            Self::Edit { .. } => "github_pull_request_edit",
            Self::Ready { .. } => "github_pull_request_ready",
            Self::Draft { .. } => "github_pull_request_draft",
            Self::RequestReviewers { .. } => "github_pull_request_request_reviewers",
            Self::SubmitReview {
                event: ReviewEvent::Comment,
                ..
            } => "github_pull_request_review_comment",
            Self::SubmitReview {
                event: ReviewEvent::Approve,
                ..
            } => "github_pull_request_review_approve",
            Self::SubmitReview {
                event: ReviewEvent::RequestChanges,
                ..
            } => "github_pull_request_review_request_changes",
        }
    }
    pub fn request_hash(&self) -> String {
        digest(self)
    }
    pub fn validate(&self) -> Result<(), Error> {
        checked_repository(self.repository())?;
        if self
            .number()
            .is_some_and(|number| number == 0 || number > i32::MAX as u32)
        {
            return invalid("Give a positive pull request number.");
        }
        let title = |value: &str| {
            !value.trim().is_empty() && value.chars().count() <= 256 && !value.chars().any(char::is_control)
        };
        let body = |value: &str| value.len() <= 10000 && !value.contains('\0');
        match self {
            Self::Create {
                base,
                head,
                title: t,
                body: b,
                ..
            } => {
                if !plain_ref(base) || !plain_ref(head) || base == head {
                    return invalid(
                        "Choose two different branch names in this repository. Fork heads are not supported by these tools.",
                    );
                }
                if !title(t) || !body(b) {
                    return invalid("The pull request title or body is invalid or too long.");
                }
            }
            Self::Edit {
                title: t,
                body: b,
                base,
                ..
            } => {
                if t.is_none() && b.is_none() && base.is_none()
                    || t.as_deref().is_some_and(|v| !title(v))
                    || b.as_deref().is_some_and(|v| !body(v))
                    || base.as_deref().is_some_and(|v| !plain_ref(v))
                {
                    return invalid("Give a valid title, body or base branch to change.");
                }
            }
            Self::RequestReviewers {
                reviewers,
                team_reviewers,
                ..
            } => {
                if reviewers.len() + team_reviewers.len() == 0
                    || reviewers.len() + team_reviewers.len() > 10
                    || !valid_names(reviewers, 39, false)
                    || !valid_names(team_reviewers, 100, true)
                {
                    return invalid("Choose one to ten distinct reviewer logins or team slugs.");
                }
            }
            Self::SubmitReview {
                event,
                body: b,
                comments,
                ..
            } => {
                if !body(b) || (*event != ReviewEvent::Approve && b.trim().is_empty()) || comments.len() > 10
                {
                    return invalid("Give the exact review body and at most ten inline comments.");
                }
                let mut seen = std::collections::HashSet::new();
                for c in comments {
                    if c.path.is_empty()
                        || c.path.len() > 1024
                        || c.path.starts_with('/')
                        || c.path.split('/').any(|part| part == ".." || part == ".")
                        || c.path.chars().any(char::is_control)
                        || c.line == 0
                        || c.line > i32::MAX as u32
                        || c.body.trim().is_empty()
                        || !body(&c.body)
                        || !seen.insert((c.path.clone(), c.line, c.side as u8))
                    {
                        return invalid(
                            "Each inline comment needs a unique file, diff side, positive line and nonempty body.",
                        );
                    }
                }
            }
            Self::Ready { .. } | Self::Draft { .. } => {}
        }
        Ok(())
    }
}

fn invalid<T>(message: &str) -> Result<T, Error> {
    Err(Error::Other(message.into()))
}
fn digest(value: &impl Serialize) -> String {
    Sha256::digest(serde_json::to_vec(value).expect("serializable request"))
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
fn valid_names(names: &[String], max: usize, team: bool) -> bool {
    let mut unique = std::collections::HashSet::new();
    names.iter().all(|name| {
        !name.is_empty()
            && name.len() <= max
            && name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || (team && c == '_'))
            && unique.insert(name.to_ascii_lowercase())
    })
}

pub async fn prepare_pull_request_write(gate: &dyn Gate, action: &PullRequestWrite) -> Result<Value, Error> {
    prepare_using(gate, action, &GhApi).await
}
pub async fn execute_pull_request_write(
    gate: &dyn Gate,
    action: &PullRequestWrite,
    expected: &Value,
    hash: &str,
) -> Result<Value, Error> {
    execute_using(gate, action, expected, hash, &GhApi).await
}

async fn prepare_using(
    gate: &dyn Gate,
    action: &PullRequestWrite,
    api: &impl GithubApi,
) -> Result<Value, Error> {
    action.validate()?;
    let current = inspect(gate, action, api, None).await?;
    if duplicate(action, &current) {
        return invalid("This pull request change is already present. No duplicate proposal was made.");
    }
    Ok(current)
}

async fn execute_using(
    gate: &dyn Gate,
    action: &PullRequestWrite,
    expected: &Value,
    hash: &str,
    api: &impl GithubApi,
) -> Result<Value, Error> {
    action.validate()?;
    if action.request_hash() != hash {
        return invalid("The pull request change no longer matches its approved request hash.");
    }
    let host = expected["source"]["host"]
        .as_str()
        .filter(|host| github_host(&format!("https://{host}")).is_some())
        .ok_or_else(|| Error::Other("Review a new card with a valid GitHub host.".into()))?;
    let before = inspect(gate, action, api, Some(host)).await?;
    if before["binding"] != expected["binding"] {
        return invalid(
            "The GitHub account, pull request, refs, commit or review context changed after approval. Review a new card.",
        );
    }
    if duplicate(action, &before) {
        return invalid("This exact change is already present. No duplicate write was sent.");
    }
    let (method, path, body) = mutation(action, &before)?;
    let result = gated(gate, api.request(Some(host), method, &path, Some(body))).await;
    let after=inspect(gate,action,api,Some(host)).await.map_err(|error|Error::Other(format!("The pull request write outcome is uncertain because verification failed ({error}). Inspect GitHub before proposing a retry.")))?;
    if !same_target(action, &before, &after) || !present(action, &after) {
        return invalid(
            "The complete pull request change could not be verified on the approved refs and commit. Inspect GitHub before retrying.",
        );
    }
    let pr = if matches!(action, PullRequestWrite::Create { .. }) {
        after["related"]["matches"][0].clone()
    } else {
        after["pullRequest"].clone()
    };
    Ok(
        json!({"status":if result.is_ok(){"verified"}else{"verified_after_error"},"source":after["source"],"pullRequest":summary(&pr),"review":after["related"]["matchingReviews"].as_array().and_then(|items|items.first())}),
    )
}

fn same_target(action: &PullRequestWrite, before: &Value, after: &Value) -> bool {
    if before["binding"]["identity"] != after["binding"]["identity"]
        || before["binding"]["repositoryId"] != after["binding"]["repositoryId"]
    {
        return false;
    }
    if matches!(action, PullRequestWrite::Create { .. }) {
        return before["binding"]["refs"] == after["binding"]["refs"];
    }
    let a = &before["pullRequest"];
    let b = &after["pullRequest"];
    let base = if let PullRequestWrite::Edit { base: Some(_), .. } = action {
        &before["proposal"]["base"]
    } else {
        &a["base"]
    };
    a["id"] == b["id"] && a["head"] == b["head"] && *base == b["base"]
}

fn summary(pr: &Value) -> Value {
    json!({"id":pr["id"],"number":pr["number"],"url":pr["url"],"title":pr["title"],"state":pr["state"],"draft":pr["draft"],"base":pr["base"],"head":pr["head"]})
}

async fn inspect(
    gate: &dyn Gate,
    action: &PullRequestWrite,
    api: &impl GithubApi,
    host: Option<&str>,
) -> Result<Value, Error> {
    let context = write_repository(gate, api, action.repository(), host).await?;
    let host = context.source["host"].as_str().expect("validated host");
    let root = format!("repos/{}", action.repository());
    let mut binding = context.binding;
    let mut proposal = json!({});
    let mut related = json!({});
    let mut pr = Value::Null;
    if let PullRequestWrite::Create { base, head, .. } = action {
        let base = branch(gate, api, host, &root, base).await?;
        let head = branch(gate, api, host, &root, head).await?;
        if base["sha"] == head["sha"] {
            return invalid(
                "These branches point to the same commit. There is no pull request change to propose.",
            );
        }
        binding["refs"] = json!({"base":base,"head":head});
        proposal = binding["refs"].clone();
        let owner = action
            .repository()
            .split_once('/')
            .expect("validated repository")
            .0;
        let path = format!(
            "{root}/pulls?state=open&base={}&head={}",
            url_encode_path(base["ref"].as_str().expect("branch ref")),
            url_encode_path(&format!("{owner}:{}", head["ref"].as_str().expect("branch ref")))
        );
        let items = collection(gate, api, host, &path).await?;
        let matches = items
            .iter()
            .map(|item| parse_pr(item, action.repository(), host, &binding["repositoryId"], None))
            .collect::<Result<Vec<_>, _>>()?;
        if matches
            .iter()
            .any(|item| item["base"] != base || item["head"] != head)
        {
            return invalid("GitHub returned a pull request outside the exact requested branch pair.");
        }
        binding["existingPullRequestIds"] = json!(matches.iter().map(|item| &item["id"]).collect::<Vec<_>>());
        related["matches"] = json!(matches);
    } else {
        let number = action.number().expect("numbered operation");
        let value = gated(
            gate,
            api.request(Some(host), "GET", &format!("{root}/pulls/{number}"), None),
        )
        .await?;
        pr = parse_pr(
            &value,
            action.repository(),
            host,
            &binding["repositoryId"],
            Some(number),
        )?;
        if let PullRequestWrite::Edit { base: Some(base), .. } = action {
            proposal["base"] = branch(gate, api, host, &root, base).await?;
            if proposal["base"]["ref"] == pr["head"]["ref"] {
                return invalid("A pull request's base and head must be different branches.");
            }
        }
        if let PullRequestWrite::RequestReviewers { reviewers, .. } = action
            && reviewers.iter().any(|login| {
                pr["author"]["login"]
                    .as_str()
                    .is_some_and(|author| author.eq_ignore_ascii_case(login))
            })
        {
            return invalid("The pull request author cannot be requested as a reviewer.");
        }
        if let PullRequestWrite::SubmitReview {
            event,
            body,
            comments,
            ..
        } = action
        {
            if *event != ReviewEvent::Comment && pr["author"]["id"] == binding["identity"]["id"] {
                return invalid("You cannot approve or request changes on your own pull request.");
            }
            let reviews = collection(gate, api, host, &format!("{root}/pulls/{number}/reviews")).await?;
            if reviews.iter().any(|review| {
                !review["id"].is_u64()
                    || !review["state"].is_string()
                    || !review["user"]["id"].is_u64()
                    || !review["commit_id"].is_string()
                    || !(review["body"].is_null() || review["body"].is_string())
            }) {
                return invalid("GitHub returned incomplete review coverage.");
            }
            if reviews.iter().any(|review| {
                review["user"]["id"] == binding["identity"]["id"] && review["state"] == "PENDING"
            }) {
                return invalid(
                    "Finish or discard your pending review on GitHub before proposing a new review here.",
                );
            }
            binding["reviewsHash"] = json!(digest(&reviews));
            let mut matching = Vec::new();
            for review in reviews.iter().filter(|review| {
                review["user"]["id"] == binding["identity"]["id"]
                    && review["state"] == event.state()
                    && review["commit_id"] == pr["head"]["sha"]
                    && review["body"].as_str().unwrap_or_default() == body
            }) {
                let posted = collection(
                    gate,
                    api,
                    host,
                    &format!("{root}/pulls/{number}/reviews/{}/comments", review["id"]),
                )
                .await?;
                if same_comments(comments, &posted) {
                    matching.push(json!({"id":review["id"],"url":review["html_url"],"state":review["state"],"commit":review["commit_id"]}));
                    if matching.len() == 2 {
                        break;
                    }
                }
            }
            related["matchingReviews"] = json!(matching);
            if !comments.is_empty() {
                let files = collection(gate, api, host, &format!("{root}/pulls/{number}/files")).await?;
                let mut anchors = Vec::new();
                for comment in comments {
                    let file = files
                        .iter()
                        .find(|file| file["filename"] == comment.path)
                        .ok_or_else(|| {
                            Error::Other("An inline comment's file is no longer in this pull request.".into())
                        })?;
                    let patch = file["patch"].as_str().ok_or_else(|| {
                        Error::Other("The current diff has no text patch for this inline comment.".into())
                    })?;
                    if file["status"] == "removed" && comment.side == Side::Right {
                        return invalid("A removed file has no right-side review line.");
                    }
                    let line = diff_line(patch, comment.line, comment.side).ok_or_else(|| {
                        Error::Other(
                            "An inline comment is not anchored to a valid line in the current diff.".into(),
                        )
                    })?;
                    let (text, truncated) = bounded_text(line, 800);
                    anchors.push(json!({"path":comment.path,"line":comment.line,"side":comment.side,"text":text,"truncated":truncated,"patchHash":digest(&patch)}));
                }
                binding["anchors"] = json!(anchors);
                proposal["inlineComments"] = json!(anchors);
            }
        }
        binding["pullRequest"] = pr.clone();
        binding["proposal"] = proposal.clone();
        binding["related"] = related.clone();
    }
    Ok(
        json!({"source":context.source,"binding":binding,"pullRequest":pr,"proposal":proposal,"related":related}),
    )
}

fn parse_pr(
    value: &Value,
    repository: &str,
    host: &str,
    repository_id: &Value,
    number: Option<u32>,
) -> Result<Value, Error> {
    let n = value["number"]
        .as_u64()
        .filter(|n| *n > 0 && *n <= i32::MAX as u64)
        .ok_or_else(|| Error::Partial("GitHub omitted the pull request number.".into()))?;
    let refs_valid = ["base", "head"].iter().all(|side| {
        value[*side]["repo"]["id"] == *repository_id
            && value[*side]["ref"].as_str().is_some_and(plain_ref)
            && value[*side]["sha"].as_str().is_some_and(commit)
    });
    if number.is_some_and(|number| u64::from(number) != n)
        || !value["id"].is_u64()
        || !value["node_id"].is_string()
        || !value["title"].is_string()
        || !(value["body"].is_null() || value["body"].is_string())
        || value["state"] != "open"
        || !value["merged_at"].is_null()
        || !value["draft"].is_boolean()
        || !value["updated_at"].is_string()
        || !value["user"]["id"].is_u64()
        || !value["user"]["login"].is_string()
        || !refs_valid
        || !value["html_url"]
            .as_str()
            .is_some_and(|url| url.eq_ignore_ascii_case(&format!("https://{host}/{repository}/pull/{n}")))
    {
        return invalid(
            "GitHub did not return the exact open pull request in this repository. Fork heads and inaccessible or removed refs are not supported by these tools.",
        );
    }
    let logins = |key: &str, field: &str| -> Result<Vec<String>, Error> {
        let items = value[key]
            .as_array()
            .ok_or_else(|| Error::Partial("GitHub omitted requested-reviewer coverage.".into()))?;
        let mut names = items
            .iter()
            .map(|item| {
                item[field]
                    .as_str()
                    .map(str::to_owned)
                    .ok_or_else(|| Error::Partial("GitHub returned malformed requested reviewers.".into()))
            })
            .collect::<Result<Vec<_>, _>>()?;
        names.sort_unstable();
        Ok(names)
    };
    Ok(
        json!({"id":value["id"],"nodeId":value["node_id"],"number":n,"url":value["html_url"],"title":value["title"],"body":value["body"].as_str().unwrap_or_default(),"state":"open","draft":value["draft"],"updatedAt":value["updated_at"],"author":value["user"],"base":{"ref":value["base"]["ref"],"sha":value["base"]["sha"]},"head":{"ref":value["head"]["ref"],"sha":value["head"]["sha"]},"reviewers":logins("requested_reviewers","login")?,"teamReviewers":logins("requested_teams","slug")?}),
    )
}

async fn branch(
    gate: &dyn Gate,
    api: &impl GithubApi,
    host: &str,
    root: &str,
    name: &str,
) -> Result<Value, Error> {
    let value = gated(
        gate,
        api.request(
            Some(host),
            "GET",
            &format!("{root}/branches/{}", url_encode_path(name)),
            None,
        ),
    )
    .await?;
    if value["name"] != name || !value["commit"]["sha"].as_str().is_some_and(commit) {
        return invalid("GitHub did not return the exact branch and commit.");
    }
    Ok(json!({"ref":name,"sha":value["commit"]["sha"]}))
}

async fn collection(
    gate: &dyn Gate,
    api: &impl GithubApi,
    host: &str,
    path: &str,
) -> Result<Vec<Value>, Error> {
    let mut kept = Vec::new();
    for page in 1..=10 {
        let path = format!(
            "{path}{}per_page=100&page={page}",
            if path.contains('?') { "&" } else { "?" }
        );
        let value = gated(gate, api.request(Some(host), "GET", &path, None)).await?;
        let items = value
            .as_array()
            .filter(|items| items.len() <= 100)
            .ok_or_else(|| Error::Partial("GitHub omitted pull request page coverage.".into()))?;
        kept.extend(items.iter().cloned());
        if items.len() < 100 {
            return Ok(kept);
        }
    }
    invalid("The pull request context reached its 1,000-item limit. Inspect GitHub before continuing.")
}

fn duplicate(action: &PullRequestWrite, current: &Value) -> bool {
    match action {
        PullRequestWrite::Create { .. } => current["related"]["matches"]
            .as_array()
            .is_some_and(|items| !items.is_empty()),
        PullRequestWrite::SubmitReview { .. } => current["related"]["matchingReviews"]
            .as_array()
            .is_some_and(|items| !items.is_empty()),
        _ => present(action, current),
    }
}

fn present(action: &PullRequestWrite, current: &Value) -> bool {
    let pr = &current["pullRequest"];
    match action {
        PullRequestWrite::Create {
            title, body, draft, ..
        } => current["related"]["matches"].as_array().is_some_and(|items| {
            items.len() == 1
                && items[0]["title"] == *title
                && items[0]["body"] == *body
                && items[0]["draft"] == *draft
        }),
        PullRequestWrite::Edit {
            title, body, base, ..
        } => {
            title.as_ref().is_none_or(|v| pr["title"] == *v)
                && body.as_ref().is_none_or(|v| pr["body"] == *v)
                && base.as_ref().is_none_or(|v| pr["base"]["ref"] == *v)
        }
        PullRequestWrite::Ready { .. } => pr["draft"] == false,
        PullRequestWrite::Draft { .. } => pr["draft"] == true,
        PullRequestWrite::RequestReviewers {
            reviewers,
            team_reviewers,
            ..
        } => {
            contains_names(&pr["reviewers"], reviewers)
                && contains_names(&pr["teamReviewers"], team_reviewers)
        }
        PullRequestWrite::SubmitReview { .. } => current["related"]["matchingReviews"]
            .as_array()
            .is_some_and(|items| items.len() == 1),
    }
}
fn contains_names(current: &Value, expected: &[String]) -> bool {
    current.as_array().is_some_and(|items| {
        expected.iter().all(|name| {
            items
                .iter()
                .any(|v| v.as_str().is_some_and(|v| v.eq_ignore_ascii_case(name)))
        })
    })
}
fn same_comments(expected: &[ReviewComment], posted: &[Value]) -> bool {
    expected.len() == posted.len()
        && expected.iter().all(|comment| {
            posted.iter().any(|value| {
                value["path"] == comment.path
                    && value["line"].as_u64() == Some(u64::from(comment.line))
                    && value["side"] == json!(comment.side)
                    && value["body"] == comment.body
            })
        })
}

fn mutation(action: &PullRequestWrite, current: &Value) -> Result<(&'static str, String, Value), Error> {
    let root = format!("repos/{}/pulls", action.repository());
    let path = format!("{root}/{}", action.number().unwrap_or_default());
    Ok(match action {
        PullRequestWrite::Create {
            base,
            head,
            title,
            body,
            draft,
            ..
        } => (
            "POST",
            root,
            json!({"base":base,"head":head,"title":title,"body":body,"draft":draft}),
        ),
        PullRequestWrite::Edit {
            title, body, base, ..
        } => {
            let mut fields = serde_json::Map::new();
            if let Some(title) = title {
                fields.insert("title".into(), json!(title));
            }
            if let Some(body) = body {
                fields.insert("body".into(), json!(body));
            }
            if let Some(base) = base {
                fields.insert("base".into(), json!(base));
            }
            ("PATCH", path, Value::Object(fields))
        }
        PullRequestWrite::RequestReviewers {
            reviewers,
            team_reviewers,
            ..
        } => (
            "POST",
            format!("{path}/requested_reviewers"),
            json!({"reviewers":reviewers,"team_reviewers":team_reviewers}),
        ),
        PullRequestWrite::SubmitReview {
            event,
            body,
            comments,
            ..
        } => (
            "POST",
            format!("{path}/reviews"),
            json!({"event":event,"body":body,"comments":comments,"commit_id":current["pullRequest"]["head"]["sha"]}),
        ),
        PullRequestWrite::Ready { .. } | PullRequestWrite::Draft { .. } => {
            let mutation = if matches!(action, PullRequestWrite::Ready { .. }) {
                "markPullRequestReadyForReview"
            } else {
                "convertPullRequestToDraft"
            };
            (
                "POST",
                "graphql".into(),
                json!({"query":format!("mutation($id:ID!){{{mutation}(input:{{pullRequestId:$id}}){{pullRequest{{id isDraft}}}}}}"),"variables":{"id":current["pullRequest"]["nodeId"]}}),
            )
        }
    })
}

fn diff_line(patch: &str, target: u32, side: Side) -> Option<&str> {
    let (mut left, mut right) = (0u32, 0u32);
    let (mut left_end, mut right_end) = (0u32, 0u32);
    let mut hunk = false;
    for line in patch.lines() {
        if let Some(header) = line.strip_prefix("@@ -") {
            let (old, new) = header.split_once(" +")?;
            let (new, _) = new.split_once(" @@")?;
            let range = |value: &str| -> Option<(u32, u32)> {
                let (start, count) = value.split_once(',').unwrap_or((value, "1"));
                let start = start.parse().ok()?;
                let count = count.parse().ok()?;
                Some((start, count))
            };
            let (a, n) = range(old)?;
            let (b, m) = range(new)?;
            left = a;
            right = b;
            left_end = a.checked_add(n)?;
            right_end = b.checked_add(m)?;
            hunk = true;
            continue;
        }
        if !hunk {
            return None;
        }
        if line.starts_with('\\') {
            continue;
        }
        let prefix = line.as_bytes().first()?;
        let text = line.get(1..)?;
        let (old, new) = match prefix {
            b' ' => (true, true),
            b'-' => (true, false),
            b'+' => (false, true),
            _ => return None,
        };
        if (old && left >= left_end) || (new && right >= right_end) {
            return None;
        }
        if (side == Side::Left && old && left == target) || (side == Side::Right && new && right == target) {
            return Some(text);
        }
        if old {
            left = left.checked_add(1)?;
        }
        if new {
            right = right.checked_add(1)?;
        }
    }
    None
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
            assert!(self.0.lock().unwrap().is_empty(), "expected request was not sent");
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
                .expect("unexpected GitHub call or retry");
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
    fn step(method: &'static str, path: &str, body: Option<Value>, result: Result<Value, Error>) -> Step {
        Step {
            host: Some("github.com"),
            method,
            path: path.into(),
            body,
            result,
        }
    }
    fn get(path: &str, value: Value) -> Step {
        step("GET", path, None, Ok(value))
    }
    fn pr() -> Value {
        json!({"id":70,"node_id":"PR_7","number":7,"html_url":"https://github.com/org/repo/pull/7","title":"Title","body":"Body","state":"open","merged_at":null,"draft":false,"updated_at":"now","user":{"id":2,"login":"author"},"base":{"ref":"main","sha":"a".repeat(40),"repo":{"id":20}},"head":{"ref":"feature","sha":"b".repeat(40),"repo":{"id":20}},"requested_reviewers":[],"requested_teams":[]})
    }
    fn context(initial: bool) -> Vec<Step> {
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
        ]
    }
    fn reads(action: &PullRequestWrite, value: Value, initial: bool, completed: bool) -> Vec<Step> {
        let mut steps = context(initial);
        if let PullRequestWrite::Create { .. } = action {
            steps.push(get(
                "repos/org/repo/branches/main",
                json!({"name":"main","commit":{"sha":"a".repeat(40)}}),
            ));
            steps.push(get(
                "repos/org/repo/branches/feature",
                json!({"name":"feature","commit":{"sha":"b".repeat(40)}}),
            ));
            steps.push(get(
                "repos/org/repo/pulls?state=open&base=main&head=org%3Afeature&per_page=100&page=1",
                if completed { json!([value]) } else { json!([]) },
            ));
        } else {
            steps.push(get("repos/org/repo/pulls/7", value));
            if let PullRequestWrite::Edit { base: Some(base), .. } = action {
                steps.push(get(
                    &format!("repos/org/repo/branches/{base}"),
                    json!({"name":base,"commit":{"sha":"c".repeat(40)}}),
                ));
            }
            if let PullRequestWrite::SubmitReview {
                event,
                body,
                comments,
                ..
            } = action
            {
                let posted = json!({"id":90,"state":event.state(),"commit_id":"b".repeat(40),"body":body,"user":{"id":1},"html_url":"https://github.com/org/repo/pull/7#pullrequestreview-90"});
                steps.push(get(
                    "repos/org/repo/pulls/7/reviews?per_page=100&page=1",
                    if completed { json!([posted]) } else { json!([]) },
                ));
                if completed {
                    steps.push(get(
                        "repos/org/repo/pulls/7/reviews/90/comments?per_page=100&page=1",
                        json!(comments),
                    ));
                }
                if !comments.is_empty() {
                    steps.push(get("repos/org/repo/pulls/7/files?per_page=100&page=1",json!([{"filename":"file.rs","status":"modified","patch":"@@ -1,3 +1,3 @@\n context\n-old\n+new\n tail"}])));
                }
            }
        }
        steps
    }
    fn review(event: ReviewEvent, inline: bool) -> PullRequestWrite {
        PullRequestWrite::SubmitReview {
            repository: "org/repo".into(),
            number: 7,
            event,
            body: "Review body".into(),
            comments: if inline {
                vec![ReviewComment {
                    path: "file.rs".into(),
                    line: 2,
                    side: Side::Right,
                    body: "Inline body".into(),
                }]
            } else {
                vec![]
            },
        }
    }
    fn edit() -> PullRequestWrite {
        PullRequestWrite::Edit {
            repository: "org/repo".into(),
            number: 7,
            title: None,
            body: Some("Changed".into()),
            base: None,
        }
    }
    async fn preview(action: &PullRequestWrite, value: Value) -> Value {
        let api = Fake::new(reads(action, value, true, false));
        let v = prepare_using(&gate(), action, &api).await.unwrap();
        api.exhausted();
        v
    }

    #[test]
    fn exact_http_contracts_bind_reviews_to_the_head_and_keep_actions_distinct() {
        let current = json!({"pullRequest":{"head":{"sha":"b".repeat(40)},"nodeId":"PR_7"}});
        let (method, path, body) = mutation(&review(ReviewEvent::RequestChanges, true), &current).unwrap();
        assert_eq!(
            (method, path.as_str()),
            ("POST", "repos/org/repo/pulls/7/reviews")
        );
        assert_eq!(
            body,
            json!({"event":"REQUEST_CHANGES","body":"Review body","commit_id":"b".repeat(40),"comments":[{"path":"file.rs","line":2,"side":"RIGHT","body":"Inline body"}]})
        );
        assert_ne!(
            review(ReviewEvent::Approve, false).request_hash(),
            review(ReviewEvent::Comment, false).request_hash()
        );
        assert_eq!(
            mutation(&edit(), &current).unwrap(),
            (
                "PATCH",
                "repos/org/repo/pulls/7".into(),
                json!({"body":"Changed"})
            )
        );
        let action = PullRequestWrite::RequestReviewers {
            repository: "org/repo".into(),
            number: 7,
            reviewers: vec!["alice".into()],
            team_reviewers: vec!["core".into()],
        };
        assert_eq!(
            mutation(&action, &current).unwrap(),
            (
                "POST",
                "repos/org/repo/pulls/7/requested_reviewers".into(),
                json!({"reviewers":["alice"],"team_reviewers":["core"]})
            )
        );
        for (action, name) in [
            (
                PullRequestWrite::Ready {
                    repository: "org/repo".into(),
                    number: 7,
                },
                "markPullRequestReadyForReview",
            ),
            (
                PullRequestWrite::Draft {
                    repository: "org/repo".into(),
                    number: 7,
                },
                "convertPullRequestToDraft",
            ),
        ] {
            let (method, path, body) = mutation(&action, &current).unwrap();
            assert_eq!((method, path.as_str()), ("POST", "graphql"));
            assert!(body["query"].as_str().unwrap().contains(name));
            assert_eq!(body["variables"]["id"], "PR_7");
        }
    }

    #[tokio::test]
    async fn every_pull_request_operation_is_verified_after_one_write() {
        let actions = [
            PullRequestWrite::Create {
                repository: "org/repo".into(),
                base: "main".into(),
                head: "feature".into(),
                title: "Title".into(),
                body: "Body".into(),
                draft: true,
            },
            edit(),
            PullRequestWrite::Edit {
                repository: "org/repo".into(),
                number: 7,
                title: Some("New".into()),
                body: None,
                base: Some("stable".into()),
            },
            PullRequestWrite::Ready {
                repository: "org/repo".into(),
                number: 7,
            },
            PullRequestWrite::Draft {
                repository: "org/repo".into(),
                number: 7,
            },
            PullRequestWrite::RequestReviewers {
                repository: "org/repo".into(),
                number: 7,
                reviewers: vec!["alice".into()],
                team_reviewers: vec!["core".into()],
            },
            review(ReviewEvent::Comment, true),
            review(ReviewEvent::Approve, false),
            review(ReviewEvent::RequestChanges, true),
        ];
        for action in actions {
            let mut before = pr();
            let mut after = pr();
            match &action {
                PullRequestWrite::Create { draft, .. } => after["draft"] = json!(draft),
                PullRequestWrite::Edit {
                    title, body, base, ..
                } => {
                    if let Some(title) = title {
                        after["title"] = json!(title);
                    }
                    if let Some(body) = body {
                        after["body"] = json!(body);
                    }
                    if let Some(base) = base {
                        after["base"]["ref"] = json!(base);
                        after["base"]["sha"] = json!("c".repeat(40));
                    }
                }
                PullRequestWrite::Ready { .. } => before["draft"] = json!(true),
                PullRequestWrite::Draft { .. } => after["draft"] = json!(true),
                PullRequestWrite::RequestReviewers { .. } => {
                    after["requested_reviewers"] = json!([{"login":"alice"}]);
                    after["requested_teams"] = json!([{"slug":"core"}]);
                }
                _ => {}
            }
            let expected = preview(&action, before.clone()).await;
            let mut steps = reads(&action, before, false, false);
            let (method, path, body) = mutation(&action, &expected).unwrap();
            steps.push(step(method, &path, Some(body), Ok(json!({}))));
            steps.extend(reads(&action, after, false, true));
            let api = Fake::new(steps);
            let result = execute_using(&gate(), &action, &expected, &action.request_hash(), &api)
                .await
                .unwrap();
            assert_eq!(result["status"], "verified", "{action:?}");
            assert_eq!(result["pullRequest"]["number"], 7);
            api.exhausted();
        }
    }

    #[tokio::test]
    async fn changed_head_base_identity_or_body_are_rejected_before_writing() {
        let action = edit();
        let expected = preview(&action, pr()).await;
        for change in ["head", "base", "identity", "body", "id", "fork"] {
            let mut value = pr();
            match change {
                "head" => value["head"]["sha"] = json!("c".repeat(40)),
                "base" => value["base"]["sha"] = json!("c".repeat(40)),
                "body" => value["body"] = json!("someone edited it"),
                "id" => value["id"] = json!(71),
                "fork" => value["head"]["repo"]["id"] = json!(30),
                _ => {}
            }
            let mut steps = reads(&action, value, false, false);
            if change == "identity" {
                steps[0].result.as_mut().unwrap()["id"] = json!(3);
            }
            let api = Fake::new(steps);
            assert!(
                execute_using(&gate(), &action, &expected, &action.request_hash(), &api)
                    .await
                    .is_err(),
                "{change}"
            );
            api.exhausted();
        }
        let api = Fake::new(vec![]);
        assert!(
            execute_using(&gate(), &action, &expected, "modified-input", &api)
                .await
                .is_err()
        );
        api.exhausted();
    }

    #[test]
    fn inline_comments_use_the_correct_side_and_an_existing_diff_line() {
        let patch = "@@ -10,3 +20,3 @@\n context\n-old\n+new\n tail\n@@ -30 +40 @@\n-last\n+next";
        assert_eq!(diff_line(patch, 11, Side::Left), Some("old"));
        assert_eq!(diff_line(patch, 21, Side::Right), Some("new"));
        assert_eq!(diff_line(patch, 40, Side::Right), Some("next"));
        assert_eq!(diff_line(patch, 11, Side::Right), None);
        assert_eq!(diff_line(patch, 25, Side::Right), None);
        assert_eq!(diff_line("@@ broken", 1, Side::Right), None);
        assert_eq!(
            diff_line("@@ -1 +0,0 @@\n-removed", 1, Side::Left),
            Some("removed")
        );
        assert_eq!(diff_line("@@ -1 +0,0 @@\n-removed", 1, Side::Right), None);
    }

    #[tokio::test]
    async fn invalid_or_stale_inline_anchors_never_send_a_review() {
        let action = review(ReviewEvent::RequestChanges, true);
        for file in [
            json!({"filename":"gone.rs","status":"modified","patch":"@@ -1 +1 @@\n-a\n+b"}),
            json!({"filename":"file.rs","status":"removed","patch":"@@ -1,2 +0,0 @@\n-a\n-b"}),
            json!({"filename":"file.rs","status":"modified","patch":"@@ -10 +10 @@\n-a\n+b"}),
            json!({"filename":"file.rs","status":"modified"}),
        ] {
            let mut steps = reads(&action, pr(), true, false);
            steps.last_mut().unwrap().result = Ok(json!([file]));
            let api = Fake::new(steps);
            assert!(prepare_using(&gate(), &action, &api).await.is_err());
            api.exhausted();
        }
    }

    #[tokio::test]
    async fn timeout_and_unknown_results_read_remote_state_without_repeating() {
        let action = edit();
        let expected = preview(&action, pr()).await;
        for changed in [true, false] {
            let mut steps = reads(&action, pr(), false, false);
            steps.push(step(
                "PATCH",
                "repos/org/repo/pulls/7",
                Some(json!({"body":"Changed"})),
                Err(Error::Other("GitHub took too long to answer".into())),
            ));
            let mut value = pr();
            if changed {
                value["body"] = json!("Changed");
            }
            steps.extend(reads(&action, value, false, true));
            let api = Fake::new(steps);
            let result = execute_using(&gate(), &action, &expected, &action.request_hash(), &api).await;
            if changed {
                assert_eq!(result.unwrap()["status"], "verified_after_error");
            } else {
                assert!(result.is_err());
            }
            api.exhausted();
        }
    }

    #[tokio::test]
    async fn duplicates_pending_reviews_denied_permission_and_revocation_are_explicit() {
        let action = review(ReviewEvent::Approve, false);
        let api = Fake::new(reads(&action, pr(), true, true));
        assert!(
            prepare_using(&gate(), &action, &api)
                .await
                .unwrap_err()
                .to_string()
                .contains("already present")
        );
        api.exhausted();
        let mut steps = reads(&action, pr(), true, false);
        steps[3].result = Ok(
            json!([{"id":90,"state":"PENDING","commit_id":"b".repeat(40),"body":"draft","user":{"id":1}}]),
        );
        let api = Fake::new(steps);
        assert!(
            prepare_using(&gate(), &action, &api)
                .await
                .unwrap_err()
                .to_string()
                .contains("pending review")
        );
        api.exhausted();
        let mut steps = context(true);
        steps[1].result.as_mut().unwrap()["permissions"]["push"] = json!(false);
        let api = Fake::new(steps);
        assert!(prepare_using(&gate(), &edit(), &api).await.is_err());
        api.exhausted();
        let api = Fake::new(vec![]);
        assert!(
            prepare_using(&Budget(Mutex::new(0)), &edit(), &api)
                .await
                .is_err()
        );
        api.exhausted();
        let expected = preview(&edit(), pr()).await;
        let api = Fake::new(reads(&edit(), pr(), false, false));
        assert!(
            execute_using(
                &Budget(Mutex::new(3)),
                &edit(),
                &expected,
                &edit().request_hash(),
                &api
            )
            .await
            .is_err()
        );
        api.exhausted();
    }

    #[tokio::test]
    async fn existing_branch_pair_and_changed_inline_context_require_a_new_proposal() {
        let create = PullRequestWrite::Create {
            repository: "org/repo".into(),
            base: "main".into(),
            head: "feature".into(),
            title: "A different title".into(),
            body: "Different body".into(),
            draft: true,
        };
        let api = Fake::new(reads(&create, pr(), true, true));
        assert!(
            prepare_using(&gate(), &create, &api)
                .await
                .unwrap_err()
                .to_string()
                .contains("already present")
        );
        api.exhausted();
        let action = review(ReviewEvent::Comment, true);
        let expected = preview(&action, pr()).await;
        let mut steps = reads(&action, pr(), false, false);
        steps.last_mut().unwrap().result = Ok(
            json!([{"filename":"file.rs","status":"modified","patch":"@@ -1,3 +1,3 @@\n context\n-old\n+different text\n tail"}]),
        );
        let api = Fake::new(steps);
        assert!(
            execute_using(&gate(), &action, &expected, &action.request_hash(), &api)
                .await
                .is_err()
        );
        api.exhausted();
    }

    #[tokio::test]
    async fn cancelling_an_inflight_write_drops_it_without_retry_or_late_verification() {
        struct Pending {
            reads: Fake,
            started: tokio::sync::Notify,
            writes: std::sync::atomic::AtomicUsize,
        }
        impl GithubApi for Pending {
            async fn request(
                &self,
                host: Option<&str>,
                method: &str,
                path: &str,
                body: Option<Value>,
            ) -> Result<Value, Error> {
                if method == "PATCH" {
                    self.writes.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    self.started.notify_one();
                    std::future::pending().await
                } else {
                    self.reads.request(host, method, path, body).await
                }
            }
        }
        let action = edit();
        let expected = preview(&action, pr()).await;
        let api = Pending {
            reads: Fake::new(reads(&action, pr(), false, false)),
            started: tokio::sync::Notify::new(),
            writes: std::sync::atomic::AtomicUsize::new(0),
        };
        let budget = gate();
        let hash = action.request_hash();
        {
            let future = execute_using(&budget, &action, &expected, &hash, &api);
            tokio::pin!(future);
            tokio::select! {_=&mut future=>panic!("write should still be pending"),_=api.started.notified()=>{}}
        }
        assert_eq!(api.writes.load(std::sync::atomic::Ordering::SeqCst), 1);
        api.reads.exhausted();
    }

    #[test]
    fn input_validation_rejects_fork_heads_duplicate_reviewers_and_bad_lines() {
        let action = PullRequestWrite::Create {
            repository: "org/repo".into(),
            base: "main".into(),
            head: "other:feature".into(),
            title: "Title".into(),
            body: "".into(),
            draft: true,
        };
        assert!(action.validate().is_err());
        let action = PullRequestWrite::RequestReviewers {
            repository: "org/repo".into(),
            number: 7,
            reviewers: vec!["Alice".into(), "alice".into()],
            team_reviewers: vec![],
        };
        assert!(action.validate().is_err());
        let mut action = review(ReviewEvent::Comment, true);
        if let PullRequestWrite::SubmitReview { comments, .. } = &mut action {
            comments[0].line = 0;
        }
        assert!(action.validate().is_err());
    }
}
