use super::*;
use crate::Connector;
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex, atomic::AtomicUsize},
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

struct Budget {
    left: AtomicUsize,
    done: Mutex<Vec<bool>>,
}
impl Gate for Budget {
    fn allow(&self, id: &str) -> bool {
        assert_eq!(id, "gitea");
        self.left
            .try_update(Ordering::SeqCst, Ordering::SeqCst, |n| n.checked_sub(1))
            .is_ok()
    }
    fn done(&self, id: &str, ok: bool, _: std::time::Duration) {
        assert_eq!(id, "gitea");
        self.done.lock().unwrap().push(ok);
    }
}
type Reply = (u16, Value);
type Calls = Arc<Mutex<Vec<String>>>;
async fn server(routes: Vec<(String, Reply)>) -> (Credential, Calls, tokio::task::JoinHandle<()>) {
    let expected = routes.len();
    let mut replies: BTreeMap<String, VecDeque<Reply>> = BTreeMap::new();
    for (path, reply) in routes {
        replies.entry(path).or_default().push_back(reply);
    }
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let calls = Calls::default();
    let recorded = calls.clone();
    let task = tokio::spawn(async move {
        for _ in 0..expected {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut bytes = vec![0; 16384];
            let mut len = 0;
            loop {
                let n = stream.read(&mut bytes[len..]).await.unwrap();
                assert!(n > 0);
                len += n;
                if bytes[..len].ends_with(b"\r\n\r\n") {
                    break;
                }
                assert!(len < bytes.len());
            }
            let raw = String::from_utf8(bytes[..len].to_vec()).unwrap();
            assert!(
                raw.to_ascii_lowercase()
                    .contains("authorization: token synthetic-token\r\n")
            );
            let path = raw
                .lines()
                .next()
                .unwrap()
                .strip_prefix("GET /team/api/v1/")
                .unwrap()
                .strip_suffix(" HTTP/1.1")
                .unwrap()
                .to_owned();
            recorded.lock().unwrap().push(path.clone());
            let (status, body) = replies
                .get_mut(&path)
                .and_then(VecDeque::pop_front)
                .unwrap_or_else(|| panic!("Unexpected fixture path: {path}"));
            let total = body
                .as_array()
                .map(|v| format!("X-Total-Count: {}\r\n", v.len()))
                .unwrap_or_default();
            let body = body.to_string();
            stream.write_all(format!("HTTP/1.1 {status} Reply\r\nConnection: close\r\nContent-Length: {}\r\n{total}\r\n{body}",body.len()).as_bytes()).await.unwrap();
        }
    });
    let mut c = Credential::new(
        Connection {
            base_url: "https://git.example.com:8443/team/".into(),
            generation: "fixture".into(),
        },
        "synthetic-token".into(),
    );
    c.fixture_base = Some(format!("http://{address}/team/"));
    (c, calls, task)
}
fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(std::time::Duration::from_secs(3))
        .build()
        .unwrap()
}
fn fixtures() -> Vec<(String, Reply)> {
    let head = "a".repeat(40);
    let branch = "b".repeat(40);
    let issue = json!({"repository":{"full_name":"me/app"},"number":7,"title":"Improve Windows","state":"open","pull_request":{},"user":{"login":"me"}});
    vec![
        ("user".into(), (200, json!({"login":"me","id":1}))),
        (
            "repos/issues/search?state=open&type=pulls&created=true&limit=100&page=1".into(),
            (200, json!([issue.clone()])),
        ),
        (
            "repos/issues/search?state=open&type=pulls&review_requested=true&limit=100&page=1".into(),
            (200, json!([issue])),
        ),
        (
            "repos/me/app/pulls/7".into(),
            (
                200,
                json!({"number":7,"state":"open","merged":false,"title":"Improve Windows","user":{"login":"me"},"head":{"sha":head,"ref":"fix/windows"},"base":{"repo":{"full_name":"me/app","has_actions":true}},"additions":12,"deletions":4,"changed_files":2,"updated_at":"2026-11-10T12:00:00.123Z","requested_reviewers":[{"login":"bob"}],"requested_reviewers_teams":[]}),
            ),
        ),
        (
            "repos/me/app/pulls/7/reviews?limit=100&page=1".into(),
            (
                200,
                json!([{"id":1,"state":"APPROVED","user":{"login":"bob"},"commit_id":head,"stale":false,"dismissed":false,"submitted_at":"2026-11-10T12:00:01Z"},{"id":2,"state":"REQUEST_CHANGES","user":{"login":"carol"},"commit_id":head,"stale":false,"dismissed":false,"submitted_at":"2026-11-10T12:00:02Z"}]),
            ),
        ),
        (
            format!("repos/me/app/commits/{head}/status?limit=100&page=1"),
            (
                200,
                json!({"sha":head,"state":"success","total_count":1,"statuses":[{"context":"lint","status":"success"}]}),
            ),
        ),
        (
            format!("repos/me/app/actions/runs?head_sha={head}&limit=100&page=1"),
            (
                200,
                json!({"total_count":1,"workflow_runs":[{"id":11,"head_sha":head,"path":".gitea/workflows/ci.yml","status":"completed","conclusion":"failure","run_attempt":2}]}),
            ),
        ),
        (
            "repos/me/app/actions/runs/11/jobs?limit=100&page=1".into(),
            (
                200,
                json!({"total_count":2,"jobs":[{"id":111,"run_id":11,"head_sha":head,"run_attempt":2,"name":"Windows build","status":"completed","conclusion":"failure"},{"id":112,"run_id":11,"head_sha":head,"run_attempt":2,"name":"Linux build","status":"completed","conclusion":"success"}]}),
            ),
        ),
        (
            "repos/search?uid=1&sort=updated&order=desc&archived=false&limit=10&page=1".into(),
            (
                200,
                json!({"ok":true,"data":[{"full_name":"me/app","archived":false,"empty":false,"default_branch":"main","has_actions":false,"has_releases":true}]}),
            ),
        ),
        (
            "repos/me/app/branches/main".into(),
            (
                200,
                json!({"name":"main","commit":{"id":branch,"message":"Default branch update\nMore detail","timestamp":"2026-11-10T11:00:00Z"}}),
            ),
        ),
        (
            format!("repos/me/app/commits/{branch}/status?limit=100&page=1"),
            (
                200,
                json!({"sha":branch,"state":"pending","total_count":1,"statuses":[{"context":"build","status":"pending"}]}),
            ),
        ),
        (
            "repos/me/app/releases?limit=1&page=1".into(),
            (
                200,
                json!([{"name":"Version one","tag_name":"v1/test","draft":false,"prerelease":true,"published_at":"2026-11-10T10:00:00Z"}]),
            ),
        ),
    ]
}
#[tokio::test]
async fn complete_native_api_snapshot_binds_jobs_reviews_and_links_and_deduplicates_a_shared_pr() {
    let routes = fixtures();
    let expected = routes.len();
    let (c, calls, task) = server(routes).await;
    let budget = Budget {
        left: expected.into(),
        done: Mutex::new(Vec::new()),
    };
    let snapshot = read(&client(), &c, &budget, EPOCH.load(Ordering::SeqCst))
        .await
        .unwrap();
    task.await.unwrap();
    let rows = board(&snapshot);
    assert_eq!(rows.len(), 4);
    assert_eq!(super::super::Gitea.watching(&snapshot), 3);
    assert_eq!(rows[0].checks, Some(Checks::Failing));
    assert_eq!(rows[0].review, Some(Review::Changes));
    assert_eq!(
        rows[0].facts.diff,
        Some(Diff {
            added: 12,
            removed: 4,
            files: 2
        })
    );
    assert!(
        rows[0]
            .facts
            .runs
            .iter()
            .any(|r| r.name.contains("Windows build") && r.state == RunState::Failed)
    );
    assert_eq!(rows[2].group, Group::Branches);
    assert_eq!(rows[2].checks, Some(Checks::Running));
    assert_eq!(rows[3].facts.release, Some(Release::Prerelease));
    assert!(
        rows[3]
            .url
            .as_deref()
            .unwrap()
            .ends_with("/releases/tag/v1%2Ftest")
    );
    assert!(rows.iter().all(|r| r.head.is_none()));
    assert!(super::super::Gitea.diff(&snapshot, &snapshot).is_empty());
    assert_eq!(
        calls
            .lock()
            .unwrap()
            .iter()
            .filter(|p| *p == "repos/me/app/pulls/7")
            .count(),
        1
    );
    assert_eq!(budget.done.lock().unwrap().len(), expected);
    assert!(budget.done.lock().unwrap().iter().all(|v| *v));
}
#[tokio::test]
async fn wrong_heads_attempts_unknown_jobs_and_partial_pages_never_publish_a_partial_board() {
    for mode in 0..7 {
        let mut routes = fixtures();
        let jobs = routes
            .iter_mut()
            .find(|(p, _)| p.contains("/runs/11/jobs"))
            .unwrap();
        match mode {
            0 => jobs.1.1["jobs"][0]["head_sha"] = json!("c".repeat(40)),
            1 => jobs.1.1["jobs"][0]["run_attempt"] = json!(1),
            2 => jobs.1.1["jobs"][0]["conclusion"] = json!("FUTURE_STATE"),
            3 => jobs.1.1["total_count"] = json!(0),
            4 => {
                routes
                    .iter_mut()
                    .find(|(p, _)| p.contains("actions/runs?head_sha"))
                    .unwrap()
                    .1
                    .1["workflow_runs"][0]["head_sha"] = json!("c".repeat(40))
            }
            5 => {
                routes
                    .iter_mut()
                    .find(|(p, _)| p.contains(&format!("commits/{}/status", "a".repeat(40))))
                    .unwrap()
                    .1
                    .1["sha"] = json!("c".repeat(40))
            }
            _ => {
                routes
                    .iter_mut()
                    .find(|(p, _)| p.contains(&format!("commits/{}/status", "a".repeat(40))))
                    .unwrap()
                    .1
                    .1["state"] = json!("failure")
            }
        };
        let (c, _, task) = server(routes).await;
        let budget = Budget {
            left: 100.into(),
            done: Mutex::new(Vec::new()),
        };
        assert!(
            matches!(
                read(&client(), &c, &budget, EPOCH.load(Ordering::SeqCst)).await,
                Err(Error::Partial(_))
            ),
            "{mode}"
        );
        task.abort();
    }
}
#[tokio::test]
async fn a_gate_refusal_mid_enrichment_is_a_failed_snapshot_not_an_empty_success() {
    let (c, _, task) = server(fixtures()).await;
    let budget = Budget {
        left: 4.into(),
        done: Mutex::new(Vec::new()),
    };
    assert_eq!(
        read(&client(), &c, &budget, EPOCH.load(Ordering::SeqCst)).await,
        Err(Error::Budget)
    );
    task.abort();
    assert_eq!(budget.done.lock().unwrap().len(), 4);
}
#[test]
fn background_cadence_reserves_room_within_the_existing_daily_budget() {
    let snapshot = Snapshot::from([
        ("meta:poll".into(), json!({"requests":40})),
        ("pr".into(), json!({"ci":"running"})),
    ]);
    assert_eq!(
        super::super::Gitea.interval(&snapshot),
        std::time::Duration::from_secs(1440)
    );
    assert_eq!(
        super::super::Gitea.interval(&Snapshot::new()),
        std::time::Duration::from_secs(300)
    );
}

