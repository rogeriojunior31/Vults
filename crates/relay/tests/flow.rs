//! The whole flow through a real relay on loopback (`docs/dev/plan-zeca.md` L2): the desktop shows
//! a QR, the phone pairs through the relay, gets a card, answers it signed, and `core`'s ledger
//! redeems the answer. The relay sees only addresses and ciphertext.
#![allow(clippy::unwrap_used)]

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use p256::ecdsa::signature::Signer;
use p256::ecdsa::{Signature, SigningKey};
use vults_core::{
    AgentEvent, AgentKind, AgentUpdate, Ask, Decision, Effect, Input, RequestId, SessionKey, State, Terminal,
    reduce,
};
use vults_link::SealKey;
use vults_link::answer::{Act, Key, Unsigned, receive};
use vults_link::card::{Card, ToPhone};
use vults_link::envelope::{Link, Side};
use vults_link::pair::{Invite, scan};
use vults_relay::client::{Client, Error, Gate, Open};
use vults_relay::server::{Limits, serve};

async fn relay() -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(serve(listener, Limits::default()));
    format!("http://{addr}")
}

fn sec1(k: &SigningKey) -> Vec<u8> {
    k.verifying_key().to_encoded_point(false).as_bytes().to_vec()
}

const WAIT: Duration = Duration::from_secs(2);

