//! The chat's agents behind [`AgentDriver`] (`docs/dev/plan-zeca.md` S8): Claude Code, one process
//! per turn, and Codex, through its app-server or, when that can't start, one read-only `codex exec`
//! per turn.

use std::path::PathBuf;

use vults_agents::driver::{AgentDriver, BoxFuture, Input, Sink};

use crate::{claude, codex, codex_server};

/// The driver for a CLI the chat runs, in `dir`; none for an API (it is not an agent).
pub fn for_cli(provider: crate::Provider, dir: PathBuf) -> Option<Box<dyn AgentDriver>> {
    match provider {
        crate::Provider::Claude => Some(Box::new(ClaudeDriver { dir, session: None })),
        crate::Provider::Codex => Some(Box::new(CodexDriver {
            dir,
            session: None,
            server: None,
        })),
        crate::Provider::ClaudeApi => None,
    }
}

#[derive(Debug)]
pub struct ClaudeDriver {
    dir: PathBuf,
    session: Option<String>,
}

impl ClaudeDriver {
    /// A driver whose conversation already started (its next turn resumes `session`).
    pub fn resumed(dir: PathBuf, session: &str) -> Self {
        Self {
            dir,
            session: Some(session.to_owned()),
        }
    }
}

impl AgentDriver for ClaudeDriver {
    fn name(&self) -> &'static str {
        "Claude"
    }

    fn start(&mut self) -> BoxFuture<'_, Result<(), String>> {
        Box::pin(async { Ok(()) })
    }

    fn send<'a>(&'a mut self, input: &'a Input, sink: &'a dyn Sink) -> BoxFuture<'a, Result<(), String>> {
        Box::pin(async move {
            let session = claude::turn(&self.dir, self.session.as_deref(), input, sink).await?;
            self.session = Some(session);
            Ok(())
        })
    }

    /// Its process lives only as long as its turn: dropping the turn ended it.
    fn interrupt(&mut self) -> BoxFuture<'_, Result<(), String>> {
        Box::pin(async { Ok(()) })
    }

    fn session(&self) -> Option<&str> {
        self.session.as_deref()
    }
}

#[derive(Debug)]
pub struct CodexDriver {
    dir: PathBuf,
    session: Option<String>,
    /// The long-lived app-server, when this Codex has one.
    server: Option<codex_server::AppServer>,
}

impl AgentDriver for CodexDriver {
    fn name(&self) -> &'static str {
        "Codex"
    }

    fn start(&mut self) -> BoxFuture<'_, Result<(), String>> {
        Box::pin(async move {
            if self.server.is_none() {
                let server = codex_server::AppServer::start(&self.dir, self.session.as_deref()).await?;
                self.session = Some(server.thread.clone());
                self.server = Some(server);
            }
            Ok(())
        })
    }

    fn send<'a>(&'a mut self, input: &'a Input, sink: &'a dyn Sink) -> BoxFuture<'a, Result<(), String>> {
        Box::pin(async move {
            if self.start().await.is_err() {
                // Without an app-server nothing can ask the user, so this fallback stays read-only.
                return codex::turn(&self.dir, &mut self.session, input, sink).await;
            }
            let Some(server) = self.server.as_mut() else {
                return Err("Codex: no app-server".into());
            };
            let result = server.turn(input, sink).await;
            if result.is_err() {
                // A dead or confused server is restarted next turn and resumes the same thread.
                self.server = None;
            }
            result.map_err(|e| format!("Codex: {e}"))
        })
    }

    /// A server stopped mid-turn is in no state to go on: the next turn starts a fresh one on the
    /// same thread.
    fn interrupt(&mut self) -> BoxFuture<'_, Result<(), String>> {
        self.server = None;
        Box::pin(async { Ok(()) })
    }

    fn session(&self) -> Option<&str> {
        self.session.as_deref()
    }
}
