use super::{Action, Error, Status};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
    mpsc,
};
use std::time::Duration;
use tokio::sync::oneshot;
use windows::{
    Win32::{
        Foundation::PROPERTYKEY,
        Media::Audio::{
            AUDIO_VOLUME_NOTIFICATION_DATA, DEVICE_STATE, DEVICE_STATE_ACTIVE, EDataFlow, ERole,
            Endpoints::{
                IAudioEndpointVolume, IAudioEndpointVolumeCallback, IAudioEndpointVolumeCallback_Impl,
            },
            IMMDevice, IMMDeviceEnumerator, IMMNotificationClient, IMMNotificationClient_Impl,
            MMDeviceEnumerator, PKEY_AudioEndpoint_PhysicalSpeakers, PKEY_AudioEngine_DeviceFormat,
            eMultimedia, eRender,
        },
        System::Com::{
            CLSCTX_ALL, COINIT_MULTITHREADED, CoCreateGuid, CoCreateInstance, CoInitializeEx, CoTaskMemFree,
            CoUninitialize,
        },
    },
    core::{GUID, PCWSTR},
};

const TIMEOUT: Duration = Duration::from_secs(2);
type Gate = Box<dyn Fn() -> bool + Send + Sync>;

enum Request {
    Apply {
        action: Action,
        allowed: Gate,
        reply: oneshot::Sender<Result<Status, Error>>,
    },
}

/// A worker pins the current multimedia output and owns its COM interfaces until this target drops.
pub struct Target(mpsc::Sender<Request>);
impl std::fmt::Debug for Target {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WindowsOutputTarget").finish_non_exhaustive()
    }
}

pub async fn pin(allowed: impl Fn() -> bool + Send + Sync + 'static) -> Result<Target, Error> {
    if !allowed() {
        return Err(Error::Cancelled);
    }
    let (tx, rx) = mpsc::channel();
    let (ready, result) = oneshot::channel();
    std::thread::Builder::new()
        .name("voice-output".into())
        .spawn(move || {
            let native = match Native::new() {
                Ok(native) if allowed() && !ready.is_closed() => native,
                Ok(_) => {
                    let _ = ready.send(Err(Error::Cancelled));
                    return;
                }
                Err(error) => {
                    let _ = ready.send(Err(error));
                    return;
                }
            };
            if let Err(error) = native
                .snapshot()
                .and_then(|snapshot| validate(&snapshot))
                .and_then(|()| native.valid().then_some(()).ok_or(Error::Changed))
            {
                let _ = ready.send(Err(error));
                return;
            }
            if !allowed() || ready.is_closed() {
                let _ = ready.send(Err(Error::Cancelled));
                return;
            }
            if ready.send(Ok(())).is_err() {
                return;
            }
            while let Ok(Request::Apply {
                action,
                allowed,
                reply,
            }) = rx.recv()
            {
                let result = apply(&native, action, &|| allowed() && !reply.is_closed());
                let _ = reply.send(result);
            }
        })
        .map_err(|_| Error::Unavailable)?;
    tokio::time::timeout(TIMEOUT, result)
        .await
        .map_err(|_| Error::Unavailable)?
        .map_err(|_| Error::Unavailable)??;
    Ok(Target(tx))
}

impl Target {
    pub async fn apply(
        &self,
        action: Action,
        allowed: impl Fn() -> bool + Send + Sync + 'static,
    ) -> Result<Status, Error> {
        if !allowed() {
            return Err(Error::Cancelled);
        }
        if matches!(action, Action::Volume(value) if value > 100) {
            return Err(Error::Invalid);
        }
        let (reply, result) = oneshot::channel();
        self.0
            .send(Request::Apply {
                action,
                allowed: Box::new(allowed),
                reply,
            })
            .map_err(|_| Error::Unavailable)?;
        tokio::time::timeout(TIMEOUT, result)
            .await
            .map_err(|_| Error::Failed)?
            .map_err(|_| Error::Unavailable)?
    }
}

#[derive(Clone, Debug)]
struct Snapshot {
    muted: bool,
    channels: Vec<f32>,
}

trait Io {
    fn valid(&self) -> bool;
    fn snapshot(&self) -> Result<Snapshot, Error>;
    fn mute(&self, muted: bool) -> Result<(), Error>;
    fn channel(&self, index: usize, volume: f32) -> Result<(), Error>;
}

fn guard(io: &impl Io, allowed: &impl Fn() -> bool) -> Result<(), Error> {
    if !allowed() {
        Err(Error::Cancelled)
    } else if !io.valid() {
        Err(Error::Changed)
    } else if !allowed() {
        Err(Error::Cancelled)
    } else {
        Ok(())
    }
}

