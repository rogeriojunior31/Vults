// No console window on Windows release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    #[cfg(target_os = "linux")]
    lean_webview();
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
