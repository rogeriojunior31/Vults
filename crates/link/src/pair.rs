//! Pairing (ADR 0017): the desktop shows a QR with its public key, the relay's URL and a one-time
//! secret; the phone answers with its three public keys and a name, sealed under a key from that
//! secret and X25519. Whoever did not see the QR in its two minutes cannot pair, and a QR pairs
//! one phone only.

use std::time::{Duration, Instant};

use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{XChaCha20Poly1305, XNonce};
use hkdf::Hkdf;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use zeroize::Zeroizing;

use crate::{Device, Error, SealKey, VERSION, b64, canonical, device_id, parse, unb64, unb64_array};

/// A QR's secret pairs only this long after it is shown.
pub const INVITE_LIFE: Duration = Duration::from_secs(120);

/// A device's name, as the user typed it on the phone, is cut to this many characters.
pub const MAX_NAME: usize = 64;

/// A relay's URL longer than this is not one.
const MAX_RELAY: usize = 512;

const AAD: &[u8] = b"vults-link/1/pair";

/// What the QR says.
#[derive(Serialize, Deserialize)]
struct Qr {
    v: u8,
    /// The desktop's X25519 public key.
    k: String,
    /// The relay's URL.
    r: String,
    /// The one-time secret.
    s: String,
}

/// The phone's answer to the QR.
#[derive(Serialize, Deserialize)]
struct Request {
    v: u8,
    /// The phone's X25519 public key.
    k: String,
    n: String,
    c: String,
}

/// What the answer seals.
#[derive(Serialize, Deserialize)]
struct Body {
    v: u8,
    name: String,
    sign: String,
    bio: String,
}

/// A pairing on the desktop's screen: one QR, for two minutes, for one phone.
pub struct Invite {
    desktop: [u8; 32],
    relay: String,
    secret: Zeroizing<[u8; 32]>,
    expires: Instant,
    used: bool,
}

impl std::fmt::Debug for Invite {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Invite")
            .field("relay", &self.relay)
            .field("used", &self.used)
            .finish_non_exhaustive()
    }
}

impl Invite {
    /// A new QR for `desktop`'s key and `relay`, shown at `now`.
    pub fn new(desktop: &SealKey, relay: &str, now: Instant) -> Self {
        Self {
            desktop: desktop.public(),
            relay: relay.to_string(),
            secret: Zeroizing::new(crate::random()),
            expires: now + INVITE_LIFE,
            used: false,
        }
    }

    /// The text the QR encodes.
    pub fn qr(&self) -> String {
        let qr = Qr {
            v: VERSION,
            k: b64(&self.desktop),
            r: self.relay.clone(),
            s: b64(&*self.secret),
        };
        serde_json::to_string(&qr).unwrap_or_default()
    }

    /// The phone's answer to this QR, at `now`: the device to save, once. A message that does not
    /// open leaves the QR as it was, so a stranger's guess cannot spend it.
    pub fn accept(&mut self, desktop: &SealKey, request: &str, now: Instant) -> Result<Device, Error> {
        if self.used {
            return Err(Error::InviteUsed);
        }
        if now >= self.expires {
            return Err(Error::InviteExpired);
        }
        let r: Request = parse(request.as_bytes())?;
        let phone: [u8; 32] = unb64_array(&r.k)?;
        let key = key(desktop, &phone, &self.secret, &self.desktop, &phone)?;
        let nonce: [u8; 24] = unb64_array(&r.n)?;
        let plain = XChaCha20Poly1305::new((&*key).into())
            .decrypt(
                XNonce::from_slice(&nonce),
                Payload {
                    msg: &unb64(&r.c)?,
                    aad: AAD,
                },
            )
            .map_err(|_| Error::Unsealed)?;
        let body: Body = parse(&plain)?;
        let sign = canonical(&unb64(&body.sign)?)?;
        let bio = canonical(&unb64(&body.bio)?)?;
        // One key for both (in any encoding) would let a plain signature pass for a biometric one.
        if sign == bio {
            return Err(Error::BadKey);
        }
        self.used = true;
        Ok(Device {
            id: device_id(&phone, &sign, &bio),
            name: clean_name(&body.name),
            seal_key: phone,
            sign_key: sign,
            bio_key: bio,
            revoked: false,
        })
    }
}

/// What the phone read from the QR.
#[derive(Clone, PartialEq, Eq)]
pub struct Scanned {
    pub desktop: [u8; 32],
    pub relay: String,
    secret: Zeroizing<[u8; 32]>,
}

