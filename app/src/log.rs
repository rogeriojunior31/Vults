//! The app's log: `~/.local/state/vultures-ai/logs/` (one file a day, the last five kept), and
//! stderr in debug builds. It records what happened, never what it was about: event names,
//! decisions, connector and chat outcomes, errors; no commands, paths, payloads or chat text.

use std::path::PathBuf;

use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

pub fn dir() -> PathBuf {
    let base = std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| crate::paths::home().join(".local").join("state"));
    base.join(vultures_ai_brand::SLUG).join("logs")
}

/// Starts logging. Keep the guard alive for the app's lifetime, or buffered lines are lost.
pub fn init() -> Option<WorkerGuard> {
    // zbus logs its own D-Bus cache misses at warn; they are noise here.
    let filter =
        EnvFilter::try_from_env("VULTURES_AI_LOG").unwrap_or_else(|_| EnvFilter::new("info,zbus=error"));
    let file = tracing_appender::rolling::Builder::new()
        .rotation(tracing_appender::rolling::Rotation::DAILY)
        .filename_prefix(vultures_ai_brand::SLUG)
        .filename_suffix("log")
        .max_log_files(5)
        .build(dir())
        .ok();
    let (writer, guard) = match file {
        Some(f) => {
            let (w, g) = tracing_appender::non_blocking(f);
            (Some(w), Some(g))
        }
        None => (None, None),
    };
    let to_file = writer.map(|w| fmt::layer().with_ansi(false).with_target(false).with_writer(w));
    let to_stderr =
        cfg!(debug_assertions).then(|| fmt::layer().with_target(false).with_writer(std::io::stderr));
    let _ = tracing_subscriber::registry()
        .with(filter)
        .with(to_file)
        .with(to_stderr)
        .try_init();
    guard
}
