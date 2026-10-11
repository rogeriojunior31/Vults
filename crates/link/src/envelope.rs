//! What travels between the desktop and a paired phone, through the relay or straight: sealed with
//! XChaCha20-Poly1305 under a key from X25519 and HKDF, one key per direction, so a message can
//! never be sent back to the side that wrote it. The relay reads only the device's id, and keeps
//! each direction's messages in a box whose address only the two ends can derive.

use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{XChaCha20Poly1305, XNonce};
use hkdf::Hkdf;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use zeroize::Zeroizing;

use crate::{Error, SealKey, VERSION, b64, parse, unb64, unb64_array};

/// Which side writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Desktop,
    Phone,
}

impl Side {
    fn byte(self) -> u8 {
        match self {
            Self::Desktop => 0,
            Self::Phone => 1,
        }
    }

    fn other(self) -> Self {
        match self {
            Self::Desktop => Self::Phone,
            Self::Phone => Self::Desktop,
        }
    }
}

#[derive(Serialize, Deserialize)]
struct Envelope {
    v: u8,
    /// The device's id: the relay routes on it.
    d: String,
    n: String,
    c: String,
}

/// One side's end of the link with one device.
pub struct Link {
    side: Side,
    device: String,
    send: Zeroizing<[u8; 32]>,
    receive: Zeroizing<[u8; 32]>,
    outbox: String,
    inbox: String,
}

impl std::fmt::Debug for Link {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Link")
            .field("side", &self.side)
            .field("device", &self.device)
            .finish_non_exhaustive()
    }
}

impl Link {
    /// `mine` is this side's key, `theirs` the other side's public key, `device` the phone's id.
    pub fn new(side: Side, mine: &SealKey, theirs: &[u8; 32], device: &str) -> Result<Self, Error> {
        let shared = mine.agree(theirs)?;
        let (desktop, phone) = match side {
            Side::Desktop => (mine.public(), *theirs),
            Side::Phone => (*theirs, mine.public()),
        };
        let derive = |label: &[u8], writer: Side| {
            let mut info = label.to_vec();
            info.push(writer.byte());
            info.extend_from_slice(&desktop);
            info.extend_from_slice(&phone);
            info.extend_from_slice(device.as_bytes());
            let mut out = Zeroizing::new([0; 32]);
            // 32 bytes is always a valid length for HKDF-SHA256.
            let _ = Hkdf::<Sha256>::new(None, &*shared).expand(&info, &mut *out);
            out
        };
        // The box's address is apart from the key: the relay learns one, never the other.
        let address = |writer: Side| crate::hex(&*derive(b"vults-link/1/box", writer));
        Ok(Self {
            side,
            device: device.to_string(),
            send: derive(b"vults-link/1/envelope", side),
            receive: derive(b"vults-link/1/envelope", side.other()),
            outbox: address(side),
            inbox: address(side.other()),
        })
    }

    pub fn device(&self) -> &str {
        &self.device
    }

    /// The relay box this side sends to.
    pub fn outbox(&self) -> &str {
        &self.outbox
    }

    /// The relay box this side reads from: the other side's outbox.
    pub fn inbox(&self) -> &str {
        &self.inbox
    }

    /// `plain`, sealed for the other side.
    pub fn seal(&self, plain: &[u8]) -> String {
        let nonce = crate::random::<24>();
        let c = XChaCha20Poly1305::new((&*self.send).into())
            .encrypt(
                XNonce::from_slice(&nonce),
                Payload {
                    msg: plain,
                    aad: &aad(self.side, &self.device),
                },
            )
            .unwrap_or_default();
        let e = Envelope {
            v: VERSION,
            d: self.device.clone(),
            n: b64(&nonce),
            c: b64(&c),
        };
        serde_json::to_string(&e).unwrap_or_default()
    }

