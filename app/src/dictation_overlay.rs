//! Independent, click-through recording feedback; never moves the island or takes focus.

use std::sync::atomic::{AtomicBool, Ordering};

use tauri::{AppHandle, Emitter, Manager};

#[derive(Default)]
pub struct Ready(AtomicBool);

pub fn reset(app: &AppHandle) {
    if let Some(state) = app.try_state::<Ready>()
        && state.0.swap(false, Ordering::SeqCst)
    {
        let _ = app.emit("dictation-overlay-ready", false);
    }
}

pub const LABEL: &str = "dictation-overlay";
#[cfg(target_os = "linux")]
const SIZE: (i32, i32) = (480, 160);
#[cfg(target_os = "linux")]
const MARGIN: i32 = 12;

#[cfg(target_os = "linux")]
pub fn layer_spec() -> vults_platform::linux::LayerSpec {
    use vults_platform::linux::{Edges, LayerSpec};
    LayerSpec {
        namespace: format!("{}-dictation", vults_brand::SLUG),
        width: SIZE.0,
        height: SIZE.1,
        edges: Edges {
            bottom: true,
            ..Default::default()
        },
        margin: MARGIN,
        keyboard: false,
        overlay: true,
    }
}

#[cfg(target_os = "linux")]
pub fn build(app: &AppHandle) {
    use tauri::{WebviewUrl, WebviewWindowBuilder};
    let result = WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App("dictation.html".into()))
        .title(format!("{} dictation", vults_brand::NAME))
        .inner_size(f64::from(SIZE.0), f64::from(SIZE.1))
        .resizable(false)
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .visible_on_all_workspaces(true)
        .focused(false)
        .focusable(false)
        .visible(false)
        .maximizable(false)
        .minimizable(false)
        .closable(false)
        .on_page_load(|window, payload| {
            if payload.event() == tauri::webview::PageLoadEvent::Started {
                reset(window.app_handle());
            }
        })
        .build();
    let win = match result {
        Ok(win) => win,
        Err(error) => {
            tracing::warn!(%error, "dictation overlay unavailable; retaining inline feedback");
            return;
        }
    };
    let Ok(gtk) = win.gtk_window() else {
        let _ = win.destroy();
        return;
    };
    use vults_platform::linux::{init_layer, set_edges, set_input_region};
    if init_layer(
        &gtk,
        LABEL,
        &layer_spec(),
        crate::settings::monitor(app).as_deref(),
    ) {
        // Keep the indicator above the desktop panel without reserving screen space.
        set_edges(&gtk, layer_spec().edges, MARGIN, true);
    } else {
        place(&win);
        let _ = win.show();
    }
    // The entire surface is passive, including when the capsule is visible.
    set_input_region(&gtk, LABEL, None);
}

#[cfg(target_os = "linux")]
pub fn place(win: &tauri::WebviewWindow) {
    if let Ok(gtk) = win.gtk_window()
        && vults_platform::linux::is_layer(&gtk)
    {
        vults_platform::linux::place(&gtk, LABEL, crate::settings::monitor(win.app_handle()).as_deref());
        return;
    }
    if let Ok(Some(monitor)) = win.current_monitor() {
        let scale = monitor.scale_factor();
        let area = monitor.work_area();
        let size = area.size.to_logical::<f64>(scale);
        let origin = area.position.to_logical::<f64>(scale);
        let _ = win.set_position(tauri::LogicalPosition::new(
            origin.x + (size.width - f64::from(SIZE.0)) / 2.0,
            origin.y + size.height - f64::from(SIZE.1 + MARGIN),
        ));
    }
}

#[tauri::command]
pub fn dictation_overlay_ready(app: AppHandle) -> bool {
    app.try_state::<Ready>()
        .is_some_and(|state| state.0.load(Ordering::SeqCst))
}

#[tauri::command]
pub fn dictation_overlay_started(app: AppHandle, window: tauri::Webview) -> Result<(), String> {
    if window.label() != LABEL {
        return Err("only the dictation overlay can announce readiness".into());
    }
    let state = app.state::<Ready>();
    state.0.store(true, Ordering::SeqCst);
    let _ = app.emit("dictation-overlay-ready", true);
    Ok(())
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    #[test]
    fn bottom_feedback_never_requests_keyboard_or_stretched_edges() {
        let spec = super::layer_spec();
        assert!(spec.edges.bottom);
        assert!(!spec.edges.top && !spec.edges.left && !spec.edges.right);
        assert!(!spec.keyboard);
        assert_eq!((spec.width, spec.height), super::SIZE);
    }
}
