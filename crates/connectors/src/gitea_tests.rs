use super::*;

fn config() -> Connection {
    Connection {
        base_url: "https://git.example.com:8443/team/".into(),
        generation: "test-generation".into(),
    }
}
#[test]
fn credentials_cannot_be_reused_for_a_different_host_port_or_subfolder() {
    let previous = || {
        Some(Credential {
            connection: config(),
            token: "synthetic-token".into(),
            fixture_base: None,
        })
    };
    assert_eq!(
        binding(previous(), &config().base_url, "", "test-generation")
            .unwrap()
            .1,
        "synthetic-token"
    );
    for base in [
        "https://other.example/team/",
        "https://git.example.com/team/",
        "https://git.example.com:8443/other/",
    ] {
        assert!(binding(previous(), base, "", "test-generation").is_err());
        assert_eq!(
            binding(previous(), base, "new-token", "test-generation")
                .unwrap()
                .1,
            "new-token"
        );
    }
    assert!(binding(previous(), &config().base_url, "new-token", "obsolete-generation").is_err());
    assert!(binding(None, &config().base_url, "", "").is_err());
    assert!(binding(None, &config().base_url, "bad\ntoken", "").is_err());
    assert_eq!(
        binding(None, &config().base_url, "new-token", "").unwrap().1,
        "new-token"
    );
}
fn issue(number: u32) -> Value {
    json!({"repository":{"full_name":"me/app"},"number":number,"title":"Example pull request","state":"open","pull_request":{},"user":{"login":"me"},"html_url":"https://untrusted.example/"})
}

#[test]
fn read_only_rows_construct_scoped_links_and_never_fabricate_checks() {
    let mut s = Snapshot::new();
    insert(&mut s, &issue(1), "yours", "me", &config()).unwrap();
    insert(&mut s, &issue(12), "review", "me", &config()).unwrap();
    let rows = board(&s);
    assert_eq!(rows.len(), 2);
    assert_eq!(
        rows[0].url.as_deref(),
        Some("https://git.example.com:8443/team/me/app/pulls/1")
    );
    assert!(
        rows.iter()
            .all(|r| r.head.is_none() && r.checks.is_none() && r.review.is_none())
    );
    assert!(!rows[1].item.starts_with(&rows[0].item));
    assert_eq!(Gitea.watching(&s), 2);
}
#[test]
fn invalid_and_duplicate_items_fail_the_entire_snapshot() {
    for (path, value) in [
        ("number", json!(0)),
        ("state", json!("closed")),
        ("title", json!("bad\ntext")),
        ("pull_request", Value::Null),
        ("repository", json!({"full_name":"../app"})),
    ] {
        let mut row = issue(1);
        row[path] = value;
        assert!(insert(&mut Snapshot::new(), &row, "yours", "me", &config()).is_err());
    }
    let mut s = Snapshot::new();
    insert(&mut s, &issue(1), "yours", "me", &config()).unwrap();
    assert!(insert(&mut s, &issue(1), "yours", "me", &config()).is_err());
}
#[test]
fn new_identity_rebaselines_and_new_reviews_report_once() {
    let meta = json!({"login":"me","generation":"test"});
    let before = Snapshot::from([("meta".into(), meta.clone())]);
    let mut after = before.clone();
    insert(&mut after, &issue(1), "review", "me", &config()).unwrap();
    assert_eq!(Gitea.diff(&before, &after).len(), 1);
    assert!(Gitea.diff(&after, &after).is_empty());
    after.insert("meta".into(), json!({"login":"other","generation":"new"}));
    assert!(Gitea.diff(&before, &after).is_empty());
}
#[tokio::test]
async fn ungated_poll_refuses_before_any_keyring_or_network_access() {
    assert_eq!(Gitea.poll().await, Err(Error::Budget));
}

#[derive(Default)]
struct Budget {
    left: std::sync::atomic::AtomicUsize,
    results: std::sync::Mutex<Vec<bool>>,
}
impl Gate for Budget {
    fn allow(&self, id: &str) -> bool {
        assert_eq!(id, "gitea");
        self.left
            .try_update(Ordering::SeqCst, Ordering::SeqCst, |n| n.checked_sub(1))
            .is_ok()
    }
    fn done(&self, id: &str, ok: bool, _: Duration) {
        assert_eq!(id, "gitea");
        self.results.lock().unwrap().push(ok);
    }
}
/// Raw loopback HTTP is private to these tests; production credentials require HTTPS.
async fn server(replies: Vec<(u16, String, String)>) -> (Credential, tokio::task::JoinHandle<Vec<String>>) {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let task = tokio::spawn(async move {
        let mut requests = Vec::new();
        for (status, headers, body) in replies {
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
            let request = String::from_utf8(bytes[..len].to_vec()).unwrap();
            assert!(request.starts_with("GET /team/api/v1/"));
            assert!(
                request
                    .to_ascii_lowercase()
                    .contains("authorization: token synthetic-token\r\n")
            );
            requests.push(request.lines().next().unwrap().to_string());
            stream.write_all(format!("HTTP/1.1 {status} Reply\r\nConnection: close\r\nContent-Length: {}\r\n{headers}\r\n{body}",body.len()).as_bytes()).await.unwrap();
        }
        requests
    });
    (
        Credential {
            connection: Connection {
                base_url: "https://git.example.com:8443/team/".into(),
                generation: "fixture".into(),
            },
            token: "synthetic-token".into(),
            fixture_base: Some(format!("http://{address}/team/")),
        },
        task,
    )
}
fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(2))
        .build()
        .unwrap()
}
fn gate(left: usize) -> Budget {
    Budget {
        left: left.into(),
        ..Budget::default()
    }
}

