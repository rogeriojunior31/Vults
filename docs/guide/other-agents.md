# Other agents

Claude Code, Codex, Antigravity, OpenCode, Hermes Agent, Qwen Code, Copilot CLI, Cursor, Factory Droid and Kimi Code are built in (**Settings →
Agents**). Any other tool can put its sessions on the wire too, if it runs a command on its events
and sends Claude Code's hook JSON on stdin (the format most agent tools copy). For tools that run
plugins instead (Pi), a short plugin does the same; recipes are below.

Point the tool's hooks at the relay with a name of your choice (on Windows the relay is in
`%LOCALAPPDATA%\Vults\bin\`):

```sh
~/.local/share/vults/bin/vults-hook --agent my-tool SessionStart
```

- The name is 1 to 24 characters of `a-z`, `0-9` and `-`, and not one the wire has a kind for
  (`claude`, `codex`, `copilot`, `opencode`, `qwen` and `other`), nor `gemini`: the wire has no kind
  for it any more, but Gemini CLI's old hooks may still be installed, so the name stays taken and
  they send nothing. The built-ins Vults sends
  by name (`antigravity`, `hermes`, `cursor`, `droid`, `kimi`) use those names themselves, so they are not
  reserved; a tool by any name gets no card, a built-in sent by name included. With a name that
  breaks these rules the hook sends nothing and exits at once.
- The event name comes from `hook_event_name` in the JSON, or from the last argument. Claude Code's
  event names are read, and Cursor's too.
- Each session needs an id: `session_id`, or `conversation_id`, `sessionId` or `conversationId`.
  An event without one is dropped. Sessions show under the tool's name, in violet.

The fields it reads are in [Hook protocol](../reference/protocol.md). The relay always exits 0 and
prints nothing for these tools, but a relay that is missing makes the command fail: end every hook
command in `|| exit 0`, or a tool that treats a failed hook as a "no" stops working when the app
is not installed.

## What each agent can do

**Settings → Agents**, under *What each agent can do*, shows each built-in agent: whether its CLI is on the `PATH` and its
version, whether its hooks are in and current (and, for Codex, trusted), and what it can do with
Vults. **Check again** asks anew; the answer is kept ten minutes.

| | Permissions | Questions | Diffs | Stopping a turn | Sandbox | Started from Vults |
|---|---|---|---|---|---|---|
| Claude Code | a card | a card | lines, numbered | at once, when Vults started it | bubblewrap, if on | yes |
| Codex | a card | its terminal | lines | at once, when Vults started it | its sandbox mode | yes |
| Antigravity | its terminal | its terminal | lines | not from Vults yet | none | not yet |
| OpenCode | a card | a card (OpenCode 1) | lines | not from Vults | none | no |
| Hermes Agent | its terminal | its terminal | patch lines and repository preview | not from Vults | agent-managed | no |
| Qwen Code | a card | its terminal | lines | not from Vults | a container, if on | no |
| Copilot CLI | a card | none | lines | not from Vults | none | no |
| Cursor | its terminal | its terminal | lines | not from Vults | none | no |
| Factory Droid | its terminal | its terminal | lines | not from Vults | none | no |
| Kimi Code | its terminal | its terminal | lines | not from Vults | none | no |

Vults stops only its own Chat turns or a session it resumed through `/continue`
([Chat](chat.md#continuing-a-session)). A watched terminal session stays under your control.
Codex's sandbox depends on its mode: `workspace-write`
writes only in its folder, the temp folders and the folders it is given. *If on*: the sandbox is off
until you turn it on in that agent. None of these sandboxes has been tried with Vults yet.

## Permissions

The island never answers the permission of a tool that is not built in: it has no way to know what
that tool expects back. A `PermissionRequest` shows as a question waiting in the terminal, the hook
does not wait, and the tool asks you there as usual.

## Antigravity

**Settings → Agents → Antigravity** installs a hook named `vults` in `~/.gemini/config/hooks.json`,
the file the agy CLI, the Antigravity app and the IDE share (checked with agy 1.2.16). The other
hooks there are never touched; if one of yours is already named `vults`, the installer writes
nothing. Its sessions show as `antigravity`.

The island shows Antigravity thinking, each step (`view_file`, `run_command`…) and when it stops,
and each edit's diff, rebuilt from the text it replaced (without line numbers).
Antigravity does not send its reply, so a finished session has no summary line, and it has no
session start or end. Silence no longer removes it: an open session can rest or fly, and a previously observed local process must be confirmed gone before cleanup. Its hook runs before every tool and does not say whether Antigravity would have asked,
so Antigravity asks its permissions itself ([Approving](approvals.md#agents-that-ask-in-their-own-prompt)).

## Cursor

**Settings → Agents → Cursor** adds its hooks to `~/.cursor/hooks.json`, the file the editor and
`cursor-agent` share; the hooks of other tools in it are kept, and **Remove hooks…** takes out only
Vults' own. Restart Cursor after. Its sessions show as `cursor`, in the project Cursor works in.

The island shows each finished step, failures included, and when a turn stops. Its hooks that run
before a step (`preToolUse`, `beforeSubmitPrompt`, `beforeShellExecution`…) are left out: Cursor
reads what they print as a decision, so Cursor asks its permissions itself
([Approving](approvals.md#agents-that-ask-in-their-own-prompt)). Each edit comes from
`afterFileEdit`, which Cursor reads nothing from, with its diff (without line numbers). What a tool
printed and the account's email never leave the hook.

Cursor also runs Claude Code's hooks (its *third-party* hooks, on by default). The relay ignores
Cursor's events there, so a session never shows twice, once as Claude Code.

## Factory Droid

**Settings → Agents → Factory Droid** adds its hooks to `~/.factory/settings.json` (Droid reads
Claude Code's hooks; the rest of the file and other tools' hooks are kept). Restart Droid after.
Its sessions show as `droid` (checked with droid 0.239.0).

The island shows each step as it starts and ends, an edit's lines, and when a turn stops. A
command that fails sends no event of its own: its step ends with the turn. Droid has no
permission event, and its `PreToolUse` runs before every tool whether it would ask or not, so
Droid asks its permissions itself ([Approving](approvals.md#agents-that-ask-in-their-own-prompt)).

## Kimi Code

**Settings → Agents → Kimi Code** adds its hooks to `~/.kimi-code/config.toml` (`KIMI_CODE_HOME`
moves it), as `[[hooks]]` tables beside your own settings: your comments and layout stay, and other
tools' hooks are kept. Restart Kimi after. Its sessions show as `kimi` (checked with Kimi Code
2.1.1).

The island shows each step, an edit's lines, a failed command and when a turn stops. Kimi tells its
`PermissionRequest` hook about a permission without waiting for an answer, so the island shows it
waiting in Kimi's terminal and Kimi asks itself ([Approving](approvals.md#agents-that-ask-in-their-own-prompt)).

## OpenCode

**Settings → Agents → OpenCode** writes a plugin to `~/.config/opencode/plugins/vults.js`, with the
same diff, backup and click as the hooks; restart OpenCode after. It reports every step and puts
OpenCode's permissions and questions on the island ([Approving](approvals.md#opencode)). A file
there without the *Generated by Vults* first line is yours, and the installer leaves it alone.
OpenCode has no event when it quits, so its birds leave the wire once quiet.

## Recipes

These were checked against each tool's documentation and installed copy (versions below), with
the relay missing too; none was run inside a live session of the tool. Each one only reports: it
never answers a permission.

### Pi

Pi (1.0) loads extensions from `~/.pi/agent/extensions/`. Save this as
`~/.pi/agent/extensions/vults.ts`. It listens to `tool_execution_start` and `_end`, never to
`tool_call`: a `tool_call` handler that fails stops the tool.

```ts
import { spawn } from "node:child_process";
import { homedir } from "node:os";
import { join } from "node:path";

