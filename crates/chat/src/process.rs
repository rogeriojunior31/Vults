//! CLI launchers: preserve argument boundaries and avoid Windows console flashes.
use std::ffi::OsStr;
#[cfg(windows)]
use std::path::{Path, PathBuf};
use tokio::process::Command;

pub(crate) fn command(program: impl AsRef<OsStr>) -> Command {
    let program = program.as_ref();
    #[cfg(windows)]
    let executable = std::env::var_os("PATH")
        .and_then(|path| resolve(program, &path))
        .unwrap_or_else(|| PathBuf::from(program));
    #[cfg(not(windows))]
    let executable = program;
    let command = Command::new(executable);
    #[cfg(windows)]
    {
        let mut command = command;
        command.creation_flags(0x0800_0000);
        command
    }
    #[cfg(not(windows))]
    command
}

#[cfg(windows)]
fn resolve(program: &OsStr, path: &OsStr) -> Option<PathBuf> {
    let name = Path::new(program);
    if name.components().count() != 1 || name.extension().is_some() {
        return None;
    }
    std::env::split_paths(path)
        .filter(|dir| dir.is_absolute())
        .find_map(|dir| {
            ["exe", "cmd", "bat"]
                .into_iter()
                .map(|extension| dir.join(name).with_extension(extension))
                .find(|file| file.is_file())
        })
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    #[test]
    fn native_executables_and_npm_shims_are_found_without_running_a_shell_search() {
        let dir = std::env::temp_dir().join(format!("vults-cli-path-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let shim = dir.join("cli.cmd");
        std::fs::write(&shim, "@echo off\r\n").unwrap();
        assert_eq!(resolve(OsStr::new("cli"), dir.as_os_str()), Some(shim.clone()));
        let exe = dir.join("cli.exe");
        std::fs::write(&exe, "fixture").unwrap();
        assert_eq!(resolve(OsStr::new("cli"), dir.as_os_str()), Some(exe.clone()));
        assert_eq!(resolve(OsStr::new("relative/cli"), dir.as_os_str()), None);
        std::fs::remove_file(shim).unwrap();
        std::fs::remove_file(exe).unwrap();
        std::fs::remove_dir(dir).unwrap();
    }
}
