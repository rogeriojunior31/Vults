//! Reserve replacements before releasing working bindings; only this worker owns native keys.
use std::collections::{BTreeMap, HashSet};
use std::sync::{Arc, Mutex, mpsc};
use windows::Win32::Foundation::{LPARAM, WPARAM};
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    HOT_KEY_MODIFIERS, MOD_ALT, MOD_CONTROL, MOD_NOREPEAT, MOD_SHIFT, RegisterHotKey, UnregisterHotKey,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetMessageW, MSG, PM_NOREMOVE, PM_REMOVE, PeekMessageW, PostThreadMessageW, WM_APP, WM_HOTKEY, WM_QUIT,
};

const WAKE: u32 = WM_APP + 71;
const DEFAULTS: &[(&str, &str)] = &[
    ("open", "Ctrl+Alt+Space"),
    ("next", "Ctrl+Alt+Right"),
    ("previous", "Ctrl+Alt+Left"),
    ("mini", "Ctrl+Alt+F1"),
    ("quick", "Ctrl+Alt+F2"),
    ("room", "Ctrl+Alt+F3"),
    ("allow", "Ctrl+Alt+F8"),
    ("deny", "Ctrl+Alt+F9"),
    ("talk-cancel", "Ctrl+Alt+F10"),
];
pub type Bindings = BTreeMap<String, String>;

#[derive(Clone, Debug, Default, serde::Serialize)]
pub struct Snapshot {
    pub requested: Bindings,
    pub active: Bindings,
    pub defaults: Bindings,
    pub conflicts: Vec<String>,
}
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct Chord {
    modifiers: u32,
    key: u32,
    label: String,
}

fn chord(text: &str) -> Result<Chord, String> {
    let parts: Vec<_> = text.split('+').collect();
    let mut modifiers = 0;
    for part in parts.get(..parts.len().saturating_sub(1)).unwrap_or_default() {
        let bit = match *part {
            "Ctrl" => MOD_CONTROL.0,
            "Alt" => MOD_ALT.0,
            "Shift" => MOD_SHIFT.0,
            _ => return Err("shortcut-invalid".into()),
        };
        if modifiers & bit != 0 {
            return Err("shortcut-invalid".into());
        }
        modifiers |= bit;
    }
    // Never claim Windows keys, typing/AltGr characters, Escape, Delete or the debugger's F12.
    if modifiers.count_ones() < 2 {
        return Err("shortcut-invalid".into());
    }
    let name = *parts.last().ok_or("shortcut-invalid")?;
    let key = match name {
        "Space" => 0x20,
        "Left" => 0x25,
        "Up" => 0x26,
        "Right" => 0x27,
        "Down" => 0x28,
        "Home" => 0x24,
        "End" => 0x23,
        "PageUp" => 0x21,
        "PageDown" => 0x22,
        "Insert" => 0x2d,
        _ => {
            let number = name
                .strip_prefix('F')
                .and_then(|s| s.parse::<u32>().ok())
                .filter(|n| (1..=24).contains(n) && *n != 12)
                .ok_or("shortcut-invalid")?;
            if name != format!("F{number}") {
                return Err("shortcut-invalid".into());
            }
            0x6f + number
        }
    };
    let mut labels = Vec::new();
    for (bit, label) in [
        (MOD_CONTROL.0, "Ctrl"),
        (MOD_ALT.0, "Alt"),
        (MOD_SHIFT.0, "Shift"),
    ] {
        if modifiers & bit != 0 {
            labels.push(label);
        }
    }
    labels.push(name);
    if labels.join("+") != text {
        return Err("shortcut-invalid".into());
    }
    Ok(Chord {
        modifiers,
        key,
        label: labels.join("+"),
    })
}
fn requested(overrides: &Bindings) -> Result<Vec<(String, Chord)>, String> {
    if overrides
        .keys()
        .any(|id| !DEFAULTS.iter().any(|(known, _)| *known == id))
    {
        return Err("shortcut-invalid".into());
    }
    let mut seen = HashSet::new();
    let mut bindings = Vec::new();
    for &(id, default) in DEFAULTS {
        let value = overrides.get(id).map(String::as_str).unwrap_or(default);
        if value.is_empty() {
            continue;
        }
        let chord = chord(value)?;
        if !seen.insert((chord.modifiers, chord.key)) {
            return Err("shortcut-duplicate".into());
        }
        bindings.push((id.into(), chord));
    }
    Ok(bindings)
}
struct Request {
    bindings: Bindings,
    persist: Box<dyn FnOnce() -> Result<(), String> + Send>,
    reply: mpsc::SyncSender<Result<Snapshot, String>>,
}
#[derive(Clone, Debug)]
pub struct Controller {
    thread: u32,
    requests: mpsc::Sender<Request>,
    state: Arc<Mutex<Snapshot>>,
    _lifetime: Arc<Lifetime>,
}

