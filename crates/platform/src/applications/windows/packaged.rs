//! Current-user package registrations only; activation uses the selected app-list entry.
use super::{Admission, Application, BUDGET, CHANGED, FAILED};
use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};
use windows::{
    ApplicationModel::{Core::AppListEntry, Package},
    Management::Deployment::PackageManager,
    Win32::System::WinRT::{RO_INIT_MULTITHREADED, RoInitialize, RoUninitialize},
    core::HSTRING,
};

const PREFIX: &str = "windows-package:";
const MAX_PACKAGES: usize = 512;
const MAX_APPS: usize = 512;
const MAX_DEFINITION: usize = 16_384;
const ENTRY_BUDGET: Duration = Duration::from_millis(150);
#[cfg(test)]
static ACTIVATIONS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
struct Apartment;
impl Drop for Apartment {
    fn drop(&mut self) {
        unsafe { RoUninitialize() };
    }
}
fn initialize() -> Result<Apartment, String> {
    unsafe { RoInitialize(RO_INIT_MULTITHREADED) }.map_err(|_| FAILED)?;
    Ok(Apartment)
}
fn runtime() -> Result<tokio::runtime::Runtime, String> {
    tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .map_err(|_| FAILED.into())
}
fn worker<T: Send + 'static>(
    job: impl FnOnce(&tokio::runtime::Runtime, &dyn Fn() -> bool) -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    let active = Admission(Arc::new(AtomicBool::new(true)));
    let live = active.0.clone();
    let deadline = Instant::now() + BUDGET - Duration::from_millis(100);
    let (send, receive) = mpsc::sync_channel(1);
    std::thread::Builder::new()
        .name("voice-packaged-apps".into())
        .spawn(move || {
            let result = (|| {
                let _apartment = initialize()?;
                let runtime = runtime()?;
                job(&runtime, &|| {
                    live.load(Ordering::Acquire) && Instant::now() < deadline
                })
            })();
            let _ = send.send(result);
        })
        .map_err(|_| FAILED)?;
    receive.recv_timeout(BUDGET).map_err(|_| FAILED.to_string())?
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Identity {
    family: String,
    full_name: String,
    aumid: String,
}
fn clean(value: &str) -> bool {
    !value.is_empty() && value.len() <= 1024 && !value.chars().any(char::is_control)
}
impl Identity {
    fn valid(&self) -> bool {
        clean(&self.family)
            && clean(&self.full_name)
            && clean(&self.aumid)
            && self
                .aumid
                .split_once('!')
                .is_some_and(|(family, id)| family == self.family && !id.is_empty() && !id.contains('!'))
    }
    fn application(&self, name: String) -> Option<Application> {
        if !self.valid() || !clean(&name) || name.starts_with("ms-resource:") {
            return None;
        }
        Some(Application {
            id: format!("{PREFIX}{}", self.aumid),
            name,
            aliases: vec![],
            source: PathBuf::new(),
            definition: serde_json::to_vec(self).ok()?.into(),
        })
    }
    fn from(application: &Application) -> Option<Self> {
        if application.definition.len() > MAX_DEFINITION || !application.source.as_os_str().is_empty() {
            return None;
        }
        let identity: Self = serde_json::from_slice(&application.definition).ok()?;
        (identity.valid() && application.id == format!("{PREFIX}{}", identity.aumid)).then_some(identity)
    }
}
fn not_stub(result: windows::core::Result<bool>) -> bool {
    match result {
        Ok(stub) => !stub,
        // Package8 and installation stubs are absent on older Windows versions.
        Err(error) => error.code() == windows::Win32::Foundation::E_NOINTERFACE,
    }
}
fn ready(package: &Package) -> bool {
    package.IsFramework() == Ok(false)
        && package.IsResourcePackage() == Ok(false)
        && package.IsBundle() == Ok(false)
        && not_stub(package.IsStub())
        && package.Status().and_then(|status| status.VerifyIsOK()) == Ok(true)
}
fn snapshot(package: &Package, entry: &AppListEntry) -> Option<Application> {
    let id = package.Id().ok()?;
    let identity = Identity {
        family: id.FamilyName().ok()?.to_string(),
        full_name: id.FullName().ok()?.to_string(),
        aumid: entry.AppUserModelId().ok()?.to_string(),
    };
    identity.application(entry.DisplayInfo().ok()?.DisplayName().ok()?.to_string())
}
async fn entries(package: &Package) -> Option<Vec<AppListEntry>> {
    let request = package.GetAppListEntriesAsync().ok()?;
    match tokio::time::timeout(ENTRY_BUDGET, request.clone()).await {
        Ok(Ok(entries)) => Some(entries.into_iter().take(MAX_APPS).collect()),
        _ => {
            let _ = request.Cancel();
            None
        }
    }
}
async fn inventory(allowed: &dyn Fn() -> bool) -> Result<Vec<Application>, String> {
    let manager = PackageManager::new().map_err(|_| FAILED)?;
    let packages = manager
        .FindPackagesByUserSecurityId(&HSTRING::new())
        .map_err(|_| FAILED)?;
    let mut apps = Vec::new();
    for package in packages.into_iter().take(MAX_PACKAGES) {
        if !allowed() || apps.len() >= MAX_APPS {
            break;
        }
        if !ready(&package) {
            continue;
        }
        let Some(entries) = entries(&package).await else {
            continue;
        };
        for entry in entries {
            if !allowed() || apps.len() >= MAX_APPS {
                break;
            }
            if let Some(app) = snapshot(&package, &entry) {
                apps.push(app);
            }
        }
    }
    apps.sort_by(|a, b| a.id.cmp(&b.id));
    // Multiple registrations for one AUMID are ambiguous, even if their labels agree.
    let mut counts = std::collections::BTreeMap::new();
    for app in &apps {
        *counts.entry(app.id.clone()).or_insert(0) += 1;
    }
    apps.retain(|app| counts.get(&app.id) == Some(&1));
    Ok(apps)
}
async fn current(application: &Application, allowed: &dyn Fn() -> bool) -> Result<AppListEntry, String> {
    let identity = Identity::from(application).ok_or(CHANGED)?;
    let manager = PackageManager::new().map_err(|_| CHANGED)?;
    let packages = manager
        .FindPackagesByUserSecurityIdPackageFamilyName(&HSTRING::new(), &HSTRING::from(&identity.family))
        .map_err(|_| CHANGED)?;
    let mut target = None;
    for package in packages.into_iter().take(MAX_PACKAGES) {
        if !allowed() {
            return Err(CHANGED.into());
        }
        if !ready(&package) {
            continue;
        }
        let entries = entries(&package).await.ok_or(CHANGED)?;
        for entry in entries {
            if entry.AppUserModelId().is_ok_and(|id| id == identity.aumid) {
                if target.is_some() || snapshot(&package, &entry).as_ref() != Some(application) {
                    return Err(CHANGED.into());
                }
                target = Some(entry);
            }
        }
    }
    target.ok_or_else(|| CHANGED.into())
}
pub(super) fn accepts(application: &Application) -> bool {
    application.id.starts_with(PREFIX)
}
pub(super) fn installed() -> Vec<Application> {
    worker(|runtime, allowed| runtime.block_on(inventory(allowed))).unwrap_or_default()
}
pub(super) fn validate(application: &Application) -> Result<(), String> {
    let application = application.clone();
    worker(move |runtime, allowed| runtime.block_on(current(&application, allowed)).map(|_| ()))
}
pub(super) async fn launch(
    application: &Application,
    allowed: impl Fn() -> bool + Send + 'static,
) -> Result<(), String> {
    let application = application.clone();
    let deadline = Instant::now() + BUDGET;
    let (send, receive) = tokio::sync::oneshot::channel();
    std::thread::Builder::new()
        .name("voice-package-launch".into())
        .spawn(move || {
            let result = (|| {
                let _apartment = initialize()?;
                let runtime = runtime()?;
                runtime.block_on(async {
                    let live = || !send.is_closed() && Instant::now() < deadline;
                    if !live() || !allowed() || !live() {
                        return Err(FAILED.into());
                    }
                    let target = current(&application, &live).await?;
                    if !live() || !allowed() || !live() {
                        return Err(FAILED.into());
                    }
                    #[cfg(test)]
                    ACTIVATIONS.fetch_add(1, Ordering::AcqRel);
                    let request = target.LaunchAsync().map_err(|_| FAILED)?;
                    match tokio::time::timeout_at(deadline.into(), request.clone()).await {
                        Ok(Ok(true)) => Ok(()),
                        _ => {
                            let _ = request.Cancel();
                            Err(FAILED.into())
                        }
                    }
                })
            })();
            let _ = send.send(result);
        })
        .map_err(|_| FAILED)?;
    tokio::time::timeout(BUDGET, receive)
        .await
        .map_err(|_| FAILED.to_string())?
        .map_err(|_| FAILED.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unknown_stub_state_fails_closed_except_on_older_package_interfaces() {
        use windows::Win32::Foundation::{E_ACCESSDENIED, E_NOINTERFACE};
        assert!(not_stub(Ok(false)));
        assert!(!not_stub(Ok(true)));
        assert!(!not_stub(Err(windows::core::Error::from_hresult(E_ACCESSDENIED))));
        assert!(not_stub(Err(windows::core::Error::from_hresult(E_NOINTERFACE))));
    }
    fn app() -> Application {
        Identity {
            family: "Fixture_abcd".into(),
            full_name: "Fixture_1.0.0.0_x64__abcd".into(),
            aumid: "Fixture_abcd!Editor".into(),
        }
        .application("Fixture editor".into())
        .unwrap()
    }
    #[test]
    fn identity_is_exact_and_rejects_cross_family_or_forged_sources() {
        let app = app();
        assert!(Identity::from(&app).is_some());
        let mut forged = app.clone();
        forged.id.push_str("Other");
        assert!(Identity::from(&forged).is_none());
        forged = app.clone();
        forged.source = "file.exe".into();
        assert!(Identity::from(&forged).is_none());
        let mut identity = Identity::from(&app).unwrap();
        identity.aumid = "Other_abcd!Editor".into();
        assert!(identity.application("Editor".into()).is_none());
    }
    #[test]
    fn changed_package_version_or_display_name_invalidates_snapshot() {
        let original = app();
        let mut identity = Identity::from(&original).unwrap();
        identity.full_name = "Fixture_2.0.0.0_x64__abcd".into();
        assert_ne!(original, identity.application(original.name.clone()).unwrap());
        identity = Identity::from(&original).unwrap();
        assert_ne!(original, identity.application("Renamed editor".into()).unwrap());
        assert!(identity.application("ms-resource:HiddenName".into()).is_none());
    }
    #[test]
    #[ignore = "reads current-user package app entries; optional explicit AUMID activates an installed app"]
    fn native_current_user_catalog_and_activation() {
        let apps = installed();
        assert!(
            !apps.is_empty(),
            "the interactive smoke requires installed packaged apps"
        );
        let app = match std::env::var("VULTS_STORE_SMOKE_AUMID") {
            Ok(id) => apps
                .iter()
                .find(|app| app.id == format!("{PREFIX}{id}"))
                .expect("requested fixture must be registered"),
            Err(_) => &apps[0],
        };
        assert!(validate(app).is_ok());
        let mut stale = app.clone();
        stale.name.push_str(" changed");
        assert!(validate(&stale).is_err());
        let mut identity = Identity::from(app).unwrap();
        identity.full_name.push_str("_changed");
        stale = identity.application(app.name.clone()).unwrap();
        assert!(
            validate(&stale).is_err(),
            "a queued old registration cannot inherit another package version"
        );
        let runtime = runtime().unwrap();
        assert!(runtime.block_on(launch(app, || false)).is_err());
        if std::env::var_os("VULTS_STORE_SMOKE_AUMID").is_some() {
            let baseline = ACTIVATIONS.load(Ordering::Acquire);
            let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
            let entered = Arc::new(AtomicBool::new(false));
            let released = Arc::new(AtomicBool::new(false));
            let finished = Arc::new(AtomicBool::new(false));
            struct Finished(Arc<AtomicBool>);
            impl Drop for Finished {
                fn drop(&mut self) {
                    self.0.store(true, Ordering::Release);
                }
            }
            let completion = Finished(finished.clone());
            let (native_entered, native_released) = (entered.clone(), released.clone());
            let original = app.clone();
            runtime.block_on(async {
                let pending = tokio::spawn(async move {
                    launch(&original, move || {
                        let _completion = &completion;
                        if calls.fetch_add(1, Ordering::AcqRel) == 1 {
                            native_entered.store(true, Ordering::Release);
                            while !native_released.load(Ordering::Acquire) {
                                std::thread::sleep(Duration::from_millis(5));
                            }
                        }
                        true
                    })
                    .await
                });
                let deadline = Instant::now() + BUDGET;
                while !entered.load(Ordering::Acquire) {
                    assert!(
                        Instant::now() < deadline,
                        "native fixture must reach final admission"
                    );
                    tokio::time::sleep(Duration::from_millis(5)).await;
                }
                pending.abort();
                assert!(pending.await.unwrap_err().is_cancelled());
                released.store(true, Ordering::Release);
                let deadline = Instant::now() + BUDGET;
                while !finished.load(Ordering::Acquire) {
                    assert!(Instant::now() < deadline, "abandoned native worker must finish");
                    tokio::time::sleep(Duration::from_millis(5)).await;
                }
                assert_eq!(
                    ACTIVATIONS.load(Ordering::Acquire),
                    baseline,
                    "abandonment must prevent the OS activation call"
                );
            });
            runtime.block_on(launch(app, || true)).unwrap();
            assert_eq!(ACTIVATIONS.load(Ordering::Acquire), baseline + 1);
        }
        eprintln!(
            "Packaged app inventory: {} entries; registration validation passed",
            apps.len()
        );
    }
}
