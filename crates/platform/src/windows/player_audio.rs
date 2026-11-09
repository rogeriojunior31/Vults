//! Explicit live mixer targets. Never infer audio ownership from a player name or PID alone.
use std::{
    collections::BTreeMap,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::Duration,
};
use tokio::sync::oneshot;
use windows::{
    Win32::{
        Devices::FunctionDiscovery::PKEY_Device_FriendlyName,
        Foundation::PROPERTYKEY,
        Media::Audio::{
            AudioSessionDisconnectReason, AudioSessionState, AudioSessionStateExpired, DEVICE_STATE,
            DEVICE_STATE_ACTIVE, EDataFlow, ERole, IAudioSessionControl2, IAudioSessionEvents,
            IAudioSessionEvents_Impl, IAudioSessionManager2, IMMDevice, IMMDeviceEnumerator,
            IMMNotificationClient, IMMNotificationClient_Impl, ISimpleAudioVolume, MMDeviceEnumerator,
            eMultimedia, eRender,
        },
        System::{
            Com::{
                CLSCTX_ALL, COINIT_MULTITHREADED, CoCreateGuid, CoCreateInstance, CoInitializeEx,
                CoTaskMemFree, CoUninitialize, STGM_READ,
                StructuredStorage::{PropVariantClear, PropVariantToStringAlloc},
            },
            RemoteDesktop::ProcessIdToSessionId,
        },
    },
    core::{GUID, Interface, PCWSTR, PWSTR},
};

