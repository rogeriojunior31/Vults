<p align="center">
  <img src="docs/assets/zeca.png" width="128" height="128" alt="Zeca, an 8-bit black vulture">
</p>

<h1 align="center">Vults</h1>

<p align="center">
  <strong>Your coding agents on one wire. Dictation, chat and GitHub within reach.</strong>
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

Watch your coding agents without switching terminals. Each session becomes an animated 8-bit
vulture: see who is working, who finished and who needs you. Dictate into other apps, talk to
**Zeca**, and follow your GitHub work from the same desktop companion. Zeca is optional.

[Features](#features) · [Screenshots](#screenshots) · [Installation](#installation) ·
[Quick start](#quick-start) · [Support](#support) · [Documentation](#documentation)

<p align="center">
  <a href="docs/assets/island-flock.png"><img src="docs/assets/island-flock.png" width="760" alt="Five animated vultures represent coding sessions; badges highlight a failed session and one waiting for attention"></a>
</p>

> **Release status:** the latest published version is **0.1.8**. This checkout includes
> **Unreleased** work targeting **0.1.9**, subject to desktop acceptance and release checks.
> The feature map describes this checkout; consult the [changelog](CHANGELOG.md) for shipped changes.
> Screenshots below are existing captures from earlier builds; navigation and spacing have since changed.

## Features

| When you want to… | What Vults provides | Guide |
| --- | --- | --- |
| Follow several agents at once | A bird per session, project species, working/finished/failed/waiting states, session details and live diffs | [Flock](docs/guide/island.md) |
| Handle interruptions from one place | Permission and question cards where supported, keyboard answers, exact Always rules, reviewed policies and an audit trail | [Approvals](docs/guide/approvals.md) |
| Return to the right terminal | Jump to a session's pane or window; continue supported sessions in Chat with an owned Stop action | [Sessions and Chat](docs/guide/chat.md#continuing-a-session) |
| Speak instead of typing in another app | Local global dictation, copy by default, optional supported insertion routes, cancel and copy-last shortcuts | [Dictation](docs/guide/dictation.md#global-dictation-first-slice) |
| Make dictation fit your vocabulary | Dictionary, snippets, spoken code, per-app choices, optional model cleanup and translation styles | [Your words](docs/guide/dictation.md#your-words-processing) |
| Control the flock and playback by voice | Separate command key, local phrase matching and opt-in selected-player playback, computer output volume/mute and installed-app opening; natural AI interpretation remains planned | [Voice commands](docs/guide/dictation.md#voice-commands-development-status) |
| Find and manage what you dictated | Searchable local history, retention controls, usage statistics, microphone/model setup and diagnostics | [Dictation history](docs/guide/dictation.md#dictation-history) |
| Ask for help without leaving your desktop | Zeca Chat through an existing Claude Code/Codex CLI login, an API or a local model; Markdown, files, saved conversations and access choices | [Chat](docs/guide/chat.md) |
| Give a specialist a consistent role | Create or copy profiles with instructions, species, accessories and pinned skills; Claude Code or Codex specialist conversations inside Chat | [Agent profiles](docs/guide/chat.md#agent-profiles) |
| Keep useful context across conversations | Optional local journal, recall and proposed facts that you review before keeping | [Memory](docs/guide/memory.md) |
| Follow and act on GitHub work | Repository connections through your gh login, PRs, checks, reviews and explicitly confirmed write actions; scoped access for Developer/Reviewer profiles | [GitHub](docs/guide/connectors.md) |
| Turn a meeting into searchable notes | Consented microphone/system-audio capture, local transcription, saved text, speaker labels, search and Markdown/SRT/VTT export; optional model summaries | [Meetings](docs/guide/dictation.md#meeting-recording) |
| Talk with Zeca aloud | Separate voice conversation setup, manual turns and optional continuous microphone routes, with explicit API consent and budget | [Voice conversation](docs/guide/dictation.md#voice-conversation-settings) |
| Review your week and spending | Project activity, available plan usage, costs, budgets and a shareable recap with project-name hiding | [Activity](docs/guide/activity.md) |
| Fit Vults around your work | Mini, Quick, Room and full screen; light/dark themes, four languages, notifications, optional media controls and Zeca on/off | [The Nest](docs/guide/nest.md) · [Settings](docs/reference/settings.md) |
| Answer away from the desktop | Paired-phone approval protocol and relay, in preview; the Android companion remains unfinished | [Phone preview](docs/guide/phone.md) |

## Screenshots

Click a capture to open its original resolution. These illustrate individual workflows rather than
the final 0.1.9 layout. Fresh release captures are still pending.

<details>
<summary>See the flock and inspect a session</summary>

[![Room view with four vultures, session activity and a selected file diff](docs/assets/nest-room.png)](docs/assets/nest-room.png)

[Guide: the Nest](docs/guide/nest.md). Earlier Room layout, including its former Tasks tab.

</details>

<details>
<summary>Review a Chat permission in the light theme</summary>

<p align="center"><a href="docs/assets/nest-chat-approval.png"><img src="docs/assets/nest-chat-approval.png" width="480" alt="Light-theme Chat permission card showing git status, Deny and Allow, above the message composer"></a></p>

[Guide: Chat access](docs/guide/chat.md#what-it-may-do). Earlier compact Chat layout.

</details>

<details>
<summary>Understand dictation usage</summary>

[![Dictation statistics with word count, estimated time saved, speaking rate and a daily chart](docs/assets/dictation-statistics.png)](docs/assets/dictation-statistics.png)

[Guide: dictation](docs/guide/dictation.md). Statistics now live in the Dictation workspace.

</details>

## Installation

**Linux first.** Download the `.deb` or `.rpm` from the
[latest release](https://github.com/rogeriojunior31/Vults/releases/latest).
Verify the archive with `sha256sum -c SHA256SUMS --ignore-missing` in your download folder.
KDE Plasma and other layer-shell compositors host the island at the screen edge; GNOME uses a
regular always-on-top window. Windows builds are previews; macOS work comes later.

Arch source packages are in `packaging/aur/` and are not yet published to the AUR.
For dependencies, package verification and installation instructions, see [Getting started](docs/getting-started.md).

To run the current development checkout, after installing the documented Linux dependencies:

```sh
git clone https://github.com/rogeriojunior31/Vults
cd Vults
npm install
npm run tauri dev
```

Coding-agent CLIs are installed separately. GitHub uses `gh`; local speech requires a checked model
download. API-backed chat, cleanup and voice features can incur provider charges.

## Quick start

1. **Connect an agent.** Open **Set up agents…** from the tray. Review its hook diff, then approve
   the write. Vults keeps a dated backup and preserves third-party hooks. Codex also requires
   trusting the installed hooks in its CLI.
2. **Start working.** Run your agent as usual. Select its bird to inspect activity, changes and any
   supported cards. Start in **Mini**; open **Quick** for a short interaction or **Room** for the Nest.
3. **Enable only what you need.** Set up local speech under **Dictation**, choose a Chat provider,
   or add a repository under **Connections**. The Nest's tabs are **Flock**, **Chat**, **Dictation**,
   **Connections** and **Activity**; Chat is hidden when Zeca is off.

| Default shortcut on Linux | Action |
| --- | --- |
| Ctrl+Alt+Space | Open the island with keyboard focus |
| Ctrl+Alt+Y / Ctrl+Alt+N | Allow / deny the card in front |
| Hold Ctrl+Alt+V | Global dictation into copied text or an explicitly enabled destination |
| Ctrl+Alt+X | Cancel dictation or Chat voice |
| Ctrl+Alt+B | Copy the last successful dictation again |
| Hold Ctrl+Alt+Z | Voice command mode, separate from dictation |
| Ctrl+K, while Vults is focused | Open the command palette |

The Chat microphone records into Chat; **Ctrl+Alt+V is global dictation**. Insertion support depends
on the desktop and destination, and never submits a message automatically.

## Support

Built-in integrations: **Claude Code, Codex, Antigravity, OpenCode, Qwen Code, Copilot CLI, Cursor,
Factory Droid and Kimi Code**. Watching a session, answering its cards and driving it are different
capabilities: some agents keep approvals in their own terminal. See the
[per-agent capability table](docs/guide/other-agents.md#what-each-agent-can-do).

Specialist profiles currently run through Claude Code or Codex. Voice conversation uses its own
API setup and budget; Codex voice consultation is unavailable. GitHub workflows, voice routes and
new UI still have native acceptance gates in the [active roadmap](docs/dev/ROADMAP.md).

### Safety and control

- **Hooks fail open.** On failure, the hook exits successfully without an answer, leaving the agent
  to ask in its own terminal.
- **You choose access.** Cards are answered by a human, an exact Always rule, an accepted policy or
  a signed paired-phone answer. Chat also respects the access you selected; destructive actions
  still ask and policy denies/asks still apply. Zeca never answers flock approvals.
- **Review config changes.** Agent config writes require a dated backup, a diff and your click.
- **Keep control of data.** No telemetry; secrets stay in the OS keyring. Network features require
  their own opt-in and use the network gate, budgets and local traces. Local transcription and
  optional provider processing are separate choices. See [Safety](docs/safety.md).

## Documentation

| Your goal | Start here |
| --- | --- |
| Install and connect an agent | [Getting started](docs/getting-started.md) · [Agent support](docs/guide/other-agents.md) |
| Understand desktop workflows | [Island](docs/guide/island.md) · [Nest](docs/guide/nest.md) · [Approvals](docs/guide/approvals.md) |
| Set up speech or meetings | [Dictation and voice](docs/guide/dictation.md) |
| Work with Zeca and specialists | [Chat](docs/guide/chat.md) · [Memory](docs/guide/memory.md) |
| Connect GitHub or review activity | [Connections](docs/guide/connectors.md) · [Activity](docs/guide/activity.md) |
| Configure or troubleshoot | [Settings and files](docs/reference/settings.md) · [Safety](docs/safety.md) |
| Follow development | [Changelog](CHANGELOG.md) · [Roadmap](docs/dev/ROADMAP.md) · [Architecture](docs/architecture.md) |

[Read the published documentation online](https://rogeriojunior31.github.io/en/docs/vults/).
It follows releases; guides in this checkout include development changes.

## Contributing

Bug reports, agent integrations, connectors and documentation improvements are welcome. See
[CONTRIBUTING.md](CONTRIBUTING.md), the [Code of Conduct](CODE_OF_CONDUCT.md) and
[SECURITY.md](SECURITY.md) for private vulnerability reports. Built with Rust and Tauri.

## License

MIT. See [LICENSE](LICENSE) and third-party attributions in [NOTICE](NOTICE).
