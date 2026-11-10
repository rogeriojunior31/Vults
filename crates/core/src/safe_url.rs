//! The only URLs the app opens: https, on an allowed host, nothing hidden in them.

use std::fmt;

/// Hosts a connector may link to.
const HOSTS: &[&str] = &["github.com", "gitlab.com"];
const MAX_LEN: usize = 2048;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SafeUrl(String);

impl SafeUrl {
    /// Canonical base explicitly configured for a self-hosted service, including a subfolder.
    pub fn service_base(raw: &str) -> Option<String> {
        if raw.len() > 1024
            || raw
                .chars()
                .any(|c| c.is_control() || c.is_whitespace() || matches!(c, '\\' | '%' | '?' | '#'))
        {
            return None;
        }
        let url = url::Url::parse(raw).ok()?;
        if raw.split('/').nth(2)?.contains('@') {
            return None;
        }
        if url.scheme() != "https"
            || url.host_str().is_none()
            || !url.username().is_empty()
            || url.password().is_some()
            || raw.split('/').any(|part| matches!(part, "." | ".."))
        {
            return None;
        }
        let path = url.path();
        if path.split('/').any(|part| {
            !part
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'-' | b'_' | b'.'))
        }) {
            return None;
        }
        Some(format!("{}/", url.as_str().trim_end_matches('/')))
    }

    /// Only links under this particular user's service base; no global host allowlist change.
    pub fn for_service(raw: &str, base: &str) -> Option<Self> {
        let base = Self::service_base(base)?;
        let suffix = raw.strip_prefix(&base)?;
        if raw.len() > MAX_LEN
            || suffix.is_empty()
            || suffix.split('/').any(|part| {
                part.is_empty()
                    || matches!(part, "." | "..")
                    || !part
                        .bytes()
                        .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'-' | b'_' | b'.'))
            })
        {
            return None;
        }
        Some(Self(raw.into()))
    }

    pub fn parse(raw: &str) -> Option<Self> {
        if raw.len() > MAX_LEN
            || raw
                .chars()
                .any(|c| c.is_control() || c.is_whitespace() || c == '\\')
        {
            return None;
        }
        let rest = raw.strip_prefix("https://")?;
        let authority = rest.split(['/', '?', '#']).next()?;
        // No `user@host` (a classic way to dress up another host), no explicit port.
        if authority.contains('@') || authority.contains(':') {
            return None;
        }
        let host = authority.to_ascii_lowercase();
        HOSTS.contains(&host.as_str()).then(|| Self(raw.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SafeUrl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::SafeUrl;

    #[test]
    fn service_scope_keeps_ports_and_subfolders_without_trusting_other_hosts() {
        let base = SafeUrl::service_base("https://Git.Example.com:8443/team").unwrap();
        assert_eq!(base, "https://git.example.com:8443/team/");
        assert!(SafeUrl::for_service("https://git.example.com:8443/team/me/app/pulls/1", &base).is_some());
        for bad in [
            "http://git.example.com/",
            "https://me:secret@git.example.com/",
            "https://git.example.com/a/../b",
            "https://git.example.com/%2e/",
            "https://git.example.com/?x=1",
            "https://git.example.com/#fragment",
        ] {
            assert!(SafeUrl::service_base(bad).is_none(), "{bad}");
        }
        for bad in [
            "https://git.example.com:8443/teammate/a",
            "https://git.example.com/team/a",
            "https://git.example.com:8443/team/../a",
            "https://git.example.com:8443/team/a?x=1",
            "https://git.example.com.evil:8443/team/a",
        ] {
            assert!(SafeUrl::for_service(bad, &base).is_none(), "{bad}");
        }
        assert!(SafeUrl::parse("https://git.example.com:8443/team/a").is_none());
    }

    #[test]
    fn accepts_plain_github_links() {
        for ok in [
            "https://github.com/me/app/pull/12",
            "https://GitHub.com/me/app/commit/abc?x=1#y",
            "https://github.com",
        ] {
            assert!(SafeUrl::parse(ok).is_some(), "{ok}");
        }
    }

    #[test]
    fn gitlab_links_accept_subgroups_and_refuse_host_spoofing() {
        assert!(SafeUrl::parse("https://gitlab.com/team/sub/app/-/merge_requests/7").is_some());
        for bad in [
            "http://gitlab.com/a",
            "https://gitlab.com.evil.example/a",
            "https://gitlab.com@evil.example/a",
            "https://gitlab.com:8443/a",
            "https://gitlab.com\\@evil.example/a",
        ] {
            assert!(SafeUrl::parse(bad).is_none(), "{bad}");
        }
    }

    #[test]
    fn refuses_everything_else() {
        for bad in [
            "http://github.com/me",
            "https://github.com.evil.example/me",
            "https://evil.example/github.com",
            "https://github.com@evil.example/",
            "https://user:pass@github.com/",
            "https://github.com:8443/",
            "https://github.com/a b",
            "https://github.com/\nx",
            "https://github.com\\@evil.example",
            "javascript:alert(1)",
            "file:///etc/passwd",
            "",
        ] {
            assert!(SafeUrl::parse(bad).is_none(), "{bad:?}");
        }
    }
}
