//! The link between the desktop and a paired phone (ADR 0017, `docs/dev/plan-zeca.md` L1).
//!
//! - **Pairing** ([`pair`]): the desktop shows a QR with its public key, the relay's URL and a
//!   one-time secret that lives two minutes; the phone answers with its keys, sealed under that
//!   secret, so only who saw the QR can pair.
//! - **Envelopes** ([`envelope`]): after pairing, everything between the two is sealed with
//!   XChaCha20-Poly1305 under a key from X25519, one key per direction. A relay sees only the
//!   device's id (to route) and ciphertext.
//! - **Answers** ([`answer`]): the phone signs each answer to a card with a Keystore key: one that
//!   needs the user's biometrics for each use (allow, always, answer) or one that does not (deny,
//!   dismiss). The desktop checks it and hands `core` an [`vults_core::Input::Paired`], whose ledger
//!   then checks the rest: the binding, the time, single use and the counter.
//!
//! The desktop stays the only one that acts; the phone only decides. No IO here: the app stores
//! the keys (the desktop's secret in the keyring, rule 4) and moves the bytes (L2).

pub mod answer;
pub mod card;
pub mod envelope;
pub mod pair;

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use sha2::{Digest, Sha256};
use x25519_dalek::{PublicKey, StaticSecret};
use zeroize::Zeroizing;

/// The wire format's version, in every message and every key's label.
pub const VERSION: u8 = 1;

/// A message larger than this is refused before it is parsed.
pub const MAX_MESSAGE: usize = 64 * 1024;

/// Why a message was refused. None says which secret was wrong: a refusal is all a sender learns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// Not the shape this version reads, or too large.
    Malformed,
    /// Another version of the wire format.
    Version,
    /// The pairing secret's two minutes are over.
    InviteExpired,
    /// The QR was already used to pair a phone.
    InviteUsed,
    /// It does not open: sealed with another key, or changed on the way.
    Unsealed,
    /// A key that cannot be one (a low-order X25519 point, a P-256 key off the curve).
    BadKey,
    /// The user revoked this device.
    Revoked,
    /// The signature does not match the answer and the key it names.
    BadSignature,
    /// An answer that needs biometrics, signed with the key that does not.
    NeedsBiometrics,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Malformed => "malformed message",
            Self::Version => "unknown version",
            Self::InviteExpired => "pairing code expired",
            Self::InviteUsed => "pairing code already used",
            Self::Unsealed => "message does not open",
            Self::BadKey => "invalid key",
            Self::Revoked => "device revoked",
            Self::BadSignature => "bad signature",
            Self::NeedsBiometrics => "answer needs biometrics",
        };
        f.write_str(s)
    }
}

impl std::error::Error for Error {}

/// An X25519 key pair: the desktop's (long-lived, its secret kept in the OS keyring) or the
/// phone's. The secret is wiped when dropped and never printed.
pub struct SealKey(StaticSecret);

impl std::fmt::Debug for SealKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("SealKey").field(&b64(&self.public())).finish()
    }
}

impl SealKey {
    pub fn generate() -> Self {
        Self(StaticSecret::random_from_rng(rand_core::OsRng))
    }

    /// From the 32 bytes [`SealKey::secret`] gave, read back from the keyring.
    pub fn from_secret(bytes: [u8; 32]) -> Self {
        Self(StaticSecret::from(bytes))
    }

    /// The secret's bytes, for the keyring only.
    pub fn secret(&self) -> Zeroizing<[u8; 32]> {
        Zeroizing::new(self.0.to_bytes())
    }

    pub fn public(&self) -> [u8; 32] {
        PublicKey::from(&self.0).to_bytes()
    }

