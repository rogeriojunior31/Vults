//! Explorer files reach WebView2 as native additional objects, never renderer-supplied paths.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

use tauri::{AppHandle, Emitter, Manager};
use webview2_com::{Microsoft::Web::WebView2::Win32::*, WebMessageReceivedEventHandler, take_pwstr};
use windows_core::{Interface, PWSTR};

const MESSAGE: &str = "vults-file-drop-v1";
const MAX_FILES: u32 = 256;

#[derive(Default)]
struct Ready(AtomicBool);

pub fn configure(config: &mut tauri::Config) {
    // Wry's OLE target prevents HTML file drops from classic Explorer folders.
    for window in &mut config.app.windows {
        if window.label == crate::ISLAND {
            window.drag_drop_enabled = false;
        }
    }
}

#[tauri::command]
pub fn file_drop_ready(app: AppHandle, webview: tauri::Webview) -> bool {
    webview.label() == crate::ISLAND && app.state::<Ready>().0.load(Ordering::Acquire)
}

#[tauri::command]
pub fn file_drop_hover(app: AppHandle, webview: tauri::Webview, over: bool) {
    if webview.label() == crate::ISLAND && !crate::lock::locked(&app) {
        crate::chat::on_drag(&app, over);
    }
}

fn trusted_source(source: &str, origin: &tauri::Url) -> bool {
    let Ok(source) = tauri::Url::parse(source) else {
        return false;
    };
    source.origin() == origin.origin()
        && source.username().is_empty()
        && source.password().is_none()
        && matches!(source.path(), "/" | "/index.html")
}

unsafe fn read_string(
    read: impl FnOnce(*mut PWSTR) -> windows_core::Result<()>,
) -> windows_core::Result<String> {
    let mut value = PWSTR::null();
    let result = read(&mut value);
    // WebView2 allocates returned strings with CoTaskMemAlloc, including empty values.
    let value = take_pwstr(value);
    result.map(|()| value)
}

unsafe fn paths(args: &ICoreWebView2WebMessageReceivedEventArgs) -> windows_core::Result<Vec<PathBuf>> {
    let objects = unsafe {
        args.cast::<ICoreWebView2WebMessageReceivedEventArgs2>()?
            .AdditionalObjects()?
    };
    unsafe { object_paths(&objects) }
}

unsafe fn object_paths(objects: &ICoreWebView2ObjectCollectionView) -> windows_core::Result<Vec<PathBuf>> {
    let mut count = 0;
    unsafe { objects.Count(&mut count)? };
    if count == 0 || count > MAX_FILES {
        return Err(windows_core::Error::from_hresult(windows_core::HRESULT(
            0x80070057_u32 as i32,
        )));
    }
    let mut paths = Vec::new();
    for index in 0..count {
        let file = unsafe { objects.GetValueAtIndex(index)? }.cast::<ICoreWebView2File>()?;
        let path = unsafe { read_string(|value| file.Path(value))? };
        let path = PathBuf::from(path);
        if !path.is_absolute() || path.as_os_str().len() > 32768 {
            return Err(windows_core::Error::from_hresult(windows_core::HRESULT(
                0x80070057_u32 as i32,
            )));
        }
        if !paths.contains(&path) {
            paths.push(path);
        }
    }
    Ok(paths)
}

