//! Human-approved Actions controls reserve a durable receipt before GitHub sees the POST.

use super::GithubSource;
use super::{GithubToolGate, Source, failed, github_result};
use serde_json::{Value, json};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use vults_agents::driver::BoxFuture;
use vults_chat::tools::{Tool, ToolApproval, Tools};
use vults_connectors::github::ActionsControl;
use vults_core::audit::{Act, Actor, Audit};

const OPERATIONS: &[&str] = &[
    "github_actions_rerun_failed",
    "github_actions_rerun_run",
    "github_actions_rerun_job",
    "github_actions_cancel",
];
pub(super) fn tools(source: Arc<dyn Source>) -> Tools {
    OPERATIONS
        .iter()
        .map(|operation| {
            Arc::new(ActionsTool {
                source: source.clone(),
                operation,
            }) as Arc<dyn Tool>
        })
        .collect()
}

fn control_key(action: &ActionsControl, preview: &Value) -> Result<String, String> {
    let host = preview["source"]["host"]
        .as_str()
        .ok_or("The approved GitHub host is missing.")?;
    let repo = preview["binding"]["repositoryId"]
        .as_u64()
        .filter(|n| *n > 0)
        .ok_or("The approved repository identity is missing.")?;
    let kind = if matches!(action, ActionsControl::Cancel { .. }) {
        "cancel"
    } else {
        "rerun"
    };
    Ok(format!(
        "{}/{repo}/{}/{}/{kind}",
        host.to_ascii_lowercase(),
        action.run_id(),
        action.attempt()
    ))
}
fn record(
    subject: &str,
    action: &ActionsControl,
    preview: &Value,
    hash: &str,
    key: &str,
    outcome: &str,
    receipt: bool,
) -> Audit {
    Audit::new(Actor::System,Act::ExternalWrite,subject,action.repository(),action.tool_name(),&json!({"controlKey":key,"receipt":receipt,"outcome":outcome,"account":preview["source"]["account"],"host":preview["source"]["host"],"runId":action.run_id(),"attempt":action.attempt(),"headSha":action.head_sha(),"requestHash":hash,"scope":preview["scope"]["kind"],"jobId":if let ActionsControl::RerunJob{job_id,..}=action{Some(*job_id)}else{None},"accountId":preview["binding"]["identity"]["id"],"repositoryId":preview["binding"]["repositoryId"]}).to_string())
}
struct Receipt<'a> {
    subject: String,
    action: &'a ActionsControl,
    preview: &'a Value,
    hash: &'a str,
    key: &'a str,
    reserved: Arc<AtomicBool>,
    outcome: String,
}
impl Drop for Receipt<'_> {
    fn drop(&mut self) {
        crate::audit::keep(record(
            &self.subject,
            self.action,
            self.preview,
            self.hash,
            self.key,
            &self.outcome,
            self.reserved.load(Ordering::SeqCst),
        ));
    }
}

pub(super) fn prepare(
    app: GithubSource,
    action: ActionsControl,
) -> BoxFuture<'static, Result<Value, String>> {
    Box::pin(async move {
        if !app.github_work_item_allowed() {
            return Err("Turn on GitHub tools for Zeca and the GitHub connector first.".into());
        }
        let gate = GithubToolGate::new(app.clone());
        let preview = vults_connectors::github::prepare_actions_control(&gate, &action)
            .await
            .map_err(|error| error.to_string())?;
        let key = control_key(&action, &preview)?;
        // This preview is a fresh read: the attempt is still the latest and in the state the
        // control needs, so an old uncertain control did not take effect (store's reconcile).
        let line = record(
            app.subject(),
            &action,
            &preview,
            &action.request_hash(),
            &key,
            "reconciled_absent",
            true,
        );
        let free = tauri::async_runtime::spawn_blocking(move || {
            vults_store::Store::open(&crate::history::dir())
                .and_then(|mut store| {
                    store.reconcile_github_control(
                        &key,
                        crate::history::unix_now(),
                        super::RECONCILE_AFTER_SECS,
                        &line,
                    )
                })
                .map_err(|error| error.to_string())
        })
        .await
        .map_err(|error| error.to_string())??;
        if !free {
            return Err("A control for this exact run attempt is already reserved, accepted or uncertain. Inspect the latest run on GitHub; do not submit it again. An uncertain one can be proposed again 15 minutes after it, if GitHub still shows the same attempt.".into());
        }
        if !app.github_work_item_allowed() {
            return Err("GitHub tools were disabled while the Actions proposal was being prepared.".into());
        }
        Ok(preview)
    })
}

