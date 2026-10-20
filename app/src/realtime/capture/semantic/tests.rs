use super::*;
use vults_voice::turn::{
    BargeIn,
    end::{Decision, EndOfTurn, Event as Edge, Language},
};
fn ticket() -> (vults_voice::turn::Generation, Ticket) {
    let mut turn = vults_voice::turn::Turn::new(BargeIn::Disabled);
    let generation = turn.start(Duration::ZERO).unwrap().generation;
    let mut end = EndOfTurn::new(true);
    end.begin(generation, Language::English, Duration::ZERO);
    end.handle(generation, Edge::SpeechStarted, Duration::ZERO);
    end.handle(generation, Edge::Transcript("real words"), Duration::ZERO);
    end.handle(generation, Edge::SpeechEnded, Duration::ZERO);
    let Decision::Score(ticket) = end.handle(generation, Edge::Tick, Duration::from_millis(1600)) else {
        panic!("score ticket")
    };
    (generation, ticket)
}
#[test]
fn the_worker_bounds_audio_and_admits_only_one_native_score() {
    let (_control, worker, _events, _cancel, _state) = super::super::tests::fixture();
    let (entered, waiting) = sync::sync_channel(1);
    let (release, released) = sync::sync_channel(1);
    let service = Service::scoring(
        worker.owner.lease.clone(),
        worker.owner.active.clone(),
        Box::new(move |pcm, _, until| {
            assert_eq!(pcm.len(), 128_000);
            assert!(until > Instant::now());
            entered.send(()).unwrap();
            released.recv().unwrap();
            Ok(0.9)
        }),
    )
    .unwrap();
    let (generation, ticket) = ticket();
    let job = service
        .request(
            generation,
            ticket,
            Instant::now() + Duration::from_secs(2),
            || vec![0.25; 200_000],
        )
        .unwrap();
    waiting.recv_timeout(Duration::from_secs(1)).unwrap();
    assert!(
        service
            .request(generation, ticket, job.until, || panic!(
                "Busy work cannot snapshot again"
            ))
            .is_none()
    );
    release.send(()).unwrap();
    let until = Instant::now() + Duration::from_secs(1);
    loop {
        if let Some(result) = job.poll() {
            assert_eq!(result, Ok(0.9));
            break;
        }
        assert!(Instant::now() < until);
        std::thread::sleep(Duration::from_millis(1));
    }
    assert_eq!(job.generation, generation);
    assert_eq!(job.ticket, ticket);
    assert!(!service.failed());
}
#[test]
fn cancellation_and_revocation_never_publish_scores_or_release_the_lease_before_native_exit() {
    for revoked in [false, true] {
        let (_control, worker, _events, _cancel, state) = super::super::tests::fixture();
        let (entered, waiting) = sync::sync_channel(1);
        let (release, released) = sync::sync_channel(1);
        let service = Service::scoring(
            worker.owner.lease.clone(),
            worker.owner.active.clone(),
            Box::new(move |_, _, _| {
                entered.send(()).unwrap();
                released.recv().unwrap();
                Ok(0.99)
            }),
        )
        .unwrap();
        let (generation, ticket) = ticket();
        let job = service
            .request(
                generation,
                ticket,
                Instant::now() + Duration::from_secs(2),
                Vec::new,
            )
            .unwrap();
        waiting.recv_timeout(Duration::from_secs(1)).unwrap();
        if revoked {
            state.epoch.fetch_add(1, Ordering::SeqCst);
        } else {
            job.stop.store(true, Ordering::SeqCst);
        }
        let start = Instant::now();
        drop(service);
        drop(worker);
        assert!(start.elapsed() < Duration::from_millis(100));
        assert!(state.running.load(Ordering::SeqCst));
        release.send(()).unwrap();
        let until = Instant::now() + Duration::from_secs(1);
        while state.running.load(Ordering::SeqCst) && Instant::now() < until {
            std::thread::sleep(Duration::from_millis(1));
        }
        assert!(!state.running.load(Ordering::SeqCst));
        assert!(!matches!(job.poll(), Some(Ok(_))));
    }
}