#[tokio::test]
async fn missing_actions_coverage_keeps_external_success_from_becoming_a_passing_summary() {
    let mut routes = fixtures();
    routes
        .iter_mut()
        .find(|(p, _)| p.contains("actions/runs?head_sha"))
        .unwrap()
        .1 = (404, json!({}));
    let (c, calls, task) = server(routes).await;
    let budget = Budget {
        left: 100.into(),
        done: Mutex::new(Vec::new()),
    };
    let snapshot = read(&client(), &c, &budget, EPOCH.load(Ordering::SeqCst))
        .await
        .unwrap();
    task.abort();
    assert_eq!(board(&snapshot)[0].checks, None);
    assert!(!calls.lock().unwrap().iter().any(|p| p.contains("/runs/11/jobs")));
}
#[tokio::test]
async fn invalid_account_ids_refuse_before_an_unscoped_repository_search() {
    for bad in [json!(0), json!(-1), json!(u64::MAX)] {
        let mut routes = fixtures();
        routes[0].1.1["id"] = bad;
        let (c, calls, task) = server(routes).await;
        let budget = Budget {
            left: 100.into(),
            done: Mutex::new(Vec::new()),
        };
        assert!(matches!(
            read(&client(), &c, &budget, EPOCH.load(Ordering::SeqCst)).await,
            Err(Error::Partial(_))
        ));
        task.abort();
        assert_eq!(calls.lock().unwrap().as_slice(), ["user"]);
    }
}