pub(super) fn execute(
    app: GithubSource,
    action: ActionsControl,
    preview: Value,
    hash: String,
) -> BoxFuture<'static, Result<Value, String>> {
    Box::pin(async move {
        let key = control_key(&action, &preview)?;
        let reserved = Arc::new(AtomicBool::new(false));
        let mut receipt = Receipt {
            subject: app.subject().into(),
            action: &action,
            preview: &preview,
            hash: &hash,
            key: &key,
            reserved: reserved.clone(),
            outcome: "cancelled_or_uncertain".into(),
        };
        if !app.github_work_item_allowed() {
            receipt.outcome = "not_sent".into();
            return Err("Turn on GitHub tools for Zeca and the GitHub connector first.".into());
        }
        let reservation = record(app.subject(), &action, &preview, &hash, &key, "reserved", true);
        let gate = GithubToolGate::new(app);
        let result=vults_connectors::github::execute_actions_control(&gate,&action,&preview,&hash,move||async move{
            let won=tauri::async_runtime::spawn_blocking(move||{
                let won=vults_store::Store::open(&crate::history::dir()).and_then(|mut store|store.reserve_github_control(crate::history::unix_now(),&reservation)).map_err(|error|error.to_string())?;
                if won{reserved.store(true,Ordering::SeqCst);}Ok::<_,String>(won)
            }).await.map_err(|error|vults_connectors::Error::Other(error.to_string()))?.map_err(vults_connectors::Error::Other)?;
            if !won{return Err(vults_connectors::Error::Other("A control for this run attempt was already reserved. Inspect GitHub instead of resending it.".into()));}
            Ok(())
        }).await.map_err(|error|error.to_string());
        receipt.outcome = match &result {
            Ok(value) => value["status"].as_str().unwrap_or("uncertain").into(),
            Err(_) if !receipt.reserved.load(Ordering::SeqCst) => "not_sent".into(),
            Err(_) => "uncertain".into(),
        };
        result
    })
}