    /// The shared secret with `their` public key, refused when it is all zeros: a low-order point
    /// would make it one anyone can guess.
    fn agree(&self, their: &[u8; 32]) -> Result<Zeroizing<[u8; 32]>, Error> {
        let shared = self.0.diffie_hellman(&PublicKey::from(*their));
        if !shared.was_contributory() {
            return Err(Error::BadKey);
        }
        Ok(Zeroizing::new(shared.to_bytes()))
    }
}

/// A paired device as the desktop knows it (the database's row, `vults_store::devices`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Device {
    pub id: String,
    pub name: String,
    /// X25519 public key.
    pub seal_key: [u8; 32],
    /// P-256 public key (SEC1) for what needs no biometrics.
    pub sign_key: Vec<u8>,
    /// P-256 public key (SEC1) that the phone unlocks only with the user's biometrics, per use.
    pub bio_key: Vec<u8>,
    pub revoked: bool,
}

/// A device's id: a hash of all its keys, so the id names those keys and no others.
pub fn device_id(seal_key: &[u8; 32], sign_key: &[u8], bio_key: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(b"vults-link/1/id");
    for k in [&seal_key[..], sign_key, bio_key] {
        h.update((k.len() as u64).to_le_bytes());
        h.update(k);
    }
    hex(&h.finalize()[..16])
}

/// Fresh random bytes from the OS.
fn random<const N: usize>() -> [u8; N] {
    use rand_core::RngCore;
    let mut out = [0; N];
    rand_core::OsRng.fill_bytes(&mut out);
    out
}

/// Lowercase hex, as a binding or a box's address is written.
pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn b64(bytes: &[u8]) -> String {
    URL_SAFE_NO_PAD.encode(bytes)
}

fn unb64(s: &str) -> Result<Vec<u8>, Error> {
    URL_SAFE_NO_PAD.decode(s).map_err(|_| Error::Malformed)
}

fn unb64_array<const N: usize>(s: &str) -> Result<[u8; N], Error> {
    unb64(s)?.try_into().map_err(|_| Error::Malformed)
}

/// A JSON message of this version, refused when too large or of another version.
fn parse<T: serde::de::DeserializeOwned>(text: &[u8]) -> Result<T, Error> {
    #[derive(serde::Deserialize)]
    struct V {
        v: u8,
    }
    if text.len() > MAX_MESSAGE {
        return Err(Error::Malformed);
    }
    let v: V = serde_json::from_slice(text).map_err(|_| Error::Malformed)?;
    if v.v != VERSION {
        return Err(Error::Version);
    }
    serde_json::from_slice(text).map_err(|_| Error::Malformed)
}

/// A P-256 public key in SEC1 form, checked to be on the curve.
fn verifying_key(sec1: &[u8]) -> Result<p256::ecdsa::VerifyingKey, Error> {
    p256::ecdsa::VerifyingKey::from_sec1_bytes(sec1).map_err(|_| Error::BadKey)
}

/// The one encoding a P-256 key is kept and hashed in (uncompressed SEC1): the same key sent
/// compressed and uncompressed is still one key, with one id.
fn canonical(sec1: &[u8]) -> Result<Vec<u8>, Error> {
    Ok(verifying_key(sec1)?.to_encoded_point(false).as_bytes().to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_low_order_point_agrees_nothing() {
        let k = SealKey::generate();
        assert_eq!(k.agree(&[0; 32]), Err(Error::BadKey));
    }

    #[test]
    fn the_secret_comes_back_from_its_bytes() {
        let k = SealKey::generate();
        let again = SealKey::from_secret(*k.secret());
        assert_eq!(k.public(), again.public());
        assert!(!format!("{k:?}").contains(&b64(&*k.secret())));
    }

    #[test]
    fn an_id_names_its_keys() {
        let a = device_id(&[1; 32], &[2; 65], &[3; 65]);
        assert_eq!(a.len(), 32);
        assert_ne!(a, device_id(&[1; 32], &[3; 65], &[2; 65]));
        assert_ne!(a, device_id(&[1; 32], &[2; 64], &[3; 66]));
    }
}
