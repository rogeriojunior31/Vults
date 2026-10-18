//! Global shortcuts on Wayland through the xdg-desktop-portal `GlobalShortcuts` interface: the
//! desktop (KDE asks the user once, and lets them change the keys) tells us when they are pressed.
//! No keyboard grab, nothing taken from the window the user is typing in.

use ashpd::desktop::global_shortcuts::{GlobalShortcuts, NewShortcut};
use futures_util::StreamExt;
use gtk::gio::{AppLaunchContext, DesktopAppInfo, prelude::*};

/// Opens KDE's own shortcut editor without changing any binding ourselves.
pub fn open_settings() -> Result<(), String> {
    launch_settings(DesktopAppInfo::new("kcm_keys.desktop"))
}

fn launch_settings(entry: Option<DesktopAppInfo>) -> Result<(), String> {
    entry
        .ok_or("KDE Shortcuts is not installed.")?
        .launch(&[], None::<&AppLaunchContext>)
        .map_err(|_| "Could not launch KDE Shortcuts.".into())
}

/// The shortcuts the island offers, with the keys it suggests.
pub const SHORTCUTS: &[(&str, &str, &str)] = &[
    (
        "allow",
        "Allow the permission waiting on the island",
        "CTRL+ALT+Y",
    ),
    ("deny", "Deny the permission waiting on the island", "CTRL+ALT+N"),
    ("talk", "Dictate, or speak to a focused Vults Chat", "CTRL+ALT+V"),
    ("talk-again", "Copy the last dictation again", "CTRL+ALT+B"),
    ("talk-cancel", "Cancel dictation or Chat voice", "CTRL+ALT+X"),
    ("next", "Put the next session in front", "CTRL+ALT+J"),
    ("previous", "Put the previous session in front", "CTRL+ALT+K"),
    // Not Ctrl+Alt+I: VS Code opens its chat with it, and sessions run in its terminal.
    ("open", "Open the island", "CTRL+ALT+space"),
    // The sizes (ADR 0021). Adding ids makes the desktop ask once for the new ones.
    ("mini", "Fold the island to the flock alone (Mini)", "CTRL+ALT+1"),
    ("quick", "Open the island (Quick)", "CTRL+ALT+2"),
    ("room", "Open the Nest (Room)", "CTRL+ALT+3"),
];

/// What a press (`down`) or a release of shortcut `id` tells the island, if anything. Only the talk
/// key cares about being let go. Dictation remains available with Zeca off.
pub fn island_event(id: &str, down: bool) -> Option<String> {
    if !SHORTCUTS.iter().any(|s| s.0 == id) {
        return None;
    }
    match (id, down) {
        (_, true) => Some(id.to_string()),
        ("talk", false) => Some("talk-up".to_string()),
        _ => None,
    }
}

