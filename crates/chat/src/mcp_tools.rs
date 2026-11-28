//! A same-user, per-turn MCP relay for host-owned specialist tools: a Unix socket, or on Windows a
//! named pipe whose name carries the user's SID and whose both ends check the other runs as us.
use crate::{Approver, Delta, tools};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
#[cfg(unix)]
use tokio::net::UnixListener;
#[cfg(windows)]
use tokio::net::windows::named_pipe::{NamedPipeServer, ServerOptions};
const LIMIT: u64 = 1024 * 1024;

/// An instance of the tools' pipe that only this user (and administrators, SYSTEM) may open.
#[cfg(windows)]
fn private_pipe(path: &Path, first: bool) -> std::io::Result<NamedPipeServer> {
    let mut options = ServerOptions::new();
    options.first_pipe_instance(first).reject_remote_clients(true);
    // SAFETY: the attributes, when given, outlive the call.
    vults_peer::private_pipe(|attributes| unsafe {
        options.create_with_security_attributes_raw(path, attributes)
    })
}

pub(crate) struct Service {
    #[cfg(unix)]
    listener: UnixListener,
    /// The first pipe instance, taken by `serve`; later ones are made as clients connect.
    #[cfg(windows)]
    listener: std::sync::Mutex<Option<NamedPipeServer>>,
    path: PathBuf,
    pub config: PathBuf,
}
impl Drop for Service {
    fn drop(&mut self) {
        #[cfg(unix)]
        let _ = std::fs::remove_file(&self.path);
        let _ = std::fs::remove_file(&self.config);
    }
}
impl Service {
    pub(crate) fn bind() -> Result<Self, String> {
        use std::io::Write;
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let name = format!(
            "{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        );
        #[cfg(unix)]
        let (listener, path, root) = {
            use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
            let root = std::env::var_os("XDG_RUNTIME_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(std::env::temp_dir)
                .join(format!("vults-tools-{}", vults_peer::current_uid()));
            std::fs::DirBuilder::new()
                .recursive(true)
                .mode(0o700)
                .create(&root)
                .map_err(|_| "Could not prepare the specialist tool service.")?;
            if !vults_peer::is_private_dir(&root) {
                return Err("The specialist tool service directory is not private.".into());
            }
            let path = root.join(format!("{name}.sock"));
            let listener =
                UnixListener::bind(&path).map_err(|_| "Could not start the specialist tool service.")?;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))
                .map_err(|_| "Could not protect the specialist tool service.")?;
            (listener, path, root)
        };
        #[cfg(windows)]
        let (listener, path, root) = {
            let sid = vults_peer::current_user_sid().ok_or("Could not start the specialist tool service.")?;
            let path = PathBuf::from(format!("{}-tools-{name}", vults_protocol::pipe_name(&sid)));
            // first_pipe_instance: a name another process made first is never served on.
            let listener =
                private_pipe(&path, true).map_err(|_| "Could not start the specialist tool service.")?;
            // The user's own temp folder, which only they (and administrators) can read.
            let root = std::env::temp_dir().join(format!("vults-tools-{sid}"));
            std::fs::create_dir_all(&root).map_err(|_| "Could not prepare the specialist tool service.")?;
            (std::sync::Mutex::new(Some(listener)), path, root)
        };
        let config = root.join(format!("{name}.json"));
        let service = Self {
            listener,
            path,
            config,
        };
        let value = json!({"mcpServers":{"vults-specialist":{"command":std::env::current_exe().map_err(|_|"The specialist tool executable is unavailable.")?,"args":["--specialist-mcp",service.path]}}});
        let mut options = std::fs::OpenOptions::new();
        options.create_new(true).write(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options
            .open(&service.config)
            .map_err(|_| "Could not prepare specialist tools.")?;
        file.write_all(value.to_string().as_bytes())
            .map_err(|_| "Could not write specialist tools.")?;
        Ok(service)
    }
    pub(crate) async fn serve(
        &self,
        tools: &tools::Tools,
        approver: &dyn Approver,
        outside: &std::sync::Mutex<Option<String>>,
        out: &tokio::sync::mpsc::Sender<Delta>,
    ) {
        let mut steps = 0usize;
        #[cfg(windows)]
        let Some(mut server) = self.listener.lock().ok().and_then(|mut first| first.take()) else {
            return;
        };
        loop {
            #[cfg(unix)]
            let stream = {
                let Ok((stream, _)) = self.listener.accept().await else {
                    return;
                };
                if !stream
                    .peer_cred()
                    .is_ok_and(|c| c.uid() == vults_peer::current_uid())
                {
                    continue;
                }
                stream
            };
            #[cfg(windows)]
            let stream = {
                if server.connect().await.is_err() {
                    // A client that came and went fails this instance, never the turn's tools.
                    let Ok(fresh) = private_pipe(&self.path, false) else {
                        return;
                    };
                    server = fresh;
                    continue;
                }
                // The next instance before this client is served, so a second call finds one.
                let Ok(next) = private_pipe(&self.path, false) else {
                    return;
                };
                let stream = std::mem::replace(&mut server, next);
                if !vults_peer::pipe_client_is_same_user(&stream) {
                    continue;
                }
                stream
            };
            let mut reader = BufReader::new(stream.take(LIMIT + 1));
            let mut line = Vec::new();
            let read = tokio::time::timeout(
                std::time::Duration::from_secs(5),
                reader.read_until(b'\n', &mut line),
            )
            .await;
            if !matches!(read, Ok(Ok(_))) || line.len() as u64 > LIMIT {
                continue;
            }
            let Ok(request) = serde_json::from_slice::<Value>(&line) else {
                continue;
            };
            let response = if request["list"] == true {
                json!({"tools":tools.iter().map(|tool|json!({"name":tool.name(),"description":tool.description(),"inputSchema":tool.schema()})).collect::<Vec<_>>()})
            } else {
                steps += 1;
                let name = request["name"].as_str().unwrap_or_default();
                if steps > tools::MAX_STEPS {
                    json!({"text":"The specialist tool limit for this turn was reached.","error":true})
                } else {
                    if tools::outside(tools, name)
                        && let Ok(mut cause) = outside.lock()
                    {
                        cause.get_or_insert_with(|| name.into());
                    }
                    let result = tools::run_with_approval(
                        tools,
                        name,
                        "specialist",
                        &request["arguments"],
                        approver,
                        outside,
                        out,
                    );
                    let (text, error) = tokio::select! {
                        biased;
                        _ = reader.read_u8() => continue,
                        result = result => result,
                    };
                    json!({"text":text,"error":error})
                }
            };
            let mut bytes = response.to_string().into_bytes();
            bytes.push(b'\n');
            let _ = reader.get_mut().get_mut().write_all(&bytes).await;
        }
    }
}

#[cfg(unix)]
fn connect(path: &Path) -> Result<std::os::unix::net::UnixStream, String> {
    let stream = std::os::unix::net::UnixStream::connect(path)
        .map_err(|_| "The specialist turn is no longer active.")?;
    stream
        .set_read_timeout(Some(crate::DECISION_TIMEOUT + std::time::Duration::from_secs(30)))
        .map_err(|_| "Could not wait for specialist tools.")?;
    stream
        .set_write_timeout(Some(std::time::Duration::from_secs(5)))
        .map_err(|_| "Could not contact specialist tools.")?;
    Ok(stream)
}

/// A pipe has no read timeout: the wait ends when the turn's service closes its end.
#[cfg(windows)]
fn connect(path: &Path) -> Result<std::fs::File, String> {
    use std::os::windows::fs::OpenOptionsExt;
    const ERROR_PIPE_BUSY: i32 = 231;
    /// SECURITY_IDENTIFICATION: a server that is not ours can never act as this user.
    const IDENTIFICATION: u32 = 0x0001_0000;
    for _ in 0..50 {
        match std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .security_qos_flags(IDENTIFICATION)
            .open(path)
        {
            Ok(pipe) if vults_peer::pipe_server_is_same_user(&pipe) => return Ok(pipe),
            Ok(_) => return Err("The specialist tool service is not this user's.".into()),
            // Between two calls the next instance may not be open yet.
            Err(e) if e.raw_os_error() == Some(ERROR_PIPE_BUSY) => {
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            Err(_) => break,
        }
    }
    Err("The specialist turn is no longer active.".into())
}

fn relay(path: &Path, request: &Value) -> Result<Value, String> {
    use std::io::{BufRead, Read, Write};
    let mut stream = connect(path)?;
    let mut request = request.to_string();
    request.push('\n');
    stream
        .write_all(request.as_bytes())
        .map_err(|_| "Could not contact specialist tools.")?;
    let mut result = Vec::new();
    std::io::BufReader::new(stream)
        .take(LIMIT + 1)
        .read_until(b'\n', &mut result)
        .map_err(|_| "The specialist tool service did not answer.")?;
    if result.len() as u64 > LIMIT {
        return Err("The specialist tool result was too large.".into());
    }
    serde_json::from_slice(&result)
        .map_err(|_| "The specialist tool service returned an invalid result.".into())
}
/// Runs only in the CLI's MCP child, not in the desktop runtime.
pub fn stdio(path: &Path) {
    use std::io::{BufRead, Read, Write};
    let mut input = std::io::stdin().lock();
    let mut output = std::io::stdout().lock();
    loop {
        let mut line = Vec::new();
        if !matches!((&mut input).take(LIMIT+1).read_until(b'\n',&mut line),Ok(n) if n>0 && n as u64<=LIMIT) {
            return;
        }
        let Ok(request) = serde_json::from_slice::<Value>(&line) else {
            continue;
        };
        let Some(id) = request.get("id") else {
            continue;
        };
        let result = match request["method"].as_str() {
            Some("initialize") => {
                json!({"protocolVersion":"2025-03-26","capabilities":{"tools":{}},"serverInfo":{"name":"vults-specialist","version":env!("CARGO_PKG_VERSION")}})
            }
            Some("tools/list") => relay(path, &json!({"list":true})).unwrap_or_else(|_| json!({"tools":[]})),
            Some("tools/call") => {
                let result = relay(
                    path,
                    &json!({"name":request["params"]["name"],"arguments":request["params"]["arguments"]}),
                );
                match result {
                    Ok(value) => {
                        json!({"content":[{"type":"text","text":value["text"]}],"isError":value["error"]})
                    }
                    Err(error) => json!({"content":[{"type":"text","text":error}],"isError":true}),
                }
            }
            _ => {
                let value =
                    json!({"jsonrpc":"2.0","id":id,"error":{"code":-32601,"message":"Unknown method."}});
                if writeln!(output, "{value}").and_then(|_| output.flush()).is_err() {
                    return;
                }
                continue;
            }
        };
        if writeln!(output, "{}", json!({"jsonrpc":"2.0","id":id,"result":result}))
            .and_then(|_| output.flush())
            .is_err()
        {
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    use vults_agents::driver::BoxFuture;
    #[derive(Debug)]
    struct Write(Arc<AtomicUsize>);
    impl tools::Tool for Write {
        fn name(&self) -> &str {
            "github_issue_close"
        }
        fn description(&self) -> &str {
            "Close one issue after human approval."
        }
        fn schema(&self) -> Value {
            json!({"type":"object"})
        }
        fn reads_outside(&self) -> bool {
            true
        }
        fn call<'a>(&'a self, _: &'a Value) -> BoxFuture<'a, Result<String, String>> {
            Box::pin(async { Err("Approval required.".into()) })
        }
        fn approval<'a>(
            &'a self,
            _: &'a Value,
        ) -> Option<BoxFuture<'a, Result<tools::ToolApproval, String>>> {
            Some(Box::pin(async {
                Ok(tools::ToolApproval {
                    target: "org/repo #7".into(),
                    description: "Close the issue".into(),
                    full: "Close issue 7".into(),
                    context: json!({"exact":7}),
                })
            }))
        }
        fn call_approved<'a>(
            &'a self,
            _: &'a Value,
            context: &'a Value,
        ) -> BoxFuture<'a, Result<String, String>> {
            Box::pin(async move {
                assert_eq!(context["exact"], 7);
                self.0.fetch_add(1, Ordering::SeqCst);
                Ok("Closed".into())
            })
        }
    }
    struct Human(crate::Answer);
    impl Approver for Human {
        fn wait(&self, _: &str, _: &str, _: &str) -> tokio::sync::oneshot::Receiver<crate::Answer> {
            let (tx, rx) = tokio::sync::oneshot::channel();
            let _ = tx.send(self.0.clone());
            rx
        }
    }
    #[tokio::test]
    async fn per_turn_tools_require_exact_human_approval_and_end_with_the_turn() {
        for (answer, count) in [(crate::Answer::Deny, 0), (crate::Answer::Allow, 1)] {
            let service = Service::bind().unwrap();
            let socket = service.path.clone();
            let config = service.config.clone();
            let calls = Arc::new(AtomicUsize::new(0));
            let tools: tools::Tools = vec![Arc::new(Write(calls.clone()))];
            let outside = std::sync::Mutex::new(None);
            let (tx, mut rx) = tokio::sync::mpsc::channel(8);
            let approver = Human(answer);
            let client = async {
                let path = socket.clone();
                let listed = tokio::task::spawn_blocking(move || relay(&path, &json!({"list":true})))
                    .await
                    .unwrap()
                    .unwrap();
                assert_eq!(listed["tools"][0]["name"], "github_issue_close");
                let path = socket.clone();
                tokio::task::spawn_blocking(move||relay(&path,&json!({"name":"github_issue_close","arguments":{"repository":"org/repo","number":7}}))).await.unwrap().unwrap()
            };
            let result = tokio::select! {result=client=>result,()=service.serve(&tools,&approver,&outside,&tx)=>panic!("service stopped")};
            assert_eq!(result["error"], count == 0);
            assert_eq!(calls.load(Ordering::SeqCst), count);
            assert_eq!(outside.lock().unwrap().as_deref(), Some("github_issue_close"));
            assert!(matches!(rx.try_recv().unwrap(), Delta::Permission { .. }));
            drop(service);
            // A pipe has no file to look for: the relay below finds it gone.
            #[cfg(unix)]
            assert!(!socket.exists());
            assert!(!config.exists());
            // The runtime closes a dropped pipe once it sees its cancelled connect.
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            let gone = tokio::task::spawn_blocking(move || relay(&socket, &json!({"list":true})));
            assert!(gone.await.unwrap().is_err());
        }
    }
    #[tokio::test]
    async fn disconnect_cancels_a_pending_write_before_a_late_allow() {
        struct Pending(std::sync::Mutex<Option<tokio::sync::oneshot::Sender<crate::Answer>>>);
        impl Approver for Pending {
            fn wait(&self, _: &str, _: &str, _: &str) -> tokio::sync::oneshot::Receiver<crate::Answer> {
                let (tx, rx) = tokio::sync::oneshot::channel();
                *self.0.lock().unwrap() = Some(tx);
                rx
            }
        }
        let service = Service::bind().unwrap();
        let calls = Arc::new(AtomicUsize::new(0));
        let tools: tools::Tools = vec![Arc::new(Write(calls.clone()))];
        let outside = std::sync::Mutex::new(None);
        let (tx, mut rx) = tokio::sync::mpsc::channel(8);
        let approver = Pending(std::sync::Mutex::new(None));
        let client = async {
            #[cfg(unix)]
            let mut stream = tokio::net::UnixStream::connect(&service.path).await.unwrap();
            #[cfg(windows)]
            let mut stream = tokio::net::windows::named_pipe::ClientOptions::new()
                .open(&service.path)
                .unwrap();
            stream.write_all(b"{\"name\":\"github_issue_close\",\"arguments\":{\"repository\":\"org/repo\",\"number\":7}}\n").await.unwrap();
            assert!(matches!(rx.recv().await, Some(Delta::Permission { .. })));
            drop(stream);
            for _ in 0..100 {
                if approver.0.lock().unwrap().as_ref().unwrap().is_closed() {
                    break;
                }
                tokio::task::yield_now().await;
            }
            assert!(
                approver
                    .0
                    .lock()
                    .unwrap()
                    .take()
                    .unwrap()
                    .send(crate::Answer::Allow)
                    .is_err()
            );
            assert_eq!(calls.load(Ordering::SeqCst), 0);
        };
        tokio::select! {()=client=>{},()=service.serve(&tools,&approver,&outside,&tx)=>panic!("service stopped")};
    }
}