const RELAY = join(homedir(), ".local/share/vults/bin/vults-hook");

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

## Trying it

```sh
echo '{"hook_event_name":"SessionStart","session_id":"s1","cwd":"'$PWD'"}' \
  | ~/.local/share/vults/bin/vults-hook --agent my-tool
echo '{"hook_event_name":"PreToolUse","session_id":"s1","tool_name":"Bash","tool_input":{"command":"make"}}' \
  | ~/.local/share/vults/bin/vults-hook --agent my-tool
echo '{"hook_event_name":"SessionEnd","session_id":"s1"}' \
  | ~/.local/share/vults/bin/vults-hook --agent my-tool
```

The protocol behind it is in [Hook protocol](../reference/protocol.md).

## Hermes Agent

Install the observer in **Settings → Agents → Hermes Agent**, then run
`hermes plugins enable vults --no-allow-tool-override` and start a new CLI session. A running
Hermes gateway may reload the plugin immediately. Files live in
`~/.hermes/plugins/vults/{__init__.py,plugin.yaml}`; `HERMES_HOME` selects another home.
The installer previews and checks both owned files before writing, keeps dated backups and
refuses to overwrite foreign files. Disable it with `hermes plugins disable vults` before removal.

The observer relays session/turn lifecycle, tool calls, failures, native call IDs and subagents.
`patch` edits keep their diffs; repository changes also cover scripts and writes. Permissions,
questions and stopping stay in Hermes. No tool arguments or tool outputs are modified.

Metrics use numeric `post_api_request` reports and a read-only lookup of the known session in
`HERMES_HOME/state.db`. Context is the latest request's tokens including cache, not cumulative
session usage; percentage is shown only with a reported context limit. Model response time sums
completed API durations observed since the plugin loaded, with repeated request IDs deduplicated.
It excludes tool execution and idle time. Cost prefers Hermes' actual reported value, then an
explicit native estimate; unknown remains unknown and an included subscription is zero only when
Hermes explicitly reports it. Sub-cent amounts stay visible. No account key, conversation text or
raw response is read or forwarded, and no model request is needed to read these metrics.

Validated on installed Hermes 0.22.2: the native Plugin Doctor accepts the plugin, and the actual
runtime dispatcher delivered a session, controlled context/time metrics and a +1/−1 patch to the
running Flock without inference. Native cost selection is checked against an isolated database.

Hermes Desktop runtime eviction/finalization is treated as rest, because it can leave the durable conversation open. Reset ends only an explicitly named old session and starts/resumes the current one. A durable `ended_at` in the local session database or an explicit deletion closes the bird; missing/unreadable lifecycle data does not.
