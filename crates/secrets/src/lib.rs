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
    entry_in(vults_brand::BUNDLE_ID, secret)
}

fn entry_in(service: &str, secret: Secret) -> Result<keyring::Entry, String> {
    keyring::Entry::new(service, &secret.account()).map_err(|e| format!("keyring: {e}"))
}

pub fn get(secret: Secret) -> Result<Option<String>, String> {
    match entry(secret)?.get_password() {
        Ok(v) => Ok(Some(v)),
        Err(keyring::Error::NoEntry) => moved_from_legacy(secret),
        Err(e) => Err(format!("keyring: {e}")),
    }
}

/// A secret saved before the rename sits under the old bundle id: move it under the new one.
fn moved_from_legacy(secret: Secret) -> Result<Option<String>, String> {
    let old = entry_in(vults_brand::LEGACY_BUNDLE_ID, secret)?;
    let value = match old.get_password() {
        Ok(v) => v,
        Err(_) => return Ok(None),
    };
    set(secret, &value)?;
    let _ = old.delete_credential();
    Ok(Some(value))
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
