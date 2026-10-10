//! The evals (see `src/lib.rs`). Each group reads its cases from `cases/` and reports every case
//! that moved, not only the first.
#![cfg(feature = "evals")]
#![allow(clippy::unwrap_used)]

use std::sync::{Arc, Mutex};

use serde_json::Value;
use tokio::sync::oneshot;
use vults_chat::{Approver, Chat, Delta, Provider, Turn};
use vults_core::policy::{Class, classify};
use vults_evals::cases;

fn read(name: &str) -> Value {
    serde_json::from_str(&std::fs::read_to_string(cases().join(name)).unwrap()).unwrap()
}

fn class_name(c: Class) -> &'static str {
    match c {
        Class::Read => "read",
        Class::ReadExternal => "read-external",
        Class::Spend => "spend",
        Class::Write { destructive: false } => "write",
        Class::Write { destructive: true } => "destructive",
        Class::External => "external",
    }
}

fn report(group: &str, failures: Vec<String>) {
    assert!(
        failures.is_empty(),
        "{group}: {} case(s) moved:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn policy_classes() {
    let mut failures = Vec::new();
    for case in read("policy.json").as_array().unwrap() {
        let (tool, target) = (case["tool"].as_str().unwrap(), case["target"].as_str().unwrap());
        let got = class_name(classify(tool, target));
        if got != case["class"] {
            failures.push(format!("  {tool} {target:?}: {got}, expected {}", case["class"]));
        }
    }
    report("policy", failures);
}

#[test]
fn redaction() {
    let mut failures = Vec::new();
    for case in read("redact.json").as_array().unwrap() {
        let text = case["text"].as_str().unwrap();
        let out = vults_core::redact::secrets(text);
        for gone in case["gone"].as_array().unwrap() {
            if out.contains(gone.as_str().unwrap()) {
                failures.push(format!("  {text:?}: {gone} is still in {out:?}"));
            }
        }
        for kept in case["kept"].as_array().unwrap() {
            if !out.contains(kept.as_str().unwrap()) {
                failures.push(format!("  {text:?}: {kept} went missing from {out:?}"));
            }
        }
    }
    report("redaction", failures);
}

/// Every prompt the chat sends, against its snapshot. A change is accepted on purpose with
/// `VULTS_EVALS_BLESS=1`, and reviewed in the diff.
#[test]
fn prompts_sent_to_models() {
    let bless = std::env::var_os("VULTS_EVALS_BLESS").is_some();
    let mut failures = Vec::new();
    for (name, prompt) in vults_chat::prompts() {
        let file = cases().join("prompts").join(format!("{name}.txt"));
        if bless {
            std::fs::write(&file, &prompt).unwrap();
            continue;
        }
        match std::fs::read_to_string(&file) {
            Ok(kept) if kept == prompt => {}
            Ok(_) => failures.push(format!("  {name}: changed (VULTS_EVALS_BLESS=1 to accept it)")),
            Err(_) => failures.push(format!("  {name}: no snapshot (VULTS_EVALS_BLESS=1 to write it)")),
        }
    }
    report("prompts", failures);
}

/// Says no to every card, and remembers what each one showed.
#[derive(Default)]
struct Refuser(Mutex<Vec<(String, String)>>);

impl Approver for Refuser {
    fn wait(&self, _id: &str, tool: &str, target: &str) -> oneshot::Receiver<bool> {
        self.0.lock().unwrap().push((tool.to_owned(), target.to_owned()));
        let (tx, rx) = oneshot::channel();
        let _ = tx.send(false);
        rx
    }
}

/// Recorded Claude Code streams where the model is told, or tells, to skip asking: every tool it
/// wants still comes as a card showing the whole command, and only the card's answer reaches the
/// CLI. Played by `cases/claude-stand-in.sh`, put on the PATH as `claude`.
#[tokio::test]
async fn injection_never_skips_a_card() {
    let bin = std::env::temp_dir().join(format!("vults-evals-bin-{}", std::process::id()));
    std::fs::create_dir_all(&bin).unwrap();
    let claude = bin.join("claude");
    std::fs::copy(cases().join("claude-stand-in.sh"), &claude).unwrap();
    let path = format!("{}:{}", bin.display(), std::env::var("PATH").unwrap_or_default());
    // SAFETY: this test binary runs this one test that touches the environment, before any thread
    // of its own reads it.
    unsafe { std::env::set_var("PATH", path) };

    let mut failures = Vec::new();
    let mut names: Vec<_> = std::fs::read_dir(cases().join("cassettes"))
        .unwrap()
        .filter_map(|e| {
            e.ok()?
                .file_name()
                .into_string()
                .ok()?
                .strip_suffix(".jsonl")
                .map(str::to_owned)
        })
        .collect();
    names.sort();
    assert!(!names.is_empty());
    for name in names {
        let cassette = cases().join("cassettes").join(format!("{name}.jsonl"));
        let expect = read(&format!("cassettes/{name}.expect.json"));
        let decisions = bin.join(format!("{name}.decisions"));
        let _ = std::fs::remove_file(&decisions);
        unsafe {
            std::env::set_var("VULTS_EVAL_CASSETTE", &cassette);
            std::env::set_var("VULTS_EVAL_DECISIONS", &decisions);
        }
        let refuser = Arc::new(Refuser::default());
        let mut chat = Chat::new(Provider::Claude, std::env::temp_dir());
        let (tx, mut rx) = tokio::sync::mpsc::channel(64);
        let (_stop, stop) = oneshot::channel();
        let turn = Turn {
            text: "Tidy up my home folder.".into(),
            files: vec![],
        };
        let approver: Arc<dyn Approver> = refuser.clone();
        let run = chat.send(turn, tx, approver, stop);
        let collect = async {
            let mut ends = Vec::new();
            while let Some(d) = rx.recv().await {
                if matches!(d, Delta::Done | Delta::Error { .. } | Delta::Stopped) {
                    ends.push(format!("{d:?}"));
                }
            }
            ends
        };
        let ((), ends) = tokio::join!(run, collect);
        if ends != ["Done"] {
            failures.push(format!("  {name}: the turn ended {ends:?}"));
        }
        let asked = refuser.0.lock().unwrap().clone();
        let want = expect["asked"].as_array().unwrap();
        if asked.len() != want.len() {
            failures.push(format!(
                "  {name}: {} card(s), expected {}: {asked:?}",
                asked.len(),
                want.len()
            ));
        }
        for ((tool, target), w) in asked.iter().zip(want) {
            if tool != w["tool"].as_str().unwrap() || !target.contains(w["target_has"].as_str().unwrap()) {
                failures.push(format!(
                    "  {name}: the card showed {tool} {target:?}, expected {w}"
                ));
            }
        }
        let got: Vec<String> = std::fs::read_to_string(&decisions)
            .unwrap_or_default()
            .lines()
            .map(|l| {
                let v: Value = serde_json::from_str(l).unwrap_or_default();
                v["response"]["response"]["behavior"]
                    .as_str()
                    .unwrap_or("?")
                    .to_owned()
            })
            .collect();
        let want: Vec<String> = expect["decisions"]
            .as_array()
            .unwrap()
            .iter()
            .map(|d| d.as_str().unwrap().to_owned())
            .collect();
        if got != want {
            failures.push(format!("  {name}: the CLI was told {got:?}, expected {want:?}"));
        }
    }
    report("injection", failures);
}
