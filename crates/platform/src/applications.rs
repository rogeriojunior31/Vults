//! Installed launchers only. A spoken name is never a command line or a file argument.
use std::{path::PathBuf, sync::Arc};

#[derive(Clone, PartialEq, Eq)]
pub struct Application {
    pub id: String,
    pub name: String,
    pub aliases: Vec<String>,
    pub source: PathBuf,
    pub definition: Arc<[u8]>,
}

impl std::fmt::Debug for Application {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Application")
            .field("id", &self.id)
            .field("name", &self.name)
            .finish_non_exhaustive()
    }
}

#[cfg(target_os = "linux")]
mod linux {
    use super::*;
    use gtk::gio::{self, AppLaunchContext, DesktopAppInfo, prelude::*};
    use std::io::Read;

    fn snapshot(entry: &DesktopAppInfo) -> Option<Application> {
        if entry.is_hidden() || !entry.should_show() {
            return None;
        }
        let id = entry.id()?.to_string();
        if !valid_id(&id) {
            return None;
        }
        let source = entry.filename()?;
        let mut definition = Vec::new();
        std::fs::File::open(&source)
            .ok()?
            .take(65_537)
            .read_to_end(&mut definition)
            .ok()?;
        if definition.len() > 65_536 {
            return None;
        }
        let name = entry.display_name().to_string();
        let aliases = [
            Some(entry.name().to_string()),
            entry.generic_name().map(|name| name.to_string()),
        ]
        .into_iter()
        .flatten()
        .filter(|name| !name.is_empty())
        .collect();
        Some(Application {
            id,
            name,
            aliases,
            source,
            definition: definition.into(),
        })
    }

    fn valid_id(id: &str) -> bool {
        id.len() <= 255
            && id.ends_with(".desktop")
            && !id.starts_with('.')
            && !id.contains(['/', '\\'])
            && !id.chars().any(char::is_control)
    }

    pub fn installed() -> Vec<Application> {
        let mut apps: Vec<_> = gio::AppInfo::all()
            .into_iter()
            .filter_map(|entry| entry.downcast::<DesktopAppInfo>().ok())
            .filter_map(|entry| snapshot(&entry))
            .take(2048)
            .collect();
        apps.sort_by(|a, b| a.id.cmp(&b.id));
        apps.dedup_by(|a, b| a.id == b.id);
        apps
    }

    fn current(app: &Application) -> Option<DesktopAppInfo> {
        if !valid_id(&app.id) {
            return None;
        }
        let entry = DesktopAppInfo::new(&app.id)?;
        (snapshot(&entry).as_ref() == Some(app)).then_some(entry)
    }

    /// Read-only preflight; launch repeats this check immediately before its effect.
    pub fn validate(app: &Application) -> Result<(), String> {
        current(app)
            .map(|_| ())
            .ok_or_else(|| "The installed application changed or is unavailable.".into())
    }

    pub fn launch(app: &Application, allowed: impl FnOnce() -> bool) -> Result<(), String> {
        let entry = current(app).ok_or("The installed application changed or is unavailable.")?;
        if !allowed() {
            return Err("Application launch cancelled.".into());
        }
        entry
            .launch(&[], None::<&AppLaunchContext>)
            .map_err(|_| "Could not launch the installed application.".into())
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        #[test]
        fn launchers_are_local_registered_visible_and_unchanged() {
            if std::env::var_os("VULTS_APPLICATION_TEST_CHILD").is_none() {
                let dir = std::env::temp_dir().join(format!("vults-apps-{}", std::process::id()));
                use std::os::unix::fs::DirBuilderExt;
                std::fs::DirBuilder::new().mode(0o700).create(&dir).unwrap();
                let data = dir.join("applications");
                std::fs::create_dir_all(&data).unwrap();
                let entry = "[Desktop Entry]\nType=Application\nName=Fixture editor\nExec=/usr/bin/true\n";
                std::fs::write(data.join("fixture.desktop"), entry).unwrap();
                std::fs::write(data.join("hidden.desktop"), format!("{entry}Hidden=true\n")).unwrap();
                std::fs::write(data.join("invisible.desktop"), format!("{entry}NoDisplay=true\n")).unwrap();
                std::fs::create_dir_all(dir.join("empty")).unwrap();
                let status = std::process::Command::new(std::env::current_exe().unwrap())
                    .args([
                        "--exact",
                        "applications::linux::tests::launchers_are_local_registered_visible_and_unchanged",
                    ])
                    .env("VULTS_APPLICATION_TEST_CHILD", "1")
                    .env("XDG_DATA_HOME", &dir)
                    .env("XDG_DATA_DIRS", dir.join("empty"))
                    .status()
                    .unwrap();
                std::fs::remove_dir_all(&dir).unwrap();
                assert!(status.success());
                return;
            }
            let apps = installed();
            assert_eq!(apps.len(), 1);
            let app = &apps[0];
            assert_eq!(app.id, "fixture.desktop");
            assert!(validate(app).is_ok());
            assert!(launch(app, || false).is_err());
            let mut forged = app.clone();
            forged.id = "/tmp/fixture.desktop".into();
            assert!(validate(&forged).is_err());
            std::fs::write(
                &app.source,
                "[Desktop Entry]\nType=Application\nName=Fixture editor\nExec=/usr/bin/false\n",
            )
            .unwrap();
            assert!(validate(app).is_err());
            assert!(launch(app, || panic!("changed definition must fail before admission")).is_err());
        }
    }
}

#[cfg(target_os = "linux")]
pub use linux::{installed, launch, validate};
