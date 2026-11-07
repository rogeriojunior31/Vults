//! Resolve a session's same-user ancestor window; never type or launch a terminal.
use std::collections::{HashMap, HashSet};
use vults_protocol::Terminal;
use windows::Win32::Foundation::{CloseHandle, HWND, LPARAM, WPARAM};
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW, TH32CS_SNAPPROCESS,
};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GW_OWNER, GetClassNameW, GetForegroundWindow, GetWindow, GetWindowThreadProcessId, IsIconic,
    IsWindow, IsWindowVisible, SMTO_ABORTIFHUNG, SMTO_BLOCK, SW_RESTORE, SendMessageTimeoutW,
    SetForegroundWindow, ShowWindowAsync,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Anchor {
    pid: u32,
    birth: u64,
}
impl Anchor {
    /// Freeze at the native session event, not when a later click resolves a window.
    pub fn capture(pid: u32) -> Option<Self> {
        Some(Self {
            pid,
            birth: vults_peer::process_birth(pid)??,
        })
    }
    fn current(self) -> bool {
        vults_peer::process_birth(self.pid) == Some(Some(self.birth))
    }
}
#[derive(Clone, Debug)]
struct Window {
    handle: isize,
    pid: u32,
    birth: u64,
    class: String,
    title: String,
}

