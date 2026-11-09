//! Registered Windows launchers. Speech selects a name, never a command line.
mod packaged;
use super::Application;
use std::{
    fs::{self, File, OpenOptions},
    io::Read,
    os::windows::fs::{MetadataExt, OpenOptionsExt},
    path::{Component, Path, PathBuf, Prefix},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};
use windows::{
    Win32::{
        Storage::FileSystem::{FILE_ATTRIBUTE_REPARSE_POINT, FILE_FLAG_OPEN_REPARSE_POINT, FILE_SHARE_READ},
        System::Com::{
            CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx, CoTaskMemFree,
            CoUninitialize, IPersistStream,
        },
        UI::Shell::{
            FOLDERID_CommonPrograms, FOLDERID_Programs, IShellLinkW, KF_FLAG_DEFAULT, SEE_MASK_FLAG_NO_UI,
            SEE_MASK_NOASYNC, SHCreateMemStream, SHELLEXECUTEINFOW, SHGetKnownFolderPath, SLGP_RAWPATH,
            ShellExecuteExW, ShellLink,
        },
    },
    core::{HSTRING, Interface, PCWSTR},
};

const LIMIT: usize = 65_536;
const BUDGET: Duration = Duration::from_secs(2);
const CHANGED: &str = "The installed application changed or is unavailable.";
const FAILED: &str = "Could not launch the installed application.";
struct Apartment;
impl Drop for Apartment {
    fn drop(&mut self) {
        unsafe { CoUninitialize() };
    }
}
struct Admission(Arc<AtomicBool>);
impl Drop for Admission {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

fn worker<T: Send + 'static>(
    job: impl FnOnce(&dyn Fn() -> bool) -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    let active = Admission(Arc::new(AtomicBool::new(true)));
    let live = active.0.clone();
    let deadline = Instant::now() + BUDGET - Duration::from_millis(100);
    let (send, receive) = mpsc::sync_channel(1);
    std::thread::Builder::new()
        .name("voice-applications".into())
        .spawn(move || {
            let allowed = || live.load(Ordering::Acquire) && Instant::now() < deadline;
            if unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) }.is_err() {
                let _ = send.send(Err(FAILED.into()));
                return;
            }
            let _apartment = Apartment;
            let result = if allowed() {
                job(&allowed)
            } else {
                Err(FAILED.into())
            };
            let _ = send.send(result);
        })
        .map_err(|_| FAILED)?;
    receive
        .recv_timeout(BUDGET)
        .map_err(|_| "The Windows application service timed out.".to_string())?
}

