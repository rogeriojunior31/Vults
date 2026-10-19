//! One bounded, tool-free text-generation call. The caller owns consent, quota and cancellation.
use std::time::Duration;

use crate::providers::{Provider, Wire};
use serde_json::{Value, json};
use vults_dictation::cleanup::{Message, Role};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Configuration,
    Transport,
    Status,
    Malformed,
    Refused,
    Truncated,
    TooLarge,
    Tools,
}

#[derive(Clone, Debug)]
pub struct Reply {
    pub text: String,
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
}

/// A local label is insufficient: only explicit loopback IPs qualify, with no URL credentials.
pub fn endpoint(provider: &Provider) -> Result<reqwest::Url, Error> {
    let mut url = reqwest::Url::parse(provider.base_url).map_err(|_| Error::Configuration)?;
    if !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(Error::Configuration);
    }
    if provider.local {
        let loopback = url
            .host_str()
            .and_then(|host| host.trim_matches(['[', ']']).parse::<std::net::IpAddr>().ok())
            .is_some_and(|ip| ip.is_loopback());
        if !loopback || !matches!(url.scheme(), "http" | "https") {
            return Err(Error::Configuration);
        }
    } else if url.scheme() != "https" {
        return Err(Error::Configuration);
    }
    let suffix = match provider.wire {
        Wire::Anthropic => "messages",
        Wire::OpenAi => "chat/completions",
    };
    url.set_path(&format!("{}/{suffix}", url.path().trim_end_matches('/')));
    Ok(url)
}

fn body(wire: Wire, model: &str, turns: &[Message]) -> Result<Value, Error> {
    if !crate::valid_model(model)
        || turns.first().is_none_or(|m| m.role != Role::System)
        || turns.last().is_none_or(|m| m.role != Role::User)
    {
        return Err(Error::Configuration);
    }
    let message = |m: &Message| json!({"role":match m.role {Role::System=>"system",Role::User=>"user",Role::Assistant=>"assistant"},"content":m.text});
    Ok(match wire {
        Wire::Anthropic => json!({"model":model,"max_tokens":4096,"stream":false,"system":turns[0].text,
            "messages":turns[1..].iter().map(message).collect::<Vec<_>>()}),
        Wire::OpenAi => json!({"model":model,"max_tokens":4096,"stream":false,
            "messages":turns.iter().map(message).collect::<Vec<_>>()}),
    })
}

fn reply(wire: Wire, value: Value) -> Result<Reply, Error> {
    let (text, input, output) = match wire {
        Wire::Anthropic => {
            match value["stop_reason"].as_str() {
                Some("end_turn") => {}
                Some("refusal") => return Err(Error::Refused),
                Some("max_tokens") => return Err(Error::Truncated),
                Some("tool_use") => return Err(Error::Tools),
                _ => return Err(Error::Malformed),
            }
            let blocks = value["content"].as_array().ok_or(Error::Malformed)?;
            if blocks.iter().any(|b| b["type"] != "text") {
                return Err(Error::Tools);
            }
            let text = blocks
                .iter()
                .map(|b| b["text"].as_str().ok_or(Error::Malformed))
                .collect::<Result<String, _>>()?;
            (
                text,
                value["usage"]["input_tokens"].as_u64(),
                value["usage"]["output_tokens"].as_u64(),
            )
        }
        Wire::OpenAi => {
            let choice = &value["choices"][0];
            if choice["message"]["tool_calls"]
                .as_array()
                .is_some_and(|v| !v.is_empty())
                || !choice["message"]["function_call"].is_null()
            {
                return Err(Error::Tools);
            }
            if !choice["message"]["refusal"].is_null() {
                return Err(Error::Refused);
            }
            match choice["finish_reason"].as_str() {
                Some("stop") => {}
                Some("length") => return Err(Error::Truncated),
                _ => return Err(Error::Malformed),
            }
            (
                choice["message"]["content"]
                    .as_str()
                    .ok_or(Error::Malformed)?
                    .to_string(),
                value["usage"]["prompt_tokens"].as_u64(),
                value["usage"]["completion_tokens"].as_u64(),
            )
        }
    };
    Ok(Reply {
        text,
        input_tokens: input,
        output_tokens: output,
    })
}

pub async fn complete(
    provider: &Provider,
    key: Option<&str>,
    model: &str,
    messages: &[Message],
    timeout: Duration,
) -> Result<Reply, Error> {
    let url = endpoint(provider)?;
    if !provider.local && key.is_none() {
        return Err(Error::Configuration);
    }
    // A redirect/proxy/retry must not turn one admitted local attempt into another destination/call.
    let client = reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .retry(reqwest::retry::never())
        .connect_timeout(Duration::from_secs(4))
        .timeout(timeout)
        .build()
        .map_err(|_| Error::Configuration)?;
    let mut request = client.post(url).json(&body(provider.wire, model, messages)?);
    if provider.wire == Wire::Anthropic {
        request = request.header("anthropic-version", "2023-06-01");
    }
    if let Some(key) = key {
        request = match provider.wire {
            Wire::Anthropic => request.header("x-api-key", key),
            Wire::OpenAi => request.bearer_auth(key),
        };
    }
    let mut response = request.send().await.map_err(|_| Error::Transport)?;
    if !response.status().is_success() {
        return Err(Error::Status);
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|_| Error::Transport)? {
        if bytes.len().saturating_add(chunk.len()) > 2 * 1024 * 1024 {
            return Err(Error::TooLarge);
        }
        bytes.extend_from_slice(&chunk);
    }
    reply(
        provider.wire,
        serde_json::from_slice(&bytes).map_err(|_| Error::Malformed)?,
    )
}

#[cfg(test)]
mod tests;
