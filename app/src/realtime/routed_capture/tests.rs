use super::*;
use crate::realtime::{Input, State};
use vults_voice::turn::{Action, BargeIn};
fn fixture(
    decode: impl Fn(&[i16], &AtomicBool) -> Result<String, Failure> + Send + Sync + 'static,
) -> (Assembler, Arc<State>, Action) {
    let state = Arc::new(State::default());
    let (lease, epoch) = state.begin().unwrap();
    let check = state.clone();
    let mut assembler = Assembler::new(Arc::new(lease), Arc::new(move || check.current(epoch)), decode);
    let mut machine = vults_voice::turn::Turn::new(BargeIn::Headphones);
    let listen = machine.start(Duration::ZERO).unwrap();
    assembler.event(&Event::Action(listen));
    (assembler, state, listen)
}
fn completed(assembler: &mut Assembler) -> Result<Command, Failure> {
    let until = Instant::now() + Duration::from_secs(2);
    loop {
        if let Some(command) = assembler.poll()? {
            return Ok(command);
        }
        assert!(Instant::now() < until);
        std::thread::sleep(Duration::from_millis(1));
    }
}
#[test]
fn only_explicit_finish_commits_exact_replay_and_following_pcm_once() {
    let (mut a, _state, listen) = fixture(|pcm, _| {
        assert_eq!(pcm, [vec![11; RATE / 2], vec![22; RATE / 2]].concat());
        Ok("Open the notes".into())
    });
    let g = listen.generation;
    a.input(&vec![99; RATE]).unwrap();
    a.event(&Event::Replay {
        generation: g,
        input: Input {
            samples: vec![11; RATE / 2],
        },
    });
    assert!(a.poll().unwrap().is_none());
    a.command(g, Operation::Finish).unwrap();
    assert!(a.poll().unwrap().is_none());
    a.command(g, Operation::Begin).unwrap();
    a.command(g, Operation::Begin).unwrap();
    a.input(&vec![22; RATE / 2]).unwrap();
    assert!(a.poll().unwrap().is_none());
    a.command(g, Operation::Finish).unwrap();
    a.command(g, Operation::Finish).unwrap();
    let Command::Turn { generation, turn } = completed(&mut a).unwrap() else {
        panic!("complete turn required")
    };
    assert_eq!(generation, g);
    assert_eq!(turn.transcript(), Some("Open the notes"));
    assert_eq!(turn.samples().len(), RATE);
    assert!(!turn.handled());
    a.command(g, Operation::Begin).unwrap();
    assert!(a.samples.is_none());
    assert!(a.poll().unwrap().is_none());
}
#[test]
fn stale_generations_cancel_and_caps_never_decode_or_commit() {
    let (mut a, _state, listen) = fixture(|_, _| panic!("uncommitted audio must not decode"));
    let g = listen.generation;
    a.command(g, Operation::Begin).unwrap();
    a.input(&vec![1; RATE]).unwrap();
    a.command(g, Operation::Cancel).unwrap();
    a.command(g, Operation::Finish).unwrap();
    assert!(a.samples.is_none());
    a.command(g, Operation::Begin).unwrap();
    assert!(a.samples.is_none());
    let (mut a, _state, listen) = fixture(|_, _| panic!("oversized capture must not decode"));
    let g = listen.generation;
    a.command(g, Operation::Begin).unwrap();
    for _ in 0..30 {
        a.input(&vec![1; RATE]).unwrap();
    }
    assert_eq!(a.samples.as_ref().unwrap().len(), MAX);
    assert!(matches!(a.input(&[1]), Err(Failure::Input)));
    assert!(a.samples.is_none());
    a.command(g, Operation::Finish).unwrap();
    let mut machine = vults_voice::turn::Turn::new(BargeIn::Headphones);
    let old = machine.start(Duration::ZERO).unwrap();
    machine.handle(
        old.generation,
        vults_voice::turn::Event::LocalCompleted,
        Duration::ZERO,
    );
    let next = machine.start(Duration::ZERO).unwrap();
    a.event(&Event::Action(next));
    a.command(g, Operation::Begin).unwrap();
    assert!(a.samples.is_none());
    a.command(next.generation, Operation::Begin).unwrap();
    a.input(&vec![1; RATE / 10 - 1]).unwrap();
    assert!(matches!(
        a.command(next.generation, Operation::Finish),
        Err(Failure::Input)
    ));
    a.event(&Event::Action(Action {
        generation: next.generation,
        effect: Effect::Generate,
    }));
    a.command(g, Operation::Begin).unwrap();
    assert!(a.samples.is_none());
    assert!(a.poll().unwrap().is_none());
}
#[test]
fn empty_invalid_and_failed_transcripts_do_not_produce_turns() {
    for text in [Ok("".into()), Ok("bad\u{0001}text".into()), Err(Failure::Input)] {
        let text = std::sync::Mutex::new(Some(text));
        let (mut a, _state, listen) = fixture(move |_, _| text.lock().unwrap().take().unwrap());
        a.command(listen.generation, Operation::Begin).unwrap();
        a.input(&vec![1; RATE]).unwrap();
        a.command(listen.generation, Operation::Finish).unwrap();
        assert!(matches!(completed(&mut a), Err(Failure::Input)));
    }
}
#[test]
fn cancellation_replacement_timeout_and_revocation_suppress_delayed_decode() {
    for mode in 0..4 {
        let (entered, entered_rx) = mpsc::sync_channel(1);
        let (release, release_rx) = mpsc::sync_channel(1);
        let release_rx = std::sync::Mutex::new(release_rx);
        let (mut a, state, listen) = fixture(move |_, _| {
            entered.send(()).unwrap();
            release_rx.lock().unwrap().recv().unwrap();
            Ok("late command".into())
        });
        let g = listen.generation;
        a.command(g, Operation::Begin).unwrap();
        a.input(&vec![1; RATE]).unwrap();
        a.command(g, Operation::Finish).unwrap();
        entered_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        match mode {
            0 => {
                a.command(g, Operation::Cancel).unwrap();
            }
            1 => a.event(&Event::Action(listen)),
            2 => {
                a.job.as_mut().unwrap().until = Instant::now();
                assert!(matches!(a.poll(), Err(Failure::Input)));
            }
            _ => {
                state.epoch.fetch_add(1, Ordering::SeqCst);
                assert!(matches!(a.poll(), Err(Failure::Cancelled)));
            }
        }
        a.command(g, Operation::Begin).ok();
        assert!(a.samples.is_none());
        let start = Instant::now();
        drop(a);
        assert!(start.elapsed() < Duration::from_millis(100));
        assert!(state.running.load(Ordering::SeqCst));
        release.send(()).unwrap();
        let until = Instant::now() + Duration::from_secs(2);
        while state.running.load(Ordering::SeqCst) && Instant::now() < until {
            std::thread::sleep(Duration::from_millis(1));
        }
        assert!(!state.running.load(Ordering::SeqCst));
    }
}

