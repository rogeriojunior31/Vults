//! Backend service for V6. No renderer or microphone entry point is exposed before V6.7.
use serde::{Deserialize, Serialize};
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, Ordering},
};
use tauri::{AppHandle, Manager};
use vults_chat::realtime::{
    Plan,
    transport::{self, Failure, OpenAi, Provider},
};

mod local;
mod playback;
pub use local::{LocalFailure, Recovery, Turn};
pub use playback::{Channels, Command, Event, Input, LocalReply, Record, Signal};
#[cfg(target_os = "linux")]
mod consultation;
mod devices;
#[cfg(target_os = "linux")]
pub(crate) mod routing;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Consent {
    pub on: bool,
    pub ledger: Option<String>,
    pub limit_microusd: u64,
    pub model: String,
}
impl Default for Consent {
    fn default() -> Self {
        Self {
            on: false,
            ledger: None,
            limit_microusd: 1_000_000,
            model: "gpt-realtime-2.1-mini".into(),
        }
    }
}
impl Consent {
    pub(crate) fn enabled(&self) -> bool {
        self.on
            && self.ledger.as_ref().is_some_and(|s| !s.is_empty())
            && self.limit_microusd > 0
            && self.limit_microusd <= i64::MAX as u64
            && Plan::new(&self.model, 8192, 1024).is_ok()
    }
}

#[derive(Debug, Default)]
pub(crate) struct State {
    closed: AtomicBool,
    epoch: AtomicU64,
    running: AtomicBool,
}
impl State {
    fn begin(self: &Arc<Self>) -> Result<(Lease, u64), Failure> {
        let epoch = self.epoch.load(Ordering::SeqCst);
        if self.closed.load(Ordering::SeqCst) {
            return Err(Failure::Off);
        }
        if self.running.swap(true, Ordering::SeqCst) {
            return Err(Failure::Busy);
        }
        Ok((Lease(self.clone()), epoch))
    }
    fn current(&self, epoch: u64) -> bool {
        !self.closed.load(Ordering::SeqCst)
            && self.running.load(Ordering::SeqCst)
            && self.epoch.load(Ordering::SeqCst) == epoch
    }
    fn key_changed(&self, provider: &str) {
        if provider == "openai" {
            self.epoch.fetch_add(1, Ordering::SeqCst);
        }
    }
    fn close(&self) {
        self.closed.store(true, Ordering::SeqCst);
        self.epoch.fetch_add(1, Ordering::SeqCst);
    }
}
pub(crate) fn invalidate<R: tauri::Runtime>(app: &tauri::AppHandle<R>) {
    if let Some(state) = app.try_state::<Arc<State>>() {
        state.epoch.fetch_add(1, Ordering::SeqCst);
    }
}
pub(crate) fn shutdown<R: tauri::Runtime>(app: &tauri::AppHandle<R>) {
    if let Some(state) = app.try_state::<Arc<State>>() {
        state.close();
    }
}
pub(crate) fn key_changed<R: tauri::Runtime>(app: &tauri::AppHandle<R>, provider: &str) {
    if let Some(state) = app.try_state::<Arc<State>>() {
        state.key_changed(provider);
    }
}
struct Lease(Arc<State>);
impl Drop for Lease {
    fn drop(&mut self) {
        self.0.epoch.fetch_add(1, Ordering::SeqCst);
        self.0.running.store(false, Ordering::SeqCst);
    }
}

/// Complete native turns only. Recovery is a reviewable draft, never an automatic Chat send.
pub async fn run(app: AppHandle, channels: Channels) -> Result<Option<Recovery>, Failure> {
    run_with_audio(app, channels, devices::Choice::Default).await
}

/// Explicit native duplex admission; no renderer entry or automatic device choice.
#[cfg(target_os = "linux")]
pub async fn run_routed(
    app: AppHandle,
    channels: Channels,
    route: vults_audio::echo::duplex::Route,
) -> Result<Option<Recovery>, Failure> {
    run_with_audio(app, channels, devices::Choice::Routed(route)).await
}

