//! Exact-head merges use GitHub's non-bypassing endpoint; deleting a branch is a separate card.

use super::{
    Digest, GhApi, GithubApi, Sha256, checked_target, gated, github_host, plain_ref, url_encode_path,
    write_repository,
};
use crate::{Error, Gate};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MergeMethod {
    Merge,
    Squash,
    Rebase,
}
impl MergeMethod {
    fn name(self) -> &'static str {
        match self {
            Self::Merge => "merge",
            Self::Squash => "squash",
            Self::Rebase => "rebase",
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(
    tag = "operation",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum MergeControl {
    Merge {
        repository: String,
        number: u32,
        head_sha: String,
        method: MergeMethod,
    },
    DeleteBranch {
        repository: String,
        number: u32,
        head_sha: String,
    },
}
impl MergeControl {
    pub fn repository(&self) -> &str {
        match self {
            Self::Merge { repository, .. } | Self::DeleteBranch { repository, .. } => repository,
        }
    }
    pub fn number(&self) -> u32 {
        match self {
            Self::Merge { number, .. } | Self::DeleteBranch { number, .. } => *number,
        }
    }
    pub fn head_sha(&self) -> &str {
        match self {
            Self::Merge { head_sha, .. } | Self::DeleteBranch { head_sha, .. } => head_sha,
        }
    }
    pub fn tool_name(&self) -> &'static str {
        match self {
            Self::Merge { .. } => "github_pull_request_merge",
            Self::DeleteBranch { .. } => "github_pull_request_delete_branch",
        }
    }
    pub fn validate(&self) -> Result<(), Error> {
        checked_target(self.repository(), self.number())?;
        if self.head_sha().len() != 40
            || !exact_sha(self.head_sha())
            || self.head_sha().bytes().any(|c| c.is_ascii_uppercase())
        {
            return invalid("Give the exact lowercase 40-digit pull request head SHA.");
        }
        Ok(())
    }
    pub fn request_hash(&self) -> String {
        Sha256::digest(serde_json::to_vec(self).expect("serializable merge control"))
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect()
    }
    pub fn control_key(&self, preview: &Value) -> Result<String, Error> {
        let host = preview["source"]["host"]
            .as_str()
            .ok_or_else(|| Error::Other("The approved host is missing.".into()))?;
        let id = preview["binding"]["repositoryId"]
            .as_u64()
            .filter(|n| *n > 0)
            .ok_or_else(|| Error::Other("The approved repository identity is missing.".into()))?;
        Ok(format!(
            "{}/{id}/pr/{}/{}/{}",
            host.to_ascii_lowercase(),
            self.number(),
            self.head_sha(),
            self.tool_name()
        ))
    }
}
fn exact_sha(value: &str) -> bool {
    value.len() == 40
        && value
            .bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
}
fn invalid<T>(message: &str) -> Result<T, Error> {
    Err(Error::Other(message.into()))
}

// The rollup and review decision include GitHub's evaluation; raw rules are also bound to the card.
const QUERY: &str = r#"query($owner:String!,$name:String!,$number:Int!){repository(owner:$owner,name:$name){databaseId nameWithOwner defaultBranchRef{name} mergeCommitAllowed squashMergeAllowed rebaseMergeAllowed pullRequest(number:$number){id number url state isDraft merged headRefOid baseRefOid headRefName baseRefName headRepository{databaseId nameWithOwner} baseRef{branchProtectionRule{id}} headRef{branchProtectionRule{id}} mergeable mergeStateStatus reviewDecision mergeCommit{oid} mergeQueueEntry{id} stack{id} statusCheckRollup{state contexts(first:100){totalCount pageInfo{hasNextPage} nodes{__typename ... on CheckRun{name status conclusion} ... on StatusContext{context state}}}} reviewThreads(first:100){pageInfo{hasNextPage} nodes{isResolved}}}}}"#;

async fn context(
    gate: &dyn Gate,
    api: &impl GithubApi,
    action: &MergeControl,
    host: Option<&str>,
) -> Result<Value, Error> {
    let repo = write_repository(gate, api, action.repository(), host).await?;
    let host = repo.source["host"].as_str().expect("validated host");
    let (owner, name) = checked_target(action.repository(), action.number())?;
    let response = gated(
        gate,
        api.request(
            Some(host),
            "POST",
            "graphql",
            Some(json!({"query":QUERY,"variables":{"owner":owner,"name":name,"number":action.number()}})),
        ),
    )
    .await?;
    if response
        .get("errors")
        .is_some_and(|e| !e.as_array().is_some_and(Vec::is_empty))
    {
        return invalid("GitHub returned incomplete merge readiness. Inspect the pull request on GitHub.");
    }
    let repository = &response["data"]["repository"];
    let pr = &repository["pullRequest"];
    if repository["databaseId"] != repo.binding["repositoryId"]
        || !repository["nameWithOwner"]
            .as_str()
            .is_some_and(|v| v.eq_ignore_ascii_case(action.repository()))
        || pr["number"].as_u64() != Some(u64::from(action.number()))
        || !pr["id"].is_string()
        || !pr["merged"].is_boolean()
        || !pr["isDraft"].is_boolean()
        || !pr["headRefOid"].as_str().is_some_and(exact_sha)
        || !pr["baseRefOid"].as_str().is_some_and(exact_sha)
        || !pr["headRefName"].as_str().is_some_and(plain_ref)
        || !pr["baseRefName"].as_str().is_some_and(plain_ref)
        || !pr["url"].as_str().is_some_and(|v| {
            v.eq_ignore_ascii_case(&format!(
                "https://{host}/{}/pull/{}",
                action.repository(),
                action.number()
            ))
        })
        || !pr["headRepository"]["databaseId"].as_u64().is_some_and(|v| v > 0)
        || !matches!(pr["state"].as_str(), Some("OPEN" | "CLOSED" | "MERGED"))
    {
        return invalid("GitHub did not return the exact pull request, repository and refs.");
    }
    let mut binding = repo.binding;
    binding["pullRequest"] = json!({"id":pr["id"],"number":pr["number"],"headSha":pr["headRefOid"],"baseSha":pr["baseRefOid"],"head":pr["headRefName"],"base":pr["baseRefName"],"headRepositoryId":pr["headRepository"]["databaseId"]});
    Ok(json!({"source":repo.source,"binding":binding,"pullRequest":pr,"repository":repository}))
}

async fn rules(
    gate: &dyn Gate,
    api: &impl GithubApi,
    host: &str,
    repo: &str,
    branch: &str,
) -> Result<Vec<Value>, Error> {
    let mut rules = Vec::new();
    for page in 1..=10 {
        let value = gated(
            gate,
            api.request(
                Some(host),
                "GET",
                &format!(
                    "repos/{repo}/rules/branches/{}?per_page=100&page={page}",
                    url_encode_path(branch)
                ),
                None,
            ),
        )
        .await?;
        let items = value
            .as_array()
            .filter(|a| a.len() <= 100)
            .ok_or_else(|| Error::Partial("GitHub omitted branch-rule coverage.".into()))?;
        if items.iter().any(|r| !r["type"].is_string()) {
            return invalid("GitHub returned incomplete branch rules.");
        }
        rules.extend(items.iter().cloned());
        if items.len() < 100 {
            return Ok(rules);
        }
    }
    invalid("Branch-rule coverage exceeds 1,000 rules. Inspect GitHub instead.")
}
async fn inspect(
    gate: &dyn Gate,
    api: &impl GithubApi,
    action: &MergeControl,
    host: Option<&str>,
) -> Result<Value, Error> {
    let mut current = context(gate, api, action, host).await?;
    let host = current["source"]["host"].as_str().expect("host");
    let pr = &current["pullRequest"];
    if pr["headRefOid"] != action.head_sha() {
        return invalid("The pull request head changed. Read the latest head and review a new card.");
    }
    let deleting = matches!(action, MergeControl::DeleteBranch { .. });
    let branch = pr[if deleting { "headRefName" } else { "baseRefName" }]
        .as_str()
        .expect("ref");
    if pr["headRepository"]["databaseId"] != current["binding"]["repositoryId"] {
        return invalid("Fork branches are not supported by these merge controls.");
    }
    if deleting {
        if !current["repository"]["defaultBranchRef"]["name"]
            .as_str()
            .is_some_and(plain_ref)
        {
            return invalid("The default branch could not be checked.");
        }
        if pr["state"] != "MERGED" || pr["merged"] != true {
            return invalid("The exact pull request must be merged before proposing branch deletion.");
        }
        if current["repository"]["defaultBranchRef"]["name"] == branch || pr["baseRefName"] == branch {
            return invalid("The default or base branch cannot be deleted.");
        }
    } else {
        if pr["state"] != "OPEN" || pr["merged"] != false || pr["isDraft"] != false {
            return invalid("This pull request is already merged, closed or a draft.");
        }
        if !pr.get("stack").is_some_and(Value::is_null) {
            return invalid("Stacked pull requests require a separate multi-PR scope and are not supported.");
        }
        if !pr.get("mergeQueueEntry").is_some_and(Value::is_null) {
            return invalid(
                "This pull request is already queued. Read its state instead of submitting another merge.",
            );
        }
        if pr["mergeable"] != "MERGEABLE" || pr["mergeStateStatus"] != "CLEAN" {
            return invalid(
                "The pull request is conflicted, blocked, behind or its readiness is unknown. Resolve it on GitHub first.",
            );
        }
        if !pr
            .get("reviewDecision")
            .is_some_and(|v| v.is_null() || v == "APPROVED")
        {
            return invalid("Required reviews are missing or changes were requested.");
        }
        let rollup = &pr["statusCheckRollup"];
        if !pr
            .get("statusCheckRollup")
            .is_some_and(|v| v.is_null() || v["state"] == "SUCCESS")
        {
            return invalid("Checks are pending, failing or unknown. Wait for a successful check rollup.");
        }
        if !rollup.is_null() {
            let checks = &rollup["contexts"];
            if checks["pageInfo"]["hasNextPage"] != false
                || checks["totalCount"].as_u64() != checks["nodes"].as_array().map(|a| a.len() as u64)
            {
                return invalid("Check coverage is incomplete. Inspect GitHub before merging.");
            }
        }
        let threads = &pr["reviewThreads"];
        if threads["pageInfo"]["hasNextPage"] != false
            || !threads["nodes"]
                .as_array()
                .is_some_and(|a| a.iter().all(|t| t["isResolved"] == true))
        {
            return invalid("Review discussions are unresolved or their coverage is incomplete.");
        }
    }
    let branch_info = gated(
        gate,
        api.request(
            Some(host),
            "GET",
            &format!(
                "repos/{}/branches/{}",
                action.repository(),
                url_encode_path(branch)
            ),
            None,
        ),
    )
    .await?;
    if branch_info["name"] != branch
        || !branch_info["protected"].is_boolean()
        || !branch_info["commit"]["sha"].as_str().is_some_and(exact_sha)
    {
        return invalid("GitHub omitted the exact branch and its protection status.");
    }
    if branch_info["commit"]["sha"] != pr[if deleting { "headRefOid" } else { "baseRefOid" }] {
        return invalid("The branch commit changed while preparing the control. Read it again.");
    }
    let active = rules(gate, api, host, action.repository(), branch).await?;
    let classic_ref = &pr[if deleting { "headRef" } else { "baseRef" }];
    if !classic_ref
        .get("branchProtectionRule")
        .is_some_and(|v| v.is_null() || v["id"].is_string())
    {
        return invalid("Classic branch-rule coverage is missing.");
    }
    let has_classic = !classic_ref["branchProtectionRule"].is_null();
    if branch_info["protected"] == true && !has_classic && active.is_empty() {
        return invalid("The protected branch rules could not be inspected.");
    }
    if branch_info["protected"] == false && has_classic {
        return invalid("Branch protection changed during inspection.");
    }
    let protection = if has_classic {
        // A 404 here is ambiguous (permissions or rulesets); fail closed rather than infer no classic rule.
        gated(
            gate,
            api.request(
                Some(host),
                "GET",
                &format!(
                    "repos/{}/branches/{}/protection",
                    action.repository(),
                    url_encode_path(branch)
                ),
                None,
            ),
        )
        .await?
    } else {
        Value::Null
    };
    if has_classic && !protection.is_object() {
        return invalid("Classic branch protection could not be inspected.");
    }
    let queue = active.iter().find(|r| r["type"] == "merge_queue");
    let mode = if deleting {
        if branch_info["protected"] != false || !active.is_empty() {
            return invalid(
                "Only an unprotected branch with no active rules can be deleted through this tool.",
            );
        }
        "delete_branch"
    } else {
        let MergeControl::Merge { method, .. } = action else {
            unreachable!()
        };
        let allowed = match method {
            MergeMethod::Merge => "mergeCommitAllowed",
            MergeMethod::Squash => "squashMergeAllowed",
            MergeMethod::Rebase => "rebaseMergeAllowed",
        };
        if current["repository"][allowed] != true {
            return invalid("The selected merge method is not enabled for this repository.");
        }
        if queue.is_some_and(|r| {
            !r["parameters"]["merge_method"]
                .as_str()
                .is_some_and(|m| m.eq_ignore_ascii_case(method.name()))
        }) {
            return invalid("Select the merge method configured by the required merge queue.");
        }
        let required_checks = active
            .iter()
            .any(|r| matches!(r["type"].as_str(), Some("required_status_checks" | "workflows")))
            || !protection["required_status_checks"].is_null();
        let required_reviews = active.iter().any(|r| r["type"] == "pull_request")
            || !protection["required_pull_request_reviews"].is_null();
        if required_checks && pr["statusCheckRollup"].is_null() {
            return invalid("Required checks have not reported a successful result.");
        }
        if required_reviews && pr["reviewDecision"] != "APPROVED" {
            return invalid("Required reviews are not approved.");
        }
        if queue.is_some() {
            "merge_queue"
        } else {
            "direct_merge"
        }
    };
    let scope = json!({"mode":mode,"branch":branch,"rules":active,"classicProtection":protection,"bypassRules":false,"deleteBranch":deleting});
    current["binding"]["scope"] = scope.clone();
    current["scope"] = scope;
    // Keep the card bounded to relevant metadata; no duplicate repository/pull request subtree.
    current.as_object_mut().expect("object").remove("repository");
    Ok(current)
}

fn valid_request_id(value: &str) -> bool {
    value.len() == 36
        && value.bytes().enumerate().all(|(i, c)| {
            if [8, 13, 18, 23].contains(&i) {
                c == b'-'
            } else {
                c.is_ascii_hexdigit()
            }
        })
}
/// Read-only reconciliation never submits or retries a merge.
pub async fn merge_status(
    gate: &dyn Gate,
    repository: &str,
    number: u32,
    request_id: Option<&str>,
) -> Result<Value, Error> {
    checked_target(repository, number)?;
    if request_id.is_some_and(|id| !valid_request_id(id)) {
        return invalid("Give the exact asynchronous merge request UUID.");
    }
    let action = MergeControl::Merge {
        repository: repository.into(),
        number,
        head_sha: "0".repeat(40),
        method: MergeMethod::Squash,
    };
    let current = context(gate, &GhApi, &action, None).await?;
    let host = current["source"]["host"].as_str().expect("host");
    let asynchronous = if let Some(id) = request_id {
        gated(
            gate,
            GhApi.request(
                Some(host),
                "GET",
                &format!("repos/{repository}/pulls/{number}/merge-async/{id}"),
                None,
            ),
        )
        .await?
    } else {
        Value::Null
    };
    let pr = &current["pullRequest"];
    let status = if pr["merged"] == true && pr["state"] == "MERGED" {
        "merged"
    } else if pr["mergeQueueEntry"]["id"].is_string() {
        "queued"
    } else if asynchronous["status"] == "failed" {
        "rejected"
    } else if asynchronous["status"] == "pending" {
        "pending"
    } else if pr["state"] == "CLOSED" {
        "closed"
    } else {
        "open"
    };
    Ok(
        json!({"source":current["source"],"pullRequest":pr,"asyncRequestId":request_id,"asyncResult":asynchronous,"status":status,"mutationSent":false}),
    )
}

pub async fn prepare_merge_control(gate: &dyn Gate, action: &MergeControl) -> Result<Value, Error> {
    action.validate()?;
    inspect(gate, &GhApi, action, None).await
}
pub async fn execute_merge_control<F, Fut>(
    gate: &dyn Gate,
    action: &MergeControl,
    expected: &Value,
    hash: &str,
    admit: F,
) -> Result<Value, Error>
where
    F: FnOnce() -> Fut + Send,
    Fut: std::future::Future<Output = Result<(), Error>> + Send,
{
    execute_using(gate, &GhApi, action, expected, hash, admit).await
}
async fn execute_using<F, Fut>(
    gate: &dyn Gate,
    api: &impl GithubApi,
    action: &MergeControl,
    expected: &Value,
    hash: &str,
    admit: F,
) -> Result<Value, Error>
where
    F: FnOnce() -> Fut + Send,
    Fut: std::future::Future<Output = Result<(), Error>> + Send,
{
    action.validate()?;
    if action.request_hash() != hash {
        return invalid("The merge control changed after approval. Review a new card.");
    }
    let host = expected["source"]["host"]
        .as_str()
        .filter(|h| github_host(&format!("https://{h}")).is_some())
        .ok_or_else(|| Error::Other("The approved host is missing.".into()))?;
    let before = inspect(gate, api, action, Some(host)).await?;
    if before["binding"] != expected["binding"] {
        return invalid("The account, refs, rules or scope changed after approval. Review a new card.");
    }
    let request = super::Request::begin(gate)?;
    admit().await?;
    let (method, path, body) = match action {
        MergeControl::Merge { method, .. } => (
            "PUT",
            format!(
                "repos/{}/pulls/{}/merge-async",
                action.repository(),
                action.number()
            ),
            Some(
                json!({"sha":action.head_sha(),"merge_method":method.name(),"merge_action":before["scope"]["mode"],"bypass_rules":false}),
            ),
        ),
        MergeControl::DeleteBranch { .. } => (
            "DELETE",
            format!(
                "repos/{}/git/refs/heads/{}",
                action.repository(),
                url_encode_path(before["scope"]["branch"].as_str().expect("branch"))
            ),
            None,
        ),
    };
    let sent = api.request(Some(host), method, &path, body).await;
    request.finish(sent.is_ok());
    let accepted = sent.is_ok();
    let request_id = sent
        .as_ref()
        .ok()
        .and_then(|v| v["details"]["uuid"].as_str())
        .filter(|id| valid_request_id(id))
        .map(str::to_owned);
    let rejected = sent
        .as_ref()
        .err()
        .is_some_and(super::actions::definitely_rejected);
    let after = match context(gate, api, action, Some(host)).await {
        Ok(value) => value,
        Err(error) => {
            return Ok(
                json!({"status":if rejected{"rejected"}else{"uncertain"},"verified":false,"accepted":accepted,"observationFresh":false,"asyncRequestId":request_id,"source":before["source"],"message":format!("The remote result could not be checked: {}. Inspect GitHub before retrying.",vults_core::redact::secrets(&error.to_string()))}),
            );
        }
    };
    let mut status = "uncertain";
    let identity_ok = ["identity", "repositoryId", "repository", "url"]
        .iter()
        .all(|k| before["binding"][*k] == after["binding"][*k]);
    let same_pr = ["id", "number", "headSha", "head", "base", "headRepositoryId"]
        .iter()
        .all(|k| before["binding"]["pullRequest"][*k] == after["binding"]["pullRequest"][*k]);
    let mut verification = Value::Null;
    if identity_ok && same_pr {
        match action {
            MergeControl::Merge { .. } => {
                let pr = &after["pullRequest"];
                if pr["merged"] == true
                    && pr["state"] == "MERGED"
                    && pr["mergeCommit"]["oid"].as_str().is_some_and(exact_sha)
                {
                    status = "merged";
                } else if pr["merged"] == false
                    && pr["state"] == "OPEN"
                    && pr["mergeQueueEntry"]["id"].is_string()
                {
                    status = "queued";
                } else if rejected {
                    status = "rejected";
                } else if let Ok(response) = &sent
                    && response["status"] == "pending"
                    && let Some(uuid) = response["details"]["uuid"]
                        .as_str()
                        .filter(|v| valid_request_id(v))
                {
                    let outcome = gated(
                        gate,
                        api.request(
                            Some(host),
                            "GET",
                            &format!(
                                "repos/{}/pulls/{}/merge-async/{uuid}",
                                action.repository(),
                                action.number()
                            ),
                            None,
                        ),
                    )
                    .await;
                    if let Ok(value) = outcome {
                        if value["status"] == "failed" {
                            status = "rejected";
                        } else if value["status"] == "pending"
                            && value["details"]["expected_head_sha"] == action.head_sha()
                            && value["details"]["bypass_rules"] == false
                            && value["details"]["merge_action"] == before["scope"]["mode"]
                            && matches!(action, MergeControl::Merge { method, .. } if value["details"]["merge_method"] == method.name())
                        {
                            status = "accepted_pending_verification";
                        }
                        verification = value;
                    }
                }
            }
            MergeControl::DeleteBranch { .. } => {
                let path = format!(
                    "repos/{}/git/matching-refs/heads/{}",
                    action.repository(),
                    url_encode_path(before["scope"]["branch"].as_str().expect("branch"))
                );
                if let Ok(value) = gated(gate, api.request(Some(host), "GET", &path, None)).await
                    && let Some(refs) = value.as_array().filter(|a| a.len() < 100)
                {
                    let exact = format!(
                        "refs/heads/{}",
                        before["scope"]["branch"].as_str().expect("branch")
                    );
                    if refs.iter().all(|r| r["ref"].as_str().is_some())
                        && !refs.iter().any(|r| r["ref"] == exact)
                    {
                        status = "branch_deleted";
                    } else if rejected {
                        status = "rejected";
                    }
                }
            }
        }
    }
    Ok(
        json!({"status":status,"verified":matches!(status,"merged"|"queued"|"branch_deleted"),"accepted":accepted,"source":after["source"],"observationFresh":true,"pullRequest":after["pullRequest"],"asyncRequestId":request_id,"asyncResult":verification,"message":match status {"merged"=>"The exact pull request is merged. Its branch was not deleted by this control.","queued"=>"The exact pull request is in the merge queue, not merged. Its branch was kept.","branch_deleted"=>"The exact merged pull request branch is absent.","rejected"=>"The control was rejected. Inspect GitHub before proposing a new control.","accepted_pending_verification"=>"The merge request is pending, not merged. Read its result; do not resend.",_=>"The result is uncertain. Inspect GitHub before retrying; no mutation was repeated."}}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{collections::VecDeque, sync::Mutex, time::Duration};
    type Step = (String, String, Result<Value, Error>);
    struct Fake {
        steps: Mutex<VecDeque<Step>>,
        writes: Mutex<Vec<Value>>,
    }
    impl Fake {
        fn new(steps: Vec<(&str, String, Result<Value, Error>)>) -> Self {
            Self {
                steps: Mutex::new(steps.into_iter().map(|(m, p, v)| (m.into(), p, v)).collect()),
                writes: Mutex::new(Vec::new()),
            }
        }
        fn exhausted(&self) {
            assert!(self.steps.lock().unwrap().is_empty(), "expected request not sent");
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
            assert!(host.is_none() || host == Some("github.com"));
            let (m, p, v) = self
                .steps
                .lock()
                .unwrap()
                .pop_front()
                .expect("unexpected request or repeated mutation");
            assert_eq!((method, path), (m.as_str(), p.as_str()));
            if method == "POST" && path == "graphql" {
                let body = body.unwrap();
                assert_eq!(body["variables"], json!({"owner":"org","name":"repo","number":7}));
                assert_eq!(body["query"], QUERY);
            } else if method != "GET" {
                self.writes
                    .lock()
                    .unwrap()
                    .push(json!({"method":method,"path":path,"body":body}));
            }
            v
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
    fn action() -> MergeControl {
        MergeControl::Merge {
            repository: "org/repo".into(),
            number: 7,
            head_sha: "b".repeat(40),
            method: MergeMethod::Squash,
        }
    }
    fn delete() -> MergeControl {
        MergeControl::DeleteBranch {
            repository: "org/repo".into(),
            number: 7,
            head_sha: "b".repeat(40),
        }
    }
    fn pr() -> Value {
        json!({"id":"PR_7","number":7,"url":"https://github.com/org/repo/pull/7","state":"OPEN","isDraft":false,"merged":false,"headRefOid":"b".repeat(40),"baseRefOid":"a".repeat(40),"headRefName":"feature","baseRefName":"main","headRepository":{"databaseId":20,"nameWithOwner":"org/repo"},"baseRef":{"branchProtectionRule":null},"headRef":{"branchProtectionRule":null},"mergeable":"MERGEABLE","mergeStateStatus":"CLEAN","reviewDecision":null,"mergeCommit":null,"mergeQueueEntry":null,"stack":null,"statusCheckRollup":{"state":"SUCCESS","contexts":{"totalCount":1,"pageInfo":{"hasNextPage":false},"nodes":[{"__typename":"CheckRun","name":"ci","status":"COMPLETED","conclusion":"SUCCESS"}]}},"reviewThreads":{"pageInfo":{"hasNextPage":false},"nodes":[]}})
    }
    fn merged() -> Value {
        let mut p = pr();
        p["state"] = json!("MERGED");
        p["merged"] = json!(true);
        p["mergeCommit"] = json!({"oid":"c".repeat(40)});
        p
    }
    fn context(pr: Value) -> Vec<(&'static str, String, Result<Value, Error>)> {
        vec![
            (
                "GET",
                "user".into(),
                Ok(json!({"id":1,"login":"person","html_url":"https://github.com/person"})),
            ),
            (
                "GET",
                "repos/org/repo".into(),
                Ok(
                    json!({"id":20,"full_name":"org/repo","html_url":"https://github.com/org/repo","permissions":{"push":true},"archived":false}),
                ),
            ),
            (
                "POST",
                "graphql".into(),
                Ok(
                    json!({"data":{"repository":{"databaseId":20,"nameWithOwner":"org/repo","defaultBranchRef":{"name":"main"},"mergeCommitAllowed":true,"squashMergeAllowed":true,"rebaseMergeAllowed":true,"pullRequest":pr}}}),
                ),
            ),
        ]
    }
    fn inspection(
        p: Value,
        active: Value,
        deleting: bool,
    ) -> Vec<(&'static str, String, Result<Value, Error>)> {
        let classic = !p[if deleting { "headRef" } else { "baseRef" }]["branchProtectionRule"].is_null();
        let branch = if deleting { "feature" } else { "main" };
        let sha = p[if deleting { "headRefOid" } else { "baseRefOid" }].clone();
        let protected = classic || !active.as_array().unwrap().is_empty();
        let mut steps = context(p);
        steps.push((
            "GET",
            format!("repos/org/repo/branches/{branch}"),
            Ok(json!({"name":branch,"protected":protected,"commit":{"sha":sha}})),
        ));
        steps.push((
            "GET",
            format!("repos/org/repo/rules/branches/{branch}?per_page=100&page=1"),
            Ok(active),
        ));
        if classic {
            steps.push((
                "GET",
                format!("repos/org/repo/branches/{branch}/protection"),
                Ok(json!({"required_pull_request_reviews":{"required_approving_review_count":1}})),
            ));
        }
        steps
    }
    async fn preview(a: &MergeControl, p: Value, rules: Value) -> Value {
        let api = Fake::new(inspection(
            p,
            rules,
            matches!(a, MergeControl::DeleteBranch { .. }),
        ));
        let value = inspect(&gate(), &api, a, None).await.unwrap();
        api.exhausted();
        value
    }
    fn merge_post(result: Result<Value, Error>) -> (&'static str, String, Result<Value, Error>) {
        ("PUT", "repos/org/repo/pulls/7/merge-async".into(), result)
    }
    #[tokio::test]
    async fn each_merge_method_uses_exact_sha_and_never_bypasses_or_deletes() {
        for method in [MergeMethod::Merge, MergeMethod::Squash, MergeMethod::Rebase] {
            let a = MergeControl::Merge {
                repository: "org/repo".into(),
                number: 7,
                head_sha: "b".repeat(40),
                method,
            };
            let expected = preview(&a, pr(), json!([])).await;
            let mut steps = inspection(pr(), json!([]), false);
            steps.push(merge_post(Ok(json!({"status":"merged"}))));
            steps.extend(context(merged()));
            let api = Fake::new(steps);
            let result = execute_using(&gate(), &api, &a, &expected, &a.request_hash(), || async {
                Ok(())
            })
            .await
            .unwrap();
            assert_eq!(result["status"], "merged");
            assert_eq!(result["verified"], true);
            let writes = api.writes.lock().unwrap();
            assert_eq!(writes.len(), 1);
            assert_eq!(
                writes[0]["body"],
                json!({"sha":"b".repeat(40),"merge_method":method.name(),"merge_action":"direct_merge","bypass_rules":false})
            );
            api.exhausted();
        }
    }
    #[tokio::test]
    async fn queue_uses_configured_method_and_never_claims_it_merged() {
        let a = action();
        let rules = json!([{"type":"merge_queue","parameters":{"merge_method":"SQUASH"}}]);
        let expected = preview(&a, pr(), rules.clone()).await;
        let mut queued = pr();
        queued["mergeQueueEntry"] = json!({"id":"QUEUE_1"});
        let mut steps = inspection(pr(), rules, false);
        steps.push(merge_post(Ok(json!({"status":"enqueued"}))));
        steps.extend(context(queued));
        let api = Fake::new(steps);
        let result = execute_using(&gate(), &api, &a, &expected, &a.request_hash(), || async {
            Ok(())
        })
        .await
        .unwrap();
        assert_eq!(result["status"], "queued");
        assert_eq!(result["pullRequest"]["merged"], false);
        assert_eq!(
            api.writes.lock().unwrap()[0]["body"]["merge_action"],
            "merge_queue"
        );
        api.exhausted();
        let api = Fake::new(inspection(
            pr(),
            json!([{"type":"merge_queue","parameters":{"merge_method":"REBASE"}}]),
            false,
        ));
        assert!(inspect(&gate(), &api, &a, None).await.is_err());
        api.exhausted();
    }
    #[tokio::test]
    async fn stale_unsafe_partial_and_already_finished_targets_never_reach_mutation() {
        for change in [
            "head",
            "conflict",
            "behind",
            "pending",
            "failing",
            "review",
            "changes",
            "draft",
            "merged",
            "queued",
            "stack",
            "threads",
            "check-coverage",
            "fork",
            "missing-state",
        ] {
            let mut p = pr();
            match change {
                "head" => p["headRefOid"] = json!("d".repeat(40)),
                "conflict" => p["mergeable"] = json!("CONFLICTING"),
                "behind" => p["mergeStateStatus"] = json!("BEHIND"),
                "pending" => p["statusCheckRollup"]["state"] = json!("PENDING"),
                "failing" => p["statusCheckRollup"]["state"] = json!("FAILURE"),
                "review" => p["reviewDecision"] = json!("REVIEW_REQUIRED"),
                "changes" => p["reviewDecision"] = json!("CHANGES_REQUESTED"),
                "draft" => p["isDraft"] = json!(true),
                "merged" => p = merged(),
                "queued" => p["mergeQueueEntry"] = json!({"id":"queued"}),
                "stack" => p["stack"] = json!({"id":"STACK"}),
                "threads" => p["reviewThreads"]["nodes"] = json!([{"isResolved":false}]),
                "check-coverage" => {
                    p["statusCheckRollup"]["contexts"]["pageInfo"]["hasNextPage"] = json!(true)
                }
                "fork" => p["headRepository"]["databaseId"] = json!(21),
                _ => {
                    p.as_object_mut().unwrap().remove("mergeStateStatus");
                }
            }
            let api = Fake::new(context(p));
            assert!(inspect(&gate(), &api, &action(), None).await.is_err(), "{change}");
            assert!(api.writes.lock().unwrap().is_empty());
            api.exhausted();
        }
    }
    #[tokio::test]
    async fn changed_account_base_rules_and_request_need_a_new_card() {
        let a = action();
        let expected = preview(&a, pr(), json!([])).await;
        for changed in ["account", "base", "rules"] {
            let mut p = pr();
            if changed == "base" {
                p["baseRefOid"] = json!("d".repeat(40));
            }
            let mut steps = inspection(
                p,
                if changed == "rules" {
                    json!([{"type":"non_fast_forward"}])
                } else {
                    json!([])
                },
                false,
            );
            if changed == "account" {
                steps[0].2.as_mut().unwrap()["id"] = json!(2);
            }
            let api = Fake::new(steps);
            assert!(
                execute_using(&gate(), &api, &a, &expected, &a.request_hash(), || async {
                    panic!("stale card must not reserve")
                })
                .await
                .is_err()
            );
            assert!(api.writes.lock().unwrap().is_empty());
            api.exhausted();
        }
        let api = Fake::new(vec![]);
        assert!(
            execute_using(&gate(), &api, &a, &expected, "changed", || async {
                panic!("changed request")
            })
            .await
            .is_err()
        );
        api.exhausted();
    }
    #[tokio::test]
    async fn permissions_classic_reviews_and_lost_coverage_fail_closed() {
        let mut denied = context(pr());
        denied.truncate(2);
        denied[1].2.as_mut().unwrap()["permissions"]["push"] = json!(false);
        let api = Fake::new(denied);
        assert!(inspect(&gate(), &api, &action(), None).await.is_err());
        api.exhausted();
        let mut p = pr();
        p["baseRef"]["branchProtectionRule"] = json!({"id":"CLASSIC"});
        let api = Fake::new(inspection(p.clone(), json!([]), false));
        assert!(inspect(&gate(), &api, &action(), None).await.is_err());
        api.exhausted();
        p["reviewDecision"] = json!("APPROVED");
        let expected = preview(&action(), p.clone(), json!([])).await;
        assert!(expected["scope"]["classicProtection"].is_object());
        let mut steps = inspection(p, json!([]), false);
        steps.last_mut().unwrap().2 = Err(Error::Other("gh: forbidden (HTTP 403)".into()));
        let api = Fake::new(steps);
        assert!(inspect(&gate(), &api, &action(), None).await.is_err());
        api.exhausted();
        let api = Fake::new(inspection(pr(), json!([{"type":"pull_request"}]), false));
        assert!(inspect(&gate(), &api, &action(), None).await.is_err());
        api.exhausted();
    }
    #[tokio::test]
    async fn uncertain_timeout_reconciles_once_and_pending_does_not_mean_merged() {
        let a = action();
        let expected = preview(&a, pr(), json!([])).await;
        for (error, after, status) in [
            ("timeout", merged(), "merged"),
            ("timeout", pr(), "uncertain"),
            ("gh: forbidden (HTTP 403)", pr(), "rejected"),
        ] {
            let mut steps = inspection(pr(), json!([]), false);
            steps.push(merge_post(Err(Error::Other(error.into()))));
            steps.extend(context(after));
            let api = Fake::new(steps);
            assert_eq!(
                execute_using(&gate(), &api, &a, &expected, &a.request_hash(), || async {
                    Ok(())
                })
                .await
                .unwrap()["status"],
                status
            );
            assert_eq!(api.writes.lock().unwrap().len(), 1);
            api.exhausted();
        }
        let uuid = "12345678-1234-1234-1234-123456789abc";
        let mut steps = inspection(pr(), json!([]), false);
        steps.push(merge_post(Ok(
            json!({"status":"pending","details":{"uuid":uuid}}),
        )));
        steps.extend(context(pr()));
        steps.push(("GET",format!("repos/org/repo/pulls/7/merge-async/{uuid}"),Ok(json!({"status":"pending","details":{"expected_head_sha":"b".repeat(40),"bypass_rules":false,"merge_action":"direct_merge","merge_method":"squash"}}))));
        let api = Fake::new(steps);
        let result = execute_using(&gate(), &api, &a, &expected, &a.request_hash(), || async {
            Ok(())
        })
        .await
        .unwrap();
        assert_eq!(result["status"], "accepted_pending_verification");
        assert_eq!(result["verified"], false);
        api.exhausted();
    }
    #[tokio::test]
    async fn deletion_is_separate_and_only_an_unchanged_unprotected_merged_branch_is_removed() {
        let a = delete();
        let expected = preview(&a, merged(), json!([])).await;
        let mut steps = inspection(merged(), json!([]), true);
        steps.push((
            "DELETE",
            "repos/org/repo/git/refs/heads/feature".into(),
            Ok(Value::Null),
        ));
        steps.extend(context(merged()));
        steps.push((
            "GET",
            "repos/org/repo/git/matching-refs/heads/feature".into(),
            Ok(json!([])),
        ));
        let api = Fake::new(steps);
        assert_eq!(
            execute_using(&gate(), &api, &a, &expected, &a.request_hash(), || async {
                Ok(())
            })
            .await
            .unwrap()["status"],
            "branch_deleted"
        );
        assert_eq!(api.writes.lock().unwrap().len(), 1);
        api.exhausted();
        let api = Fake::new(context(pr()));
        assert!(inspect(&gate(), &api, &a, None).await.is_err());
        api.exhausted();
        let mut advanced = merged();
        advanced["headRefOid"] = json!("d".repeat(40));
        let api = Fake::new(context(advanced));
        assert!(inspect(&gate(), &api, &a, None).await.is_err());
        api.exhausted();
        let api = Fake::new(inspection(merged(), json!([{"type":"deletion"}]), true));
        assert!(inspect(&gate(), &api, &a, None).await.is_err());
        api.exhausted();
        let mut default = merged();
        default["headRefName"] = json!("main");
        let api = Fake::new(context(default));
        assert!(inspect(&gate(), &api, &a, None).await.is_err());
        api.exhausted();
    }
    #[tokio::test]
    async fn duplicate_admission_revocation_and_unavailable_readback_never_repeat_a_write() {
        let a = action();
        let expected = preview(&a, pr(), json!([])).await;
        let api = Fake::new(inspection(pr(), json!([]), false));
        assert!(
            execute_using(&gate(), &api, &a, &expected, &a.request_hash(), || async {
                Err(Error::Other("already reserved".into()))
            })
            .await
            .is_err()
        );
        api.exhausted();
        assert!(api.writes.lock().unwrap().is_empty());
        let api = Fake::new(inspection(pr(), json!([]), false));
        assert!(
            execute_using(
                &Budget(Mutex::new(5)),
                &api,
                &a,
                &expected,
                &a.request_hash(),
                || async { panic!("gate before admission") }
            )
            .await
            .is_err()
        );
        api.exhausted();
        let mut steps = inspection(pr(), json!([]), false);
        steps.push(merge_post(Ok(json!({"status":"pending"}))));
        let api = Fake::new(steps);
        let result = execute_using(
            &Budget(Mutex::new(6)),
            &api,
            &a,
            &expected,
            &a.request_hash(),
            || async { Ok(()) },
        )
        .await
        .unwrap();
        assert_eq!(result["status"], "uncertain");
        assert_eq!(result["observationFresh"], false);
        assert_eq!(api.writes.lock().unwrap().len(), 1);
        api.exhausted();
    }
}
