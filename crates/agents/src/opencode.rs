//! OpenCode runs no hook commands: it loads every `.js` or `.ts` file in its `plugins/` folder.
//! Ours is written whole by the installer, one file for OpenCode 1 and 2, whose plugin APIs share
//! nothing. It reports every step without waiting on the relay, and answers a permission (and, on
//! OpenCode 1, a question) only with a click on the island: OpenCode's own prompt stays up
//! meanwhile, and whichever answers first wins. No click, no app, a crash: OpenCode asks as if we
//! were not there.

use std::path::{Path, PathBuf};

use serde_json::Value;
use vults_core::AgentUpdate;
use vults_protocol::{AgentKind, Event};

use crate::{Agent, Claude, PluginAgent};

pub const NAME: &str = "opencode";

pub static OPENCODE: PluginAgent = PluginAgent {
    name: NAME,
    files: |_, config, hook| vec![(file(config), text(hook))],
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
        // OpenCode 2 renamed bash to shell, and task to subagent.
        "bash" | "shell" => "Bash",
        "read" => "Read",
        "write" => "Write",
        "edit" | "patch" => "Edit",
        "multiedit" => "MultiEdit",
        "list" => "LS",
        "glob" => "Glob",
        "grep" => "Grep",
        "webfetch" => "WebFetch",
        "websearch" => "WebSearch",
        "todowrite" => "TodoWrite",
        "task" | "subagent" => "Task",
        // Its questions arrive as Claude Code's, so the card and the tool's step match.
        "question" => vults_protocol::QUESTION_TOOL,
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

/// The plugin, pointing at `hook_exe`. Its one export is the default object OpenCode 1 (1.3 and
/// later) and 2 both take: 1 refuses a file exporting anything but plugins, 2 a file without a
/// default `{ id, setup }`. A hook that throws stops the tool: every hook is guarded. Steps go to
/// the relay in the background, one at a time so they arrive in order. A permission or a question
/// runs the relay apart, after the steps before it, and is replied to only with what the hook
/// printed: `once` or `reject` (never OpenCode's `always`: an *Always* is our own rule), or the
/// answers. On OpenCode 1 the plugin's `client` is the v1 SDK, which has no question call: the
/// answers go through its underlying client, which knows where OpenCode's server is. OpenCode 2
/// gives a plugin no way to answer its questions (forms), so the island only shows them.
fn text(hook_exe: &Path) -> String {
    // A JSON string is a valid JS string literal: no path can break out of it.
    let relay = serde_json::to_string(&hook_exe.to_string_lossy()).unwrap_or_default();
    let marker = crate::plugin_marker();
    // The hook gives up on its own before this; the plugin's limit only guards against a hang.
    let wait_ms = vults_protocol::limits::DECISION_BUDGET.as_millis() + 5_000;
    const QUESTION: &str = vults_protocol::QUESTION_TOOL;
    const ASK: &str = vults_protocol::ASK_FLAG;
    const SLUG: &str = vults_brand::SLUG;
    format!(
        r#"// {marker}: shows OpenCode's sessions on the island, and answers its permissions from there.
// Remove it from Settings → Agents.
import {{ spawn }} from "node:child_process";

const RELAY = {relay};

// OpenCode 2 calls a file tool's file `path`: the island reads it as Claude Code's `file_path`.
const FILE_TOOLS = new Set(["read", "write", "edit"]);

function toolInput(args, tool) {{
  const a = args ?? {{}};
  const out = {{}};
  if (typeof a.command === "string") out.command = a.command;
  for (const key of ["workdir", "cwd", "directory", "dir_path"]) {{
    if (typeof a[key] === "string" && a[key].trim()) {{ out.workdir = a[key]; break; }}
  }}
  if (typeof a.filePath === "string") out.file_path = a.filePath;
  if (typeof a.filepath === "string") out.file_path = a.filepath;
  if (typeof a.path === "string") {{
    if (FILE_TOOLS.has(tool) && !out.file_path) out.file_path = a.path;
    else out.path = a.path;
  }}
  if (typeof a.pattern === "string") out.pattern = a.pattern;
  if (typeof a.url === "string") out.url = a.url;
  // An edit's or a write's text, in Claude Code's names: its card shows what Allow would change.
  if (typeof a.oldString === "string") out.old_string = a.oldString;
  if (typeof a.newString === "string") out.new_string = a.newString;
  if (typeof a.content === "string") out.content = a.content;
  if (Array.isArray(a.edits)) out.edits = a.edits.map((e) => ({{ old_string: e.oldString, new_string: e.newString }}));
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
function ask(args, payload) {{
  let child = null;
  const result = new Promise((done) => {{
    try {{
      let out = "";
      child = spawn(RELAY, ["--agent", "{NAME}", ...args], {{ stdio: ["pipe", "pipe", "ignore"] }});
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

// What both versions share: the steps in order, the cards waiting on the island, each running
// call's tool.
function relay(directory) {{
  let queue = Promise.resolve();
  const tools = new Map(); // a running call's tool and input, by call id: the permission's card
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
  // Asks the island for `id`, after the steps before it without holding back the ones after,
  // and hands what the hook printed to `answer`; nothing when OpenCode was answered first.
  const waitFor = (id, args, payload, answer) => {{
    open.set(id, null);
    queue.then(async () => {{
      if (!open.has(id)) return;
      const asking = ask(args, payload);
      open.set(id, asking.stop);
      const out = await asking.result;
      if (!open.delete(id)) return;
      let printed;
      try {{ printed = JSON.parse(out); }} catch {{ return; }}
      await answer(printed ?? {{}});
    }}).catch(() => {{}});
  }};
  const answered = (id) => {{
    // Answered in OpenCode (or by us): the island's card has nothing left to wait for.
    const stop = open.get(id);
    open.delete(id);
    stop?.();
  }};
  const stopAll = () => {{
    for (const stop of open.values()) stop?.();
    open.clear();
  }};
  return {{ tools, payloadOf, send, waitFor, answered, stopAll }};
}}

// OpenCode 1 calls `server` with its v1 SDK client and takes the hooks it returns.
async function server({{ client, directory }}) {{
  const {{ tools, payloadOf, send, waitFor, answered }} = relay(directory);
  const permission = (p) => {{
    if (!p.id || !p.sessionID) return;
    const m = p.metadata ?? {{}};
    // The call's own input: the metadata names an edit's file but not its text.
    const call = tools.get(p.tool?.callID);
    const tool = call?.tool ?? p.permission;
    const input = toolInput(call?.input ?? m, tool);
    if (!Object.keys(input).length && Array.isArray(p.patterns)) input.pattern = p.patterns.join(" ");
    const payload = payloadOf("PermissionRequest", p.sessionID, {{ tool_use_id: p.tool?.callID, tool_name: tool, tool_input: input }});
    waitFor(p.id, ["PermissionRequest"], payload, async ({{ reply }}) => {{
      if (reply !== "once" && reply !== "reject") return;
      await client.postSessionIdPermissionsPermissionId({{ path: {{ id: p.sessionID, permissionID: p.id }}, body: {{ response: reply }} }});
    }});
  }};
  const question = (q) => {{
    if (!q.id || !q.sessionID || !Array.isArray(q.questions)) return;
    // In Claude Code's words: the island shows it as Claude Code's AskUserQuestion.
    const questions = q.questions.map((x) => ({{
      question: x.question,
      header: x.header,
      multiSelect: x.multiple === true,
      options: (x.options ?? []).map((o) => ({{ label: o.label, description: o.description }})),
    }}));
    const payload = payloadOf("PreToolUse", q.sessionID, {{ tool_name: "{QUESTION}", tool_input: {{ questions }} }});
    waitFor(q.id, ["{ASK}", "PreToolUse"], payload, async ({{ answers }}) => {{
      const lists = Array.isArray(answers) && answers.length === questions.length && answers.every((a) => Array.isArray(a) && a.every((l) => typeof l === "string"));
      if (!lists) return;
      await client._client.post({{ url: "/question/{{requestID}}/reply", path: {{ requestID: q.id }}, body: {{ answers }}, headers: {{ "Content-Type": "application/json" }} }});
    }});
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
          case "question.asked": question(p); break;
          case "permission.replied":
          case "question.replied":
          case "question.rejected": answered(p.requestID); break;
          case "message.part.updated": {{
            // A tool that throws skips tool.execute.after: its failure shows here.
            const part = p.part;
            if (part?.type === "tool" && part.state?.status === "error")
              send("PostToolUseFailure", part.sessionID, {{ tool_use_id: part.callID, tool_name: part.tool, tool_input: toolInput(part.state.input, part.tool), error: part.state.error }});
            break;
          }}
        }}
      }} catch {{}}
    }},
    "chat.message": async (input) => {{ try {{ send("UserPromptSubmit", input.sessionID); }} catch {{}} }},
    "tool.execute.before": async (input, output) => {{
      try {{
        if (tools.size > 256) tools.clear();
        if (input.callID) tools.set(input.callID, {{ tool: input.tool, input: output?.args }});
        send("PreToolUse", input.sessionID, {{ tool_use_id: input.callID, tool_name: input.tool, tool_input: toolInput(output?.args, input.tool) }});
      }} catch {{}}
    }},
    "tool.execute.after": async (input) => {{
      try {{
        const args = input.args ?? tools.get(input.callID)?.input;
        tools.delete(input.callID);
        send("PostToolUse", input.sessionID, {{ tool_use_id: input.callID, tool_name: input.tool, tool_input: toolInput(args, input.tool) }});
      }} catch {{}}
    }},
  }};
}}

// OpenCode 2 calls `setup` with its context: events come from a subscription and each hook is
// registered on its domain. Its questions are forms, which a plugin has no call to answer: the
// island shows them as a question, answered in OpenCode.
async function setup(ctx) {{
  const {{ payloadOf, send, waitFor, answered, stopAll }} = relay(ctx.location?.directory);
  // A running call's tool and input, by call id: its permission names neither.
  const calls = new Map();
  // Each place OpenCode serves runs its own setup over one event bus: an event is handled once.
  const seen = (globalThis[Symbol.for(RELAY)] ??= new Set());
  const first = (id) => {{
    if (!id) return true;
    if (seen.has(id)) return false;
    if (seen.size > 1024) seen.clear();
    seen.add(id);
    return true;
  }};
  const permission = (p, where) => {{
    if (!p.id || !p.sessionID) return;
    const call = calls.get(p.source?.id);
    const tool = call?.tool ?? p.action;
    const input = toolInput(call?.input ?? p.metadata, tool);
    if (!Object.keys(input).length && Array.isArray(p.resources)) input.pattern = p.resources.join(" ");
    const payload = payloadOf("PermissionRequest", p.sessionID, {{ ...where, tool_use_id: p.source?.id, tool_name: tool, tool_input: input }});
    waitFor(p.id, ["PermissionRequest"], payload, async ({{ reply }}) => {{
      if (reply !== "once" && reply !== "reject") return;
      await ctx.permission.reply({{ sessionID: p.sessionID, requestID: p.id, decision: reply }});
    }});
  }};
  const question = (f, where) => {{
    if (f?.metadata?.kind !== "question" || !f.sessionID || !Array.isArray(f.fields)) return;
    const questions = f.fields.map((x) => ({{
      question: x.description ?? x.title,
      header: x.title,
      multiSelect: x.type === "multiselect",
      options: (x.options ?? []).map((o) => ({{ label: o.label, description: o.description }})),
    }}));
    // A question as a permission request is never a card: the app lets it go at once.
    send("PermissionRequest", f.sessionID, {{ ...where, tool_name: "{QUESTION}", tool_input: {{ questions }} }});
  }};
  const stop = new AbortController();
  (async () => {{
    for await (const e of ctx.event.subscribe({{ signal: stop.signal }})) {{
      try {{
        if (!first(e.id)) continue;
        const d = e.data ?? {{}};
        const where = e.location?.directory ? {{ cwd: e.location.directory }} : {{}};
        switch (e.type) {{
          case "session.created": send("SessionStart", d.sessionID, where); break;
          case "session.deleted": send("SessionEnd", d.sessionID, where); break;
          case "session.execution.succeeded": send("Stop", d.sessionID, where); break;
          // A new prompt taking over the turn is not its end.
          case "session.execution.interrupted": if (d.reason !== "superseded") send("Stop", d.sessionID, where); break;
          case "session.execution.failed": send("StopFailure", d.sessionID, {{ ...where, error: d.error?.message ?? d.error?.type }}); break;
          case "permission.asked": permission(d, where); break;
          case "permission.replied": answered(d.requestID); break;
          case "form.created": question(d.form, where); break;
        }}
      }} catch {{}}
    }}
  }})().catch(() => {{}});
  // A hook that throws stops what it hooks; one OpenCode lacks must not cost the others.
  const hook = async (domain, name, callback) => {{
    try {{ await domain.hook(name, (e) => {{ try {{ callback(e); }} catch {{}} }}); }} catch {{}}
  }};
  await hook(ctx.session, "prompt", (e) => send("UserPromptSubmit", e.sessionID));
  await hook(ctx.tool, "execute.before", (e) => {{
    if (calls.size > 256) calls.clear();
    if (e.id) calls.set(e.id, {{ tool: e.tool, input: e.input }});
    send("PreToolUse", e.sessionID, {{ tool_use_id: e.id, tool_name: e.tool, tool_input: toolInput(e.input, e.tool) }});
  }});
  await hook(ctx.tool, "execute.after", (e) => {{
    const input = e.input ?? calls.get(e.id)?.input;
    calls.delete(e.id);
    const step = {{ tool_use_id: e.id, tool_name: e.tool, tool_input: toolInput(input, e.tool) }};
    if (e.status === "error") send("PostToolUseFailure", e.sessionID, {{ ...step, error: e.error?.message }});
    else send("PostToolUse", e.sessionID, step);
  }});
  return () => {{ stop.abort(); stopAll(); }};
}}

// OpenCode 1 (1.3 and later) reads `server`, OpenCode 2 reads `setup`; each ignores the other.
export default {{ id: "{SLUG}", server, setup }};
"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn both_plugin_apis_keep_edit_arguments_until_the_finished_event() {
        use std::os::unix::fs::PermissionsExt;
        let root = std::env::temp_dir().join(format!("vults-plugin-inputs-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let hook = root.join("vults-hook");
        std::fs::write(
            &hook,
            "#!/bin/sh\ncat >> \"$VULTS_PLUGIN_EVENTS\"\nprintf '\\n' >> \"$VULTS_PLUGIN_EVENTS\"\nprintf '%s\\n' '{\"reply\":\"once\"}'\n",
        )
        .unwrap();
        std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o700)).unwrap();
        let module = root.join("plugin.mjs");
        std::fs::write(&module, text(&hook)).unwrap();
        let result = std::process::Command::new("node")
            .arg(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/opencode-plugin.mjs"))
            .env("VULTS_PLUGIN_MODULE", module)
            .env("VULTS_PLUGIN_EVENTS", root.join("events.jsonl"))
            .output()
            .expect("Node.js (the UI build's runtime)");
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        std::fs::remove_dir_all(root).unwrap();
    }

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
        // One export, which OpenCode 1 reads `server` from and OpenCode 2 `setup`: 1 refuses a
        // file exporting anything but plugins, 2 one without a default `{ id, setup }`.
        assert_eq!(t.matches("export ").count(), 1);
        assert!(t.contains(r#"export default { id: "vults", server, setup };"#));
        assert!(t.contains("async function server({ client, directory })"));
        assert!(t.contains("async function setup(ctx)"));
    }

    #[test]
    fn the_plugin_replies_only_once_or_reject_and_only_what_the_hook_printed() {
        let t = text(Path::new("/opt/vults-hook"));
        assert!(t.contains(r#"if (reply !== "once" && reply !== "reject") return;"#));
        // A question goes through the ask entry, and comes back only as one list per question.
        assert!(t.contains(r#"["--ask", "PreToolUse"]"#));
        assert!(t.contains("answers.length === questions.length"));
        assert!(t.contains("body: { response: reply }"));
        assert!(
            t.contains("ctx.permission.reply({ sessionID: p.sessionID, requestID: p.id, decision: reply })")
        );
        assert!(!t.contains(r#""always""#));
        // An edit's card is built from its call's own input, which carries the text it changes.
        assert!(t.contains("toolInput(call?.input ?? m, tool)"));
        assert!(t.contains("out.old_string = a.oldString"));
        assert!(t.contains("out.content = a.content"));
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
        let asks = payload.contains(r#""tool_name":"AskUserQuestion""#);
        Event {
            v: vults_protocol::VERSION,
            id: "r1".into(),
            agent: AgentKind::OpenCode,
            agent_name: None,
            event: name.into(),
            wants_reply: name == "PermissionRequest" || asks,
            zeca: false,
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

    /// The payloads the plugin built for an edit and a write in opencode 1.19.0 (the model changed
    /// one line, then wrote a two-line file): each card counts what Allow would change, and the
    /// finished edit carries its diff.
    #[test]
    fn a_recorded_edit_and_write_show_what_they_change() {
        let card = |payload: &str| match crate::parse(&event("PermissionRequest", payload)).map(|u| u.event) {
            Some(AgentEvent::PermissionRequested { ask, .. }) => (ask.added, ask.removed),
            other => panic!("not a card: {other:?}"),
        };
        let edit = r#"{"hook_event_name":"PermissionRequest","session_id":"ses_ecea293f1ffesdG9LCueMWqxZa","cwd":"/w","tool_name":"edit","tool_input":{"file_path":"/w/notes.txt","old_string":"old line","new_string":"new line"}}"#;
        let write = r#"{"hook_event_name":"PermissionRequest","session_id":"ses_ecea293f1ffesdG9LCueMWqxZa","cwd":"/w","tool_name":"write","tool_input":{"file_path":"/w/new.txt","content":"fresh\nsecond"}}"#;
        assert_eq!(card(edit), (1, 1));
        assert_eq!(card(write), (2, 0));
        let done = crate::parse(&event(
            "PostToolUse",
            &edit.replace("PermissionRequest", "PostToolUse"),
        ));
        assert!(
            matches!(done.map(|u| u.event), Some(AgentEvent::ToolFinished { diff: Some(ref d), .. }) if (d.added(), d.removed()) == (1, 1))
        );
    }

    /// The payload the plugin built from a `question.asked` opencode 1.18.35 sent (the model asked
    /// for a color): a question card, and the question tool's own step reads as the same tool.
    #[test]
    fn a_recorded_question_is_a_question_card() {
        let u = crate::parse(&event(
            "PreToolUse",
            r#"{"hook_event_name":"PreToolUse","session_id":"ses_edd9d5738ffezFD52oSlEoPnrE","cwd":"/w","tool_name":"AskUserQuestion","tool_input":{"questions":[{"question":"Which color do you prefer?","header":"Color","multiSelect":false,"options":[{"label":"Red","description":"Choose red."},{"label":"Blue","description":"Choose blue."}]}]}}"#,
        ))
        .expect("a card");
        let AgentEvent::QuestionAsked { questions, .. } = u.event else {
            panic!("{:?}", u.event);
        };
        assert_eq!(questions.len(), 1);
        assert_eq!(questions[0].question, "Which color do you prefer?");
        assert_eq!(questions[0].options.len(), 2);
        let step = crate::parse(&event(
            "PostToolUse",
            r#"{"session_id":"ses_edd9d5738ffezFD52oSlEoPnrE","tool_name":"question","tool_input":{}}"#,
        ))
        .expect("a step");
        assert!(
            matches!(step.event, AgentEvent::ToolFinished { target: Some(ref t), .. } if t == vults_protocol::QUESTION_TOOL),
            "{:?}",
            step.event
        );
    }

    /// The PreToolUse and PermissionRequest that opencode 2.0.26 sent through this plugin
    /// (`echo` under `"shell": "ask"`), recorded: its `shell` reads as Claude Code's Bash.
    #[test]
    fn a_recorded_opencode_2_permission_is_a_card() {
        let step = crate::parse(&event(
            "PreToolUse",
            r#"{"hook_event_name":"PreToolUse","session_id":"ses_ed8c64871ffe63ZSc0lKqO6KTQ","cwd":"/w","tool_name":"shell","tool_input":{"command":"echo hi > b.txt"}}"#,
        ))
        .expect("a step");
        assert!(
            matches!(step.event, AgentEvent::ToolStarted(ref s) if s.tool == "Bash"),
            "{:?}",
            step.event
        );
        let u = crate::parse(&event(
            "PermissionRequest",
            r#"{"hook_event_name":"PermissionRequest","session_id":"ses_ed8c64871ffe63ZSc0lKqO6KTQ","cwd":"/w","tool_name":"shell","tool_input":{"command":"echo hi > b.txt"}}"#,
        ))
        .expect("a card");
        assert!(
            matches!(u.event, AgentEvent::PermissionRequested { ref tool, ref target, .. }
                if tool == "Bash" && target == "Bash · echo hi > b.txt"),
            "{:?}",
            u.event
        );
    }

    /// The payload the plugin built from a question form opencode 2.0.26 sent: OpenCode 2 gives a
    /// plugin no call to answer it, so it is a question the island shows, never a card.
    #[test]
    fn a_recorded_opencode_2_question_waits_in_opencode() {
        let u = crate::parse(&event(
            "PermissionRequest",
            r#"{"hook_event_name":"PermissionRequest","session_id":"ses_ed8c6cc52ffeC5Q0FkLCr02U9n","cwd":"/w","tool_name":"AskUserQuestion","tool_input":{"questions":[{"question":"Which color do you prefer?","header":"Color","multiSelect":false,"options":[{"label":"Red","description":"Choose red."},{"label":"Blue","description":"Choose blue."}]}]}}"#,
        ))
        .expect("a question");
        assert!(
            matches!(u.event, AgentEvent::Question { ref message } if message == "Which color do you prefer?"),
            "{:?}",
            u.event
        );
    }
}
