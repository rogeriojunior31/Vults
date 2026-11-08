//! The blocking worker owns WinRT and every toast; callbacks only request Open.

use std::collections::BTreeMap;
use tauri::AppHandle;
use tokio::sync::mpsc;
use vults_core::{
    SessionKey,
    notify::{self, Change},
};
use vults_platform::notifications::{Desktop, Toast};

use super::Note;

struct Entry {
    tag: String,
    toast: Toast,
}

pub(super) fn run(app: AppHandle, mut rx: mpsc::UnboundedReceiver<Note>) {
    let mut desktop = None;
    let mut entries = BTreeMap::<SessionKey, Entry>::new();
    let mut plain = None;
    let mut serial = 0_u64;
    while let Some(note) = rx.blocking_recv() {
        if let Note::Stop(done) = note {
            if let Some(desktop) = &desktop {
                clear(desktop, &mut entries, &mut plain);
            }
            let _ = done.send(());
            return;
        }
        if let Some(native) = &desktop {
            if !crate::settings::notifications(&app) || crate::lock::locked(&app) {
                clear(native, &mut entries, &mut plain);
            } else if crate::settings::dnd_instant(crate::settings::dnd_until(&app)).is_some()
                && let Some(toast) = plain.take()
            {
                native.remove(toast);
            }
        }
        if matches!(note, Note::Refresh) {
            continue;
        }
        if desktop.is_none() {
            match Desktop::new(&app.config().identifier) {
                Ok(native) => desktop = Some(native),
                Err(error) => {
                    tracing::warn!(%error, "Windows notifications unavailable");
                    continue;
                }
            }
        }
        let native = desktop.as_ref().expect("desktop initialized");
        match note {
            Note::Core(Change::Withdraw { session }) => {
                if let Some(entry) = entries.remove(&session) {
                    native.remove(entry.toast);
                }
            }
            Note::Core(Change::Show { session, notice }) => {
                let previous = entries.remove(&session);
                let tag = if let Some(entry) = &previous {
                    entry.tag.clone()
                } else {
                    serial += 1;
                    format!("{serial:016x}")
                };
                if !crate::settings::notifications(&app) || crate::lock::locked(&app) {
                    if let Some(entry) = previous {
                        native.remove(entry.toast);
                    }
                    continue;
                }
                if !enabled(native) {
                    if let Some(entry) = previous {
                        native.remove(entry.toast);
                    }
                    continue;
                }
                let handle = app.clone();
                let selected = session.clone();
                let open = move || {
                    if !crate::lock::locked(&handle) {
                        crate::runtime::shortcut_intent(&handle, notify::open(&selected));
                        crate::tray::activate(&handle);
                    }
                };
                let shown = match previous {
                    Some(entry) => {
                        native.replace(entry.toast, &notice.title, &notice.body, Some(open), failure)
                    }
                    None => native.show(&tag, &notice.title, &notice.body, Some(open), failure),
                };
                match shown {
                    Ok(toast) => {
                        entries.insert(session, Entry { tag, toast });
                    }
                    Err(error) => tracing::warn!(%error, "Windows notification not shown"),
                }
            }
            Note::Plain { title, body } => {
                let previous = plain.take();
                if !crate::settings::notifications(&app)
                    || crate::lock::locked(&app)
                    || crate::settings::dnd_instant(crate::settings::dnd_until(&app)).is_some()
                    || !enabled(native)
                {
                    if let Some(toast) = previous {
                        native.remove(toast);
                    }
                    continue;
                }
                let shown = match previous {
                    Some(toast) => native.replace(toast, &title, &body, None::<fn()>, failure),
                    None => native.show("plain", &title, &body, None::<fn()>, failure),
                };
                match shown {
                    Ok(toast) => plain = Some(toast),
                    Err(error) => tracing::warn!(%error, "Windows notification not shown"),
                }
            }
            Note::Stop(_) | Note::Refresh => unreachable!(),
        }
    }
    if let Some(desktop) = &desktop {
        clear(desktop, &mut entries, &mut plain);
    }
}

fn failure(code: i32) {
    tracing::warn!(code, "Windows notification delivery failed");
}

fn enabled(desktop: &Desktop) -> bool {
    match desktop.enabled() {
        Ok(enabled) => enabled,
        Err(error) => {
            tracing::warn!(%error, "Windows notification settings unavailable");
            false
        }
    }
}

fn clear(desktop: &Desktop, entries: &mut BTreeMap<SessionKey, Entry>, plain: &mut Option<Toast>) {
    for (_, entry) in std::mem::take(entries) {
        desktop.remove(entry.toast);
    }
    if let Some(toast) = plain.take() {
        desktop.remove(toast);
    }
}
