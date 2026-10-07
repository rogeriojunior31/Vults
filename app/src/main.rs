// No console window on Windows release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    #[cfg(target_os = "linux")]
    lean_webview();
    #[cfg(target_os = "linux")]
    own_gst_registry();
    vults_app::run();
}

/// WebKitGTK's software path (no compositing) used 44 MB less and less CPU (scripts/perf.py), but
/// since WebKitGTK 2.54 it repaints a transparent layer surface only where something moved: the
/// island showed Zeca and stray text with no card behind them. Compositing stays on;
/// `VULTS_LEAN=1` asks for the software path, and a value the user set for WebKit wins.
#[cfg(target_os = "linux")]
fn lean_webview() {
    const VAR: &str = "WEBKIT_DISABLE_COMPOSITING_MODE";
    if std::env::var_os("VULTS_LEAN").is_none() || std::env::var_os(VAR).is_some() {
        return;
    }
    // SAFETY: first thing in main, before Tauri, GTK or any other thread exists.
    unsafe { std::env::set_var(VAR, "1") };
}

/// Inside the AppImage, WebKit's bundled GStreamer would keep its plugin registry in the system's
/// `~/.cache/gstreamer-1.0`, and each launch would rewrite it with plugin paths from a mount that
/// vanishes on exit. Give ours its own file. A registry the user set wins.
#[cfg(target_os = "linux")]
fn own_gst_registry() {
    if std::env::var_os("APPIMAGE").is_none() || std::env::var_os("GST_REGISTRY").is_some() {
        return;
    }
    let cache = std::env::var_os("XDG_CACHE_HOME")
        .map(std::path::PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".cache")));
    let Some(dir) = cache.map(|c| c.join(vults_brand::SLUG)) else {
        return;
    };
    if std::fs::create_dir_all(&dir).is_ok() {
        // SAFETY: first thing in main, before Tauri, GTK or any other thread exists.
        unsafe { std::env::set_var("GST_REGISTRY", dir.join("gstreamer-registry.bin")) };
    }
}
