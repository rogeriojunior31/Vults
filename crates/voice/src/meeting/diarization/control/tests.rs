use super::*;
use std::sync::atomic::AtomicUsize;
use std::sync::{Arc, mpsc};
use std::time::Instant;

fn until(mut ready: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(2);
    while !ready() {
        assert!(Instant::now() < deadline, "control transition timed out");
        std::thread::park_timeout(Duration::from_millis(1));
    }
}

#[test]
fn pre_cancelled_work_neither_creates_options_nor_runs() {
    let result: Result<Option<usize>, String> = run(
        &Scheduler::new(),
        &AtomicBool::new(true),
        || panic!("options created"),
        |_: &()| panic!("terminate"),
        |_| panic!("native work"),
    );
    assert_eq!(result.expect("cancelled"), None);
}

#[test]
fn cancellation_during_option_creation_skips_native_work() {
    let scheduler = Scheduler::new();
    let stop = AtomicBool::new(false);
    let calls = AtomicUsize::new(0);
    let result = run(
        &scheduler,
        &stop,
        || {
            stop.store(true, Ordering::SeqCst);
            Ok(())
        },
        |_| Ok(()),
        |_| {
            calls.fetch_add(1, Ordering::SeqCst);
            Ok(())
        },
    );
    assert_eq!(result.expect("cancelled during preparation"), None);
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    drop(scheduler.lock());
}

#[test]
fn foreground_arriving_during_option_creation_runs_before_native_work() {
    let scheduler = Scheduler::new();
    let stop = AtomicBool::new(false);
    let served = AtomicBool::new(false);
    let preparations = AtomicUsize::new(0);
    let premature = AtomicUsize::new(0);
    let calls = AtomicUsize::new(0);
    let (start, started) = mpsc::channel();
    std::thread::scope(|scope| {
        let scheduler_ref = &scheduler;
        let served_ref = &served;
        let foreground = scope.spawn(move || {
            started.recv_timeout(Duration::from_secs(2)).expect("preparing");
            let _busy = scheduler_ref.lock();
            served_ref.store(true, Ordering::SeqCst);
        });
        let result = run(
            &scheduler,
            &stop,
            || {
                if preparations.fetch_add(1, Ordering::SeqCst) == 0 {
                    start.send(()).expect("foreground request");
                    until(|| scheduler.foreground_waiting());
                }
                Ok(())
            },
            |_| Ok(()),
            |_| {
                if !served.load(Ordering::SeqCst) {
                    premature.fetch_add(1, Ordering::SeqCst);
                }
                calls.fetch_add(1, Ordering::SeqCst);
                Ok("complete operation")
            },
        )
        .expect("retry preparation");
        foreground.join().expect("foreground worker");
        assert_eq!(result, Some("complete operation"));
    });
    assert_eq!(premature.load(Ordering::SeqCst), 0);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(preparations.load(Ordering::SeqCst), 2);
    drop(scheduler.lock());
}

#[test]
fn cancellation_interrupts_inflight_work_and_discards_its_success() {
    let scheduler = Scheduler::new();
    let stop = AtomicBool::new(false);
    let result = run(
        &scheduler,
        &stop,
        || Ok(AtomicBool::new(false)),
        |options| {
            options.store(true, Ordering::SeqCst);
            Ok(())
        },
        |options| {
            stop.store(true, Ordering::SeqCst);
            until(|| options.load(Ordering::SeqCst));
            Ok("a late successful result must not escape")
        },
    );
    assert_eq!(result.expect("cancel"), None);
    drop(scheduler.lock());
}

