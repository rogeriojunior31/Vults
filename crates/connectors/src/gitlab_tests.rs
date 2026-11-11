use super::*;

fn response() -> Value {
    let head = "a".repeat(40);
    json!({"data":{"currentUser":{"username":"alice",
        "contributedProjects":{"nodes":[]},
        "authoredMergeRequests":{"pageInfo":{"hasNextPage":false},"nodes":[{
            "iid":"7","title":"Fix window placement","webUrl":"https://gitlab.com/team/sub/app/-/merge_requests/7",
            "sourceBranch":"fix/placement","diffHeadSha":head,"draft":false,"approved":true,"approvalsRequired":1,
            "updatedAt":"2026-11-10T12:00:00.123Z","diffStatsSummary":{"additions":12,"deletions":4,"fileCount":2},
            "reviewers":{"pageInfo":{"hasNextPage":false},"nodes":[]},
            "approvedBy":{"pageInfo":{"hasNextPage":false},"nodes":[]},
            "author":{"username":"alice"},"project":{"fullPath":"team/sub/app"},"headPipeline":{"status":"SUCCESS","sha":head,"jobs":{"pageInfo":{"hasNextPage":false},"nodes":[{"name":"Unit tests","status":"SUCCESS"}]}}
        }]},"reviewRequestedMergeRequests":{"pageInfo":{"hasNextPage":false},"nodes":[]}
    }}})
}
fn parse(response: &Value) -> Result<Snapshot, Error> {
    answer(true, &serde_json::to_vec(response).unwrap(), b"")
}
#[test]
fn reads_subgroups_and_never_offers_a_merge() {
    let snapshot = parse(&response()).unwrap();
    let rows = board(&snapshot);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].name, "app!7");
    assert_eq!(rows[0].facts.repo.as_deref(), Some("team/sub/app"));
    assert_eq!(rows[0].checks, Some(Checks::Passing));
    assert_eq!(rows[0].review, Some(Review::Approved));
    assert_eq!(rows[0].head, None);
    assert_eq!(
        rows[0].facts.diff,
        Some(Diff {
            added: 12,
            removed: 4,
            files: 2
        })
    );
    assert_eq!(
        rows[0].facts.runs,
        vec![Run {
            name: "Unit tests".into(),
            state: RunState::Passed
        }]
    );
    assert!(rows[0].facts.updated.is_some());
    assert_eq!(
        GitLab.identity(&snapshot),
        Some(("alice".into(), "gitlab.com".into()))
    );
    assert_eq!(GitLab.watching(&snapshot), 1);
    assert!(diff(&snapshot, &snapshot).is_empty());
}

