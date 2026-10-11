//! The desktop's half, on the command line, to try `phone` against a relay before the app sends
//! real cards (plan-zeca L2). It shows the QR's text, waits for the phone, sends it one card, and
//! prints what `core` does with the phone's answer.
//!
//! ```text
//! cargo run -p vults-relay --bin vults-relay                # a relay on :8787
//! cargo run -p vults-relay --example desk -- http://127.0.0.1:8787
//! cargo run -p vults-relay --example phone -- '<the text desk printed>'
//! ```

use std::sync::Arc;
use std::time::{Duration, Instant};

use vults_core::{
    AgentEvent, AgentKind, AgentUpdate, Ask, Input, RequestId, SessionKey, State, Terminal, reduce,
};
use vults_link::SealKey;
use vults_link::card::{Card, ToPhone};
use vults_link::envelope::{Link, Side};
use vults_link::pair::Invite;
use vults_relay::client::{Client, Open};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let relay = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "http://127.0.0.1:8787".into());
    let rt = tokio::runtime::Runtime::new()?;
    let client = Client::new(&relay, Arc::new(Open))?;
    let key = SealKey::generate();
    let mut invite = Invite::new(&key, &relay, Instant::now());
    println!("QR text (valid 2 minutes):\n{}\n", invite.qr());

    let device = loop {
        let got = rt.block_on(client.receive(&invite.pair_box(), Duration::from_secs(25)))?;
        if let Some(request) = got.first() {
            break invite.accept(&key, request, Instant::now())?;
        }
    };
    println!("paired with {} ({})", device.name, device.id);
    let link = Link::new(Side::Desktop, &key, &device.seal_key, &device.id)?;
    let welcome = ToPhone::Welcome {
        device: device.id.clone(),
        name: device.name.clone(),
    };
    rt.block_on(client.send(link.outbox(), &link.seal(&welcome.to_bytes())))?;

    let mut state = State::default();
    let now = Instant::now();
    reduce(
        &mut state,
        Input::Agent(AgentUpdate {
            session: SessionKey {
                agent: AgentKind::Claude,
                session_id: "demo".into(),
            },
            cwd: Some("/home/me/vults".into()),
            terminal: Terminal::default(),
            agent_id: None,
            event: AgentEvent::PermissionRequested {
                request: RequestId("demo-1".into()),
                tool: "Bash".into(),
                target: "Bash · cargo test".into(),
                ask: Ask::default(),
            },
            received: Some(now),
        }),
        now,
    );
    let card = ToPhone::Card(Box::new(Card::of(&state, &state.pending[0], now)));
    rt.block_on(client.send(link.outbox(), &link.seal(&card.to_bytes())))?;
    println!("card sent; waiting for the answer");

    loop {
        for envelope in rt.block_on(client.receive(link.inbox(), Duration::from_secs(25)))? {
            match vults_link::answer::receive(&key, &device, &envelope) {
                Ok(v) => {
                    println!("verified: {:?} with counter {}", v.act, v.counter);
                    for effect in reduce(&mut state, v.input(), Instant::now()) {
                        println!("core: {effect:?}");
                    }
                    if state.pending.is_empty() {
                        return Ok(());
                    }
                }
                Err(e) => println!("refused: {e}"),
            }
        }
    }
}
