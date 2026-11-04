//! Native hotkeys use non-printing keys so AltGr characters remain available.
use windows::Win32::UI::Input::KeyboardAndMouse::{
    MOD_ALT, MOD_CONTROL, MOD_NOREPEAT, RegisterHotKey, UnregisterHotKey,
};
use windows::Win32::UI::WindowsAndMessaging::{GetMessageW, MSG, WM_HOTKEY};

const KEYS: &[(&str, u32, &str)] = &[
    ("open", 0x20, "Ctrl+Alt+Space"),
    ("next", 0x27, "Ctrl+Alt+Right"),
    ("previous", 0x25, "Ctrl+Alt+Left"),
    ("mini", 0x70, "Ctrl+Alt+F1"),
    ("quick", 0x71, "Ctrl+Alt+F2"),
    ("room", 0x72, "Ctrl+Alt+F3"),
    ("allow", 0x77, "Ctrl+Alt+F8"),
    ("deny", 0x78, "Ctrl+Alt+F9"),
    ("talk-cancel", 0x79, "Ctrl+Alt+F10"),
];

/// Run on one dedicated worker: thread-owned registrations live until this loop exits.
pub fn listen(bound: impl FnOnce(Vec<(String, String)>), on: impl Fn(&str)) -> Result<(), String> {
    let mut registered = Vec::new();
    for (index, &(id, key, label)) in KEYS.iter().enumerate() {
        // SAFETY: no HWND means the current thread owns the hotkey and receives its messages.
        if unsafe { RegisterHotKey(None, index as i32 + 1, MOD_CONTROL | MOD_ALT | MOD_NOREPEAT, key) }
            .is_ok()
        {
            registered.push((index as i32 + 1, id, label));
        }
    }
    bound(
        registered
            .iter()
            .map(|(_, id, label)| (id.to_string(), label.to_string()))
            .collect(),
    );
    if registered.is_empty() {
        return Err("No global shortcut could be registered.".into());
    }
    let mut message = MSG::default();
    // SAFETY: the buffer is live and no window handle filters this thread's queue.
    let result = loop {
        let status = unsafe { GetMessageW(&mut message, None, 0, 0) }.0;
        if status <= 0 {
            break status;
        }
        if message.message == WM_HOTKEY
            && let Some((_, id, _)) = registered
                .iter()
                .find(|(key, _, _)| *key == message.wParam.0 as i32)
        {
            on(id);
        }
    };
    for (key, _, _) in registered {
        // SAFETY: these registrations belong to this same thread.
        let _ = unsafe { UnregisterHotKey(None, key) };
    }
    if result < 0 {
        Err("The global shortcut message loop failed.".into())
    } else {
        Ok(())
    }
}