impl std::fmt::Debug for Scanned {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Scanned")
            .field("relay", &self.relay)
            .finish_non_exhaustive()
    }
}

/// The phone reads the QR.
pub fn scan(qr: &str) -> Result<Scanned, Error> {
    let q: Qr = parse(qr.as_bytes())?;
    let relay_ok = q.r.len() <= MAX_RELAY
        && (q.r.starts_with("https://") || q.r.starts_with("http://"))
        && !q.r.chars().any(char::is_control);
    if !relay_ok {
        return Err(Error::Malformed);
    }
    Ok(Scanned {
        desktop: unb64_array(&q.k)?,
        relay: q.r,
        secret: Zeroizing::new(unb64_array(&q.s)?),
    })
}

impl Scanned {
    /// The phone's answer: its seal key, its two signing keys' public halves (SEC1) and the name
    /// the user gave it. Also the id the desktop will know it by.
    pub fn request(
        &self,
        phone: &SealKey,
        name: &str,
        sign_key: &[u8],
        bio_key: &[u8],
    ) -> Result<(String, String), Error> {
        let key = key(phone, &self.desktop, &self.secret, &self.desktop, &phone.public())?;
        let body = Body {
            v: VERSION,
            name: name.to_string(),
            sign: b64(sign_key),
            bio: b64(bio_key),
        };
        let plain = serde_json::to_vec(&body).map_err(|_| Error::Malformed)?;
        let nonce = crate::random::<24>();
        let c = XChaCha20Poly1305::new((&*key).into())
            .encrypt(
                XNonce::from_slice(&nonce),
                Payload {
                    msg: &plain,
                    aad: AAD,
                },
            )
            .map_err(|_| Error::Malformed)?;
        let r = Request {
            v: VERSION,
            k: b64(&phone.public()),
            n: b64(&nonce),
            c: b64(&c),
        };
        let id = device_id(&phone.public(), &canonical(sign_key)?, &canonical(bio_key)?);
        Ok((serde_json::to_string(&r).map_err(|_| Error::Malformed)?, id))
    }
}

/// The pairing key: X25519 between the two, salted with the QR's secret, bound to both keys.
fn key(
    mine: &SealKey,
    theirs: &[u8; 32],
    secret: &[u8; 32],
    desktop: &[u8; 32],
    phone: &[u8; 32],
) -> Result<Zeroizing<[u8; 32]>, Error> {
    let shared = mine.agree(theirs)?;
    let mut info = AAD.to_vec();
    info.extend_from_slice(desktop);
    info.extend_from_slice(phone);
    let mut out = Zeroizing::new([0; 32]);
    // 32 bytes is always a valid length for HKDF-SHA256.
    let _ = Hkdf::<Sha256>::new(Some(secret), &*shared).expand(&info, &mut *out);
    Ok(out)
}