#[test]
fn cancelled_decode_cannot_commit_or_replace_a_new_generation() {
    let (entered, entered_rx) = mpsc::sync_channel(1);
    let (release, release_rx) = mpsc::sync_channel(1);
    let release_rx = std::sync::Mutex::new(release_rx);
    let (mut a, _state, listen) = fixture(move |_, _| {
        entered.send(()).unwrap();
        release_rx.lock().unwrap().recv().unwrap();
        Ok("discard these final words".into())
    });
    a.command(listen.generation, Operation::Begin).unwrap();
    a.input(&vec![1; RATE]).unwrap();
    a.command(listen.generation, Operation::Finish).unwrap();
    entered_rx.recv_timeout(Duration::from_secs(2)).unwrap();
    let mut machine = vults_voice::turn::Turn::new(BargeIn::Headphones);
    let first = machine.start(Duration::ZERO).unwrap();
    machine.handle(
        first.generation,
        vults_voice::turn::Event::LocalCompleted,
        Duration::ZERO,
    );
    let next = machine.start(Duration::ZERO).unwrap();
    a.event(&Event::Action(next));
    a.command(listen.generation, Operation::Cancel).unwrap();
    a.external(listen.generation);
    assert_eq!(a.generation, Some(next.generation));
    a.command(next.generation, Operation::Begin).unwrap();
    assert!(a.samples.is_none(), "only one final decoder may be admitted");
    release.send(()).unwrap();
    let until = Instant::now() + Duration::from_secs(2);
    while a.job.is_some() {
        assert!(a.poll().unwrap().is_none());
        assert!(Instant::now() < until);
        std::thread::sleep(Duration::from_millis(1));
    }
    a.command(next.generation, Operation::Begin).unwrap();
    a.input(&vec![2; RATE]).unwrap();
    a.external(listen.generation);
    assert_eq!(a.samples.as_ref().unwrap(), &vec![2; RATE]);
    a.command(next.generation, Operation::Cancel).unwrap();
    assert!(a.poll().unwrap().is_none());
}
