//! The evals (see `src/lib.rs`) that need no process: policy, redaction, the prompts. Each group
//! reads its cases from `cases/` and reports every case that moved, not only the first. Injection
//! has a test binary of its own (`injection.rs`).
#![cfg(feature = "evals")]
#![allow(clippy::unwrap_used)]

use serde_json::Value;
use vults_core::policy::{Class, Confidence, classify};
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
        let got = classify(tool, target);
        let name = class_name(got.class);
        if name != case["class"] {
            failures.push(format!("  {tool} {target:?}: {name}, expected {}", case["class"]));
        }
        // Optional per case: only a known class may ever be allowed by a policy (W2).
        let sure = match got.confidence {
            Confidence::Known => "known",
            Confidence::Guessed => "guessed",
        };
        if let Some(want) = case["confidence"].as_str().filter(|w| *w != sure) {
            failures.push(format!("  {tool} {target:?}: {sure}, expected {want}"));
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
        // The whole output: a secret half left, or text changed around it, moves the case.
        if out != case["out"].as_str().unwrap() {
            failures.push(format!("  {text:?}: {out:?}, expected {}", case["out"]));
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
    let mut prompts = vults_chat::prompts();
    prompts.extend(vults_zeca::context::prompts());
    // A snapshot left after its prompt went fails too: it would vouch for nothing.
    for entry in std::fs::read_dir(cases().join("prompts")).unwrap() {
        let file = entry.unwrap().file_name().into_string().unwrap();
        let name = file.strip_suffix(".txt").unwrap_or(&file);
        if !prompts.iter().any(|(n, _)| *n == name) {
            failures.push(format!("  {file}: no prompt by that name any more"));
        }
    }
    for (name, prompt) in prompts {
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

/// Recall (plan-zeca M5): questions about earlier days, asked as a chat turn asks (strict, today's
/// chat left out); the day that answers each must be among the first three passages for at least
/// 0.8 of them. Full-text search alone: the local embedding is an addition when it runs.
#[test]
fn recall_hit_at_3() {
    use vults_store::journal::Entry;
    let case = read("recall.json");
    let dir = std::env::temp_dir().join(format!("vults-evals-recall-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let mut store = vults_store::Store::open(&dir).unwrap();
    let mut add = |day: &str, l: &Value| {
        let f = |i: usize| l[i].as_str().unwrap().to_string();
        store
            .journal_add(&Entry {
                ms: 0,
                day: day.into(),
                kind: f(0),
                agent: f(1),
                project: f(2),
                text: f(3),
                outside: false,
            })
            .unwrap();
    };
    let noise = case["noise"]["lines"].as_array().unwrap();
    let per_day = case["noise"]["per_day"].as_u64().unwrap() as usize;
    for d in case["days"].as_array().unwrap() {
        let day = d["day"].as_str().unwrap();
        for i in 0..per_day {
            add(day, &noise[i % noise.len()]);
        }
        for l in d["lines"].as_array().unwrap() {
            add(day, l);
        }
    }
    let today = case["today"]["day"].as_str().unwrap();
    for l in case["today"]["lines"].as_array().unwrap() {
        add(today, l);
    }
    for d in case["days"].as_array().unwrap() {
        store
            .journal_summary_set(d["day"].as_str().unwrap(), d["summary"].as_str().unwrap(), 0)
            .unwrap();
    }
    let ask = vults_store::recall::Ask {
        vector: None,
        chat_before: Some(today),
        strict: true,
    };
    let questions = case["questions"].as_array().unwrap();
    let mut misses = Vec::new();
    for q in questions {
        let (question, day) = (q[0].as_str().unwrap(), q[1].as_str().unwrap());
        let hits = store.recall(question, ask, 3).unwrap();
        assert!(
            hits.iter().all(|h| h.day != today || h.kind != "chat"),
            "today's chat came back: {question}"
        );
        if !hits.iter().any(|h| h.day == day) {
            let got: Vec<&str> = hits.iter().map(|h| h.day.as_str()).collect();
            misses.push(format!("  {question} (wanted {day}, got {got:?})"));
        }
    }
    let hit = 1.0 - misses.len() as f64 / questions.len() as f64;
    let _ = std::fs::remove_dir_all(&dir);
    eprintln!(
        "recall hit@3 = {hit:.2} on {} questions; missed:\n{}",
        questions.len(),
        misses.join("\n")
    );
    assert!(hit >= 0.8, "recall hit@3 = {hit:.2}, below 0.8");
}
