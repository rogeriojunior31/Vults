//! Commit statuses and native Actions are matched to one exact commit, with complete bounded pages.
use super::{BTreeMap, BTreeSet, Credential, Error, Gate, Value, get, id, invalid, json, list, sha, text};

fn state(status: &str, conclusion: Option<&str>) -> Result<&'static str, Error> {
    match (status, conclusion.filter(|s| !s.is_empty())) {
        ("completed", Some("success")) | ("success", None) => Ok("passed"),
        ("completed", Some("failure" | "timed_out" | "startup_failure" | "action_required"))
        | ("failure" | "error" | "warning", None) => Ok("failed"),
        ("completed", Some("cancelled" | "canceled" | "skipped" | "neutral"))
        | ("cancelled" | "canceled" | "skipped" | "neutral", None) => Ok("skipped"),
        ("in_progress" | "running", None) => Ok("running"),
        ("requested" | "pending" | "queued" | "waiting" | "blocked", None) => Ok("waiting"),
        _ => Err(invalid()),
    }
}
pub(super) async fn read(
    client: &reqwest::Client,
    c: &Credential,
    gate: &dyn Gate,
    epoch: u64,
    repo: &str,
    head: &str,
    actions: Option<bool>,
) -> Result<(Option<&'static str>, Vec<Value>), Error> {
    let mut runs = Vec::new();
    let mut expected = None;
    let mut reported = None;
    let mut contexts = BTreeSet::new();
    for page in 1..=super::MAX_PAGES {
        let (v, _) = get(
            client,
            c,
            gate,
            epoch,
            &format!("repos/{repo}/commits/{head}/status?limit=100&page={page}"),
        )
        .await?;
        if sha(&v["sha"])? != head {
            return Err(invalid());
        }
        let status = v["state"].as_str().ok_or_else(invalid)?;
        let overall = state(status, None)?;
        if reported.is_some_and(|old| old != overall) {
            return Err(invalid());
        }
        reported = Some(overall);
        let total = v["total_count"]
            .as_u64()
            .filter(|n| *n <= 500)
            .ok_or_else(invalid)? as usize;
        let rows = v["statuses"]
            .as_array()
            .filter(|r| r.len() <= 100)
            .ok_or_else(invalid)?;
        if expected.is_some_and(|n| n != total) {
            return Err(invalid());
        }
        expected = Some(total);
        for row in rows {
            let name = text(&row["context"], 1024).ok_or_else(invalid)?;
            if !contexts.insert(name.to_owned()) {
                return Err(invalid());
            }
            let state = state(row["status"].as_str().ok_or_else(invalid)?, None)?;
            runs.push(json!({"name":name,"state":state}));
        }
        if runs.len() > total || (rows.is_empty() && runs.len() != total) {
            return Err(invalid());
        }
        if runs.len() == total {
            break;
        }
        if page == super::MAX_PAGES {
            return Err(invalid());
        }
    }
    if !runs.is_empty() {
        let observed = aggregate(&runs, true);
        let expected = match reported {
            Some("passed") => Some("passing"),
            Some("failed") => Some("failing"),
            Some("waiting" | "running") => Some("running"),
            _ => None,
        };
        let skipped =
            reported == Some("passed") && observed.is_none() && runs.iter().any(|r| r["state"] == "skipped");
        if expected != observed && !skipped {
            return Err(invalid());
        }
    }
    let complete = if actions == Some(false) {
        true
    } else {
        match list(
            client,
            c,
            gate,
            epoch,
            &format!("repos/{repo}/actions/runs?head_sha={head}"),
            Some("workflow_runs"),
            true,
        )
        .await?
        {
            None => false,
            Some(workflows) => {
                runs.extend(actions_jobs(client, c, gate, epoch, repo, head, &workflows).await?);
                true
            }
        }
    };
    Ok((aggregate(&runs, complete), runs))
}
fn aggregate(runs: &[Value], complete: bool) -> Option<&'static str> {
    if runs.iter().any(|r| r["state"] == "failed") {
        Some("failing")
    } else if runs
        .iter()
        .any(|r| r["state"] == "running" || r["state"] == "waiting")
    {
        Some("running")
    } else if complete && !runs.is_empty() && runs.iter().all(|r| r["state"] == "passed") {
        Some("passing")
    } else {
        None
    }
}
async fn actions_jobs(
    client: &reqwest::Client,
    c: &Credential,
    gate: &dyn Gate,
    epoch: u64,
    repo: &str,
    head: &str,
    workflows: &[Value],
) -> Result<Vec<Value>, Error> {
    let mut latest: BTreeMap<&str, &Value> = BTreeMap::new();
    let mut ids = BTreeSet::new();
    for run in workflows {
        let number = id(&run["id"])?;
        if !ids.insert(number) || run["head_sha"] != head {
            return Err(invalid());
        }
        let path = text(&run["path"], 1024).ok_or_else(invalid)?;
        if latest
            .get(path)
            .is_none_or(|old| old["id"].as_u64().is_some_and(|old| old < number))
        {
            latest.insert(path, run);
        }
    }
    let mut result = Vec::new();
    for (path, run) in latest {
        let run_id = id(&run["id"])?;
        let outcome = state(
            run["status"].as_str().ok_or_else(invalid)?,
            run["conclusion"].as_str(),
        )?;
        let jobs = list(
            client,
            c,
            gate,
            epoch,
            &format!("repos/{repo}/actions/runs/{run_id}/jobs"),
            Some("jobs"),
            false,
        )
        .await?
        .ok_or_else(invalid)?;
        let mut job_ids = BTreeSet::new();
        if jobs.is_empty() {
            result.push(json!({"name":path,"state":outcome}));
        }
        for job in jobs {
            if !job_ids.insert(id(&job["id"])?) || job["run_id"] != run_id || job["head_sha"] != head {
                return Err(invalid());
            }
            if let Some(attempt) = run["run_attempt"].as_u64().filter(|n| *n > 0)
                && job["run_attempt"] != attempt
            {
                return Err(invalid());
            }
            let name = text(&job["name"], 1024).ok_or_else(invalid)?;
            let phase = state(
                job["status"].as_str().ok_or_else(invalid)?,
                job["conclusion"].as_str(),
            )?;
            result.push(json!({"name":format!("{path} / {name}"),"state":phase}));
        }
        // The workflow can fail before a job starts; never infer its success from a subset of jobs.
        if outcome != "passed" {
            result.push(json!({"name":path,"state":outcome}));
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cancellation_unknown_states_and_missing_actions_coverage_never_turn_green() {
        for status in ["cancelled", "skipped", "neutral"] {
            assert_eq!(state("completed", Some(status)).unwrap(), "skipped");
        }
        assert!(state("completed", None).is_err());
        assert!(state("in_progress", Some("success")).is_err());
        assert_eq!(aggregate(&[json!({"state":"passed"})], false), None);
        assert_eq!(
            aggregate(&[json!({"state":"passed"}), json!({"state":"skipped"})], true),
            None
        );
        assert_eq!(aggregate(&[json!({"state":"failed"})], false), Some("failing"));
    }
}
