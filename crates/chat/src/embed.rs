//! A local embedding for recall (plan-zeca M5): Ollama on this computer, `127.0.0.1` only, so the
//! journal's words never leave it. When no Ollama answers, or it lacks the model, recall goes on
//! with full-text search alone.

use std::time::Duration;

use serde_json::{Value, json};

/// The embedding model asked of Ollama (`ollama pull nomic-embed-text`).
pub const MODEL: &str = "nomic-embed-text";
const URL: &str = "http://127.0.0.1:11434/api/embed";
/// A local call: past this, Ollama is busy or gone (or loading the model), and a chat turn must
/// not wait on it.
const TIMEOUT: Duration = Duration::from_secs(2);
/// Most of a text embedded, in characters: a pasted page is not a question.
const MAX_TEXT: usize = 2_000;

/// What the text is for: the model was trained with these prefixes.
#[derive(Debug, Clone, Copy)]
pub enum Task {
    Query,
    Document,
}

/// Its own client, with no proxy: a `HTTP_PROXY` in the environment must never carry the
/// journal's words off this computer.
fn client() -> &'static reqwest::Client {
    static CLIENT: std::sync::OnceLock<reqwest::Client> = std::sync::OnceLock::new();
    CLIENT.get_or_init(|| reqwest::Client::builder().no_proxy().build().unwrap_or_default())
}

fn input(task: Task, text: &str) -> String {
    let prefix = match task {
        Task::Query => "search_query: ",
        Task::Document => "search_document: ",
    };
    format!("{prefix}{}", text.chars().take(MAX_TEXT).collect::<String>())
}

/// `text` as a vector, or why not.
pub async fn local(task: Task, text: &str) -> Result<Vec<f32>, String> {
    let response = client()
        .post(URL)
        .timeout(TIMEOUT)
        .json(&json!({ "model": MODEL, "input": input(task, text) }))
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
        assert_eq!(input(Task::Query, "hi"), "search_query: hi");
        assert_eq!(
            input(Task::Document, &"x".repeat(5_000)).len(),
            "search_document: ".len() + MAX_TEXT
        );
    }
}
