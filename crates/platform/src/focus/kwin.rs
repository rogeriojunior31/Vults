use super::Window;
use crate::jump::kwin::{is_unique_name, script_dir, write_script};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::oneshot;

const PATH: &str = "/focus";
const INTERFACE: &str = "io.github.rogeriojunior31.vults.Focus";
const KWIN: &str = "org.kde.KWin";
const DEADLINE: Duration = Duration::from_millis(300);
static COUNTER: AtomicU32 = AtomicU32::new(0);

pub(super) fn read() -> Option<Window> {
    let started = Instant::now();
    let deadline = SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_millis() + DEADLINE.as_millis();
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .ok()?;
    rt.block_on(async {
        let dir = script_dir()?;
        let (tx, rx) = oneshot::channel();
        let conn = tokio::time::timeout(DEADLINE, async {
            let conn = zbus::Connection::session().await?;
            let bus = zbus::fdo::DBusProxy::new(&conn).await?;
            let owner = bus.get_name_owner(KWIN.try_into()?).await?.to_string();
            conn.object_server()
                .at(
                    PATH,
                    Reply {
                        owner,
                        send: Mutex::new(Some(tx)),
                    },
                )
                .await?;
            Ok::<_, zbus::Error>(conn)
        })
        .await
        .ok()?
        .ok()?;
        let name = format!(
            "{}-focus-{}-{}",
            vults_brand::SLUG,
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        );
        let text = script(conn.unique_name()?.as_str(), deadline)?;
        let file = write_script(&dir, &name, &text)?;
        let scripting = zbus::Proxy::new(&conn, KWIN, "/Scripting", "org.kde.kwin.Scripting")
            .await
            .ok()?;
        let result = tokio::time::timeout(DEADLINE.saturating_sub(started.elapsed()), async {
            let id: i32 = scripting
                .call("loadScript", &(file.0.to_str()?, name.as_str()))
                .await
                .ok()?;
            if id < 0 {
                return None;
            }
            let mut ran = false;
            for at in [format!("/Scripting/Script{id}"), format!("/{id}")] {
                let Ok(script) = zbus::Proxy::new(&conn, KWIN, at, "org.kde.kwin.Script").await else {
                    continue;
                };
                if script.call::<_, _, ()>("run", &()).await.is_ok() {
                    ran = true;
                    break;
                }
            }
            if !ran {
                return None;
            }
            rx.await.ok().flatten()
        })
        .await
        .ok()
        .flatten();
        // Cleanup has its own deadline, including when loading or running timed out.
        let _ = tokio::time::timeout(
            Duration::from_millis(500),
            scripting.call::<_, _, bool>("unloadScript", &(name.as_str(),)),
        )
        .await;
        drop(file);
        result
    })
}

fn script(reply: &str, deadline: u128) -> Option<String> {
    if !is_unique_name(reply) {
        return None;
    }
    Some(format!(
        r#"(function () {{
    var w = typeof workspace.windowList === "function" ? workspace.activeWindow : workspace.activeClient;
    var data = [];
    if (Date.now() <= {deadline} && w && !w.deleted && (w.normalWindow || w.dialog)) {{
        data = [String(w.resourceClass).slice(0, 256), String(w.caption).slice(0, 4096), String(w.pid), String(w.internalId)];
    }}
    callDBus("{reply}", "{PATH}", "{INTERFACE}", "Observed", JSON.stringify(data));
}})();
"#
    ))
}

struct Reply {
    owner: String,
    send: Mutex<Option<oneshot::Sender<Option<Window>>>>,
}

#[zbus::interface(name = "io.github.rogeriojunior31.vults.Focus")]
impl Reply {
    fn observed(&self, data: &str, #[zbus(header)] header: zbus::message::Header<'_>) {
        self.receive(header.sender().map(|s| s.as_str()), data);
    }
}

impl Reply {
    fn receive(&self, sender: Option<&str>, data: &str) {
        if sender != Some(self.owner.as_str()) {
            return;
        }
        if let Some(send) = self.send.lock().ok().and_then(|mut s| s.take()) {
            let _ = send.send(parse(data));
        }
    }
}

pub(crate) fn parse(data: &str) -> Option<Window> {
    if data.len() > 32_768 {
        return None;
    }
    let data: serde_json::Value = serde_json::from_str(data).ok()?;
    let data = data.as_array()?;
    if data.len() != 4 {
        return None;
    }
    let class = data[0].as_str()?;
    let title = data[1].as_str()?;
    let identity = data[3].as_str()?;
    if class.is_empty()
        || class.len() > 1024
        || title.len() > 16_384
        || identity.is_empty()
        || identity.len() > 128
        || identity == "undefined"
    {
        return None;
    }
    Some(Window {
        class: class.into(),
        title: title.into(),
        pid: data[2].as_str()?.parse().ok().filter(|p| *p > 1),
        identity: format!("kwin:{identity}"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn script_is_read_only_and_only_interpolates_checked_bus_names_and_numbers() {
        assert!(script("\"); evil(); //", 123).is_none());
        let code = script(":1.42", 123).expect("script");
        assert!(code.contains("Date.now() <= 123"));
        assert!(!code.contains("workspace.activeWindow ="));
        assert!(!code.contains("workspace.activeClient ="));
        assert!(!code.contains("activateWindow"));
        let quoted: Vec<_> = code.split('"').skip(1).step_by(2).collect();
        assert_eq!(quoted, ["function", ":1.42", PATH, INTERFACE, "Observed"]);
        assert_eq!(INTERFACE, format!("{}.Focus", vults_brand::BUNDLE_ID));
    }
    #[test]
    fn only_the_kwin_owner_can_answer_and_only_once() {
        let (send, mut receive) = oneshot::channel();
        let reply = Reply {
            owner: ":1.42".into(),
            send: Mutex::new(Some(send)),
        };
        let data = r#"["konsole","t","42","id"]"#;
        reply.receive(Some(":1.43"), data);
        reply.receive(None, data);
        assert!(matches!(
            receive.try_recv(),
            Err(oneshot::error::TryRecvError::Empty)
        ));
        reply.receive(Some(":1.42"), data);
        assert_eq!(
            receive.try_recv().expect("KWin reply").expect("window").pid,
            Some(42)
        );
        reply.receive(Some(":1.42"), data);
        assert!(reply.send.lock().expect("sender").is_none());
    }
    #[test]
    fn replies_are_bounded_data_not_code_and_missing_pids_are_unknown() {
        let w = parse(r#"["konsole","x\"); evil(); //","42","abc"]"#).expect("window");
        assert_eq!(w.pid, Some(42));
        assert_eq!(w.identity, "kwin:abc");
        assert!(parse("[]").is_none());
        assert!(parse("{}").is_none());
        assert!(parse(r#"["x","t","42","undefined"]"#).is_none());
        assert_eq!(parse(r#"["x","t","0","id"]"#).expect("window").pid, None);
        assert!(parse(&"x".repeat(32_769)).is_none());
    }
}
