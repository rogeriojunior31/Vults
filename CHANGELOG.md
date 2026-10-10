# Changelog

What changed in each release, newest first. Each release's notes on GitHub come from its section here, so a change a user would notice adds a line under **Unreleased** in the same PR.

## Unreleased

### New

- **Open terminal beyond KDE:** the session's window comes forward in any X11 session (Xfce, Cinnamon, MATE, i3…), on its workspace, and for XWayland windows elsewhere; with several windows, the one titled after the project wins. On KDE a minimized window comes back.
- **OpenCode from Settings:** **Settings → Agents → OpenCode** installs the plugin that brings its sessions to the island, with the same diff, backup and click as the hooks. A plugin file of yours with that name is never overwritten.
- **Approve OpenCode from the island:** OpenCode's permissions get a card, like Claude Code's and Codex's. OpenCode's own prompt stays up meanwhile; the first answer wins. **Allow** is OpenCode's *Allow once*, and **Always allow** stays a Vults rule. A plugin from the previous version shows **Update available**.
- **OpenCode's questions on the island:** when OpenCode asks you something with choices, the island opens on a question card, as for Claude Code. OpenCode's own prompt stays up; the first answer wins.
- **Qwen Code:** **Settings → Agents → Qwen Code** installs its hooks in `~/.qwen/settings.json`, and its permissions get a card on the island, like Claude Code's. Its questions stay in its terminal.
- **Told once:** desktop notifications come only in *Panel*, where the island is out of sight. In *Island* and *Quiet* the island already shows it all (a card opens it with its sound), so nothing is repeated on the desktop, not even after 20 seconds.
- **Open on hover:** with **Settings → General → Open on hover** on, resting the pointer on the island opens it without a click; opened that way it folds as soon as the pointer leaves, unless you clicked in it. Off by default.

### Fixes

- OpenCode's plugin backup goes in `~/.config/opencode/`, no longer next to the plugin in `plugins/`.
- kitty's tab is focused again: the app talks to kitty's remote control socket (`listen_on unix:…` in `kitty.conf`).
- Open terminal on KDE says when it found no window, and its script lives in your private runtime folder.

## 0.1.6 (2026-10-09)

**Vultures AI is now Vults.** This is the first release under the new name.

### Upgrading from Vultures AI (0.1.5 and older)

- The `.deb` and `.rpm` replace the old `vultures-ai` package instead of installing beside it.
- On its first start Vults moves your settings, data and API keys to the new names, and leaves a link where the old hook was, so your agents keep reporting.
- If Vultures AI started at login, the old login entry is swapped for the new one.
- Open **Set up agents…** and click **Update hooks…** next to each agent: the old entries are replaced, not doubled. Codex asks for its trust again.

### New

- **Arch Linux:** build the package with `makepkg` from `packaging/aur/vults` (this release) or `packaging/aur/vults-git` (follows `main`). See [Getting started](docs/getting-started.md).

### Fixes

