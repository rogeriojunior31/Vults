//! Keep stdin responsive while a human edits a text question.
use super::*;
use std::collections::BTreeSet;
use std::io::{BufRead, Read};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
    mpsc,
};
use std::time::Duration;

struct Pending {
    id: Value,
    cancel: Arc<AtomicBool>,
    result: mpsc::Receiver<Result<crate::questions::transport::RelayReply, String>>,
    suppressed: bool,
}
impl Drop for Pending {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Release);
    }
}
fn result(id: Value, reply: Result<String, String>) -> Value {
    let (text, error) = match reply {
        Ok(text) => (text, false),
        Err(error) => (error, true),
    };
    json!({"jsonrpc":"2.0","id":id,"result":{"content":[{"type":"text","text":text}],"isError":error}})
}
fn error(id: Value, code: i64, message: &str) -> Value {
    json!({"jsonrpc":"2.0","id":id,"error":{"code":code,"message":message}})
}
pub(crate) fn read(input: impl BufRead, tx: mpsc::SyncSender<Result<Value, ()>>) {
    let mut input = input;
    loop {
        let mut line = Vec::new();
        let n = (&mut input).take(MAX_LINE).read_until(b'\n', &mut line);
        if matches!(n, Ok(0) | Err(_)) {
            break;
        }
        if !line.ends_with(b"\n") && line.len() as u64 >= MAX_LINE {
            // Drain with a bounded buffer, including a malicious line without a delimiter.
            loop {
                let Ok(buf) = input.fill_buf() else { return };
                let len = buf.iter().position(|b| *b == b'\n').map(|i| i + 1);
                let n = len.unwrap_or(buf.len());
                if n == 0 {
                    return;
                }
                input.consume(n);
                if len.is_some() {
                    break;
                }
            }
            continue;
        }
        if line.iter().all(u8::is_ascii_whitespace) {
            continue;
        }
        if tx.send(serde_json::from_slice(&line).map_err(|_| ())).is_err() {
            break;
        }
    }
}

pub(super) fn run() {
    let (tx, rx) = mpsc::sync_channel(16);
    std::thread::spawn(move || read(std::io::stdin().lock(), tx));
    let folder = std::env::current_dir().unwrap_or_default();
    serve(
        rx,
        std::io::stdout().lock(),
        &folder,
        crate::questions::transport::path().ok(),
        questions_on(),
    );
}
pub(crate) fn serve(
    rx: mpsc::Receiver<Result<Value, ()>>,
    mut out: impl Write,
    folder: &Path,
    path: Option<PathBuf>,
    enabled: bool,
) {
    let mut pending: Option<Pending> = None;
    let mut used = BTreeSet::new();
    let mut initialized = false;
    let mut ready = false;
    let write = |out: &mut dyn Write, value: Value| writeln!(out, "{value}").and_then(|()| out.flush());
    loop {
        if let Some(work) = &pending {
            match work.result.try_recv() {
                Ok(reply) => {
                    if !work.suppressed {
                        match reply {
                            Ok(mut reply) => {
                                if write(
                                    &mut out,
                                    result(work.id.clone(), Ok(std::mem::take(&mut reply.text))),
                                )
                                .is_err()
                                {
                                    break;
                                }
                                let _ = reply.acknowledge();
                            }
                            Err(error) => {
                                if write(&mut out, result(work.id.clone(), Err(error))).is_err() {
                                    break;
                                }
                            }
                        }
                    }
                    pending = None;
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    if !work.suppressed
                        && write(
                            &mut out,
                            result(
                                work.id.clone(),
                                Err("The question connection ended. Ask a new question.".into()),
                            ),
                        )
                        .is_err()
                    {
                        break;
                    }
                    pending = None;
                }
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }
        let msg = match rx.recv_timeout(Duration::from_millis(20)) {
            Ok(Ok(msg)) => msg,
            Ok(Err(())) => {
                if write(&mut out, error(Value::Null, -32700, "parse error")).is_err() {
                    break;
                }
                continue;
            }
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        };
        if msg["method"] == "notifications/cancelled" && msg.get("id").is_none() {
            if let Some(work) = &mut pending
                && msg["params"]["requestId"] == work.id
            {
                work.suppressed = true;
                work.cancel.store(true, Ordering::Release);
            }
            continue;
        }
        if msg["method"] == "initialize" && msg.get("id").is_some() {
            initialized = true;
        }
        if msg["method"] == "notifications/initialized" && msg.get("id").is_none() && initialized {
            ready = true;
        }
        let ask = msg["method"] == "tools/call" && msg["params"]["name"] == "ask_user";
        if ask {
            let Some(id) = msg.get("id").filter(|id| id.is_string() || id.is_i64()).cloned() else {
                continue;
            };
            let key = id.to_string();
            let args = &msg["params"]["arguments"];
            let question = args["question"]
                .as_str()
                .filter(|q| !q.trim().is_empty() && q.len() <= 4096);
            let valid = args.as_object().is_some_and(|args| args.len() == 1)
                && question.is_some()
                && key.len() <= 128;
            let failure = if !enabled || !ready {
                Some("Text questions are unavailable for this MCP client.")
            } else if !valid {
                Some("Give only a nonempty question of at most 4096 bytes.")
            } else if pending.is_some() {
                Some("A question is already pending. Wait for its reply or cancel it.")
            } else if used.contains(&key) {
                Some("This request id was already consumed. Use a new request id.")
            } else if used.len() >= 256 {
                Some("This connection reached its question limit. Reconnect before asking again.")
            } else {
                None
            };
            if let Some(message) = failure {
                if write(&mut out, result(id, Err(message.into()))).is_err() {
                    break;
                }
                continue;
            }
            let Some(path) = path.clone() else {
                if write(&mut out, result(id, Err(NOT_RUNNING.into()))).is_err() {
                    break;
                }
                continue;
            };
            used.insert(key.clone());
            let question = question.unwrap_or_default().to_owned();
            let cancel = Arc::new(AtomicBool::new(false));
            let cancelled = cancel.clone();
            let (tx, result) = mpsc::sync_channel(1);
            std::thread::spawn(move || {
                let _ = tx.send(crate::questions::transport::ask(&path, key, question, cancelled));
            });
            pending = Some(Pending {
                id,
                cancel,
                result,
                suppressed: false,
            });
            continue;
        }
        if let Some(reply) = answer(&msg, folder)
            && write(&mut out, reply).is_err()
        {
            break;
        }
    }
}
