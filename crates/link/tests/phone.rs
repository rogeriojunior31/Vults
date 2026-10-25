//! A paired phone answering cards end to end: paired by QR, signed with its keys, sealed, opened
//! by the desktop and redeemed by `core`'s ledger (ADR 0004, `docs/dev/plan-zeca.md` L1).
#![allow(clippy::unwrap_used)]

use std::time::{Duration, Instant};

use p256::ecdsa::signature::Signer;
use p256::ecdsa::{Signature, SigningKey};
use p256::elliptic_curve::Generate;
use vults_core::audit::Actor;
use vults_core::ledger::Binding;
use vults_core::{
    AgentEvent, AgentKind, AgentUpdate, Ask, Decision, Effect, Input, RequestId, SessionKey, State, Terminal,
    reduce,
};
use vults_link::answer::{Act, Key, Unsigned, receive};
use vults_link::envelope::{Link, Side};
use vults_link::pair::{Invite, scan};
use vults_link::{Device, Error, SealKey};

struct Phone {
    sign: SigningKey,
    bio: SigningKey,
    link: Link,
    id: String,
}

impl Phone {
    /// The phone's whole answer: built, signed with `key` (`with` is the key it really signs with),
    /// sealed for the desktop.
    fn send(
        &self,
        request: &str,
        binding: Binding,
        counter: u64,
        act: Act,
        key: Key,
        with: &SigningKey,
    ) -> String {
        let a = Unsigned::new(&self.id, request, &binding.0, counter, act);
        let sig: Signature = with.sign(&a.message());
        a.seal(&self.link, key, sig.to_der().as_bytes())
    }

    fn allow(&self, request: &str, binding: Binding, counter: u64) -> String {
        self.send(request, binding, counter, Act::Allow, Key::Bio, &self.bio)
    }
}

struct Desk {
    key: SealKey,
    devices: Vec<Device>,
    state: State,
    start: Instant,
}

impl Desk {
    /// The desktop receives `envelope` at `secs` after the start.
    fn hear(&mut self, envelope: &str, secs: f64) -> Result<Vec<Effect>, Error> {
        let v = receive(&self.key, &self.devices[0], envelope)?;
        let now = self.start + Duration::from_secs_f64(secs);
        Ok(reduce(&mut self.state, v.input(), now))
    }

    fn agent(&mut self, cwd: &str, event: AgentEvent, secs: f64) -> Vec<Effect> {
        let now = self.start + Duration::from_secs_f64(secs);
        let update = AgentUpdate {
            session: session(),
            cwd: Some(cwd.into()),
            terminal: Terminal::default(),
            agent_id: None,
            event,
            received: Some(now),
        };
        reduce(&mut self.state, Input::Agent(update), now)
    }

    /// An agent asks for `target` in `cwd`; the binding the desktop sends the phone with the card.
    fn card(&mut self, request: &str, target: &str, cwd: &str, secs: f64) -> Binding {
        self.agent(
            cwd,
            AgentEvent::PermissionRequested {
                request: RequestId(request.into()),
                tool: "Bash".into(),
                target: target.into(),
                ask: Ask::default(),
            },
            secs,
        );
        let p = self
            .state
            .pending
            .iter()
            .find(|p| p.request.0 == request)
            .unwrap();
        Binding::of(p)
    }

    fn waiting(&self, request: &str) -> bool {
        self.state.pending.iter().any(|p| p.request.0 == request)
    }
}

fn session() -> SessionKey {
    SessionKey {
        agent: AgentKind::Claude,
        session_id: "s1".into(),
    }
}

fn sec1(k: &SigningKey) -> Vec<u8> {
    k.verifying_key().to_sec1_point(false).as_bytes().to_vec()
}

/// A desktop and a phone paired through the QR, as the user would.
fn paired() -> (Desk, Phone) {
    let start = Instant::now();
    let desktop = SealKey::generate();
    let mut invite = Invite::new(&desktop, "https://relay.example", start);
    let seal = SealKey::generate();
    let sign = SigningKey::generate();
    let bio = SigningKey::generate();
    let scanned = scan(&invite.qr()).unwrap();
    let (request, id) = scanned
        .request(&seal, "Pixel", &sec1(&sign), &sec1(&bio))
        .unwrap();
    let device = invite.accept(&desktop, &request, start).unwrap();
    assert_eq!(device.id, id);
    let link = Link::new(Side::Phone, &seal, &scanned.desktop, &id).unwrap();
    let desk = Desk {
        key: desktop,
        devices: vec![device],
        state: State::default(),
        start,
    };
    (desk, Phone { sign, bio, link, id })
}