const TIMEOUT: Duration = Duration::from_secs(2);
type Gate = Box<dyn Fn() -> bool + Send + Sync>;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Selection {
    endpoint: String,
    default: Option<String>,
    pid: u32,
    birth: u64,
    instances: Vec<String>,
}
#[derive(Clone, Debug)]
pub struct Candidate {
    pub selection: Selection,
    pub app: String,
    pub output: Option<String>,
    pub pid: u32,
    pub percent: u8,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Volume(u8),
    Louder,
    Quieter,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Cancelled,
    Changed,
    Unavailable,
    Invalid,
    Failed,
}
struct Apartment;
impl Drop for Apartment {
    fn drop(&mut self) {
        unsafe { CoUninitialize() };
    }
}
fn initialize() -> Result<Apartment, Error> {
    unsafe { CoInitializeEx(None, COINIT_MULTITHREADED).ok() }.map_err(|_| Error::Unavailable)?;
    Ok(Apartment)
}
fn enumerator() -> Result<IMMDeviceEnumerator, Error> {
    unsafe { CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL) }.map_err(|_| Error::Unavailable)
}
fn owned_string(raw: PWSTR) -> Option<String> {
    let text = unsafe { raw.to_string() }.ok();
    unsafe { CoTaskMemFree(Some(raw.0.cast())) };
    text
}
fn device_id(device: &IMMDevice) -> Option<String> {
    owned_string(unsafe { device.GetId() }.ok()?)
}
fn default_id(enumerator: &IMMDeviceEnumerator) -> Option<String> {
    device_id(&unsafe { enumerator.GetDefaultAudioEndpoint(eRender, eMultimedia) }.ok()?)
}
fn friendly(device: &IMMDevice) -> Option<String> {
    let store = unsafe { device.OpenPropertyStore(STGM_READ) }.ok()?;
    let mut value = unsafe { store.GetValue(&PKEY_Device_FriendlyName) }.ok()?;
    let text = unsafe { PropVariantToStringAlloc(&value) }
        .ok()
        .and_then(owned_string);
    let _ = unsafe { PropVariantClear(&mut value) };
    text.filter(|text| !text.is_empty() && text.len() < 512 && !text.chars().any(char::is_control))
}
fn process(control: &IAudioSessionControl2) -> Option<(u32, u64)> {
    if unsafe { control.IsSystemSoundsSession() }.0 != 1 {
        return None;
    }
    let mut pid = 0;
    // The safe wrapper discards AUDCLNT_S_NO_SINGLE_PROCESS; shared ownership is not a PID pin.
    let status = unsafe { (Interface::vtable(control).GetProcessId)(Interface::as_raw(control), &mut pid) };
    if status.0 != 0 || pid == 0 || pid == std::process::id() {
        return None;
    }
    let (mut mine, mut theirs) = (0, 0);
    unsafe {
        ProcessIdToSessionId(std::process::id(), &mut mine).ok()?;
        ProcessIdToSessionId(pid, &mut theirs).ok()?;
    }
    if mine != theirs {
        return None;
    }
    Some((pid, vults_peer::process_birth(pid)??))
}
fn instance(control: &IAudioSessionControl2) -> Option<String> {
    owned_string(unsafe { control.GetSessionInstanceIdentifier() }.ok()?)
}
fn sessions(device: &IMMDevice) -> Result<Vec<IAudioSessionControl2>, Error> {
    let manager: IAudioSessionManager2 =
        unsafe { device.Activate(CLSCTX_ALL, None) }.map_err(|_| Error::Unavailable)?;
    let list = unsafe { manager.GetSessionEnumerator() }.map_err(|_| Error::Unavailable)?;
    let count = unsafe { list.GetCount() }.map_err(|_| Error::Unavailable)?;
    if !(0..=512).contains(&count) {
        return Err(Error::Invalid);
    }
    (0..count)
        .map(|i| {
            unsafe { list.GetSession(i) }
                .and_then(|control| control.cast())
                .map_err(|_| Error::Unavailable)
        })
        .collect()
}
fn volume(control: &IAudioSessionControl2) -> Result<(f32, bool), Error> {
    let volume: ISimpleAudioVolume = control.cast().map_err(|_| Error::Unavailable)?;
    Ok((
        unsafe { volume.GetMasterVolume() }.map_err(|_| Error::Failed)?,
        unsafe { volume.GetMute() }.map_err(|_| Error::Failed)?.as_bool(),
    ))
}
fn enumerate(enumerator: &IMMDeviceEnumerator, allowed: &dyn Fn() -> bool) -> Result<Vec<Candidate>, Error> {
    let endpoints = unsafe { enumerator.EnumAudioEndpoints(eRender, DEVICE_STATE_ACTIVE) }
        .map_err(|_| Error::Unavailable)?;
    let count = unsafe { endpoints.GetCount() }
        .map_err(|_| Error::Unavailable)?
        .min(16);
    let default = default_id(enumerator);
    let mut candidates = Vec::new();
    for i in 0..count {
        if !allowed() {
            return Err(Error::Cancelled);
        }
        let Ok(device) = (unsafe { endpoints.Item(i) }) else {
            continue;
        };
        let Some(endpoint) = device_id(&device) else {
            continue;
        };
        let output = friendly(&device);
        let mut groups = BTreeMap::<(u32, u64), Vec<IAudioSessionControl2>>::new();
        for control in sessions(&device).unwrap_or_default() {
            if !allowed() {
                return Err(Error::Cancelled);
            }
            if !unsafe { control.GetState() }.is_ok_and(|state| state != AudioSessionStateExpired) {
                continue;
            }
            if let Some(key) = process(&control) {
                groups.entry(key).or_default().push(control);
            }
        }
        for ((pid, birth), controls) in groups {
            if controls.len() > 16 {
                continue;
            }
            let Some(image) = vults_peer::process_image(pid) else {
                continue;
            };
            let Some(app) = image
                .file_name()
                .and_then(|name| name.to_str())
                .map(str::to_owned)
            else {
                continue;
            };
            let mut ids = Vec::new();
            let mut levels = Vec::new();
            for control in &controls {
                let Some(id) = instance(control) else { break };
                let Ok((level, _)) = volume(control) else { break };
                ids.push(id);
                levels.push(level);
            }
            if ids.len() != controls.len()
                || validate_levels(&levels).is_err()
                || vults_peer::process_birth(pid) != Some(Some(birth))
            {
                continue;
            }
            ids.sort();
            let selection = Selection {
                endpoint: endpoint.clone(),
                default: default.clone(),
                pid,
                birth,
                instances: ids,
            };
            candidates.push(Candidate {
                selection,
                app,
                output: output.clone(),
                pid,
                percent: percent(&levels),
            });
        }
    }
    Ok(candidates)
}
pub async fn list(allowed: impl Fn() -> bool + Send + Sync + 'static) -> Result<Vec<Candidate>, Error> {
    let (send, receive) = oneshot::channel();
    std::thread::Builder::new()
        .name("player-audio-list".into())
        .spawn(move || {
            let result = (|| {
                let _apartment = initialize()?;
                enumerate(&enumerator()?, &|| {
                    !send.is_closed() && allowed() && !send.is_closed()
                })
            })();
            let _ = send.send(result);
        })
        .map_err(|_| Error::Unavailable)?;
    tokio::time::timeout(TIMEOUT, receive)
        .await
        .map_err(|_| Error::Unavailable)?
        .map_err(|_| Error::Unavailable)?
}

