# 0027. An open reply for any tool that asks for one

**Status:** Proposed, 2026-10-12. Amends [0004](0004-a-human-answers-permissions.md) and
[0014](0014-consent-before-autonomy.md), within [0003](0003-never-block-an-agent.md) and
[0025](0025-destructive-calls-always-ask.md).

## Context

Vults answers a permission only for the agents it knows: each one reads its own reply shape (Claude
Code's `hookSpecificOutput`, Copilot CLI's bare `behavior`), so each was added by hand after a
recorded session. Any other tool can already put its sessions on the wire with
`vults-hook --agent <name>` and Claude Code's hook JSON, but its `PermissionRequest` is shown as a
question waiting in its terminal and the hook never waits: nothing in the app knows what that tool
would read back (see *Permissions* in [Other agents](../guide/other-agents.md)).

That keeps the door shut for the tools people build themselves (scripts, in-house agents,
orchestrators running several agents), which are what the flock and its cards are for. A tool
that wants a card should be able to ask for one, if it takes one fixed answer and honors it.

## Decision

- **A tool asks for the reply.** Its hook command adds `--reply vults`:
  `vults-hook --agent <name> --reply vults PermissionRequest`. Without it nothing changes: the
  hook never waits for a named tool, as today. Built-in names (`claude`, `codex`, `copilot`,
  `gemini`, `opencode`, `qwen`, `other`, and every agent the registry lists, such as `cursor` or
  `kimi`) cannot take it.
- **It sends Claude Code's hook JSON**, the format most tools already copy: a `PermissionRequest`
  with `session_id`, `cwd`, `tool_name` and `tool_input`. The card is built from it as for any
  agent: the whole command, file or URL, never a summary the tool wrote.
- **It reads one fixed reply** on stdout, one JSON line:
  `{"decision":"allow"}` or `{"decision":"deny","reason":"Denied from Vults"}`.
  Anything else, empty stdout included, means *ask the user yourself*: no click in time, the app
  closed or paused, an unsupported protocol version, any failure. The hook still exits 0 and never
  blocks the tool ([0003](0003-never-block-an-agent.md)); silence is never an allow.
- **Who answers is who answers today** ([0004](0004-a-human-answers-permissions.md)): a click, a
  signed answer from a paired phone ([0017](0017-a-signed-phone-decision-is-a-click.md)), or an
  exact *Always* rule the user made on that tool's own card. Rules are kept by the tool's name, its
  folder, the tool and the target: a rule made for one named tool never answers another, nor a
  built-in agent.
- **Policies** ([0014](0014-consent-before-autonomy.md)) reach a named tool only when they name it
  (`agents my-tool`), and then only to **deny** or **ask**: a policy never allows a named tool's
  call, as it never allows any agent's but Claude Code's today. A destructive call always asks
  ([0025](0025-destructive-calls-always-ask.md)). Every answer is audited with the tool's name.
- **A named tool gains nothing a built-in agent's card would not show.** It reaches the same
  socket any process of the user's can already reach (same user, checked by `peer`); its card
  shows exactly what it sent; an Allow goes back only to the call that asked. Vults cannot make a
  tool honor the reply, as it cannot make any agent: a tool that ignores it is one that never
  asked.
- **A name that becomes a built-in agent** later loses its rules: they are not carried over to an
  agent that reads another reply.

## Consequences

- G2 builds it: the hook's flag and reply, a protocol `VERSION` bump (an old app answers
  `unsupported`, so the tool asks itself), rules and policies keyed by name in the core, the named
  tools in **Settings → Agents** with their rules, and [Hook protocol](../reference/protocol.md)
  written as a spec with an example tool in shell and in TypeScript.
- `CLAUDE.md`'s rule 2, [Safety](../safety.md) and [Other agents](../guide/other-agents.md) say a
  tool that asks with `--reply vults` gets a card, and how it is answered.
- A test tool in shell gets a card, an allow and a deny, and asks itself when the app is closed;
  an old hook stays silent.