    /// What the other side sealed for this one.
    pub fn open(&self, envelope: &str) -> Result<Vec<u8>, Error> {
        let e: Envelope = parse(envelope.as_bytes())?;
        if e.d != self.device {
            return Err(Error::Unsealed);
        }
        let nonce: [u8; 24] = unb64_array(&e.n)?;
        let c = unb64(&e.c)?;
        XChaCha20Poly1305::new((&*self.receive).into())
            .decrypt(
                XNonce::from_slice(&nonce),
                Payload {
                    msg: &c,
                    aad: &aad(self.side.other(), &self.device),
                },
            )
            .map_err(|_| Error::Unsealed)
    }
}

/// The id an envelope is for, read without opening it: which device's link opens it.
pub fn device_of(envelope: &str) -> Result<String, Error> {
    let e: Envelope = parse(envelope.as_bytes())?;
    Ok(e.d)
}

fn aad(writer: Side, device: &str) -> Vec<u8> {
    let mut aad = b"vults-link/1/envelope".to_vec();
    aad.push(writer.byte());
    aad.extend_from_slice(device.as_bytes());
    aad
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    fn pair() -> (Link, Link) {
        let desktop = SealKey::generate();
        let phone = SealKey::generate();
        (
            Link::new(Side::Desktop, &desktop, &phone.public(), "dev1").unwrap(),
            Link::new(Side::Phone, &phone, &desktop.public(), "dev1").unwrap(),
        )
    }

    #[test]
    fn each_side_opens_what_the_other_sealed() {
        let (desktop, phone) = pair();
        let e = phone.seal(b"deny r1");
        assert_eq!(device_of(&e).unwrap(), "dev1");
        assert_eq!(desktop.open(&e).unwrap(), b"deny r1");
        assert_eq!(phone.open(&desktop.seal(b"card")).unwrap(), b"card");
    }

    #[test]
    fn each_side_reads_the_box_the_other_writes() {
        let (desktop, phone) = pair();
        assert_eq!(desktop.outbox(), phone.inbox());
        assert_eq!(phone.outbox(), desktop.inbox());
        assert_ne!(desktop.outbox(), desktop.inbox());
        assert_eq!(desktop.outbox().len(), 64);
        let (other, _) = pair();
        assert_ne!(other.outbox(), desktop.outbox());
    }

    #[test]
    fn a_message_never_goes_back_to_its_writer() {
        let (desktop, phone) = pair();
        assert_eq!(phone.open(&phone.seal(b"x")), Err(Error::Unsealed));
        assert_eq!(desktop.open(&desktop.seal(b"x")), Err(Error::Unsealed));
    }

    #[test]
    fn a_changed_envelope_does_not_open() {
        let (desktop, phone) = pair();
        let e = phone.seal(b"deny r1");
        let mut v: serde_json::Value = serde_json::from_str(&e).unwrap();
        let c = v["c"].as_str().unwrap().to_string();
        let flipped = format!("{}{}", if c.starts_with('A') { "B" } else { "A" }, &c[1..]);
        v["c"] = flipped.into();
        assert_eq!(desktop.open(&v.to_string()), Err(Error::Unsealed));
        // Readdressed to another device's link over the same keys: the id is sealed in.
        let (desktop_key, phone_key) = (SealKey::generate(), SealKey::generate());
        let phone1 = Link::new(Side::Phone, &phone_key, &desktop_key.public(), "dev1").unwrap();
        let desktop2 = Link::new(Side::Desktop, &desktop_key, &phone_key.public(), "dev2").unwrap();
        let mut v: serde_json::Value = serde_json::from_str(&phone1.seal(b"allow")).unwrap();
        v["d"] = "dev2".into();
        assert_eq!(desktop2.open(&v.to_string()), Err(Error::Unsealed));
        // Another version is refused before anything is opened.
        let mut v: serde_json::Value = serde_json::from_str(&e).unwrap();
        v["v"] = 2.into();
        assert_eq!(desktop.open(&v.to_string()), Err(Error::Version));
    }

    #[test]
    fn a_stranger_does_not_open_it() {
        let (desktop, _) = pair();
        let (_, stranger) = pair();
        assert_eq!(desktop.open(&stranger.seal(b"allow")), Err(Error::Unsealed));
    }
}
