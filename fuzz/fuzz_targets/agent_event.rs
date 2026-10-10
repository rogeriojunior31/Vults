//! Any hook JSON from any agent: the first byte picks the agent and whether the hook waits for a
//! reply, the rest is the payload (its `hook_event_name`, or the second byte's pick, names the
//! event). Normalizing it never panics, and neither does the core taking it in.
#![no_main]

use std::time::Instant;

use libfuzzer_sys::fuzz_target;
use serde_json::Value;
use vults_protocol::{AgentKind, Event, Terminal, VERSION};

const AGENTS: [AgentKind; 6] = [
    AgentKind::Claude,
    AgentKind::Codex,
    AgentKind::Gemini,
    AgentKind::OpenCode,
    AgentKind::Qwen,
    AgentKind::Other,
];

const EVENTS: [&str; 12] = [
    "SessionStart",
    "UserPromptSubmit",
    "PreToolUse",
    "PermissionRequest",
    "PostToolUse",
    "Notification",
    "Stop",
    "SubagentStop",
    "SessionEnd",
    "BeforeTool",
    "AfterTool",
    "permission.ask",
];

fuzz_target!(|data: &[u8]| {
    let [pick, name, payload @ ..] = data else { return };
    let Ok(payload) = serde_json::from_slice::<Value>(payload) else { return };
    let agent = AGENTS[usize::from(pick % 6)];
    let event = payload
        .get("hook_event_name")
        .and_then(Value::as_str)
        .map_or_else(|| EVENTS[usize::from(name % 12)].to_owned(), str::to_owned);
    let event = Event {
        v: VERSION,
        id: "fuzz".into(),
        agent,
        agent_name: (agent == AgentKind::Other).then(|| "tool".into()),
        event,
        wants_reply: pick & 0x80 != 0,
        terminal: Terminal::default(),
        payload,
    };
    if let Some(update) = vults_agents::parse(&event) {
        let mut state = vults_core::State::default();
        let now = Instant::now();
        vults_core::reduce(&mut state, vults_core::Input::Agent(update), now);
        vults_core::reduce(&mut state, vults_core::Input::Tick, now);
    }
});
