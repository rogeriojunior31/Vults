//! The only URLs the app opens: https, on an allowed host, nothing hidden in them.

use std::fmt;

/// Hosts a connector may link to.
const HOSTS: &[&str] = &["github.com"];
const MAX_LEN: usize = 2048;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SafeUrl(String);

impl SafeUrl {
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
