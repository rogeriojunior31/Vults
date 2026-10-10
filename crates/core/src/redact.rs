//! Secrets out of text the app keeps (the audit log; later the journal and traces): known token
//! shapes, `NAME=value` where the name says secret, a bearer header, a password in a URL. A guard,
//! not a promise: an unusual secret may still pass, which is why what is kept stays on this machine.

const REDACTED: &str = "[redacted]";

/// Prefixes of tokens that are secret by shape alone.
const TOKEN_PREFIXES: &[&str] = &[
    "github_pat_",
    "ghp_",
    "gho_",
    "ghu_",
    "ghs_",
    "ghr_",
    "glpat-",
    "sk-ant-",
    "sk-proj-",
    "sk_live_",
    "sk_test_",
    "sk-",
    "xoxb-",
    "xoxp-",
    "xoxa-",
    "AKIA",
    "ASIA",
    "AIza",
    "hf_",
    "npm_",
];

/// Words that make a variable or flag name a secret's.
const SECRET_NAMES: &[&str] = &[
    "token",
    "secret",
    "password",
    "passwd",
    "apikey",
    "api_key",
    "credential",
    "private_key",
];

pub fn secrets(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while !rest.is_empty() {
        let mut word_end = rest.find(char::is_whitespace).unwrap_or(rest.len());
        // `NAME="a value with spaces"`: the word runs to the closing quote.
        if let Some((name, value)) = rest[..word_end].split_once('=')
            && is_secret_name(name)
            && let Some(q) = value.chars().next().filter(|c| *c == '"' || *c == '\'')
            && !value[1..].contains(q)
        {
            let open = name.len() + 1;
            if let Some(close) = rest[open + 1..].find(q) {
                word_end = open + 1 + close + 1;
            }
        }
        let (word, tail) = rest.split_at(word_end);
        out.push_str(&word_redacted(word));
        let ws_end = tail.find(|c: char| !c.is_whitespace()).unwrap_or(tail.len());
        out.push_str(&tail[..ws_end]);
        rest = &tail[ws_end..];
    }
    bearer(&out)
}

/// One whitespace-free word: `NAME=value`, a URL with a password, or a bare token.
fn word_redacted(word: &str) -> String {
    if let Some((name, value)) = word.split_once('=')
        && is_secret_name(name)
        && !value.is_empty()
    {
        let quote = value.chars().next().filter(|c| *c == '"' || *c == '\'');
        return match quote {
            Some(q) => format!("{name}={q}{REDACTED}{q}"),
            None => format!("{name}={REDACTED}"),
        };
    }
    if let Some(at) = url_password(word) {
        return at;
    }
    tokens(word)
}

fn is_secret_name(name: &str) -> bool {
    let name = name.trim_start_matches('-').to_ascii_lowercase();
    let name = name.rsplit(['.', ':']).next().unwrap_or(&name);
    !name.is_empty() && SECRET_NAMES.iter().any(|s| name.contains(s))
}

/// `scheme://user:pass@host…` with the password hidden.
fn url_password(word: &str) -> Option<String> {
    let scheme = word.find("://")? + 3;
    // The user info is before the host: no '/' may come before its '@'.
    let authority = word[scheme..].find('/').map_or(word.len(), |i| scheme + i);
    let at = scheme + word[scheme..authority].find('@')?;
    let colon = scheme + word[scheme..at].find(':')?;
    (colon + 1 < at).then(|| format!("{}{REDACTED}{}", &word[..=colon], &word[at..]))
}

/// Every run of token characters that starts with a known prefix and is long enough to be one.
fn tokens(word: &str) -> String {
    let mut out = String::with_capacity(word.len());
    let mut i = 0;
    let bytes = word.as_bytes();
    while i < word.len() {
        let start_of_run = i == 0 || !is_token_byte(bytes[i - 1]);
        let prefix = start_of_run
            .then(|| TOKEN_PREFIXES.iter().find(|p| word[i..].starts_with(**p)))
            .flatten();
        if let Some(p) = prefix {
            let len = word[i..].bytes().take_while(|b| is_token_byte(*b)).count();
            if len >= p.len() + 8 {
                out.push_str(REDACTED);
                i += len;
                continue;
            }
        }
        let c = word[i..].chars().next().unwrap_or(' ');
        out.push(c);
        i += c.len_utf8();
    }
    out
}

fn is_token_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b'-'
}

/// `Bearer <token>` (any case), as in an Authorization header.
fn bearer(text: &str) -> String {
    let lower = text.to_ascii_lowercase();
    let mut out = String::with_capacity(text.len());
    let mut last = 0;
    let mut from = 0;
    while let Some(pos) = lower[from..].find("bearer ") {
        let start = from + pos + "bearer ".len();
        let len = text[start..]
            .bytes()
            .take_while(|b| !b.is_ascii_whitespace() && *b != b'"' && *b != b'\'')
            .count();
        if len > 0 && !text[start..].starts_with(REDACTED) {
            out.push_str(&text[last..start]);
            out.push_str(REDACTED);
            last = start + len;
        }
        from = start + len.max(1).min(text.len() - start);
        if from >= text.len() {
            break;
        }
    }
    out.push_str(&text[last..]);
    out
}

#[cfg(test)]
mod tests {
    use super::secrets;

    #[test]
    fn known_tokens_go() {
        assert_eq!(
            secrets("gh auth login --with-token ghp_abcdefghijklmnopqrstuvwxyz0123456789"),
            "gh auth login --with-token [redacted]"
        );
        assert_eq!(
            secrets("export KEY_ID=AKIAIOSFODNN7EXAMPLE"),
            "export KEY_ID=[redacted]"
        );
        assert_eq!(secrets("curl -d sk-ant-api03-abcdefghijkl"), "curl -d [redacted]");
    }

    #[test]
    fn secret_names_hide_their_values() {
        assert_eq!(
            secrets("API_TOKEN=abc123 npm publish"),
            "API_TOKEN=[redacted] npm publish"
        );
        assert_eq!(secrets("--password='hunter2' db"), "--password='[redacted]' db");
        assert_eq!(
            secrets("env GITHUB_TOKEN=\"x y\" make"),
            "env GITHUB_TOKEN=\"[redacted]\" make"
        );
    }

    #[test]
    fn bearer_and_url_passwords_go() {
        assert_eq!(
            secrets("curl -H 'Authorization: Bearer eyJhbGciOi' https://x"),
            "curl -H 'Authorization: Bearer [redacted]' https://x"
        );
        assert_eq!(
            secrets("git clone https://me:s3cret@example.com/r.git"),
            "git clone https://me:[redacted]@example.com/r.git"
        );
    }

    #[test]
    fn ordinary_commands_stay() {
        for cmd in [
            "cargo test -p vults-core",
            "rm -rf build && npm run build",
            "git commit -m 'skip the sk- prefix talk'",
            "ls ~/.ssh",
            "echo AUTHOR=me PWD=/home/me",
            "https://example.com/a:b@c",
        ] {
            assert_eq!(secrets(cmd), cmd, "{cmd}");
        }
    }
}
