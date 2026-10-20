//! Monetary admission belongs to the app, including the clock and content-free request trace.
use std::{path::PathBuf, sync::Arc, time::Instant};
use vults_agents::driver::BoxFuture;
use vults_chat::realtime::transport::{Admission, Failure, Gate};

pub(crate) struct RealtimeGate {
    directory: PathBuf,
    consent: crate::realtime::Consent,
    authorized: Arc<dyn Fn() -> bool + Send + Sync>,
    // All blocking workers and unclaimed admissions retain the same kernel lock.
    owner: Arc<std::fs::File>,
}

impl RealtimeGate {
    pub async fn open(
        directory: PathBuf,
        consent: crate::realtime::Consent,
        authorized: Arc<dyn Fn() -> bool + Send + Sync>,
    ) -> Result<Self, Failure> {
        let worker = tokio::task::spawn_blocking(move || {
            if !consent.enabled() || !authorized() {
                return Err(Failure::Off);
            }
            let owner = exclusive(&directory)?;
            let mut db = vults_store::Store::open_current(&directory).map_err(|_| Failure::Ledger)?;
            db.reconcile_costs(
                consent.ledger.as_deref().ok_or(Failure::Off)?,
                super::VOICE_REALTIME,
                now_ms(),
            )
            .map_err(|_| Failure::Ledger)?;
            if !authorized() {
                return Err(Failure::Off);
            }
            Ok(Self {
                directory,
                consent,
                authorized,
                owner,
            })
        });
        tokio::time::timeout(std::time::Duration::from_secs(10), worker)
            .await
            .map_err(|_| Failure::Timeout)?
            .map_err(|_| Failure::Ledger)?
    }
}

fn exclusive(directory: &std::path::Path) -> Result<Arc<std::fs::File>, Failure> {
    // Linux flock supports directories: no removable lock file can split ownership.
    #[cfg(target_os = "linux")]
    {
        let file = std::fs::File::open(directory).map_err(|_| Failure::Ledger)?;
        if !file.metadata().is_ok_and(|m| m.is_dir()) {
            return Err(Failure::Ledger);
        }
        file.try_lock().map_err(|e| match e {
            std::fs::TryLockError::WouldBlock => Failure::Busy,
            _ => Failure::Ledger,
        })?;
        Ok(Arc::new(file))
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = directory;
        Err(Failure::Off)
    }
}

fn now_ms() -> i64 {
    static CLOCK: std::sync::OnceLock<(i64, Instant)> = std::sync::OnceLock::new();
    let (epoch, start) = CLOCK.get_or_init(|| (crate::traces::now_ms(), Instant::now()));
    epoch.saturating_add(i64::try_from(start.elapsed().as_millis()).unwrap_or(i64::MAX))
}

