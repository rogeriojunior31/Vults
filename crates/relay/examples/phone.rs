//! A phone on the command line, to try the relay before the Android app exists (plan-zeca L2).
//!
//! ```text
//! cargo run -p vults-relay --example phone -- '<the QR text from Settings>' [name]
//! ```
//!
//! It pairs, then prints each card and asks what to do with it. Its keys live only in memory, as a
//! test client's should: quit and it must pair again. Allow, Always and an answer are signed with its
//! "biometric" key, deny and dismiss with the other one, as a real phone would.

use std::io::{BufRead, Write};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use p256::ecdsa::signature::Signer;
use p256::ecdsa::{Signature, SigningKey};
use p256::elliptic_curve::Generate;
use vults_link::SealKey;
use vults_link::answer::{Act, Key, Unsigned};
use vults_link::card::{Card, ToPhone};
use vults_link::envelope::{Link, Side};
use vults_link::pair::scan;
use vults_relay::client::{Client, Open};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let qr = args.next().ok_or("usage: phone '<QR text>' [name]")?;
    let name = args.next().unwrap_or_else(|| "Test phone".into());
    let rt = tokio::runtime::Runtime::new()?;

    let scanned = scan(&qr).map_err(|e| format!("not a Vults QR: {e}"))?;
    let seal = SealKey::generate();
    let sign = SigningKey::generate();
    let bio = SigningKey::generate();
    let public = |k: &SigningKey| k.verifying_key().to_sec1_point(false).as_bytes().to_vec();
    let (request, id) = scanned.request(&seal, &name, &public(&sign), &public(&bio))?;
    let client = Client::new(&scanned.relay, Arc::new(Open))?;
    rt.block_on(client.send(&scanned.pair_box(), &request))?;
    println!("pairing as {id} through {}", scanned.relay);

    let link = Link::new(Side::Phone, &seal, &scanned.desktop, &id)?;
    let mut counter = 0u64;
    let stdin = std::io::stdin();
    loop {
        let inbox = match rt.block_on(client.receive(link.inbox(), Duration::from_secs(25))) {
            Ok(inbox) => inbox,
            Err(e) => {
                eprintln!("{e}; trying again in 5 s");
                std::thread::sleep(Duration::from_secs(5));
                continue;
            }
        };
        for envelope in inbox {
            let message = link.open(&envelope).and_then(|plain| ToPhone::from_bytes(&plain));
            match message {
                Ok(ToPhone::Welcome { name, .. }) => {
                    println!("paired: the desktop knows this phone as {name}")
                }
                Ok(ToPhone::Gone { request }) => println!("card {request} left the line"),
                Ok(ToPhone::Card(card)) => {
                    let Some(act) = ask(&card, &mut stdin.lock())? else {
                        continue;
                    };
                    // Higher than any used before, even after this client restarts.
                    counter = (counter + 1).max(now_ms());
                    let (key, signer) = if act.needs_biometrics() {
                        (Key::Bio, &bio)
                    } else {
                        (Key::Sign, &sign)
                    };
                    let answer = Unsigned::new(&id, &card.request, &card.binding()?, counter, act);
                    let sig: Signature = signer.sign(&answer.message());
                    let sealed = answer.seal(&link, key, sig.to_der().as_bytes());
                    rt.block_on(client.send(link.outbox(), &sealed))?;
                    println!("sent");
                }
                Err(e) => eprintln!("a message that does not open ({e}): dropped"),
            }
        }
    }
}

/// Shows the card and reads what to do; `None` leaves it to the desktop.
fn ask(card: &Card, input: &mut impl BufRead) -> std::io::Result<Option<Act>> {
    println!();
    println!("{} · {} wants: {}", card.agent, card.project, card.target);
    if let Some(full) = &card.full {
        println!("  whole command: {full}");
    }
    if let Some(cwd) = &card.cwd {
        println!("  in {cwd}");
    }
    println!("  {} s left", card.left_ms / 1000);
    if !card.questions.is_empty() {
        let mut answers = Vec::new();
        for q in &card.questions {
            println!("  {}: {} ({})", q.header, q.question, q.options.join(" / "));
            let line = prompt("  answer (empty to skip): ", input)?;
            if line.is_empty() {
                return Ok(None);
            }
            answers.push(vults_core::Answer::One(line));
        }
        return Ok(Some(Act::Answer { answers }));
    }
    let always = if card.cut { "" } else { ", [A]lways" };
    let line = prompt(&format!("  [a]llow, [d]eny{always}, [r]elease, [s]kip: "), input)?;
    Ok(match line.as_str() {
        "a" => Some(Act::Allow),
        "A" if !card.cut => Some(Act::Always),
        "d" => Some(Act::Deny),
        "r" => Some(Act::Release),
        _ => None,
    })
}

fn prompt(text: &str, input: &mut impl BufRead) -> std::io::Result<String> {
    print!("{text}");
    std::io::stdout().flush()?;
    let mut line = String::new();
    input.read_line(&mut line)?;
    Ok(line.trim().to_string())
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(0))
}
