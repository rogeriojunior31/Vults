# Hook protocol

`vultures-ai-hook` and the app exchange one JSON object per line over a local socket.

## Transport

| Platform | Endpoint | Access control |
|---|---|---|
| Linux | `$XDG_RUNTIME_DIR/vultures-ai.sock` (fallback `/tmp/vultures-ai-<uid>/`, folder `0700`) | socket `0600`, `SO_PEERCRED` |
| Windows | `\\.\pipe\vultures-ai-<SID>` | both ends check the other process's SID |
| macOS | planned | `getpeereid` |

## Messages

Hook to app, version 3:

```json
{ "kind": "event", "v": 3, "id": "18f…-1a2b", "agent": "claude", "event": "PermissionRequest",
  "wants_reply": true,
  "terminal": { "cwd": "/home/me/project", "pid": 4242, "env": { "TERM_PROGRAM": "kitty" } },
  "payload": { "tool_name": "Bash", "tool_input": { "command": "cargo test" } } }
```

- `agent`: `claude`, `codex`, `gemini`, or `other` for any other tool (see [Other agents](../guide/other-agents.md)).
- `agent_name`: only with `other`, the tool's name: 1 to 24 of `a-z`, `0-9` and `-`, never `claude`,
  `codex`, `gemini` or `other`. Absent otherwise.
- `id`: unique per message, opaque.
- `terminal`: every field is optional. `env` only lists terminal-identifying variables that were set.
- `payload`: the agent's hook JSON without `tool_response` and `transcript_path`; strings are capped
  at 2000 bytes. Two things survive in `tool_response`: a tool's `error`, and on a `PostToolUse` Claude
  Code's `structuredPatch`, cut to its first 400 lines (with `"cut": true` when lines were left out).
  On a `PostToolUse` of Codex's `apply_patch`, `tool_input.command` (the patch) keeps up to 64 KiB.
  The fields are optional and the envelope did not change, so this needed no new version.

An event waits for a reply (`wants_reply`) when it is a `PermissionRequest` from Claude Code or
Codex, or a Claude Code `PreToolUse` for `AskUserQuestion` sent by an entry installed with `--ask`
(`vultures-ai-hook --agent claude --ask PreToolUse`, with a 120-second timeout). An entry without the
flag has the short timeout of every other event, so it never waits.

App to hook, only when `wants_reply` is true:

```json
{ "kind": "decision", "v": 3, "id": "18f…-1a2b", "decision": "allow" }
{ "kind": "answer", "v": 3, "id": "18f…-1a2b", "answers": ["Blue", ["S", "M"]] }
{ "kind": "unsupported", "v": 3, "id": "18f…-1a2b" }
```

`answers` has one entry per question, in the order of `tool_input.questions`: a string (a choice's
label, or the user's own words) or, for a multi-select, a list of them. It goes by position because
the app only saw the questions with their strings capped; the hook keys the answers with the
questions it read.

An event from `gemini` or `other` never has `wants_reply`: Gemini's hooks can't approve a tool, and
nothing in the app answers another tool's permission.

`StatusLine` is Claude Code's statusLine input, sent by `vultures-ai-hook --agent claude --statusline`.
Its payload is only `rate_limits` (the plan's 5-hour and weekly windows) and `session_id`; the
session's paths, cost and model never leave the hook. Before the session's first reply there are
no `rate_limits`, and nothing is sent. The hook prints only what the user's own status line prints
(saved on install, see [settings](settings.md)), or nothing when there is none, so Claude Code's
status line looks as it did before.

Version 2 added `other` and `agent_name`, then `gemini`. Version 3 added `answer`. The app installs its own hook when it starts, so the two
always speak the same version; an event from another version gets `unsupported`.

A connection that gets no reply, a reply for another `id`, or `unsupported` makes the hook print
nothing.

## Limits

| Limit | Value |
|---|---|
| Message size | 1 MiB |
| Hook: connect | 300 ms |
| Hook: event with no reply | 2 s in total |
| Hook: waiting for a decision | 110 s |
| App: UI acknowledgement of a card | 800 ms |
| App: decision | 108 s |
| App: reading a message | 5 s |
| App: simultaneous connections | 32 |

## Agent output

The hook turns a decision into the format each agent expects. Claude Code and Codex read the same
`PermissionRequest` output:

```json
{"hookSpecificOutput":{"hookEventName":"PermissionRequest","decision":{"behavior":"allow"}}}
{"hookSpecificOutput":{"hookEventName":"PermissionRequest","decision":{"behavior":"deny","message":"Denied from Vultures AI"}}}
```

An answer becomes Claude Code's `PreToolUse` output: the tool runs with the answers added to its
own input, keyed by each question's text (checked against Claude Code 2.1.286):

```json
{"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"allow",
  "updatedInput":{"questions":[…],"answers":{"Which color?":"Blue","Which sizes?":["S","M"]}}}}
```

When the number of answers does not match the questions, the hook prints nothing and Claude Code
asks in its terminal.
