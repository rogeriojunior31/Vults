//! The audit log (ADR 0014; docs/safety.md): every answer to a card and every agent config the
//! installer wrote, kept 90 days in the local database. Always on, unlike the activity history: it
//! is the record policies will be checked against. Nothing of it leaves the machine.

use vults_core::audit::Audit;
use vults_store::Store;

/// Adds one line; off the core's loop (it touches the disk). A line that can't be written is
/// logged, never retried: the answer itself already went to the agent.
pub fn keep(audit: Audit) {
    tauri::async_runtime::spawn_blocking(move || {
        let result = Store::open(&crate::history::dir())
            .and_then(|mut s| s.append_audit(crate::history::unix_now(), &audit));
        if let Err(e) = result {
            tracing::warn!(error = %e, act = audit.act.name(), "audit line not kept");
        }
    });
}

/// Drops the lines past 90 days.
pub fn prune() -> Result<(), vults_store::Error> {
    Store::open(&crate::history::dir())?.prune_audit(crate::history::unix_now())
}
