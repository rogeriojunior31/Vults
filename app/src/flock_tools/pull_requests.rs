//! Pull request tools share Chat's human cards and the connector's gated typed service.

use super::GithubSource;
use super::{GithubToolGate, Source, failed, github_result};
use serde_json::{Value, json};
use std::sync::Arc;
use vults_agents::driver::BoxFuture;
use vults_chat::tools::{Tool, ToolApproval, Tools};
use vults_connectors::github::PullRequestWrite;
use vults_core::audit::{Act, Actor, Audit};

const OPERATIONS: &[&str] = &[
    "github_pull_request_create",
    "github_pull_request_edit",
    "github_pull_request_ready",
    "github_pull_request_draft",
    "github_pull_request_request_reviewers",
    "github_pull_request_review_comment",
    "github_pull_request_review_approve",
    "github_pull_request_review_request_changes",
];

pub(super) fn tools(source: Arc<dyn Source>) -> Tools {
    OPERATIONS
        .iter()
        .map(|operation| {
            Arc::new(PullRequestTool {
                source: source.clone(),
                operation,
            }) as Arc<dyn Tool>
        })
        .collect()
}

pub(super) fn prepare(
    app: GithubSource,
    action: PullRequestWrite,
) -> BoxFuture<'static, Result<Value, String>> {
    Box::pin(async move {
        if !app.github_work_item_allowed() {
            return Err("Turn on GitHub tools for Zeca and the GitHub connector first.".into());
        }
        let gate = GithubToolGate::new(app.clone());
        let preview = vults_connectors::github::prepare_pull_request_write(&gate, &action)
            .await
            .map_err(|error| error.to_string())?;
        if !app.github_work_item_allowed() {
            return Err(
                "GitHub tools were disabled while the pull request proposal was being prepared.".into(),
            );
        }
        Ok(preview)
    })
}

struct WriteAudit<'a> {
    subject: String,
    action: &'a PullRequestWrite,
    hash: &'a str,
    preview: &'a Value,
    outcome: &'static str,
}
impl Drop for WriteAudit<'_> {
    fn drop(&mut self) {
        crate::audit::keep(Audit::new(
            Actor::System,
            Act::ExternalWrite,
            &self.subject,
            self.action.repository(),
            self.action.tool_name(),
            &audit_target(self.action, self.preview, self.hash, self.outcome),
        ));
    }
}

fn audit_target(action: &PullRequestWrite, preview: &Value, hash: &str, outcome: &str) -> String {
    let refs = if preview["pullRequest"].is_null() {
        &preview["proposal"]
    } else {
        &preview["pullRequest"]
    };
    json!({"number":action.number(),"account":preview["source"]["account"],"host":preview["source"]["host"],"base":refs["base"],"head":refs["head"],"proposedBase":preview["proposal"]["base"],"requestHash":hash,"outcome":outcome}).to_string()
}

pub(super) fn execute(
    app: GithubSource,
    action: PullRequestWrite,
    preview: Value,
    hash: String,
) -> BoxFuture<'static, Result<Value, String>> {
    Box::pin(async move {
        let mut audit = WriteAudit {
            subject: app.subject().into(),
            action: &action,
            hash: &hash,
            preview: &preview,
            outcome: "cancelled_or_uncertain",
        };
        if !app.github_work_item_allowed() {
            audit.outcome = "denied";
            return Err("Turn on GitHub tools for Zeca and the GitHub connector first.".into());
        }
        let gate = GithubToolGate::new(app);
        let result = vults_connectors::github::execute_pull_request_write(&gate, &action, &preview, &hash)
            .await
            .map_err(|error| error.to_string());
        audit.outcome = if result.is_ok() {
            "verified"
        } else {
            "failed_or_uncertain"
        };
        result
    })
}

