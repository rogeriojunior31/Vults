//! A local embedding for recall (plan-zeca M5): Ollama on this computer, `127.0.0.1` only, so the
//! journal's words never leave it. When no Ollama answers, or it lacks the model, recall goes on
//! with full-text search alone.

use std::time::Duration;

use serde_json::{Value, json};

/// The embedding model asked of Ollama (`ollama pull nomic-embed-text`).
pub const MODEL: &str = "nomic-embed-text";
const URL: &str = "http://127.0.0.1:11434/api/embed";
/// A local call: past this, Ollama is busy or gone, and recall must not wait on it.
const TIMEOUT: Duration = Duration::from_secs(5);

/// `text` as a vector, or why not.
pub async fn local(text: &str) -> Result<Vec<f32>, String> {
    let response = crate::providers::client()
        .post(URL)
        .timeout(TIMEOUT)
        .json(&json!({ "model": MODEL, "input": text }))
        .send()
        .await
        .map_err(|e| format!("no local Ollama: {e}"))?;
    if !response.status().is_success() {
        return Err(format!("Ollama answered {}", response.status()));
    }
    let body: Value = response.json().await.map_err(|e| e.to_string())?;
    vector(&body).ok_or_else(|| "Ollama sent no embedding".into())
}

fn vector(body: &Value) -> Option<Vec<f32>> {
    let v: Vec<f32> = body["embeddings"][0]
        .as_array()?
        .iter()
        .map(|x| x.as_f64().map(|f| f as f32))
        .collect::<Option<_>>()?;
    (!v.is_empty()).then_some(v)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_ollama_s_answer() {
        assert_eq!(
            vector(&json!({ "model": MODEL, "embeddings": [[0.5, -1.0, 2.0]] })),
            Some(vec![0.5, -1.0, 2.0])
        );
        assert_eq!(vector(&json!({ "embeddings": [] })), None);
        assert_eq!(vector(&json!({ "embeddings": [[]] })), None);
        assert_eq!(vector(&json!({ "embeddings": [["x"]] })), None);
        assert!(URL.starts_with("http://127.0.0.1:"), "never off this computer");
    }
}