fn project_row() -> Value {
    let sha = "a".repeat(40);
    json!({"fullPath":"team/sub/app","archived":false,"repository":{"rootRef":"main","lastCommit":{"sha":sha,
        "title":"Fix the default branch","committedDate":"2026-11-10T12:00:00+00:00",
        "latestPipeline":{"sha":sha,"status":"FAILED","jobs":{"pageInfo":{"hasNextPage":false},"nodes":[{"name":"Build","status":"FAILED"},{"name":"Deploy","status":"MANUAL"}]}}}},
        "releases":{"nodes":[{"name":"Version one","tagName":"v1/test","releasedAt":"2026-11-10T10:00:00Z","upcomingRelease":false}]}})
}
#[test]
fn default_branches_releases_and_changes_requested_share_the_complete_board() {
    let mut v = response();
    v["data"]["currentUser"]["contributedProjects"]["nodes"] = json!([project_row()]);
    v["data"]["currentUser"]["authoredMergeRequests"]["nodes"][0]["reviewers"]["nodes"] = json!([
        {"username":"bob","mergeRequestInteraction":{"approved":false,"reviewState":"REQUESTED_CHANGES"}},
        {"username":"carol","mergeRequestInteraction":{"approved":true,"reviewState":"APPROVED"}}]);
    let snapshot = parse(&v).unwrap();
    let rows = board(&snapshot);
    assert_eq!(rows.len(), 3);
    assert_eq!(GitLab.watching(&snapshot), 3);
    assert_eq!(rows[0].review, Some(Review::Changes));
    assert_eq!(rows[0].facts.reviewers[0].said, Said::Changes);
    assert_eq!(rows[0].facts.reviewers[1].said, Said::Approved);
    assert_eq!(rows[1].group, Group::Branches);
    assert_eq!(rows[1].checks, Some(Checks::Failing));
    assert_eq!(rows[1].facts.runs[1].state, RunState::Waiting);
    assert_eq!(rows[2].group, Group::Releases);
    assert_eq!(rows[2].facts.release, Some(Release::Latest));
    assert_eq!(
        rows[2].url.as_deref(),
        Some("https://gitlab.com/team/sub/app/-/releases/v1%2Ftest")
    );
    assert!(rows.iter().all(|r| r.head.is_none()));
    let events = diff(&parse(&response()).unwrap(), &snapshot);
    assert_eq!(events.len(), 2);
    assert!(events.iter().any(|e| {
        e.story
            .as_ref()
            .is_some_and(|s| s.kind == StoryKind::ChangesRequested)
    }));
    assert!(events.iter().any(|e| {
        e.story
            .as_ref()
            .is_some_and(|s| s.kind == StoryKind::ChecksFailed && s.branch.as_deref() == Some("main"))
    }));
    assert!(diff(&snapshot, &snapshot).is_empty());
}
#[test]
fn missing_or_paginated_details_do_not_silently_claim_complete_coverage() {
    for mutation in 0..7 {
        let mut v = response();
        let mr = &mut v["data"]["currentUser"]["authoredMergeRequests"]["nodes"][0];
        match mutation {
            0 => mr["reviewers"]["pageInfo"]["hasNextPage"] = json!(true),
            1 => mr["headPipeline"]["jobs"]["pageInfo"]["hasNextPage"] = json!(true),
            2 => mr["headPipeline"]["jobs"]["nodes"][0]["status"] = json!("FUTURE_STATE"),
            3 => mr["headPipeline"]["jobs"] = Value::Null,
            4 => mr["diffStatsSummary"]["additions"] = json!(-1),
            5 => {
                mr["reviewers"]["nodes"] = json!([{"username":"bob","mergeRequestInteraction":{"approved":false,"reviewState":"FUTURE_STATE"}}])
            }
            _ => v["data"]["currentUser"]["contributedProjects"] = Value::Null,
        };
        assert!(matches!(parse(&v), Err(Error::Partial(_))), "{mutation}");
    }
}
#[test]
fn stale_jobs_archived_projects_empty_repositories_and_upcoming_releases_stay_truthful() {
    let mut v = response();
    let mr = &mut v["data"]["currentUser"]["authoredMergeRequests"]["nodes"][0];
    mr["headPipeline"]["sha"] = json!("b".repeat(40));
    assert!(board(&parse(&v).unwrap())[0].facts.runs.is_empty());
    let mut p = project_row();
    p["archived"] = json!(true);
    v["data"]["currentUser"]["contributedProjects"]["nodes"] = json!([p]);
    assert_eq!(board(&parse(&v).unwrap()).len(), 1);
    p = project_row();
    p["repository"] = Value::Null;
    p["releases"]["nodes"][0]["upcomingRelease"] = json!(true);
    v["data"]["currentUser"]["contributedProjects"]["nodes"] = json!([p]);
    let rows = board(&parse(&v).unwrap());
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[1].group, Group::Releases);
    assert_eq!(
        rows[1].facts.release, None,
        "A future public release is not a draft"
    );
}

