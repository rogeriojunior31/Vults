//! The real road an agent's hook JSON takes: the hook builds the event from its stdin, the line
//! goes on the socket, the app decodes it, the agent's parser normalizes it, the core takes it in.
//! The first byte picks the agent (and the `--ask` flag), the second the event named in argv; the
//! rest is stdin. Nothing panics, and every event the hook builds fits in a line the app reads.
#![no_main]

use std::time::Instant;

use libfuzzer_sys::fuzz_target;
use vults_hook::{Args, build_event};
use vults_protocol::{MAX_MESSAGE, decode_event, encode};

const AGENTS: [&str; 8] = ["claude", "codex", "gemini", "opencode", "qwen", "antigravity", "cursor", "-bad name"];

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
    "PreInvocation",
    "StatusLine",
];

fuzz_target!(|data: &[u8]| {
    let [pick, name, stdin @ ..] = data else { return };
    let mut argv = vec!["--agent".to_owned(), AGENTS[usize::from(pick % 8)].to_owned()];
    if pick & 0x80 != 0 {
        argv.push(vults_protocol::ASK_FLAG.to_owned());
    }
    argv.push(EVENTS[usize::from(name % 16)].to_owned());
    let args = Args::parse(argv.into_iter());
    let Some(event) = build_event(&args, stdin, None, |_| None, Some(1), "fuzz".into()) else {
        return;
    };
    let line = encode(&event);
    assert!(line.len() <= MAX_MESSAGE, "the app would drop this line unread: {} bytes", line.len());
    let decoded = decode_event(&line[..line.len() - 1]).expect("the hook's own line decodes");
    if let Some(update) = vults_agents::parse(&decoded) {
        let mut state = vults_core::State::default();
        let now = Instant::now();
        vults_core::reduce(&mut state, vults_core::Input::Agent(update), now);
        let _ = state.view();
    }
});
