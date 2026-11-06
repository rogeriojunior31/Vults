//! Native foreground identity. Window titles are context, never insertion authority.
use std::sync::{
    OnceLock,
    atomic::{AtomicBool, AtomicU64, Ordering},
};
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Accessibility::{HWINEVENTHOOK, SetWinEventHook};
use windows::Win32::UI::WindowsAndMessaging::{
    GUITHREADINFO, GetClassNameW, GetForegroundWindow, GetGUIThreadInfo, GetMessageW,
    GetWindowThreadProcessId, MSG,
};

static GENERATION: AtomicU64 = AtomicU64::new(0);
static WATCH: OnceLock<bool> = OnceLock::new();
static ACTIVE: AtomicBool = AtomicBool::new(false);
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Window {
    pub class: String,
    pub title: String,
    pub pid: Option<u32>,
    pub identity: String,
}
impl Window {
    pub fn is_terminal(&self) -> bool {
        let class = self.class.to_ascii_lowercase();
        class.contains("console")
            || class.contains("cascadia")
            || class.contains("terminal")
            || class.contains("mintty")
            || class.contains("putty")
            || matches!(
                class.rsplit('.').next().unwrap_or(&class),
                "konsole"
                    | "kitty"
                    | "alacritty"
                    | "wezterm"
                    | "foot"
                    | "footclient"
                    | "ghostty"
                    | "xterm"
                    | "uxterm"
                    | "ptyxis"
            )
            || (matches!(class.as_str(), "code" | "code-oss" | "code-insiders")
                && self.title.to_ascii_lowercase().contains("terminal"))
    }
}
unsafe extern "system" fn changed(_: HWINEVENTHOOK, _: u32, _: HWND, _: i32, _: i32, _: u32, _: u32) {
    GENERATION.fetch_add(1, Ordering::SeqCst);
}
pub fn start() -> bool {
    let installed = *WATCH.get_or_init(|| {
        let (ready, receive) = std::sync::mpsc::sync_channel(1);
        if std::thread::Builder::new()
            .name("vults-focus".into())
            .spawn(move || {
                // SAFETY: callbacks only increment an atomic; the worker pumps both out-of-context hooks.
                unsafe {
                    let foreground = SetWinEventHook(3, 3, None, Some(changed), 0, 0, 0);
                    let focus = SetWinEventHook(0x8005, 0x8005, None, Some(changed), 0, 0, 0);
                    let ok = !foreground.0.is_null() && !focus.0.is_null();
                    ACTIVE.store(ok, Ordering::SeqCst);
                    let _ = ready.send(ok);
                    if ok {
                        let mut message = MSG::default();
                        while GetMessageW(&mut message, None, 0, 0).0 > 0 {}
                    }
                    if !foreground.0.is_null() {
                        let _ = windows::Win32::UI::Accessibility::UnhookWinEvent(foreground);
                    }
                    if !focus.0.is_null() {
                        let _ = windows::Win32::UI::Accessibility::UnhookWinEvent(focus);
                    }
                }
                ACTIVE.store(false, Ordering::SeqCst);
                GENERATION.fetch_add(1, Ordering::SeqCst);
            })
            .is_err()
        {
            return false;
        }
        receive.recv().unwrap_or(false)
    });
    installed && ACTIVE.load(Ordering::SeqCst)
}
pub(crate) fn generation() -> u64 {
    GENERATION.load(Ordering::SeqCst)
}
pub(crate) fn native() -> Option<(Window, isize, isize, u64)> {
    // SAFETY: bounded native output buffers, query-only APIs, no window is activated.
    unsafe {
        let root = GetForegroundWindow();
        if root.0.is_null() {
            return None;
        }
        let mut pid = 0;
        let thread = GetWindowThreadProcessId(root, Some(&mut pid));
        let birth = vults_peer::process_birth(pid)??;
        let mut gui = GUITHREADINFO {
            cbSize: std::mem::size_of::<GUITHREADINFO>() as u32,
            ..Default::default()
        };
        GetGUIThreadInfo(thread, &mut gui).ok()?;
        let mut class = [0u16; 256];
        let length = GetClassNameW(root, &mut class);
        if GetForegroundWindow() != root {
            return None;
        }
        let window = Window {
            class: vults_peer::process_image(pid)
                .and_then(|path| path.file_stem().map(|stem| stem.to_string_lossy().into_owned()))
                .unwrap_or_else(|| String::from_utf16_lossy(&class[..length as usize])),
            title: String::new(),
            pid: Some(pid),
            identity: format!("{:x}:{pid}:{birth}", root.0 as usize),
        };
        Some((window, root.0 as isize, gui.hwndFocus.0 as isize, birth))
    }
}
pub fn read() -> Option<Window> {
    native().map(|(window, _, _, _)| window)
}
pub fn key_down() -> Option<Window> {
    read()
}
pub fn session<'a, K>(window: &Window, sessions: &'a [(K, vults_protocol::Terminal)]) -> Option<&'a K> {
    if window.is_terminal() {
        return None;
    }
    let mut matches = sessions
        .iter()
        .filter(|(_, terminal)| terminal.pid == window.pid && window.pid.is_some());
    let (key, _) = matches.next()?;
    matches.next().is_none().then_some(key)
}
#[derive(Debug)]
pub struct Tracker(std::sync::atomic::AtomicBool);
impl Tracker {
    pub async fn start() -> Option<Self> {
        start().then(|| Self(std::sync::atomic::AtomicBool::new(true)))
    }
    pub fn snapshot(&self) -> Option<Window> {
        self.0.load(Ordering::SeqCst).then(read).flatten()
    }
    pub async fn close(&self) {
        self.0.store(false, Ordering::SeqCst);
    }
}
