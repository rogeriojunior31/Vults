//! Chat and connector rows share gated, non-bypassing merges and durable execution receipts.
use super::{GithubToolGate, Source, failed, github_result};
use serde_json::{Value, json};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use tauri::AppHandle;
use vults_agents::driver::BoxFuture;
use vults_chat::tools::{Tool, ToolApproval, Tools};
use vults_connectors::github::MergeControl;
use vults_core::audit::{Act, Actor, Audit};

pub(super) fn tools(source: Arc<dyn Source>) -> Tools {
    ["github_pull_request_merge", "github_pull_request_delete_branch"]
        .into_iter()
        .map(|operation| {
            Arc::new(MergeTool {
                source: source.clone(),
                operation,
            }) as Arc<dyn Tool>
        })
        .collect()
}
fn record(
    action: &MergeControl,
    preview: &Value,
    hash: &str,
    key: &str,
    outcome: &str,
    receipt: bool,
) -> Audit {
    Audit::new(Actor::System,Act::ExternalWrite,"zeca",action.repository(),action.tool_name(),&json!({"controlKey":key,"receipt":receipt,"outcome":outcome,"account":preview["source"]["account"],"host":preview["source"]["host"],"repositoryId":preview["binding"]["repositoryId"],"number":action.number(),"headSha":action.head_sha(),"requestHash":hash,"scope":preview["scope"]}).to_string())
}
struct Receipt<'a> {
    action: &'a MergeControl,
    preview: &'a Value,
    hash: &'a str,
    key: &'a str,
    reserved: Arc<AtomicBool>,
    outcome: String,
}
impl Drop for Receipt<'_> {
    fn drop(&mut self) {
        crate::audit::keep(record(
            self.action,
            self.preview,
            self.hash,
            self.key,
            &self.outcome,
            self.reserved.load(Ordering::SeqCst),
        ));
    }
}
async fn ensure_available(key: String) -> Result<(), String> {
    let blocked = tauri::async_runtime::spawn_blocking(move || {
        vults_store::Store::open(&crate::history::dir())
            .and_then(|store| store.github_control_blocked(&key))
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())??;
    if blocked {
        return Err("This exact pull request control is already reserved, accepted or uncertain. Inspect GitHub instead of resending it.".into());
    }
    Ok(())
}
pub(super) fn prepare(app: AppHandle, action: MergeControl) -> BoxFuture<'static, Result<Value, String>> {
    Box::pin(async move {
        if !app.github_work_item_allowed() {
            return Err("Turn on GitHub tools for Zeca and the GitHub connector first.".into());
        }
        let preview =
            vults_connectors::github::prepare_merge_control(&GithubToolGate::new(app.clone()), &action)
                .await
                .map_err(|e| e.to_string())?;
        ensure_available(action.control_key(&preview).map_err(|e| e.to_string())?).await?;
        if !app.github_work_item_allowed() {
            return Err("GitHub tools were disabled while preparing this control.".into());
        }
        Ok(preview)
    })
}
pub(super) fn execute(
    app: AppHandle,
    action: MergeControl,
    preview: Value,
    hash: String,
) -> BoxFuture<'static, Result<Value, String>> {
    let gate = Arc::new(GithubToolGate::new(app));
    Box::pin(execute_gated(gate, action, preview, hash))
}
async fn execute_gated(
    gate: Arc<dyn vults_connectors::Gate>,
    action: MergeControl,
    preview: Value,
    hash: String,
) -> Result<Value, String> {
    let key = action.control_key(&preview).map_err(|e| e.to_string())?;
    let reserved = Arc::new(AtomicBool::new(false));
    let mut receipt = Receipt {
        action: &action,
        preview: &preview,
        hash: &hash,
        key: &key,
        reserved: reserved.clone(),
        outcome: "cancelled_or_uncertain".into(),
    };
    let reservation = record(&action, &preview, &hash, &key, "reserved", true);
    let result=vults_connectors::github::execute_merge_control(&*gate,&action,&preview,&hash,move||async move{
        let won=tauri::async_runtime::spawn_blocking(move||{
            let won=vults_store::Store::open(&crate::history::dir()).and_then(|mut s|s.reserve_github_control(crate::history::unix_now(),&reservation)).map_err(|e|e.to_string())?;
            if won {reserved.store(true,Ordering::SeqCst);}Ok::<_,String>(won)
        }).await.map_err(|e|vults_connectors::Error::Other(e.to_string()))?.map_err(vults_connectors::Error::Other)?;
        if !won {return Err(vults_connectors::Error::Other("A control for this exact pull request head is already reserved. Inspect GitHub instead of resending it.".into()));}Ok(())
    }).await.map_err(|e|e.to_string());
    receipt.outcome = match &result {
        Ok(v) => v["status"].as_str().unwrap_or("uncertain").into(),
        Err(_) if !receipt.reserved.load(Ordering::SeqCst) => "not_sent".into(),
        Err(_) => "uncertain".into(),
    };
    result
}
/// The row's explicit squash click keeps its head binding and never deletes a branch.
pub(crate) async fn merge_row(app: AppHandle, item: &str, head: &str) -> Result<Value, String> {
    let (repository, number) = item
        .strip_prefix("pr:")
        .and_then(|v| v.rsplit_once('#'))
        .ok_or("Not a pull request.")?;
    let number = number
        .parse::<u32>()
        .map_err(|_| "Invalid pull request number.")?;
    let action = MergeControl::Merge {
        repository: repository.into(),
        number,
        head_sha: head.into(),
        method: vults_connectors::github::MergeMethod::Squash,
    };
    let gate = crate::gate::shared(&app);
    let preview = vults_connectors::github::prepare_merge_control(&*gate, &action)
        .await
        .map_err(|e| e.to_string())?;
    ensure_available(action.control_key(&preview).map_err(|e| e.to_string())?).await?;
    let hash = action.request_hash();
    execute_gated(gate, action, preview, hash).await
}
#[derive(Debug)]
struct MergeTool {
    source: Arc<dyn Source>,
    operation: &'static str,
}
impl Tool for MergeTool {
    fn name(&self) -> &str {
        self.operation
    }
    fn description(&self) -> &str {
        if self.operation == "github_pull_request_merge" {
            "Proposes merging the exact PR head using merge, squash or rebase. Requires its own human card, successful checks, approved required reviews and complete branch-rule inspection. Never bypasses rules. A required queue uses its configured method; queued and pending do not mean merged. Never resend accepted or uncertain controls. Branch deletion is separate."
        } else {
            "Proposes deleting the unchanged head branch of an already merged PR in the same repository. Requires a separate human card. Default, base, protected, ruled, fork and advanced branches are refused. Never deletes as a side effect of merging."
        }
    }
    fn schema(&self) -> Value {
        let mut properties = json!({"repository":{"type":"string"},"number":{"type":"integer","minimum":1,"maximum":2147483647},"headSha":{"type":"string","pattern":"^[0-9a-f]{40}$"}});
        let mut required = vec!["repository", "number", "headSha"];
        if self.operation == "github_pull_request_merge" {
            properties["method"] = json!({"type":"string","enum":["merge","squash","rebase"]});
            required.push("method");
        }
        json!({"type":"object","properties":properties,"required":required,"additionalProperties":false})
    }
    fn call<'a>(&'a self, _: &'a Value) -> BoxFuture<'a, Result<String, String>> {
        failed(json!({"error":"Merge and branch deletion require their own explicit human approval cards."}))
    }
    fn approval<'a>(&'a self, input: &'a Value) -> Option<BoxFuture<'a, Result<ToolApproval, String>>> {
        Some(Box::pin(async move {
            let action = parse(self.operation, input)?;
            let preview = self.source.github_merge_prepare(action.clone()).await?;
            let hash = action.request_hash();
            Ok(ToolApproval{target:format!("{} · PR #{} · {} · sha256:{hash}",action.repository(),action.number(),self.operation),description:"Review the exact head, base, merge method, checks, reviews and branch rules. Branch deletion is a separate choice.".into(),full:serde_json::to_string_pretty(&json!({"request":action,"requestHash":hash,"source":preview["source"],"pullRequest":preview["pullRequest"],"scope":preview["scope"]})).map_err(|e|e.to_string())?,context:json!({"preview":preview,"requestHash":hash})})
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
                return Err("The merge control changed after approval. Review a new card.".into());
            }
            let value = self
                .source
                .github_merge_execute(action, context["preview"].clone(), hash)
                .await?;
            let failed = matches!(value["status"].as_str(), Some("uncertain" | "rejected"));
            let text = github_result(value)?;
            if failed { Err(text) } else { Ok(text) }
        })
    }
    fn reads_outside(&self) -> bool {
        true
    }
    fn activity_mode(&self) -> Option<&'static str> {
        Some("live")
    }
}
fn parse(operation: &str, input: &Value) -> Result<MergeControl, String> {
    let mut fields = input
        .as_object()
        .cloned()
        .ok_or("Merge arguments must be an object.")?;
    if fields.contains_key("operation") {
        return Err("The selected tool fixes the operation.".into());
    }
    fields.insert(
        "operation".into(),
        json!(if operation == "github_pull_request_merge" {
            "merge"
        } else if operation == "github_pull_request_delete_branch" {
            "delete_branch"
        } else {
            return Err("Unsupported merge control.".into());
        }),
    );
    let action: MergeControl = serde_json::from_value(Value::Object(fields)).map_err(|e| e.to_string())?;
    action.validate().map_err(|e| e.to_string())?;
    Ok(action)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;
    struct Writes(AtomicUsize);
    impl Source for Writes {
        fn view(&self) -> Option<Value> {
            None
        }
        fn usage(&self) -> Value {
            Value::Null
        }
        fn github_merge_prepare(&self, _: MergeControl) -> BoxFuture<'static, Result<Value, String>> {
            Box::pin(async {
                Ok(
                    json!({"source":{"host":"github.com","account":"person"},"binding":{"repositoryId":20},"pullRequest":{"id":"PR7","headRefOid":"b".repeat(40)},"scope":{"mode":"direct_merge","bypassRules":false}}),
                )
            })
        }
        fn github_merge_execute(
            &self,
            _: MergeControl,
            _: Value,
            _: String,
        ) -> BoxFuture<'static, Result<Value, String>> {
            self.0.fetch_add(1, Ordering::SeqCst);
            Box::pin(async { Ok(json!({"status":"queued","verified":true})) })
        }
    }
    fn input() -> Value {
        json!({"repository":"org/repo","number":7,"headSha":"b".repeat(40),"method":"squash"})
    }
    #[tokio::test]
    async fn merge_and_delete_are_separate_human_cards_and_changed_requests_do_not_execute() {
        let source = Arc::new(Writes(AtomicUsize::new(0)));
        let tools = tools(source.clone());
        assert_eq!(tools.len(), 2);
        for tool in tools {
            let mut input = input();
            if tool.name() == "github_pull_request_delete_branch" {
                input.as_object_mut().unwrap().remove("method");
            }
            assert!(tool.call(&input).await.is_err());
            assert_eq!(tool.schema()["additionalProperties"], false);
            let card = tool.approval(&input).unwrap().await.unwrap();
            assert!(card.full.contains(&"b".repeat(40)));
            assert!(card.full.contains("bypassRules"));
            let mut changed = input.clone();
            changed["headSha"] = json!("c".repeat(40));
            let count = source.0.load(Ordering::SeqCst);
            assert!(tool.call_approved(&changed, &card.context).await.is_err());
            assert_eq!(source.0.load(Ordering::SeqCst), count);
            let result = tool.call_approved(&input, &card.context).await.unwrap();
            assert!(result.contains("queued"));
            assert!(!result.contains("merged"));
        }
        assert_eq!(source.0.load(Ordering::SeqCst), 2);
    }
    #[test]
    fn operation_method_sha_and_duplicate_key_are_exact() {
        let action = parse("github_pull_request_merge", &input()).unwrap();
        let preview = json!({"source":{"host":"github.com"},"binding":{"repositoryId":20}});
        let mut alternative = input();
        alternative["method"] = json!("rebase");
        let rebase = parse("github_pull_request_merge", &alternative).unwrap();
        assert_ne!(action.request_hash(), rebase.request_hash());
        assert_eq!(
            action.control_key(&preview).unwrap(),
            rebase.control_key(&preview).unwrap()
        );
        let mut deletion = input();
        deletion.as_object_mut().unwrap().remove("method");
        let deletion = parse("github_pull_request_delete_branch", &deletion).unwrap();
        assert_ne!(
            action.control_key(&preview).unwrap(),
            deletion.control_key(&preview).unwrap()
        );
        for (key, value) in [
            ("operation", json!("delete_branch")),
            ("admin", json!(true)),
            ("deleteBranch", json!(true)),
            ("headSha", json!("abc1234")),
            ("method", json!("admin")),
        ] {
            let mut bad = input();
            bad[key] = value;
            assert!(parse("github_pull_request_merge", &bad).is_err(), "{key}");
        }
        assert!(parse("github_pull_request_delete_branch", &input()).is_err());
        let audit = record(&action, &preview, &action.request_hash(), "key", "reserved", true);
        let audit: Value = serde_json::from_str(&audit.target).unwrap();
        assert_eq!(audit["headSha"], "b".repeat(40));
        assert_eq!(audit["repositoryId"], 20);
    }
}
