//! GH1r: a check the user asks for, telling apart why GitHub access fails. One account read and,
//! when a repository is named, one repository read. Only a classification leaves here: never
//! `gh`'s own words, a token or a header.

use serde::Serialize;
use serde_json::Value;

use super::{ACCOUNT_QUERY, Error, account_answer, checked_repository, gh_api, graphql};

/// What one read found. Order of the checks in [`classify`] matters: a rate limit and SSO also
/// come as 403s.
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Access {
    Ready,
    /// `gh` is not installed or not on the PATH.
    NoCli,
    /// `gh` has no login for this host.
    SignedOut,
    /// A login exists but GitHub refused it: expired, revoked or a bad token.
    Rejected,
    RateLimited,
    /// An organization requires SAML single sign-on for this token.
    Sso,
    /// The login works but may not read this.
    Forbidden,
    /// Not found, or private and not visible to this login: GitHub does not tell them apart.
    NotFound,
    /// GitHub answered without the fields the check needs.
    Incomplete,
    Failed,
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RepositoryCheck {
    pub name: String,
    pub access: Access,
    pub push: Option<bool>,
    pub archived: Option<bool>,
    pub private: Option<bool>,
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Diagnosis {
    pub account: Access,
    pub login: Option<String>,
    pub host: Option<String>,
    /// Only checked once the account is ready.
    pub repository: Option<RepositoryCheck>,
}

/// A failed `gh` call, by what it printed.
pub fn classify(stderr: &str) -> Access {
    let said = stderr.to_ascii_lowercase();
    let has = |words: &[&str]| words.iter().any(|w| said.contains(w));
    if has(&["rate limit", "abuse detection"]) {
        Access::RateLimited
    } else if has(&["saml", "single sign-on", "sso"]) {
        Access::Sso
    } else if has(&[
        "gh auth login",
        "not logged",
        "no oauth token",
        "authentication required",
    ]) {
        Access::SignedOut
    } else if has(&[
        "http 401",
        "bad credentials",
        "token is expired",
        "requires authentication",
    ]) {
        Access::Rejected
    } else if has(&["http 404", "not found", "could not resolve to a repository"]) {
        Access::NotFound
    } else if has(&[
        "http 403",
        "forbidden",
        "resource not accessible",
        "must have push access",
    ]) {
        Access::Forbidden
    } else {
        Access::Failed
    }
}

fn from_error(error: &Error) -> Access {
    match error {
        Error::Unavailable(_) => Access::NoCli,
        Error::RateLimited { .. } => Access::RateLimited,
        Error::Auth(_) => Access::Rejected,
        Error::Partial(_) => Access::Incomplete,
        Error::Budget | Error::Other(_) => Access::Failed,
    }
}

/// The account read: `gh api graphql` with the viewer's login and profile.
pub fn account_check(
    read: Result<(bool, Vec<u8>, Vec<u8>), Error>,
) -> (Access, Option<String>, Option<String>) {
    let (success, stdout, stderr) = match read {
        Ok(read) => read,
        Err(error) => return (from_error(&error), None, None),
    };
    if !success {
        return (classify(&String::from_utf8_lossy(&stderr)), None, None);
    }
    match account_answer(true, &stdout, &stderr, std::time::SystemTime::now()) {
        Ok(value) => {
            let text = |key: &str| value["source"][key].as_str().map(str::to_owned);
            (Access::Ready, text("account"), text("host"))
        }
        Err(error) => (from_error(&error), None, None),
    }
}

/// The repository read: `gh api repos/owner/name`, whose `permissions` say what this login may do.
pub fn repository_check(name: &str, read: Result<(bool, Vec<u8>, Vec<u8>), Error>) -> RepositoryCheck {
    let fail = |access| RepositoryCheck {
        name: name.to_owned(),
        access,
        push: None,
        archived: None,
        private: None,
    };
    let (success, stdout, stderr) = match read {
        Ok(read) => read,
        Err(error) => return fail(from_error(&error)),
    };
    if !success {
        return fail(classify(&String::from_utf8_lossy(&stderr)));
    }
    let Ok(value) = serde_json::from_slice::<Value>(&stdout) else {
        return fail(Access::Incomplete);
    };
    let (Some(full), Some(archived), Some(private)) = (
        value["full_name"].as_str(),
        value["archived"].as_bool(),
        value["private"].as_bool(),
    ) else {
        return fail(Access::Incomplete);
    };
    RepositoryCheck {
        name: full.to_owned(),
        access: Access::Ready,
        push: value["permissions"]["push"].as_bool(),
        archived: Some(archived),
        private: Some(private),
    }
}

/// The check itself. The repository, when named, must be `owner/name`; it is read only once the
/// account answers, so a missing login is never reported as a missing repository.
pub async fn diagnose(repository: Option<&str>) -> Result<Diagnosis, Error> {
    let repository = repository.map(str::trim).filter(|r| !r.is_empty());
    if let Some(repository) = repository {
        // GitHub names never start with a dash: none can pass for a flag of `gh`.
        if repository.starts_with('-') || repository.contains("/-") {
            return Err(Error::Other("Use a GitHub repository in owner/name form.".into()));
        }
        checked_repository(repository)?;
    }
    let (account, login, host) = account_check(graphql(ACCOUNT_QUERY, &[], 64 * 1024).await);
    let repository = match repository {
        Some(name) if account == Access::Ready => Some(repository_check(
            name,
            gh_api(&format!("repos/{name}"), None, 128 * 1024).await,
        )),
        _ => None,
    };
    Ok(Diagnosis {
        account,
        login,
        host,
        repository,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn failed(stderr: &str) -> Result<(bool, Vec<u8>, Vec<u8>), Error> {
        Ok((false, Vec::new(), stderr.as_bytes().to_vec()))
    }
    fn answered(body: serde_json::Value) -> Result<(bool, Vec<u8>, Vec<u8>), Error> {
        Ok((true, body.to_string().into_bytes(), Vec::new()))
    }

    #[test]
    fn each_failure_from_gh_has_its_own_name() {
        for (said, access) in [
            (
                "To get started with GitHub CLI, please run:  gh auth login\nAlternatively, populate the GH_TOKEN environment variable",
                Access::SignedOut,
            ),
            (
                "You are not logged into any GitHub hosts. To log in, run: gh auth login",
                Access::SignedOut,
            ),
            ("gh: Bad credentials (HTTP 401)", Access::Rejected),
            (
                "HTTP 401: Requires authentication (https://api.github.com/graphql)",
                Access::Rejected,
            ),
            (
                "gh: API rate limit exceeded for user ID 1. (HTTP 403)",
                Access::RateLimited,
            ),
            (
                "gh: You have exceeded a secondary rate limit (HTTP 403)",
                Access::RateLimited,
            ),
            (
                "gh: Resource protected by organization SAML enforcement. You must grant your OAuth token access to this organization. (HTTP 403)",
                Access::Sso,
            ),
            (
                "gh: Resource not accessible by integration (HTTP 403)",
                Access::Forbidden,
            ),
            ("gh: Not Found (HTTP 404)", Access::NotFound),
            (
                "GraphQL: Could not resolve to a Repository with the name 'o/r'. (repository)",
                Access::NotFound,
            ),
            ("error connecting to api.github.com", Access::Failed),
        ] {
            assert_eq!(classify(said), access, "{said}");
        }
    }

    #[test]
    fn the_account_read_names_the_login_or_why_not() {
        let viewer = serde_json::json!({"data":{"viewer":{"login":"person","name":"P","url":"https://github.com/person"}}});
        assert_eq!(
            account_check(answered(viewer)),
            (Access::Ready, Some("person".into()), Some("github.com".into()))
        );
        assert_eq!(
            account_check(Err(Error::Unavailable(
                "The GitHub CLI (gh) isn't installed".into()
            ))),
            (Access::NoCli, None, None)
        );
        assert_eq!(
            account_check(failed("gh: Bad credentials (HTTP 401)")).0,
            Access::Rejected
        );
        assert_eq!(account_check(failed("gh auth login")).0, Access::SignedOut);
        assert_eq!(
            account_check(answered(serde_json::json!({"data":{"viewer":{}}}))).0,
            Access::Incomplete
        );
        assert_eq!(
            account_check(answered(serde_json::json!({"errors":[{"message":"x"}]}))).0,
            Access::Incomplete
        );
    }

    #[test]
    fn the_repository_read_says_what_this_login_may_do() {
        let ok = repository_check(
            "o/r",
            answered(
                serde_json::json!({"full_name":"o/r","archived":false,"private":true,"permissions":{"pull":true,"push":false}}),
            ),
        );
        assert_eq!(ok.access, Access::Ready);
        assert_eq!(
            (ok.push, ok.archived, ok.private),
            (Some(false), Some(false), Some(true))
        );
        assert_eq!(
            repository_check("o/r", failed("gh: Not Found (HTTP 404)")).access,
            Access::NotFound
        );
        assert_eq!(
            repository_check(
                "o/r",
                failed("gh: Resource protected by organization SAML enforcement (HTTP 403)")
            )
            .access,
            Access::Sso
        );
        assert_eq!(
            repository_check("o/r", answered(serde_json::json!({"full_name":"o/r"}))).access,
            Access::Incomplete
        );
        assert_eq!(
            repository_check("o/r", Ok((true, b"not json".to_vec(), Vec::new()))).access,
            Access::Incomplete
        );
    }

    #[test]
    fn nothing_of_what_gh_said_reaches_the_result() {
        let secret = "gh: Bad credentials ghp_0123456789abcdefghijABCDEFGHIJ012345 (HTTP 401)";
        let (access, login, host) = account_check(failed(secret));
        let repo = repository_check("o/r", failed(secret));
        let all = serde_json::to_string(&(access, login, host, repo)).unwrap();
        assert!(!all.contains("ghp_"), "{all}");
        assert!(!all.contains("Bad credentials"), "{all}");
    }

    #[tokio::test]
    async fn a_repository_must_be_owner_slash_name() {
        // Every case here is refused before `gh` runs: this test never leaves the machine.
        for bad in [
            "o",
            "o/r/x",
            "-o/r",
            "o/-r",
            "o/..",
            "https://github.com/o/r",
            " ",
        ] {
            if bad.trim().is_empty() {
                continue;
            }
            assert!(diagnose(Some(bad)).await.is_err(), "{bad}");
        }
    }
}