fn roots() -> Vec<PathBuf> {
    [FOLDERID_Programs, FOLDERID_CommonPrograms]
        .into_iter()
        .filter_map(|id| unsafe {
            let raw = SHGetKnownFolderPath(&id, KF_FLAG_DEFAULT, None).ok()?;
            let path = raw.to_string().ok().map(PathBuf::from);
            CoTaskMemFree(Some(raw.0.cast()));
            path.filter(|path| ordinary(path))
                .and_then(|path| canonical_local(&path))
        })
        .collect()
}
fn local(path: &Path) -> bool {
    path.is_absolute()
        && matches!(path.components().next(), Some(Component::Prefix(prefix))
        if matches!(prefix.kind(), Prefix::Disk(_) | Prefix::VerbatimDisk(_)))
        && !path.components().any(|part| matches!(part, Component::ParentDir))
}
fn ordinary(path: &Path) -> bool {
    fs::symlink_metadata(path)
        .is_ok_and(|metadata| metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT.0 == 0)
}
fn canonical_local(path: &Path) -> Option<PathBuf> {
    if !local(path) {
        return None;
    }
    let resolved = fs::canonicalize(path).ok()?;
    if !local(&resolved) {
        return None;
    }
    let text = resolved.to_str()?;
    Some(PathBuf::from(text.strip_prefix(r"\\?\").unwrap_or(text)))
}
fn source(root: &Path, path: &Path) -> Option<String> {
    let relative = path.strip_prefix(root).ok()?;
    let mut cursor = root.to_path_buf();
    for component in relative.components() {
        let Component::Normal(name) = component else {
            return None;
        };
        cursor.push(name);
        if !ordinary(&cursor) {
            return None;
        }
    }
    (path.extension()?.eq_ignore_ascii_case("lnk") && !relative.as_os_str().is_empty())
        .then(|| relative.to_string_lossy().into_owned())
}
fn read(path: &Path) -> Option<(File, Vec<u8>)> {
    // Keep writers and replacement denied while the snapshot is checked and submitted.
    let file = OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ.0)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT.0)
        .open(path)
        .ok()?;
    let metadata = file.metadata().ok()?;
    if !metadata.is_file() || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT.0 != 0 {
        return None;
    }
    let mut bytes = Vec::new();
    (&file).take((LIMIT + 1) as u64).read_to_end(&mut bytes).ok()?;
    (bytes.len() <= LIMIT).then_some((file, bytes))
}
fn wide_text(read: impl FnOnce(&mut [u16]) -> windows::core::Result<()>) -> Option<String> {
    let mut text = vec![0_u16; 32_768];
    read(&mut text).ok()?;
    let end = text.iter().position(|&ch| ch == 0)?;
    String::from_utf16(&text[..end]).ok()
}
struct Launcher {
    application: Application,
    executable: PathBuf,
    arguments: String,
    directory: PathBuf,
    show: i32,
    _file: File,
}
fn snapshot(roots: &[PathBuf], path: &Path) -> Option<Launcher> {
    let (index, relative) = roots
        .iter()
        .enumerate()
        .find_map(|(index, root)| source(root, path).map(|name| (index, name)))?;
    let (file, bytes) = read(path)?;
    let link: IShellLinkW = unsafe { CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER).ok()? };
    // Loading from the retained bytes avoids shell link resolution, repair and network searches.
    let stream = unsafe { SHCreateMemStream(Some(&bytes)) }?;
    unsafe { link.cast::<IPersistStream>().ok()?.Load(&stream).ok()? };
    let executable = PathBuf::from(wide_text(|out| unsafe {
        link.GetPath(out, std::ptr::null_mut(), SLGP_RAWPATH.0 as u32)
    })?);
    let executable = canonical_local(&executable)?;
    if !executable.extension()?.eq_ignore_ascii_case("exe") || !executable.is_file() {
        return None;
    }
    let arguments = wide_text(|out| unsafe { link.GetArguments(out) })?;
    let directory = wide_text(|out| unsafe { link.GetWorkingDirectory(out) })?;
    let directory = if directory.is_empty() {
        executable.parent()?.to_path_buf()
    } else {
        PathBuf::from(directory)
    };
    let directory = canonical_local(&directory)?;
    if !directory.is_dir() {
        return None;
    }
    let name = path.file_stem()?.to_str()?.to_owned();
    if name.is_empty() || name.chars().any(char::is_control) {
        return None;
    }
    let show = unsafe { link.GetShowCmd().ok()?.0 };
    let mut definition = (bytes.len() as u32).to_le_bytes().to_vec();
    definition.extend_from_slice(&bytes);
    definition.extend_from_slice(&serde_json::to_vec(&(&executable, &arguments, &directory, show)).ok()?);
    let application = Application {
        id: format!("windows-start:{index}:{relative}"),
        name,
        aliases: vec![],
        source: path.to_path_buf(),
        definition: definition.into(),
    };
    Some(Launcher {
        application,
        executable,
        arguments,
        directory,
        show,
        _file: file,
    })
}
fn inventory(roots: &[PathBuf], allowed: &dyn Fn() -> bool) -> Vec<Application> {
    let mut queue: Vec<_> = roots.iter().cloned().map(|path| (path, 0)).collect();
    let mut apps = Vec::new();
    let mut visited = 0;
    let mut total_bytes = 0;
    while let Some((directory, depth)) = queue.pop() {
        if !allowed() || visited >= 8192 || apps.len() >= 2048 {
            break;
        }
        let Ok(entries) = fs::read_dir(&directory) else {
            continue;
        };
        for entry in entries.flatten() {
            visited += 1;
            if !allowed() || visited > 8192 || apps.len() >= 2048 {
                break;
            }
            let path = entry.path();
            if !ordinary(&path) {
                continue;
            }
            if path.is_dir() && depth < 8 {
                queue.push((path, depth + 1));
            } else if path
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("lnk"))
                && let Some(launcher) = snapshot(roots, &path)
            {
                total_bytes += launcher.application.definition.len();
                if total_bytes > 16 * 1024 * 1024 {
                    return apps;
                }
                apps.push(launcher.application);
            }
        }
    }
    apps.sort_by(|a, b| a.id.cmp(&b.id));
    // The per-user menu overrides the common menu at the same relative shortcut path.
    let mut seen = std::collections::BTreeSet::new();
    apps.retain(|app| seen.insert(app.id.splitn(3, ':').nth(2).unwrap_or(&app.id).to_lowercase()));
    apps
}
pub fn installed() -> Vec<Application> {
    let mut apps = worker(|allowed| Ok(inventory(&roots(), allowed))).unwrap_or_default();
    apps.extend(packaged::installed());
    apps
}
pub fn validate(application: &Application) -> Result<(), String> {
    if packaged::accepts(application) {
        return packaged::validate(application);
    }
    let application = application.clone();
    worker(move |_| {
        snapshot(&roots(), &application.source)
            .filter(|now| now.application == application)
            .map(|_| ())
            .ok_or_else(|| CHANGED.into())
    })
}
pub async fn launch(
    application: &Application,
    allowed: impl Fn() -> bool + Send + 'static,
) -> Result<(), String> {
    if packaged::accepts(application) {
        return packaged::launch(application, allowed).await;
    }
    let application = application.clone();
    let deadline = Instant::now() + BUDGET;
    let (send, receive) = tokio::sync::oneshot::channel();
    std::thread::Builder::new()
        .name("voice-application-launch".into())
        .spawn(move || {
            let result = (|| {
                unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) }
                    .ok()
                    .map_err(|_| FAILED)?;
                let _apartment = Apartment;
                let launcher = snapshot(&roots(), &application.source)
                    .filter(|now| now.application == application)
                    .ok_or(CHANGED)?;
                submit(&launcher, || {
                    !send.is_closed()
                        && Instant::now() < deadline
                        && allowed()
                        && !send.is_closed()
                        && Instant::now() < deadline
                })
            })();
            let _ = send.send(result);
        })
        .map_err(|_| FAILED)?;
    tokio::time::timeout(BUDGET, receive)
        .await
        .map_err(|_| "The Windows application service timed out.".to_string())?
        .map_err(|_| FAILED.to_string())?
}
fn submit(launcher: &Launcher, allowed: impl FnOnce() -> bool) -> Result<(), String> {
    let executable = HSTRING::from(launcher.executable.as_os_str());
    let arguments = HSTRING::from(&launcher.arguments);
    let directory = HSTRING::from(launcher.directory.as_os_str());
    let mut info = SHELLEXECUTEINFOW {
        cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
        fMask: SEE_MASK_FLAG_NO_UI | SEE_MASK_NOASYNC,
        lpVerb: windows::core::w!("open"),
        lpFile: PCWSTR(executable.as_ptr()),
        lpParameters: PCWSTR(arguments.as_ptr()),
        lpDirectory: PCWSTR(directory.as_ptr()),
        nShow: launcher.show,
        ..Default::default()
    };
    if !allowed() {
        return Err("Application launch cancelled.".into());
    }
    unsafe { ShellExecuteExW(&mut info) }.map_err(|_| FAILED.into())
}

#[cfg(test)]
#[path = "windows_tests.rs"]
mod tests;
