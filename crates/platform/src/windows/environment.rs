//! The PATH Windows keeps for new sign-ins (the user's, then the machine's), as `setx` and
//! installers such as winget write it. A program started earlier (the app, from Explorer's
//! environment) never sees what was installed after the user signed in.

use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use std::path::PathBuf;

use windows::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, RRF_RT_REG_EXPAND_SZ, RRF_RT_REG_SZ, RegGetValueW,
};
use windows::core::{HSTRING, PCWSTR};

/// The folders of the saved PATH, the user's first, `%VARS%` expanded; none it can't read.
pub fn saved_path() -> Vec<PathBuf> {
    let user = read(HKEY_CURRENT_USER, "Environment");
    let machine = read(
        HKEY_LOCAL_MACHINE,
        r"SYSTEM\CurrentControlSet\Control\Session Manager\Environment",
    );
    [user, machine]
        .into_iter()
        .flatten()
        .flat_map(|path| std::env::split_paths(&path).collect::<Vec<_>>())
        .filter(|dir| dir.is_absolute())
        .collect()
}

fn read(root: HKEY, key: &str) -> Option<OsString> {
    let (key, name) = (HSTRING::from(key), HSTRING::from("Path"));
    // REG_EXPAND_SZ comes back expanded (no RRF_NOEXPAND).
    let flags = RRF_RT_REG_SZ | RRF_RT_REG_EXPAND_SZ;
    let mut bytes = 0u32;
    // SAFETY: a size query with no buffer; the strings outlive the call.
    let sized = unsafe {
        RegGetValueW(
            root,
            PCWSTR(key.as_ptr()),
            PCWSTR(name.as_ptr()),
            flags,
            None,
            None,
            Some(&mut bytes),
        )
    };
    // The longest PATH Windows edits is 32 767 characters; anything far past it is not one.
    if sized.is_err() || bytes == 0 || bytes > 256 * 1024 {
        return None;
    }
    let mut buffer = vec![0u16; (bytes as usize).div_ceil(2)];
    // SAFETY: `buffer` holds `bytes` bytes, as the query said.
    let read = unsafe {
        RegGetValueW(
            root,
            PCWSTR(key.as_ptr()),
            PCWSTR(name.as_ptr()),
            flags,
            None,
            Some(buffer.as_mut_ptr().cast()),
            Some(&mut bytes),
        )
    };
    if read.is_err() {
        return None;
    }
    let len = buffer.iter().position(|&c| c == 0).unwrap_or(buffer.len());
    Some(OsString::from_wide(&buffer[..len]))
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_saved_path_holds_the_system_folder() {
        let windows = std::env::var_os("SystemRoot")
            .map(std::path::PathBuf::from)
            .unwrap();
        let path = super::saved_path();
        assert!(
            path.iter().any(|d| d
                .as_os_str()
                .eq_ignore_ascii_case(windows.join("system32").as_os_str())),
            "{path:?}"
        );
    }
}
