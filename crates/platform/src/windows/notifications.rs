//! Local Windows toasts, attributed to the identity registered by the installer.

use std::marker::PhantomData;
use std::rc::Rc;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use windows::{
    Data::Xml::Dom::XmlDocument,
    Foundation::TypedEventHandler,
    UI::Notifications::{NotificationSetting, ToastNotification, ToastNotificationManager, ToastNotifier},
    Win32::System::WinRT::{RO_INIT_MULTITHREADED, RoInitialize, RoUninitialize},
    core::{HSTRING, IInspectable, Result},
};

const GROUP: &str = "vults.sessions";

#[derive(Clone, Default)]
struct Activation(Arc<AtomicBool>);

impl Activation {
    fn armed() -> Self {
        Self(Arc::new(AtomicBool::new(true)))
    }
    fn take(&self) -> bool {
        self.0.swap(false, Ordering::AcqRel)
    }
    fn cancel(&self) {
        self.0.store(false, Ordering::Release);
    }
}

/// Kept on the worker that initialized WinRT; never moved onto a runtime thread.
pub struct Desktop {
    notifier: ToastNotifier,
    identity: HSTRING,
    _apartment: Apartment,
}

impl std::fmt::Debug for Desktop {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DesktopNotifications").finish_non_exhaustive()
    }
}

impl Drop for Desktop {
    fn drop(&mut self) {
        if let Ok(history) = ToastNotificationManager::History() {
            let _ = history.RemoveGroupWithId(&HSTRING::from(GROUP), &self.identity);
        }
    }
}

struct Apartment(PhantomData<Rc<()>>);
impl Drop for Apartment {
    fn drop(&mut self) {
        unsafe { RoUninitialize() };
    }
}

impl Desktop {
    pub fn new(identity: &str) -> Result<Self> {
        unsafe { RoInitialize(RO_INIT_MULTITHREADED)? };
        let apartment = Apartment(PhantomData);
        let identity = HSTRING::from(identity);
        match ToastNotificationManager::CreateToastNotifierWithId(&identity) {
            Ok(notifier) => {
                if let Ok(history) = ToastNotificationManager::History() {
                    let _ = history.RemoveGroupWithId(&HSTRING::from(GROUP), &identity);
                }
                Ok(Self {
                    notifier,
                    identity,
                    _apartment: apartment,
                })
            }
            Err(error) => Err(error),
        }
    }

    pub fn enabled(&self) -> Result<bool> {
        setting_allows(self.notifier.Setting())
    }

    pub fn show(
        &self,
        tag: &str,
        title: &str,
        body: &str,
        open: Option<impl Fn() + Send + 'static>,
        failure: impl Fn(i32) + Send + 'static,
    ) -> Result<Toast> {
        let document = XmlDocument::new()?;
        document.LoadXml(&HSTRING::from(content(title, body)))?;
        let toast = ToastNotification::CreateToastNotification(&document)?;
        toast.SetTag(&HSTRING::from(tag))?;
        toast.SetGroup(&HSTRING::from(GROUP))?;
        // The app owns the sound policy; the Windows sound would duplicate it.
        let active = Activation::armed();
        let activation = active.clone();
        let token = toast.Activated(&TypedEventHandler::<ToastNotification, IInspectable>::new(
            move |_, _| {
                if activation.take()
                    && let Some(open) = &open
                {
                    open();
                }
                Ok(())
            },
        ))?;
        let failed = toast.Failed(&TypedEventHandler::<
            ToastNotification,
            windows::UI::Notifications::ToastFailedEventArgs,
        >::new(move |_, args| {
            if let Some(args) = args.as_ref() {
                failure(args.ErrorCode()?.0);
            }
            Ok(())
        }))?;
        let result = Toast {
            toast,
            tag: HSTRING::from(tag),
            active,
            token,
            failed,
        };
        self.notifier.Show(&result.toast)?;
        Ok(result)
    }

    pub fn remove(&self, toast: Toast) {
        toast.active.cancel();
        let _ = self.notifier.Hide(&toast.toast);
        if let Ok(history) = ToastNotificationManager::History() {
            let _ = history.RemoveGroupedTagWithId(&toast.tag, &HSTRING::from(GROUP), &self.identity);
        }
    }

