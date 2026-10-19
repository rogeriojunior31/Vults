use super::*;

fn process(pid: u32, parent: u32, name: &str) -> Snapshot {
    Snapshot {
        process: Process {
            pid,
            started: u64::from(pid) * 10,
        },
        parent,
        name: name.into(),
        executable: (1, u64::from(pid)),
    }
}

#[test]
fn ancestry_is_from_the_os_and_shell_wrappers_are_retained_in_the_proof() {
    let exe = std::fs::metadata("/bin/sh").unwrap();
    let mut shell = process(20, 10, "sh");
    shell.executable = (exe.dev(), exe.ino());
    assert!(shell.is_shell());
    let proof = Requester::with(30, |pid| match pid {
        30 => Some(process(30, 20, "vults")),
        20 => Some(shell.clone()),
        10 => Some(process(10, 2, "agent")),
        _ => None,
    })
    .unwrap();
    assert_eq!(
        proof.relay(),
        Process {
            pid: 30,
            started: 300
        }
    );
    assert_eq!(
        proof.client(),
        Process {
            pid: 10,
            started: 100
        }
    );
    assert_eq!(proof.chain.len(), 3);
    assert!(Requester::with(30, |pid| Some(process(pid, pid, "sh"))).is_none());
    assert!(Requester::with(30, |_| None).is_none());
    assert!(
        Requester::with(30, |pid| {
            let mut next = shell.clone();
            next.process.pid = pid;
            next.parent = pid - 1;
            Some(next)
        })
        .is_none()
    );
}

#[test]
fn a_shell_name_without_its_system_executable_cannot_hide_the_requester() {
    assert!(shell_name("bash"));
    assert!(!shell_name("busybox"));
    assert!(!shell_name("toybox"));
    let spoof = process(20, 10, "sh");
    assert!(!spoof.is_shell());
    let proof = Requester::with(30, |pid| match pid {
        30 => Some(process(30, 20, "vults")),
        20 => Some(spoof.clone()),
        _ => None,
    })
    .unwrap();
    assert_eq!(proof.client().pid, 20);
    assert_eq!(proof.chain.len(), 2);
}

#[test]
fn process_stat_handles_parentheses_and_rejects_wrong_pid_dead_or_incomplete_processes() {
    let stat = |state, start| format!("42 (agent ) name) {state} 20 {} {start} 0", "0 ".repeat(17));
    assert_eq!(parse(&stat("S", 77), 42), Some(("agent ) name".into(), 20, 77)));
    for state in ["Z", "X", "x"] {
        assert!(parse(&stat(state, 77), 42).is_none());
    }
    assert!(parse(&stat("S", 77), 41).is_none());
    assert!(parse(&stat("S", 0), 42).is_none());
    assert!(parse("42 (broken) S 20", 42).is_none());
}

#[test]
fn kernel_peer_and_live_stdio_ancestry_are_revalidated() {
    let (socket, _other) = std::os::unix::net::UnixStream::pair().unwrap();
    assert_eq!(crate::peer_pid(&socket), Some(std::process::id()));
    let proof = Requester::socket(&socket).unwrap();
    assert_eq!(proof.relay().pid, std::process::id());
    assert!(proof.is_current());
    let own = Requester::stdio().unwrap();
    assert_eq!(own.relay(), proof.relay());
    let mut reused = proof.clone();
    reused.chain[0].process.started += 1;
    assert!(!reused.is_current());
    let mut executed = proof.clone();
    executed.chain[0].executable.1 += 1;
    assert!(!executed.is_current());
    let mut reparented = proof;
    reparented.chain[0].parent += 1;
    assert!(!reparented.is_current());
}

#[test]
fn a_relay_that_exits_cannot_keep_its_question_owner_alive() {
    let mut child = std::process::Command::new("sleep").arg("30").spawn().unwrap();
    let proof = Requester::capture(child.id());
    let live = proof.as_ref().is_some_and(Requester::is_current);
    let _ = child.kill();
    let _ = child.wait();
    assert!(live);
    assert!(!proof.unwrap().is_current());
}