- A Claude chat turn no longer picks up a project's settings (#154).
- No *Always* for a target that was cut short, and other hardening from a review (#155).
- Zeca's local voice (Kokoro, espeak-ng) is gone.

## 0.1.5 (2026-10-07)

### Fixes

- **The island no longer opens half painted.** WebKitGTK 2.54 repaints a transparent window without compositing only where something moves, so the island showed Zeca and stray text with no card behind them. WebKit's compositing now stays on. (#126)
- Right-clicking an empty spot of the island no longer shows the web view's own menu. (#127)
- Back from away, the summary is no longer hidden behind the chat. (#127)
- Settings → Projects: long folder paths wrap and Forget stays in its card; Settings → Agents reads the agents' files again when you come back to the window. (#127)

## 0.1.4 (2026-10-07)

### Zeca speaks

- Zeca can read his chat replies aloud, off by default, with one voice for English and one for Portuguese. (#123) Removed again in 0.1.6.

### Control

- Quick actions: right-click a bird, a flock row or the session in front for its terminal, folder, activity, last diff and its file. (#115)
- Per-project mute, pin and hide; undo in Settings → Projects. A card always shows. (#119)
- A quiet bird: a session silent for 5 minutes is flagged, at 15 minutes loudly; snooze, keep going or dismiss. It only informs. (#120)
- A waiting card calls again at 45, 75 and 105 s; do not disturb from Settings keeps only a card's own sound. (#121)
- While you were away: a short summary after you unlock the screen or leave *Paused*. (#122)

### Agents

- Antigravity (`agy`) through its hooks file, installed with the same backup, diff and click; recipes for OpenCode, Pi and Cursor in the guide. (#116)

### Smaller

- Tray icons drawn natively at 22, 24 and 32 px. (#114)

## 0.1.3 (2026-10-06)

### How present the flock is

- **Presence presets** (Settings or the tray menu, no restart): *Island*, *Panel*, *Quiet* and *Paused*. (#103)
- **Corner widget**, off by default: up to three birds and the counts in the corner you pick; a click opens the island, on the card if one waits. (#111)
- In *Panel* mode the open island folds away when you go to Settings. (#109)
- Tray badges you can read at panel size: working, needs you, done, failed. (#104)

### Zeca

- Zeca can be turned off (Settings → Flock): the flock, cards, notifications and connectors keep working; no chat, mic or talk key. (#105)
- Right-click Zeca for his looks, with a live preview. (#107)

### Voice

- While you speak, the words show dimmed as they are heard (on the GPU); the final text replaces them. (#110)
- Tap-to-talk stops about 1.5 s after your last word. (#106)
- The chat input keeps its height after a transcription. (#113)

### Fixes

- Nothing that finished while *Paused* pops up late when you resume. (#113)

## 0.1.2 (2026-10-06)

### The flock, from more places

- **Panel mode**: Zeca lives in the tray icon, one look per state; a click, or a card, opens the island by the panel. The icon never answers a card. (#96–#99)
- **Desktop notifications**: a session finished or failed, and a card waiting. The only action is **Open**; nothing can be allowed from a notification. (#101)
- **Shortcuts**: Ctrl+Alt+J / Ctrl+Alt+K move to the next / previous session, Ctrl+Alt+Space opens the island. (#88)

### Underneath

- Who needs you, who is in front and how each card ended are decided in one place (the core), so every surface agrees. (#82, #84, #87)
- A card whose agent quit still says "Answered in the terminal"; "Answered" reads green. (#90)

### Voice

- Tap the mic and talk: it stops by itself after 1.5 s of silence (Silero VAD), and trims the silence around your words. (#94, #95)

### Agents

- Your own Claude Code status line is kept: ours runs it and prints its output, and removing our hooks puts it back. (#93)

## 0.1.1 (2026-10-05)

- Zeca polished: a lit edge along the back, a folded wing with its shadow, and a breath and a shuffle of the feet in his idle; every species inherits the light from its own body color. (#78)
- Rule 2 (a permission is answered only by a human) pinned on every input; settings read their version. (#69)
- A volume for the sounds (#73); the hello bounce in Zeca's greeting (#75).
- Packages depend on `gst-plugins-good`, so the island's sounds play. (#77)
- Release packages named as GitHub stores them before writing `SHA256SUMS`. (#67)

## 0.1.0 (2026-10-05)

The first release. Linux first (KDE Plasma and other layer-shell compositors).

- Every Claude Code, Codex or Gemini CLI session is a vulture on an island at the top of the screen; it shows what its session does, step by step.
- Permission and question cards: the whole command, file or diff; Allow, Deny, Always, or Ctrl+Alt+Y / Ctrl+Alt+N. Nothing is allowed without you.
- Live diffs, jump to the terminal (tmux, kitty, wezterm, herdr; on KDE Plasma the window is raised), GitHub news through `gh`, and plan usage for Claude and Codex.
- Zeca: chat through the `claude` or `codex` CLI, an API key or a local model; voice with whisper.cpp on your computer; seasonal looks and, if you want it, what is playing.
- The hook never blocks your agent. Agent configs change only after a dated backup and a diff you approve. Secrets live in the OS keyring. No telemetry.
