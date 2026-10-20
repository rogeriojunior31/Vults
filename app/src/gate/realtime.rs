//! Monetary admission belongs to the app, including the clock and content-free request trace.
use std::{path::PathBuf, sync::Arc, time::Instant};
use vults_agents::driver::BoxFuture;
use vults_chat::realtime::transport::{Admission, Gate};

pub(crate) struct RealtimeGate {
    pub directory: PathBuf,
    pub consent: crate::realtime::Consent,
    pub authorized: Arc<dyn Fn() -> bool + Send + Sync>,
}

impl Gate for RealtimeGate {
    fn active(&self) -> bool {
        self.consent.enabled() && (self.authorized)()
    }
    fn now_ms(&self) -> i64 {
        static CLOCK: std::sync::OnceLock<(i64, Instant)> = std::sync::OnceLock::new();
        let (epoch, start) = CLOCK.get_or_init(|| (crate::traces::now_ms(), Instant::now()));
        epoch.saturating_add(i64::try_from(start.elapsed().as_millis()).unwrap_or(i64::MAX))
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
            tokio::task::spawn_blocking(|| {
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
            tokio::task::spawn_blocking(move || {
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