/// Binds the shortcuts, reports the keys the desktop actually bound (`bound`: id → how to press
/// it), then calls `on` with an id each time one is pressed (`true`) or let go (`false`). Runs until the portal goes away;
/// returns why it could not start (no portal, the user declined…).
pub async fn listen(
    app_id: &str,
    describe: impl Fn(&str) -> Option<String>,
    bound: impl Fn(Vec<(String, String)>),
    on: impl Fn(&str, bool) + Send + 'static,
) -> Result<(), String> {
    // Unsandboxed apps tell the portal who they are, so the desktop can remember the binding.
    if let Ok(id) = app_id.parse() {
        let _ = ashpd::register_host_app(id).await;
    }
    let portal = GlobalShortcuts::new()
        .await
        .map_err(|e| format!("no global shortcuts portal: {e}"))?;
    let session = portal
        .create_session(Default::default())
        .await
        .map_err(|e| e.to_string())?;
    let wanted: Vec<NewShortcut> = SHORTCUTS
        .iter()
        .map(|(id, about, keys)| {
            NewShortcut::new(*id, describe(id).unwrap_or_else(|| about.to_string())).preferred_trigger(*keys)
        })
        .collect();
    let result = portal
        .bind_shortcuts(&session, &wanted, None, Default::default())
        .await
        .and_then(|r| r.response())
        .map_err(|e| format!("shortcuts not bound: {e}"))?;
    bound(
        result
            .shortcuts()
            .iter()
            .map(|s| (s.id().to_string(), s.trigger_description().to_string()))
            .collect(),
    );
    let session_path = serde_json::to_value(&session)
        .map_err(|e| e.to_string())?
        .as_str()
        .ok_or("the portal returned an invalid session handle")?
        .to_owned();
    let pressed = portal.receive_activated().await.map_err(|e| e.to_string())?;
    let released = portal.receive_deactivated().await.map_err(|e| e.to_string())?;
    let changed = portal
        .receive_shortcuts_changed()
        .await
        .map_err(|e| e.to_string())?;
    let closed = session.receive_closed().await.map_err(|e| e.to_string())?;
    enum Event {
        Closed,
        Key(String, bool),
        Bound(Vec<(String, String)>),
    }
    let presses = futures_util::stream::select(
        pressed
            .filter(|e| std::future::ready(e.session_handle().as_str() == session_path))
            .map(|e| Event::Key(e.shortcut_id().to_string(), true)),
        released
            .filter(|e| std::future::ready(e.session_handle().as_str() == session_path))
            .map(|e| Event::Key(e.shortcut_id().to_string(), false)),
    );
    let events = futures_util::stream::select(
        presses,
        changed
            .filter(|e| std::future::ready(e.session_handle().as_str() == session_path))
            .map(|e| {
                Event::Bound(
                    e.shortcuts()
                        .iter()
                        .map(|s| (s.id().to_string(), s.trigger_description().to_string()))
                        .collect(),
                )
            }),
    );
    let mut events = futures_util::stream::select(events, closed.map(|_| Event::Closed));
    while let Some(event) = events.next().await {
        match event {
            Event::Closed => break,
            Event::Key(id, down) => on(&id, down),
            Event::Bound(keys) => bound(keys),
        }
    }
    drop(events);
    // Keep the session alive for as long as we listen.
    drop(session);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shortcut_editor_reports_missing_entries_and_launch_failures() {
        assert!(launch_settings(None).is_err());
        // A harmless executable exercises the desktop launcher without opening any window.
        let key = gtk::glib::KeyFile::new();
        key.set_string("Desktop Entry", "Type", "Application");
        key.set_string("Desktop Entry", "Name", "Shortcut launcher test");
        key.set_string("Desktop Entry", "Exec", "/bin/true");
        assert!(launch_settings(DesktopAppInfo::from_keyfile(&key)).is_ok());
        key.set_string("Desktop Entry", "Path", "/proc/self/vults-missing-directory");
        let entry = DesktopAppInfo::from_keyfile(&key).expect("the desktop entry is valid");
        assert!(launch_settings(Some(entry)).is_err());
    }

    #[test]
    fn a_press_reaches_the_island_and_only_talk_has_a_release() {
        assert_eq!(island_event("allow", true).as_deref(), Some("allow"));
        assert_eq!(island_event("allow", false), None);
        assert_eq!(island_event("talk", true).as_deref(), Some("talk"));
        assert_eq!(island_event("talk", false).as_deref(), Some("talk-up"));
    }

    #[test]
    fn dictation_is_independent_of_chat_and_auxiliary_keys_only_press() {
        for id in ["talk-again", "talk-cancel", "deny", "open"] {
            assert_eq!(island_event(id, true).as_deref(), Some(id));
            assert_eq!(island_event(id, false), None);
        }
        for id in ["talk", "talk-again", "talk-cancel"] {
            assert_eq!(SHORTCUTS.iter().filter(|s| s.0 == id).count(), 1);
        }
    }
}
