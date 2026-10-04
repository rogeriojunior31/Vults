// No console window on Windows release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    #[cfg(target_os = "linux")]
    lean_webview();
    #[cfg(target_os = "linux")]
    own_gst_registry();
    vultures_ai_app::run();
}

/// WebKitGTK's GPU compositing allocates buffers a small island never needs: software rendering
/// measured 44 MB less and less CPU (scripts/perf.py). `VULTURES_AI_GPU=1` keeps the GPU path;
/// a value the user already set for WebKit wins.
#[cfg(target_os = "linux")]
fn lean_webview() {
    const VAR: &str = "WEBKIT_DISABLE_COMPOSITING_MODE";
    if std::env::var_os("VULTURES_AI_GPU").is_some() || std::env::var_os(VAR).is_some() {
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
    let Some(dir) = cache.map(|c| c.join(vultures_ai_brand::SLUG)) else {
        return;
    };
    if std::fs::create_dir_all(&dir).is_ok() {
        // SAFETY: first thing in main, before Tauri, GTK or any other thread exists.
        unsafe { std::env::set_var("GST_REGISTRY", dir.join("gstreamer-registry.bin")) };
    }
}
