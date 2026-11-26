//! The monitors and their work areas: a screen plugged in or out, or a taskbar moved or resized,
//! changes where the surfaces belong. Windows announces it only to window procedures, which Tauri
//! owns, so the layout is read again every two seconds: one enumeration, no window of ours.
use std::time::Duration;

use windows::Win32::Foundation::{LPARAM, RECT};
use windows::Win32::Graphics::Gdi::{EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFO};
use windows::core::BOOL;

/// Each active monitor's name for people (`DELL U2720Q`), keyed by the device name Tauri gives
/// (`\\.\DISPLAY1`); a monitor with no name of its own (an internal panel) is left out.
pub fn friendly_names() -> std::collections::HashMap<String, String> {
    use windows::Win32::Devices::Display::{
        DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME, DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME,
        DISPLAYCONFIG_MODE_INFO, DISPLAYCONFIG_PATH_INFO, DISPLAYCONFIG_SOURCE_DEVICE_NAME,
        DISPLAYCONFIG_TARGET_DEVICE_NAME, DisplayConfigGetDeviceInfo, GetDisplayConfigBufferSizes,
        QDC_ONLY_ACTIVE_PATHS, QueryDisplayConfig,
    };
    let text = |wide: &[u16]| {
        let len = wide.iter().position(|&c| c == 0).unwrap_or(wide.len());
        String::from_utf16_lossy(&wide[..len]).trim().to_owned()
    };
    let mut names = std::collections::HashMap::new();
    let (mut paths, mut modes) = (0u32, 0u32);
    // SAFETY (whole body): buffers sized by GetDisplayConfigBufferSizes, packets of the documented
    // layout with their own size in their header.
    unsafe {
        if GetDisplayConfigBufferSizes(QDC_ONLY_ACTIVE_PATHS, &mut paths, &mut modes).is_err() {
            return names;
        }
        let mut path_info = vec![DISPLAYCONFIG_PATH_INFO::default(); paths as usize];
        let mut mode_info = vec![DISPLAYCONFIG_MODE_INFO::default(); modes as usize];
        if QueryDisplayConfig(
            QDC_ONLY_ACTIVE_PATHS,
            &mut paths,
            path_info.as_mut_ptr(),
            &mut modes,
            mode_info.as_mut_ptr(),
            None,
        )
        .is_err()
        {
            return names;
        }
        for path in path_info.iter().take(paths as usize) {
            let mut source = DISPLAYCONFIG_SOURCE_DEVICE_NAME::default();
            source.header.r#type = DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME;
            source.header.size = std::mem::size_of::<DISPLAYCONFIG_SOURCE_DEVICE_NAME>() as u32;
            source.header.adapterId = path.sourceInfo.adapterId;
            source.header.id = path.sourceInfo.id;
            let mut target = DISPLAYCONFIG_TARGET_DEVICE_NAME::default();
            target.header.r#type = DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME;
            target.header.size = std::mem::size_of::<DISPLAYCONFIG_TARGET_DEVICE_NAME>() as u32;
            target.header.adapterId = path.targetInfo.adapterId;
            target.header.id = path.targetInfo.id;
            if DisplayConfigGetDeviceInfo(&mut source.header) != 0
                || DisplayConfigGetDeviceInfo(&mut target.header) != 0
            {
                continue;
            }
            let friendly = text(&target.monitorFriendlyDeviceName);
            if !friendly.is_empty() {
                names.insert(text(&source.viewGdiDeviceName), friendly);
            }
        }
    }
    names
}

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

/// Whether the taskbar sits at the top of its screen (Windows 10 allows it); none when unknown.
pub fn taskbar_at_top() -> Option<bool> {
    use windows::Win32::UI::Shell::{ABE_TOP, ABM_GETTASKBARPOS, APPBARDATA, SHAppBarMessage};
    let mut data = APPBARDATA {
        cbSize: std::mem::size_of::<APPBARDATA>() as u32,
        ..Default::default()
    };
    // SAFETY: `data` has its required size; the call only fills it in.
    let found = unsafe { SHAppBarMessage(ABM_GETTASKBARPOS, &mut data) };
    (found != 0).then_some(data.uEdge == ABE_TOP)
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
    fn monitor_names_are_keyed_by_device_name() {
        for (device, friendly) in super::friendly_names() {
            assert!(device.starts_with(r"\\.\DISPLAY"), "{device}");
            assert!(!friendly.is_empty());
        }
    }

    #[test]
    fn the_taskbar_edge_is_read() {
        // This desktop has a taskbar; which edge is the user's choice.
        assert!(super::taskbar_at_top().is_some());
    }

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
