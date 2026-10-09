//! KDE Plasma: a tiny KWin script, loaded over D-Bus, finds the window by process id, activates
//! it, tells us whether it found one, and is unloaded. KWin sees every window, native Wayland
//! ones included, which no app can raise on its own.

use std::io::Write;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

use tokio::sync::oneshot;

/// Where the script says what it did: our own connection, this object and interface. The macro
/// below needs the interface as a literal; a test keeps it under the app's bundle id.
const REPLY_PATH: &str = "/jump";
const REPLY_INTERFACE: &str = "io.github.rogeriojunior31.vults.Jump";
const KWIN: &str = "org.kde.KWin";
const TIMEOUT: Duration = Duration::from_secs(2);

static COUNTER: AtomicU32 = AtomicU32::new(0);

/// True when KWin said it activated a window of `pids`.
pub fn activate(pids: &[u32], folder: &str) -> bool {
    // A runtime of our own: jump runs on a blocking thread, and zbus's tasks only need to live
    // as long as this call.
    let Ok(rt) = tokio::runtime::Builder::new_current_thread().enable_all().build() else {
        return false;
    };
    rt.block_on(async {
        tokio::time::timeout(TIMEOUT * 2, try_activate(pids, folder))
            .await
            .ok()
            .and_then(Result::ok)
            .unwrap_or(false)
    })
}

/// A unique bus name as the bus hands it out (`:1.42`), nothing else: it is the one string the
/// script holds.
fn is_unique_name(name: &str) -> bool {
    let Some(rest) = name.strip_prefix(':') else {
        return false;
    };
    let mut parts = rest.split('.');
    let ok = |p: Option<&str>| p.is_some_and(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()));
    ok(parts.next()) && ok(parts.next()) && parts.next().is_none()
}

/// The script: the windows of the nearest ancestor that has any, the one whose caption names
/// the folder or else the first, unminimized and activated; then `Activated` or `NotFound` to
/// `reply_to`. Process ids are numbers and the folder travels as UTF-16 codes, so nothing in it
/// can be read as code. Plasma 6 (`windowList`, `activeWindow`) and 5 (`clientList`,
/// `activeClient`).
fn script(pids: &[u32], folder: &str, reply_to: &str) -> Option<String> {
    if !is_unique_name(reply_to) || pids.is_empty() {
        return None;
    }
    let pids = pids.iter().map(u32::to_string).collect::<Vec<_>>().join(", ");
    let folder = folder
        .to_lowercase()
        .encode_utf16()
        .map(|c| c.to_string())
        .collect::<Vec<_>>()
        .join(", ");
    Some(format!(
        r#"(function () {{
    var pids = [{pids}];
    var folder = String.fromCharCode({folder});
    var plasma6 = typeof workspace.windowList === "function";
    var windows = plasma6 ? workspace.windowList() : workspace.clientList();
    var found = null;
    for (var i = 0; i < pids.length && !found; i++) {{
        var first = null;
        for (var j = 0; j < windows.length; j++) {{
            var w = windows[j];
            if (w.pid !== pids[i] || !w.normalWindow) continue;
            if (!first) first = w;
            if (folder && String(w.caption).toLowerCase().indexOf(folder) >= 0) {{ found = w; break; }}
        }}
        if (!found) found = first;
    }}
    if (found) {{
        if (found.minimized) found.minimized = false;
        if (plasma6) workspace.activeWindow = found; else workspace.activeClient = found;
    }}
    callDBus("{reply_to}", "{REPLY_PATH}", "{REPLY_INTERFACE}", found ? "Activated" : "NotFound");
}})();
"#
    ))
}

/// Receives the script's answer once.
struct Reply(Mutex<Option<oneshot::Sender<bool>>>);

impl Reply {
    fn send(&self, found: bool) {
        if let Some(tx) = self.0.lock().ok().and_then(|mut t| t.take()) {
            let _ = tx.send(found);
        }
    }
}

#[zbus::interface(name = "io.github.rogeriojunior31.vults.Jump")]
impl Reply {
    fn activated(&self) {
        self.send(true);
    }

    fn not_found(&self) {
        self.send(false);
    }
}

/// `$XDG_RUNTIME_DIR` when it is ours and private: a script written to a shared folder could be
/// swapped, or a symlink planted there written through.
fn script_dir() -> Option<PathBuf> {
    let dir = PathBuf::from(std::env::var_os("XDG_RUNTIME_DIR")?);
    let meta = std::fs::symlink_metadata(&dir).ok()?;
    let ours = Some(meta.uid()) == super::my_uid();
    (dir.is_absolute() && meta.is_dir() && ours && meta.mode() & 0o077 == 0).then_some(dir)
}