#[test]
fn unassigned_approvers_and_nullable_review_state_are_reported_without_replaying_old_news() {
    let mut v = response();
    let mr = &mut v["data"]["currentUser"]["authoredMergeRequests"]["nodes"][0];
    mr["reviewers"]["nodes"] =
        json!([{"username":"bob","mergeRequestInteraction":{"approved":false,"reviewState":null}}]);
    mr["approvedBy"]["nodes"] = json!([{"username":"dana"}]);
    let snapshot = parse(&v).unwrap();
    let rows = board(&snapshot);
    assert_eq!(
        rows[0].facts.reviewers,
        vec![
            Reviewer {
                login: "bob".into(),
                said: Said::Waiting
            },
            Reviewer {
                login: "dana".into(),
                said: Said::Approved
            }
        ]
    );
    let before = Snapshot::from([("meta:account".into(), json!({"login":"alice"}))]);
    assert!(
        diff(&before, &snapshot).is_empty(),
        "The old snapshot format is rebaselined once"
    );
    v["data"]["currentUser"]["authoredMergeRequests"]["nodes"][0]["approvedBy"]["pageInfo"]["hasNextPage"] =
        json!(true);
    assert!(matches!(parse(&v), Err(Error::Partial(_))));
}
#[test]
fn partial_errors_overflow_and_invalid_links_never_replace_a_snapshot() {
    for mutate in [0, 1, 2, 3, 4, 5] {
        let mut value = response();
        match mutate {
            0 => value["errors"] = json!([{"message":"private server message"}]),
            1 => {
                value["data"]["currentUser"]["authoredMergeRequests"]["pageInfo"]["hasNextPage"] = json!(true)
            }
            2 => value["data"]["currentUser"]["reviewRequestedMergeRequests"] = Value::Null,
            3 => {
                value["data"]["currentUser"]["authoredMergeRequests"]["nodes"][0]["webUrl"] =
                    json!("https://gitlab.com.evil.example/path")
            }
            4 => {
                value["data"]["currentUser"]["authoredMergeRequests"]["nodes"][0]["project"]["fullPath"] =
                    json!("team/../app")
            }
            _ => value["data"]["currentUser"]["authoredMergeRequests"]["nodes"][0] = Value::Null,
        }
        assert!(matches!(parse(&value), Err(Error::Partial(_))), "{mutate}");
    }
}
#[test]
fn absent_stale_skipped_and_manual_pipelines_are_unknown() {
    for status in ["SKIPPED", "MANUAL", "CANCELED", "SCHEDULED", "FUTURE_STATUS"] {
        let mut value = response();
        value["data"]["currentUser"]["authoredMergeRequests"]["nodes"][0]["headPipeline"]["status"] =
            json!(status);
        assert_eq!(board(&parse(&value).unwrap())[0].checks, None);
    }
    let mut value = response();
    value["data"]["currentUser"]["authoredMergeRequests"]["nodes"][0]["headPipeline"]["sha"] =
        json!("b".repeat(40));
    assert_eq!(board(&parse(&value).unwrap())[0].checks, None);
    value["data"]["currentUser"]["authoredMergeRequests"]["nodes"][0]["headPipeline"] = Value::Null;
    assert_eq!(board(&parse(&value).unwrap())[0].checks, None);
}
#[test]
fn changed_commits_and_repeated_review_requests_are_distinct_news() {
    let before = parse(&response()).unwrap();
    let mut value = response();
    let row = &mut value["data"]["currentUser"]["authoredMergeRequests"]["nodes"][0];
    row["diffHeadSha"] = json!("b".repeat(40));
    row["headPipeline"]["sha"] = json!("b".repeat(40));
    row["headPipeline"]["status"] = json!("FAILED");
    let after = parse(&value).unwrap();
    let events = diff(&before, &after);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].story.as_ref().unwrap().kind, StoryKind::ChecksFailed);
    assert!(events[0].key.ends_with(&"b".repeat(40)));
    value["data"]["currentUser"]["reviewRequestedMergeRequests"]["nodes"] =
        json!([value["data"]["currentUser"]["authoredMergeRequests"]["nodes"][0]]);
    let requested = parse(&value).unwrap();
    assert_eq!(
        GitLab.watching(&requested),
        1,
        "One MR in two groups is still one service object"
    );
    assert_eq!(diff(&after, &requested).len(), 1);
    assert!(diff(&requested, &requested).is_empty());
    assert_eq!(
        diff(&after, &requested)[0].story.as_ref().unwrap().kind,
        StoryKind::ReviewRequested
    );
}
#[test]
fn errors_are_classified_without_exposing_diagnostics() {
    assert_eq!(
        answer(false, b"", b"HTTP 401 secret@example.com token=private"),
        Err(Error::Auth(AUTH.into()))
    );
    assert!(matches!(
        answer(false, b"", b"HTTP 429"),
        Err(Error::RateLimited { .. })
    ));
    assert_eq!(
        answer(false, b"", b"private data"),
        Err(Error::Other(FAILED.into()))
    );
    assert!(matches!(
        answer(true, b"<html>bad gateway</html>", b""),
        Err(Error::Partial(_))
    ));
    assert!(matches!(
        parse(&json!({"data":{"currentUser":null}})),
        Err(Error::Auth(_))
    ));
}

