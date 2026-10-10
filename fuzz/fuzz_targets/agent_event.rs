//! Any sequence of hook JSON from one agent: the first byte picks the agent (its name, for the tools
//! that are not built in) and whether the hook waits; each following line is one event's payload
//! (its `hook_event_name`, or the line's first byte's pick, names the event). Normalizing never
//! panics, and neither does the core taking them all in, with time going on, nor its view.
#![no_main]

use std::time::{Duration, Instant};

use libfuzzer_sys::fuzz_target;
use serde_json::Value;
use vults_protocol::{AgentKind, Event, Terminal, VERSION};

const AGENTS: [(AgentKind, Option<&str>); 8] = [
    (AgentKind::Claude, None),
    (AgentKind::Codex, None),
    (AgentKind::Gemini, None),
    (AgentKind::OpenCode, None),
    (AgentKind::Qwen, None),
    (AgentKind::Other, Some("antigravity")),
    (AgentKind::Other, Some("cursor")),
    (AgentKind::Other, Some("tool")),
];

const EVENTS: [&str; 16] = [
    "SessionStart",
    "UserPromptSubmit",
    "PreToolUse",
    "PermissionRequest",
    "PostToolUse",
    "PostToolUseFailure",
    "Notification",
    "Stop",
    "StopFailure",
    "SubagentStart",
    "SubagentStop",
    "SessionEnd",
    "BeforeTool",
    "AfterTool",
    "BeforeAgent",
    "PreInvocation",
];

fuzz_target!(|data: &[u8]| {
    let [pick, rest @ ..] = data else { return };
    let (agent, agent_name) = AGENTS[usize::from(pick % 8)];
    let mut state = vults_core::State::default();
    let mut now = Instant::now();
    for line in rest.split(|b| *b == b'\n') {
        let [name, payload @ ..] = line else { continue };
        // Antigravity's own lines are `[event, payload]`.
        let (named, payload) = match serde_json::from_slice::<Value>(payload) {
            Ok(Value::Array(pair)) if pair.len() == 2 => (pair[0].as_str().map(str::to_owned), pair[1].clone()),
            Ok(v) => (None, v),
            Err(_) => continue,
        };
        let event = payload
            .get("hook_event_name")
            .and_then(Value::as_str)
            .map(str::to_owned)
            .or(named)
            .unwrap_or_else(|| EVENTS[usize::from(name % 16)].to_owned());
        let event = Event {
            v: VERSION,
            id: "fuzz".into(),
            agent,
            agent_name: agent_name.map(str::to_owned),
            event,
            wants_reply: pick & 0x80 != 0,
            terminal: Terminal::default(),
            payload,
        };
        if let Some(update) = vults_agents::parse(&event) {
            vults_core::reduce(&mut state, vults_core::Input::Agent(update), now);
        }
        now += Duration::from_secs(u64::from(*name));
        vults_core::reduce(&mut state, vults_core::Input::Tick, now);
    }
    let _ = state.view();
});
