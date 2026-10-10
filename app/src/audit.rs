//! The audit log (ADR 0014; docs/safety.md): every answer to a card and every agent config the
//! installer wrote, kept 90 days in the local database. Always on, unlike the activity history: it
//! is the record policies will be checked against. Nothing of it leaves the machine.

use std::sync::OnceLock;
use std::sync::mpsc::{Sender, channel};
use std::time::Duration;

use vults_core::audit::Audit;
use vults_store::Store;

/// One writer thread, in arrival order: an Always and the rule line after it never swap, and a
/// line that meets a busy database (a start-up import, a clear) waits its turn instead of failing.
static WRITER: OnceLock<std::sync::Mutex<Sender<(i64, Audit)>>> = OnceLock::new();

/// Adds one line, timed now; off the caller's thread. A line that can't be written after a few
/// tries is logged, never retried later: the answer itself already went to the agent.
pub fn keep(audit: Audit) {
    let at = crate::history::unix_now();
    let writer = WRITER.get_or_init(|| {
        let (tx, rx) = channel::<(i64, Audit)>();
        std::thread::spawn(move || {
            let mut store: Option<Store> = None;
            for (at, line) in rx {
                let mut tries = 0;
                loop {
                    let result = match store.as_mut() {
                        Some(s) => s.append_audit(at, &line),
                        None => Store::open(&crate::history::dir()).and_then(|mut s| {
                            let r = s.append_audit(at, &line);
                            store = Some(s);
                            r
                        }),
                    };
                    match result {
                        Ok(()) => break,
                        Err(e) if tries < 5 => {
                            tries += 1;
                            tracing::debug!(error = %e, "audit line waits");
                            store = None;
                            std::thread::sleep(Duration::from_millis(200));
                        }
                        Err(e) => {
                            tracing::warn!(error = %e, act = line.act.name(), "audit line not kept");
                            store = None;
                            break;
                        }
                    }
                }
            }
        });
        std::sync::Mutex::new(tx)
    });
    if let Ok(tx) = writer.lock() {
        let _ = tx.send((at, audit));
    }
}

/// Drops the lines past 90 days.
pub fn prune() -> Result<(), vults_store::Error> {
    Store::open(&crate::history::dir())?.prune_audit(crate::history::unix_now())
}
