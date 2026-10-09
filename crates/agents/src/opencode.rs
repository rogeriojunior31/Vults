//! OpenCode runs no hook commands: it loads every `.js` or `.ts` file in its `plugins/` folder.
//! Ours is written whole by the installer. It reports every step without waiting on the relay,
//! and answers a permission only with a click on the island: OpenCode's own prompt stays up
//! meanwhile, and whichever answers first wins. No click, no app, a crash: OpenCode asks as if
//! we were not there.

use std::path::{Path, PathBuf};

use serde_json::Value;
use vults_core::AgentUpdate;
use vults_protocol::{AgentKind, Event};

use crate::{Agent, Claude, PluginAgent};

pub const NAME: &str = "opencode";

pub static OPENCODE: PluginAgent = PluginAgent {
    name: NAME,
    file,
    text,
};

/// Reads the plugin's events in Claude Code's words, with OpenCode's tool names turned into
/// Claude Code's so steps get their activity and cards their usual titles.
pub(crate) fn parse(e: &Event) -> Option<AgentUpdate> {
    let mut e = crate::other::normalized(e)?;
    if let Some(name) = e.payload.get_mut("tool_name")
        && let Some(claude) = name.as_str().and_then(claude_tool)
    {
        *name = Value::String(claude.into());
    }
    let mut update = Claude.parse(&e)?;
    update.session.agent = AgentKind::OpenCode;
    Some(update)
}

/// OpenCode's built-in tools (and permission names) that Claude Code has under another name.
fn claude_tool(name: &str) -> Option<&'static str> {
    Some(match name {
        "bash" => "Bash",
        "read" => "Read",
        "write" => "Write",
        "edit" | "patch" | "multiedit" => "Edit",
        "list" => "LS",
        "glob" => "Glob",
        "grep" => "Grep",
        "webfetch" => "WebFetch",
        "websearch" => "WebSearch",
        "todowrite" => "TodoWrite",
        "task" => "Task",
        _ => return None,
    })
}

/// `<config>/opencode/plugins/vults.js`, `<config>` being `$XDG_CONFIG_HOME` or `~/.config`, as
/// OpenCode reads it.
fn file(config_dir: &Path) -> PathBuf {
    config_dir
        .join("opencode")
        .join("plugins")
        .join(format!("{}.js", vults_brand::SLUG))
}

