//! Global shortcuts on Wayland through the xdg-desktop-portal `GlobalShortcuts` interface: the
//! desktop (KDE asks the user once, and lets them change the keys) tells us when they are pressed.
//! No keyboard grab, nothing taken from the window the user is typing in.

use ashpd::desktop::global_shortcuts::{GlobalShortcuts, NewShortcut};
use futures_util::StreamExt;

/// The shortcuts the island offers, with the keys it suggests.
pub const SHORTCUTS: &[(&str, &str, &str)] = &[
    (
        "allow",
        "Allow the permission waiting on the island",
        "CTRL+ALT+Y",
    ),
    ("deny", "Deny the permission waiting on the island", "CTRL+ALT+N"),
    ("talk", "Hold to speak to the chat", "CTRL+ALT+V"),
    ("next", "Put the next session in front", "CTRL+ALT+J"),
    ("previous", "Put the previous session in front", "CTRL+ALT+K"),
    // Not Ctrl+Alt+I: VS Code opens its chat with it, and sessions run in its terminal.
    ("open", "Open the island", "CTRL+ALT+space"),
];

/// What a press (`down`) or a release of shortcut `id` tells the island, if anything. Only the talk
/// key cares about being let go, and it means nothing without the chat (`chat` false: Zeca off).
///
/// The desktop still holds the talk key then: the portal cannot release one shortcut and keep it.
/// Binding again without it makes the desktop forget it (KDE removes its keys), so turning Zeca
/// back on would ask the user again; closing the session releases every shortcut, Allow and Deny
/// included, which still work without Zeca.
pub fn island_event(id: &str, down: bool, chat: bool) -> Option<String> {
    match (id, down) {
        ("talk", _) if !chat => None,
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
    bound: impl FnOnce(Vec<(String, String)>),
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
        .map(|(id, about, keys)| NewShortcut::new(*id, *about).preferred_trigger(*keys))
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
    let pressed = portal.receive_activated().await.map_err(|e| e.to_string())?;
    let released = portal.receive_deactivated().await.map_err(|e| e.to_string())?;
    let mut events = futures_util::stream::select(
        pressed.map(|e| (e.shortcut_id().to_string(), true)),
        released.map(|e| (e.shortcut_id().to_string(), false)),
    );
    while let Some((id, down)) = events.next().await {
        on(&id, down);
    }
    // Keep the session alive for as long as we listen.
    drop(session);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_press_reaches_the_island_and_only_talk_has_a_release() {
        assert_eq!(island_event("allow", true, true).as_deref(), Some("allow"));
        assert_eq!(island_event("allow", false, true), None);
        assert_eq!(island_event("talk", true, true).as_deref(), Some("talk"));
        assert_eq!(island_event("talk", false, true).as_deref(), Some("talk-up"));
    }

    #[test]
    fn without_the_chat_the_talk_key_does_nothing_and_the_rest_still_work() {
        assert_eq!(island_event("talk", true, false), None);
        assert_eq!(island_event("talk", false, false), None);
        assert_eq!(island_event("deny", true, false).as_deref(), Some("deny"));
        assert_eq!(island_event("open", true, false).as_deref(), Some("open"));
    }
}