#[test]
fn foreground_preempts_and_exact_operation_retries_with_fresh_options() {
    let scheduler = Arc::new(Scheduler::new());
    let stop = AtomicBool::new(false);
    let attempts = AtomicUsize::new(0);
    let (start, started) = mpsc::channel();
    let (order, ordered) = mpsc::channel();
    let (release, held) = mpsc::channel();
    std::thread::scope(|scope| {
        let scheduler_ref = &scheduler;
        let attempts_ref = &attempts;
        let interactive = scope.spawn(move || {
            started
                .recv_timeout(Duration::from_secs(2))
                .expect("background entered");
            let _busy = scheduler_ref.lock();
            order.send("foreground").expect("order");
            held.recv_timeout(Duration::from_secs(2)).expect("release");
        });
        let coordinator = scope.spawn(move || {
            assert_eq!(
                ordered.recv_timeout(Duration::from_secs(2)).expect("interactive"),
                "foreground"
            );
            assert_eq!(
                attempts_ref.load(Ordering::SeqCst),
                1,
                "background retried while foreground owned the slot"
            );
            release.send(()).expect("release");
        });
        let result = run(
            &scheduler,
            &stop,
            || Ok(AtomicBool::new(false)),
            |options| {
                options.store(true, Ordering::SeqCst);
                Ok(())
            },
            |options| {
                assert!(!options.load(Ordering::SeqCst), "termination leaked into retry");
                if attempts.fetch_add(1, Ordering::SeqCst) == 0 {
                    start.send(()).expect("start");
                    until(|| options.load(Ordering::SeqCst));
                    return Err("interrupted partial result".into());
                }
                Ok("complete repeated operation")
            },
        )
        .expect("retry");
        assert_eq!(result, Some("complete repeated operation"));
        assert_eq!(attempts.load(Ordering::SeqCst), 2);
        interactive.join().expect("interactive worker");
        coordinator.join().expect("coordinator");
    });
}

#[test]
fn cancellation_while_queued_does_not_run_native_work() {
    let scheduler = Scheduler::new();
    let stop = AtomicBool::new(false);
    let _foreground = scheduler.lock();
    std::thread::scope(|scope| {
        let worker = scope.spawn(|| {
            run(
                &scheduler,
                &stop,
                || Ok(()),
                |_| Ok(()),
                |_| -> Result<(), String> { panic!("queued work ran") },
            )
        });
        stop.store(true, Ordering::SeqCst);
        assert_eq!(worker.join().expect("worker").expect("cancel"), None);
    });
}

#[test]
fn native_and_option_errors_are_preserved_and_release_the_slot() {
    let scheduler = Scheduler::new();
    let stop = AtomicBool::new(false);
    assert_eq!(
        run::<(), ()>(
            &scheduler,
            &stop,
            || Err("option failure".into()),
            |_| Ok(()),
            |_| panic!("work ran")
        ),
        Err("option failure".into())
    );
    assert_eq!(
        run::<(), ()>(
            &scheduler,
            &stop,
            || Ok(()),
            |_| Ok(()),
            |_| Err("model failure".into())
        ),
        Err("model failure".into())
    );
    drop(scheduler.lock());
}

#[test]
fn termination_failure_returns_an_error_instead_of_retrying_forever() {
    let scheduler = Scheduler::new();
    let stop = AtomicBool::new(false);
    let terminated = AtomicBool::new(false);
    let result = run(
        &scheduler,
        &stop,
        || Ok(()),
        |_| {
            terminated.store(true, Ordering::SeqCst);
            Err("terminate failure".into())
        },
        |_| {
            stop.store(true, Ordering::SeqCst);
            until(|| terminated.load(Ordering::SeqCst));
            Ok(())
        },
    );
    assert_eq!(result, Err("terminate failure".into()));
    drop(scheduler.lock());
}

#[test]
fn native_panic_joins_the_watcher_and_releases_the_slot() {
    let scheduler = Scheduler::new();
    let stop = AtomicBool::new(false);
    let result = std::panic::catch_unwind(|| {
        run::<(), ()>(
            &scheduler,
            &stop,
            || Ok(()),
            |_| Ok(()),
            |_| panic!("synthetic model panic"),
        )
    });
    assert!(result.is_err());
    drop(scheduler.lock());
}
