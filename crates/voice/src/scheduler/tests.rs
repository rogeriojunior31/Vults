use super::*;
use std::sync::{Arc, mpsc};
use std::time::Instant;

fn waiting(scheduler: &Scheduler) {
    let deadline = Instant::now() + Duration::from_secs(2);
    while !scheduler.foreground_waiting() {
        assert!(Instant::now() < deadline, "foreground did not queue");
        std::thread::sleep(Duration::from_millis(1));
    }
}

#[test]
fn foreground_runs_before_a_waiting_background_chunk() {
    let scheduler = Arc::new(Scheduler::new());
    let stop = Arc::new(AtomicBool::new(false));
    let initial = scheduler.background(&stop).expect("initial chunk");
    let (tx, order) = mpsc::channel();
    let (release, hold) = mpsc::channel();
    let interactive = {
        let scheduler = scheduler.clone();
        let tx = tx.clone();
        std::thread::spawn(move || {
            let _guard = scheduler.lock();
            tx.send("foreground").expect("order");
            hold.recv().expect("release");
        })
    };
    waiting(&scheduler);
    let meeting = {
        let scheduler = scheduler.clone();
        let stop = stop.clone();
        std::thread::spawn(move || {
            let _guard = scheduler.background(&stop).expect("next chunk");
            tx.send("background").expect("order");
        })
    };
    drop(initial);
    assert_eq!(
        order.recv_timeout(Duration::from_secs(2)).expect("interactive"),
        "foreground"
    );
    assert!(order.try_recv().is_err(), "GPU work must stay serialized");
    release.send(()).expect("release");
    assert_eq!(
        order.recv_timeout(Duration::from_secs(2)).expect("meeting"),
        "background"
    );
    interactive.join().expect("foreground worker");
    meeting.join().expect("background worker");
    assert!(!scheduler.foreground_waiting());
}

#[test]
fn a_queued_background_job_can_cancel_without_releasing_the_foreground() {
    let scheduler = Scheduler::new();
    let _busy = scheduler.lock();
    let stop = AtomicBool::new(false);
    std::thread::scope(|scope| {
        let worker = scope.spawn(|| scheduler.background(&stop).is_none());
        stop.store(true, Ordering::SeqCst);
        assert!(worker.join().expect("cancelled worker"));
    });
    assert!(scheduler.background(&stop).is_none());
}

#[test]
fn unwind_releases_the_slot_without_poisoning_the_queue() {
    let scheduler = Scheduler::new();
    assert!(
        std::panic::catch_unwind(|| {
            let _guard = scheduler.lock();
            panic!("synthetic worker failure");
        })
        .is_err()
    );
    let stop = AtomicBool::new(false);
    assert!(scheduler.background(&stop).is_some());
    drop(scheduler.lock());
}