fn validate(snapshot: &Snapshot) -> Result<(), Error> {
    if snapshot.channels.is_empty()
        || snapshot.channels.len() > 32
        || snapshot
            .channels
            .iter()
            .any(|volume| !volume.is_finite() || !(0.0..=1.0).contains(volume))
    {
        Err(Error::Invalid)
    } else {
        Ok(())
    }
}

fn percent(channels: &[f32]) -> u32 {
    (channels.iter().copied().fold(0.0_f32, f32::max) * 100.0).round() as u32
}

fn levels(before: &[f32], level: u8) -> Vec<f32> {
    let maximum = before.iter().copied().fold(0.0_f32, f32::max);
    let target = f32::from(level) / 100.0;
    before
        .iter()
        .map(|value| {
            if maximum > 0.0 {
                (value / maximum * target).clamp(0.0, 1.0)
            } else {
                target
            }
        })
        .collect()
}

fn apply(io: &impl Io, action: Action, allowed: &impl Fn() -> bool) -> Result<Status, Error> {
    if matches!(action, Action::Volume(value) if value > 100) {
        return Err(Error::Invalid);
    }
    guard(io, allowed)?;
    let before = io.snapshot()?;
    validate(&before)?;
    guard(io, allowed)?;
    let expected = match action {
        Action::Mute(muted) => {
            if muted != before.muted {
                io.mute(muted)?;
            }
            Snapshot {
                muted,
                channels: before.channels,
            }
        }
        action => {
            let level = match action {
                Action::Volume(level) => level,
                Action::Louder => percent(&before.channels).saturating_add(10).min(100) as u8,
                Action::Quieter => percent(&before.channels).saturating_sub(10).min(100) as u8,
                Action::Mute(_) => unreachable!(),
            };
            let channels = levels(&before.channels, level);
            for (index, (&value, &old)) in channels.iter().zip(&before.channels).enumerate() {
                if (value - old).abs() > 0.00001 {
                    guard(io, allowed)?;
                    io.channel(index, value)?;
                }
            }
            Snapshot {
                muted: before.muted,
                channels,
            }
        }
    };
    guard(io, allowed)?;
    let after = io.snapshot()?;
    validate(&after)?;
    guard(io, allowed)?;
    if after.muted != expected.muted
        || after.channels.len() != expected.channels.len()
        || after
            .channels
            .iter()
            .zip(&expected.channels)
            .any(|(actual, expected)| (actual - expected).abs() > 0.02)
    {
        return Err(Error::Failed);
    }
    Ok(Status {
        muted: after.muted,
        percent: percent(&after.channels),
    })
}

struct Apartment;
impl Drop for Apartment {
    fn drop(&mut self) {
        unsafe { CoUninitialize() };
    }
}

#[windows::core::implement(IMMNotificationClient)]
struct DeviceWatch {
    endpoint_id: String,
    invalid: Arc<AtomicBool>,
}
impl DeviceWatch_Impl {
    fn changed(&self, identity: &PCWSTR) {
        if identity.is_null() {
            self.invalid.store(true, Ordering::Release);
            return;
        }
        if unsafe { identity.to_string() }.is_ok_and(|identity| identity == self.endpoint_id) {
            self.invalid.store(true, Ordering::Release);
        }
    }
}
impl IMMNotificationClient_Impl for DeviceWatch_Impl {
    fn OnDeviceStateChanged(&self, id: &PCWSTR, _: DEVICE_STATE) -> windows::core::Result<()> {
        self.changed(id);
        Ok(())
    }
    fn OnDeviceAdded(&self, _: &PCWSTR) -> windows::core::Result<()> {
        Ok(())
    }
    fn OnDeviceRemoved(&self, id: &PCWSTR) -> windows::core::Result<()> {
        self.changed(id);
        Ok(())
    }
    fn OnDefaultDeviceChanged(&self, flow: EDataFlow, role: ERole, _: &PCWSTR) -> windows::core::Result<()> {
        if flow == eRender && role == eMultimedia {
            self.invalid.store(true, Ordering::Release);
        }
        Ok(())
    }
    fn OnPropertyValueChanged(&self, id: &PCWSTR, key: &PROPERTYKEY) -> windows::core::Result<()> {
        if [PKEY_AudioEndpoint_PhysicalSpeakers, PKEY_AudioEngine_DeviceFormat]
            .iter()
            .any(|routing| routing.fmtid == key.fmtid && routing.pid == key.pid)
        {
            self.changed(id);
        }
        Ok(())
    }
}

#[windows::core::implement(IAudioEndpointVolumeCallback)]
struct VolumeWatch {
    invalid: Arc<AtomicBool>,
    context: GUID,
}
impl IAudioEndpointVolumeCallback_Impl for VolumeWatch_Impl {
    fn OnNotify(&self, data: *mut AUDIO_VOLUME_NOTIFICATION_DATA) -> windows::core::Result<()> {
        if let Some(data) = unsafe { data.as_ref() }
            && data.guidEventContext != self.context
        {
            self.invalid.store(true, Ordering::Release);
        }
        Ok(())
    }
}

