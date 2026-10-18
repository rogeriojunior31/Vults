//! Secrets live in the OS keyring (Secret Service on Linux, Credential Manager on Windows), under
//! the app's bundle id. Never in a file, never in a log, never sent back to the UI.

/// The secrets the app knows about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Secret {
    /// An API key for the chat, by provider id (`anthropic`, `openai`, …).
    ApiKey(&'static str),
    /// A web search engine's key, by engine (`brave`, `tavily`), for Zeca's web tools (Z7).
    WebKey(&'static str),
    /// The desktop's X25519 secret for the paired phones (ADR 0004), as hex.
    PhoneKey,
    /// Single-use restore token for the dictation keyboard portal.
    DictationKeyboard,
}

impl Secret {
    pub fn account(self) -> String {
        match self {
            Secret::ApiKey(provider) => format!("{provider}-api-key"),
            Secret::WebKey(engine) => format!("web-{engine}-key"),
            Secret::PhoneKey => "phone-seal-key".into(),
            Secret::DictationKeyboard => "dictation-keyboard-restore-token".into(),
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
        Err(keyring::Error::NoEntry) if secret == Secret::DictationKeyboard => Ok(None),
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn keyboard_restore_token_round_trip_uses_its_own_keyring_account() {
        use keyring::credential::{Credential, CredentialApi, CredentialBuilderApi, CredentialPersistence};
        use std::sync::{Arc, Mutex};
        #[derive(Clone, Default)]
        struct Memory {
            values: Arc<Mutex<std::collections::HashMap<String, Vec<u8>>>>,
            key: String,
        }
        impl CredentialApi for Memory {
            fn set_secret(&self, value: &[u8]) -> keyring::Result<()> {
                self.values
                    .lock()
                    .expect("memory")
                    .insert(self.key.clone(), value.into());
                Ok(())
            }
            fn get_secret(&self) -> keyring::Result<Vec<u8>> {
                self.values
                    .lock()
                    .expect("memory")
                    .get(&self.key)
                    .cloned()
                    .ok_or(keyring::Error::NoEntry)
            }
            fn delete_credential(&self) -> keyring::Result<()> {
                self.values
                    .lock()
                    .expect("memory")
                    .remove(&self.key)
                    .ok_or(keyring::Error::NoEntry)?;
                Ok(())
            }
            fn as_any(&self) -> &dyn std::any::Any {
                self
            }
        }
        impl CredentialBuilderApi for Memory {
            fn build(&self, _: Option<&str>, service: &str, user: &str) -> keyring::Result<Box<Credential>> {
                Ok(Box::new(Self {
                    values: self.values.clone(),
                    key: format!("{service}/{user}"),
                }))
            }
            fn as_any(&self) -> &dyn std::any::Any {
                self
            }
            fn persistence(&self) -> CredentialPersistence {
                CredentialPersistence::ProcessOnly
            }
        }
        keyring::set_default_credential_builder(Box::new(Memory::default()));
        let secret = Secret::DictationKeyboard;
        assert!(get(secret).expect("missing token").is_none());
        set(secret, "test-token").expect("save token");
        assert!(get(secret).expect("read token").as_deref() == Some("test-token"));
        set(secret, "rotated-token").expect("rotate token");
        assert!(get(secret).expect("rotated read").as_deref() == Some("rotated-token"));
        delete(secret).expect("remove token");
        assert!(get(secret).expect("removed token").is_none());
        assert_ne!(secret.account(), Secret::ApiKey("dictation").account());
        assert_ne!(secret.account(), Secret::PhoneKey.account());
    }
}
