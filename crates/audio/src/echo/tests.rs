use super::*;
use std::ffi::CString;

fn fixture() -> (ServerInfo, Vec<ModuleInfo>, Vec<SourceInfo>, Vec<SinkInfo>) {
    let server = ServerInfo {
        server_name: Some(c"PulseAudio (on PipeWire 1.6.9)".into()),
        cookie: 42,
        ..Default::default()
    };
    let module = ModuleInfo {
        index: 20,
        name: c"module-echo-cancel".into(),
        argument: Some(c"aec_method=webrtc aec_args='noise_suppression=0'".into()),
        ..Default::default()
    };
    let mut source = SourceInfo {
        index: 30,
        name: c"clean-input".into(),
        owner_module_index: Some(20),
        ..Default::default()
    };
    let mut sink = SinkInfo {
        index: 40,
        name: c"reference-output".into(),
        owner_module_index: Some(20),
        ..Default::default()
    };
    source.sample_spec.channels = 1;
    source.sample_spec.sample_rate = 48_000;
    sink.sample_spec = source.sample_spec;
    for (props, serial) in [(&mut source.props, c"31"), (&mut sink.props, c"41")] {
        props.set_bytes(c"object.serial", serial.to_bytes_with_nul());
        props.set_bytes(c"node.group", c"echo-cancel-group".to_bytes_with_nul());
    }
    (server, vec![module], vec![source], vec![sink])
}

#[test]
fn pairs_require_the_same_real_module_group_and_serials_in_both_directions() {
    let (server, modules, mut sources, mut sinks) = fixture();
    let pair = pairs(&server, &modules, &sources, &sinks).unwrap().remove(0);
    assert_eq!((pair.source(), pair.sink()), ("clean-input", "reference-output"));
    assert!(!format!("{pair:?}").contains("clean-input"));
    sinks[0].owner_module_index = Some(21);
    assert!(pairs(&server, &modules, &sources, &sinks).unwrap().is_empty());
    sinks[0].owner_module_index = Some(20);
    sinks[0]
        .props
        .set_bytes(c"node.group", c"other-group".to_bytes_with_nul());
    assert!(pairs(&server, &modules, &sources, &sinks).unwrap().is_empty());
    sinks[0]
        .props
        .set_bytes(c"node.group", c"echo-cancel-group".to_bytes_with_nul());
    sources[0]
        .props
        .set_bytes(c"object.serial", c"32".to_bytes_with_nul());
    assert!(
        !pairs(&server, &modules, &sources, &sinks)
            .unwrap()
            .contains(&pair)
    );
    sources[0].monitor_of_sink_index = Some(40);
    assert!(pairs(&server, &modules, &sources, &sinks).unwrap().is_empty());
}

#[test]
fn labels_missing_or_duplicate_nodes_and_non_pipewire_servers_never_qualify() {
    let (mut server, modules, mut sources, mut sinks) = fixture();
    sources[0].name = c"Echo Cancelled Microphone".into();
    sources[0].owner_module_index = None;
    assert!(pairs(&server, &modules, &sources, &sinks).unwrap().is_empty());
    sources[0].owner_module_index = Some(20);
    sources.push(sources[0].clone());
    assert!(pairs(&server, &modules, &sources, &sinks).unwrap().is_empty());
    sources.pop();
    sinks.clear();
    assert!(pairs(&server, &modules, &sources, &sinks).unwrap().is_empty());
    server.server_name = Some(c"pulseaudio".into());
    assert_eq!(
        pairs(&server, &modules, &sources, &sinks),
        Err(Failure::Unsupported)
    );
}

#[test]
fn a_method_inside_other_quoted_arguments_or_ambiguous_syntax_is_not_a_method() {
    let (_, mut modules, _, _) = fixture();
    for (args, expected) in [
        ("aec_method=webrtc", true),
        ("aec_method='webrtc' aec_args='noise_suppression=0'", true),
        ("aec_args='other=1 aec_method=webrtc ignored=2'", false),
        ("aec_method=webrtc aec_method=speex", false),
        ("aec_method=webrtc aec_args='unterminated", false),
        ("aec_method=webrtc\\ ignored", false),
        ("source_name=echo", false),
        (
            "aec_args=x\" aec_method=speex ignored=\" aec_method=webrtc",
            false,
        ),
        ("aec_method=web\"rtc\"", false),
        ("aec_method=\"webrtc\"ignored=1", false),
        ("\"aec_method\"=webrtc", false),
    ] {
        modules[0].argument = Some(CString::new(args).unwrap());
        assert_eq!(webrtc(&modules[0]), expected, "{args}");
    }
}

#[test]
fn restart_and_inventory_limits_invalidate_snapshots() {
    let (mut server, modules, sources, sinks) = fixture();
    let pair = pairs(&server, &modules, &sources, &sinks).unwrap().remove(0);
    server.cookie += 1;
    assert!(
        !pairs(&server, &modules, &sources, &sinks)
            .unwrap()
            .contains(&pair)
    );
    assert_eq!(
        pairs(&server, &modules, &sources, &vec![sinks[0].clone(); 1025]),
        Err(Failure::Inventory)
    );
}