#[derive(Debug)]
struct Lifetime(u32);
impl Drop for Lifetime {
    fn drop(&mut self) {
        // SAFETY: only this worker's queue is asked to stop; no foreign thread is terminated.
        let _ = unsafe { PostThreadMessageW(self.0, WM_QUIT, WPARAM(0), LPARAM(0)) };
    }
}
impl Controller {
    pub fn snapshot(&self) -> Snapshot {
        self.state.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }
    /// Off the UI thread. Save only after every new key has been reserved.
    pub fn apply(
        &self,
        bindings: Bindings,
        persist: impl FnOnce() -> Result<(), String> + Send + 'static,
    ) -> Result<Snapshot, String> {
        requested(&bindings)?;
        let (reply, receive) = mpsc::sync_channel(1);
        self.requests
            .send(Request {
                bindings,
                persist: Box::new(persist),
                reply,
            })
            .map_err(|_| "shortcut-unavailable")?;
        // SAFETY: the worker creates its queue before publishing this thread id.
        unsafe { PostThreadMessageW(self.thread, WAKE, WPARAM(0), LPARAM(0)) }
            .map_err(|_| "shortcut-unavailable")?;
        receive.recv().map_err(|_| "shortcut-unavailable".to_string())?
    }
}
#[derive(Debug)]
struct Registered {
    cookie: i32,
    action: String,
    chord: Chord,
}
fn register(cookie: i32, chord: &Chord) -> bool {
    // SAFETY: this thread owns registrations and their unique message cookies.
    unsafe {
        RegisterHotKey(
            None,
            cookie,
            HOT_KEY_MODIFIERS(chord.modifiers | MOD_NOREPEAT.0),
            chord.key,
        )
    }
    .is_ok()
}
fn unregister(cookie: i32) {
    // SAFETY: only cookies owned by this worker are passed here.
    let _ = unsafe { UnregisterHotKey(None, cookie) };
}
fn snapshot(overrides: &Bindings, active: &[Registered], conflicts: Vec<String>) -> Snapshot {
    let defaults: Bindings = DEFAULTS
        .iter()
        .map(|(id, key)| (id.to_string(), key.to_string()))
        .collect();
    let mut requested = defaults.clone();
    requested.extend(overrides.clone());
    Snapshot {
        requested,
        defaults,
        active: active
            .iter()
            .map(|r| (r.action.clone(), r.chord.label.clone()))
            .collect(),
        conflicts,
    }
}
fn replace(
    active: &mut Vec<Registered>,
    next_cookie: &mut i32,
    request: Request,
) -> Result<Snapshot, String> {
    let desired = requested(&request.bindings)?;
    let mut reserved: Vec<Registered> = Vec::new();
    for (action, chord) in &desired {
        if active.iter().any(|old| old.chord == *chord) {
            continue;
        }
        let Some(cookie) = next_cookie.checked_add(1).filter(|cookie| *cookie <= 0xbfff) else {
            for entry in reserved {
                unregister(entry.cookie);
            }
            return Err("shortcut-unavailable".into());
        };
        *next_cookie = cookie;
        if !register(*next_cookie, chord) {
            for entry in reserved {
                unregister(entry.cookie);
            }
            return Err(format!("shortcut-conflict:{action}"));
        }
        reserved.push(Registered {
            cookie: *next_cookie,
            action: action.clone(),
            chord: chord.clone(),
        });
    }
    if let Err(error) = (request.persist)() {
        for entry in reserved {
            unregister(entry.cookie);
        }
        return Err(error);
    }
    for old in active.drain(..) {
        if let Some((action, _)) = desired.iter().find(|(_, chord)| *chord == old.chord) {
            reserved.push(Registered {
                action: action.clone(),
                ..old
            });
        } else {
            unregister(old.cookie);
        }
    }
    *active = reserved;
    let mut queued = MSG::default();
    // SAFETY: discard only this worker's pre-commit key messages, including remapped cookies.
    while unsafe { PeekMessageW(&mut queued, None, WM_HOTKEY, WM_HOTKEY, PM_REMOVE) }.as_bool() {}
    Ok(snapshot(&request.bindings, active, Vec::new()))
}
pub fn start(
    initial: Bindings,
    changed: impl Fn(Snapshot) + Send + 'static,
    on: impl Fn(&str) + Send + 'static,
) -> Result<Controller, String> {
    let (requests, receive) = mpsc::channel::<Request>();
    let (ready, wait) = mpsc::sync_channel(1);
    let state = Arc::new(Mutex::new(Snapshot::default()));
    let shared = state.clone();
    std::thread::Builder::new()
        .name("vults-hotkeys".into())
        .spawn(move || {
            let mut message = MSG::default();
            // SAFETY: initialize the queue before another thread can send requests.
            unsafe {
                let _ = PeekMessageW(&mut message, None, 0, 0, PM_NOREMOVE);
            }
            let mut next_cookie = 0;
            let mut active = Vec::new();
            let mut conflicts = Vec::new();
            match requested(&initial) {
                Ok(bindings) => {
                    for (action, chord) in bindings {
                        next_cookie += 1;
                        if register(next_cookie, &chord) {
                            active.push(Registered {
                                cookie: next_cookie,
                                action,
                                chord,
                            });
                        } else {
                            conflicts.push(action);
                        }
                    }
                }
                Err(_) => conflicts.extend(DEFAULTS.iter().map(|(id, _)| id.to_string())),
            }
            let status = snapshot(&initial, &active, conflicts);
            *shared.lock().unwrap_or_else(|e| e.into_inner()) = status.clone();
            changed(status);
            // SAFETY: current thread id is valid while its message loop lives.
            let _ = ready.send(unsafe { GetCurrentThreadId() });
            loop {
                // SAFETY: live output buffer, with no foreign window filter.
                if unsafe { GetMessageW(&mut message, None, 0, 0) }.0 <= 0 {
                    break;
                }
                if message.message == WAKE {
                    while let Ok(request) = receive.try_recv() {
                        let reply = request.reply.clone();
                        let result = replace(&mut active, &mut next_cookie, request);
                        if let Ok(status) = &result {
                            *shared.lock().unwrap_or_else(|e| e.into_inner()) = status.clone();
                            changed(status.clone());
                        }
                        let _ = reply.send(result);
                    }
                } else if message.message == WM_HOTKEY
                    && let Some(entry) = active.iter().find(|r| r.cookie == message.wParam.0 as i32)
                {
                    on(&entry.action);
                }
            }
            for entry in active {
                unregister(entry.cookie);
            }
        })
        .map_err(|_| "shortcut-unavailable")?;
    let thread = wait.recv().map_err(|_| "shortcut-unavailable")?;
    Ok(Controller {
        thread,
        requests,
        state,
        _lifetime: Arc::new(Lifetime(thread)),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reserved_typing_keys_duplicates_and_unknown_actions_are_rejected() {
        for key in [
            "Win+L",
            "Ctrl+Alt+Q",
            "Ctrl+Shift+Escape",
            "Ctrl+Alt+Delete",
            "Ctrl+Alt+F12",
            "Ctrl+Ctrl+F1",
            "Ctrl+F1",
        ] {
            assert!(chord(key).is_err(), "{key}");
        }
        assert!(requested(&Bindings::from([("unknown".into(), "".into())])).is_err());
        assert!(requested(&Bindings::from([("open".into(), "Ctrl+Alt+F1".into())])).is_err());
        assert_eq!(
            requested(&Bindings::from([("open".into(), "".into())]))
                .unwrap()
                .len(),
            DEFAULTS.len() - 1
        );
    }
    #[test]
    fn conflicts_and_save_failures_preserve_working_keys() {
        let disabled = || Bindings::from_iter(DEFAULTS.iter().map(|(id, _)| (id.to_string(), String::new())));
        let mut initial = disabled();
        initial.insert("open".into(), "Ctrl+Alt+Shift+F22".into());
        let controller = start(initial, |_| {}, |_| {}).unwrap();
        let before = controller.snapshot().active;
        assert_eq!(before.get("open").map(String::as_str), Some("Ctrl+Alt+Shift+F22"));
        let mut desired = disabled();
        desired.insert("open".into(), "Ctrl+Alt+Shift+F23".into());
        assert!(
            controller
                .apply(desired.clone(), || Err("save-failed".into()))
                .is_err()
        );
        assert_eq!(controller.snapshot().active, before);
        let updated = controller.apply(desired, || Ok(())).unwrap();
        let mut other_keys = disabled();
        other_keys.insert("open".into(), "Ctrl+Alt+Shift+F24".into());
        let other = start(other_keys, |_| {}, |_| {}).unwrap();
        let mut conflicting = updated.requested;
        conflicting.insert("open".into(), "Ctrl+Alt+Shift+F24".into());
        assert!(
            controller
                .apply(conflicting, || panic!("a conflict cannot save"))
                .is_err()
        );
        assert_eq!(controller.snapshot().active, updated.active);
        controller.apply(disabled(), || Ok(())).unwrap();
        other.apply(disabled(), || Ok(())).unwrap();
    }
}
