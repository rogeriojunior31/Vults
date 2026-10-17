//! Zeca's context for a chat turn (`docs/dev/plan-zeca.md` M2): the frozen part (his memory as the
//! conversation begins) and the volatile part (`<today>`, `<flock>`) in front of each turn.

use std::path::Path;

use vults_zeca::context::{self, Now};
use vults_zeca::flock::FlockReader;
use vults_zeca::memory::Memory;

use crate::flock_tools::Source;

/// The frozen part, from the memory in `dir` (none while the user has the memory off: off means
/// none of it reaches a model; a conversation already running keeps the part it began with until
/// it ends, and one reopened with the memory off gets this part again without the facts), with the
/// roost of the repository the conversation works in (M3).
/// A memory never written is not made here: reading it must leave no folder behind (M6's first
/// accepted proposal makes it).
pub fn frozen(dir: Option<&Path>, folder: &Path) -> String {
    let roost = crate::roosts::repo_of(folder).and_then(|r| r.memory());
    let memory = if let Some(dir) = dir.filter(|d| d.join(".git").exists()) {
        Memory::open(dir)
            .inspect_err(|e| tracing::warn!("{e}: the chat starts without it"))
            .ok()
    } else {
        None
    };
    // A memory that fails to read leaves the conversation without it, never without a reply.
    context::frozen(memory.as_ref(), roost.as_ref())
        .or_else(|e| {
            tracing::warn!("{e}: the chat starts without it");
            context::frozen(None, None)
        })
        .unwrap_or_default()
}

/// The volatile part, now: the user's clock, the island's flock and what recall found (M5).
pub fn volatile(source: &dyn Source, memories: &[context::Passage]) -> String {
    let unix = crate::history::unix_now();
    let now = Now {
        unix,
        offset: crate::history::local_offset(unix),
    };
    let flock = source
        .view()
        .map(|v| crate::flock_tools::flock(&v))
        .unwrap_or_default();
    context::volatile(now, &FlockReader::new(flock), memories)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn reading_a_memory_never_written_makes_nothing() {
        let dir = std::env::temp_dir().join(format!("vults-context-app-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let f = frozen(Some(&dir), &dir);
        assert!(f.starts_with("The app may begin a message with <today>"), "{f}");
        assert!(!dir.exists());
    }
}
