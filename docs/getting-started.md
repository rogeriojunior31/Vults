# Getting started

## Install

Releases ship a `.deb` and an `.rpm` (Ubuntu 22.04, Debian 12, Fedora and newer) and an installer
for Windows. There is no AppImage for now. Get them from the
[latest release](https://github.com/rogeriojunior31/Vults/releases/latest) and check a download
with `sha256sum -c SHA256SUMS --ignore-missing`.

On Arch Linux, build the package with `makepkg`: `packaging/aur/vults` builds the latest
release, `packaging/aur/vults-git` follows `main`. Both build from source, so the first install
takes a while. They are not on the AUR yet.

```sh
git clone https://github.com/rogeriojunior31/Vults
cd Vults/packaging/aur/vults && makepkg -si
```

Or build from source:

```sh
git clone https://github.com/rogeriojunior31/Vults
cd Vults
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
kept. **Remove hooks…** takes out only what Vults added.

| Agent | File | After installing |
|---|---|---|
| Claude Code | `~/.claude/settings.json` | Nothing: new sessions report to the island |
| Codex | `~/.codex/hooks.json` | Open Codex, type `/hooks` and trust the Vults hooks. Codex runs a hook only once you trust it, and only Codex records that trust. The settings window shows how many are still waiting. A reinstall that changes a hook asks for that trust again |
| Gemini CLI | `~/.gemini/settings.json` | Nothing: new sessions report to the island. Gemini asks its permissions in its own terminal (see [Approving](guide/approvals.md)) |
| Antigravity | `~/.gemini/config/hooks.json` | Nothing: the agy CLI, the app and the IDE report to the island, under the name `antigravity`. Antigravity asks its permissions itself (see [Other agents](guide/other-agents.md#antigravity)) |
| OpenCode | `~/.config/opencode/plugins/vults.js`, a plugin of ours | Restart OpenCode. Its sessions report to the island, and its permissions and questions show there as cards: answer on the island or in OpenCode, whichever comes first (see [Approving](guide/approvals.md#opencode)) |
| Qwen Code | `~/.qwen/settings.json` | Restart Qwen Code. Its sessions report to the island, and its permissions show there as cards (see [Approving](guide/approvals.md#qwen-code)) |

Other tools (Pi, Cursor…) can report too, with a few lines in their own config: see
[Other agents](guide/other-agents.md).

### Coming from Vultures AI

Vults was called Vultures AI up to 0.1.5. On its first start it moves the old folders
(`~/.config/vultures-ai`, `~/.local/share/vultures-ai`, …) to the new names and the API keys to
the new keyring entry, and leaves a link where the old hook was, so your agents keep reporting.
If it started at login, the old login entry is swapped for the new one. The `.deb` and `.rpm`
replace the old `vultures-ai` package instead of installing beside it.
Open **Set up agents…** and click **Update hooks…** next to each agent: the old entries are
replaced, not doubled. Codex asks for its trust again.

## Language

Vults speaks English, Brazilian Portuguese, Spanish and Simplified Chinese. It follows your
desktop's language; **Settings → General → Language** picks another for the island, Settings, the
tray and the notifications. What your agents say stays in their own words.

## Next

- [The island](guide/island.md): the flock, folded and open, getting to a session
- [Approving from the island](guide/approvals.md)
- [Chatting with Zeca](guide/chat.md)
- [Connectors](guide/connectors.md)