#[tokio::test]
async fn loopback_poll_admits_every_get_and_uses_fixed_filters() {
    let (c, task) = server(vec![
        (200, String::new(), json!({"login":"me","id":1}).to_string()),
        (200, "X-Total-Count: 0\r\n".into(), "[]".into()),
        (200, "X-Total-Count: 0\r\n".into(), "[]".into()),
        (200, String::new(), json!({"ok":true,"data":[]}).to_string()),
    ])
    .await;
    let gate = gate(4);
    let s = read(&client(), &c, &gate, EPOCH.load(Ordering::SeqCst))
        .await
        .unwrap();
    let requests = task.await.unwrap();
    assert_eq!(requests.len(), 4);
    assert!(requests[1].contains("created=true&limit=100&page=1"));
    assert!(requests[2].contains("review_requested=true&limit=100&page=1"));
    assert!(board(&s).is_empty());
    assert_eq!(*gate.results.lock().unwrap(), [true, true, true, true]);
}
#[tokio::test]
async fn budget_refusal_between_requests_never_publishes_a_partial_snapshot() {
    let (c, task) = server(vec![(
        200,
        String::new(),
        json!({"login":"me","id":1}).to_string(),
    )])
    .await;
    let gate = gate(1);
    assert_eq!(
        read(&client(), &c, &gate, EPOCH.load(Ordering::SeqCst)).await,
        Err(Error::Budget)
    );
    assert_eq!(task.await.unwrap().len(), 1);
    assert_eq!(*gate.results.lock().unwrap(), [true]);
}
#[tokio::test]
async fn errors_are_sanitized_and_redirects_are_not_followed() {
    for (status, headers, body) in [
        (401, "", "private account details"),
        (429, "Retry-After: 7\r\n", "secret"),
        (302, "Location: http://127.0.0.1:1/stolen\r\n", ""),
        (200, "", "not json"),
    ] {
        let (c, task) = server(vec![(status, headers.into(), body.into())]).await;
        let gate = gate(1);
        let err = get(&client(), &c, &gate, EPOCH.load(Ordering::SeqCst), "user")
            .await
            .unwrap_err();
        assert!(!err.to_string().contains("private account"));
        assert!(!err.to_string().contains("secret"));
        if status == 429 {
            assert_eq!(
                err,
                Error::RateLimited {
                    retry_after: Duration::from_secs(60)
                }
            );
        }
        assert_eq!(task.await.unwrap().len(), 1);
        assert_eq!(*gate.results.lock().unwrap(), [false]);
    }
}
#[tokio::test]
async fn missing_total_count_and_oversized_bodies_are_not_empty_successes() {
    let (c, task) = server(vec![
        (200, String::new(), json!({"login":"me","id":1}).to_string()),
        (200, String::new(), "[]".into()),
    ])
    .await;
    assert!(matches!(
        read(&client(), &c, &gate(2), EPOCH.load(Ordering::SeqCst)).await,
        Err(Error::Partial(_))
    ));
    task.await.unwrap();
    let (c, task) = server(vec![(200, String::new(), "x".repeat(MAX_BYTES + 1))]).await;
    assert!(matches!(
        get(&client(), &c, &gate(1), EPOCH.load(Ordering::SeqCst), "user").await,
        Err(Error::Partial(_))
    ));
    // A bounded reader can close before the test server finishes writing.
    let _ = task.await;
}

#[tokio::test]
async fn obsolete_configuration_and_closed_gate_do_not_attempt_http() {
    let c = Credential {
        connection: Connection {
            base_url: "http://127.0.0.1:1/".into(),
            generation: "test".into(),
        },
        token: "synthetic-token".into(),
        fixture_base: None,
    };
    let gate = gate(1);
    assert_eq!(
        get(
            &client(),
            &c,
            &gate,
            EPOCH.load(Ordering::SeqCst).wrapping_add(1),
            "user"
        )
        .await,
        Err(Error::Budget)
    );
    assert_eq!(gate.left.load(Ordering::SeqCst), 1);
    assert!(gate.results.lock().unwrap().is_empty());
    let closed = super::tests::gate(0);
    assert_eq!(
        get(&client(), &c, &closed, EPOCH.load(Ordering::SeqCst), "user").await,
        Err(Error::Budget)
    );
    assert!(closed.results.lock().unwrap().is_empty());
}