fn allowed(effects: &[Effect], request: &str) -> bool {
    effects.iter().any(|e| {
        matches!(e, Effect::RespondPermission { request: r, decision: Decision::Allow } if r.0 == request)
    })
}

#[test]
fn an_approval_with_biometrics_answers_the_card() {
    let (mut desk, phone) = paired();
    let b = desk.card("r1", "Bash · cargo test", "/work", 0.0);
    let effects = desk.hear(&phone.allow("r1", b, 1), 100.0).unwrap();
    assert!(allowed(&effects, "r1"));
    assert!(
        effects
            .iter()
            .any(|e| matches!(e, Effect::Audit(a) if a.actor == Actor::Phone))
    );
    assert!(effects.contains(&Effect::DeviceCounter {
        device: phone.id.clone(),
        counter: 1,
    }));
    assert!(!desk.waiting("r1"));
}

#[test]
fn a_replayed_answer_takes_nothing() {
    let (mut desk, phone) = paired();
    let b1 = desk.card("r1", "Bash · cargo test", "/work", 0.0);
    let first = phone.allow("r1", b1, 5);
    assert!(allowed(&desk.hear(&first, 1.0).unwrap(), "r1"));
    // The same envelope again, for a new card with the same id and command.
    desk.card("r1", "Bash · cargo test", "/work", 2.0);
    assert_eq!(desk.hear(&first, 3.0).unwrap(), vec![]);
    // A fresh answer with a counter already used, or a lower one.
    let b2 = desk.card("r2", "Bash · cargo build", "/work", 4.0);
    assert_eq!(desk.hear(&phone.allow("r2", b2, 5), 5.0).unwrap(), vec![]);
    assert_eq!(desk.hear(&phone.allow("r2", b2, 4), 5.0).unwrap(), vec![]);
    assert!(desk.waiting("r2"));
    assert!(allowed(&desk.hear(&phone.allow("r2", b2, 6), 6.0).unwrap(), "r2"));
}

#[test]
fn a_replay_stays_one_after_a_restart() {
    let (mut desk, phone) = paired();
    // What the database kept from before the restart.
    reduce(
        &mut desk.state,
        Input::DeviceCounters(vec![(phone.id.clone(), 9)]),
        desk.start,
    );
    let b = desk.card("r1", "Bash · cargo test", "/work", 0.0);
    assert_eq!(desk.hear(&phone.allow("r1", b, 9), 1.0).unwrap(), vec![]);
    assert!(allowed(&desk.hear(&phone.allow("r1", b, 10), 1.0).unwrap(), "r1"));
}

#[test]
fn a_late_answer_is_refused() {
    let (mut desk, phone) = paired();
    let b = desk.card("r1", "Bash · cargo test", "/work", 0.0);
    // The hook waits 108 s; the offer ends a second before, so the answer can still travel.
    assert_eq!(desk.hear(&phone.allow("r1", b, 1), 107.0).unwrap(), vec![]);
    let b = desk.card("r2", "Bash · cargo test", "/work", 200.0);
    assert!(allowed(
        &desk.hear(&phone.allow("r2", b, 2), 306.9).unwrap(),
        "r2"
    ));
}

#[test]
fn an_answer_for_another_command_is_refused() {
    let (mut desk, phone) = paired();
    desk.card("r1", "Bash · rm -rf /", "/work", 0.0);
    // The phone was shown (or a forger claims) a harmless command under the same id.
    let mut other = Desk {
        key: SealKey::generate(),
        devices: Vec::new(),
        state: State::default(),
        start: desk.start,
    };
    let wrong = phone.allow("r1", other.card("r1", "Bash · ls", "/work", 0.0), 1);
    assert_eq!(desk.hear(&wrong, 1.0).unwrap(), vec![]);
    assert!(desk.waiting("r1"));
    // The same card offered in another folder binds differently too.
    let elsewhere = phone.allow("r1", other.card("r9", "Bash · rm -rf /", "/tmp", 0.0), 2);
    assert_eq!(desk.hear(&elsewhere, 1.0).unwrap(), vec![]);
    // A refused answer took no counter: the right one with it still counts.
    let b = Binding::of(desk.state.pending.iter().find(|p| p.request.0 == "r1").unwrap());
    assert!(allowed(&desk.hear(&phone.allow("r1", b, 1), 2.0).unwrap(), "r1"));
}

