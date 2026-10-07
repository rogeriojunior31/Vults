//! Where the API chat can send a conversation. A provider is data: one more row here is one more
//! provider, as long as it speaks one of the two wires (Anthropic Messages or OpenAI Chat
//! Completions), which nearly all of them do.

use std::sync::OnceLock;
use std::time::Duration;

use serde::Serialize;
use serde_json::Value;

#[derive(Serialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Wire {
    Anthropic,
    OpenAi,
}

#[derive(Serialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Provider {
    /// Also names its key in the keyring: `<id>-api-key`.
    pub id: &'static str,
    pub label: &'static str,
    #[serde(skip)]
    pub base_url: &'static str,
    #[serde(skip)]
    pub wire: Wire,
    /// Runs on this machine: no key, and nothing leaves it.
    pub local: bool,
    /// How its keys start, as a hint in the key field (and a check for Anthropic's).
    pub key_hint: &'static str,
    /// Used until the user picks one from the live list.
    pub default_model: &'static str,
}

pub const PROVIDERS: &[Provider] = &[
    cloud(
        "anthropic",
        "Anthropic",
        "https://api.anthropic.com/v1",
        Wire::Anthropic,
        "sk-ant-",
        "claude-opus-5-5",
    ),
    cloud(
        "openai",
        "OpenAI",
        "https://api.openai.com/v1",
        Wire::OpenAi,
        "sk-",
        "",
    ),
    cloud(
        "google",
        "Google Gemini",
        "https://generativelanguage.googleapis.com/v1beta/openai",
        Wire::OpenAi,
        "AIza",
        "",
    ),
    cloud(
        "openrouter",
        "OpenRouter",
        "https://openrouter.ai/api/v1",
        Wire::OpenAi,
        "sk-or-",
        "",
    ),
    cloud(
        "groq",
        "Groq",
        "https://api.groq.com/openai/v1",
        Wire::OpenAi,
        "gsk_",
        "",
    ),
    cloud(
        "deepseek",
        "DeepSeek",
        "https://api.deepseek.com",
        Wire::OpenAi,
        "sk-",
        "",
    ),
    cloud(
        "mistral",
        "Mistral",
        "https://api.mistral.ai/v1",
        Wire::OpenAi,
        "",
        "",
    ),
    cloud("xai", "xAI", "https://api.x.ai/v1", Wire::OpenAi, "xai-", ""),
    local("ollama", "Ollama", "http://127.0.0.1:11434/v1"),
    local("lmstudio", "LM Studio", "http://127.0.0.1:1234/v1"),
];

const fn cloud(
    id: &'static str,
    label: &'static str,
    base_url: &'static str,
    wire: Wire,
    key_hint: &'static str,
    default_model: &'static str,
) -> Provider {
    Provider {
        id,
        label,
        base_url,
        wire,
        local: false,
        key_hint,
        default_model,
    }
}

const fn local(id: &'static str, label: &'static str, base_url: &'static str) -> Provider {
    Provider {
        id,
        label,
        base_url,
        wire: Wire::OpenAi,
        local: true,
        key_hint: "",
        default_model: "",
    }
}

pub fn find(id: &str) -> Option<&'static Provider> {
    PROVIDERS.iter().find(|p| p.id == id)
}

impl Provider {
    pub fn secret(&self) -> vults_secrets::Secret {
        vults_secrets::Secret::ApiKey(self.id)
    }

    /// Whether `key` could be one of this provider's keys. Only a shape check: the provider has
    /// the last word, at the first request.
    pub fn accepts_key(&self, key: &str) -> bool {
        key.starts_with(if self.wire == Wire::Anthropic {
            self.key_hint
        } else {
            ""
        }) && (16..=512).contains(&key.len())
            && key.bytes().all(|b| b.is_ascii_graphic())
    }

    /// Where it says it isn't reachable, for local servers.
    pub fn host(&self) -> &'static str {
        self.base_url
            .split("://")
            .nth(1)
            .and_then(|rest| rest.split('/').next())
            .unwrap_or(self.base_url)
    }
}

pub(crate) fn client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(15))
            .build()
            .unwrap_or_default()
    })
}

