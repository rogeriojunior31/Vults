<p align="center">
  <img src="docs/assets/zeca.png" width="128" height="128" alt="Zeca, an 8-bit black vulture">
</p>

<h1 align="center">Vults</h1>

<p align="center">
  <strong>Your coding agents, alive on your desktop. And a vulture who helps.</strong>
</p>

<p align="center">
  <a href="https://github.com/rogeriojunior31/Vults/actions/workflows/ci.yml"><img src="https://github.com/rogeriojunior31/Vults/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="https://github.com/rogeriojunior31/Vults/releases/latest"><img src="https://img.shields.io/github/v/release/rogeriojunior31/Vults" alt="Latest release"></a>
  <a href="https://rogeriojunior31.github.io/en/docs/vults/"><img src="https://img.shields.io/badge/docs-read%20online-f2984a" alt="Documentation"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue" alt="MIT license"></a>
  <img src="https://img.shields.io/badge/platform-Linux-informational" alt="Linux">
  <img src="https://img.shields.io/badge/built%20with-Rust%20%2B%20Tauri-orange" alt="Built with Rust and Tauri">
</p>

<p align="center"><b>English</b> · <a href="README.pt-br.md">Português</a></p> <!-- check-english:allow (language name) -->

<p align="center">
  <img src="docs/assets/island-flock.png" width="500" alt="The open island at rest: vults of different species on one wire, each with its project's name, and a chip for each session that needs a look">
</p>

Vults is for people who run several coding agents at once. Every **Claude Code, Codex, Antigravity,
OpenCode, Qwen Code, Copilot CLI, Cursor and Factory Droid** session becomes an 8-bit vulture at the top of your screen,
doing what its session does. You see who is working, who finished and who needs you, and you
answer from one place. **Zeca**, a black vulture, sits on top as a companion you can chat with; he
is optional.

> **0.1.7, early.** Linux first (KDE Plasma and other layer-shell compositors); Windows and macOS
> later. Install the `.deb` or `.rpm` from the
> [latest release](https://github.com/rogeriojunior31/Vults/releases/latest), the Arch package
> with `makepkg`, or build from source: [Getting started](docs/getting-started.md).

## One surface, three sizes

The size switch at the top changes how much you see, in the same place:

- **Mini**: a thin pill with the flock on its wire. A colored badge marks who finished, failed or
  waits on you, and the edge glows amber while someone does.
- **Quick**: the island open. At rest, every session is on the wire with its name, and a chip
  marks each one that needs a look. Click a bird to open its session; **‹ Flock** goes back.
- **Room** and **full screen**: the Nest, Vults' own window, in the island's place.

<p align="center">
  <img src="docs/assets/island-compact-flock.png" width="440" alt="Mini: a thin pill with five vults on its wire and their badges">
</p>
<p align="center">
  <img src="docs/assets/island-quick.png" width="880" alt="Quick: the flock at rest with each session's name and the need chips, and a session opened from it with Flock to go back">
</p>

Quick has five tabs: **Flock**, **Chat** (Zeca, with a paperclip for files and the microphone),
**Activity** (your week), **Connections** (GitHub's pull requests and checks) and the gear (the
island's settings, and the rest of Settings in the Nest).

<p align="center">
  <img src="docs/assets/island-tabs.png" width="880" alt="Quick's other tabs: the chat, this week's activity, GitHub's rows in Connections, and the gear's settings">
</p>

**Room** opens the Nest in the island's place: the flock across the window over a card per session,
the one picked below with its changes, activity and context. Pull its edges to any size.

<p align="center">
  <img src="docs/assets/nest-room.png" width="880" alt="Room: the Nest's Flock, four vults over their session cards, and the picked session's diff below">
</p>

## The flock

- **A bird per session.** It reads, pecks the wire while editing, tugs at it while a command runs,
  and spreads its wings when something waits for you. Each project keeps its own species.
- **Approvals in one place.** The card shows the whole command, file or diff: **Allow**, **Deny**,
  or the keyboard from any window. Nothing is approved without you, except the exact commands you
  allowed with **Always** and the policies you wrote and accepted (a destructive action always
  asks).
- **Jump to the terminal.** One click focuses the session's tmux, kitty, wezterm or herdr pane, or
  raises its window.
- **Around it:** live diffs, GitHub checks and reviews through your `gh`, plan usage for Claude and
  Codex, a weekly recap of your work with the agents, and, in preview, answering cards from a
  paired phone.

<p align="center">
  <img src="docs/assets/island-approval.png" width="500" alt="A permission card: the whole command, Deny, Allow and Always allow">
</p>

## Zeca

- **Chat** through the `claude` or `codex` you are already logged into, with your own setup (every
  command and edit still asks first), or an API key, or a local model. Drop files on him, or hold
  **Ctrl+Alt+V** and speak: the audio is transcribed on your computer.
- **The Nest**, his own window: the conversation in full Markdown, your saved conversations, and
  (next release) his memory and your projects with their branch, pull requests and checks.
- **Memory** (next release), off until you turn it on: a local journal of your days, recall of earlier ones in
  front of each message, and facts he proposes and only keeps with your yes. Through Claude Code he
  gets the same tools from his own MCP server.

<p align="center">
  <img src="docs/assets/island-chat-permission.png" width="500" alt="The chat: Zeca asks before running a command">
</p>

## Safe by design

- **Your agents never wait on Vults.** If the app is closed or slow, the hook exits at once and the
  agent asks in its terminal.
- **Only you answer.** A card is answered by your click, a rule you made, or your paired phone with
  its biometrics. Zeca never answers a card.
- **Your files, with a backup and a diff.** An agent's config, or a project's `CLAUDE.md`, changes
  only after a dated backup and a diff you approve; other tools' hooks are kept.
- **Nothing leaves without your switch.** No telemetry. Every request the app makes on its own
  (GitHub's checks, the web, daily summaries, the phone's relay) is off until you turn it on (the
  speech detector's one-time download aside), has a
  daily budget and a local trace. Secrets live only in the OS keyring. See [Safety](docs/safety.md).

## Documentation

Online at [rogeriojunior31.github.io/en/docs/vults](https://rogeriojunior31.github.io/en/docs/vults/),
updated on each release, or here: [Getting started](docs/getting-started.md) ·
[The island](docs/guide/island.md) · [The Nest](docs/guide/nest.md) · [Approvals](docs/guide/approvals.md) ·
[Chat](docs/guide/chat.md) · [Zeca's memory](docs/guide/memory.md) ·
[Connectors](docs/guide/connectors.md) · [Activity](docs/guide/activity.md) ·
[Other agents](docs/guide/other-agents.md) · [The phone](docs/guide/phone.md) ·
[Settings and files](docs/reference/settings.md) · [Architecture](docs/architecture.md) ·
[Decisions](docs/adr/README.md).

**What's next:** 0.2.0 brings Operations: tasks Zeca hands to the right agent, policies you write
and approve, and a review of the work before it lands. What changed in each release is in the
[changelog](CHANGELOG.md).

## Contributing

Bug reports, agents, connectors and docs fixes are welcome: see [CONTRIBUTING.md](CONTRIBUTING.md)
and the [Code of Conduct](CODE_OF_CONDUCT.md). Security issues go privately, as in
[SECURITY.md](SECURITY.md). Built with Rust and Tauri; not to be confused with *Vulture*, the
Python dead-code finder.

## License

MIT, see [LICENSE](LICENSE). Third-party attributions in [NOTICE](NOTICE).
