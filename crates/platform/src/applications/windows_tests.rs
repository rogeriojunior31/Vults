use super::*;
use windows::Win32::System::Com::IPersistFile;

fn link(path: &Path, target: &Path, arguments: &str) {
    let link: IShellLinkW = unsafe { CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER).unwrap() };
    unsafe {
        link.SetPath(&HSTRING::from(target.as_os_str())).unwrap();
        link.SetArguments(&HSTRING::from(arguments)).unwrap();
        link.SetShowCmd(windows::Win32::UI::WindowsAndMessaging::SW_HIDE)
            .unwrap();
        link.SetWorkingDirectory(&HSTRING::from(path.parent().unwrap().as_os_str()))
            .unwrap();
        link.cast::<IPersistFile>()
            .unwrap()
            .Save(&HSTRING::from(path.as_os_str()), true)
            .unwrap();
    }
}

#[test]
fn bounded_registered_snapshots_reject_changes_and_never_run_spoken_arguments() {
    worker(|_| {
        let root = std::env::temp_dir().join(format!("vults-launchers-{}", std::process::id()));
        fs::create_dir_all(root.join("nested")).unwrap();
        let roots = vec![root.clone()];
        let path = root.join("nested/Fixture editor.lnk");
        let executable = std::env::current_exe().unwrap();
        link(&path, &executable, "--installed-argument");
        let launcher = snapshot(&roots, &path).unwrap();
        assert_eq!(launcher.application.name, "Fixture editor");
        assert_eq!(launcher.arguments, "--installed-argument");
        assert!(submit(&launcher, || false).is_err());
        assert!(
            OpenOptions::new().write(true).open(&path).is_err(),
            "a checked launcher cannot be replaced during submission"
        );
        let application = launcher.application.clone();
        drop(launcher);
        assert_eq!(snapshot(&roots, &path).unwrap().application, application);
        link(&path, &executable, "--changed-argument");
        assert_ne!(snapshot(&roots, &path).unwrap().application, application);
        link(&root.join("Document.lnk"), &root.join("file.txt"), "");
        fs::write(root.join("file.txt"), "fixture").unwrap();
        link(&root.join("URL.lnk"), Path::new("https://example.invalid"), "");
        fs::write(root.join("large.lnk"), vec![0; LIMIT + 1]).unwrap();
        fs::write(root.join("broken.lnk"), b"not a shell link").unwrap();
        let apps = inventory(&roots, &|| true);
        assert_eq!(apps.len(), 1);
        assert!(source(&root, &root.join("nested/../Fixture.lnk")).is_none());
        assert!(snapshot(&[], &path).is_none());
        assert!(inventory(&roots, &|| false).is_empty());
        fs::remove_dir_all(&root).unwrap();
        Ok(())
    })
    .unwrap();
}

#[test]
fn network_and_relative_paths_are_not_executable_authority() {
    for path in [
        r"\\server\share\app.exe",
        r"\\?\UNC\server\share\app.exe",
        r"C:app.exe",
        r"app.exe",
        r"C:\apps\..\app.exe",
    ] {
        assert!(!local(Path::new(path)));
    }
    assert!(local(Path::new(r"C:\apps\app.exe")));
}

#[test]
#[ignore = "reads the current user's Start Menu and verifies native Shell launch of an owned fixture process"]
fn native_start_menu_inventory_and_owned_launch() {
    unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) }
        .ok()
        .unwrap();
    let _apartment = Apartment;
    {
        let roots = roots();
        let folder = roots
            .first()
            .unwrap()
            .join(format!("Vults QA {}", std::process::id()));
        fs::create_dir(&folder).unwrap();
        struct Cleanup(PathBuf);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                for _ in 0..40 {
                    if fs::remove_dir_all(&self.0).is_ok() || !self.0.exists() {
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(25));
                }
            }
        }
        let _cleanup = Cleanup(folder.clone());
        let marker = folder.join("launched.txt");
        let path = folder.join("Fixture editor.lnk");
        let arguments =
            "--exact applications::windows::tests::owned_launch_child --ignored --nocapture".to_string();
        link(&path, &std::env::current_exe().unwrap(), &arguments);
        let launcher = snapshot(&roots, &path).unwrap();
        let application = launcher.application.clone();
        assert!(validate(&application).is_ok());
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .unwrap();
        assert!(runtime.block_on(launch(&application, || false)).is_err());
        assert!(
            inventory(&roots, &|| true)
                .iter()
                .any(|app| app == &launcher.application)
        );
        // The child derives the owned marker from its fixed installed working directory.
        drop(launcher);
        let entered = Arc::new(AtomicBool::new(false));
        let released = Arc::new(AtomicBool::new(false));
        let (native_entered, native_released) = (entered.clone(), released.clone());
        let original = application.clone();
        runtime.block_on(async {
            let pending = tokio::spawn(async move {
                launch(&original, move || {
                    native_entered.store(true, Ordering::Release);
                    while !native_released.load(Ordering::Acquire) {
                        std::thread::sleep(Duration::from_millis(5));
                    }
                    true
                })
                .await
            });
            let deadline = Instant::now() + BUDGET;
            while !entered.load(Ordering::Acquire) {
                assert!(Instant::now() < deadline);
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
            pending.abort();
            assert!(pending.await.unwrap_err().is_cancelled());
            released.store(true, Ordering::Release);
            tokio::time::sleep(Duration::from_millis(500)).await;
            assert!(!marker.exists(), "an abandoned reply must not launch its process");
        });
        runtime.block_on(launch(&application, || true)).unwrap();
        let until = Instant::now() + Duration::from_secs(5);
        while !marker.exists() && Instant::now() < until {
            std::thread::sleep(Duration::from_millis(25));
        }
        assert!(marker.exists());
    }
}

#[test]
fn expired_native_workers_cannot_admit_a_late_launch() {
    let wrote = Arc::new(AtomicBool::new(false));
    let observed = wrote.clone();
    let result = worker(move |allowed| {
        std::thread::sleep(BUDGET + Duration::from_millis(100));
        if allowed() {
            observed.store(true, Ordering::Release);
        }
        Ok(())
    });
    assert!(result.is_err());
    std::thread::sleep(Duration::from_millis(150));
    assert!(!wrote.load(Ordering::Acquire));
}

#[test]
#[ignore = "child helper, launched only through the owned native smoke-test shortcut"]
fn owned_launch_child() {
    let cwd = std::env::current_dir().unwrap();
    assert!(
        cwd.file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with("Vults QA ")
    );
    fs::write(cwd.join("launched.txt"), "requested").unwrap();
}