/// Removes the script file whatever happens.
struct ScriptFile(PathBuf);

impl Drop for ScriptFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

fn write_script(dir: &Path, name: &str, text: &str) -> Option<ScriptFile> {
    let path = dir.join(format!("{name}.js"));
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&path)
        .ok()?;
    let file_guard = ScriptFile(path);
    file.write_all(text.as_bytes()).ok()?;
    Some(file_guard)
}

async fn try_activate(pids: &[u32], folder: &str) -> zbus::Result<bool> {
    let Some(dir) = script_dir() else {
        return Ok(false);
    };
    let (tx, rx) = oneshot::channel();
    let conn = zbus::connection::Builder::session()?
        .serve_at(REPLY_PATH, Reply(Mutex::new(Some(tx))))?
        .build()
        .await?;
    let Some(reply_to) = conn.unique_name().map(|n| n.to_string()) else {
        return Ok(false);
    };
    let Some(text) = script(pids, folder, &reply_to) else {
        return Ok(false);
    };
    let name = format!(
        "{}-jump-{}-{}",
        vults_brand::SLUG,
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    );
    let Some(file) = write_script(&dir, &name, &text) else {
        return Ok(false);
    };
    let Some(path) = file.0.to_str() else {
        return Ok(false);
    };

    let scripting = zbus::Proxy::new(&conn, KWIN, "/Scripting", "org.kde.kwin.Scripting").await?;
    let id: i32 = scripting.call("loadScript", &(path, name.as_str())).await?;
    let found = if id < 0 {
        false
    } else {
        // Plasma 6 puts the script at /Scripting/Script<id>, Plasma 5 at /<id>.
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
        // KWin runs it on its own time: wait for its answer.
        ran && matches!(tokio::time::timeout(TIMEOUT, rx).await, Ok(Ok(true)))
    };
    let _: zbus::Result<bool> = scripting.call("unloadScript", &(name.as_str(),)).await;
    drop(file);
    Ok(found)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_reply_interface_is_under_the_bundle_id() {
        assert_eq!(REPLY_INTERFACE, format!("{}.Jump", vults_brand::BUNDLE_ID));
    }

    #[test]
    fn only_a_bus_unique_name_is_put_in_the_script() {
        assert!(is_unique_name(":1.42"));
        assert!(is_unique_name(":12.3456"));
        for bad in [
            "",
            ":",
            ":1",
            ":1.",
            ":.1",
            "1.42",
            ":1.42.3",
            ":1.4a",
            "org.kde.KWin",
            ":1.42\"); x(\"",
        ] {
            assert!(!is_unique_name(bad), "{bad}");
        }
        assert!(script(&[1], "x", "org.kde.KWin").is_none());
        assert!(script(&[], "x", ":1.2").is_none());
    }

    #[test]
    fn the_script_holds_numbers_only() {
        let text = script(&[4242, 17], "Cafe \"); evil(); //", ":1.42").unwrap_or_default();
        assert!(text.contains("var pids = [4242, 17];"));
        // The folder travels as UTF-16 codes, lower-cased: no quote, no word of it.
        assert!(text.contains("String.fromCharCode(99, 97, 102, 101, 32, 34, 41, 59, 32"));
        assert!(!text.contains("evil"));
        // Apart from the code itself, every quoted string is one we wrote.
        let quoted: Vec<&str> = text.split('"').skip(1).step_by(2).collect();
        assert_eq!(
            quoted,
            [
                "function",
                ":1.42",
                REPLY_PATH,
                REPLY_INTERFACE,
                "Activated",
                "NotFound"
            ]
        );
        let empty = script(&[1], "", ":1.2").unwrap_or_default();
        assert!(empty.contains("String.fromCharCode();"));
        assert!(empty.contains("workspace.activeWindow = found"));
        assert!(empty.contains("workspace.activeClient = found"));
    }

    #[test]
    fn the_script_file_is_new_private_and_removed() {
        let dir = std::env::temp_dir().join(format!("vults-kwin-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap_or_default();
        let file = write_script(&dir, "s", "x").map(|f| f.0.clone());
        let path = file.clone().unwrap_or_default();
        // Written once and gone: the guard was dropped right away.
        assert!(file.is_some() && !path.exists());
        // An existing file (or a planted symlink) is never written through.
        std::fs::write(dir.join("t.js"), "old").unwrap_or_default();
        assert!(write_script(&dir, "t", "new").is_none());
        assert_eq!(
            std::fs::read_to_string(dir.join("t.js")).unwrap_or_default(),
            "old"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
