//! The audit log (ADR 0014; docs/safety.md): every answer to a card and every agent config the
//! installer wrote, kept 90 days in the local database. Always on, unlike the activity history: it
//! is the record policies will be checked against. Nothing of it leaves the machine.

use vults_core::audit::Audit;
use vults_store::Store;

/// Adds one line, timed now, through the database's one writer (`writer`).
pub fn keep(audit: Audit) {
    crate::writer::send(crate::writer::Job::Audit(crate::history::unix_now(), audit));
}

/// Drops the lines past 90 days.
pub fn prune() -> Result<(), vults_store::Error> {
    Store::open(&crate::history::dir())?.prune_audit(crate::history::unix_now())
}