#[derive(Debug)]
struct ActionsTool {
    source: Arc<dyn Source>,
    operation: &'static str,
}
impl Tool for ActionsTool {
    fn name(&self) -> &str {
        self.operation
    }
    fn description(&self) -> &str {
        match self.operation {
            "github_actions_rerun_failed" => {
                "Proposes rerunning all failed jobs AND their dependent jobs in the exact latest completed workflow attempt. Read github_actions_run first and supply its runId, attempt and headSha. Always needs a human card. Accepted pending verification is not a verified new attempt; read the latest run instead of resending."
            }
            "github_actions_rerun_run" => {
                "Proposes rerunning the ENTIRE workflow, including jobs that already passed, for the exact completed run attempt and commit. May consume Actions minutes. Always needs its exact human card. Inspect the returned status and read the latest run when verification is pending."
            }
            "github_actions_rerun_job" => {
                "Proposes rerunning one FAILED job AND its dependent jobs. The job must belong to the exact latest completed run attempt and headSha. Always needs a human card. Dependency IDs are not enumerated by the API; the card names that scope explicitly. Never resend an accepted or uncertain control."
            }
            _ => {
                "Proposes normal cancellation of the exact active run attempt and commit. Never force-cancels or approves deployments. Always requires a human card. Cancellation accepted is not cancellation completed: inspect the observed phase and read the latest run without resending."
            }
        }
    }
    fn schema(&self) -> Value {
        let mut properties = json!({"repository":{"type":"string","pattern":"^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$"},"runId":{"type":"integer","minimum":1},"attempt":{"type":"integer","minimum":1,"maximum":4294967294u64},"headSha":{"type":"string","pattern":"^[0-9a-f]{40}$"}});
        let mut required = vec!["repository", "runId", "attempt", "headSha"];
        if self.operation == "github_actions_rerun_job" {
            properties["jobId"] = json!({"type":"integer","minimum":1});
            required.push("jobId");
        }
        json!({"type":"object","properties":properties,"required":required,"additionalProperties":false})
    }
    fn call<'a>(&'a self, _: &'a Value) -> BoxFuture<'a, Result<String, String>> {
        failed(json!({"error":"GitHub Actions controls require the explicit human approval card."}))
    }
    fn approval<'a>(&'a self, input: &'a Value) -> Option<BoxFuture<'a, Result<ToolApproval, String>>> {
        Some(Box::pin(async move {
            let action = parse(self.operation, input)?;
            let preview = self.source.github_actions_prepare(action.clone()).await?;
            let hash = action.request_hash();
            let target = format!(
                "{} · run #{} · attempt {} · {} · sha256:{hash}",
                action.repository(),
                action.run_id(),
                action.attempt(),
                self.operation
            );
            let full=serde_json::to_string_pretty(&json!({"request":action,"requestHash":hash,"source":preview["source"],"current":preview["run"],"scope":preview["scope"]})).map_err(|error|error.to_string())?;
            Ok(ToolApproval{target,description:"Review the exact workflow run, attempt, commit and affected job scope before allowing this control.".into(),full,context:json!({"preview":preview,"requestHash":hash})})
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
                return Err("The Actions request changed after approval. Review a new card.".into());
            }
            let value = self
                .source
                .github_actions_execute(action, context["preview"].clone(), hash)
                .await?;
            let failed = matches!(
                value["status"].as_str(),
                Some("rejected" | "uncertain" | "completed_without_cancellation")
            );
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
fn parse(operation: &str, input: &Value) -> Result<ActionsControl, String> {
    let mut fields = input
        .as_object()
        .cloned()
        .ok_or("Actions arguments must be an object.")?;
    if fields.contains_key("operation") {
        return Err("The selected tool fixes the Actions operation.".into());
    }
    let kind = match operation {
        "github_actions_rerun_failed" => "rerun_failed",
        "github_actions_rerun_run" => "rerun_run",
        "github_actions_rerun_job" => "rerun_job",
        "github_actions_cancel" => "cancel",
        _ => return Err("Unsupported Actions control.".into()),
    };
    fields.insert("operation".into(), json!(kind));
    let action: ActionsControl = serde_json::from_value(Value::Object(fields))
        .map_err(|error| format!("Invalid Actions arguments: {error}"))?;
    action.validate().map_err(|error| error.to_string())?;
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
        fn github_actions_prepare(&self, _: ActionsControl) -> BoxFuture<'static, Result<Value, String>> {
            Box::pin(async {
                Ok(
                    json!({"source":{"host":"github.com","account":"person"},"binding":{"repositoryId":20,"identity":{"id":1}},"run":{"id":12,"attempt":1,"headSha":"b".repeat(40)},"scope":{"kind":"failed-jobs-and-dependents"}}),
                )
            })
        }
        fn github_actions_execute(
            &self,
            _: ActionsControl,
            _: Value,
            _: String,
        ) -> BoxFuture<'static, Result<Value, String>> {
            self.0.fetch_add(1, Ordering::SeqCst);
            Box::pin(async { Ok(json!({"status":"accepted_pending_verification","verified":false})) })
        }
    }
    fn input() -> Value {
        json!({"repository":"org/repo","runId":12,"attempt":1,"headSha":"b".repeat(40)})
    }
    #[tokio::test]
    async fn exact_cards_are_required_and_edited_arguments_never_execute() {
        let source = Arc::new(Writes(AtomicUsize::new(0)));
        let tools = tools(source.clone());
        assert_eq!(tools.len(), 4);
        for tool in tools {
            let mut input = input();
            if tool.name() == "github_actions_rerun_job" {
                input["jobId"] = json!(55);
            }
            assert!(tool.call(&input).await.is_err());
            assert_eq!(tool.schema()["additionalProperties"], false);
            let card = tool.approval(&input).unwrap().await.unwrap();
            assert!(card.full.contains(&"b".repeat(40)));
            assert!(card.full.contains("failed-jobs-and-dependents"));
            let mut edited = input.clone();
            edited["attempt"] = json!(2);
            let before = source.0.load(Ordering::SeqCst);
            assert!(tool.call_approved(&edited, &card.context).await.is_err());
            assert_eq!(source.0.load(Ordering::SeqCst), before);
            assert!(
                tool.call_approved(&input, &card.context)
                    .await
                    .unwrap()
                    .contains("accepted_pending_verification")
            );
        }
        assert_eq!(source.0.load(Ordering::SeqCst), 4);
    }
    #[test]
    fn a_process_killed_after_the_reservation_still_blocks_the_same_control_after_restart() {
        let dir = std::env::temp_dir().join(format!("vults-app-actions-restart-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let action = parse("github_actions_rerun_failed", &input()).unwrap();
        let preview = json!({"source":{"host":"github.com","account":"person"},"binding":{"repositoryId":20,"identity":{"id":1}},"scope":{"kind":"failed-jobs-and-dependents"}});
        let key = control_key(&action, &preview).unwrap();
        {
            // What execute commits before its POST; the Receipt's Drop never runs.
            let mut store = vults_store::Store::open(&dir).unwrap();
            let reservation = record("zeca", &action, &preview, "h", &key, "reserved", true);
            assert!(store.reserve_github_control(1, &reservation).unwrap());
        }
        // After the restart, prepare rereads GitHub: the host may come back in another case.
        let mut fresh = preview.clone();
        fresh["source"]["host"] = json!("GitHub.com");
        let again = parse("github_actions_rerun_run", &input()).unwrap();
        let store = vults_store::Store::open(&dir).unwrap();
        assert!(
            store
                .github_control_blocked(&control_key(&again, &fresh).unwrap())
                .unwrap()
        );
        // A control never sent (no reservation, receipt false) blocks nothing.
        let next = parse(
            "github_actions_rerun_failed",
            &json!({"repository":"org/repo","runId":12,"attempt":2,"headSha":"b".repeat(40)}),
        )
        .unwrap();
        let next_key = control_key(&next, &preview).unwrap();
        let mut store = store;
        store
            .append_audit(
                2,
                &record("zeca", &next, &preview, "h", &next_key, "not_sent", false),
            )
            .unwrap();
        assert!(!store.github_control_blocked(&next_key).unwrap());
        drop(store);
        let _ = std::fs::remove_dir_all(&dir);
    }
    #[test]
    fn rerun_variants_share_a_receipt_and_cancel_has_an_independent_key() {
        let preview = json!({"source":{"host":"github.com","account":"person"},"binding":{"repositoryId":20,"identity":{"id":1}},"scope":{"kind":"selected-failed-job-and-dependents"}});
        let first = parse("github_actions_rerun_run", &input()).unwrap();
        let second = parse("github_actions_rerun_failed", &input()).unwrap();
        let cancel = parse("github_actions_cancel", &input()).unwrap();
        assert_eq!(
            control_key(&first, &preview).unwrap(),
            control_key(&second, &preview).unwrap()
        );
        assert_ne!(
            control_key(&first, &preview).unwrap(),
            control_key(&cancel, &preview).unwrap()
        );
        let mut job = input();
        job["jobId"] = json!(55);
        let action = parse("github_actions_rerun_job", &job).unwrap();
        let audit = record(
            "zeca",
            &action,
            &preview,
            &action.request_hash(),
            "key",
            "reserved",
            true,
        );
        let value: Value = serde_json::from_str(&audit.target).unwrap();
        assert_eq!(value["jobId"], 55);
        assert_eq!(value["repositoryId"], 20);
        assert_eq!(value["accountId"], 1);
        for field in ["operation", "extra"] {
            let mut bad = input();
            bad[field] = json!("cancel");
            assert!(parse("github_actions_rerun_run", &bad).is_err());
        }
    }
}
