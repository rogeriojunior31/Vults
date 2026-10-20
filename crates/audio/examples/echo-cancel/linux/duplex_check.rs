use super::*;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use vults_audio::{
    echo::duplex::{self, Interruption, Route, Session},
    playback::{Cursor, Phase},
};
fn tick(session: &Session) -> Result<()> {
    while session
        .take_input()
        .map_err(|f| format!("duplex capture failed: {f:?}"))?
        .is_some()
    {}
    thread::sleep(Duration::from_millis(2));
    Ok(())
}
fn until(session: &Session, mut done: impl FnMut() -> bool) -> Result<()> {
    let start = Instant::now();
    while !done() {
        tick(session)?;
        if start.elapsed() > Duration::from_secs(4) {
            return Err("duplex wait timed out".into());
        }
    }
    Ok(())
}
fn wait_release(released: &AtomicBool) -> Result<()> {
    let start = Instant::now();
    while !released.load(Ordering::Acquire) {
        if start.elapsed() > Duration::from_secs(3) {
            return Err("native lease was not released".into());
        }
        thread::sleep(Duration::from_millis(2));
    }
    Ok(())
}
fn pactl(args: &[&str]) -> Result<String> {
    let root = PathBuf::from(std::env::var_os("VULTS_ECHO_SPIKE_DIR").ok_or("private runtime required")?);
    if std::env::var("PULSE_SERVER")? != format!("unix:{}", root.join("pulse/native").display()) {
        return Err("private Pulse required".into());
    }
    let mut command = Command::new("pactl");
    command.args(args).stderr(Stdio::null());
    super::run(command, &root.join("route-check.log"), 5)
}
fn indexes() -> Result<Vec<u32>> {
    pactl(&["list", "short", "sink-inputs"])?
        .lines()
        .map(|line| {
            line.split_whitespace()
                .next()
                .ok_or("missing stream index".into())
                .and_then(|n| n.parse().map_err(Into::into))
        })
        .collect()
}
fn output(session: &Session) -> Result<duplex::Output> {
    let start = Instant::now();
    loop {
        match session.output() {
            Ok(output) => {
                until(session, || output.cursor().phase == Phase::Playing)?;
                return Ok(output);
            }
            Err(duplex::Failure::Busy) if start.elapsed() < Duration::from_secs(3) => tick(session)?,
            Err(f) => return Err(format!("output failed: {f:?}").into()),
        }
    }
}
fn drain(session: &Session, frames: usize) -> Result<Cursor> {
    let output = output(session)?;
    output
        .push(&vec![200; frames])
        .map_err(|f| format!("duplex push failed: {f:?}"))?;
    output.finish().map_err(|_| "duplex finish failed")?;
    until(session, || output.cursor().phase == Phase::Finished)?;
    let cursor = output.cursor();
    assert_eq!(cursor.played_frames, frames as u64);
    Ok(cursor)
}
struct Lease(Arc<AtomicBool>);
impl Drop for Lease {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Release);
    }
}
fn start(route: Route) -> Result<(Session, Arc<AtomicBool>)> {
    let released = Arc::new(AtomicBool::new(false));
    let session = Session::start(route, Arc::default(), Arc::new(Lease(released.clone())))
        .map_err(|_| "duplex start failed")?;
    until(&session, || session.ready() == Ok(true))?;
    assert!(!released.load(Ordering::Acquire));
    Ok((session, released))
}
fn revoked(session: &Session) -> Result<duplex::Failure> {
    let start = Instant::now();
    loop {
        match session.take_input() {
            Err(f) => {
                assert_eq!(session.interruption(), Interruption::Disabled);
                return Ok(f);
            }
            Ok(_) => thread::sleep(Duration::from_millis(2)),
        }
        if start.elapsed() > Duration::from_secs(3) {
            return Err("changed route was not revoked".into());
        }
    }
}
pub(super) fn run() -> Result<()> {
    let pair = vults_audio::echo::discover()
        .map_err(|_| "discovery failed")?
        .pop()
        .ok_or("missing echo pair")?;
    let inner = indexes()?;
    assert_eq!(inner.len(), 1);
    let inner = inner[0];
    let modules = pactl(&["list", "short", "modules"])?;
    let module = modules
        .lines()
        .find(|s| s.split_whitespace().nth(1) == Some("module-echo-cancel"))
        .and_then(|s| s.split_whitespace().next())
        .ok_or("missing module")?
        .to_owned();
    let (session, released) = start(Route::echo(pair.clone()))?;
    assert!(matches!(
        Session::start(Route::echo(pair.clone()), Arc::default(), Arc::new(())),
        Err(duplex::Failure::Busy)
    ));
    until(&session, || session.interruption() == Interruption::EchoCancelled)?;
    pactl(&["set-default-sink", "spike_mic"])?;
    pactl(&["set-default-source", "spike_mic.monitor"])?;
    let cursor = drain(&session, 24000)?;
    eprintln!("duplex: full drain passed");
    let interrupted = output(&session)?;
    interrupted.push(&vec![200; 24000]).map_err(|_| "push failed")?;
    let elapsed = Instant::now();
    while elapsed.elapsed() < Duration::from_millis(180) {
        tick(&session)?;
    }
    let frozen = interrupted.stop();
    assert!(frozen.played_frames > 0 && frozen.played_frames < 24000);
    let elapsed = Instant::now();
    while elapsed.elapsed() < Duration::from_millis(50) {
        tick(&session)?;
    }
    assert_eq!(interrupted.cursor().played_frames, frozen.played_frames);
    drain(&session, 2400)?;
    eprintln!("duplex: stop and restart passed");
    let active = output(&session)?;
    active.push(&vec![200; 24000]).map_err(|_| "push failed")?;
    let own = indexes()?
        .into_iter()
        .find(|i| *i != inner)
        .ok_or("missing owned output")?;
    let move_rejected = pactl(&["move-sink-input", &own.to_string(), "spike_mic"]).is_err();
    if move_rejected {
        pactl(&["move-sink-input", &inner.to_string(), "spike_mic"])?;
    }
    let moved = revoked(&session)?;
    eprintln!("duplex: move revoked");
    assert_eq!(active.cursor().phase, Phase::Failed);
    assert!(active.push(&[1; 480]).is_err());
    drop(session);
    wait_release(&released)?;
    pactl(&["move-sink-input", &inner.to_string(), "spike_speaker"])?;
    eprintln!("duplex: native lease released");
    let (headphones, released) = start(
        Route::headphones("spike_clean", "spike_reference").map_err(|_| "headphones snapshot failed")?,
    )?;
    until(&headphones, || {
        headphones.interruption() == Interruption::Headphones
    })?;
    headphones.stop();
    drop(headphones);
    wait_release(&released)?;
    let (speakers, released) =
        start(Route::speakers("spike_clean", "spike_reference").map_err(|_| "speaker snapshot failed")?)?;
    assert_eq!(speakers.interruption(), Interruption::Disabled);
    speakers.stop();
    drop(speakers);
    wait_release(&released)?;
    eprintln!("duplex: speakers passed");
    let (session, released) = start(Route::echo(pair.clone()))?;
    pactl(&["unload-module", &module])?;
    let unloaded = revoked(&session)?;
    drop(session);
    wait_release(&released)?;
    let stale =
        Session::start(Route::echo(pair), Arc::default(), Arc::new(())).map_err(|_| "stale start failed")?;
    let stale_reason = revoked(&stale)?;
    drop(stale);
    println!(
        "{{\"routed_playback_drained\":true,\"played_frames\":{},\"default_changes_preserved_targets\":true,\"interrupted_frames\":{},\"stop_froze_cursor\":true,\"new_output_after_stop_drained\":true,\"dont_move_rejected_manual_move\":{move_rejected},\"changed_route_failure\":\"{moved:?}\",\"explicit_headphone_policy\":true,\"speaker_policy_disabled\":true,\"module_unload_failure\":\"{unloaded:?}\",\"stale_pair_failure\":\"{stale_reason:?}\",\"native_lease_released_after_teardown\":true,\"physical_acceptance\":false}}",
        cursor.played_frames, frozen.played_frames
    );
    Ok(())
}
