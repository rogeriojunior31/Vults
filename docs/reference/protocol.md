# Hook protocol

`vultures-ai-hook` and the app exchange one JSON object per line over a local socket.

## Transport

| Platform | Endpoint | Access control |
|---|---|---|
| Linux | `$XDG_RUNTIME_DIR/vultures-ai.sock` (fallback `/tmp/vultures-ai-<uid>/`, folder `0700`) | socket `0600`, `SO_PEERCRED` |
| Windows | `\\.\pipe\vultures-ai-<SID>` | both ends check the other process's SID |
| macOS | planned | `getpeereid` |

## Messages

Hook to app, version 1:

```json
{ "kind": "event", "v": 1, "id": "18f…-1a2b", "agent": "claude", "event": "PermissionRequest",
  "wants_reply": true,
  "terminal": { "cwd": "/home/me/project", "pid": 4242, "env": { "TERM_PROGRAM": "kitty" } },
  "payload": { "tool_name": "Bash", "tool_input": { "command": "cargo test" } } }
```

- `agent`: `claude` or `codex`.
- `id`: unique per message, opaque.
- `terminal`: every field is optional. `env` only lists terminal-identifying variables that were set.
- `payload`: the agent's hook JSON without `tool_response` and `transcript_path`; strings are capped
  at 2000 bytes.

App to hook, only when `wants_reply` is true:

```json
{ "kind": "decision", "v": 1, "id": "18f…-1a2b", "decision": "allow" }
{ "kind": "unsupported", "v": 1, "id": "18f…-1a2b" }
```

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
