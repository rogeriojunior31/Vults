//! Desktop notifications over org.freedesktop.Notifications. Core decides what to show
//! (`core::notify`); this only sends it, one notification per session (replaced in place), and
//! hears the one action a notification has: *Open*. Never Allow or Deny (rule 2, ADR 0004).

use tauri::{AppHandle, Manager};
use tokio::sync::mpsc;
use vults_core::notify::Change;

/// The changes, on their way to the D-Bus task.
pub struct Notices(mpsc::UnboundedSender<Change>);

/// From the runtime loop: never waits on D-Bus.
pub fn send(app: &AppHandle, changes: Vec<Change>) {
    if let Some(n) = app.try_state::<Notices>() {
        for change in changes {
            let _ = n.0.send(change);
        }
    }
}

pub fn start(app: &AppHandle) {
    let (tx, rx) = mpsc::unbounded_channel();
    app.manage(Notices(tx));
    #[cfg(target_os = "linux")]
    tauri::async_runtime::spawn(linux::run(app.clone(), rx));
    #[cfg(not(target_os = "linux"))]
    drop(rx);
}

/// The body may be read as markup (the spec's `body-markup`): an agent's words must stay words.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(target_os = "linux")]
mod linux {
    use std::collections::{BTreeMap, HashMap};

    use futures_util::StreamExt;
    use tauri::AppHandle;
    use tokio::sync::mpsc;
    use vults_core::SessionKey;
    use vults_core::notify::{self, Change};
    use zbus::zvariant::Value;

    #[zbus::proxy(
        interface = "org.freedesktop.Notifications",
        default_service = "org.freedesktop.Notifications",
        default_path = "/org/freedesktop/Notifications"
    )]
    trait Notifications {
        #[allow(clippy::too_many_arguments)]
        fn notify(
            &self,
            app_name: &str,
            replaces_id: u32,
            app_icon: &str,
            summary: &str,
            body: &str,
            actions: &[&str],
            hints: HashMap<&str, Value<'_>>,
            expire_timeout: i32,
        ) -> zbus::Result<u32>;

        fn close_notification(&self, id: u32) -> zbus::Result<()>;

        #[zbus(signal)]
        fn action_invoked(&self, id: u32, action_key: &str) -> zbus::Result<()>;

        #[zbus(signal)]
        fn notification_closed(&self, id: u32, reason: u32) -> zbus::Result<()>;
    }

    pub async fn run(app: AppHandle, mut rx: mpsc::UnboundedReceiver<Change>) {
        let proxy = match connect().await {
            Ok(p) => p,
            Err(e) => {
                tracing::warn!("no desktop notifications: {e}");
                // Keep draining, so nothing piles up.
                while rx.recv().await.is_some() {}
                return;
            }
        };
        let (Ok(mut actions), Ok(mut closed)) = (
            proxy.receive_action_invoked().await,
            proxy.receive_notification_closed().await,
        ) else {
            tracing::warn!("desktop notifications: can't hear their clicks");
            while rx.recv().await.is_some() {}
            return;
        };
        // The server's id for each session's notification.
        let mut ids: BTreeMap<SessionKey, u32> = BTreeMap::new();
        loop {
            tokio::select! {
                // A click comes with its NotificationClosed: hear the click first, while its id
                // still names the session.
                biased;
                Some(signal) = actions.next() => {
                    let Ok(args) = signal.args() else { continue };
                    let session = ids.iter().find(|(_, id)| **id == args.id).map(|(k, _)| k.clone());
                    if let Some(session) = session
                        && matches!(args.action_key, "default" | "open")
                    {
                        tracing::info!("notification opened");
                        crate::runtime::shortcut_intent(&app, notify::open(&session));
                        crate::tray::activate(&app);
                    }
                }
                Some(signal) = closed.next() => {
                    // Dismissed or expired: the next one starts anew, nothing left to close.
                    if let Ok(args) = signal.args() {
                        ids.retain(|_, id| *id != args.id);
                    }
                }
                change = rx.recv() => match change {
                    None => return,
                    Some(Change::Show { session, notice }) => {
                        let replaces = ids.get(&session).copied().unwrap_or(0);
                        let hints = HashMap::from([
                            ("desktop-entry", Value::from(vults_brand::SLUG)),
                            ("urgency", Value::U8(1)),
                        ]);
                        let body = super::escape(&notice.body);
                        let open = vults_core::i18n::open(crate::settings::lang(&app));
                        let sent = proxy
                            .notify(
                                vults_brand::NAME,
                                replaces,
                                vults_brand::SLUG,
                                &notice.title,
                                &body,
                                // "default" is a click on the notification itself; "open" its
                                // button, in the user's language. Nothing else.
                                &["default", open, "open", open],
                                hints,
                                -1,
                            )
                            .await;
                        match sent {
                            Ok(id) => {
                                tracing::debug!(id, kind = ?notice.kind, "notification shown");
                                ids.insert(session, id);
                            }
                            Err(e) => tracing::warn!(kind = ?notice.kind, "notification not shown: {e}"),
                        }
                    }
                    Some(Change::Withdraw { session }) => {
                        if let Some(id) = ids.remove(&session) {
                            let _ = proxy.close_notification(id).await;
                        }
                    }
                },
            }
        }
    }

    async fn connect() -> zbus::Result<NotificationsProxy<'static>> {
        let conn = zbus::Connection::session().await?;
        NotificationsProxy::new(&conn).await
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn an_agents_words_cannot_become_markup() {
        assert_eq!(
            super::escape("<a href=\"x\">a & b</a>"),
            "&lt;a href=\"x\"&gt;a &amp; b&lt;/a&gt;"
        );
    }
}
