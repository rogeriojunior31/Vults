use super::*;
#[test]
fn silence_gaps_never_become_content_and_drain_needs_an_acknowledgement() {
    let mut s = OutputState::new();
    s.phase = Phase::Draining;
    s.eof = true;
    s.total = 20;
    s.submitted = 20;
    s.spans.extend([
        Span {
            wire: 100,
            before: 0,
            frames: 10,
        },
        Span {
            wire: 200,
            before: 10,
            frames: 10,
        },
    ]);
    s.advance(50);
    assert_eq!(s.played, 0);
    assert!(s.first.is_none());
    s.advance(150);
    assert_eq!(s.played, 10);
    s.advance(199);
    assert_eq!(s.played, 10);
    s.advance(205);
    assert_eq!(s.played, 15);
    s.advance(210);
    assert_eq!(s.played, 20);
    assert_eq!(s.phase, Phase::Draining);
    s.drained = true;
    s.advance(210);
    assert_eq!(s.phase, Phase::Finished);
    let mut s = OutputState::new();
    s.phase = Phase::Playing;
    s.total = 20;
    s.submitted = 20;
    s.spans.push_back(Span {
        wire: 100,
        before: 0,
        frames: 20,
    });
    s.advance(105);
    s.halt(None);
    s.advance(120);
    assert_eq!(s.played, 5);
    assert_eq!(s.phase, Phase::Stopped);
}
#[test]
fn revocation_discards_queued_capture_output_and_all_further_writes() {
    let shared = Arc::new(Shared {
        state: Mutex::new(State {
            failure: None,
            ready: true,
            verified: true,
            output_live: true,
            checked: Instant::now(),
            capture: VecDeque::from([vec![7; 480]]),
            output: None,
        }),
        cancel: Arc::default(),
        active: Arc::new(|| true),
        started: Instant::now(),
    });
    let session = Session {
        shared: shared.clone(),
        mode: Interruption::Headphones,
    };
    let output = session.output().unwrap();
    assert!(matches!(session.output(), Err(Failure::Busy)));
    output.state.lock().unwrap().phase = Phase::Playing;
    assert_eq!(output.push(&[]), Err(Failure::Input));
    assert_eq!(output.push(&vec![0; 24001]), Err(Failure::Input));
    output.push(&[4; 480]).unwrap();
    shared.fail(Failure::Route);
    assert_eq!(session.interruption(), Interruption::Disabled);
    assert_eq!(session.take_input(), Err(Failure::Route));
    assert_eq!(output.push(&[4; 480]), Err(Failure::Route));
    assert_eq!(output.cursor().phase, Phase::Failed);
    assert!(output.state.lock().unwrap().pcm.is_empty());
    assert!(shared.state.lock().unwrap().capture.is_empty());
}
#[test]
fn graph_identity_covers_internal_routes_ports_serials_and_server_cookie() {
    use graph::{Edge, Graph, Node, Pinned};
    let node = |index, name: &str| Node {
        index,
        serial: index as u64 + 1,
        name: name.into(),
        group: Some("echo".into()),
        module: Some(20),
        port: None,
        rate: 48000,
        channels: 1,
        monitor: false,
    };
    let source = node(30, "clean");
    let sink = node(40, "reference");
    let pair = Pair {
        server: 42,
        module: 20,
        source: Endpoint {
            index: 30,
            serial: 31,
            name: "clean".into(),
        },
        sink: Endpoint {
            index: 40,
            serial: 41,
            name: "reference".into(),
        },
        group: "echo".into(),
    };
    let route = Route::echo(pair);
    let mut g = Graph::default();
    g.cookie = Some(42);
    g.pipewire = true;
    g.modules = vec![20];
    g.sources = vec![source, node(50, "microphone")];
    g.sinks = vec![sink, node(60, "speaker")];
    g.capture = vec![Edge {
        index: 70,
        serial: 71,
        module: Some(20),
        device: 50,
    }];
    g.playback = vec![Edge {
        index: 80,
        serial: 81,
        module: Some(20),
        device: 60,
    }];
    let pinned = Pinned::from_graph(&route, &g).unwrap();
    g.sinks[1].port = Some("different-port".into());
    assert_ne!(Pinned::from_graph(&route, &g).unwrap(), pinned);
    g.sinks[1].port = None;
    g.playback[0].device = 40;
    assert_ne!(Pinned::from_graph(&route, &g).unwrap(), pinned);
    g.playback[0].device = 60;
    g.sources[0].serial += 1;
    assert_eq!(Pinned::from_graph(&route, &g), Err(Failure::Route));
    g.sources[0].serial -= 1;
    g.cookie = Some(43);
    assert_eq!(Pinned::from_graph(&route, &g), Err(Failure::Route));
    g.cookie = Some(42);
    g.modules.clear();
    assert_eq!(Pinned::from_graph(&route, &g), Err(Failure::Route));
}

#[test]
fn dropping_the_intentional_output_stops_even_while_a_worker_holds_its_state() {
    let shared = Arc::new(Shared {
        state: Mutex::new(State {
            failure: None,
            ready: true,
            verified: true,
            output_live: true,
            checked: Instant::now(),
            capture: VecDeque::new(),
            output: None,
        }),
        cancel: Arc::default(),
        active: Arc::new(|| true),
        started: Instant::now(),
    });
    let session = Session {
        shared,
        mode: Interruption::Headphones,
    };
    let output = session.output().unwrap();
    let worker = output.state.clone();
    worker.lock().unwrap().phase = Phase::Playing;
    output.push(&[3; 480]).unwrap();
    drop(output);
    assert_eq!(worker.lock().unwrap().phase, Phase::Stopped);
    assert!(worker.lock().unwrap().pcm.is_empty());
}

#[test]
fn revoked_admission_never_claims_or_opens_native_audio() {
    let route = Route {
        server: 1,
        source: Endpoint {
            index: 2,
            serial: 3,
            name: "unused".into(),
        },
        sink: Endpoint {
            index: 4,
            serial: 5,
            name: "unused-output".into(),
        },
        echo: None,
        mode: Interruption::Headphones,
    };
    assert!(matches!(
        Session::start_guarded(route, Arc::default(), Arc::new(()), Arc::new(|| false)),
        Err(Failure::Cancelled)
    ));
}
