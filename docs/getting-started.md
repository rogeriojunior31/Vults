# Getting started

There is no release yet. To build the hook relay from source:

```sh
git clone https://github.com/rogeriojunior31/vultures-ai
cd vultures-ai
cargo build --release -p vultures-ai-hook
cargo test --workspace
```

## Connecting your agents

Open **Set up agents…** from the tray icon. For each agent you see the file that will change, the exact
diff, and a **Write the file** button; a dated backup is taken first, and hooks from other tools are
kept. **Remove hooks…** takes out only what Vultures AI added.

| Agent | File | After installing |
|---|---|---|
| Claude Code | `~/.claude/settings.json` | Nothing: new sessions report to the island |
| Codex | `~/.codex/hooks.json` | Open Codex, type `/hooks` and trust the Vultures AI hooks. Codex runs a hook only once you trust it, and only Codex records that trust. The settings window shows how many are still waiting |

## Chatting with Zeca

Click Zeca on the wire, or choose **Chat…** from the tray icon. Pick Claude or Codex at the top of the
panel; **New** starts a fresh conversation. Drop a file on the island to ask about it: Zeca picks it
up and swallows it, and the next message carries it.

Both stream the reply as it is written: Claude through `claude -p`, Codex through one long-lived
`codex app-server` (with a fallback to `codex exec` on versions without it).

The chat runs through the `claude` or `codex` command you already logged into, so it uses your own
subscription and Vultures AI never sees your credentials. Every turn is read-only: the model may read
the files you drop, never change anything. Chat turns ignore your hooks and settings, so they never
show up on the island as an agent session.

## Connectors

**Set up agents…** also lists the connectors; each is off until you switch it on.

- **GitHub** watches your open pull requests (checks, approvals, changes requested), reviews requested
  from you, and checks on the default branch of your recently pushed repositories, every two minutes.
  It uses the GitHub CLI you are already logged into (`gh auth login`), so Vultures AI never sees a
  token. The first check only learns how things are: alerts start with the next change.

Click an alert to open it on GitHub; × dismisses it.
