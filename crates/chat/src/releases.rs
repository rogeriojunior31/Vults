//! The update notice's one request (FF4, ADR 0004): the public releases list of the app's own
//! repository. No credentials, cookies, proxy or redirects, and nothing about the user beyond what
//! any HTTPS request carries. The app owns consent, cadence, admission and parsing.

/// Fixed: the repository the desktop releases are published from (`release.yml`).
pub const SOURCE: &str = "https://api.github.com/repos/rogeriojunior31/Vults/releases?per_page=20";
/// Twenty releases with their notes fit easily; anything larger is not this API answering.
const MAX_BYTES: usize = 2 * 1024 * 1024;

/// The raw JSON of the releases list. `Err` holds a short English reason, never the response.
pub async fn fetch() -> Result<Vec<u8>, String> {
    let client = reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(std::time::Duration::from_secs(20))
        // GitHub refuses requests without a User-Agent; the app's name and version are enough.
        .user_agent(concat!("vults/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|_| "Release service unavailable")?;
    let mut response = client
        .get(SOURCE)
        .header(reqwest::header::ACCEPT, "application/vnd.github+json")
        .send()
        .await
        .map_err(|_| "Could not reach the releases list")?
        .error_for_status()
        .map_err(|_| "Release service unavailable")?;
    if response.content_length().is_some_and(|n| n > MAX_BYTES as u64) {
        return Err("Releases list too large".into());
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| "Could not read the releases list")?
    {
        if chunk.len() > MAX_BYTES.saturating_sub(bytes.len()) {
            return Err("Releases list too large".into());
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    #[ignore = "Contacts the public GitHub releases API, no credentials or session data"]
    async fn live_releases_list_is_json() {
        let raw = super::fetch().await.unwrap();
        let list: Vec<serde_json::Value> = serde_json::from_slice(&raw).unwrap();
        assert!(
            list.iter()
                .any(|r| r["tag_name"].as_str().is_some_and(|t| t.starts_with('v')))
        );
    }

    #[test]
    fn source_is_fixed_https() {
        assert!(super::SOURCE.starts_with("https://api.github.com/repos/"));
    }
}
