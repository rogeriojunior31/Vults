# Other agents

Claude Code, Codex, Gemini CLI and Antigravity are built in: **Settings → Agents** installs their
hooks. Any other tool can put its sessions on the wire too, if it runs a command on its events and
sends Claude Code's hook JSON on stdin (the format most agent tools copy). For tools that run
plugins instead of commands (OpenCode, Pi), a short plugin does the same; recipes are below.

Point the tool's hooks at the relay with a name of your choice:

```sh
~/.local/share/vultures-ai/bin/vultures-ai-hook --agent my-tool SessionStart
```

- The name is 1 to 24 characters of `a-z`, `0-9` and `-`. `claude`, `codex`, `gemini` and `other` are taken,
  so nothing can pass for a built-in agent. With a name that breaks these rules the hook sends
  nothing and exits at once: the tool never waits on it.
- The event name comes from `hook_event_name` in the JSON, or from the last argument.
- Each session needs an id: `session_id`, or `conversation_id`, `sessionId` or `conversationId`.
  An event without one is dropped. Sessions show under the tool's name, in violet.
- The folder is `cwd`, else the first of `workspace_roots` (or Antigravity's `workspacePaths`),
  else the folder the hook runs in.

The island follows these events: `SessionStart`, `SessionEnd`, `UserPromptSubmit`, `PreToolUse`,
`PostToolUse`, `PostToolUseFailure`, `PermissionRequest`, `Notification`, `Stop`, `StopFailure`,
`SubagentStart` and `SubagentStop`. Cursor's names for them (`sessionStart`, `beforeSubmitPrompt`,
`preToolUse`, `postToolUseFailure`, `stop`…) are read too. Fields it reads: `session_id`, `cwd`,
`tool_name`, `tool_input` (`command`, `file_path`, `path`, `url`, `query`, `pattern`), `message`,
`last_assistant_message`, `error` (or `error_message`).

The relay always exits 0 and prints nothing for these tools. Keep it that way in a wrapper: a
tool that treats a failed hook as a "no" would stop working when the app is not installed.

## Permissions

The island never answers another tool's permission: it has no way to know what that tool expects
back. A `PermissionRequest` shows as a question waiting in the terminal, the hook does not wait, and
the tool asks you there as usual.

## Antigravity

**Settings → Agents → Antigravity** installs the hooks in `~/.gemini/config/hooks.json`, the file
the agy CLI, the Antigravity app and the IDE share. The file's top-level keys name hooks; ours is
`vultures-ai`, and the others are never touched. Its sessions show as `antigravity`.

What Antigravity tells hooks, and so what the island shows:

| Antigravity event | On the island |
|---|---|
| `PreInvocation` (before each model call) | Thinking |
| `PreToolUse` | The step: `view_file`, `run_command`, `grep_search`… |
| `PostToolUse` | The step finished, or failed when it has an `error` |
| `Stop` | Done, or failed when it stopped on an error |

- There is no session start or end: a session shows with its first event and leaves the wire
  when it has been quiet for a while (10 minutes once done, 30 otherwise).
- Antigravity does not send the agent's reply, so a finished session has no summary line.
- A `PreToolUse` hook could allow or deny a tool, but it runs before every tool, reads included,
  and does not say whether Antigravity would have asked. So Antigravity asks its permissions
  itself, and the island only shows the step.
- Antigravity stops a tool when a hook fails. Every command of ours ends in `|| exit 0`, so a
  missing or broken relay never stops it.

## Recipes

These were checked against each tool's documentation and installed copy (versions below), with
the relay missing too; none was run inside a live session of the tool. Each one only reports: it
never answers a permission.

### OpenCode

OpenCode (1.18) loads every `.js` or `.ts` file in `~/.config/opencode/plugins/`. Save this as
`~/.config/opencode/plugins/vultures-ai.js`. Export only the function: OpenCode refuses a plugin file
that exports anything else. A hook that throws would stop the tool, so every one is guarded, and
the relay runs in the background, one at a time so steps arrive in order.

