# Getting started

## Install

Releases ship a `.deb` and an `.rpm` (Ubuntu 22.04, Debian 12, Fedora and newer) and an installer
for Windows. There is no AppImage for now. Get them from the
[latest release](https://github.com/rogeriojunior31/vultures-ai/releases/latest) and check a download
with `sha256sum -c SHA256SUMS --ignore-missing`. Or build from source:

```sh
git clone https://github.com/rogeriojunior31/vultures-ai
cd vultures-ai
npm install
npm run tauri dev        # run it
npm run bundle:linux     # or build the .deb and .rpm into target/release/bundle/
```

Linux needs `webkit2gtk-4.1`, `gtk3`, `gtk-layer-shell`, `libayatana-appindicator`, `openssl`, `alsa-lib`, `gst-plugins-good` (the island's sounds:
WebKitGTK plays them through GStreamer)
and the Vulkan loader (building also needs `cmake`, the Vulkan headers and `glslc`). On KDE Plasma,
Hyprland, Sway and other compositors with layer-shell, the island sits on the top edge like a panel;
on GNOME it is a regular always-on-top window.

## Connecting your agents

Open **Set up agents…** from the tray icon and click **Install hooks…** next to an agent. You see the
file that will change, the exact diff, and a **Write the file** button; a dated backup is taken first, and hooks from other tools are
kept. **Remove hooks…** takes out only what Vultures AI added.

| Agent | File | After installing |
|---|---|---|
| Claude Code | `~/.claude/settings.json` | Nothing: new sessions report to the island |
| Codex | `~/.codex/hooks.json` | Open Codex, type `/hooks` and trust the Vultures AI hooks. Codex runs a hook only once you trust it, and only Codex records that trust. The settings window shows how many are still waiting. A reinstall that changes a hook asks for that trust again |
| Gemini CLI | `~/.gemini/settings.json` | Nothing: new sessions report to the island. Gemini asks its permissions in its own terminal (see [Approving](guide/approvals.md)) |
| Antigravity | `~/.gemini/config/hooks.json` | Nothing: the agy CLI, the app and the IDE report to the island, under the name `antigravity`. Antigravity asks its permissions itself (see [Other agents](guide/other-agents.md#antigravity)) |

Other tools (OpenCode, Pi, Cursor…) can report too, with a few lines in their own config: see
[Other agents](guide/other-agents.md).

## Next

- [The island](guide/island.md): the flock, folded and open, getting to a session
- [Approving from the island](guide/approvals.md)
- [Chatting with Zeca](guide/chat.md)
- [Connectors](guide/connectors.md)