async fn run_with_audio(
    app: AppHandle,
    channels: Channels,
    choice: devices::Choice,
) -> Result<Option<Recovery>, Failure> {
    let state = app.state::<Arc<State>>().inner().clone();
    let (lease, epoch) = state.begin()?;
    let lease = Arc::new(lease);
    if crate::lock::locked(&app) {
        return Err(Failure::Off);
    }
    let (consent, model, language, zeca) = app
        .state::<crate::settings::SettingsState>()
        .0
        .lock()
        .map(|s| {
            (
                s.voice_realtime.clone(),
                s.voice_model.clone(),
                s.voice_language.clone(),
                s.zeca,
            )
        })
        .map_err(|_| Failure::Off)?;
    if !zeca {
        return Err(Failure::Off);
    }
    let expected = consent.clone();
    let decode_language = crate::voice::language(&app);
    let cancel = channels.cancel.clone();
    let check_app = app.clone();
    let publication = channels.output.clone();
    let (selected, spoken, cancellation) = (model.clone(), language.clone(), cancel.clone());
    let valid: Arc<dyn Fn(&crate::settings::Settings) -> bool + Send + Sync> = Arc::new(move |s| {
        state.current(epoch)
            && !publication.is_closed()
            && !*cancellation.borrow()
            && cancellation.has_changed().is_ok()
            && !crate::lock::locked(&check_app)
            && s.zeca
            && s.voice_realtime == expected
            && s.voice_model == selected
            && s.voice_language == spoken
    });
    let app_check = app.clone();
    let authority = valid.clone();
    let active: Arc<dyn Fn() -> bool + Send + Sync> = Arc::new(move || {
        app_check
            .state::<crate::settings::SettingsState>()
            .0
            .lock()
            .is_ok_and(|s| authority(&s))
    });
    let source: Arc<dyn crate::flock_tools::Source> = Arc::new(app.clone());
    #[allow(unused_mut)]
    let mut tools = crate::flock_tools::all(source);
    #[cfg(target_os = "linux")]
    consultation::offer(
        app.clone(),
        lease.clone(),
        valid.clone(),
        channels.output.clone(),
        &mut tools,
    );
    #[cfg(target_os = "linux")]
    let router = routing::Router::new(lease.clone(), valid);
    let remote_active = active.clone();
    if choice.captures() && !consent.enabled() {
        return Err(Failure::Off);
    }
    local::check(&*active, &cancel)?;
    let devices = Arc::new(devices::Devices::open(choice, lease.clone(), active.clone())?);
    while !devices.ready()? {
        local::check(&*active, &cancel)?;
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    }
    local::check(&*active, &cancel)?;
    let output = channels.output.clone();
    let factory = devices.clone();
    let outcome = {
        let driver = playback::drive(
            channels,
            &*active,
            devices.interruption(),
            move |cancel| factory.output(cancel),
            move |io| connect(consent, remote_active, io, tools),
            {
                let app = app.clone();
                move |turn, deadline| {
                    #[cfg(target_os = "linux")]
                    {
                        router.clone().prepare(app.clone(), turn, deadline)
                    }
                    #[cfg(not(target_os = "linux"))]
                    {
                        let _ = (&app, turn, deadline);
                        std::future::ready(Ok(None))
                    }
                }
            },
        );
        tokio::pin!(driver);
        let mut poll = tokio::time::interval(std::time::Duration::from_millis(5));
        loop {
            tokio::select! { biased;
                result = &mut driver => break result?,
                _ = poll.tick() => {
                    local::check(&*active, &cancel)?;
                    while let Some(samples) = devices.input()? {
                        output.try_send(Event::Input(Input { samples })).map_err(|_| Failure::Output)?;
                    }
                },
            }
        }
    };
    drop(devices);
    let Some((turn, reason)) = outcome else {
        return Ok(None);
    };
    local::recover(turn, reason, active, cancel, lease, move |pcm, stop| {
        if stop.load(Ordering::SeqCst) {
            return Err(LocalFailure::Decode);
        }
        let model = model.ok_or(LocalFailure::Unavailable)?;
        let directory = crate::voice::models_dir();
        if !vults_voice::installed(&directory).contains(&model.as_str()) {
            return Err(LocalFailure::Unavailable);
        }
        let native = crate::voice::transcriber(&app, &model).map_err(|_| LocalFailure::Decode)?;
        if stop.load(Ordering::SeqCst) {
            return Err(LocalFailure::Decode);
        }
        let input: Vec<f32> = pcm.iter().map(|s| f32::from(*s) / 32768.0).collect();
        let pcm = vults_audio::resample(&input, 1, 24_000).map_err(|_| LocalFailure::Decode)?;
        native
            .transcribe_cancellable(
                &pcm,
                decode_language.as_deref(),
                vults_voice::vad_path(&directory).as_deref(),
                &stop,
                true,
            )
            .map_err(|_| LocalFailure::Decode)?
            .ok_or(LocalFailure::Decode)
    })
    .await
    .map(Some)
}

