use super::{Cancel, Gate, Outcome, Progress, catalog::Download};
use futures_util::{Stream, StreamExt};
use reqwest::Url;
use sha2::{Digest, Sha256};
use std::path::Path;
use std::time::Instant;
use tokio::io::AsyncWriteExt;
use tokio::time::{Duration, timeout_at};

struct Trace<'a> {
    gate: &'a dyn Gate,
    id: &'static str,
    started: Instant,
    outcome: Outcome,
}
impl<'a> Trace<'a> {
    fn begin(
        gate: &'a dyn Gate,
        id: &'static str,
        cancel: &Cancel,
        deadline: tokio::time::Instant,
    ) -> Result<Self, String> {
        cancel.check()?;
        if tokio::time::Instant::now() >= deadline {
            return Err("meeting asset download timed out".into());
        }
        if !gate.allow(id) {
            return Err("meeting asset request was refused by its network gate".into());
        }
        let mut trace = Self {
            gate,
            id,
            started: Instant::now(),
            outcome: Outcome::Cancelled,
        };
        // Durable quota admission can block; recheck before creating the HTTP future.
        trace.check(cancel, deadline)?;
        Ok(trace)
    }

    fn check(&mut self, cancel: &Cancel, deadline: tokio::time::Instant) -> Result<(), String> {
        if let Err(error) = cancel.check() {
            self.outcome = Outcome::Cancelled;
            return Err(error);
        }
        if tokio::time::Instant::now() >= deadline {
            self.outcome = Outcome::Failed;
            return Err("meeting asset download timed out".into());
        }
        Ok(())
    }
}
impl Drop for Trace<'_> {
    fn drop(&mut self) {
        self.gate.done(
            self.id,
            self.outcome,
            self.started.elapsed().as_millis().min(u64::MAX as u128) as u64,
        );
    }
}
fn allowed(url: &Url) -> bool {
    url.scheme() == "https"
        && url.username().is_empty()
        && url.password().is_none()
        && url.port_or_known_default() == Some(443)
        && url.fragment().is_none()
        && matches!(
            url.host_str(),
            Some("github.com" | "release-assets.githubusercontent.com" | "objects.githubusercontent.com")
        )
}

pub(super) async fn fetch(
    artifact: Download,
    directory: &Path,
    gate: &dyn Gate,
    cancel: &Cancel,
    progress: &(impl Fn(Progress) + Send + Sync),
) -> Result<(), String> {
    cancel.check()?;
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .no_proxy()
        .connect_timeout(Duration::from_secs(10))
        .read_timeout(Duration::from_secs(15))
        .user_agent("Vults")
        .build()
        .map_err(|e| e.to_string())?;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(180);
    let mut url = Url::parse(artifact.url).map_err(|_| "invalid pinned asset URL")?;
    let mut options = tokio::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        options.mode(0o600).custom_flags(libc::O_NOFOLLOW);
    }
    let mut file = options
        .open(directory.join(format!("{}.download", artifact.id)))
        .await
        .map_err(|e| e.to_string())?;
    for redirect in 0..=3 {
        if !allowed(&url) {
            return Err("meeting asset redirect left the approved HTTPS hosts".into());
        }
        let mut trace = Trace::begin(gate, artifact.id, cancel, deadline)?;
        let response = tokio::select! {
            _ = cancel.cancelled() => return Err("meeting asset installation cancelled".into()),
            result = timeout_at(deadline,client.get(url.clone()).send()) => {
                trace.outcome = Outcome::Failed;
                result.map_err(|_|"meeting asset download timed out")?.map_err(|_|"meeting asset connection failed")?
            }
        };
        trace.check(cancel, deadline)?;
        if response.status().is_redirection() {
            if redirect == 3 {
                return Err("too many meeting asset redirects".into());
            }
            let location = response
                .headers()
                .get(reqwest::header::LOCATION)
                .and_then(|v| v.to_str().ok())
                .ok_or("meeting asset redirect has no location")?;
            let next = url.join(location).map_err(|_| "invalid meeting asset redirect")?;
            if !allowed(&next) {
                return Err("meeting asset redirect left the approved HTTPS hosts".into());
            }
            trace.outcome = Outcome::Complete;
            url = next;
            continue;
        }
        if !response.status().is_success()
            || response.content_length().is_some_and(|len| len != artifact.size)
        {
            return Err("meeting asset response has an unexpected status or size".into());
        }
        // Raw URLs and errors can contain signed CDN query tokens; traces retain artifact IDs only.
        let mut stream = response
            .bytes_stream()
            .map(|chunk| chunk.map_err(|_| "meeting asset stream failed".to_string()));
        trace.outcome = Outcome::Cancelled;
        let result = copy_chunks(&mut stream, &mut file, artifact, cancel, deadline, progress).await;
        trace.outcome = if result.is_ok() {
            Outcome::Complete
        } else if cancel.is_cancelled() {
            Outcome::Cancelled
        } else {
            Outcome::Failed
        };
        return result;
    }
    Err("meeting asset redirect budget exhausted".into())
}

async fn copy_chunks<S, B>(
    stream: &mut S,
    file: &mut tokio::fs::File,
    artifact: Download,
    cancel: &Cancel,
    deadline: tokio::time::Instant,
    progress: &(impl Fn(Progress) + Send + Sync),
) -> Result<(), String>
where
    S: Stream<Item = Result<B, String>> + Unpin,
    B: AsRef<[u8]>,
{
    let mut done = 0;
    let mut hash = Sha256::new();
    loop {
        cancel.check()?;
        // A ready stream can complete before timeout_at checks its elapsed timer.
        if tokio::time::Instant::now() >= deadline {
            return Err("meeting asset download timed out".into());
        }
        let chunk = tokio::select! {
            _ = cancel.cancelled()=>return Err("meeting asset installation cancelled".into()),
            chunk = timeout_at(deadline,stream.next())=>chunk.map_err(|_|"meeting asset download timed out")?,
        };
        cancel.check()?;
        if tokio::time::Instant::now() >= deadline {
            return Err("meeting asset download timed out".into());
        }
        let Some(chunk) = chunk else { break };
        let chunk = chunk?;
        let bytes = chunk.as_ref();
        done += bytes.len() as u64;
        if done > artifact.size {
            return Err("meeting asset download exceeds its pinned size".into());
        }
        hash.update(bytes);
        file.write_all(bytes).await.map_err(|e| e.to_string())?;
        cancel.check()?;
        progress(Progress {
            artifact: artifact.id,
            downloaded: done,
            total: artifact.size,
        });
    }
    cancel.check()?;
    if done != artifact.size || format!("{:x}", hash.finalize()) != artifact.sha256 {
        return Err("meeting asset download fingerprint or size mismatch".into());
    }
    file.flush().await.map_err(|e| e.to_string())?;
    cancel.check()?;
    file.sync_all().await.map_err(|e| e.to_string())?;
    cancel.check()
}

#[cfg(test)]
mod tests;