#[test]
fn panic_cleanup_keeps_native_destruction_under_the_real_lease() {
    struct NativeDrop {
        entered: sync::SyncSender<()>,
        release: sync::Receiver<()>,
    }
    impl Drop for NativeDrop {
        fn drop(&mut self) {
            let _ = self.entered.send(());
            let _ = self.release.recv();
        }
    }
    let (_control, worker, _events, _cancel, state) = super::super::tests::fixture();
    let (entered, waiting) = sync::sync_channel(1);
    let (release, released) = sync::sync_channel(1);
    let native = NativeDrop {
        entered,
        release: released,
    };
    let service = Service::scoring(
        worker.owner.lease.clone(),
        worker.owner.active.clone(),
        Box::new(move |_, _, _| {
            let _ = &native;
            panic!("controlled native score failure")
        }),
    )
    .unwrap();
    drop(worker);
    let (generation, ticket) = ticket();
    let job = service
        .request(
            generation,
            ticket,
            Instant::now() + Duration::from_secs(2),
            Vec::new,
        )
        .unwrap();
    waiting.recv_timeout(Duration::from_secs(1)).unwrap();
    assert_eq!(job.poll(), Some(Err(())));
    assert!(state.running.load(Ordering::SeqCst));
    drop(service);
    assert!(state.running.load(Ordering::SeqCst));
    release.send(()).unwrap();
    let until = Instant::now() + Duration::from_secs(1);
    while state.running.load(Ordering::SeqCst) && Instant::now() < until {
        std::thread::sleep(Duration::from_millis(1));
    }
    assert!(!state.running.load(Ordering::SeqCst));
}

#[tokio::test]
async fn preparation_timeout_or_revocation_returns_without_waiting_for_native_initialization() {
    for revoked in [false, true] {
        let (_control, worker, _events, _cancel, state) = super::super::tests::fixture();
        let (entered, waiting) = sync::sync_channel(1);
        let (release, released) = sync::sync_channel(1);
        let active = worker.owner.active.clone();
        let service = Service::start(worker.owner.lease.clone(), active.clone(), move |_| {
            entered.send(()).unwrap();
            released.recv().unwrap();
            Err(())
        })
        .unwrap();
        waiting.recv_timeout(Duration::from_secs(1)).unwrap();
        if revoked {
            state.epoch.fetch_add(1, Ordering::SeqCst);
        }
        let started = Instant::now();
        let result = service.prepare(&*active, Duration::from_millis(10)).await;
        assert_eq!(
            result,
            Err(if revoked {
                Failure::Cancelled
            } else {
                Failure::Timeout
            })
        );
        assert!(started.elapsed() < Duration::from_millis(100));
        drop(service);
        drop(worker);
        assert!(state.running.load(Ordering::SeqCst));
        release.send(()).unwrap();
        let until = Instant::now() + Duration::from_secs(1);
        while state.running.load(Ordering::SeqCst) && Instant::now() < until {
            std::thread::sleep(Duration::from_millis(1));
        }
        assert!(!state.running.load(Ordering::SeqCst));
    }
}

#[tokio::test]
async fn unavailable_preparation_selects_silence_and_never_grants_a_later_score() {
    let (_control, worker, _events, _cancel, _state) = super::super::tests::fixture();
    let active = worker.owner.active.clone();
    let service = Service::start(worker.owner.lease.clone(), active.clone(), |_| Err(())).unwrap();
    assert_eq!(service.prepare(&*active, Duration::from_secs(1)).await, Ok(()));
    assert!(service.failed());
    assert!(!service.policy.lock().unwrap().semantic_enabled());
    let (generation, ticket) = ticket();
    assert!(
        service
            .request(
                generation,
                ticket,
                Instant::now() + Duration::from_secs(1),
                || panic!("Unavailable models cannot snapshot audio")
            )
            .is_none()
    );
    assert_eq!(
        service.prepare(&*active, Duration::ZERO).await,
        Err(Failure::Timeout),
        "An already-ready result cannot restart an expired preparation budget"
    );
}
