//! A read-only focus epoch running before key-down, with a compositor checkpoint before paste.
use crate::focus::Window;
use crate::jump::kwin::{is_unique_name, script_dir, write_script};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};

const PATH: &str = "/dictationFocus";
const INTERFACE: &str = "io.github.rogeriojunior31.vults.DictationFocus";
const KWIN: &str = "org.kde.KWin";

#[derive(Clone, Default)]
pub(super) struct Snapshot {
    pub sequence: u64,
    pub window: Option<Window>,
}

#[derive(Default)]
struct Data {
    snapshot: Snapshot,
    pending: Option<(u64, oneshot::Sender<Snapshot>)>,
    nonce: u64,
}

pub(super) struct Watch {
    data: Arc<Mutex<Data>>,
    next: mpsc::UnboundedSender<u64>,
    conn: zbus::Connection,
    name: String,
    _file: crate::jump::kwin::ScriptFile,
}

struct Reply {
    data: Arc<Mutex<Data>>,
    next: tokio::sync::Mutex<mpsc::UnboundedReceiver<u64>>,
    owner: String,
}

#[zbus::interface(name = "io.github.rogeriojunior31.vults.DictationFocus")]
impl Reply {
    async fn next(&self, #[zbus(header)] header: zbus::message::Header<'_>) -> String {
        if header.sender().map(|s| s.as_str()) != Some(self.owner.as_str()) {
            return "0".into();
        }
        self.next.lock().await.recv().await.unwrap_or(0).to_string()
    }
    fn observed(&self, payload: &str, #[zbus(header)] header: zbus::message::Header<'_>) {
        if header.sender().map(|s| s.as_str()) != Some(self.owner.as_str()) {
            return;
        }
        if payload.len() > 32_768 {
            return;
        }
        let Ok(payload) = serde_json::from_str::<serde_json::Value>(payload) else {
            return;
        };
        let Some(parts) = payload.as_array().filter(|p| p.len() == 3) else {
            return;
        };
        let Some(sequence) = parts[0].as_u64() else {
            return;
        };
        let Some(nonce) = parts[1].as_str().and_then(|s| s.parse::<u64>().ok()) else {
            return;
        };
        let window = crate::focus::kwin::parse(&parts[2].to_string());
        let Ok(mut data) = self.data.lock() else {
            return;
        };
        if sequence < data.snapshot.sequence {
            return;
        }
        data.snapshot = Snapshot { sequence, window };
        if data
            .pending
            .as_ref()
            .is_some_and(|(expected, _)| *expected == nonce)
            && let Some((_, send)) = data.pending.take()
        {
            let _ = send.send(data.snapshot.clone());
        }
    }
}

impl Watch {
    pub async fn start() -> Option<Self> {
        let conn = zbus::Connection::session().await.ok()?;
        let bus = zbus::fdo::DBusProxy::new(&conn).await.ok()?;
        let owner = bus.get_name_owner(KWIN.try_into().ok()?).await.ok()?.to_string();
        let data = Arc::new(Mutex::new(Data::default()));
        let (next, receive) = mpsc::unbounded_channel();
        conn.object_server()
            .at(
                PATH,
                Reply {
                    data: data.clone(),
                    next: tokio::sync::Mutex::new(receive),
                    owner,
                },
            )
            .await
            .ok()?;
        let code = script(conn.unique_name()?.as_str())?;
        let name = format!("{}-dictation-focus-{}", vults_brand::SLUG, std::process::id());
        let file = write_script(&script_dir()?, &name, &code)?;
        let watch = Self {
            data,
            next,
            conn,
            name,
            _file: file,
        };
        let loaded = async {
            let proxy = zbus::Proxy::new(&watch.conn, KWIN, "/Scripting", "org.kde.kwin.Scripting")
                .await
                .ok()?;
            let id: i32 = proxy
                .call("loadScript", &(watch._file.0.to_str()?, watch.name.as_str()))
                .await
                .ok()?;
            if id < 0 {
                return None;
            }
            for path in [format!("/Scripting/Script{id}"), format!("/{id}")] {
                let script = zbus::Proxy::new(&watch.conn, KWIN, path, "org.kde.kwin.Script")
                    .await
                    .ok()?;
                if script.call::<_, _, ()>("run", &()).await.is_ok() {
                    return Some(());
                }
            }
            None
        };
        if tokio::time::timeout(Duration::from_secs(2), loaded)
            .await
            .ok()
            .flatten()
            .is_none()
            || watch.checkpoint().await.is_none()
        {
            watch.close().await;
            return None;
        }
        Some(watch)
    }

    pub fn snapshot(&self) -> Snapshot {
        self.data.lock().map(|d| d.snapshot.clone()).unwrap_or_default()
    }

    pub async fn checkpoint(&self) -> Option<Snapshot> {
        let (send, receive) = oneshot::channel();
        let nonce = {
            let mut data = self.data.lock().ok()?;
            data.nonce += 1;
            let nonce = data.nonce;
            data.pending = Some((nonce, send));
            nonce
        };
        self.next.send(nonce).ok()?;
        tokio::time::timeout(Duration::from_millis(300), receive)
            .await
            .ok()?
            .ok()
    }

    pub async fn close(&self) {
        let _ = self.next.send(0);
        let _ = tokio::time::timeout(Duration::from_millis(500), async {
            let proxy = zbus::Proxy::new(&self.conn, KWIN, "/Scripting", "org.kde.kwin.Scripting").await?;
            proxy
                .call::<_, _, bool>("unloadScript", &(self.name.as_str(),))
                .await
        })
        .await;
    }
}

fn script(reply: &str) -> Option<String> {
    if !is_unique_name(reply) {
        return None;
    }
    Some(format!(
        r#"(function () {{
    var sequence = 1;
    function report(nonce) {{
        var w = workspace.activeWindow;
        var data = w && !w.deleted && w.normalWindow ? [String(w.resourceClass).slice(0, 256), "", String(w.pid), String(w.internalId)] : [];
        callDBus("{reply}", "{PATH}", "{INTERFACE}", "Observed", JSON.stringify([sequence, String(nonce), data]));
    }}
    function changed() {{ sequence++; report(0); }}
    function next() {{
        callDBus("{reply}", "{PATH}", "{INTERFACE}", "Next", function (nonce) {{
            if (!nonce || nonce === "0") {{ workspace.windowActivated.disconnect(changed); return; }}
            report(nonce); next();
        }});
    }}
    workspace.windowActivated.connect(changed);
    report(0); next();
}})();
"#
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn monitor_is_read_only_and_callbacks_use_checked_bus_names() {
        assert!(script("\"); evil(); //").is_none());
        let code = script(":1.42").expect("script");
        assert!(code.contains("windowActivated.connect(changed)"));
        assert!(code.contains("windowActivated.disconnect(changed)"));
        assert!(!code.contains("activeWindow ="));
        assert!(!code.contains("caption"), "insertion does not need window titles");
    }
}
