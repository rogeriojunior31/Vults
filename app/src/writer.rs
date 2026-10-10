//! One thread writes to the local database for the app's own records (the audit log, traces), in
//! arrival order: an Always and the rule line after it never swap, and a line that meets a busy
//! database (a start-up import, a clear) waits its turn instead of failing.

use std::sync::OnceLock;
use std::sync::mpsc::{Sender, channel};
use std::time::Duration;

use vults_core::audit::Audit;
use vults_store::Store;
use vults_store::traces::Span;

pub enum Job {
    /// An audit line, at its time (Unix seconds).
    Audit(i64, Audit),
    Span(Span),
}

impl Job {
    fn write(&self, store: &mut Store) -> Result<(), vults_store::Error> {
        match self {
            Self::Audit(at, line) => store.append_audit(*at, line),
            Self::Span(span) => store.append_span(span),
        }
    }

    fn what(&self) -> &'static str {
        match self {
            Self::Audit(..) => "audit line",
            Self::Span(_) => "trace span",
        }
    }
}

static WRITER: OnceLock<std::sync::Mutex<Sender<Job>>> = OnceLock::new();

/// Queues a write; off the caller's thread. A write that fails a few times is logged, never retried
/// later: what it records already happened.
pub fn send(job: Job) {
    let writer = WRITER.get_or_init(|| {
        let (tx, rx) = channel::<Job>();
        std::thread::spawn(move || {
            let mut store: Option<Store> = None;
            for job in rx {
                for tries in 0.. {
                    let result = match store.as_mut() {
                        Some(s) => job.write(s),
                        None => Store::open(&crate::history::dir()).and_then(|mut s| {
                            let r = job.write(&mut s);
                            store = Some(s);
                            r
                        }),
                    };
                    match result {
                        Ok(()) => break,
                        Err(e) if tries < 5 => {
                            tracing::debug!(error = %e, what = job.what(), "write waits");
                            store = None;
                            std::thread::sleep(Duration::from_millis(200));
                        }
                        Err(e) => {
                            tracing::warn!(error = %e, what = job.what(), "not written");
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
        let _ = tx.send(job);
    }
}
