# Getting started

## Install

Releases ship a `.deb` and an `.rpm` (Ubuntu 22.04, Debian 12, Fedora and newer) and a preview
installer for Windows (no global shortcuts, desktop notifications, Open terminal or tray presets
yet). There is no AppImage for now. Get them from the
[latest release](https://github.com/rogeriojunior31/Vults/releases/latest) and check a download
with `sha256sum -c SHA256SUMS --ignore-missing`. To check that it was built by this repository's
release workflow: `gh attestation verify <file> --repo rogeriojunior31/Vults`.

On Arch Linux, build the package with `makepkg`: `packaging/aur/vults` builds the latest release,
`packaging/aur/vults-git` follows `main`. Both build from source, so the first install takes a
while. They are not on the AUR yet.

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

Linux needs `webkit2gtk-4.1`, `gtk3`, `gtk-layer-shell`, `libayatana-appindicator`, `openssl`,
`alsa-lib`, `gst-plugins-good` (WebKitGTK plays the island's sounds through GStreamer) and the
Vulkan loader (building also needs `cmake`, the Vulkan headers and `glslc`). On KDE Plasma,
Hyprland, Sway and other compositors with layer-shell, the island sits on the top edge like a panel;
on GNOME it is a regular always-on-top window.

## Connecting your agents

Open **Set up agents…** from the tray icon and click **Install hooks…** next to an agent. You see the
file that will change, the exact diff, and a **Write the file** button; a dated backup is taken
first, and hooks from other tools are kept. **Remove hooks…** takes out only what Vults added.

| Agent | File | After installing |
|---|---|---|
| Claude Code | `~/.claude/settings.json` | Nothing: new sessions report to the island |
| Codex | `~/.codex/hooks.json` | Open Codex, type `/hooks` and trust the Vults hooks (Codex runs a hook only when it is trusted and on); Settings shows how many still wait. A reinstall asks again |
| Antigravity | `~/.gemini/config/hooks.json` | Nothing ([it asks in its own terminal](guide/other-agents.md#antigravity)) |
| OpenCode | `~/.config/opencode/plugins/vults.js`, a plugin of ours | Restart OpenCode ([its cards](guide/approvals.md#opencode)) |
| Qwen Code | `~/.qwen/settings.json` | Restart Qwen Code ([its cards](guide/approvals.md#qwen-code)) |
| Copilot CLI | `~/.copilot/hooks/vults.json`, a file of ours | Restart Copilot CLI ([its cards](guide/approvals.md#copilot-cli)) |
| Cursor | `~/.cursor/hooks.json` | Restart Cursor ([it asks in its own prompt](guide/other-agents.md#cursor)) |

With `CLAUDE_CONFIG_DIR`, `CODEX_HOME`, `QWEN_HOME`, `COPILOT_HOME` or `XDG_CONFIG_HOME` set, the file in that
folder is used. Antigravity's is always `~/.gemini/config/hooks.json`: `GEMINI_CLI_HOME` does not
move it.

Vults no longer supports Gemini CLI. If an earlier version installed hooks in Gemini's settings,
**Settings → Agents** shows a Gemini CLI card whose **Remove hooks…** takes them out.

Other tools (Pi, Cursor…) can report too, with a few lines in their own config: see
[Other agents](guide/other-agents.md).

### Coming from Vultures AI

Vults was called Vultures AI up to 0.1.5; on its first start it moves the old folders, keyring entry
and login entry to the new names, and the `.deb` and `.rpm` replace the old package. Then click
**Update hooks…** next to each agent in **Set up agents…** (Codex asks for its trust again).

## Keys

On Linux, from anywhere: **Ctrl+Alt+Space** opens the island and hands it the keyboard,
**Ctrl+Alt+Y** / **Ctrl+Alt+N** allow or deny the card in front, **Ctrl+Alt+J** / **Ctrl+Alt+K**
put the next or previous session in front, and holding **Ctrl+Alt+V** talks to Zeca
([all of them](guide/island.md#getting-to-a-session)).

## Language

Vults speaks English, Brazilian Portuguese, Spanish and Simplified Chinese. It follows your
desktop's language; **Settings → General → Language** picks another for the island, Settings, the
tray and the notifications. What your agents say stays in their own words.

## Next

- [The island](guide/island.md): the flock, folded and open, getting to a session
- [Approving from the island](guide/approvals.md)
- [Activity](guide/activity.md)
- [Other agents](guide/other-agents.md)
- [The Nest](guide/nest.md): the whole flock, tasks, records and connections
- [Chatting with Zeca](guide/chat.md)
- [Zeca's memory](guide/memory.md)
- [Connectors](guide/connectors.md)
- [The phone](guide/phone.md)
