//! A phone's answer to a card (ADR 0017): signed by one of the device's two Keystore keys, sealed
//! in an envelope. The desktop opens it, checks the device, the signature and that the key fits the
//! act (allow, always and answer need the biometric key; deny and dismiss do not), and hands `core`
//! an [`Input::Paired`]: the ledger then checks the binding, the time, single use and the counter.

use p256::ecdsa::signature::Verifier;
use serde::{Deserialize, Serialize};
use vults_core::ledger::Binding;
use vults_core::{Decision, Input, Intent, RequestId};

use crate::envelope::{Link, Side};
use crate::{Device, Error, SealKey, VERSION, b64, hex, parse, unb64, verifying_key};

/// What the phone's answer does. Sending a message or moving money is not in the protocol.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(tag = "act", rename_all = "kebab-case")]
pub enum Act {
    Allow,
    Deny,
    /// Allow, and every identical request in the folder the card was offered in.
    Always,
    /// A question card's answers, one per question.
    Answer {
        answers: Vec<vults_core::Answer>,
    },
    /// Dismiss: the agent's terminal asks instead.
    Release,
}

impl Act {
    /// Whether it lets something run: then only the biometric key may sign it.
    pub fn needs_biometrics(&self) -> bool {
        matches!(self, Self::Allow | Self::Always | Self::Answer { .. })
    }
}

/// Which of the device's keys signed.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Key {
    /// Unlocked with the phone, no biometrics.
    Sign,
    /// Unlocked by the user's biometrics for this one use.
    Bio,
}

#[derive(Serialize, Deserialize)]
struct Body {
    v: u8,
    device: String,
    request: String,
    /// The card's binding, in hex, as the desktop sent it with the card.
    binding: String,
    counter: u64,
    #[serde(flatten)]
    act: Act,
}

#[derive(Serialize, Deserialize)]
struct Signed {
    v: u8,
    /// The body's exact bytes, so the signature is checked on what was signed.
    body: String,
    key: Key,
    /// ECDSA P-256 over SHA-256, DER, as Android's `SHA256withECDSA` gives it.
    sig: String,
}

/// An answer the phone built and has yet to sign.
#[derive(Debug, Clone)]
pub struct Unsigned {
    body: Vec<u8>,
}

impl Unsigned {
    /// The answer of device `device` to card `request` (whose binding the desktop sent), with a
    /// counter higher than any it used before.
    pub fn new(device: &str, request: &str, binding: &[u8; 32], counter: u64, act: Act) -> Self {
        let body = Body {
            v: VERSION,
            device: device.to_string(),
            request: request.to_string(),
            binding: hex(binding),
            counter,
            act,
        };
        Self {
            body: serde_json::to_vec(&body).unwrap_or_default(),
        }
    }

    /// The bytes the Keystore key signs: labeled, so they can never pass for another message.
    pub fn message(&self) -> Vec<u8> {
        message(&self.body)
    }

    /// The signed answer, ready to seal in an envelope.
    pub fn signed(&self, key: Key, der: &[u8]) -> Vec<u8> {
        let s = Signed {
            v: VERSION,
            body: b64(&self.body),
            key,
            sig: b64(der),
        };
        serde_json::to_vec(&s).unwrap_or_default()
    }

    /// The phone's whole send: signed and sealed for the desktop.
    pub fn seal(&self, link: &Link, key: Key, der: &[u8]) -> String {
        link.seal(&self.signed(key, der))
    }
}

/// An answer that is the device's own, signed with the key its act needs. Not yet an answer to the
/// card: the ledger decides that.
#[derive(Debug, Clone, PartialEq)]
pub struct Verified {
    pub device: String,
    pub request: RequestId,
    pub binding: Binding,
    pub counter: u64,
    pub act: Act,
}

impl Verified {
    /// What `core` gets.
    pub fn input(self) -> Input {
        let request = self.request;
        let intent = match self.act {
            Act::Allow => Intent::Decide {
                request,
                decision: Decision::Allow,
            },
            Act::Deny => Intent::Decide {
                request,
                decision: Decision::Deny,
            },
            Act::Always => Intent::DecideAlways { request },
            Act::Answer { answers } => Intent::Answer { request, answers },
            Act::Release => Intent::Release { request },
        };
        Input::Paired {
            device: self.device,
            counter: self.counter,
            binding: self.binding,
            intent,
        }
    }
}

/// The desktop receives an envelope from `device`'s box (the envelope does not say whose it is).
pub fn receive(desktop: &SealKey, device: &Device, envelope: &str) -> Result<Verified, Error> {
    if device.revoked {
        return Err(Error::Revoked);
    }
    let plain = Link::new(Side::Desktop, desktop, &device.seal_key, &device.id)?.open(envelope)?;
    let signed: Signed = parse(&plain)?;
    let body_bytes = unb64(&signed.body)?;
    let public = match signed.key {
        Key::Sign => &device.sign_key,
        Key::Bio => &device.bio_key,
    };
    let sig = p256::ecdsa::Signature::from_der(&unb64(&signed.sig)?).map_err(|_| Error::BadSignature)?;
    verifying_key(public)?
        .verify(&message(&body_bytes), &sig)
        .map_err(|_| Error::BadSignature)?;
    let body: Body = parse(&body_bytes)?;
    // Signed by this device, but for another one: not an answer it may give.
    if body.device != device.id {
        return Err(Error::Malformed);
    }
    if body.act.needs_biometrics() && signed.key != Key::Bio {
        return Err(Error::NeedsBiometrics);
    }
    Ok(Verified {
        device: body.device,
        request: RequestId(body.request),
        binding: Binding(unhex(&body.binding)?),
        counter: body.counter,
        act: body.act,
    })
}

fn message(body: &[u8]) -> Vec<u8> {
    let mut m = b"vults-link/1/answer\0".to_vec();
    m.extend_from_slice(body);
    m
}

pub(crate) fn unhex(s: &str) -> Result<[u8; 32], Error> {
    if s.len() != 64 || !s.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(Error::Malformed);
    }
    let mut out = [0; 32];
    for (i, b) in out.iter_mut().enumerate() {
        *b = u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).map_err(|_| Error::Malformed)?;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn hex_goes_both_ways() {
        let b = [0xab; 32];
        assert_eq!(unhex(&hex(&b)).unwrap(), b);
        assert_eq!(unhex("zz"), Err(Error::Malformed));
        assert_eq!(unhex(&"g".repeat(64)), Err(Error::Malformed));
        assert_eq!(unhex(&format!("+a{}", "0".repeat(62))), Err(Error::Malformed));
    }

    #[test]
    fn only_what_lets_something_run_needs_biometrics() {
        assert!(Act::Allow.needs_biometrics());
        assert!(Act::Always.needs_biometrics());
        assert!(Act::Answer { answers: vec![] }.needs_biometrics());
        assert!(!Act::Deny.needs_biometrics());
        assert!(!Act::Release.needs_biometrics());
    }
}
