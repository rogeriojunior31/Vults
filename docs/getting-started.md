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
