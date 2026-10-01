//! Where the app keeps its own files: `~/.local/share/vultures-ai/` (Linux),
//! `%LOCALAPPDATA%\Vultures AI\` (Windows).

use std::path::PathBuf;

pub fn home() -> PathBuf {
    std::env::home_dir().unwrap_or_else(|| PathBuf::from("."))
}

pub fn data_dir() -> PathBuf {
    if cfg!(windows) {
        let base = std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(home);
        return base.join(vultures_ai_brand::NAME);
    }
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home().join(".local").join("share"))
        .join(vultures_ai_brand::SLUG)
}

/// The hook relay the agents' configs point at: a stable path, whatever rebuilds the app.
pub fn hook_exe() -> PathBuf {
    data_dir().join("bin").join(vultures_ai_brand::HOOK_EXE)
}

/// An empty folder for chat turns, so no project's instructions or settings leak in.
pub fn chat_dir() -> PathBuf {
    data_dir().join("chat")
}

/// Copies of the files dropped on the island.
pub fn inbox_dir() -> PathBuf {
    data_dir().join("inbox")
}
