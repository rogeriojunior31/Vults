//! One reviewed installation owns admission; no background switch can open this gate.
use std::path::PathBuf;
use std::sync::{
    Mutex,
    atomic::{AtomicBool, Ordering},
};

use vults_store::traces::Span;
use vults_voice::meeting::assets::{Cancel, Gate, Outcome};

pub const FEATURE: &str = "meeting-assets";
pub const BUDGET: usize = 24;

pub struct MeetingAssetsGate {
    directory: PathBuf,
    identity: String,
    cancel: Cancel,
    pending: Mutex<Option<Span>>,
    denied: AtomicBool,
}

impl MeetingAssetsGate {
    pub fn new(directory: PathBuf, identity: String, cancel: Cancel) -> Self {
        Self {
            directory,
            identity,
            cancel,
            pending: Mutex::new(None),
            denied: AtomicBool::new(false),
        }
    }
    pub fn denied(&self) -> bool {
        self.denied.load(Ordering::SeqCst)
    }
}

impl Gate for MeetingAssetsGate {
    fn allow(&self, artifact: &'static str) -> bool {
        let admit = || {
            if self.cancel.is_cancelled() || !matches!(artifact, "segmentation" | "embedding" | "runtime") {
                return None;
            }
            let mut pending = self.pending.lock().ok()?;
            if pending.is_some() {
                return None;
            }
            let mut store = vults_store::Store::open(&self.directory).ok()?;
            let now = crate::traces::now_ms();
            let mut span = super::request_span(FEATURE, false, now, now);
            span.status = "started".into();
            span.attrs["vults.artifact"] = serde_json::json!(artifact);
            if !store
                .reserve_request(&self.identity, FEATURE, BUDGET, &span)
                .ok()?
            {
                return None;
            }
            *pending = Some(span);
            Some(())
        };
        let allowed = admit().is_some();
        if !allowed {
            self.denied.store(true, Ordering::SeqCst);
        }
        allowed
    }

    fn done(&self, artifact: &'static str, outcome: Outcome, elapsed_ms: u64) {
        let Ok(mut pending) = self.pending.lock() else {
            return;
        };
        let Some(mut span) = pending.take() else {
            return;
        };
        if span.attrs["vults.artifact"] != artifact {
            *pending = Some(span);
            return;
        }
        span.end_ms = span
            .start_ms
            .saturating_add(i64::try_from(elapsed_ms).unwrap_or(i64::MAX));
        span.status = match outcome {
            Outcome::Complete => "ok",
            Outcome::Failed => "error",
            Outcome::Cancelled => "cancelled",
        }
        .into();
        // A failed completion write leaves a charged, started attempt, never a fresh allowance.
        if let Ok(mut store) = vults_store::Store::open(&self.directory) {
            let _ = store.finish_request(&span);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn redirects_restarts_cancellation_and_ledger_replacement_cannot_reset_admission() {
        let dir = std::env::temp_dir().join(format!("vults-assets-gate-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let identity = vults_store::Store::open(&dir)
            .unwrap()
            .request_budget_id()
            .unwrap();
        let cancel = Cancel::default();
        let generic = super::super::NetGate::new(|_| true, super::super::budget, Default::default(), |_| {});
        assert!(
            !vults_connectors::Gate::allow(&generic, FEATURE),
            "A generic feature switch cannot bypass explicit review"
        );
        assert_eq!(super::super::budget(FEATURE), BUDGET);
        let gate = MeetingAssetsGate::new(dir.clone(), identity.clone(), cancel.clone());
        assert!(!gate.allow("unlisted"));
        assert!(
            vults_store::Store::open(&dir)
                .unwrap()
                .request_attempt_starts(FEATURE, 0)
                .unwrap()
                .is_empty()
        );
        for _ in 0..BUDGET {
            let gate = MeetingAssetsGate::new(dir.clone(), identity.clone(), cancel.clone());
            assert!(gate.allow("runtime"));
            assert!(!gate.allow("embedding"), "Only one request can be in flight");
            gate.done("runtime", Outcome::Complete, 5);
        }
        let gate = MeetingAssetsGate::new(dir.clone(), identity.clone(), cancel.clone());
        assert!(!gate.allow("runtime"));
        assert!(gate.denied());
        let store = vults_store::Store::open(&dir).unwrap();
        assert_eq!(store.request_attempt_starts(FEATURE, 0).unwrap().len(), BUDGET);
        for span in store.spans(100).unwrap() {
            assert_eq!(span.status, "ok");
            assert_eq!(
                span.attrs.as_object().unwrap().len(),
                3,
                "No content or URL is stored"
            );
        }
        drop(store);
        std::fs::remove_dir_all(&dir).unwrap();
        assert!(
            !gate.allow("runtime"),
            "A recreated ledger invalidates the review"
        );
        let fresh = vults_store::Store::open(&dir)
            .unwrap()
            .request_budget_id()
            .unwrap();
        let gate = MeetingAssetsGate::new(dir.clone(), fresh, cancel.clone());
        cancel.cancel();
        assert!(!gate.allow("segmentation"));
        assert!(!gate.allow("unlisted"));
        std::fs::remove_dir_all(dir).unwrap();
    }
}
