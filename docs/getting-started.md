# Getting started

## Install

Releases ship a `.deb` and an AppImage for Linux and an installer for Windows. Until the first
release, build from source:

```sh
git clone https://github.com/rogeriojunior31/vultures-ai
cd vultures-ai
npm install
npm run tauri dev        # run it
npm run bundle:linux     # or build the .deb and AppImage into target/release/bundle/
```

Linux needs `webkit2gtk-4.1`, `gtk3`, `gtk-layer-shell` and `libayatana-appindicator`. On KDE Plasma,
Hyprland, Sway and other compositors with layer-shell, the island sits on the top edge like a panel;
on GNOME it is a regular always-on-top window.

## Connecting your agents

Open **Set up agents…** from the tray icon. For each agent you see the file that will change, the exact
diff, and a **Write the file** button; a dated backup is taken first, and hooks from other tools are
kept. **Remove hooks…** takes out only what Vultures AI added.

| Agent | File | After installing |
|---|---|---|
| Claude Code | `~/.claude/settings.json` | Nothing: new sessions report to the island |
| Codex | `~/.codex/hooks.json` | Open Codex, type `/hooks` and trust the Vultures AI hooks. Codex runs a hook only once you trust it, and only Codex records that trust. The settings window shows how many are still waiting |

## Next

- [The island](guide/island.md): the flock, folded and open, getting to a session
- [Approving from the island](guide/approvals.md)
- [Chatting with Zeca](guide/chat.md)
- [Connectors](guide/connectors.md)
