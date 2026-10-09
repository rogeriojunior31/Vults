//! OpenCode runs no hook commands: it loads every `.js` or `.ts` file in its `plugins/` folder.
//! Ours is the recipe from `docs/guide/other-agents.md`, written whole by the installer. It only
//! reports (sessions show as `opencode`): it never answers a permission, and never waits on the
//! relay, so a missing app costs OpenCode nothing.

use std::path::{Path, PathBuf};

use crate::PluginAgent;

pub const NAME: &str = "opencode";

pub static OPENCODE: PluginAgent = PluginAgent {
    name: NAME,
    file,
    text,
};

/// `<config>/opencode/plugins/vults.js`, `<config>` being `$XDG_CONFIG_HOME` or `~/.config`, as
/// OpenCode reads it.
fn file(config_dir: &Path) -> PathBuf {
    config_dir
        .join("opencode")
        .join("plugins")
        .join(format!("{}.js", vults_brand::SLUG))
}

/// The plugin, pointing at `hook_exe`. OpenCode refuses a plugin file that exports anything but
/// the plugin function, and a hook that throws stops the tool: every hook is guarded, and the
/// relay runs in the background, one at a time so steps arrive in order.
fn text(hook_exe: &Path) -> String {
    // A JSON string is a valid JS string literal: no path can break out of it.
    let relay = serde_json::to_string(&hook_exe.to_string_lossy()).unwrap_or_default();
    let marker = crate::plugin_marker();
    format!(
        r#"// {marker}: shows OpenCode's sessions on the island. Remove it from Settings → Agents.
import {{ spawn }} from "node:child_process";

const RELAY = {relay};

function toolInput(args) {{
  const a = args ?? {{}};
  const out = {{}};
  if (typeof a.command === "string") out.command = a.command;
  if (typeof a.filePath === "string") out.file_path = a.filePath;
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

export const Vults = async ({{ directory }}) => {{
  let queue = Promise.resolve();
  const send = (event, sessionID, extra = {{}}) => {{
    try {{
      if (!sessionID) return;
      const payload = JSON.stringify({{ hook_event_name: event, session_id: sessionID, cwd: directory, ...extra }});
      queue = queue.then(() => run(event, payload));
    }} catch {{}}
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
      try {{ send("PreToolUse", input.sessionID, {{ tool_name: input.tool, tool_input: toolInput(output?.args) }}); }} catch {{}}
    }},
    "tool.execute.after": async (input) => {{
      try {{ send("PostToolUse", input.sessionID, {{ tool_name: input.tool, tool_input: toolInput(input.args) }}); }} catch {{}}
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
        // Only the plugin function is exported, and nothing answers a permission.
        assert_eq!(t.matches("export ").count(), 1);
        assert!(!t.contains("permission"));
    }
}

#[cfg(test)]
mod recorded {
    /// The PreToolUse that opencode 1.18.35 sent through this plugin, recorded.
    #[test]
    fn a_recorded_step_reads_as_a_read() {
        let payload: serde_json::Value = serde_json::from_str(r#"{"hook_event_name":"PreToolUse","session_id":"ses_ede3ab383ffeqD1ohz7v1Jb49t","cwd":"/home/me/notes","tool_name":"read","tool_input":{"file_path":"/home/me/notes/a.txt"}}"#).expect("json");
        let event = vults_protocol::Event {
            v: vults_protocol::VERSION,
            id: "r1".into(),
            agent: vults_protocol::AgentKind::Other,
            agent_name: Some(super::NAME.into()),
            event: "PreToolUse".into(),
            wants_reply: false,
            terminal: Default::default(),
            payload,
        };
        let u = crate::parse(&event).expect("a step");
        assert!(u.session.session_id.starts_with("opencode/ses_"));
        assert!(
            matches!(u.event, vults_core::AgentEvent::ToolStarted(ref s) if s.detail.as_deref().is_some_and(|d| d.ends_with("a.txt"))),
            "{:?}",
            u.event
        );
    }
}