impl Gate for RealtimeGate {
    fn active(&self) -> bool {
        self.consent.enabled() && (self.authorized)()
    }
    fn now_ms(&self) -> i64 {
        now_ms()
    }
    fn reserve(&self, ceiling: u64) -> BoxFuture<'_, Option<Admission>> {
        Box::pin(async move {
            if !self.active() {
                return None;
            }
            let (directory, consent, authorized) = (
                self.directory.clone(),
                self.consent.clone(),
                self.authorized.clone(),
            );
            let now = self.now_ms();
            let owner = self.owner.clone();
            tokio::task::spawn_blocking(move || {
                if !(authorized)() {
                    return None;
                }
                let mut db = vults_store::Store::open_current(&directory).ok()?;
                let mut span = super::request_span(super::VOICE_REALTIME, false, now, now);
                span.status = "started".into();
                span.attrs["vults.provider"] = serde_json::json!("openai");
                span.attrs["gen_ai.request.model"] = serde_json::json!(consent.model);
                span.attrs["vults.local"] = serde_json::json!(false);
                let identity = consent.ledger.as_deref()?;
                // The desktop has one voice owner. An orphaned/late settlement must finish first.
                if db.has_open_cost(identity, super::VOICE_REALTIME).ok()? {
                    return None;
                }
                if !db
                    .reserve_cost(
                        identity,
                        super::VOICE_REALTIME,
                        consent.limit_microusd,
                        Some(ceiling),
                        &span,
                    )
                    .ok()?
                {
                    return None;
                }
                let (path, expected, attempt, started) = (
                    directory.clone(),
                    identity.to_owned(),
                    span.span.clone(),
                    Instant::now(),
                );
                let admission = Admission::new(span.span, move || {
                    let _owner = owner;
                    let end =
                        now.saturating_add(i64::try_from(started.elapsed().as_millis()).unwrap_or(i64::MAX));
                    if let Ok(mut db) = vults_store::Store::open_current(&path) {
                        let _ = db.settle_cost(&expected, &attempt, Some(0), end, false);
                    }
                });
                if !(authorized)() {
                    return None;
                }
                Some(admission)
            })
            .await
            .ok()
            .flatten()
        })
    }
    fn key(&self) -> BoxFuture<'_, Option<String>> {
        Box::pin(async move {
            if !self.active() {
                return None;
            }
            let owner = self.owner.clone();
            tokio::task::spawn_blocking(move || {
                let _owner = owner;
                vults_secrets::get(vults_secrets::Secret::ApiKey("openai"))
                    .ok()
                    .flatten()
            })
            .await
            .ok()
            .flatten()
            .filter(|key| !key.is_empty() && key.len() <= 16 * 1024)
        })
    }
    fn finish<'a>(&'a self, attempt: &'a str, actual: Option<u64>, ok: bool) -> BoxFuture<'a, bool> {
        Box::pin(async move {
            let (directory, identity, attempt, now) = (
                self.directory.clone(),
                self.consent.ledger.clone(),
                attempt.to_owned(),
                self.now_ms(),
            );
            let owner = self.owner.clone();
            tokio::task::spawn_blocking(move || {
                let _owner = owner;
                let Some(identity) = identity else {
                    return false;
                };
                vults_store::Store::open_current(&directory).is_ok_and(|mut db| {
                    db.settle_cost(&identity, &attempt, actual, now, ok)
                        .unwrap_or(false)
                })
            })
            .await
            .unwrap_or(false)
        })
    }
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;
    use std::{path::Path, process::Command, time::Duration};

    struct Directory(PathBuf);
    impl Directory {
        fn new(name: &str) -> Self {
            let path = std::env::temp_dir().join(format!("vults-realtime-{name}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&path);
            std::fs::create_dir(&path).expect("create isolated realtime test directory");
            Self(path)
        }
    }
    impl Drop for Directory {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    fn consent(db: &vults_store::Store) -> crate::realtime::Consent {
        crate::realtime::Consent {
            on: true,
            ledger: Some(db.request_budget_id().expect("read test ledger identity")),
            limit_microusd: 200,
            ..Default::default()
        }
    }
    async fn open(path: &Path, consent: &crate::realtime::Consent) -> Result<RealtimeGate, Failure> {
        RealtimeGate::open(path.into(), consent.clone(), Arc::new(|| true)).await
    }

    #[tokio::test]
    async fn only_a_new_exclusive_owner_reconciles_and_never_refunds_unknown_work() {
        let directory = Directory::new("reconcile");
        let db = vults_store::Store::open(&directory.0).unwrap();
        let consent = consent(&db);
        let gate = open(&directory.0, &consent).await.unwrap();
        let old = gate.reserve(100).await.unwrap().claim();
        assert!(matches!(open(&directory.0, &consent).await, Err(Failure::Busy)));
        assert!(
            db.has_open_cost(consent.ledger.as_deref().unwrap(), super::super::VOICE_REALTIME)
                .unwrap()
        );
        drop(gate);
        let gate = open(&directory.0, &consent).await.unwrap();
        assert!(gate.reserve(101).await.is_none());
        assert!(!gate.finish(&old, Some(0), true).await);
        let receipt = db.spans(10).unwrap().into_iter().find(|s| s.span == old).unwrap();
        assert_eq!(receipt.status, "error");
        assert_eq!(receipt.attrs["vults.cost.reconciled"], true);
        assert!(receipt.attrs["vults.cost.actual_microusd"].is_null());
        let next = gate.reserve(100).await.unwrap().claim();
        assert!(gate.finish(&next, Some(20), true).await);
        drop(gate);
        let gate = open(&directory.0, &consent).await.unwrap();
        assert_eq!(
            db.spans(10)
                .unwrap()
                .into_iter()
                .find(|s| s.span == old)
                .unwrap()
                .end_ms,
            receipt.end_ms
        );
        assert!(gate.reserve(81).await.is_none());
        drop(gate.reserve(80).await.unwrap());
        drop(gate);
        let replaced = crate::realtime::Consent {
            ledger: Some("replaced".into()),
            ..consent.clone()
        };
        assert!(matches!(
            open(&directory.0, &replaced).await,
            Err(Failure::Ledger)
        ));
        assert!(open(&directory.0, &consent).await.is_ok());
    }

    // A separate test process exercises the kernel lock without loading a desktop or provider.
    #[test]
    fn process_lock_helper() {
        let Some(path) = std::env::var_os("VULTS_REALTIME_LOCK_TEST") else {
            return;
        };
        let path = PathBuf::from(path);
        if std::env::var_os("VULTS_REALTIME_LOCK_HOLD").is_none() {
            assert!(matches!(exclusive(&path), Err(Failure::Busy)));
            return;
        }
        let _owner = exclusive(&path).unwrap();
        std::fs::write(path.join("ready"), []).unwrap();
        loop {
            std::thread::park();
        }
    }

    struct Child(std::process::Child);
    impl Drop for Child {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }

    #[tokio::test]
    async fn a_live_process_blocks_reconciliation_and_a_killed_process_releases_ownership() {
        let directory = Directory::new("process");
        let db = vults_store::Store::open(&directory.0).unwrap();
        let consent = consent(&db);
        let gate = open(&directory.0, &consent).await.unwrap();
        let attempt = gate.reserve(200).await.unwrap().claim();
        let command = || {
            let mut command = Command::new(std::env::current_exe().unwrap());
            command
                .args(["--exact", "gate::realtime::tests::process_lock_helper"])
                .env("VULTS_REALTIME_LOCK_TEST", &directory.0)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null());
            command
        };
        let mut probe = Child(command().spawn().unwrap());
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                if let Some(status) = probe.0.try_wait().unwrap() {
                    assert!(status.success());
                    break;
                }
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .unwrap();
        drop(gate);
        let mut holder = Child(command().env("VULTS_REALTIME_LOCK_HOLD", "1").spawn().unwrap());
        tokio::time::timeout(Duration::from_secs(5), async {
            while !directory.0.join("ready").exists() {
                assert!(holder.0.try_wait().unwrap().is_none());
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .unwrap();
        assert!(matches!(open(&directory.0, &consent).await, Err(Failure::Busy)));
        assert!(
            db.has_open_cost(consent.ledger.as_deref().unwrap(), super::super::VOICE_REALTIME)
                .unwrap()
        );
        holder.0.kill().unwrap();
        holder.0.wait().unwrap();
        let gate = open(&directory.0, &consent).await.unwrap();
        assert!(gate.reserve(1).await.is_none());
        assert!(!gate.finish(&attempt, Some(0), true).await);
        assert!(
            !db.has_open_cost(consent.ledger.as_deref().unwrap(), super::super::VOICE_REALTIME)
                .unwrap()
        );
    }
}
