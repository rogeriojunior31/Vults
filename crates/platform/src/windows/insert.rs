//! Targeted native edit replacement. No keyboard injection, clipboard replacement or terminal submit.
use crate::focus;
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::IsWindowEnabled;
use windows::Win32::UI::WindowsAndMessaging::{
    GWL_STYLE, GetClassNameW, GetWindowLongPtrW, GetWindowThreadProcessId, SMTO_ABORTIFHUNG, SMTO_BLOCK,
    SendMessageTimeoutW,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Pasted,
    Copied,
    Session,
    Held,
}
#[derive(Debug)]
pub struct Target {
    window: focus::Window,
    root: isize,
    edit: isize,
    birth: u64,
    generation: u64,
}
impl Target {
    pub fn capture() -> Option<Self> {
        if !focus::start() {
            return None;
        }
        let generation = focus::generation();
        let (window, root, edit, birth) = focus::native()?;
        let target = Self {
            window,
            root,
            edit,
            birth,
            generation,
        };
        (target.current() && target.editor()).then_some(target)
    }
    pub fn window(&self) -> &focus::Window {
        &self.window
    }
    pub fn current(&self) -> bool {
        focus::start()
            && self.generation == focus::generation()
            && focus::native().is_some_and(|(window, root, edit, birth)| {
                window.identity == self.window.identity
                    && root == self.root
                    && edit == self.edit
                    && birth == self.birth
            })
    }
    pub fn unchanged(&self) -> bool {
        self.current()
    }
    fn editor(&self) -> bool {
        if self.window.is_terminal() || self.edit == 0 {
            return false;
        }
        // SAFETY: metadata only; a timed-out password query refuses delivery.
        unsafe {
            let hwnd = HWND(self.edit as *mut _);
            let mut pid = 0;
            GetWindowThreadProcessId(hwnd, Some(&mut pid));
            if Some(pid) != self.window.pid || !IsWindowEnabled(hwnd).as_bool() {
                return false;
            }
            let mut class = [0u16; 128];
            let length = GetClassNameW(hwnd, &mut class);
            let class = String::from_utf16_lossy(&class[..length as usize]).to_ascii_lowercase();
            if !matches!(class.as_str(), "edit" | "richedit20w" | "richedit50w") {
                return false;
            }
            if GetWindowLongPtrW(hwnd, GWL_STYLE) & (0x20 | 0x800) != 0 {
                return false;
            }
            let mut password = 1;
            SendMessageTimeoutW(
                hwnd,
                0x00d2,
                WPARAM(0),
                LPARAM(0),
                SMTO_ABORTIFHUNG | SMTO_BLOCK,
                100,
                Some(&mut password),
            )
            .0 != 0
                && password == 0
        }
    }
    pub fn insert(&self, text: &str) -> bool {
        if text.contains('\0') || text.len() > 65_536 {
            return false;
        }
        if !self.current() || !self.editor() || !self.current() {
            return false;
        }
        // Single-line controls must not truncate a multiline transcript.
        if text.contains(['\r', '\n'])
            && unsafe { GetWindowLongPtrW(HWND(self.edit as *mut _), GWL_STYLE) } & 4 == 0
        {
            return false;
        }
        replace_edit(HWND(self.edit as *mut _), text)
    }
}
pub fn wayland_available() -> bool {
    false
}

fn replace_edit(hwnd: HWND, text: &str) -> bool {
    let text: Vec<u16> = text
        .replace("\r\n", "\n")
        .replace('\n', "\r\n")
        .encode_utf16()
        .chain(Some(0))
        .collect();
    // SAFETY: EM_REPLACESEL is system-marshaled below WM_USER. Its UTF-16 buffer
    // stays alive through the bounded call; only the captured native edit receives it.
    unsafe {
        SendMessageTimeoutW(
            hwnd,
            0x00c2,
            WPARAM(1),
            LPARAM(text.as_ptr() as isize),
            SMTO_ABORTIFHUNG | SMTO_BLOCK,
            200,
            None,
        )
        .0 != 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DestroyWindow, GetWindowTextW, SendMessageW, WINDOW_EX_STYLE, WINDOW_STYLE, WS_POPUP,
    };
    struct Edit(HWND);
    impl Edit {
        fn new(style: u32) -> Self {
            // SAFETY: a hidden test-owned built-in EDIT control; no user window is activated.
            Self(
                unsafe {
                    CreateWindowExW(
                        WINDOW_EX_STYLE(0),
                        windows::core::w!("EDIT"),
                        windows::core::w!("leftRIGHTend"),
                        WS_POPUP | WINDOW_STYLE(style),
                        0,
                        0,
                        200,
                        80,
                        None,
                        None,
                        None,
                        None,
                    )
                }
                .unwrap(),
            )
        }
        fn target(&self) -> Target {
            Target {
                window: focus::Window {
                    class: "notepad".into(),
                    title: String::new(),
                    pid: Some(std::process::id()),
                    identity: "hidden-fixture".into(),
                },
                root: self.0.0 as isize,
                edit: self.0.0 as isize,
                birth: 0,
                generation: focus::generation(),
            }
        }
    }
    impl Drop for Edit {
        fn drop(&mut self) {
            let _ = unsafe { DestroyWindow(self.0) };
        }
    }
    #[test]
    fn unicode_selection_replacement_uses_the_exact_hidden_edit_without_keyboard_or_clipboard() {
        let edit = Edit::new(4);
        unsafe {
            SendMessageW(edit.0, 0x00b1, Some(WPARAM(4)), Some(LPARAM(9)));
        }
        let inserted = "A\u{e7}\u{e3}\u{1f985}\u{4e2d}\nB";
        assert!(replace_edit(edit.0, inserted));
        let mut text = [0u16; 128];
        let length = unsafe { GetWindowTextW(edit.0, &mut text) };
        assert_eq!(
            String::from_utf16_lossy(&text[..length as usize]),
            format!("left{}end", inserted.replace('\n', "\r\n"))
        );
        assert!(edit.target().editor());
        assert!(!edit.target().insert("must not reach a hidden target"));
    }
    #[test]
    fn password_read_only_disabled_and_terminal_targets_are_never_deliverable() {
        let password = Edit::new(0x20);
        assert!(!password.target().editor());
        let readonly = Edit::new(0x800);
        assert!(!readonly.target().editor());
        let plain = Edit::new(0);
        unsafe {
            SendMessageW(plain.0, 0x00cc, Some(WPARAM(0x25cf)), Some(LPARAM(0)));
        }
        assert!(
            !plain.target().editor(),
            "a dynamically masked control must also be refused"
        );
        let mut terminal = plain.target();
        terminal.window.class = "WindowsTerminal".into();
        assert!(!terminal.editor());
    }
}