#[tokio::test]
async fn a_phone_pairs_and_answers_a_card_through_the_relay() {
    let url = relay().await;
    let net: Arc<dyn Gate> = Arc::new(Open);
    let start = Instant::now();

    // The desktop shows a QR.
    let desktop_key = SealKey::generate();
    let mut invite = Invite::new(&desktop_key, &url, start);
    let desktop = Client::new(&url, net.clone()).unwrap();

    // The phone scans it and answers through the relay the QR names.
    let scanned = scan(&invite.qr()).unwrap();
    let phone_seal = SealKey::generate();
    let sign = SigningKey::random(&mut rand_core::OsRng);
    let bio = SigningKey::random(&mut rand_core::OsRng);
    let (request, id) = scanned
        .request(&phone_seal, "Pixel", &sec1(&sign), &sec1(&bio))
        .unwrap();
    let phone = Client::new(&scanned.relay, net.clone()).unwrap();
    phone.send(&scanned.pair_box(), &request).await.unwrap();

    // The desktop reads the pairing box and welcomes the phone over the new link.
    let got = desktop.receive(&invite.pair_box(), WAIT).await.unwrap();
    assert_eq!(got.len(), 1);
    let device = invite.accept(&desktop_key, &got[0], start).unwrap();
    let to_phone = Link::new(Side::Desktop, &desktop_key, &device.seal_key, &device.id).unwrap();
    let welcome = ToPhone::Welcome {
        device: device.id.clone(),
        name: device.name.clone(),
    };
    desktop
        .send(to_phone.outbox(), &to_phone.seal(&welcome.to_bytes()))
        .await
        .unwrap();

    // An agent asks; the desktop sends the card.
    let mut state = State::default();
    let session = SessionKey {
        agent: AgentKind::Claude,
        session_id: "s1".into(),
    };
    reduce(
        &mut state,
        Input::Agent(AgentUpdate {
            session,
            cwd: Some("/home/me/vults".into()),
            terminal: Terminal::default(),
            agent_id: None,
            event: AgentEvent::PermissionRequested {
                request: RequestId("r1".into()),
                tool: "Bash".into(),
                target: "Bash · cargo test".into(),
                ask: Ask::default(),
            },
            received: Some(start),
        }),
        start,
    );
    let card = ToPhone::Card(Box::new(Card::of(&state, &state.pending[0], start)));
    let sealed = to_phone.seal(&card.to_bytes());
    assert!(!sealed.contains("cargo test"), "the relay sees the command");
    desktop.send(to_phone.outbox(), &sealed).await.unwrap();

    // The phone gets both, in order, and answers the card with biometrics.
    let link = Link::new(Side::Phone, &phone_seal, &scanned.desktop, &id).unwrap();
    let inbox = phone.receive(link.inbox(), WAIT).await.unwrap();
    let messages: Vec<ToPhone> = inbox
        .iter()
        .map(|e| ToPhone::from_bytes(&link.open(e).unwrap()).unwrap())
        .collect();
    assert_eq!(messages[0], welcome);
    let ToPhone::Card(card) = &messages[1] else {
        panic!("no card: {messages:?}")
    };
    assert_eq!(card.target, "Bash · cargo test");
    assert_eq!(card.cwd.as_deref(), Some("/home/me/vults"));
    let answer = Unsigned::new(&id, &card.request, &card.binding().unwrap(), 1, Act::Allow);
    let sig: Signature = bio.sign(&answer.message());
    phone
        .send(
            link.outbox(),
            &answer.seal(&link, Key::Bio, sig.to_der().as_bytes()),
        )
        .await
        .unwrap();

    // The desktop hears it; core answers the agent.
    let heard = desktop.receive(to_phone.inbox(), WAIT).await.unwrap();
    let verified = receive(&desktop_key, std::slice::from_ref(&device), &heard[0]).unwrap();
    let effects = reduce(&mut state, verified.input(), start + Duration::from_secs(3));
    assert!(effects.contains(&Effect::RespondPermission {
        request: RequestId("r1".into()),
        decision: Decision::Allow,
    }));
    // Taken once: nothing is left in either box.
    assert!(
        desktop
            .receive(to_phone.inbox(), Duration::ZERO)
            .await
            .unwrap()
            .is_empty()
    );
    assert!(
        phone
            .receive(link.inbox(), Duration::ZERO)
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn a_read_waits_for_what_comes() {
    let url = relay().await;
    let a = Client::new(&url, Arc::new(Open)).unwrap();
    let b = Client::new(&url, Arc::new(Open)).unwrap();
    let id = "c".repeat(64);
    let waiting = {
        let id = id.clone();
        tokio::spawn(async move { a.receive(&id, Duration::from_secs(5)).await })
    };
    tokio::time::sleep(Duration::from_millis(200)).await;
    let sent = Instant::now();
    b.send(&id, "late").await.unwrap();
    assert_eq!(waiting.await.unwrap().unwrap(), vec!["late"]);
    assert!(sent.elapsed() < Duration::from_secs(2), "the read did not wake");
}

#[tokio::test]
async fn the_relay_refuses_what_is_not_a_box_or_too_big() {
    let url = relay().await;
    let c = Client::new(&url, Arc::new(Open)).unwrap();
    assert_eq!(c.send("nope", "x").await, Err(Error::Relay(Some(404))));
    let big = "x".repeat(vults_link::MAX_MESSAGE + 1);
    assert_eq!(c.send(&"d".repeat(64), &big).await, Err(Error::Relay(Some(413))));
}

/// A gate that counts what it let out, closed or open.
struct Counted {
    open: bool,
    asked: AtomicUsize,
    done: AtomicUsize,
}

impl Gate for Counted {
    fn allow(&self) -> bool {
        self.asked.fetch_add(1, Ordering::SeqCst);
        self.open
    }
    fn done(&self, _: bool, _: Duration) {
        self.done.fetch_add(1, Ordering::SeqCst);
    }
}

#[tokio::test]
async fn a_closed_gate_lets_nothing_out() {
    let url = relay().await;
    let gate = Arc::new(Counted {
        open: false,
        asked: AtomicUsize::new(0),
        done: AtomicUsize::new(0),
    });
    let c = Client::new(&url, gate.clone()).unwrap();
    let id = "e".repeat(64);
    assert_eq!(c.send(&id, "x").await, Err(Error::Closed));
    assert_eq!(c.receive(&id, Duration::ZERO).await, Err(Error::Closed));
    assert_eq!(gate.asked.load(Ordering::SeqCst), 2);
    assert_eq!(gate.done.load(Ordering::SeqCst), 0);
    // Nothing reached the relay.
    let open = Client::new(&url, Arc::new(Open)).unwrap();
    assert!(open.receive(&id, Duration::ZERO).await.unwrap().is_empty());
}
