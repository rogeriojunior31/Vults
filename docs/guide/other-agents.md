# Other agents

Claude Code, Codex and Gemini CLI are built in: **Settings → Agents** installs their hooks. Any other tool can
put its sessions on the wire too, if it runs a command on its events and sends Claude Code's hook
JSON on stdin (the format most agent tools copy).

Point the tool's hooks at the relay with a name of your choice:

```sh
~/.local/share/vultures-ai/bin/vultures-ai-hook --agent my-tool SessionStart
```

- The name is 1 to 24 characters of `a-z`, `0-9` and `-`. `claude`, `codex`, `gemini` and `other` are taken,
  so nothing can pass for a built-in agent. With a name that breaks these rules the hook sends
  nothing and exits at once: the tool never waits on it.
- The event name comes from `hook_event_name` in the JSON, or from the last argument.
- Each session needs a `session_id`. Sessions show under the tool's name, in violet.

The island follows these events: `SessionStart`, `SessionEnd`, `UserPromptSubmit`, `PreToolUse`,
`PostToolUse`, `PostToolUseFailure`, `PermissionRequest`, `Notification`, `Stop`, `StopFailure`,
`SubagentStart` and `SubagentStop`. Fields it reads: `session_id`, `cwd`, `tool_name`,
`tool_input`, `message`, `last_assistant_message`, `error`.

## Permissions

The island never answers another tool's permission: it has no way to know what that tool expects
back. A `PermissionRequest` shows as a question waiting in the terminal, the hook does not wait, and
the tool asks you there as usual.

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
