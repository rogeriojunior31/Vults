//! Evals (`docs/dev/plan-zeca.md` S7): cases kept as files in `cases/`, run by
//! `cargo test -p vults-evals --features evals`. A change to a prompt, the policy or the redactor
//! that moves a case fails here, and is accepted on purpose (`VULTS_EVALS_BLESS=1` for the prompt
//! snapshots) or fixed. Recorded model streams (`cases/cassettes/`) replay through a stand-in CLI,
//! so no case costs a token.

use std::path::{Path, PathBuf};

/// The `cases/` folder.
pub fn cases() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("cases")
}