#[derive(Debug)]
struct PullRequestTool {
    source: Arc<dyn Source>,
    operation: &'static str,
}
impl Tool for PullRequestTool {
    fn name(&self) -> &str {
        self.operation
    }
    fn description(&self) -> &str {
        match self.operation {
            "github_pull_request_create" => {
                "Proposes creating a pull request or draft between two branches in the same repository. Fork heads are not supported. The human card includes the exact title, body, refs and commit SHAs."
            }
            "github_pull_request_edit" => {
                "Proposes changing a pull request title, body or base branch. Requires a human card bound to its current base, head and commit SHAs."
            }
            "github_pull_request_ready" => {
                "Proposes marking a draft pull request ready for review. This may notify code owners. Always requires the user's exact approval card."
            }
            "github_pull_request_draft" => {
                "Proposes converting an open pull request to a draft. Always requires the user's exact approval card."
            }
            "github_pull_request_request_reviewers" => {
                "Proposes requesting reviews from the listed user logins and team slugs. List only the exact recipients to notify; use empty arrays for unused recipient types. Always requires a human card."
            }
            "github_pull_request_review_comment" => {
                "Proposes submitting a COMMENT review with an exact body and optional single-line diff comments. Read the current diff first. Requires human approval bound to the exact head commit and valid current file/side/line anchors."
            }
            "github_pull_request_review_approve" => {
                "Proposes submitting an APPROVE review, with exact body and optional single-line diff comments. Only a direct human approval of this exact review can submit it. Conversational approval of a draft is not authorization."
            }
            _ => {
                "Proposes submitting a REQUEST_CHANGES review with an exact body and optional single-line diff comments. Only a direct human approval of this exact review can submit it. Read the current diff before choosing file/side/line anchors."
            }
        }
    }
    fn schema(&self) -> Value {
        let mut properties = json!({"repository":{"type":"string","pattern":"^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$","maxLength":200},"number":{"type":"integer","minimum":1,"maximum":2147483647}});
        let branch = json!({"type":"string","minLength":1,"maxLength":100,"description":"Exact branch in this repository; fork prefixes are not supported."});
        let title = json!({"type":"string","minLength":1,"maxLength":256});
        let body = json!({"type":"string","maxLength":10000});
        let required = match self.operation {
            "github_pull_request_create" => {
                properties.as_object_mut().expect("object").remove("number");
                properties["base"] = branch.clone();
                properties["head"] = branch;
                properties["title"] = title;
                properties["body"] = body;
                properties["draft"] = json!({"type":"boolean"});
                json!(["repository", "base", "head", "title", "body", "draft"])
            }
            "github_pull_request_edit" => {
                properties["title"] = title;
                properties["body"] = body;
                properties["base"] = branch;
                json!(["repository", "number"])
            }
            "github_pull_request_request_reviewers" => {
                properties["reviewers"] = json!({"type":"array","maxItems":10,"uniqueItems":true,"items":{"type":"string","minLength":1,"maxLength":39}});
                properties["teamReviewers"] = json!({"type":"array","maxItems":10,"uniqueItems":true,"items":{"type":"string","minLength":1,"maxLength":100}});
                json!(["repository", "number", "reviewers", "teamReviewers"])
            }
            name if name.starts_with("github_pull_request_review_") => {
                properties["body"] = body;
                properties["comments"] = json!({"type":"array","maxItems":10,"items":{"type":"object","properties":{"path":{"type":"string","minLength":1,"maxLength":1024},"line":{"type":"integer","minimum":1,"maximum":2147483647},"side":{"type":"string","enum":["LEFT","RIGHT"]},"body":{"type":"string","minLength":1,"maxLength":10000}},"required":["path","line","side","body"],"additionalProperties":false}});
                json!(["repository", "number", "body", "comments"])
            }
            _ => json!(["repository", "number"]),
        };
        json!({"type":"object","properties":properties,"required":required,"additionalProperties":false})
    }
    fn call<'a>(&'a self, _: &'a Value) -> BoxFuture<'a, Result<String, String>> {
        failed(json!({"error":"Pull request changes require the explicit human approval card."}))
    }
    fn approval<'a>(&'a self, input: &'a Value) -> Option<BoxFuture<'a, Result<ToolApproval, String>>> {
        Some(Box::pin(async move {
            let action = parse(self.operation, input)?;
            let preview = self.source.github_pull_request_prepare(action.clone()).await?;
            let hash = action.request_hash();
            let target = format!(
                "{} · {} · {} · sha256:{hash}",
                action.repository(),
                action
                    .number()
                    .map(|n| format!("PR #{n}"))
                    .unwrap_or_else(|| "new pull request".into()),
                self.operation
            );
            let pr = &preview["pullRequest"];
            let full=serde_json::to_string_pretty(&json!({"request":action,"requestHash":hash,"source":preview["source"],"current":{"number":pr["number"],"url":pr["url"],"title":pr["title"],"draft":pr["draft"],"base":pr["base"],"head":pr["head"]},"proposal":preview["proposal"]})).map_err(|error|error.to_string())?;
            Ok(ToolApproval{target,description:"Review the exact pull request change, recipients and commit. GitHub will be checked again before this runs.".into(),full,context:json!({"preview":preview,"requestHash":hash})})
        }))
    }
    fn call_approved<'a>(
        &'a self,
        input: &'a Value,
        context: &'a Value,
    ) -> BoxFuture<'a, Result<String, String>> {
        Box::pin(async move {
            let action = parse(self.operation, input)?;
            let hash = action.request_hash();
            if context["requestHash"].as_str() != Some(hash.as_str()) {
                return Err("The pull request request changed after approval. Review a new card.".into());
            }
            let value = self
                .source
                .github_pull_request_execute(action, context["preview"].clone(), hash)
                .await?;
            github_result(value)
        })
    }
    fn reads_outside(&self) -> bool {
        true
    }
    fn activity_mode(&self) -> Option<&'static str> {
        Some("live")
    }
}

