//! One thread writes to the local database for the app's own records (the audit log, traces), in
//! arrival order: an Always and the rule line after it never swap, and a line that meets a busy
//! database (a start-up import, a clear) waits its turn instead of failing.

use std::sync::OnceLock;
use std::sync::mpsc::{Sender, channel};
use std::time::Duration;

use vults_core::audit::Audit;
use vults_store::Store;
use vults_store::conversations::ConversationId;
use vults_store::traces::Span;

pub enum Job {
    Barrier(tokio::sync::oneshot::Sender<()>),
    /// An audit line, at its time (Unix seconds).
    Audit(i64, Audit),
    Span(Span),
    FinishRequest(Span),
    /// A line said in one of Zeca's conversations: who, the text, the files' names, when (ms).
    Say(ConversationId, &'static str, String, Vec<String>, i64),
    /// What resumes a conversation after a turn: the CLI's session, the API's history, the tool
    /// that brought outside content in (Z8), when (ms).
    Save(
        ConversationId,
        Option<String>,
        Option<String>,
        Option<String>,
        i64,
    ),
}

impl Job {
    fn write(&self, store: &mut Store) -> Result<(), vults_store::Error> {
        match self {
            Self::Barrier(..) => Ok(()),
            Self::Audit(at, line) => store.append_audit(*at, line),
            Self::Span(span) => store.append_span(span),
            Self::FinishRequest(span) => store.finish_request(span),
            Self::Say(id, who, text, files, ms) => store.conversation_say(*id, who, text, files, *ms),
            Self::Save(id, session, history, outside, ms) => store.conversation_save(
                *id,
                session.as_deref(),
                history.as_deref(),
                outside.as_deref(),
                *ms,
            ),
        }
    }

    fn what(&self) -> &'static str {
        match self {
            Self::Barrier(..) => "barrier",
            Self::Audit(..) => "audit line",
            Self::Span(_) | Self::FinishRequest(_) => "trace span",
            Self::Say(..) | Self::Save(..) => "conversation",
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
                if let Job::Barrier(reply) = job {
                    let _ = reply.send(());
                    continue;
                }
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

/// Wait until earlier writes have completed before exposing a resumable session.
pub async fn flush() {
    let (tx, rx) = tokio::sync::oneshot::channel();
    send(Job::Barrier(tx));
    let _ = rx.await;
}
