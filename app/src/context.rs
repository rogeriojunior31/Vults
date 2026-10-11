//! Zeca's context for a chat turn (`docs/dev/plan-zeca.md` M2): the frozen part (his memory as the
//! conversation begins) and the volatile part (`<today>`, `<flock>`) in front of each turn.

use std::path::Path;

use tauri::AppHandle;
use vults_zeca::context::{self, Now};
use vults_zeca::flock::{Flock, FlockReader};
use vults_zeca::memory::Memory;

use crate::flock_tools::Source;

/// The frozen part, from the memory in `dir`. A memory never written is not made here: reading
/// it must leave no folder behind (M6's first accepted proposal makes it).
pub fn frozen(dir: &Path) -> String {
    let memory = if dir.join(".git").exists() {
        Memory::open(dir)
            .inspect_err(|e| tracing::warn!("{e}: the chat starts without it"))
            .ok()
    } else {
        None
    };
    // A memory that fails to read leaves the conversation without it, never without a reply.
    context::frozen(memory.as_ref(), None)
        .or_else(|e| {
            tracing::warn!("{e}: the chat starts without it");
            context::frozen(None, None)
        })
        .unwrap_or_default()
}

/// The volatile part, now: the user's clock and the island's flock.
pub fn volatile(app: &AppHandle) -> String {
    let unix = crate::history::unix_now();
    let now = Now {
        unix,
        offset: crate::history::local_offset(unix),
    };
    let flock = app
        .view()
        .map(|v| crate::flock_tools::flock(&v))
        .unwrap_or_else(Flock::default);
    context::volatile(now, &FlockReader::new(flock), &[])
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn reading_a_memory_never_written_makes_nothing() {
        let dir = std::env::temp_dir().join(format!("vults-context-app-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let f = frozen(&dir);
        assert!(f.starts_with("The app may begin a message with <today>"), "{f}");
        assert!(!dir.exists());
    }
}
