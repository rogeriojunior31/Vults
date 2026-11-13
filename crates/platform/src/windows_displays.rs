//! The monitors and their work areas: a screen plugged in or out, or a taskbar moved or resized,
//! changes where the surfaces belong. Windows announces it only to window procedures, which Tauri
//! owns, so the layout is read again every two seconds: one enumeration, no window of ours.
use std::time::Duration;

use windows::Win32::Foundation::{LPARAM, RECT};
use windows::Win32::Graphics::Gdi::{EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFO};
use windows::core::BOOL;

/// Each monitor's bounds and work area, in the order Windows lists them.
pub fn layout() -> Vec<[i32; 8]> {
    unsafe extern "system" fn each(monitor: HMONITOR, _: HDC, _: *mut RECT, data: LPARAM) -> BOOL {
        // SAFETY: `data` is the Vec `layout` passed, alive for the whole enumeration.
        let out = unsafe { &mut *(data.0 as *mut Vec<[i32; 8]>) };
        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        // SAFETY: `info` has its required size; the monitor handle comes from the enumeration.
        if unsafe { GetMonitorInfoW(monitor, &mut info) }.as_bool() {
            let (m, w) = (info.rcMonitor, info.rcWork);
            out.push([m.left, m.top, m.right, m.bottom, w.left, w.top, w.right, w.bottom]);
        }
        true.into()
    }
    let mut out: Vec<[i32; 8]> = Vec::new();
    // SAFETY: the callback only writes into `out`, which outlives the call.
    unsafe {
        let _ = EnumDisplayMonitors(None, None, Some(each), LPARAM(&mut out as *mut _ as isize));
    }
    out
}

/// Calls `changed` from a thread of its own each time the layout differs from the last one read.
pub fn on_monitors_changed(changed: impl Fn() + Send + 'static) {
    std::thread::spawn(move || {
        let mut last = layout();
        loop {
            std::thread::sleep(Duration::from_secs(2));
            let now = layout();
            // An empty read (a session switching, the screen off) is no change: keep the last.
            if !now.is_empty() && now != last {
                last = now;
                changed();
            }
        }
    });
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_desktop_has_a_monitor_with_its_work_area_inside() {
        let layout = super::layout();
        assert!(!layout.is_empty());
        for [l, t, r, b, wl, wt, wr, wb] in layout {
            assert!(l < r && t < b);
            assert!(
                wl >= l && wt >= t && wr <= r && wb <= b,
                "work area inside the monitor"
            );
        }
    }
}