fn parse(operation: &str, input: &Value) -> Result<PullRequestWrite, String> {
    let mut input = input
        .as_object()
        .cloned()
        .ok_or("Pull request arguments must be an object.")?;
    if input.contains_key("operation") || input.contains_key("event") {
        return Err("The pull request operation and review decision are fixed by the selected tool.".into());
    }
    for key in ["title", "body", "base"] {
        if input.get(key).is_some_and(|value| !value.is_string()) {
            return Err(format!("The {key} field must be text."));
        }
    }
    let kind = match operation {
        "github_pull_request_create" => "create",
        "github_pull_request_edit" => "edit",
        "github_pull_request_ready" => "ready",
        "github_pull_request_draft" => "draft",
        "github_pull_request_request_reviewers" => "request_reviewers",
        "github_pull_request_review_comment"
        | "github_pull_request_review_approve"
        | "github_pull_request_review_request_changes" => {
            input.insert(
                "event".into(),
                json!(match operation {
                    "github_pull_request_review_comment" => "COMMENT",
                    "github_pull_request_review_approve" => "APPROVE",
                    _ => "REQUEST_CHANGES",
                }),
            );
            "submit_review"
        }
        _ => return Err("Unsupported pull request change.".into()),
    };
    input.insert("operation".into(), json!(kind));
    let action: PullRequestWrite = serde_json::from_value(Value::Object(input))
        .map_err(|error| format!("Invalid pull request arguments: {error}"))?;
    action.validate().map_err(|error| error.to_string())?;
    Ok(action)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    struct Writes {
        calls: AtomicUsize,
    }
    impl Source for Writes {
        fn view(&self) -> Option<Value> {
            None
        }
        fn usage(&self) -> Value {
            Value::Null
        }
        fn github_pull_request_prepare(
            &self,
            _: PullRequestWrite,
        ) -> BoxFuture<'static, Result<Value, String>> {
            Box::pin(async {
                Ok(
                    json!({"source":{"account":"person","host":"github.com"},"pullRequest":{"number":7,"url":"https://github.com/org/repo/pull/7","title":"PR","base":{"ref":"main","sha":"a".repeat(40)},"head":{"ref":"feature","sha":"b".repeat(40)}},"proposal":{"inlineComments":[{"path":"file.rs","side":"RIGHT","line":2,"text":"current line"}]},"binding":{"internal":"huge history".repeat(5000)}}),
                )
            })
        }
        fn github_pull_request_execute(
            &self,
            _: PullRequestWrite,
            _: Value,
            _: String,
        ) -> BoxFuture<'static, Result<Value, String>> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Box::pin(async { Ok(json!({"status":"verified"})) })
        }
    }
    fn input() -> Value {
        json!({"repository":"org/repo","number":7,"body":"Exact review body","comments":[{"path":"file.rs","side":"RIGHT","line":2,"body":"Exact inline comment"}]})
    }
    #[tokio::test]
    async fn tools_are_typed_and_cannot_write_without_their_human_card() {
        let source = Arc::new(Writes {
            calls: AtomicUsize::new(0),
        });
        let tools = tools(source.clone());
        assert_eq!(tools.len(), 8);
        for tool in &tools {
            assert!(tool.reads_outside());
            assert_eq!(tool.activity_mode(), Some("live"));
            assert_eq!(tool.schema()["additionalProperties"], false);
            assert!(tool.call(&input()).await.is_err());
        }
        assert_eq!(source.calls.load(Ordering::SeqCst), 0);
        let approve = tools
            .iter()
            .find(|tool| tool.name() == "github_pull_request_review_approve")
            .unwrap();
        let prepared = approve.approval(&input()).unwrap().await.unwrap();
        assert!(prepared.full.contains("APPROVE"));
        assert!(prepared.full.contains("Exact review body"));
        assert!(prepared.full.contains("Exact inline comment"));
        assert!(prepared.full.contains(&"b".repeat(40)));
        assert!(!prepared.full.contains("huge history"));
        let mut changed = input();
        changed["body"] = json!("Changed after approval");
        assert!(approve.call_approved(&changed, &prepared.context).await.is_err());
        assert_eq!(source.calls.load(Ordering::SeqCst), 0);
        approve.call_approved(&input(), &prepared.context).await.unwrap();
        assert_eq!(source.calls.load(Ordering::SeqCst), 1);
    }
    #[test]
    fn audit_keeps_approved_account_and_commit_without_copying_review_content() {
        let action = parse("github_pull_request_review_approve", &input()).unwrap();
        let preview = json!({"source":{"account":"person","host":"github.com"},"pullRequest":{"base":{"ref":"main","sha":"a".repeat(40)},"head":{"ref":"feature","sha":"b".repeat(40)}}});
        let record = audit_target(
            &action,
            &preview,
            &action.request_hash(),
            "cancelled_or_uncertain",
        );
        let record: Value = serde_json::from_str(&record).unwrap();
        assert_eq!(record["head"]["sha"], "b".repeat(40));
        assert_eq!(record["account"], "person");
        assert_eq!(record["outcome"], "cancelled_or_uncertain");
        assert!(!record.to_string().contains("Exact review body"));
    }

    #[test]
    fn decision_and_scope_cannot_be_overridden_in_model_arguments() {
        let approve = parse("github_pull_request_review_approve", &input()).unwrap();
        let comment = parse("github_pull_request_review_comment", &input()).unwrap();
        assert_ne!(approve.request_hash(), comment.request_hash());
        for (key, value) in [
            ("event", json!("COMMENT")),
            ("operation", json!("edit")),
            ("number", json!(0)),
            ("body", Value::Null),
            ("extra", json!(true)),
        ] {
            let mut bad = input();
            bad[key] = value;
            assert!(
                parse("github_pull_request_review_approve", &bad).is_err(),
                "{key}"
            );
        }
        assert!(
            parse(
                "github_pull_request_edit",
                &json!({"repository":"org/repo","number":7,"title":null,"body":"valid"})
            )
            .is_err()
        );
        assert!(parse("github_pull_request_create",&json!({"repository":"org/repo","base":"main","head":"fork:feature","title":"Title","body":"Body","draft":true})).is_err());
    }
}