    pub fn replace(
        &self,
        previous: Toast,
        title: &str,
        body: &str,
        open: Option<impl Fn() + Send + 'static>,
        failure: impl Fn(i32) + Send + 'static,
    ) -> Result<Toast> {
        previous.active.cancel();
        let result = self.show(&previous.tag.to_string(), title, body, open, failure);
        if result.is_err() {
            self.remove(previous);
        }
        // On success Windows replaced this tag; hiding the old handle would remove the new toast.
        result
    }
}

pub struct Toast {
    toast: ToastNotification,
    tag: HSTRING,
    active: Activation,
    token: i64,
    failed: i64,
}

impl std::fmt::Debug for Toast {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Toast").finish_non_exhaustive()
    }
}

impl Drop for Toast {
    fn drop(&mut self) {
        self.active.cancel();
        let _ = self.toast.RemoveActivated(self.token);
        let _ = self.toast.RemoveFailed(self.failed);
    }
}

fn content(title: &str, body: &str) -> String {
    fn text(value: &str) -> String {
        value
            .chars()
            .filter(|ch| {
                matches!(*ch, '\t' | '\n' | '\r') || (*ch >= ' ' && !matches!(*ch, '\u{fffe}' | '\u{ffff}'))
            })
            .take(256)
            .collect::<String>()
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
    }
    format!(
        "<toast><visual><binding template=\"ToastGeneric\"><text>{}</text><text>{}</text></binding></visual><audio silent=\"true\"/></toast>",
        text(title),
        text(body)
    )
}