fn parents() -> Option<HashMap<u32, (u32, String)>> {
    // SAFETY: query-only snapshot, bounded enumeration, closed on every success path.
    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0).ok()?;
        let mut entry = PROCESSENTRY32W {
            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };
        let mut rows = HashMap::new();
        let mut next = Process32FirstW(snapshot, &mut entry);
        while next.is_ok() && rows.len() < 65_536 {
            let end = entry
                .szExeFile
                .iter()
                .position(|c| *c == 0)
                .unwrap_or(entry.szExeFile.len());
            rows.insert(
                entry.th32ProcessID,
                (
                    entry.th32ParentProcessID,
                    String::from_utf16_lossy(&entry.szExeFile[..end]),
                ),
            );
            next = Process32NextW(snapshot, &mut entry);
        }
        let _ = CloseHandle(snapshot);
        Some(rows)
    }
}
fn chain(anchor: Anchor, rows: &HashMap<u32, (u32, String)>) -> Vec<(u32, u64)> {
    chain_with(
        anchor,
        rows,
        |pid| vults_peer::process_birth(pid).flatten(),
        std::process::id(),
    )
}
fn chain_with(
    anchor: Anchor,
    rows: &HashMap<u32, (u32, String)>,
    birth_of: impl Fn(u32) -> Option<u64>,
    own_pid: u32,
) -> Vec<(u32, u64)> {
    if birth_of(anchor.pid) != Some(anchor.birth) {
        return Vec::new();
    }
    let mut result = Vec::new();
    let mut seen = HashSet::new();
    let (mut pid, mut birth) = (anchor.pid, anchor.birth);
    for _ in 0..32 {
        let Some((parent, exe)) = rows.get(&pid) else {
            break;
        };
        if !seen.insert(pid) || system_root(exe) || pid == own_pid {
            break;
        }
        result.push((pid, birth));
        let Some(parent_birth) = birth_of(*parent) else {
            break;
        };
        if parent_birth > birth || *parent == pid {
            break;
        }
        (pid, birth) = (*parent, parent_birth);
    }
    result
}
fn system_root(exe: &str) -> bool {
    matches!(
        exe.to_ascii_lowercase().as_str(),
        "explorer.exe"
            | "services.exe"
            | "wininit.exe"
            | "winlogon.exe"
            | "svchost.exe"
            | "smss.exe"
            | "csrss.exe"
            | "system"
            | "sihost.exe"
            | "userinit.exe"
    )
}
fn caption(hwnd: HWND) -> Option<String> {
    let mut text = [0u16; 1024];
    let mut count = 0;
    // SAFETY: WM_GETTEXT is system-marshaled; the buffer outlives this bounded call.
    let sent = unsafe {
        SendMessageTimeoutW(
            hwnd,
            0x000d,
            WPARAM(text.len()),
            LPARAM(text.as_mut_ptr() as isize),
            SMTO_ABORTIFHUNG | SMTO_BLOCK,
            75,
            Some(&mut count),
        )
    };
    (sent.0 != 0 && count < text.len() - 1).then(|| String::from_utf16_lossy(&text[..count]))
}
fn class(hwnd: HWND) -> String {
    let mut text = [0u16; 256];
    // SAFETY: native metadata only, bounded output buffer.
    let count = unsafe { GetClassNameW(hwnd, &mut text) };
    String::from_utf16_lossy(&text[..count.max(0) as usize])
}
fn windows(owners: &[(u32, u64)]) -> Vec<Window> {
    struct Query<'a> {
        owners: &'a [(u32, u64)],
        found: Vec<Window>,
        started: std::time::Instant,
        truncated: bool,
    }
    unsafe extern "system" fn collect(hwnd: HWND, param: LPARAM) -> windows::core::BOOL {
        // SAFETY: EnumWindows borrows this query synchronously; no callback retains the pointer.
        let query = unsafe { &mut *(param.0 as *mut Query<'_>) };
        if query.found.len() >= 128 || query.started.elapsed() > std::time::Duration::from_millis(250) {
            query.truncated = true;
            return false.into();
        }
        let mut pid = 0;
        // SAFETY: query-only APIs; unowned visible top-level windows only.
        unsafe {
            GetWindowThreadProcessId(hwnd, Some(&mut pid));
            let Some((_, birth)) = query.owners.iter().find(|(owner, _)| *owner == pid) else {
                return true.into();
            };
            if !IsWindowVisible(hwnd).as_bool()
                || GetWindow(hwnd, GW_OWNER).is_ok_and(|owner| !owner.0.is_null())
            {
                return true.into();
            }
            query.found.push(Window {
                handle: hwnd.0 as isize,
                pid,
                birth: *birth,
                class: class(hwnd),
                title: caption(hwnd).unwrap_or_default(),
            });
        }
        true.into()
    }
    let mut query = Query {
        owners,
        found: Vec::new(),
        started: std::time::Instant::now(),
        truncated: false,
    };
    // SAFETY: the callback and query live for the synchronous enumeration.
    let _ = unsafe { EnumWindows(Some(collect), LPARAM(&mut query as *mut _ as isize)) };
    if query.truncated { Vec::new() } else { query.found }
}
fn named(title: &str, folder: &str) -> bool {
    if folder.is_empty() {
        return false;
    }
    title.match_indices(folder).any(|(at, _)| {
        let word = |c: char| c.is_alphanumeric() || c == '_' || c == '-';
        title[..at].chars().next_back().is_none_or(|c| !word(c))
            && title[at + folder.len()..].chars().next().is_none_or(|c| !word(c))
    })
}
fn choose<'a>(owners: &[(u32, u64)], windows: &'a [Window], cwd: Option<&str>) -> Option<&'a Window> {
    let folder = cwd
        .unwrap_or("")
        .trim_end_matches(['/', '\\'])
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or("")
        .to_lowercase();
    for (pid, birth) in owners {
        let own: Vec<_> = windows
            .iter()
            .filter(|w| w.pid == *pid && w.birth == *birth)
            .collect();
        if own.is_empty() {
            continue;
        }
        if own
            .iter()
            .any(|window| window.title.is_empty() || window.class.is_empty())
        {
            return None;
        }
        if own.len() == 1 {
            return Some(own[0]);
        }
        let matched: Vec<_> = own
            .into_iter()
            .filter(|w| named(&w.title.to_lowercase(), &folder))
            .collect();
        // A shared editor process with multiple matching windows is not enough authority.
        return (matched.len() == 1).then(|| matched[0]);
    }
    None
}
fn target(anchor: Anchor, terminal: &Terminal) -> Option<Window> {
    if terminal.pid != Some(anchor.pid) {
        return None;
    }
    let owners = chain(anchor, &parents()?);
    let windows = windows(&owners);
    choose(&owners, &windows, terminal.cwd.as_deref()).cloned()
}
pub fn available(anchor: Anchor, terminal: &Terminal) -> bool {
    crate::windows_session::locked() == Some(false) && target(anchor, terminal).is_some()
}
/// One process/window snapshot for a whole UI refresh, including when the island is hidden.
pub fn availability(candidates: &[(Anchor, Terminal)]) -> Vec<bool> {
    let unavailable = || vec![false; candidates.len()];
    if crate::windows_session::locked() != Some(false) {
        return unavailable();
    }
    let Some(rows) = parents() else {
        return unavailable();
    };
    let chains: Vec<_> = candidates
        .iter()
        .map(|(anchor, terminal)| {
            if terminal.pid == Some(anchor.pid) {
                chain(*anchor, &rows)
            } else {
                Vec::new()
            }
        })
        .collect();
    let owners: HashSet<_> = chains.iter().flatten().copied().collect();
    let windows = windows(&owners.into_iter().collect::<Vec<_>>());
    candidates
        .iter()
        .zip(chains)
        .map(|((_, terminal), chain)| choose(&chain, &windows, terminal.cwd.as_deref()).is_some())
        .collect()
}
pub fn jump_if(anchor: Anchor, terminal: &Terminal, allowed: &dyn Fn() -> bool) -> bool {
    if !allowed() || crate::windows_session::locked() != Some(false) {
        return false;
    }
    let Some(target) = target(anchor, terminal) else {
        return false;
    };
    let hwnd = HWND(target.handle as *mut _);
    let current = || {
        let mut pid = 0;
        // SAFETY: the handle is revalidated before activation, including owner creation time.
        unsafe {
            GetWindowThreadProcessId(hwnd, Some(&mut pid));
        }
        anchor.current()
            && pid == target.pid
            && vults_peer::process_birth(pid) == Some(Some(target.birth))
            && unsafe { IsWindow(Some(hwnd)).as_bool() }
            && class(hwnd) == target.class
    };
    if !allowed() || !current() {
        return false;
    }
    // No input-queue attachment: a stalled target must not join the app's input queue.
    unsafe {
        if IsIconic(hwnd).as_bool() {
            let _ = ShowWindowAsync(hwnd, SW_RESTORE);
        }
        if !allowed() || !current() || crate::windows_session::locked() != Some(false) {
            return false;
        }
        let _ = SetForegroundWindow(hwnd);
    }
    for _ in 0..20 {
        if !allowed() || !current() {
            return false;
        }
        if unsafe { GetForegroundWindow() } == hwnd {
            return true;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    fn window(handle: isize, pid: u32, title: &str) -> Window {
        Window {
            handle,
            pid,
            birth: 10,
            class: "fixture".into(),
            title: title.into(),
        }
    }
    #[test]
    fn process_chain_rejects_recycled_parents_cycles_and_desktop_roots() {
        let mut rows = HashMap::from([
            (10, (20, "agent.exe".into())),
            (20, (30, "pwsh.exe".into())),
            (30, (40, "WindowsTerminal.exe".into())),
            (40, (0, "explorer.exe".into())),
        ]);
        let births = HashMap::from([(10, 100), (20, 90), (30, 80), (40, 70)]);
        let anchor = Anchor { pid: 10, birth: 100 };
        assert_eq!(
            chain_with(anchor, &rows, |pid| births.get(&pid).copied(), 99),
            vec![(10, 100), (20, 90), (30, 80)]
        );
        assert_eq!(
            chain_with(
                anchor,
                &rows,
                |pid| if pid == 20 {
                    Some(110)
                } else {
                    births.get(&pid).copied()
                },
                99
            ),
            vec![(10, 100)]
        );
        assert!(
            chain_with(
                Anchor { birth: 101, ..anchor },
                &rows,
                |pid| births.get(&pid).copied(),
                99
            )
            .is_empty()
        );
        rows.get_mut(&30).unwrap().0 = 20;
        assert_eq!(chain_with(anchor, &rows, |_| Some(100), 99).len(), 3);
        assert!(chain_with(anchor, &rows, |pid| births.get(&pid).copied(), 10).is_empty());
    }
    #[test]
    fn closest_owner_and_unique_folder_match_are_required() {
        let rows = vec![
            window(1, 20, "Alpha — editor"),
            window(2, 20, "Beta — editor"),
            window(3, 30, "Alpha"),
        ];
        assert_eq!(
            choose(&[(20, 10), (30, 10)], &rows, Some("C:\\work\\Alpha"))
                .unwrap()
                .handle,
            1
        );
        assert!(choose(&[(20, 10), (30, 10)], &rows, Some("C:\\work\\Other")).is_none());
        assert!(choose(&[(20, 11)], &rows, Some("Alpha")).is_none());
        let rows = vec![window(1, 20, "Alpha"), window(2, 20, "Alpha")];
        assert!(choose(&[(20, 10)], &rows, Some("Alpha")).is_none());
        assert!(!named("alphabet — editor", "alpha"));
        assert!(named("editor — 项目", "项目"));
        assert!(system_root("Explorer.EXE"));
    }
    #[test]
    fn recycled_or_unreadable_processes_and_cancelled_actions_do_not_activate() {
        let mut anchor = Anchor::capture(std::process::id()).unwrap();
        anchor.birth += 1;
        assert!(!available(
            anchor,
            &Terminal {
                pid: Some(anchor.pid),
                ..Default::default()
            }
        ));
        assert!(!jump_if(anchor, &Terminal::default(), &|| false));
        assert!(Anchor::capture(u32::MAX).is_none());
    }
    #[test]
    fn native_caption_is_bounded_unicode_metadata_and_dead_handles_are_refused() {
        use windows::Win32::UI::WindowsAndMessaging::{
            CreateWindowExW, DestroyWindow, WINDOW_EX_STYLE, WS_POPUP,
        };
        // SAFETY: a hidden built-in fixture owned by this test; no foreground change or user input.
        let hwnd = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE(0),
                windows::core::w!("STATIC"),
                windows::core::w!("Vults fixture \u{e7}\u{4e2d}"),
                WS_POPUP,
                0,
                0,
                100,
                60,
                None,
                None,
                None,
                None,
            )
        }
        .unwrap();
        assert_eq!(caption(hwnd).as_deref(), Some("Vults fixture \u{e7}\u{4e2d}"));
        assert_eq!(class(hwnd), "Static");
        unsafe {
            DestroyWindow(hwnd).unwrap();
        }
        assert!(caption(hwnd).is_none());
    }
    #[test]
    #[ignore = "subprocess fixture for native_window_discovery_tracks_the_actual_owner"]
    fn window_fixture() {
        if std::env::var("VULTS_TERMINAL_FIXTURE").as_deref() != Ok("1") {
            return;
        }
        use std::io::Write;
        use windows::Win32::UI::WindowsAndMessaging::{
            CreateWindowExW, DispatchMessageW, GetMessageW, MSG, TranslateMessage, WS_EX_NOACTIVATE,
            WS_EX_TOOLWINDOW, WS_POPUP, WS_VISIBLE,
        };
        // SAFETY: an off-screen, non-activating fixture with no taskbar entry or user content.
        let _window = unsafe {
            CreateWindowExW(
                WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW,
                windows::core::w!("STATIC"),
                windows::core::w!("Vults session window fixture"),
                WS_POPUP | WS_VISIBLE,
                100_000,
                100_000,
                80,
                40,
                None,
                None,
                None,
                None,
            )
        }
        .unwrap();
        println!("VULTS_WINDOW_READY");
        std::io::stdout().flush().unwrap();
        let mut message = MSG::default();
        while unsafe { GetMessageW(&mut message, None, 0, 0) }.0 > 0 {
            unsafe {
                let _ = TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
    }
    #[test]
    fn native_window_discovery_tracks_the_actual_owner() {
        use std::io::BufRead;
        use std::os::windows::process::CommandExt;
        use std::process::{Child, Command, Stdio};
        struct Fixture(Child);
        impl Drop for Fixture {
            fn drop(&mut self) {
                let _ = self.0.kill();
                let _ = self.0.wait();
            }
        }
        let mut fixture = Fixture(
            Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "jump::tests::window_fixture",
                    "--ignored",
                    "--nocapture",
                ])
                .env("VULTS_TERMINAL_FIXTURE", "1")
                .creation_flags(0x08000000)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .unwrap(),
        );
        let stdout = fixture.0.stdout.take().unwrap();
        let (send, receive) = std::sync::mpsc::sync_channel(1);
        std::thread::spawn(move || {
            for line in std::io::BufReader::new(stdout).lines().map_while(Result::ok) {
                if line == "VULTS_WINDOW_READY" {
                    let _ = send.send(());
                    return;
                }
            }
        });
        receive
            .recv_timeout(std::time::Duration::from_secs(5))
            .expect("native fixture ready");
        let anchor = Anchor::capture(fixture.0.id()).unwrap();
        let terminal = Terminal {
            pid: Some(fixture.0.id()),
            ..Default::default()
        };
        let found = target(anchor, &terminal).expect("the actual child window, not a parent application");
        assert_eq!(found.pid, fixture.0.id());
        assert_eq!(found.title, "Vults session window fixture");
        fixture.0.kill().unwrap();
        fixture.0.wait().unwrap();
        assert!(target(anchor, &terminal).is_none());
    }
}
