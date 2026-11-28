//! Keeps the computer from suspending while agents work (t3code's *keep awake*): a logind
//! `sleep` inhibitor held while a session in the flock or a chat turn is busy, released as soon
//! as nothing is. Only `sleep`, never `idle`: the screen still dims and locks, so a computer left
//! alone is never left unlocked. Local only: a call on the system bus, nothing leaves it.

use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Manager};

/// A session in the flock is thinking or working (set by the runtime on each view).
#[derive(Default)]
pub struct Flock(AtomicBool);

pub fn flock(app: &AppHandle, working: bool) {
    if let Some(f) = app.try_state::<Flock>() {
        f.0.store(working, Ordering::Relaxed);
    }
}

/// Whether the inhibitor should be held now.
#[cfg(target_os = "linux")]
fn wanted(app: &AppHandle) -> bool {
    let flock = app
        .try_state::<Flock>()
        .is_some_and(|f| f.0.load(Ordering::Relaxed));
    crate::settings::keep_awake(app) && (flock || crate::chat::busy(app))
}

/// Watches every few seconds and holds or drops the inhibitor; logind takes it back by itself
/// if the app dies, since it lives in the file descriptor.
pub fn start(app: &AppHandle) {
    app.manage(Flock::default());
    #[cfg(target_os = "linux")]
    {
        let app = app.clone();
        tauri::async_runtime::spawn(async move {
            let mut held: Option<zbus::zvariant::OwnedFd> = None;
            let mut failed_at: Option<std::time::Instant> = None;
            loop {
                let want = wanted(&app);
                if want && held.is_none() && failed_at.is_none_or(|t| t.elapsed().as_secs() > 300) {
                    match inhibit().await {
                        Ok(fd) => {
                            held = Some(fd);
                            failed_at = None;
                        }
                        Err(e) => {
                            // No logind (a container, another init): try again later, quietly.
                            tracing::info!("awake: could not hold the sleep inhibitor: {e}");
                            failed_at = Some(std::time::Instant::now());
                        }
                    }
                } else if !want {
                    held = None;
                }
                tokio::time::sleep(std::time::Duration::from_secs(10)).await;
            }
        });
    }
}

#[cfg(target_os = "linux")]
async fn inhibit() -> zbus::Result<zbus::zvariant::OwnedFd> {
    let conn = zbus::Connection::system().await?;
    let reply = conn
        .call_method(
            Some("org.freedesktop.login1"),
            "/org/freedesktop/login1",
            Some("org.freedesktop.login1.Manager"),
            "Inhibit",
            &("sleep", vults_brand::NAME, "Agents are working", "block"),
        )
        .await?;
    reply.body().deserialize::<zbus::zvariant::OwnedFd>()
}