#[test]
fn a_revoked_phone_is_not_heard() {
    let (mut desk, phone) = paired();
    let b = desk.card("r1", "Bash · cargo test", "/work", 0.0);
    desk.devices[0].revoked = true;
    assert_eq!(desk.hear(&phone.allow("r1", b, 1), 1.0), Err(Error::Revoked));
    // Another phone's envelope does not open as this one's.
    desk.devices[0].revoked = false;
    let (_, stranger) = paired();
    assert_eq!(desk.hear(&stranger.allow("r1", b, 1), 1.0), Err(Error::Unsealed));
    assert!(desk.waiting("r1"));
}

#[test]
fn an_approval_without_biometrics_is_refused() {
    let (mut desk, phone) = paired();
    let b = desk.card("r1", "Bash · cargo test", "/work", 0.0);
    // Signed with the plain key, and said so.
    let plain = phone.send("r1", b, 1, Act::Allow, Key::Sign, &phone.sign);
    assert_eq!(desk.hear(&plain, 1.0), Err(Error::NeedsBiometrics));
    let always = phone.send("r1", b, 1, Act::Always, Key::Sign, &phone.sign);
    assert_eq!(desk.hear(&always, 1.0), Err(Error::NeedsBiometrics));
    let answers = Act::Answer {
        answers: vec![vults_core::Answer::One("yes".into())],
    };
    let answer = phone.send("r1", b, 1, answers, Key::Sign, &phone.sign);
    assert_eq!(desk.hear(&answer, 1.0), Err(Error::NeedsBiometrics));
    // Signed with the plain key, but claiming the biometric one.
    let lie = phone.send("r1", b, 1, Act::Allow, Key::Bio, &phone.sign);
    assert_eq!(desk.hear(&lie, 1.0), Err(Error::BadSignature));
    // Signed by a key that is not the device's at all.
    let stranger = SigningKey::generate();
    let forged = phone.send("r1", b, 1, Act::Allow, Key::Bio, &stranger);
    assert_eq!(desk.hear(&forged, 1.0), Err(Error::BadSignature));
    assert!(desk.waiting("r1"));
    // A deny needs no biometrics.
    let deny = phone.send("r1", b, 1, Act::Deny, Key::Sign, &phone.sign);
    let effects = desk.hear(&deny, 1.0).unwrap();
    assert!(effects.iter().any(|e| matches!(
        e,
        Effect::RespondPermission {
            decision: Decision::Deny,
            ..
        }
    )));
}

#[test]
fn the_last_counter_is_refused() {
    let (mut desk, phone) = paired();
    let b = desk.card("r1", "Bash · cargo test", "/work", 0.0);
    assert_eq!(desk.hear(&phone.allow("r1", b, u64::MAX), 1.0).unwrap(), vec![]);
    assert!(desk.waiting("r1"));
}

#[test]
fn always_from_the_phone_is_scoped_to_the_folder_it_was_offered_in() {
    let (mut desk, phone) = paired();
    let b = desk.card("r1", "Bash · cargo test", "/work", 0.0);
    // The agent moves before the user answers on the phone (a parallel call asks from there).
    desk.card("r2", "Bash · ls", "/elsewhere", 1.0);
    assert!(desk.waiting("r1"));
    let always = phone.send("r1", b, 1, Act::Always, Key::Bio, &phone.bio);
    let effects = desk.hear(&always, 2.0).unwrap();
    assert!(allowed(&effects, "r1"));
    assert_eq!(desk.state.rules.len(), 1);
    assert_eq!(desk.state.rules[0].cwd, "/work");
}

#[test]
fn a_dismiss_goes_to_the_terminal() {
    let (mut desk, phone) = paired();
    let b = desk.card("r1", "Bash · cargo test", "/work", 0.0);
    let dismiss = phone.send("r1", b, 1, Act::Release, Key::Sign, &phone.sign);
    let effects = desk.hear(&dismiss, 1.0).unwrap();
    assert!(effects.contains(&Effect::ReleasePermission(RequestId("r1".into()))));
    // A replay of it finds nothing.
    assert_eq!(desk.hear(&dismiss, 1.0).unwrap(), vec![]);
}
