//! The chat's agents behind [`AgentDriver`] (`docs/dev/plan-zeca.md` S8, S11): Claude Code, one
//! process per turn, and Codex, through its app-server or, when that can't start, one read-only
//! `codex exec` per turn. Each is made from a [`Spec`] and stops through a shared [`Stop`].

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use tokio::sync::Notify;
use vults_agents::driver::{AgentDriver, BoxFuture, Input, Interrupter, Sink, Spec};

use crate::{claude, codex, codex_server};

/// The driver for a CLI the chat runs, with `spec`; none for an API (it is not an agent).
pub fn for_cli(provider: crate::Provider, spec: Spec) -> Option<Box<dyn AgentDriver>> {
    match provider {
        crate::Provider::Claude => Some(Box::new(ClaudeDriver::new(spec, None))),
        crate::Provider::Codex => Some(Box::new(CodexDriver {
            spec,
            session: None,
            server: None,
            stop: Arc::default(),
        })),
        crate::Provider::ClaudeApi => None,
    }
}

/// A press on Stop, shared between whoever holds the [`Interrupter`] and the running turn. It
/// stays pressed until the turn it stopped has ended: a press just before a turn starts still
/// stops it.
#[derive(Debug, Default)]
pub struct Stop {
    pressed: AtomicBool,
    wake: Notify,
}

impl Stop {
    pub(crate) fn pressed(&self) -> bool {
        self.pressed.load(Ordering::SeqCst)
    }

    /// Resolves once Stop is pressed (at once if it already was).
    pub(crate) async fn wait(&self) {
        loop {
            let woken = self.wake.notified();
            tokio::pin!(woken);
            woken.as_mut().enable();
            if self.pressed() {
                return;
            }
            woken.await;
        }
    }

    /// The turn is over: the next one starts unpressed.
    pub(crate) fn release(&self) {
        self.pressed.store(false, Ordering::SeqCst);
    }
}

impl Interrupter for Stop {
    fn interrupt(&self) -> BoxFuture<'_, Result<(), String>> {
        self.pressed.store(true, Ordering::SeqCst);
        self.wake.notify_waiters();
        Box::pin(async { Ok(()) })
    }
}

#[derive(Debug)]
pub struct ClaudeDriver {
    spec: Spec,
    session: Option<String>,
    stop: Arc<Stop>,
}

impl ClaudeDriver {
    fn new(spec: Spec, session: Option<String>) -> Self {
        Self {
            spec,
            session,
            stop: Arc::default(),
        }
    }

    /// A driver whose conversation already started (its next turn resumes `session`).
    pub fn resumed(spec: Spec, session: &str) -> Self {
        Self::new(spec, Some(session.to_owned()))
    }
}

impl AgentDriver for ClaudeDriver {
    fn spec(&self) -> &Spec {
        &self.spec
    }

    fn interrupter(&self) -> Arc<dyn Interrupter> {
        self.stop.clone()
    }

    fn start(&mut self) -> BoxFuture<'_, Result<(), String>> {
        Box::pin(async { Ok(()) })
    }

    fn send<'a>(&'a mut self, input: &'a Input, sink: &'a dyn Sink) -> BoxFuture<'a, Result<(), String>> {
        Box::pin(async move {
            let result = claude::turn(&self.spec, self.session.as_deref(), input, sink, &self.stop).await;
            self.stop.release();
            // An interrupted or failed turn that got a session id keeps it: the next one resumes.
            let (session, result) = result;
            if session.is_some() {
                self.session = session;
            }
            result
        })
    }

    fn session(&self) -> Option<&str> {
        self.session.as_deref()
    }
}

#[derive(Debug)]
pub struct CodexDriver {
    spec: Spec,
    session: Option<String>,
    /// The long-lived app-server, when this Codex has one.
    server: Option<codex_server::AppServer>,
    stop: Arc<Stop>,
}

impl AgentDriver for CodexDriver {
    fn spec(&self) -> &Spec {
        &self.spec
    }

    fn interrupter(&self) -> Arc<dyn Interrupter> {
        self.stop.clone()
    }

    fn start(&mut self) -> BoxFuture<'_, Result<(), String>> {
        Box::pin(async move {
            if self.server.is_none() {
                let server = codex_server::AppServer::start(&self.spec, self.session.as_deref()).await?;
                self.session = Some(server.thread.clone());
                self.server = Some(server);
            }
            Ok(())
        })
    }

    fn send<'a>(&'a mut self, input: &'a Input, sink: &'a dyn Sink) -> BoxFuture<'a, Result<(), String>> {
        Box::pin(async move {
            let result = self.turn(input, sink).await;
            self.stop.release();
            result
        })
    }

    fn session(&self) -> Option<&str> {
        self.session.as_deref()
    }
}

impl CodexDriver {
    async fn turn(&mut self, input: &Input, sink: &dyn Sink) -> Result<(), String> {
        if self.start().await.is_err() {
            // Without an app-server nothing can ask the user, so this fallback stays read-only.
            return codex::turn(&self.spec, &mut self.session, input, sink, &self.stop).await;
        }
        // start() succeeded, so the server is there; the guard only avoids a panic.
        let Some(server) = self.server.as_mut() else {
            return Err("Codex: no app-server".into());
        };
        let result = server.turn(input, sink, &self.stop).await;
        if result.is_err() {
            // A dead or confused server is restarted next turn and resumes the same thread.
            self.server = None;
        }
        result.map_err(|e| format!("Codex: {e}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn a_press_stays_until_the_turn_ends() {
        let stop = Arc::new(Stop::default());
        stop.interrupt().await.unwrap();
        // A turn that starts after the press still sees it.
        tokio::time::timeout(std::time::Duration::from_secs(1), stop.wait())
            .await
            .unwrap();
        stop.release();
        assert!(!stop.pressed());
        let waiting = tokio::spawn({
            let stop = stop.clone();
            async move { stop.wait().await }
        });
        tokio::task::yield_now().await;
        stop.interrupt().await.unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(1), waiting)
            .await
            .unwrap()
            .unwrap();
    }
}
