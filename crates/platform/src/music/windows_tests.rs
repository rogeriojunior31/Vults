use super::*;
use windows::Media::{
    MediaPlaybackStatus, MediaPlaybackType, Playback::MediaPlayer, SystemMediaTransportControls,
    SystemMediaTransportControlsButton, SystemMediaTransportControlsButtonPressedEventArgs,
};

#[test]
#[ignore = "requires the interactive Windows media-session service; publishes only a silent test session"]
fn native_owned_session_pause_and_resume() {
    unsafe { RoInitialize(RO_INIT_MULTITHREADED).unwrap() };
    let _apartment = Apartment;
    let identity = format!("Vults.DictationMediaTest.{}", std::process::id());
    let wide: Vec<_> = identity.encode_utf16().chain(Some(0)).collect();
    unsafe {
        windows::Win32::UI::Shell::SetCurrentProcessExplicitAppUserModelID(windows::core::PCWSTR(
            wide.as_ptr(),
        ))
        .unwrap();
    }
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .unwrap();
    runtime.block_on(async {
        let player = MediaPlayer::new().unwrap();
        player.CommandManager().unwrap().SetIsEnabled(false).unwrap();
        let controls = player.SystemMediaTransportControls().unwrap();
        controls.SetIsEnabled(true).unwrap();
        controls.SetIsPlayEnabled(true).unwrap();
        controls.SetIsPauseEnabled(true).unwrap();
        let display = controls.DisplayUpdater().unwrap();
        display.SetType(MediaPlaybackType::Music).unwrap();
        display
            .MusicProperties()
            .unwrap()
            .SetTitle(&windows::core::HSTRING::from("Vults silent dictation test"))
            .unwrap();
        display.Update().unwrap();
        let pauses = Arc::new(AtomicUsize::new(0));
        let plays = Arc::new(AtomicUsize::new(0));
        let (pause_count, play_count) = (pauses.clone(), plays.clone());
        let handler =
            controls
                .ButtonPressed(
                    &TypedEventHandler::new(
                        move |sender: windows::core::Ref<SystemMediaTransportControls>,
                              args: windows::core::Ref<
                            SystemMediaTransportControlsButtonPressedEventArgs,
                        >| {
                            let (Some(sender), Some(args)) = (sender.as_ref(), args.as_ref()) else {
                                return Ok(());
                            };
                            match args.Button()? {
                                SystemMediaTransportControlsButton::Pause => {
                                    pause_count.fetch_add(1, Ordering::AcqRel);
                                    sender.SetPlaybackStatus(MediaPlaybackStatus::Paused)?;
                                }
                                SystemMediaTransportControlsButton::Play => {
                                    play_count.fetch_add(1, Ordering::AcqRel);
                                    std::thread::sleep(Duration::from_millis(100));
                                    sender.SetPlaybackStatus(MediaPlaybackStatus::Playing)?;
                                }
                                _ => {}
                            }
                            Ok(())
                        },
                    ),
                )
                .unwrap();
        controls.SetPlaybackStatus(MediaPlaybackStatus::Playing).unwrap();
        let manager = Manager::RequestAsync().unwrap().await.unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        let session = loop {
            let session = manager
                .GetSessions()
                .unwrap()
                .into_iter()
                .find(|session| session.SourceAppUserModelId().is_ok_and(|id| id == identity));
            if let Some(session) = session {
                break session;
            }
            assert!(Instant::now() < deadline, "owned SMTC fixture was not registered");
            tokio::time::sleep(TICK).await;
        };
        while !playing(&session, Playback::Playing) || !super::controls(&session, true) {
            assert!(
                Instant::now() < deadline,
                "owned SMTC fixture did not publish playback controls: {:?}, {:?}",
                session.GetPlaybackInfo().and_then(|info| info.PlaybackStatus()),
                session
                    .GetPlaybackInfo()
                    .and_then(|info| info.Controls())
                    .map(|controls| (controls.IsPauseEnabled(), controls.IsPlayEnabled()))
            );
            tokio::time::sleep(TICK).await;
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
        let end = Instant::now() + Duration::from_millis(700);
        hold_sessions(|| Instant::now() < end, &manager, vec![session.clone()]).await;
        assert!(
            playing(&session, Playback::Playing),
            "restoration returned before the delayed player update"
        );
        assert_eq!(
            pauses.load(Ordering::Acquire),
            1,
            "the fixture must actually receive Pause"
        );
        assert_eq!(
            plays.load(Ordering::Acquire),
            1,
            "the fixture must actually receive Play"
        );
        controls.SetPlaybackStatus(MediaPlaybackStatus::Paused).unwrap();
        tokio::time::sleep(Duration::from_millis(150)).await;
        let end = Instant::now() + Duration::from_millis(150);
        hold_sessions(|| Instant::now() < end, &manager, vec![session.clone()]).await;
        assert_eq!(
            pauses.load(Ordering::Acquire),
            1,
            "already-paused media is left alone"
        );
        assert_eq!(plays.load(Ordering::Acquire), 1);

        controls.SetPlaybackStatus(MediaPlaybackStatus::Playing).unwrap();
        tokio::time::sleep(Duration::from_millis(150)).await;
        hold_sessions(|| false, &manager, vec![session.clone()]).await;
        assert_eq!(
            pauses.load(Ordering::Acquire),
            1,
            "revoked admission never pauses"
        );

        let active = std::sync::atomic::AtomicBool::new(true);
        let manual = async {
            let deadline = Instant::now() + Duration::from_secs(2);
            while pauses.load(Ordering::Acquire) != 2 || !playing(&session, Playback::Paused) {
                assert!(Instant::now() < deadline, "second pause was not observed");
                tokio::time::sleep(TICK).await;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
            controls.SetPlaybackStatus(MediaPlaybackStatus::Playing).unwrap();
            tokio::time::sleep(Duration::from_millis(100)).await;
            controls.SetPlaybackStatus(MediaPlaybackStatus::Paused).unwrap();
            tokio::time::sleep(Duration::from_millis(100)).await;
            active.store(false, Ordering::Release);
        };
        tokio::join!(
            hold_sessions(|| active.load(Ordering::Acquire), &manager, vec![session.clone()]),
            manual
        );
        assert_eq!(
            plays.load(Ordering::Acquire),
            1,
            "a manual play/pause relinquishes automatic resume"
        );
        assert!(playing(&session, Playback::Paused));

        controls.SetPlaybackStatus(MediaPlaybackStatus::Playing).unwrap();
        tokio::time::sleep(Duration::from_millis(150)).await;
        let deadline = Instant::now() + Duration::from_secs(2);
        hold_sessions(
            || pauses.load(Ordering::Acquire) < 3 && Instant::now() < deadline,
            &manager,
            vec![session.clone()],
        )
        .await;
        let deadline = Instant::now() + Duration::from_secs(2);
        while !playing(&session, Playback::Playing) {
            assert!(
                Instant::now() < deadline,
                "immediate cancellation did not restore the owned pause"
            );
            tokio::time::sleep(TICK).await;
        }
        assert_eq!(plays.load(Ordering::Acquire), 2);

        let active = std::sync::atomic::AtomicBool::new(true);
        let change_track = async {
            let deadline = Instant::now() + Duration::from_secs(2);
            while pauses.load(Ordering::Acquire) != 4 || !playing(&session, Playback::Paused) {
                assert!(Instant::now() < deadline, "track-change pause was not observed");
                tokio::time::sleep(TICK).await;
            }
            display
                .MusicProperties()
                .unwrap()
                .SetTitle(&windows::core::HSTRING::from("Vults replacement test track"))
                .unwrap();
            display.Update().unwrap();
            tokio::time::sleep(Duration::from_millis(150)).await;
            active.store(false, Ordering::Release);
        };
        tokio::join!(
            hold_sessions(|| active.load(Ordering::Acquire), &manager, vec![session.clone()]),
            change_track
        );
        assert_eq!(
            plays.load(Ordering::Acquire),
            2,
            "a changed track must stay paused"
        );
        assert!(playing(&session, Playback::Paused));

        controls.RemoveButtonPressed(handler).unwrap();
        controls.SetIsEnabled(false).unwrap();
        player.Close().unwrap();
    });
}