fn setting_allows(setting: Result<NotificationSetting>) -> Result<bool> {
    match setting {
        // Desktop preference records can be absent before the first toast. Show enforces OS policy.
        Err(error) if error.code().0 == 0x80070490_u32 as i32 => Ok(true),
        value => value.map(|setting| setting == NotificationSetting::Enabled),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stale_and_repeated_callbacks_cannot_open_a_session() {
        let activation = Activation::armed();
        let stale = activation.clone();
        activation.cancel();
        assert!(!stale.take());
        let active = Activation::armed();
        assert!(active.take());
        assert!(!active.take());
    }

    #[test]
    fn explicit_os_blocks_hold_but_absent_preferences_do_not_prevent_first_delivery() {
        assert!(!setting_allows(Ok(NotificationSetting::DisabledForApplication)).unwrap());
        assert!(!setting_allows(Ok(NotificationSetting::DisabledForUser)).unwrap());
        assert!(!setting_allows(Ok(NotificationSetting::DisabledByGroupPolicy)).unwrap());
        assert!(
            setting_allows(Err(windows::core::Error::from_hresult(windows::core::HRESULT(
                0x80070490_u32 as i32
            ))))
            .unwrap()
        );
        assert!(
            setting_allows(Err(windows::core::Error::from_hresult(windows::core::HRESULT(
                0x80004005_u32 as i32
            ))))
            .is_err()
        );
    }

    #[test]
    fn external_text_cannot_create_actions_or_invalid_xml() {
        let xml = content("<action arguments='allow'/>", "a & b\0\u{ffff}");
        assert!(xml.contains("&lt;action arguments='allow'/&gt;"));
        assert!(xml.contains("a &amp; b"));
        assert!(!xml.contains('\0'));
        assert!(!xml.contains('\u{ffff}'));
        assert!(!xml.contains("<action"));
        assert!(content(&"x".repeat(5000), "").len() < 4500);
    }

    #[test]
    fn windows_xml_parser_keeps_external_text_in_text_nodes() {
        unsafe { RoInitialize(RO_INIT_MULTITHREADED).unwrap() };
        let _apartment = Apartment(PhantomData);
        let document = XmlDocument::new().unwrap();
        document
            .LoadXml(&HSTRING::from(content("<action/>", "a & b\0")))
            .unwrap();
        assert_eq!(
            document
                .GetElementsByTagName(&HSTRING::from("action"))
                .unwrap()
                .Length()
                .unwrap(),
            0
        );
        let text = document.GetElementsByTagName(&HSTRING::from("text")).unwrap();
        assert_eq!(text.Item(0).unwrap().InnerText().unwrap(), "<action/>");
        assert_eq!(text.Item(1).unwrap().InnerText().unwrap(), "a & b");
    }

    #[test]
    #[ignore = "shows a local Windows toast with a temporary Start menu identity"]
    fn native_delivery_replacement_and_withdrawal() {
        use std::{
            os::windows::ffi::OsStrExt,
            time::{Duration, Instant},
        };
        use windows::{
            Win32::{
                Storage::EnhancedStorage::PKEY_AppUserModel_ID,
                System::Com::{
                    CLSCTX_INPROC_SERVER, CoCreateInstance, IPersistFile, StructuredStorage::PROPVARIANT,
                },
                UI::Shell::{
                    IShellLinkW, PropertiesSystem::IPropertyStore, SHCNE_CREATE, SHCNE_DELETE, SHCNF_PATHW,
                    SHChangeNotify, ShellLink,
                },
            },
            core::{Interface, PCWSTR},
        };
        unsafe { RoInitialize(RO_INIT_MULTITHREADED).unwrap() };
        let _apartment = Apartment(PhantomData);
        let identity = format!("Vults.NotificationQA.{}", std::process::id());
        let shortcut = std::path::PathBuf::from(std::env::var_os("APPDATA").unwrap())
            .join("Microsoft/Windows/Start Menu/Programs")
            .join(format!("{identity}.lnk"));
        // Reserving a new name prevents replacing any user-created shortcut.
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&shortcut)
            .unwrap();
        struct Cleanup(std::path::PathBuf);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = std::fs::remove_file(&self.0);
                let path: Vec<u16> = self.0.as_os_str().encode_wide().chain(Some(0)).collect();
                unsafe { SHChangeNotify(SHCNE_DELETE, SHCNF_PATHW, Some(path.as_ptr().cast()), None) };
            }
        }
        let _cleanup = Cleanup(shortcut.clone());
        let target: Vec<u16> = std::env::current_exe()
            .unwrap()
            .as_os_str()
            .encode_wide()
            .chain(Some(0))
            .collect();
        let file: Vec<u16> = shortcut.as_os_str().encode_wide().chain(Some(0)).collect();
        unsafe {
            let link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER).unwrap();
            link.SetPath(PCWSTR(target.as_ptr())).unwrap();
            let properties: IPropertyStore = link.cast().unwrap();
            properties
                .SetValue(&PKEY_AppUserModel_ID, &PROPVARIANT::from(identity.as_str()))
                .unwrap();
            properties.Commit().unwrap();
            link.cast::<IPersistFile>()
                .unwrap()
                .Save(PCWSTR(file.as_ptr()), true)
                .unwrap();
        }
        unsafe { SHChangeNotify(SHCNE_CREATE, SHCNF_PATHW, Some(file.as_ptr().cast()), None) };
        let desktop = Desktop::new(&identity).unwrap();
        assert!(
            desktop.enabled().unwrap(),
            "Windows notifications must be enabled for this manual smoke test"
        );
        let delivered = Arc::new(AtomicBool::new(false));
        let failure = delivered.clone();
        let first = desktop
            .show(
                "qa",
                "Vults",
                "Local notification delivery test",
                None::<fn()>,
                move |_| {
                    failure.store(true, Ordering::Release);
                },
            )
            .unwrap();
        let history = ToastNotificationManager::History().unwrap();
        let count = || {
            history
                .GetHistoryWithId(&HSTRING::from(&identity))
                .unwrap()
                .Size()
                .unwrap()
        };
        let deadline = Instant::now() + Duration::from_secs(10);
        while count() == 0 && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(100));
        }
        assert!(
            !delivered.load(Ordering::Acquire),
            "Windows reported delivery failure"
        );
        assert_eq!(count(), 1, "Windows notification history confirms delivery");
        let next = desktop
            .replace(
                first,
                "Vults",
                "Local notification replacement test",
                None::<fn()>,
                |_| {},
            )
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        let updated = || {
            history
                .GetHistoryWithId(&HSTRING::from(&identity))
                .and_then(|items| items.GetAt(0))
                .and_then(|toast| toast.Content())
                .and_then(|document| document.InnerText())
                .is_ok_and(|text| text.to_string().contains("replacement test"))
        };
        while !updated() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(100));
        }
        assert_eq!(count(), 1);
        assert!(updated());
        desktop.remove(next);
        assert_eq!(count(), 0, "Withdrawal clears notification center history");
    }
}
