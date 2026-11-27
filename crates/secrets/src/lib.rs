//! Secrets live in the OS keyring (Secret Service on Linux, Credential Manager on Windows), under
//! the app's bundle id. Never in a file, never in a log, never sent back to the UI.

/// The secrets the app knows about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Secret {
    /// An API key for the chat, by provider id (`anthropic`, `openai`, …).
    ApiKey(&'static str),
    /// A web search engine's key, by engine (`brave`, `tavily`), for Zeca's web tools (Z7).
    WebKey(&'static str),
    /// The desktop's X25519 secret for the paired phones (ADR 0002), as hex.
    PhoneKey,
    /// Single-use restore token for the dictation keyboard portal.
    DictationKeyboard,
    /// Atomic Gitea host/token binding; never returned to a renderer.
    Gitea,
}

impl Secret {
    pub fn account(self) -> String {
        match self {
            Secret::ApiKey(provider) => format!("{provider}-api-key"),
            Secret::WebKey(engine) => format!("web-{engine}-key"),
            Secret::PhoneKey => "phone-seal-key".into(),
            Secret::DictationKeyboard => "dictation-keyboard-restore-token".into(),
            Secret::Gitea => "gitea-connection".into(),
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
        Err(keyring::Error::NoEntry) if matches!(secret, Secret::DictationKeyboard | Secret::Gitea) => {
            Ok(None)
        }
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
    // Tests run against keyring's in-memory store, never the user's Credential Manager.
    #[cfg(all(windows, not(test)))]
    return windows::set_local(vults_brand::BUNDLE_ID, &secret.account(), value);
    #[cfg(any(not(windows), test))]
    entry(secret)?
        .set_password(value)
        .map_err(|e| format!("keyring: {e}"))
}

/// keyring writes every Windows credential `CRED_PERSIST_ENTERPRISE`: on a domain account with a
/// roaming profile it follows the user to every PC, and two desktops would share one phone's seal
/// key and every API key. Written here `CRED_PERSIST_LOCAL_MACHINE`, in keyring's own shape (its
/// target name, user name and UTF-16 blob), so keyring still reads and deletes it.
#[cfg(windows)]
mod windows {
    use std::ffi::c_void;

    #[repr(C)]
    struct Credential {
        flags: u32,
        kind: u32,
        target_name: *mut u16,
        comment: *mut u16,
        last_written: [u32; 2],
        blob_size: u32,
        blob: *mut u8,
        persist: u32,
        attribute_count: u32,
        attributes: *mut c_void,
        target_alias: *mut u16,
        user_name: *mut u16,
    }

    #[link(name = "advapi32")]
    unsafe extern "system" {
        fn CredWriteW(credential: *const Credential, flags: u32) -> i32;
    }

    const CRED_TYPE_GENERIC: u32 = 1;
    const CRED_PERSIST_LOCAL_MACHINE: u32 = 2;
    /// CRED_MAX_CREDENTIAL_BLOB_SIZE.
    const MAX_BLOB: usize = 5 * 512;

    fn wide(text: &str) -> Vec<u16> {
        text.encode_utf16().chain(Some(0)).collect()
    }

    pub(super) fn set_local(service: &str, account: &str, value: &str) -> Result<(), String> {
        let mut blob: Vec<u8> = value.encode_utf16().flat_map(u16::to_le_bytes).collect();
        if blob.len() > MAX_BLOB {
            return Err("keyring: the secret is too long for Windows Credential Manager".into());
        }
        let (mut target, mut user) = (wide(&format!("{account}.{service}")), wide(account));
        let (mut comment, mut alias) = (wide(vults_brand::NAME), wide(""));
        let credential = Credential {
            flags: 0,
            kind: CRED_TYPE_GENERIC,
            target_name: target.as_mut_ptr(),
            comment: comment.as_mut_ptr(),
            last_written: [0; 2],
            blob_size: blob.len() as u32,
            blob: blob.as_mut_ptr(),
            persist: CRED_PERSIST_LOCAL_MACHINE,
            attribute_count: 0,
            attributes: std::ptr::null_mut(),
            target_alias: alias.as_mut_ptr(),
            user_name: user.as_mut_ptr(),
        };
        // SAFETY: every pointer is to a buffer alive until the call returns; Windows copies them.
        let written = unsafe { CredWriteW(&credential, 0) } != 0;
        let error = std::io::Error::last_os_error();
        // The copy of the secret goes as soon as Windows has it.
        blob.iter_mut().for_each(|b| *b = 0);
        written.then_some(()).ok_or_else(|| format!("keyring: {error}"))
    }

    #[cfg(test)]
    mod tests {
        #[test]
        fn a_local_credential_reads_back_through_keyring() {
            use keyring::credential::CredentialApi;
            let service = format!("{}.test-{}", vults_brand::BUNDLE_ID, std::process::id());
            let account = "local-persist-check";
            // keyring's own Windows credential, not its default store, which other tests mock.
            let native = keyring::windows::WinCredential::new_with_target(None, &service, account).unwrap();
            let written = super::set_local(&service, account, "secret \u{2713} \u{79d8}");
            let read = native.get_password();
            // Removed before any assert: a failing test leaves nothing in Credential Manager.
            let _ = native.delete_credential();
            written.unwrap();
            assert_eq!(read.unwrap(), "secret \u{2713} \u{79d8}");
            assert!(super::set_local(&service, account, &"x".repeat(1281)).is_err());
            assert!(native.get_password().is_err(), "a refused secret writes nothing");
        }
    }
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

/// Read an existing Codex CLI login without migrating, copying or writing its credential.
pub fn codex_login(account: &str) -> Result<Option<String>, String> {
    if account.len() != 20
        || !account.starts_with("cli|")
        || !account[4..].bytes().all(|b| b.is_ascii_hexdigit())
    {
        return Err("Invalid Codex credential identity.".into());
    }
    let entry =
        keyring::Entry::new("Codex Auth", account).map_err(|_| "Could not read the Codex CLI login.")?;
    match entry.get_password() {
        Ok(value) => Ok(Some(value)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(_) => Err("Could not read the Codex CLI login.".into()),
    }
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
        assert!(get(Secret::Gitea).unwrap().is_none());
        set(Secret::Gitea, "synthetic-host-token-bundle").unwrap();
        assert_eq!(
            get(Secret::Gitea).unwrap().as_deref(),
            Some("synthetic-host-token-bundle")
        );
        delete(Secret::Gitea).unwrap();
        assert!(get(Secret::Gitea).unwrap().is_none());
    }
}