/// The chat models the provider offers today, sorted by name. Embedding, audio, image and
/// moderation models are left out: they can't hold a conversation.
pub async fn models(provider: &Provider, key: Option<&str>) -> Result<Vec<String>, String> {
    // Anthropic pages its list, 20 by default.
    let query = if provider.wire == Wire::Anthropic {
        "?limit=1000"
    } else {
        ""
    };
    let mut request = client()
        .get(format!("{}/models{query}", provider.base_url))
        .timeout(Duration::from_secs(20));
    request = match (provider.wire, key) {
        (Wire::Anthropic, Some(key)) => request
            .header("x-api-key", key)
            .header("anthropic-version", "2023-06-01"),
        (Wire::OpenAi, Some(key)) => request.bearer_auth(key),
        (_, None) => request,
    };
    let response = request.send().await.map_err(|e| {
        if provider.local && e.is_connect() {
            format!("{} isn't running on {}.", provider.label, provider.host())
        } else {
            format!("Can't reach {}. Check the connection.", provider.label)
        }
    })?;
    match response.status().as_u16() {
        200 => {}
        401 | 403 => return Err(format!("{} rejected the key. Check it here.", provider.label)),
        s => return Err(format!("{} answered {s} for its model list.", provider.label)),
    }
    let body: Value = response
        .json()
        .await
        .map_err(|_| format!("{} sent a model list we can't read.", provider.label))?;
    Ok(chat_models(&body))
}

const NOT_CHAT: &[&str] = &[
    "embed",
    "tts",
    "whisper",
    "transcribe",
    "audio",
    "realtime",
    "dall-e",
    "image",
    "imagen",
    "veo",
    "sora",
    "moderation",
    "aqa",
    "babbage",
    "davinci",
    "guard",
];

/// `{"data": [{"id": …}, …]}`, the shape both wires use. Google prefixes ids with `models/`.
fn chat_models(body: &Value) -> Vec<String> {
    let mut ids: Vec<String> = body["data"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|m| m["id"].as_str())
        .map(|id| id.strip_prefix("models/").unwrap_or(id).to_string())
        .filter(|id| {
            let low = id.to_lowercase();
            !NOT_CHAT.iter().any(|w| low.contains(w))
        })
        .collect();
    ids.sort();
    ids.dedup();
    ids
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn ids_are_unique_and_keys_named_after_them() {
        let mut ids: Vec<&str> = PROVIDERS.iter().map(|p| p.id).collect();
        ids.dedup();
        assert_eq!(ids.len(), PROVIDERS.len());
        // The account the app has always used for the Anthropic key.
        assert_eq!(find("anthropic").unwrap().secret().account(), "anthropic-api-key");
    }

    #[test]
    fn local_servers_stay_on_this_machine() {
        for p in PROVIDERS {
            assert_eq!(p.local, p.base_url.starts_with("http://127.0.0.1:"), "{}", p.id);
            if !p.local {
                assert!(p.base_url.starts_with("https://"), "{}", p.id);
            }
        }
        assert_eq!(find("ollama").unwrap().host(), "127.0.0.1:11434");
    }

    #[test]
    fn keys_are_checked_for_shape_only() {
        let anthropic = find("anthropic").unwrap();
        assert!(anthropic.accepts_key("sk-ant-api03-abcdefghijklmnop"));
        assert!(!anthropic.accepts_key("sk-or-v1-abcdefghijklmnopqrst"));
        let openrouter = find("openrouter").unwrap();
        assert!(openrouter.accepts_key("sk-or-v1-abcdefghijklmnopqrst"));
        assert!(!openrouter.accepts_key("short"));
        assert!(!openrouter.accepts_key("sk-or-v1 abcdefghijklmnopqrst"));
    }

    #[test]
    fn model_lists_keep_only_chat_models() {
        let body = json!({ "data": [
            { "id": "gpt-x" }, { "id": "text-embedding-3" }, { "id": "models/gemini-y" },
            { "id": "whisper-1" }, { "id": "gpt-x" }, { "no": "id" },
        ]});
        assert_eq!(chat_models(&body), vec!["gemini-y", "gpt-x"]);
    }
}
