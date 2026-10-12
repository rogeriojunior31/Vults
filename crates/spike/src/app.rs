//! A fake app on the hook's socket: it logs every event the real `vults-hook` sends, one JSON
//! line each, and answers the ones waiting for a card as told: `allow`, `deny`, or `none` (it
//! closes without a word, so the hook gives up and the agent's own prompt takes over). A question
//! gets its first option for each question asked.

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;
use std::sync::{Arc, Mutex};

use serde_json::{Value, json};

/// Listens at `<runtime>/vults.sock` until killed.
pub fn serve(runtime: &Path, log: &Path, answer: &str) -> std::io::Result<()> {
    let socket = runtime.join("vults.sock");
    let _ = std::fs::remove_file(&socket);
    let listener = UnixListener::bind(&socket)?;
    let log = Arc::new(Mutex::new(
        std::fs::OpenOptions::new().create(true).append(true).open(log)?,
    ));
    let answer: Arc<str> = answer.into();
    for stream in listener.incoming().flatten() {
        let (log, answer) = (Arc::clone(&log), Arc::clone(&answer));
        std::thread::spawn(move || {
            let _ = handle(stream, &log, &answer);
        });
    }
    Ok(())
}

fn handle(stream: UnixStream, log: &Mutex<std::fs::File>, answer: &str) -> std::io::Result<()> {
    let mut line = String::new();
    BufReader::new(stream.try_clone()?).read_line(&mut line)?;
    let Ok(event) = serde_json::from_str::<Value>(&line) else {
        return Ok(());
    };
    if let Ok(mut f) = log.lock() {
        writeln!(f, "{event}")?;
    }
    if event["wants_reply"] != json!(true) || answer == "none" {
        return Ok(());
    }
    let (v, id) = (&event["v"], &event["id"]);
    let mut out = stream;
    writeln!(out, "{}", json!({ "kind": "waiting", "v": v, "id": id }))?;
    let questions = event
        .pointer("/payload/tool_input/questions")
        .and_then(Value::as_array);
    let reply = match questions {
        Some(qs) if event["event"] == "PreToolUse" => {
            let answers: Vec<Value> = qs
                .iter()
                .map(|q| q.pointer("/options/0/label").cloned().unwrap_or(json!("x")))
                .collect();
            json!({ "kind": "answer", "v": v, "id": id, "answers": answers })
        }
        _ => json!({ "kind": "decision", "v": v, "id": id, "decision": answer }),
    };
    writeln!(out, "{reply}")?;
    out.flush()
}