```js
import { spawn } from "node:child_process";
import { homedir } from "node:os";
import { join } from "node:path";

const RELAY = join(homedir(), ".local/share/vultures-ai/bin/vultures-ai-hook");

function toolInput(args) {
  const a = args ?? {};
  const out = {};
  if (typeof a.command === "string") out.command = a.command;
  if (typeof a.filePath === "string") out.file_path = a.filePath;
  if (typeof a.path === "string") out.path = a.path;
  if (typeof a.pattern === "string") out.pattern = a.pattern;
  if (typeof a.url === "string") out.url = a.url;
  return out;
}

function run(event, payload) {
  return new Promise((done) => {
    try {
      const child = spawn(RELAY, ["--agent", "opencode", event], { stdio: ["pipe", "ignore", "ignore"], detached: true });
      setTimeout(done, 2000).unref();
      child.on("error", () => done());
      child.on("exit", () => done());
      child.stdin?.on("error", () => {});
      child.stdin?.end(payload);
      child.unref();
    } catch { done(); }
  });
}

export const VulturesAI = async ({ directory }) => {
  let queue = Promise.resolve();
  const send = (event, sessionID, extra = {}) => {
    try {
      if (!sessionID) return;
      const payload = JSON.stringify({ hook_event_name: event, session_id: sessionID, cwd: directory, ...extra });
      queue = queue.then(() => run(event, payload));
    } catch {}
  };
  return {
    event: async ({ event }) => {
      try {
        const p = event.properties ?? {};
        switch (event.type) {
          case "session.created": send("SessionStart", p.info?.id ?? p.sessionID); break;
          case "session.deleted": send("SessionEnd", p.info?.id ?? p.sessionID); break;
          case "session.idle": send("Stop", p.sessionID); break;
          case "session.error": send("StopFailure", p.sessionID, { error: p.error?.data?.message ?? p.error?.name }); break;
          case "message.part.updated": {
            // A tool that throws skips tool.execute.after: its failure shows here.
            const part = p.part;
            if (part?.type === "tool" && part.state?.status === "error")
              send("PostToolUseFailure", part.sessionID, { tool_name: part.tool, tool_input: toolInput(part.state.input), error: part.state.error });
            break;
          }
        }
      } catch {}
    },
    "chat.message": async (input) => { try { send("UserPromptSubmit", input.sessionID); } catch {} },
    "tool.execute.before": async (input, output) => {
      try { send("PreToolUse", input.sessionID, { tool_name: input.tool, tool_input: toolInput(output?.args) }); } catch {}
    },
    "tool.execute.after": async (input) => {
      try { send("PostToolUse", input.sessionID, { tool_name: input.tool, tool_input: toolInput(input.args) }); } catch {}
    },
  };
};
```

OpenCode has no event when it quits, so its birds leave the wire once quiet. Subagent sessions
are sessions too: each shows as its own bird.

### Pi

Pi (1.0) loads extensions from `~/.pi/agent/extensions/`. Save this as
`~/.pi/agent/extensions/vultures-ai.ts`. It listens to `tool_execution_start` and `_end`, never to
`tool_call`: a `tool_call` handler that fails stops the tool.

```ts
import { spawn } from "node:child_process";
import { homedir } from "node:os";
import { join } from "node:path";

const RELAY = join(homedir(), ".local/share/vultures-ai/bin/vultures-ai-hook");

function toolInput(args: any): Record<string, unknown> {
  const a = args ?? {};
  const out: Record<string, unknown> = {};
  if (typeof a.command === "string") out.command = a.command;
  if (typeof a.path === "string") out[a.pattern === undefined ? "file_path" : "path"] = a.path;
  if (typeof a.pattern === "string") out.pattern = a.pattern;
  return out;
}

let queue: Promise<void> = Promise.resolve();

function run(event: string, payload: string): Promise<void> {
  return new Promise((done) => {
    try {
      const child = spawn(RELAY, ["--agent", "pi", event], { stdio: ["pipe", "ignore", "ignore"], detached: true });
      setTimeout(done, 2000).unref();
      child.on("error", () => done());
      child.on("exit", () => done());
      child.stdin?.on("error", () => {});
      child.stdin?.end(payload);
      child.unref();
    } catch { done(); }
  });
}

function send(event: string, ctx: any, extra: Record<string, unknown> = {}): void {
  try {
    const payload = JSON.stringify({ hook_event_name: event, session_id: ctx.sessionManager.getSessionId(), cwd: ctx.cwd, ...extra });
    queue = queue.then(() => run(event, payload));
  } catch {}
}

function text(content: any): string | undefined {
  if (!Array.isArray(content)) return undefined;
  const s = content.filter((c) => c?.type === "text").map((c) => c.text).join("\n").trim();
  return s || undefined;
}

export default function (pi: any) {
  let last: string | undefined;
  let failed = false;
  pi.on("session_start", (_e: any, ctx: any) => send("SessionStart", ctx));
  pi.on("session_shutdown", (_e: any, ctx: any) => send("SessionEnd", ctx));
  pi.on("before_agent_start", (_e: any, ctx: any) => { last = undefined; failed = false; send("UserPromptSubmit", ctx); });
  pi.on("tool_execution_start", (e: any, ctx: any) =>
    send("PreToolUse", ctx, { tool_name: e.toolName, tool_input: toolInput(e.args) }));
  pi.on("tool_execution_end", (e: any, ctx: any) =>
    send(e.isError ? "PostToolUseFailure" : "PostToolUse", ctx, { tool_name: e.toolName }));
  pi.on("agent_end", (e: any, ctx: any) => {
    try {
      const reply = [...(e.messages ?? [])].reverse().find((m: any) => m?.role === "assistant");
      if (reply?.stopReason === "error") { failed = true; send("StopFailure", ctx, { error: reply.errorMessage }); }
      else last = text(reply?.content) ?? last;
    } catch {}
  });
  pi.on("agent_settled", (_e: any, ctx: any) => { if (!failed) send("Stop", ctx, { last_assistant_message: last }); });
}
```