/// The name as shown in Settings: one line, no control characters, at most [`MAX_NAME`].
fn clean_name(name: &str) -> String {
    let name: String = name.chars().filter(|c| !c.is_control()).take(MAX_NAME).collect();
    let name = name.trim();
    if name.is_empty() {
        "Phone".to_string()
    } else {
        name.to_string()
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use p256::ecdsa::SigningKey;

    fn sec1(k: &SigningKey) -> Vec<u8> {
        k.verifying_key().to_encoded_point(false).as_bytes().to_vec()
    }

    struct Phone {
        seal: SealKey,
        sign: Vec<u8>,
        bio: Vec<u8>,
    }

    fn phone() -> Phone {
        Phone {
            seal: SealKey::generate(),
            sign: sec1(&SigningKey::random(&mut rand_core::OsRng)),
            bio: sec1(&SigningKey::random(&mut rand_core::OsRng)),
        }
    }

    fn request(qr: &str, p: &Phone, name: &str) -> (String, String) {
        scan(qr).unwrap().request(&p.seal, name, &p.sign, &p.bio).unwrap()
    }

    #[test]
    fn a_phone_pairs_with_what_it_scanned() {
        let now = Instant::now();
        let desktop = SealKey::generate();
        let mut invite = Invite::new(&desktop, "https://relay.example", now);
        let scanned = scan(&invite.qr()).unwrap();
        assert_eq!(scanned.desktop, desktop.public());
        assert_eq!(scanned.relay, "https://relay.example");
        let p = phone();
        let (req, id) = request(&invite.qr(), &p, "  Pixel\n8 ");
        let d = invite
            .accept(&desktop, &req, now + Duration::from_secs(119))
            .unwrap();
        assert_eq!(d.id, id);
        assert_eq!(d.name, "Pixel8");
        assert_eq!(d.seal_key, p.seal.public());
        assert_eq!((d.sign_key, d.bio_key), (p.sign, p.bio));
        assert!(!d.revoked);
    }

    #[test]
    fn a_compressed_key_is_kept_uncompressed() {
        let now = Instant::now();
        let desktop = SealKey::generate();
        let mut invite = Invite::new(&desktop, "https://relay.example", now);
        let p = phone();
        let k = SigningKey::random(&mut rand_core::OsRng);
        let short = k.verifying_key().to_encoded_point(true).as_bytes().to_vec();
        let (req, id) = scan(&invite.qr())
            .unwrap()
            .request(&p.seal, "x", &short, &p.bio)
            .unwrap();
        let d = invite.accept(&desktop, &req, now).unwrap();
        assert_eq!(d.sign_key, sec1(&k));
        assert_eq!(d.id, id);
    }

    #[test]
    fn a_qr_pairs_once_and_only_in_time() {
        let now = Instant::now();
        let desktop = SealKey::generate();
        let mut invite = Invite::new(&desktop, "https://relay.example", now);
        let (req, _) = request(&invite.qr(), &phone(), "a");
        assert_eq!(
            invite.accept(&desktop, &req, now + INVITE_LIFE),
            Err(Error::InviteExpired)
        );
        invite.accept(&desktop, &req, now).unwrap();
        let (again, _) = request(&invite.qr(), &phone(), "b");
        assert_eq!(invite.accept(&desktop, &again, now), Err(Error::InviteUsed));
    }

    #[test]
    fn without_the_secret_nobody_pairs() {
        let now = Instant::now();
        let desktop = SealKey::generate();
        let mut invite = Invite::new(&desktop, "https://relay.example", now);
        // Someone who knows the desktop's public key and relay, but not the QR's secret.
        let mut qr: serde_json::Value = serde_json::from_str(&invite.qr()).unwrap();
        qr["s"] = b64(&[7; 32]).into();
        let (forged, _) = request(&qr.to_string(), &phone(), "evil");
        assert_eq!(invite.accept(&desktop, &forged, now), Err(Error::Unsealed));
        // It spent nothing: the real phone still pairs.
        let (req, _) = request(&invite.qr(), &phone(), "mine");
        assert!(invite.accept(&desktop, &req, now).is_ok());
    }

    #[test]
    fn bad_keys_are_refused() {
        let now = Instant::now();
        let desktop = SealKey::generate();
        let mut invite = Invite::new(&desktop, "https://relay.example", now);
        let p = phone();
        let scanned = scan(&invite.qr()).unwrap();
        let (same, _) = scanned.request(&p.seal, "x", &p.sign, &p.sign).unwrap();
        assert_eq!(invite.accept(&desktop, &same, now), Err(Error::BadKey));
        // The same key twice, once compressed and once not.
        let k = SigningKey::random(&mut rand_core::OsRng);
        let short = k.verifying_key().to_encoded_point(true).as_bytes().to_vec();
        let (twice, _) = scanned.request(&p.seal, "x", &short, &sec1(&k)).unwrap();
        assert_eq!(invite.accept(&desktop, &twice, now), Err(Error::BadKey));
        // Off the curve: the phone cannot even build its request.
        assert_eq!(
            scanned.request(&p.seal, "x", &[4; 65], &p.bio),
            Err(Error::BadKey)
        );
        // A low-order point as the phone's key.
        let mut r: serde_json::Value = serde_json::from_str(&same).unwrap();
        r["k"] = b64(&[0; 32]).into();
        assert_eq!(invite.accept(&desktop, &r.to_string(), now), Err(Error::BadKey));
    }

    #[test]
    fn a_qr_that_is_not_ours_is_refused() {
        let desktop = SealKey::generate();
        let invite = Invite::new(&desktop, "file:///etc/passwd", Instant::now());
        assert_eq!(scan(&invite.qr()), Err(Error::Malformed));
        assert_eq!(scan("https://example.com"), Err(Error::Malformed));
        assert_eq!(scan(r#"{"v":2,"k":"","r":"","s":""}"#), Err(Error::Version));
    }
}
