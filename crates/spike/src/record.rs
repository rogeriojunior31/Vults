//! Around a run: our hooks installed in a scratch home as the installer writes them, the logged
//! events read back through the real parsers and core, and turned into a fixture.

use std::path::Path;
use std::time::{Duration, Instant};

use serde_json::{Value, json};
use vults_agents::registry::{self, Setup};

/// Writes our hooks (or plugin) for `agent` into the scratch `home`, as Settings would; prints the
/// file. A scratch home only: there is no backup and no diff here.
pub fn install(agent: &str, home: &Path, hook_exe: &Path) -> Result<(), String> {
    let entry = registry::of(agent).ok_or_else(|| format!("unknown agent {agent}"))?;
    let file = match entry.setup {
        Setup::Plugin(p) => {
            let files = (p.files)(
                &vults_agents::Dirs::home_only(home),
                &home.join(".config"),
                hook_exe,
            );
            for (path, text) in &files {
                write(path, text.as_bytes())?;
            }
            files.first().ok_or("empty plugin")?.0.clone()
        }
        Setup::Hooks(a) | Setup::RemoveOnly(a) => {
            let file = a.config_file(&vults_agents::Dirs::home_only(home));
            let current = std::fs::read(&file)
                .ok()
                .and_then(|b| serde_json::from_slice(&b).ok())
                .unwrap_or(json!({}));
            let next =
                serde_json::to_vec_pretty(&a.install(&current, hook_exe)).map_err(|e| e.to_string())?;
            write(&file, &next)?;
            file
        }
        Setup::Toml(t) => {
            use vults_agent_config::toml;
            let file = (t.file)(&vults_agents::Dirs::home_only(home));
            let mut doc = toml::read(&file).map_err(|e| e.to_string())?;
            toml::with_ours(&mut doc, t.key, &(t.tables)(hook_exe), vults_agents::MARKER);
            write(&file, doc.to_string().as_bytes())?;
            file
        }
        Setup::None => return Err(format!("{agent} has nothing to install")),
    };
    println!("{}", file.display());
    Ok(())
}

fn write(file: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(dir) = file.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    std::fs::write(file, bytes).map_err(|e| e.to_string())
}

fn events(log: &Path) -> Result<Vec<vults_protocol::Event>, String> {
    let text = std::fs::read_to_string(log).map_err(|e| format!("{}: {e}", log.display()))?;
    text.lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(l).map_err(|e| e.to_string()))
        .collect()
}

/// Each logged event as the app reads it, then the sessions and the card it leaves.
pub fn replay(log: &Path) -> Result<(), String> {
    let mut state = vults_core::State::default();
    let mut now = Instant::now();
    for ev in events(log)? {
        let tool = ev
            .payload
            .get("tool_name")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let update = vults_agents::parse(&ev);
        let seen = update
            .as_ref()
            .map_or("ignored".to_string(), |u| format!("{:?}", u.event));
        println!(
            "{} {tool} wants_reply={} -> {}",
            ev.event,
            ev.wants_reply,
            cut(&seen, 220)
        );
        if let Some(u) = update {
            vults_core::reduce(&mut state, vults_core::Input::Agent(u), now);
        }
        now += Duration::from_millis(500);
    }
    let view = serde_json::to_value(state.view()).map_err(|e| e.to_string())?;
    for s in view["sessions"].as_array().into_iter().flatten() {
        println!(
            "SESSION agent={} project={} status={} steps={} diffs={}",
            s["agent"],
            s["project"],
            s["status"],
            s["steps"],
            cut(&s["diffs"].to_string(), 200)
        );
    }
    println!("CARD {}", view["approval"]);
    Ok(())
}

fn cut(text: &str, max: usize) -> String {
    text.chars().take(max).collect()
}

/// The logged events as a fixture: one payload per line, or `[event, payload]` for an agent whose
/// payload does not name its event (Antigravity); `workdir` written as `as_dir`, so no scratch
/// folder lands in the repository.
pub fn fixture(log: &Path, workdir: &str, as_dir: &str) -> Result<(), String> {
    for ev in events(log)? {
        let line = if ev.payload.get("hook_event_name").is_some() {
            ev.payload.to_string()
        } else {
            json!([ev.event, ev.payload]).to_string()
        };
        println!("{}", line.replace(workdir, as_dir));
    }
    Ok(())
}
