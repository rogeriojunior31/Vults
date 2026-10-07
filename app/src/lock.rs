//! The screen lock, from `org.freedesktop.ScreenSaver` (KDE, GNOME and others): while locked the
//! connectors rest and the island stops animating; on unlock the core tells what happened
//! (`core::away`). Only listens: the app never locks or unlocks anything.

use std::sync::atomic::{AtomicBool, Ordering};

use tauri::{AppHandle, Manager};

/// Locked now, for the connectors (`connectors::running`).
#[derive(Default)]
pub struct Locked(AtomicBool);

pub fn locked(app: &AppHandle) -> bool {
    app.try_state::<Locked>()
        .is_some_and(|l| l.0.load(Ordering::Relaxed))
}

pub fn start(app: &AppHandle) {
    app.manage(Locked::default());
    #[cfg(target_os = "linux")]
    tauri::async_runtime::spawn(linux::run(app.clone()));
}

/// A change of the lock: kept, the connectors follow, and the core hears it.
fn changed(app: &AppHandle, now: bool) {
    let Some(state) = app.try_state::<Locked>() else {
        return;
    };
    if state.0.swap(now, Ordering::Relaxed) == now {
        return;
    }
    tracing::info!(locked = now, "screen lock");
    crate::connectors::apply(app);
    crate::runtime::set_locked(app, now);
}

#[cfg(target_os = "linux")]
mod linux {
    use futures_util::StreamExt;
    use tauri::AppHandle;

    const NAME: &str = "org.freedesktop.ScreenSaver";

    pub async fn run(app: AppHandle) {
        if let Err(e) = listen(&app).await {
            tracing::info!("no screen lock signal: {e}");
        }
    }

    async fn listen(app: &AppHandle) -> zbus::Result<()> {
        let conn = zbus::Connection::session().await?;
        // Every path: KDE sends it on /ScreenSaver and on /org/freedesktop/ScreenSaver.
        let rule = zbus::MatchRule::builder()
            .msg_type(zbus::message::Type::Signal)
            .interface(NAME)?
            .member("ActiveChanged")?
            .build();
        let mut signals = zbus::MessageStream::for_match_rule(rule, &conn, None).await?;
        // Locked already when the app starts (autostart behind a lock screen).
        if let Ok(reply) = conn
            .call_method(
                Some(NAME),
                "/org/freedesktop/ScreenSaver",
                Some(NAME),
                "GetActive",
                &(),
            )
            .await
            && let Ok(active) = reply.body().deserialize::<bool>()
        {
            super::changed(app, active);
        }
        while let Some(msg) = signals.next().await {
            if let Ok(msg) = msg
                && let Ok(active) = msg.body().deserialize::<bool>()
            {
                super::changed(app, active);
            }
        }
        Ok(())
    }
}