#[test]
fn account_changes_and_zero_required_approvals_do_not_invent_news() {
    let before = parse(&response()).unwrap();
    let mut value = response();
    value["data"]["currentUser"]["username"] = json!("bob");
    value["data"]["currentUser"]["authoredMergeRequests"]["nodes"][0]["headPipeline"]["status"] =
        json!("FAILED");
    let after = parse(&value).unwrap();
    assert!(diff(&before, &after).is_empty());
    assert!(
        before
            .keys()
            .filter(|key| !key.starts_with("meta:"))
            .all(|key| !after.contains_key(key)),
        "Changing accounts retires every old row and its alerts"
    );
    value["data"]["currentUser"]["authoredMergeRequests"]["nodes"][0]["approvalsRequired"] = json!(0);
    assert_eq!(board(&parse(&value).unwrap())[0].review, None);
}

#[test]
fn native_cli_is_fixed_to_https_gitlab_com_with_no_update_check() {
    let mut command = Command::new("glab");
    configure(&mut command);
    let command = command.as_std();
    let args: Vec<_> = command.get_args().map(|arg| arg.to_str().unwrap()).collect();
    assert_eq!(
        args,
        [
            "api",
            "graphql",
            "--hostname",
            "gitlab.com",
            "--method",
            "POST",
            "--input",
            "-"
        ]
    );
    let env: std::collections::BTreeMap<_, _> = command.get_envs().collect();
    for (key, value) in [
        ("GITLAB_API_HOST", "gitlab.com"),
        ("GLAB_API_PROTOCOL", "https"),
        ("GLAB_CHECK_UPDATE", "false"),
        ("GLAB_SKIP_TLS_VERIFY", "false"),
        ("GLAB_ENABLE_CI_AUTOLOGIN", "false"),
    ] {
        assert_eq!(env[std::ffi::OsStr::new(key)], Some(std::ffi::OsStr::new(value)));
    }
}

#[test]
#[ignore = "private transport child, invoked only by the offline transport test"]
fn transport_child() {
    use std::io::{Read, Write};
    let mode = std::env::var("VULTS_GITLAB_FIXTURE").unwrap_or_default();
    if mode.is_empty() {
        return;
    }
    let mut input = Vec::new();
    std::io::stdin().read_to_end(&mut input).unwrap();
    assert_eq!(serde_json::from_slice::<Value>(&input).unwrap()["query"], QUERY);
    let bytes = if mode == "flood" {
        vec![b'x'; MAX_BYTES + 1024]
    } else {
        serde_json::to_vec(&response()).unwrap()
    };
    let _ = std::io::stdout().write_all(&bytes);
}

#[tokio::test]
async fn offline_native_transport_bounds_output_and_sends_only_the_fixed_query() {
    for mode in ["normal", "flood"] {
        let mut child = Command::new(std::env::current_exe().unwrap());
        child
            .args([
                "--exact",
                "gitlab::tests::transport_child",
                "--ignored",
                "--nocapture",
            ])
            .env("VULTS_GITLAB_FIXTURE", mode)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        #[cfg(windows)]
        child.creation_flags(0x0800_0000);
        let result = tokio::time::timeout(Duration::from_secs(5), read(&mut child))
            .await
            .unwrap();
        if mode == "normal" {
            let (success, stdout, _) = result.unwrap();
            assert!(success);
            assert!(String::from_utf8_lossy(&stdout).contains("Fix window placement"));
        } else {
            assert!(matches!(result, Err(Error::Partial(_))));
        }
    }
}
