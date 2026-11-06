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
`alsa-lib`, `libpulse`, `gst-plugins-good` (WebKitGTK plays the island's sounds through GStreamer) and the
Vulkan loader (building also needs `cmake`, the Vulkan headers and `glslc`). On KDE Plasma,
Hyprland, Sway and other compositors with layer-shell, the island sits on the top edge like a panel;
on GNOME it is a regular always-on-top window. **Settings → General → Start with the desktop** opens
Vults when you log in.

### Windows development checkout

The Windows release installer remains a preview. This development checkout adds native input
regions, keyboard focus, global navigation shortcuts, tray opening, monitor selection and the
shared Room/Settings window. It also tracks same-user session processes, observes screen locking,
and reads and controls Windows system media sessions. Linux retains its existing desktop integrations.

Install these prerequisites together: Node.js 22 or a newer supported LTS, Rust for
`x86_64-pc-windows-msvc`, Visual Studio **Desktop development with C++** (MSVC x64/x86,
Windows SDK and CMake/Ninja tools), LLVM with `libclang.dll`, and Microsoft Edge WebView2 Runtime.
Set `LIBCLANG_PATH` to LLVM's `bin` directory if it is installed elsewhere. Visual Studio Insiders
is supported by the launcher; Ninja avoids its CMake instance-discovery issues.

From the repository root:

```bat
scripts\dev-windows.cmd --check
npm ci
scripts\dev-windows.cmd
```

The check reports missing tools before compiling. The launcher changes only its own environment.
If a previous build cached a different CMake generator, run `cargo clean -p whisper-rs-sys` once.

Windows shortcuts use non-printing keys to preserve AltGr: **Ctrl+Alt+Space** opens the island;
**Ctrl+Alt+Left/Right** changes the session; **Ctrl+Alt+F1/F2/F3** selects Mini/Quick/Room;
**Ctrl+Alt+F8/F9** allows/denies the visible card; **Ctrl+Alt+F10** cancels Chat voice. A key already
owned by another application is left unbound. Settings shows only successful registrations.

Global dictation/insertion, voice commands, the native MCP question bridge, desktop notifications, meeting capture, live voice
conversation and raising an existing terminal window are still Linux-only. On Windows,
the terminal action falls back to opening the project folder. Chat microphone capture and local
transcription are separate from global dictation; microphone/model acceptance still requires a
real-device test. Renderer scale emulation does not establish native per-monitor DPI acceptance.

Now Playing runs only with its setting enabled. Play/pause and previous/next target the displayed
player; duplicate application sessions are refused rather than controlling another window. Per-player
volume is not provided by the Windows media-session API. Screen locking is observed every 500 ms;
an unreadable state never counts as an unlock. Process liveness uses the creation time to detect PID
reuse and never closes a session whose owner was not previously observed. Hermes conversations keep
their explicit lifecycle because a gateway worker can end while its conversation remains open.
The Windows backends use Microsoft's [media-session API](https://learn.microsoft.com/en-us/uwp/api/windows.media.control.globalsystemmediatransportcontrolssessionmanager)
and [session lock flags](https://learn.microsoft.com/en-us/windows/win32/api/wtsapi32/ns-wtsapi32-wtsinfoex_level1_w).

The remaining Windows implementation order and acceptance criteria are:

| Order | Module | Required acceptance before calling it supported |
| --- | --- | --- |
| 1 | Sessions, hooks and metrics | Native exe/npm launchers, cmd/PowerShell wrappers, Unicode and spaced paths, process exit/PID reuse, native MCP question ownership, unknown versus zero, cumulative tokens versus current context, actual versus estimated cost, elapsed-time semantics and stale/reset reports. Subscription reads retain their separate network switches. |
| 2 | Notifications | Registered application identity for installed and development builds; per-session replacement, withdrawal, Open-only activation, do not disturb, setting off, shutdown and notification-service failure. |
| 3 | Focus, clipboard and text insertion | Pin the exact foreground window/process and focus generation; Unicode/IME, clipboard restoration, user clipboard changes, cancellation, protected/elevated targets and focus changes must never type into another window. |
| 4 | Global dictation and voice commands | Native press/release/cancel shortcuts, device selection, local transcription, the insertion checks above, lock interruption and consent revocation. Keep the existing command approval and audit rules. |
| 5 | Audio and meetings | WASAPI microphone and output loopback, endpoint identity/change/unplug, cancellation on lock or page loss, bounded buffers, recoverable recording and local transcription. Live conversation additionally needs echo cancellation, interruption and route-change tests. |
| 6 | Desktop completion | Raise the exact existing terminal, installed application inventory, per-application volume and temporary music ducking/restoration; validate native multi-monitor 100/125/150/175/200% DPI, reconnect, sleep/wake, autostart and installed upgrades. |

Every phase retains the Linux backend and its regression checks. Automated fixture tests and renderer
scale tests do not replace a native device, compositor, installer or multi-monitor acceptance test.

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
| Hermes Agent | `~/.hermes/plugins/vults/` on Linux; `%LOCALAPPDATA%\hermes\plugins\vults\` on Windows (`HERMES_HOME` overrides either) | Enable with `hermes plugins enable vults`; open a new CLI session ([details](guide/other-agents.md#hermes-agent)) |
| Qwen Code | `~/.qwen/settings.json` | Restart Qwen Code ([its cards](guide/approvals.md#qwen-code)) |
| Copilot CLI | `~/.copilot/hooks/vults.json`, a file of ours | Restart Copilot CLI ([its cards](guide/approvals.md#copilot-cli)) |
| Cursor | `~/.cursor/hooks.json` | Restart Cursor ([it asks in its own prompt](guide/other-agents.md#cursor)) |
| Factory Droid | `~/.factory/settings.json` | Restart Droid ([it asks in its own prompt](guide/other-agents.md#factory-droid)) |
| Kimi Code | `~/.kimi-code/config.toml` | Restart Kimi ([it asks in its own prompt](guide/other-agents.md#kimi-code)) |

With `CLAUDE_CONFIG_DIR`, `CODEX_HOME`, `QWEN_HOME`, `COPILOT_HOME`, `FACTORY_HOME_OVERRIDE`,
`KIMI_CODE_HOME` or `XDG_CONFIG_HOME` set, the file in that folder is used
([details](reference/settings.md#agent-configs-vults-edits)). Antigravity's is always
`~/.gemini/config/hooks.json`: `GEMINI_CLI_HOME` does not move it.

Other tools (Pi…) can report too, with a few lines in their own config: see
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
- [The Nest](guide/nest.md): the whole flock, chat, records and connections
- [Chatting with Zeca](guide/chat.md)
- [Zeca's memory](guide/memory.md)
- [Connectors](guide/connectors.md)
- [The phone](guide/phone.md)
