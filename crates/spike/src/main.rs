//! `vults-spike`: the kit that records a real agent's hooks without an account
//! (`docs/dev/plan-every-agent.md`, F2). `scripts/spike.sh` runs it end to end for each agent;
//! the commands are its parts:
//!
//! ```text
//! vults-spike model <port> <script.json> <workdir> [requests.jsonl]
//! vults-spike app <runtime-dir> <events.jsonl> <allow|deny|none>
//! vults-spike install <agent> <home> <hook-exe>
//! vults-spike replay <events.jsonl>
//! vults-spike fixture <events.jsonl> <workdir> [as-dir]
//! vults-spike tty <seconds> [--screen <file>] [--keys <text> --keys-at <seconds>] -- <cmd...>
//! vults-spike codex-trust <codex> <home> <workdir>
//! ```
//!
//! Dev only, never in the app; Linux, like the rest of the work on agents for now.

#[cfg(unix)]
mod app;
#[cfg(unix)]
mod codex;
mod model;
mod record;
#[cfg(unix)]
mod tty;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("vults-spike: {e}");
            ExitCode::FAILURE
        }
    }
}

fn arg<'a>(args: &'a [String], i: usize, what: &str) -> Result<&'a str, String> {
    args.get(i)
        .map(String::as_str)
        .ok_or_else(|| format!("missing {what}"))
}

fn run(args: &[String]) -> Result<(), String> {
    let cmd = arg(args, 0, "a command (see the source's first lines)")?;
    match cmd {
        "model" => {
            let port = arg(args, 1, "port")?.parse().map_err(|_| "bad port")?;
            let script = std::fs::read_to_string(arg(args, 2, "script")?).map_err(|e| e.to_string())?;
            let script = serde_json::from_str(&script).map_err(|e| format!("script: {e}"))?;
            let log = args.get(4).map(Path::new);
            model::serve(port, script, PathBuf::from(arg(args, 3, "workdir")?), log)
                .map_err(|e| e.to_string())
        }
        "install" => record::install(
            arg(args, 1, "agent")?,
            Path::new(arg(args, 2, "home")?),
            Path::new(arg(args, 3, "hook exe")?),
        ),
        "replay" => record::replay(Path::new(arg(args, 1, "events")?)),
        "fixture" => record::fixture(
            Path::new(arg(args, 1, "events")?),
            arg(args, 2, "workdir")?,
            args.get(3).map_or("/home/me/notes", String::as_str),
        ),
        #[cfg(unix)]
        "app" => app::serve(
            Path::new(arg(args, 1, "runtime dir")?),
            Path::new(arg(args, 2, "events log")?),
            arg(args, 3, "answer")?,
        )
        .map_err(|e| e.to_string()),
        #[cfg(unix)]
        "tty" => {
            let secs: f64 = arg(args, 1, "seconds")?.parse().map_err(|_| "bad seconds")?;
            let split = args
                .iter()
                .position(|a| a == "--")
                .ok_or("missing -- before the command")?;
            let opt = |name: &str| {
                args[..split]
                    .iter()
                    .position(|a| a == name)
                    .and_then(|i| args.get(i + 1))
            };
            let keys_at = opt("--keys-at")
                .and_then(|s| s.parse::<f64>().ok())
                .unwrap_or(1.0);
            let keys = opt("--keys").map(|k| (k.replace("\\r", "\r").replace("\\n", "\n"), keys_at));
            tty::run(
                &args[split + 1..],
                std::time::Duration::from_secs_f64(secs),
                opt("--screen").map(Path::new),
                keys.as_ref()
                    .map(|(k, at)| (k.as_bytes(), std::time::Duration::from_secs_f64(*at))),
            )
            .map_err(|e| e.to_string())
        }
        #[cfg(unix)]
        "codex-trust" => codex::trust(
            arg(args, 1, "codex")?,
            Path::new(arg(args, 2, "home")?),
            Path::new(arg(args, 3, "workdir")?),
        ),
        other => Err(format!("unknown command {other}")),
    }
}