`ctx.sessionManager.getSessionId()` is in Pi but not in its documentation; if a later Pi drops
it, the extension sends nothing rather than failing.

### Cursor

Cursor (the editor and `cursor-agent`) runs the hooks in `~/.cursor/hooks.json`, and its JSON is
read as is: no wrapper needed.

```json
{
  "version": 1,
  "hooks": {
    "sessionStart": [{ "command": "~/.local/share/vultures-ai/bin/vultures-ai-hook --agent cursor", "timeout": 5 }],
    "sessionEnd": [{ "command": "~/.local/share/vultures-ai/bin/vultures-ai-hook --agent cursor", "timeout": 5 }],
    "postToolUse": [{ "command": "~/.local/share/vultures-ai/bin/vultures-ai-hook --agent cursor", "timeout": 5 }],
    "postToolUseFailure": [{ "command": "~/.local/share/vultures-ai/bin/vultures-ai-hook --agent cursor", "timeout": 5 }],
    "stop": [{ "command": "~/.local/share/vultures-ai/bin/vultures-ai-hook --agent cursor", "timeout": 5 }]
  }
}
```

- If the island stays empty, write your home's full path in place of `~`.
- `preToolUse` and `beforeSubmitPrompt` are left out on purpose: Cursor reads their output as a
  decision, and whether an empty one counts as "go ahead" is not documented. So the island sees
  finished steps, not steps starting.
- Cursor also runs Claude Code's hooks (its "third-party configs" setting, on by default). With
  the Claude Code hooks installed, Cursor sessions may already show, as Claude Code; adding the
  hooks above too would show each of them twice.

### Tools that copy Claude Code's hooks

These run a command per event with Claude Code's JSON, so the relay with `--agent <name>` is all
they need. From their documentation, not tried here:

| Tool | Where its hooks go | Name to use |
|---|---|---|
| Factory Droid | `~/.factory/settings.json`, `hooks` | `--agent droid` |
| Qwen Code | `~/.qwen/settings.json`, `hooks` | `--agent qwen` |
| Kimi CLI | `[[hooks]]` in its `config.toml` | `--agent kimi` |
| GitHub Copilot CLI | `~/.copilot/hooks/*.json` | `--agent copilot` |

Their tool names differ from Claude Code's (`WriteFile`, `ReadFile`…), so some steps show as
generic work. Copilot CLI also reads a repository's `.claude/settings.json`: with project-level
Claude Code hooks, its sessions may already show.

## Trying it

```sh
echo '{"hook_event_name":"SessionStart","session_id":"s1","cwd":"'$PWD'"}' \
  | ~/.local/share/vultures-ai/bin/vultures-ai-hook --agent my-tool
echo '{"hook_event_name":"PreToolUse","session_id":"s1","tool_name":"Bash","tool_input":{"command":"make"}}' \
  | ~/.local/share/vultures-ai/bin/vultures-ai-hook --agent my-tool
echo '{"hook_event_name":"SessionEnd","session_id":"s1"}' \
  | ~/.local/share/vultures-ai/bin/vultures-ai-hook --agent my-tool
```

The protocol behind it is in [Hook protocol](../reference/protocol.md).