async fn connect(
    consent: Consent,
    active: Arc<dyn Fn() -> bool + Send + Sync>,
    channels: transport::Channels,
    tools: vults_chat::tools::Tools,
) -> Result<(), Failure> {
    if !consent.enabled() {
        return Err(if consent.limit_microusd == 0 {
            Failure::Budget
        } else {
            Failure::Off
        });
    }
    let plan = Plan::new(&consent.model, 8192, 1024)
        .map_err(|_| Failure::Budget)?
        .with_tools(tools)
        .map_err(|_| Failure::Protocol)?;
    let cancel = channels.cancel.clone();
    let gate = crate::gate::realtime::RealtimeGate::open(
        crate::history::dir(),
        consent,
        Arc::new(move || active() && !*cancel.borrow() && cancel.has_changed().is_ok()),
    )
    .await?;
    OpenAi.run(plan, &gate, channels).await
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(target_os = "linux")]
    use vults_chat::realtime::transport::Gate;

    #[test]
    fn realtime_offers_only_the_existing_five_read_tools() {
        struct Cached;
        impl crate::flock_tools::Source for Cached {
            fn view(&self) -> Option<serde_json::Value> {
                Some(serde_json::json!({"sessions":[]}))
            }
            fn usage(&self) -> serde_json::Value {
                serde_json::json!([])
            }
        }
        let tools = crate::flock_tools::all(Arc::new(Cached));
        let plan = Plan::new("gpt-realtime-2.1-mini", 8192, 1024)
            .unwrap()
            .with_tools(tools)
            .unwrap();
        let update = plan.session_update();
        let offered = update["session"]["tools"].as_array().unwrap();
        assert_eq!(
            offered
                .iter()
                .map(|v| v["name"].as_str().unwrap())
                .collect::<Vec<_>>(),
            [
                "flock_list",
                "session_detail",
                "pending_asks",
                "usage",
                "github_snapshot"
            ]
        );
        for spec in offered {
            assert_eq!(spec["parameters"]["additionalProperties"], false);
            assert_eq!(
                spec["parameters"]["properties"].as_object().unwrap().len(),
                usize::from(spec["name"] == "session_detail")
            );
        }
    }

    #[cfg(not(windows))]
    #[test]
    fn settings_restored_before_the_next_poll_cannot_revive_an_old_turn() {
        let app = tauri::test::mock_builder()
            .build(tauri::test::mock_context(tauri::test::noop_assets()))
            .unwrap();
        let state = Arc::new(State::default());
        app.manage(state.clone());
        let mut settings = crate::settings::Settings::default();
        type Change = fn(&mut crate::settings::Settings);
        for (change, restore) in [
            (
                (|s| s.voice_model = Some("small".into())) as Change,
                (|s| s.voice_model = None) as Change,
            ),
            (
                (|s| s.voice_language = Some("pt".into())) as Change,
                (|s| s.voice_language = None) as Change,
            ),
            ((|s| s.zeca = false) as Change, (|s| s.zeca = true) as Change),
            (
                (|s| s.dictation_microphone = Some("test".into())) as Change,
                (|s| s.dictation_microphone = None) as Change,
            ),
        ] {
            let (lease, epoch) = state.begin().unwrap();
            crate::settings::apply_change(app.handle(), &mut settings, change);
            crate::settings::apply_change(app.handle(), &mut settings, restore);
            assert!(!state.current(epoch));
            drop(lease);
        }
        let (_lease, epoch) = state.begin().unwrap();
        crate::settings::apply_change(app.handle(), &mut settings, |_| {});
        assert!(state.current(epoch));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn microphone_setter_revokes_old_turns_without_touching_user_settings() {
        use std::os::unix::fs::DirBuilderExt;
        use std::path::PathBuf;
        const CHILD: &str = "VULTS_MICROPHONE_SETTER_TEST";
        let Some(root) = std::env::var_os(CHILD).map(PathBuf::from) else {
            let root =
                std::env::temp_dir().join(format!("vults-microphone-setter-{}", crate::traces::new_id()));
            std::fs::DirBuilder::new().mode(0o700).create(&root).unwrap();
            let result = std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "realtime::tests::microphone_setter_revokes_old_turns_without_touching_user_settings",
                    "--nocapture",
                ])
                .env(CHILD, &root)
                .env("XDG_CONFIG_HOME", &root)
                .output();
            let _ = std::fs::remove_dir_all(&root);
            let output = result.unwrap();
            assert!(
                output.status.success(),
                "isolated setter regression failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            return;
        };
        assert_eq!(
            std::env::var_os("XDG_CONFIG_HOME"),
            Some(root.clone().into_os_string())
        );
        assert_eq!(root.parent(), Some(std::env::temp_dir().as_path()));
        assert!(
            root.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("vults-microphone-setter-")
        );
        assert!(vults_peer::is_private_dir(&root));
        let app = tauri::test::mock_app();
        let state = Arc::new(State::default());
        app.manage(state.clone());
        app.manage(crate::settings::SettingsState(std::sync::Mutex::new(
            crate::settings::Settings::default(),
        )));
        let settings = tauri::WebviewWindowBuilder::new(&app, crate::SETTINGS, tauri::WebviewUrl::default())
            .build()
            .unwrap();
        let set = |device| {
            crate::settings::set_dictation_microphone(app.handle().clone(), settings.as_ref().clone(), device)
        };
        let (lease, epoch) = state.begin().unwrap();
        set(Some("Synthetic microphone".into())).unwrap();
        set(None).unwrap();
        assert!(
            !state.current(epoch),
            "restoring the device cannot restore the old turn"
        );
        drop(lease);
        let (_lease, epoch) = state.begin().unwrap();
        set(None).unwrap();
        assert!(state.current(epoch), "unchanged device must not revoke a turn");
        let directory = root.join(vults_brand::SLUG);
        std::fs::remove_dir_all(&directory).unwrap();
        std::fs::write(&directory, "blocked save").unwrap();
        assert!(set(Some("Other synthetic microphone".into())).is_err());
        assert!(
            state.current(epoch),
            "failed persistence must not publish a device change"
        );
        assert!(
            app.state::<crate::settings::SettingsState>()
                .0
                .lock()
                .unwrap()
                .dictation_microphone
                .is_none()
        );
    }

    #[test]
    fn revocation_between_entry_and_snapshot_and_shutdown_cannot_be_undone() {
        let state = Arc::new(State::default());
        let (lease, epoch) = state.begin().unwrap();
        assert!(state.current(epoch));
        assert_eq!(state.begin().err(), Some(Failure::Busy));
        state.epoch.fetch_add(1, Ordering::SeqCst);
        assert!(!state.current(epoch));
        drop(lease);
        let (next, new_epoch) = state.begin().unwrap();
        assert!(state.current(new_epoch));
        assert!(!state.current(epoch));
        state.close();
        assert!(!state.current(new_epoch));
        drop(next);
        assert_eq!(state.begin().err(), Some(Failure::Off));
    }

    #[test]
    fn old_settings_and_zero_or_unbound_consent_never_enable_remote_voice() {
        let mut value = serde_json::to_value(crate::settings::Settings::default()).unwrap();
        value.as_object_mut().unwrap().remove("voice_realtime");
        let old: crate::settings::Settings = serde_json::from_value(value).unwrap();
        assert_eq!(old.voice_realtime, Consent::default());
        assert!(!old.voice_realtime.enabled());
        let mut consent = Consent {
            on: true,
            ..Consent::default()
        };
        assert!(!consent.enabled());
        consent.ledger = Some("identity".into());
        assert!(consent.enabled());
        consent.limit_microusd = 0;
        assert!(!consent.enabled());
        consent.limit_microusd = 1_000_000;
        consent.model = "unknown".into();
        assert!(!consent.enabled());
    }

    #[test]
    fn openai_key_changes_revoke_existing_connections_without_coupling_other_providers() {
        let state = Arc::new(State::default());
        let (lease, epoch) = state.begin().unwrap();
        state.key_changed("anthropic");
        assert!(state.current(epoch));
        state.key_changed("openai");
        assert!(!state.current(epoch));
        drop(lease);
        let (_lease, epoch) = state.begin().unwrap();
        state.key_changed("openai");
        assert!(!state.current(epoch));
    }

    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn durable_admission_survives_erasure_and_abandoned_handoff_never_leaks_allowance() {
        let dir = std::env::temp_dir().join(format!("vults-realtime-admission-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut db = vults_store::Store::open(&dir).unwrap();
        let identity = db.request_budget_id().unwrap();
        let gate = crate::gate::realtime::RealtimeGate::open(
            dir.clone(),
            Consent {
                on: true,
                ledger: Some(identity.clone()),
                limit_microusd: 179200,
                ..Consent::default()
            },
            Arc::new(|| true),
        )
        .await
        .unwrap();
        drop(gate.reserve(179200).await.unwrap());
        assert!(!db.has_open_cost(&identity, crate::gate::VOICE_REALTIME).unwrap());
        let attempt = gate.reserve(179200).await.unwrap().claim();
        assert!(gate.reserve(1).await.is_none());
        db.clear_all(&dir).unwrap();
        assert!(gate.reserve(1).await.is_none());
        assert!(gate.finish(&attempt, Some(2059), false).await);
        assert!(!gate.finish(&attempt, Some(0), true).await);
        let next = gate.reserve(177141).await.unwrap().claim();
        assert!(gate.finish(&next, None, false).await);
        assert!(gate.reserve(1).await.is_none());
        assert!(
            db.spans(20)
                .unwrap()
                .iter()
                .all(|s| !s.attrs.to_string().contains("audio"))
        );
        drop(db);
        std::fs::remove_dir_all(&dir).unwrap();
        vults_store::Store::open(&dir).unwrap();
        assert!(gate.reserve(1).await.is_none());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn detached_unclaimed_admission_is_refunded_even_when_authority_stays_valid() {
        let dir = std::env::temp_dir().join(format!("vults-realtime-handoff-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let db = vults_store::Store::open(&dir).unwrap();
        let identity = db.request_budget_id().unwrap();
        let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let checks = calls.clone();
        let release = Arc::new(std::sync::Barrier::new(2));
        let unblock = release.clone();
        let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
        let signal = std::sync::Mutex::new(Some(entered_tx));
        let gate = Arc::new(
            crate::gate::realtime::RealtimeGate::open(
                dir.clone(),
                Consent {
                    on: true,
                    ledger: Some(identity.clone()),
                    ..Consent::default()
                },
                Arc::new(move || {
                    if checks.fetch_add(1, Ordering::SeqCst) == 4 {
                        let _ = signal.lock().unwrap().take().unwrap().send(());
                        unblock.wait();
                    }
                    true
                }),
            )
            .await
            .unwrap(),
        );
        let pending = gate.clone();
        let task = tokio::spawn(async move { pending.reserve(179200).await });
        tokio::time::timeout(std::time::Duration::from_secs(2), entered_rx)
            .await
            .unwrap()
            .unwrap();
        assert!(db.has_open_cost(&identity, crate::gate::VOICE_REALTIME).unwrap());
        task.abort();
        let _ = task.await;
        let consent = Consent {
            on: true,
            ledger: Some(identity.clone()),
            ..Consent::default()
        };
        drop(gate);
        assert!(matches!(
            crate::gate::realtime::RealtimeGate::open(dir.clone(), consent.clone(), Arc::new(|| true)).await,
            Err(Failure::Busy)
        ));
        release.wait();
        tokio::time::timeout(std::time::Duration::from_secs(2), async {
            while db.has_open_cost(&identity, crate::gate::VOICE_REALTIME).unwrap() {
                tokio::time::sleep(std::time::Duration::from_millis(5)).await;
            }
        })
        .await
        .unwrap();
        assert_eq!(db.spans(10).unwrap()[0].attrs["vults.cost.actual_microusd"], 0);
        let next = tokio::time::timeout(std::time::Duration::from_secs(2), async {
            loop {
                match crate::gate::realtime::RealtimeGate::open(
                    dir.clone(),
                    consent.clone(),
                    Arc::new(|| true),
                )
                .await
                {
                    Err(Failure::Busy) => tokio::task::yield_now().await,
                    result => break result.unwrap(),
                }
            }
        })
        .await
        .unwrap();
        assert_eq!(db.spans(10).unwrap()[0].attrs["vults.cost.actual_microusd"], 0);
        drop(next);
        drop(db);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
