//! Quick actions that open a session's folder or a changed file: in VS Code when `code` is on the
//! PATH, else the folder in the file manager (ADR 0011: where a terminal's window cannot be raised,
//! this is what works). The paths come from hook payloads, so no shell ever sees them: only an
//! existing folder or file, by its absolute path, goes on as one argument.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use tauri::AppHandle;

/// `code`, when it is installed.
pub fn editor() -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|dir| dir.join("code"))
        .find(|p| p.is_file())
}

/// The session's folder, in the editor or the file manager.
pub fn folder(app: &AppHandle, path: &str) {
    let Some(dir) = existing(path, Path::is_dir) else {
        tracing::info!("quick action: no such folder");
        return;
    };
    if spawn(editor(), [dir.as_os_str()]) {
        return;
    }
    use tauri_plugin_opener::OpenerExt;
    let _ = app.opener().open_path(dir.to_string_lossy(), None::<&str>);
}

/// A changed file, at its line, in the editor; without one, shown in its folder. Never opened
/// with whatever handles its type: that could run it.
pub fn file(app: &AppHandle, path: &str, line: Option<u32>) {
    let Some(file) = existing(path, Path::is_file) else {
        tracing::info!("quick action: no such file");
        return;
    };
    let at = match line {
        Some(n) => format!("{}:{n}", file.display()),
        None => file.display().to_string(),
    };
    if spawn(editor(), [std::ffi::OsStr::new("-g"), std::ffi::OsStr::new(&at)]) {
        return;
    }
    use tauri_plugin_opener::OpenerExt;
    let _ = app.opener().reveal_item_in_dir(&file);
}

/// Only an absolute path to something that is there, of the kind asked for.
fn existing(path: &str, kind: impl Fn(&Path) -> bool) -> Option<PathBuf> {
    let p = Path::new(path);
    (p.is_absolute() && kind(p)).then(|| p.to_path_buf())
}

fn spawn<'a>(program: Option<PathBuf>, args: impl IntoIterator<Item = &'a std::ffi::OsStr>) -> bool {
    let Some(program) = program else { return false };
    Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        // Reaped off the loop: the launcher hands over to a running editor and exits.
        .map(|mut child| std::thread::spawn(move || child.wait()))
        .is_ok()
}

/// Whether the quick actions open things in VS Code, for their labels.
#[tauri::command]
pub fn editor_found() -> bool {
    editor().is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_an_existing_absolute_path_of_the_right_kind_goes_on() {
        let dir = std::env::temp_dir();
        let dir = dir.to_str().unwrap();
        assert!(existing(dir, Path::is_dir).is_some());
        assert!(existing(dir, Path::is_file).is_none(), "a folder is not a file");
        assert!(existing("relative/dir", |_| true).is_none());
        assert!(existing("--new-window", |_| true).is_none());
        assert!(existing("/surely/not/here/at/all", Path::is_dir).is_none());
    }
}
