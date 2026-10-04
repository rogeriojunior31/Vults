<p align="center">
  <img src="docs/assets/zeca.png" width="128" height="128" alt="Zeca, an 8-bit black vulture">
</p>

<h1 align="center">Vultures AI</h1>

<p align="center">
  <strong>Your coding agents, alive on your desktop. And a vulture who helps.</strong>
</p>

<p align="center">
  <a href="https://github.com/rogeriojunior31/vultures-ai/actions/workflows/ci.yml"><img src="https://github.com/rogeriojunior31/vultures-ai/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue" alt="MIT license"></a>
  <img src="https://img.shields.io/badge/platform-Linux-informational" alt="Linux">
  <img src="https://img.shields.io/badge/built%20with-Rust%20%2B%20Tauri-orange" alt="Built with Rust and Tauri">
</p>

<p align="center">
  <img src="docs/assets/island-compact-flock.png" alt="The folded island: Zeca and four vults on the wire">
</p>

Vultures AI is a desktop app for people who run several coding agents at once. It has two parts:
the flock is the tool, and Zeca is the companion on top of it.

- **The flock.** Every Claude Code, Codex or Gemini CLI session becomes an 8-bit vulture on your
  desktop, a vult, doing what its session does. You see at a glance who is working, who finished,
  who failed and who needs you; you approve, answer and jump to the right terminal without
  hunting for it. Today the flock lives on an island at the top of the screen; the tray, a corner
  widget and desktop notifications come next, so you choose how present it is.
- **Zeca, the companion.** A black vulture who chats with you through the CLIs you already use,
  listens when you hold a key and speak, takes the files you drop on him, and over time becomes a
  personal agent that can act for you, always asking before anything runs. He is optional: a switch
  to turn him off and keep only the flock comes in 0.2.

> **Status: early development.** Linux first (KDE Plasma and other layer-shell compositors); Windows
> and macOS later. Build from source until the first release: see [Getting started](docs/getting-started.md).

<p align="center">
  <img src="docs/assets/island-approval.png" width="640" alt="A permission card: the whole command, Deny, Allow and Always allow">
</p>

## What it does

**The flock**

- **A bird per session.** It reads, pecks the wire while editing, tugs at it while a command runs,
  flies off when the agent goes to the web, and spreads its wings when a permission waits for you.
- **Approvals in one place.** The card shows the whole command, file or diff; **Allow**, **Deny**, or
  **Ctrl+Alt+Y** / **Ctrl+Alt+N** from any window. Nothing is approved without you, except what
  you allowed yourself with **Always**, for that exact command in that project.
- **Live diffs.** Each edit shows its lines added and removed; a click shows the diff.
- **Jump to the terminal.** One click focuses the session's tmux, kitty, wezterm or herdr pane, and on
  KDE Plasma raises its window (a terminal, VS Code or Cursor).
- **News from GitHub.** Failed checks, approvals and review requests, through the `gh` you already use.
- **Plan usage at a glance.** How much of your Claude and Codex limits you have spent, from the CLIs.

**Zeca**

- **Chat.** Through the `claude` or `codex` CLI you are logged into (every command and edit asks
  first), or an API key, or a local model with Ollama or LM Studio. Drop files on him.
- **Voice.** Hold **Ctrl+Alt+V** and speak: whisper.cpp transcribes it on your computer, and you
  read the text before it is sent.
- **Now playing**, if you want it: the song on the island, and the flock dances to it.

<p align="center">
  <img src="docs/assets/island-busy-flock.png" width="640" alt="The open island: the session in front and the flock list">
  <img src="docs/assets/island-chat-permission.png" width="640" alt="The chat: Zeca asks before running a command">
</p>

## Safe by design

The hook never blocks your agent: if the app is closed or slow, it exits at once and the agent asks in
its terminal. Agent configs change only after a dated backup and a diff you approve, and other tools'
hooks are kept. Secrets live only in the OS keyring, and there is no telemetry. See
[Safety](docs/safety.md).

## Where it is going

The first release is **0.1.0**. After it, one theme per version, Linux first:

- **0.2 Experience**: more ways to keep the flock around: the tray, a corner widget, desktop
  notifications, a session in focus, presence modes from *Island* to *Quiet* and *Paused*.
- **0.3 Control**: the birds become handles (quick actions), a command palette, waiting cards
  that call louder over time, a summary of what happened while you were away.
- **0.4 Platform**: local history, your projects with their branch, pull request and checks, cost
  per session.
- **0.5 Operations**: the full app, policies you write and approve, starting agents from here.

Zeca stays optional all the way: the flock works without him. Why we chose this, and what we
will not do, is in the [decision records](docs/adr/README.md).

## Documentation

- [Getting started](docs/getting-started.md)
- [The island](docs/guide/island.md) · [Approvals](docs/guide/approvals.md) · [Chat](docs/guide/chat.md) · [Connectors](docs/guide/connectors.md) · [Other agents](docs/guide/other-agents.md)
- [Settings and files](docs/reference/settings.md) · [Hook protocol](docs/reference/protocol.md)
- [Architecture](docs/architecture.md) · [Decisions](docs/adr/README.md) · [Animations](docs/ANIMATIONS.md) · [Adding a connector](docs/contributing/connectors.md)

Built with Rust and Tauri. Not to be confused with *Vulture*, the Python dead-code finder.

## License

MIT, see [LICENSE](LICENSE). Third-party attributions in [NOTICE](NOTICE).
