<p align="center">
  <img src="docs/assets/zeca.png" width="128" height="128" alt="Zeca, an 8-bit black vulture">
</p>

<h1 align="center">Vultures AI</h1>

<p align="center">
  <strong>A friendly flock watching your coding agents.</strong>
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

Zeca, an 8-bit black vulture, perches on a wire at the top of your screen with his flock, the vults:
one bird per Claude Code, Codex or Gemini CLI session, each doing what its session does. Approve or
deny permissions with a click, jump to a session's terminal, chat with Zeca through the CLIs you
already use, drop files on him, and follow your GitHub pull requests and checks.

> **Status: early development.** Linux first (KDE Plasma and other layer-shell compositors); Windows
> and macOS later. Build from source until the first release: see [Getting started](docs/getting-started.md).

<p align="center">
  <img src="docs/assets/island-approval.png" width="640" alt="A permission card: the whole command, Deny, Allow and Always allow">
</p>

## What it does

- **A bird per session.** Zeca reads, pecks the wire while editing, tugs at it while a command runs,
  flies off when the agent goes to the web, and spreads his wings when a permission waits for you.
- **Approvals from anywhere.** The card shows the whole command, file or diff; **Allow**, **Deny**, or
  **Ctrl+Alt+Y** / **Ctrl+Alt+N** from any window. Nothing is ever approved without you.
- **Jump to the terminal.** One click brings the session's tmux, kitty, wezterm or herdr pane forward,
  or its VS Code or Cursor window.
- **Chat with Zeca.** Through the `claude` or `codex` CLI you are logged into (every command and edit
  asks first), or an API key, or a local model with Ollama or LM Studio. Drop files on him, or hold
  **Ctrl+Alt+V** and speak: whisper.cpp transcribes it on your computer.
- **News from GitHub.** Failed checks, approvals and review requests, through the `gh` you already use.
- **Plan usage at a glance.** How much of your Claude and Codex limits you have spent, from the CLIs.
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

## Documentation

- [Getting started](docs/getting-started.md)
- [The island](docs/guide/island.md) · [Approvals](docs/guide/approvals.md) · [Chat](docs/guide/chat.md) · [Connectors](docs/guide/connectors.md) · [Other agents](docs/guide/other-agents.md)
- [Settings and files](docs/reference/settings.md) · [Hook protocol](docs/reference/protocol.md)
- [Architecture](docs/architecture.md) · [Animations](docs/ANIMATIONS.md) · [Adding a connector](docs/contributing/connectors.md)

Built with Rust and Tauri. Not to be confused with *Vulture*, the Python dead-code finder.

## License

MIT, see [LICENSE](LICENSE). Third-party attributions in [NOTICE](NOTICE).
