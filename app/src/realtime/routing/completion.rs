//! Native receipts distinguish a submitted effect from an acknowledged result.
use super::*;
use vults_core::i18n::Lang;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Receipt {
    Completed,
    Submitted,
    Failed,
}
impl Receipt {
    pub fn submitted(ok: bool) -> Self {
        if ok { Self::Submitted } else { Self::Failed }
    }
}
impl From<bool> for Receipt {
    fn from(ok: bool) -> Self {
        if ok { Self::Completed } else { Self::Failed }
    }
}

struct Live(Arc<AtomicBool>);
impl Drop for Live {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}
pub(crate) struct Completion {
    pub report: Report,
    reply: Reply,
    router: Arc<Router>,
    deadline: Instant,
    live: Live,
    receipts: Vec<oneshot::Receiver<Receipt>>,
}
impl Completion {
    pub(in crate::realtime) fn new(
        reply: Reply,
        report: Report,
        router: Arc<Router>,
        deadline: Instant,
        waiting: Arc<AtomicBool>,
    ) -> Self {
        Self {
            report,
            reply,
            router,
            deadline: deadline.min(Instant::now() + std::time::Duration::from_secs(2)),
            live: Live(waiting),
            receipts: Vec::new(),
        }
    }
    pub fn work(&mut self) -> Work {
        let (reply, receive) = oneshot::channel();
        self.receipts.push(receive);
        Work {
            reply,
            router: self.router.clone(),
            deadline: self.deadline,
            live: self.live.0.clone(),
        }
    }
    pub fn finish(self, lang: Lang) {
        tauri::async_runtime::spawn(async move {
            let Self {
                mut report,
                reply,
                router: _router,
                deadline,
                live: _live,
                receipts,
            } = self;
            if !receipts.is_empty() || report.feedback == Feedback::Requested {
                let feedback = outcome(receipts, deadline).await;
                if feedback != Feedback::Done || report.feedback == Feedback::Requested {
                    report.feedback = feedback;
                    report.text = vults_core::i18n::voice_feedback(lang, report.feedback).into();
                }
            }
            let _ = reply.send(Ok(Some(report)));
        });
    }
}
pub(crate) struct Work {
    reply: oneshot::Sender<Receipt>,
    // A queued or detached native worker retains the exclusive conversation owner.
    router: Arc<Router>,
    deadline: Instant,
    live: Arc<AtomicBool>,
}
impl Work {
    pub fn run<R: tauri::Runtime>(self, app: &AppHandle<R>, action: impl FnOnce() -> Receipt) -> Receipt {
        let settings = app.state::<crate::settings::SettingsState>();
        let allowed = settings.0.lock().is_ok_and(|s| {
            (self.router.valid)(&s)
                && s.voice_realtime.enabled()
                && s.dictation_commands
                && self.live.load(Ordering::SeqCst)
                && !self.reply.is_closed()
                && Instant::now() < self.deadline
        });
        let result = if allowed { action() } else { Receipt::Failed };
        let _ = self.reply.send(result);
        result
    }
}
async fn outcome(receipts: Vec<oneshot::Receiver<Receipt>>, deadline: Instant) -> Feedback {
    if receipts.is_empty() {
        return Feedback::ExecutionFailed;
    }
    let mut unknown = false;
    for receive in receipts {
        match tokio::time::timeout_at(deadline.into(), receive).await {
            Ok(Ok(Receipt::Completed)) => {}
            Ok(Ok(Receipt::Failed) | Err(_)) => return Feedback::ExecutionFailed,
            Ok(Ok(Receipt::Submitted)) | Err(_) => unknown = true,
        }
    }
    if unknown {
        Feedback::Requested
    } else {
        Feedback::Done
    }
}

#[cfg(test)]
mod tests;
