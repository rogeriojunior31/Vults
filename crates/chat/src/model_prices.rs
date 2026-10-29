//! Fixed public catalog endpoint. The app owns consent, cadence, cache and admission.
use vults_agents::prices::{Catalog, SOURCE};

pub async fn fetch(etag: Option<&str>) -> Result<Option<Catalog>, String> {
    let client = reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .map_err(|_| "Price service unavailable")?;
    let mut request = client.get(SOURCE);
    if let Some(etag) = etag {
        request = request.header(reqwest::header::IF_NONE_MATCH, etag);
    }
    let response = request
        .send()
        .await
        .map_err(|_| "Could not refresh model prices")?;
    if response.status() == reqwest::StatusCode::NOT_MODIFIED {
        return Ok(None);
    }
    let mut response = response
        .error_for_status()
        .map_err(|_| "Price service unavailable")?;
    const MAX_BYTES: usize = 16 * 1024 * 1024;
    if response.content_length().is_some_and(|n| n > MAX_BYTES as u64) {
        return Err("Price catalog too large".into());
    }
    let etag = response
        .headers()
        .get(reqwest::header::ETAG)
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);
    let mut bytes = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| "Could not read model prices")?
    {
        if chunk.len() > MAX_BYTES.saturating_sub(bytes.len()) {
            return Err("Price catalog too large".into());
        }
        bytes.extend_from_slice(&chunk);
    }
    let value = serde_json::from_slice(&bytes).map_err(|_| "Invalid price catalog")?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64);
    Catalog::from_models_dev(&value, now, etag).map(Some)
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    #[ignore = "Contacts the public models.dev catalog, no credentials or session data"]
    async fn live_catalog_contains_openai_prices() {
        let book = super::fetch(None).await.unwrap().unwrap();
        assert!(book.valid());
        assert!(book.models.get("openai").is_some_and(|m| !m.is_empty()));
        eprintln!("Public price catalog parsed; {} providers", book.models.len());
    }
}
