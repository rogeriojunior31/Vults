//! Secrets live in the OS keyring (Secret Service on Linux, Credential Manager on Windows), under
//! the app's bundle id. Never in a file, never in a log, never sent back to the UI.

/// The secrets the app knows about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Secret {
    /// An API key for the chat, by provider id (`anthropic`, `openai`, …).
    ApiKey(&'static str),
}

impl Secret {
    pub fn account(self) -> String {
        match self {
            Secret::ApiKey(provider) => format!("{provider}-api-key"),
        }
    }
}

fn entry(secret: Secret) -> Result<keyring::Entry, String> {
    keyring::Entry::new(vultures_ai_brand::BUNDLE_ID, &secret.account()).map_err(|e| format!("keyring: {e}"))
}

pub fn get(secret: Secret) -> Result<Option<String>, String> {
    match entry(secret)?.get_password() {
        Ok(v) => Ok(Some(v)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(format!("keyring: {e}")),
    }
}

pub fn set(secret: Secret, value: &str) -> Result<(), String> {
    entry(secret)?
        .set_password(value)
        .map_err(|e| format!("keyring: {e}"))
}

pub fn delete(secret: Secret) -> Result<(), String> {
    match entry(secret)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(format!("keyring: {e}")),
    }
}

pub fn has(secret: Secret) -> bool {
    matches!(get(secret), Ok(Some(_)))
}