enum Request {
    Apply {
        action: Option<Action>,
        allowed: Gate,
        reply: oneshot::Sender<Result<u8, Error>>,
    },
}
#[derive(Debug)]
pub struct Target(mpsc::Sender<Request>);
pub async fn pin(
    selection: Selection,
    allowed: impl Fn() -> bool + Send + Sync + 'static,
) -> Result<Target, Error> {
    if !allowed() {
        return Err(Error::Cancelled);
    }
    let (send, receive) = mpsc::channel();
    let (ready, result) = oneshot::channel();
    std::thread::Builder::new()
        .name("player-audio-volume".into())
        .spawn(move || {
            let native = match Native::new(selection) {
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
            if !native.valid() || !allowed() || ready.is_closed() {
                let _ = ready.send(Err(Error::Changed));
                return;
            }
            if ready.send(Ok(())).is_err() {
                return;
            }
            while let Ok(Request::Apply {
                action,
                allowed,
                reply,
            }) = receive.recv()
            {
                let gate = || !reply.is_closed() && allowed() && !reply.is_closed();
                let result = match action {
                    Some(action) => apply(&native, action, &gate),
                    None => guard(&native, &gate)
                        .and_then(|()| native.snapshot())
                        .and_then(|snapshot| {
                            validate_levels(&snapshot.levels).map(|()| percent(&snapshot.levels))
                        }),
                };
                let _ = reply.send(result);
            }
        })
        .map_err(|_| Error::Unavailable)?;
    tokio::time::timeout(TIMEOUT, result)
        .await
        .map_err(|_| Error::Unavailable)?
        .map_err(|_| Error::Unavailable)??;
    Ok(Target(send))
}
impl Target {
    pub async fn validate(&self, allowed: impl Fn() -> bool + Send + Sync + 'static) -> Result<(), Error> {
        self.request(None, allowed).await.map(|_| ())
    }
    pub async fn apply(
        &self,
        action: Action,
        allowed: impl Fn() -> bool + Send + Sync + 'static,
    ) -> Result<u8, Error> {
        self.request(Some(action), allowed).await
    }
    async fn request(
        &self,
        action: Option<Action>,
        allowed: impl Fn() -> bool + Send + Sync + 'static,
    ) -> Result<u8, Error> {
        if !allowed() {
            return Err(Error::Cancelled);
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
#[derive(Clone, Debug, PartialEq)]
struct Snapshot {
    levels: Vec<f32>,
    muted: Vec<bool>,
}
trait Io {
    fn valid(&self) -> bool;
    fn snapshot(&self) -> Result<Snapshot, Error>;
    fn set(&self, index: usize, value: f32) -> Result<(), Error>;
}
fn validate_levels(levels: &[f32]) -> Result<(), Error> {
    if levels.is_empty()
        || levels.len() > 16
        || levels.iter().any(|v| !v.is_finite() || !(0.0..=1.0).contains(v))
    {
        Err(Error::Invalid)
    } else {
        Ok(())
    }
}
fn percent(levels: &[f32]) -> u8 {
    (levels.iter().copied().fold(0.0_f32, f32::max) * 100.0).round() as u8
}
fn guard(io: &impl Io, allowed: &dyn Fn() -> bool) -> Result<(), Error> {
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
fn apply(io: &impl Io, action: Action, allowed: &dyn Fn() -> bool) -> Result<u8, Error> {
    if matches!(action, Action::Volume(v) if v > 100) {
        return Err(Error::Invalid);
    }
    guard(io, allowed)?;
    let before = io.snapshot()?;
    validate_levels(&before.levels)?;
    if before.muted.len() != before.levels.len() {
        return Err(Error::Invalid);
    }
    let level = match action {
        Action::Volume(v) => v,
        Action::Louder => percent(&before.levels).saturating_add(10).min(100),
        Action::Quieter => percent(&before.levels).saturating_sub(10),
    };
    let max = before.levels.iter().copied().fold(0.0_f32, f32::max);
    let expected: Vec<_> = before
        .levels
        .iter()
        .map(|v| {
            if max > 0.0 {
                v / max * f32::from(level) / 100.0
            } else {
                f32::from(level) / 100.0
            }
        })
        .collect();
    for (index, &value) in expected.iter().enumerate() {
        guard(io, allowed)?;
        if (before.levels[index] - value).abs() > 0.00001 {
            io.set(index, value)?;
        }
    }
    guard(io, allowed)?;
    let after = io.snapshot()?;
    if after.muted != before.muted
        || after.levels.len() != expected.len()
        || after
            .levels
            .iter()
            .zip(&expected)
            .any(|(a, b)| !a.is_finite() || (a - b).abs() > 0.0001)
    {
        return Err(Error::Changed);
    }
    Ok(percent(&after.levels))
}

#[windows::core::implement(IAudioSessionEvents)]
struct Watch {
    invalid: Arc<AtomicBool>,
    context: GUID,
}
impl Watch {
    fn foreign(&self, context: *const GUID) {
        if unsafe { context.as_ref() } != Some(&self.context) {
            self.invalid.store(true, Ordering::Release);
        }
    }
}
impl IAudioSessionEvents_Impl for Watch_Impl {
    fn OnDisplayNameChanged(&self, _: &PCWSTR, _: *const GUID) -> windows::core::Result<()> {
        Ok(())
    }
    fn OnIconPathChanged(&self, _: &PCWSTR, _: *const GUID) -> windows::core::Result<()> {
        Ok(())
    }
    fn OnSimpleVolumeChanged(
        &self,
        _: f32,
        _: windows::core::BOOL,
        context: *const GUID,
    ) -> windows::core::Result<()> {
        self.foreign(context);
        Ok(())
    }
    fn OnChannelVolumeChanged(
        &self,
        _: u32,
        _: *const f32,
        _: u32,
        context: *const GUID,
    ) -> windows::core::Result<()> {
        self.foreign(context);
        Ok(())
    }
    fn OnGroupingParamChanged(&self, _: *const GUID, _: *const GUID) -> windows::core::Result<()> {
        Ok(())
    }
    fn OnStateChanged(&self, state: AudioSessionState) -> windows::core::Result<()> {
        if state == AudioSessionStateExpired {
            self.invalid.store(true, Ordering::Release);
        }
        Ok(())
    }
    fn OnSessionDisconnected(&self, _: AudioSessionDisconnectReason) -> windows::core::Result<()> {
        self.invalid.store(true, Ordering::Release);
        Ok(())
    }
}
struct Stream {
    control: IAudioSessionControl2,
    volume: ISimpleAudioVolume,
    watch: IAudioSessionEvents,
}
impl Drop for Stream {
    fn drop(&mut self) {
        let _ = unsafe { self.control.UnregisterAudioSessionNotification(&self.watch) };
    }
}
struct Native {
    enumerator: IMMDeviceEnumerator,
    device: IMMDevice,
    selection: Selection,
    streams: Vec<Stream>,
    invalid: Arc<AtomicBool>,
    context: GUID,
    device_watch: IMMNotificationClient,
    _apartment: Apartment,
}
impl Native {
    fn new(selection: Selection) -> Result<Self, Error> {
        let apartment = initialize()?;
        let enumerator = enumerator()?;
        if default_id(&enumerator) != selection.default {
            return Err(Error::Changed);
        }
        let wide: Vec<_> = selection.endpoint.encode_utf16().chain(Some(0)).collect();
        let device = unsafe { enumerator.GetDevice(PCWSTR(wide.as_ptr())) }.map_err(|_| Error::Changed)?;
        let invalid = Arc::new(AtomicBool::new(false));
        let context = unsafe { CoCreateGuid() }.map_err(|_| Error::Unavailable)?;
        let mut streams = Vec::new();
        for control in sessions(&device)? {
            if process(&control) != Some((selection.pid, selection.birth))
                || !instance(&control).is_some_and(|id| selection.instances.contains(&id))
            {
                continue;
            }
            let volume = control.cast().map_err(|_| Error::Unavailable)?;
            let watch: IAudioSessionEvents = Watch {
                invalid: invalid.clone(),
                context,
            }
            .into();
            unsafe { control.RegisterAudioSessionNotification(&watch) }.map_err(|_| Error::Unavailable)?;
            streams.push(Stream {
                control,
                volume,
                watch,
            });
        }
        streams.sort_by_key(|stream| instance(&stream.control));
        if streams.len() != selection.instances.len() {
            return Err(Error::Changed);
        }
        let device_watch: IMMNotificationClient = DeviceWatch {
            endpoint: selection.endpoint.clone(),
            invalid: invalid.clone(),
        }
        .into();
        unsafe { enumerator.RegisterEndpointNotificationCallback(&device_watch) }
            .map_err(|_| Error::Unavailable)?;
        let native = Self {
            enumerator,
            device,
            selection,
            streams,
            invalid,
            context,
            device_watch,
            _apartment: apartment,
        };
        validate_levels(&native.snapshot()?.levels)?;
        Ok(native)
    }
}
impl Drop for Native {
    fn drop(&mut self) {
        let _ = unsafe {
            self.enumerator
                .UnregisterEndpointNotificationCallback(&self.device_watch)
        };
    }
}

#[windows::core::implement(IMMNotificationClient)]
struct DeviceWatch {
    endpoint: String,
    invalid: Arc<AtomicBool>,
}
impl DeviceWatch {
    fn changed(&self, id: &PCWSTR) {
        if unsafe { id.to_string() }.is_ok_and(|id| id == self.endpoint) {
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
    fn OnPropertyValueChanged(&self, id: &PCWSTR, _: &PROPERTYKEY) -> windows::core::Result<()> {
        self.changed(id);
        Ok(())
    }
}
impl Io for Native {
    fn valid(&self) -> bool {
        if self.invalid.load(Ordering::Acquire)
            || default_id(&self.enumerator) != self.selection.default
            || unsafe { self.device.GetState() } != Ok(DEVICE_STATE_ACTIVE)
            || vults_peer::process_birth(self.selection.pid) != Some(Some(self.selection.birth))
        {
            return false;
        }
        let Ok(controls) = sessions(&self.device) else {
            return false;
        };
        let mut ids:Vec<_>=controls.iter().filter(|control|unsafe {control.GetState()}!=Ok(AudioSessionStateExpired)&&process(control)==Some((self.selection.pid,self.selection.birth))).filter_map(instance).collect();
        ids.sort();
        ids == self.selection.instances && !self.invalid.load(Ordering::Acquire)
    }
    fn snapshot(&self) -> Result<Snapshot, Error> {
        let values = self
            .streams
            .iter()
            .map(|s| volume(&s.control))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Snapshot {
            levels: values.iter().map(|v| v.0).collect(),
            muted: values.iter().map(|v| v.1).collect(),
        })
    }
    fn set(&self, index: usize, value: f32) -> Result<(), Error> {
        unsafe { self.streams[index].volume.SetMasterVolume(value, &self.context) }.map_err(|_| Error::Failed)
    }
}

#[cfg(test)]
#[path = "player_audio_tests.rs"]
mod tests;