pub fn install(app: &AppHandle) {
    app.manage(Ready::default());
    let Some(window) = app.get_webview_window(crate::ISLAND) else {
        return;
    };
    let origin = if tauri::is_dev() {
        app.config().build.dev_url.clone()
    } else {
        tauri::Url::parse("http://tauri.localhost").ok()
    };
    let Some(origin) = origin else { return };
    let app = app.clone();
    let result = window.with_webview(move |webview| {
        let installed = (|| -> windows_core::Result<()> {
            let core = unsafe { webview.controller().CoreWebView2()? };
            // AdditionalObjects requires a recent WebView2; keep drops disabled on older runtimes.
            core.cast::<ICoreWebView2_23>()?;
            let receiver = app.clone();
            let handler = WebMessageReceivedEventHandler::create(Box::new(move |_, args| {
                let Some(args) = args else { return Ok(()) };
                let message = unsafe { read_string(|value| args.TryGetWebMessageAsString(value)) };
                if message.as_deref() != Ok(MESSAGE) {
                    return Ok(());
                }
                let source = unsafe { read_string(|value| args.Source(value)) };
                if !source.is_ok_and(|source| trusted_source(&source, &origin))
                    || crate::lock::locked(&receiver)
                {
                    return Ok(());
                }
                crate::chat::on_drag(&receiver, false);
                match unsafe { paths(&args) } {
                    Ok(paths) => crate::chat::on_drop(&receiver, &paths, crate::ISLAND),
                    Err(_) => tracing::warn!("rejected invalid native file drop"),
                }
                Ok(())
            }));
            let mut token = 0;
            unsafe { core.add_WebMessageReceived(&handler, &mut token)? };
            Ok(())
        })();
        match installed {
            Ok(()) => {
                app.state::<Ready>().0.store(true, Ordering::Release);
                let _ = app.emit_to(crate::ISLAND, "file-drop-ready", true);
            }
            Err(error) => tracing::warn!(%error, "native file drop unavailable"),
        }
    });
    if let Err(error) = result {
        tracing::warn!(%error, "could not install native file drop");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[windows_core::implement(ICoreWebView2File)]
    struct FileObject(String);

    impl ICoreWebView2File_Impl for FileObject_Impl {
        fn Path(&self, value: *mut PWSTR) -> windows_core::Result<()> {
            unsafe { *value = webview2_com::pwstr_from_str(&self.0) };
            Ok(())
        }
    }

    #[windows_core::implement(ICoreWebView2ObjectCollectionView)]
    struct Objects {
        file_count: u32,
        files: Vec<windows_core::IUnknown>,
    }

    impl ICoreWebView2ObjectCollectionView_Impl for Objects_Impl {
        fn Count(&self, value: *mut u32) -> windows_core::Result<()> {
            unsafe { *value = self.file_count };
            Ok(())
        }

        fn GetValueAtIndex(&self, index: u32) -> windows_core::Result<windows_core::IUnknown> {
            Ok(self.files[index as usize].clone())
        }
    }

    fn collection(paths: &[&str]) -> ICoreWebView2ObjectCollectionView {
        Objects {
            file_count: paths.len() as u32,
            files: paths
                .iter()
                .map(|path| {
                    let file: ICoreWebView2File = FileObject((*path).into()).into();
                    file.cast().unwrap()
                })
                .collect(),
        }
        .into()
    }

    #[test]
    fn native_objects_are_bounded_absolute_and_deduplicated() {
        let objects = collection(&[
            r"C:\fixtures\one.txt",
            r"C:\fixtures\one.txt",
            r"C:\fixtures\two.txt",
        ]);
        let paths = unsafe { object_paths(&objects) }.unwrap();
        assert_eq!(paths.len(), 2);
        assert!(unsafe { object_paths(&collection(&["relative.txt"])) }.is_err());
        assert!(unsafe { object_paths(&collection(&[])) }.is_err());
        let oversized: ICoreWebView2ObjectCollectionView = Objects {
            file_count: MAX_FILES + 1,
            files: vec![],
        }
        .into();
        assert!(unsafe { object_paths(&oversized) }.is_err());
        let not_a_file: ICoreWebView2ObjectCollectionView = Objects {
            file_count: 1,
            files: vec![objects.cast().unwrap()],
        }
        .into();
        assert!(unsafe { object_paths(&not_a_file) }.is_err());
    }

    #[test]
    fn only_the_island_origin_and_document_accept_files() {
        let origin = tauri::Url::parse("http://localhost:1420").unwrap();
        for source in [
            "http://localhost:1420/",
            "http://localhost:1420/index.html?test=1",
        ] {
            assert!(trusted_source(source, &origin));
        }
        for source in [
            "https://localhost:1420/",
            "http://localhost:1421/",
            "http://localhost:1420/nest.html",
            "http://user@localhost:1420/",
            "http://example.com/",
            "invalid",
        ] {
            assert!(!trusted_source(source, &origin));
        }
    }

    #[test]
    fn native_drag_is_disabled_only_on_the_island() {
        let mut config = tauri::Config::default();
        for label in [crate::ISLAND, crate::NEST] {
            let window = tauri::utils::config::WindowConfig {
                label: label.into(),
                ..Default::default()
            };
            config.app.windows.push(window);
        }
        configure(&mut config);
        assert!(!config.app.windows[0].drag_drop_enabled);
        assert!(config.app.windows[1].drag_drop_enabled);
    }
}