/// The plugin, pointing at `hook_exe`. OpenCode refuses a plugin file that exports anything but
/// the plugin function, and a hook that throws stops the tool: every hook is guarded. Steps go to
/// the relay in the background, one at a time so they arrive in order. A permission runs the
/// relay apart, after the steps before it, and is replied to only with what the hook printed:
/// `once` or `reject`, never OpenCode's `always` (an *Always* is our own rule).
fn text(hook_exe: &Path) -> String {
    // A JSON string is a valid JS string literal: no path can break out of it.
    let relay = serde_json::to_string(&hook_exe.to_string_lossy()).unwrap_or_default();
    let marker = crate::plugin_marker();
    // The hook gives up on its own before this; the plugin's limit only guards against a hang.
    let wait_ms = vults_protocol::limits::DECISION_BUDGET.as_millis() + 5_000;
    format!(
        r#"// {marker}: shows OpenCode's sessions on the island, and answers its permissions from there.
// Remove it from Settings → Agents.
import {{ spawn }} from "node:child_process";

const RELAY = {relay};

function toolInput(args) {{
  const a = args ?? {{}};
  const out = {{}};
  if (typeof a.command === "string") out.command = a.command;
  if (typeof a.filePath === "string") out.file_path = a.filePath;
  if (typeof a.filepath === "string") out.file_path = a.filepath;
  if (typeof a.path === "string") out.path = a.path;
  if (typeof a.pattern === "string") out.pattern = a.pattern;
  if (typeof a.url === "string") out.url = a.url;
  return out;
}}

function run(event, payload) {{
  return new Promise((done) => {{
    try {{
      const child = spawn(RELAY, ["--agent", "{NAME}", event], {{ stdio: ["pipe", "ignore", "ignore"], detached: true }});
      setTimeout(done, 2000).unref();
      child.on("error", () => done());
      child.on("exit", () => done());
      child.stdin?.on("error", () => {{}});
      child.stdin?.end(payload);
      child.unref();
    }} catch {{ done(); }}
  }});
}}

// The relay waiting for a click on the island: what it printed, or "" (no click, no app).
function ask(payload) {{
  let child = null;
  const result = new Promise((done) => {{
    try {{
      let out = "";
      child = spawn(RELAY, ["--agent", "{NAME}", "PermissionRequest"], {{ stdio: ["pipe", "pipe", "ignore"] }});
      const limit = setTimeout(() => {{ try {{ child.kill(); }} catch {{}} }}, {wait_ms});
      limit.unref();
      child.stdout?.setEncoding("utf8");
      child.stdout?.on("data", (d) => {{ if (out.length < 4096) out += d; }});
      child.on("error", () => done(""));
      child.on("close", () => {{ clearTimeout(limit); done(out); }});
      child.stdin?.on("error", () => {{}});
      child.stdin?.end(payload);
    }} catch {{ done(""); }}
  }});
  return {{ result, stop: () => {{ try {{ child?.kill(); }} catch {{}} }} }};
}}

export const Vults = async ({{ client, directory }}) => {{
  let queue = Promise.resolve();
  const tools = new Map(); // a running call's tool, by call id: names the permission's card
  const open = new Map(); // permissions nobody answered yet, by id: how to stop their relay
  const payloadOf = (event, sessionID, extra) =>
    JSON.stringify({{ hook_event_name: event, session_id: sessionID, cwd: directory, ...extra }});
  const send = (event, sessionID, extra = {{}}) => {{
    try {{
      if (!sessionID) return;
      const payload = payloadOf(event, sessionID, extra);
      queue = queue.then(() => run(event, payload));
    }} catch {{}}
  }};
  const permission = (p) => {{
    if (!p.id || !p.sessionID) return;
    const m = p.metadata ?? {{}};
    const input = toolInput(m);
    if (!Object.keys(input).length && Array.isArray(p.patterns)) input.pattern = p.patterns.join(" ");
    const tool = tools.get(p.tool?.callID) ?? p.permission;
    const payload = payloadOf("PermissionRequest", p.sessionID, {{ tool_name: tool, tool_input: input }});
    open.set(p.id, null);
    // After the steps before it, without holding back the ones after.
    queue.then(async () => {{
      if (!open.has(p.id)) return;
      const asking = ask(payload);
      open.set(p.id, asking.stop);
      const out = await asking.result;
      if (!open.delete(p.id)) return;
      let reply;
      try {{ reply = JSON.parse(out).reply; }} catch {{}}
      if (reply !== "once" && reply !== "reject") return;
      await client.postSessionIdPermissionsPermissionId({{ path: {{ id: p.sessionID, permissionID: p.id }}, body: {{ response: reply }} }});
    }}).catch(() => {{}});
  }};
  return {{
    event: async ({{ event }}) => {{
      try {{
        const p = event.properties ?? {{}};
        switch (event.type) {{
          case "session.created": send("SessionStart", p.info?.id ?? p.sessionID); break;
          case "session.deleted": send("SessionEnd", p.info?.id ?? p.sessionID); break;
          case "session.idle": send("Stop", p.sessionID); break;
          case "session.error": send("StopFailure", p.sessionID, {{ error: p.error?.data?.message ?? p.error?.name }}); break;
          case "permission.asked": permission(p); break;
          case "permission.replied": {{
            // Answered in OpenCode (or by us): the island's card has nothing left to wait for.
            const stop = open.get(p.requestID);
            open.delete(p.requestID);
            stop?.();
            break;
          }}
          case "message.part.updated": {{
            // A tool that throws skips tool.execute.after: its failure shows here.
            const part = p.part;
            if (part?.type === "tool" && part.state?.status === "error")
              send("PostToolUseFailure", part.sessionID, {{ tool_name: part.tool, tool_input: toolInput(part.state.input), error: part.state.error }});
            break;
          }}
        }}
      }} catch {{}}
    }},
    "chat.message": async (input) => {{ try {{ send("UserPromptSubmit", input.sessionID); }} catch {{}} }},
    "tool.execute.before": async (input, output) => {{
      try {{
        if (tools.size > 256) tools.clear();
        if (input.callID) tools.set(input.callID, input.tool);
        send("PreToolUse", input.sessionID, {{ tool_name: input.tool, tool_input: toolInput(output?.args) }});
      }} catch {{}}
    }},
    "tool.execute.after": async (input) => {{
      try {{
        tools.delete(input.callID);
        send("PostToolUse", input.sessionID, {{ tool_name: input.tool, tool_input: toolInput(input.args) }});
      }} catch {{}}
    }},
  }};
}};
"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_plugin_lives_in_opencodes_config_folder() {
        assert_eq!(
            file(Path::new("/home/me/.config")),
            Path::new("/home/me/.config/opencode/plugins/vults.js")
        );
    }

    #[test]
    fn the_plugin_carries_our_mark_and_the_relay_as_a_string() {
        let t = text(Path::new("/home/me/it's \"here\"/vults-hook"));
        assert!(t.starts_with(&format!("// {}", crate::plugin_marker())));
        assert!(t.contains(r#"const RELAY = "/home/me/it's \"here\"/vults-hook";"#));
        assert!(t.contains(r#"["--agent", "opencode", event]"#));
        // Only the plugin function is exported.
        assert_eq!(t.matches("export ").count(), 1);
    }

    #[test]
    fn the_plugin_replies_only_once_or_reject_and_only_what_the_hook_printed() {
        let t = text(Path::new("/opt/vults-hook"));
        assert!(t.contains(r#"if (reply !== "once" && reply !== "reject") return;"#));
        assert!(t.contains("body: { response: reply }"));
        assert!(!t.contains(r#""always""#));
        // Its own limit is past the hook's, which gives up first.
        let hook = vults_protocol::limits::DECISION_BUDGET.as_millis();
        assert!(t.contains(&format!("}}, {}", hook + 5_000)));
    }
}

#[cfg(test)]
mod recorded {
    use vults_core::AgentEvent;
    use vults_protocol::{AgentKind, Event};

    fn event(name: &str, payload: &str) -> Event {
        Event {
            v: vults_protocol::VERSION,
            id: "r1".into(),
            agent: AgentKind::OpenCode,
            agent_name: None,
            event: name.into(),
            wants_reply: name == "PermissionRequest",
            terminal: Default::default(),
            payload: serde_json::from_str(payload).expect("json"),
        }
    }

    /// The PreToolUse that opencode 1.18.35 sent through this plugin, recorded.
    #[test]
    fn a_recorded_step_reads_as_a_read() {
        let u = crate::parse(&event(
            "PreToolUse",
            r#"{"hook_event_name":"PreToolUse","session_id":"ses_ede3ab383ffeqD1ohz7v1Jb49t","cwd":"/home/me/notes","tool_name":"read","tool_input":{"file_path":"/home/me/notes/a.txt"}}"#,
        ))
        .expect("a step");
        assert_eq!(u.session.agent, AgentKind::OpenCode);
        assert_eq!(u.session.session_id, "ses_ede3ab383ffeqD1ohz7v1Jb49t");
        assert!(
            matches!(u.event, AgentEvent::ToolStarted(ref s) if s.activity == vults_core::Activity::Read
                && s.detail.as_deref() == Some("a.txt")),
            "{:?}",
            u.event
        );
    }

    /// The payload the plugin builds from a `permission.asked` opencode 1.18.35 sent (`echo` under
    /// `"bash": "ask"`): a card, unlike another tool's permission.
    #[test]
    fn a_recorded_permission_is_a_card() {
        let u = crate::parse(&event(
            "PermissionRequest",
            r#"{"hook_event_name":"PermissionRequest","session_id":"ses_eddc50821ffejrDNy4w1HUon9l","cwd":"/home/me/notes","tool_name":"bash","tool_input":{"command":"echo hi-from-probe > b.txt"}}"#,
        ))
        .expect("a card");
        assert!(
            matches!(u.event, AgentEvent::PermissionRequested { ref tool, ref target, .. }
                if tool == "Bash" && target == "Bash · echo hi-from-probe > b.txt"),
            "{:?}",
            u.event
        );
    }

    /// An edit's permission names the file (`metadata.filepath`, turned into `file_path`).
    #[test]
    fn an_edit_permission_names_its_file() {
        let u = crate::parse(&event(
            "PermissionRequest",
            r#"{"session_id":"s1","tool_name":"write","tool_input":{"file_path":"/w/c.txt"}}"#,
        ))
        .expect("a card");
        assert!(
            matches!(u.event, AgentEvent::PermissionRequested { ref target, .. } if target == "Write · /w/c.txt"),
            "{:?}",
            u.event
        );
    }
}
