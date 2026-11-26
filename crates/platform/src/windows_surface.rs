//! Clip transparent desktop surfaces to the rectangle the renderer actually draws.
use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Gdi::{CreateRectRgn, DeleteObject, SetWindowRgn};

fn physical_rect(rect: (i32, i32, i32, i32), scale: f64) -> Option<[i32; 4]> {
    if !scale.is_finite() || scale <= 0.0 {
        return None;
    }
    let (x, y, width, height) = rect;
    if width <= 0 || height <= 0 {
        return Some([0; 4]);
    }
    Some([
        (f64::from(x) * scale).floor() as i32,
        (f64::from(y) * scale).floor() as i32,
        ((f64::from(x) + f64::from(width)) * scale).ceil() as i32,
        ((f64::from(y) + f64::from(height)) * scale).ceil() as i32,
    ])
}

/// `handle` is a live Tauri HWND, borrowed for this call on its UI thread.
pub fn set_region(handle: isize, rect: (i32, i32, i32, i32), scale: f64) -> bool {
    let Some([left, top, right, bottom]) = physical_rect(rect, scale) else {
        return false;
    };
    // SAFETY: GDI owns the new region until SetWindowRgn succeeds, then Windows owns it.
    unsafe {
        let region = CreateRectRgn(left, top, right, bottom);
        if region.is_invalid() {
            return false;
        }
        if SetWindowRgn(HWND(handle as *mut _), Some(region), true) == 0 {
            let _ = DeleteObject(region.into());
            return false;
        }
    }
    true
}

/// Makes a surface a tool window: on every virtual desktop (Tauri's `visible_on_all_workspaces`
/// does nothing on Windows, so the island stayed behind on the desktop it started on), and out
/// of Alt+Tab and Task View. `handle` is a live HWND, on its UI thread.
pub fn set_tool_window(handle: isize) {
    use windows::Win32::UI::WindowsAndMessaging::{
        GWL_EXSTYLE, GetWindowLongPtrW, SetWindowLongPtrW, WS_EX_APPWINDOW, WS_EX_TOOLWINDOW,
    };
    // SAFETY: style bits of a window this process owns, read then written on its thread.
    unsafe {
        let hwnd = HWND(handle as *mut _);
        let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        let style = (style | WS_EX_TOOLWINDOW.0 as isize) & !(WS_EX_APPWINDOW.0 as isize);
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, style);
    }
}

/// The window in front before `own` took the keyboard: none when `own` already was.
pub fn foreground_before(own: isize) -> Option<isize> {
    // SAFETY: a read of the current foreground window's handle.
    let front = unsafe { windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow() };
    (!front.is_invalid() && front.0 as isize != own).then_some(front.0 as isize)
}

/// Gives the keyboard back to `previous` when `own` still holds it, as Linux does by dropping
/// keyboard interactivity: without it the next keys go to an island that no longer wants them.
/// A window closed since, or one the user moved to meanwhile, is left alone.
pub fn give_back(own: isize, previous: isize) {
    use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, IsWindow, SetForegroundWindow};
    // SAFETY: handles are only compared and checked live before use; Windows refuses the call if
    // this process is no longer in front.
    unsafe {
        let previous = HWND(previous as *mut _);
        if GetForegroundWindow().0 as isize == own && IsWindow(Some(previous)).as_bool() {
            let _ = SetForegroundWindow(previous);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn region_keeps_fractional_scale_edges_and_hides_empty_surfaces() {
        for (scale, expected) in [
            (1.0, [3, 5, 14, 14]),
            (1.25, [3, 6, 18, 18]),
            (1.5, [4, 7, 21, 21]),
            (1.75, [5, 8, 25, 25]),
            (2.0, [6, 10, 28, 28]),
        ] {
            assert_eq!(physical_rect((3, 5, 11, 9), scale), Some(expected));
        }
        assert_eq!(physical_rect((-3, -5, 11, 9), 1.5), Some([-5, -8, 12, 6]));
        assert_eq!(physical_rect((100, 200, 0, 20), 2.0), Some([0; 4]));
        assert_eq!(physical_rect((0, 0, 20, 20), f64::NAN), None);
    }
}