struct Native {
    enumerator: IMMDeviceEnumerator,
    device: IMMDevice,
    volume: IAudioEndpointVolume,
    identity: String,
    count: u32,
    context: GUID,
    invalid: Arc<AtomicBool>,
    device_watch: IMMNotificationClient,
    volume_watch: IAudioEndpointVolumeCallback,
    _apartment: Apartment,
}

fn identity(device: &IMMDevice) -> Result<String, Error> {
    let value = unsafe { device.GetId() }.map_err(|_| Error::Unavailable)?;
    let identity = unsafe { value.to_string() };
    unsafe { CoTaskMemFree(Some(value.0.cast())) };
    identity.map_err(|_| Error::Invalid)
}

impl Native {
    fn new() -> Result<Self, Error> {
        unsafe { CoInitializeEx(None, COINIT_MULTITHREADED).ok() }.map_err(|_| Error::Unavailable)?;
        let apartment = Apartment;
        let enumerator: IMMDeviceEnumerator =
            unsafe { CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL) }
                .map_err(|_| Error::Unavailable)?;
        let device = unsafe { enumerator.GetDefaultAudioEndpoint(eRender, eMultimedia) }
            .map_err(|_| Error::Unavailable)?;
        let identity = identity(&device)?;
        let volume: IAudioEndpointVolume =
            unsafe { device.Activate(CLSCTX_ALL, None) }.map_err(|_| Error::Unavailable)?;
        let count = unsafe { volume.GetChannelCount() }.map_err(|_| Error::Unavailable)?;
        if count == 0 || count > 32 {
            return Err(Error::Invalid);
        }
        let invalid = Arc::new(AtomicBool::new(false));
        let device_watch: IMMNotificationClient = DeviceWatch {
            endpoint_id: identity.clone(),
            invalid: invalid.clone(),
        }
        .into();
        let context = unsafe { CoCreateGuid() }.map_err(|_| Error::Unavailable)?;
        let volume_watch: IAudioEndpointVolumeCallback = VolumeWatch {
            invalid: invalid.clone(),
            context,
        }
        .into();
        unsafe { enumerator.RegisterEndpointNotificationCallback(&device_watch) }
            .map_err(|_| Error::Unavailable)?;
        if unsafe { volume.RegisterControlChangeNotify(&volume_watch) }.is_err() {
            let _ = unsafe { volume.UnregisterControlChangeNotify(&volume_watch) };
            let _ = unsafe { enumerator.UnregisterEndpointNotificationCallback(&device_watch) };
            return Err(Error::Unavailable);
        }
        Ok(Self {
            enumerator,
            device,
            volume,
            identity,
            count,
            context,
            invalid,
            device_watch,
            volume_watch,
            _apartment: apartment,
        })
    }
}

impl Drop for Native {
    fn drop(&mut self) {
        let _ = unsafe { self.volume.UnregisterControlChangeNotify(&self.volume_watch) };
        let _ = unsafe {
            self.enumerator
                .UnregisterEndpointNotificationCallback(&self.device_watch)
        };
    }
}

impl Io for Native {
    fn valid(&self) -> bool {
        !self.invalid.load(Ordering::Acquire)
            && unsafe { self.device.GetState() }.is_ok_and(|state| state == DEVICE_STATE_ACTIVE)
            && unsafe { self.volume.GetChannelCount() }.is_ok_and(|count| count == self.count)
            && unsafe { self.enumerator.GetDefaultAudioEndpoint(eRender, eMultimedia) }
                .ok()
                .and_then(|device| identity(&device).ok())
                .as_deref()
                == Some(self.identity.as_str())
            && !self.invalid.load(Ordering::Acquire)
    }
    fn snapshot(&self) -> Result<Snapshot, Error> {
        Ok(Snapshot {
            muted: unsafe { self.volume.GetMute() }
                .map_err(|_| Error::Changed)?
                .as_bool(),
            channels: (0..self.count)
                .map(|index| {
                    unsafe { self.volume.GetChannelVolumeLevelScalar(index) }.map_err(|_| Error::Changed)
                })
                .collect::<Result<_, _>>()?,
        })
    }
    fn mute(&self, muted: bool) -> Result<(), Error> {
        unsafe { self.volume.SetMute(muted, &self.context) }.map_err(|_| Error::Failed)
    }
    fn channel(&self, index: usize, volume: f32) -> Result<(), Error> {
        unsafe {
            self.volume
                .SetChannelVolumeLevelScalar(index as u32, volume, &self.context)
        }
        .map_err(|_| Error::Failed)
    }
}

#[cfg(test)]
#[path = "system_audio_tests.rs"]
mod tests;
