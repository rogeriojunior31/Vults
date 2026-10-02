//! The user's first name, from the full name on their account (GECOS on Linux), so Zeca can greet
//! them by it. Login handles never pass for a name: no name is better than "Hi, rogeradas".

use std::sync::OnceLock;

/// Read once: the account does not change while the app runs.
pub fn first_name() -> Option<String> {
    static NAME: OnceLock<Option<String>> = OnceLock::new();
    NAME.get_or_init(|| full_name().as_deref().and_then(first_name_of))
        .clone()
}

/// The first word of a full name, unless the whole thing looks like a login handle.
fn first_name_of(full: &str) -> Option<String> {
    // GECOS: "Full Name,room,phone,…".
    let full = full.split(',').next()?.trim();
    let first = full.split_whitespace().next()?;
    let one_lowercase_word = !full.contains(' ') && first.chars().all(|c| !c.is_uppercase());
    let handle_like = first
        .chars()
        .any(|c| c.is_ascii_digit() || matches!(c, '.' | '_' | '@'));
    if one_lowercase_word || handle_like || first.chars().count() > 32 {
        return None;
    }
    Some(first.to_string())
}

#[cfg(target_os = "linux")]
fn full_name() -> Option<String> {
    use std::ffi::CStr;
    let mut buf = vec![0u8; 4096];
    // SAFETY: an all-zero passwd is valid (null pointers, zero ids) and getpwuid_r fills it in.
    let mut pwd: libc::passwd = unsafe { std::mem::zeroed() };
    let mut found: *mut libc::passwd = std::ptr::null_mut();
    // SAFETY: every pointer is valid for the call; strings in `pwd` point into `buf`, which
    // outlives their use below.
    let rc = unsafe {
        libc::getpwuid_r(
            libc::getuid(),
            &mut pwd,
            buf.as_mut_ptr().cast(),
            buf.len(),
            &mut found,
        )
    };
    if rc != 0 || found.is_null() || pwd.pw_gecos.is_null() {
        return None;
    }
    // SAFETY: non-null and NUL-terminated inside `buf`.
    let gecos = unsafe { CStr::from_ptr(pwd.pw_gecos) };
    Some(gecos.to_string_lossy().into_owned())
}

#[cfg(not(target_os = "linux"))]
fn full_name() -> Option<String> {
    None
}

#[cfg(test)]
mod tests {
    use super::first_name_of;

    #[test]
    fn a_full_name_gives_its_first_word() {
        assert_eq!(first_name_of("Rogerio Junior").as_deref(), Some("Rogerio"));
        assert_eq!(
            first_name_of("Ana Maria Souza,Room 1,555").as_deref(),
            Some("Ana")
        );
        assert_eq!(first_name_of("Zoe").as_deref(), Some("Zoe"));
        assert_eq!(first_name_of("Jean-Luc Picard").as_deref(), Some("Jean-Luc"));
    }

    #[test]
    fn a_login_handle_is_no_name() {
        for handle in [
            "",
            "rogeradas",
            "dev.ops",
            "jdoe42",
            "me@host",
            "build_bot",
            "build-bot",
            ",Room 1",
        ] {
            assert_eq!(first_name_of(handle), None, "{handle:?}");
        }
        assert_eq!(first_name_of(&"A".repeat(33)), None);
    }
}
