# Changelog

What changed in each release, newest first. Each release's notes on GitHub come from its section here, so a change a user would notice adds a line under **Unreleased** in the same PR.

## Unreleased

### New

- **Zeca's memory:** **Settings → Chat → Memory**, off until you turn it on. A journal of each agent turn, each card's answer and the chat's lines, secrets redacted, kept 90 days on this computer; only what joins a conversation reaches its model.
- **He remembers in every conversation:** what he knows about you, what he learned and the project's facts join the conversation when it starts, fixed for its whole length; each message also carries the date and your sessions, and up to three passages from earlier days that answer it.
- **Only you write it:** `/remember <fact>` and `/forget <words>` in the chat; what Zeca proposes (his tool, or what he read in a finished conversation with **Suggest what to remember** on) waits in the Nest's new **Memory** tab, with its place and a diff, until you accept it. Every change can be undone. A fact said again in other words is never kept twice.
- **Roosts:** the Nest's **Roosts** tab groups your sessions by repository, with each worktree's branch, your pull requests and checks from the GitHub card, and what Zeca remembers of the project.
- **One project memory for Claude and Codex:** a roost whose `AGENTS.md` its `CLAUDE.md` does not import offers to add `@AGENTS.md`, with the diff first, a click, and a dated backup outside the repository.
- **Daily summary, sleep and recall by meaning** (each off until you turn it on): a past day summed up by Claude Code's smallest model; a daily sleep that proposes merges and leaves a debrief; and, with an Ollama on this computer, recall that finds summaries by meaning. Each is a request on your behalf, with its own budget and local trace.
- **Zeca's tools in Claude Code:** the chat's Claude Code gets his own MCP server (`flock_list`, `recall`, `journal_day`, `roost_memory`, `memory_propose`) through `--mcp-config`, never a change to your Claude config; each call your rules don't allow asks you with a card.
- **Delegate:** `/delegate <goal>` makes a task and starts it on the agent the router picks: the project's rule, the other agent when one's plan is above 85%, and, after 10 finished tasks, the one that finishes that kind of task best there. The card says why.
- **Continue a session:** `/continue <words>` in the chat gives your words to a Claude Code or Codex session in that folder that stopped in the last ten minutes, resumed here; one still open in its terminal is refused.
- **Cross review:** **Review with Codex** (or with Claude Code) has the other agent review a task's work against its criteria; what it finds goes back to the task's session to fix, up to three rounds.
- **Codex takes tasks too:** **Start with Codex** runs a task in its own clone and sandbox, its approvals as island cards; **Hand off to Codex** (or to Claude Code) passes a task's context pack to the other agent.
- **Start fresh with a handoff:** a task whose session has grown long can **Start fresh**: the old session writes a context pack (objective, constraints, done, in progress, decisions, files, failures, next), Vults adds what it knows for certain, and a new session starts from it alone.
- **Start a task:** **Start with Claude Code** runs Claude Code on a task in a clone and branch of its own, in Claude Code's sandbox, still asking before each command; nothing it does can touch your repository, and only its branch comes back. It shows in the flock, its cards come to the island, and **Stop** ends it. Two tasks run side by side.
- **Tasks:** `/task` in the chat keeps a task in one message: its goal, then a line for each criterion. The Nest's new **Tasks** tab lists them with their roost, who works on them, where they stand and what they cost; with the GitHub connector on, **Open an issue** makes one from a task through your `gh`.
- **Policies:** **Settings → Approvals → Policies** lets you write what to allow, deny or always ask, in a project folder or everywhere, see it as a diff and accept it with a click; it then answers those cards for you (allows for Claude Code's calls; denies and asks for every agent). A destructive action always asks, a deny beats an allow (and an older **Always allow**), an allow reaches only the folder the session started in and needs Vults to know what the action does, and every answer goes to the audit log with the policy that gave it. **Revoke** turns it off at once.
- **What each agent can do:** the Nest's new **Agents** tab shows, for each agent, whether its CLI is installed and its version, whether its hooks are in (and, for Codex, trusted), and what it can do with Vults: cards, questions, diffs, how a turn could be stopped, its sandbox, and whether Vults can start it.

### Fixes

- Going back to 0.1.7 now keeps a copy of your settings: the file is at version 18.
- With Zeca's memory off, nothing of it reaches a model or a window.

## 0.1.7 (2026-10-11)

### New

- **The Nest:** the arrow at the top of the island's chat opens Zeca's own window, the same conversation wide, with replies in full Markdown. A message sent in either one shows in both; only the island answers cards.
- **Conversations kept:** every chat is saved on this computer as it goes. The list at the top of the chat reopens one where it stopped, even after a restart, with its engine, folder, model and effort; rename or delete it there. The newest 200 are kept, and **Clear history** removes them.
- **Model and effort:** beside the chat's folder, pick the model and how hard it thinks, for Claude and for Codex. The choice is remembered per folder and per engine.
- **Your own Claude Code and Codex setup in the chat:** the chat reads your `CLAUDE.md`, skills, MCP servers, hooks and allow rules, as in your terminal; a project's own settings only in a folder you already trust in Claude Code. Cards still ask for every command and edit your rules don't allow. **Settings → Chat** turns it off.
- **Always in this chat:** a chat card can allow that exact command again until the conversation ends, the whole command and its whole input, never a look-alike. Never saved.
- **The API chat looks at the island:** asked "what is Codex doing?", it reads your sessions, the cards waiting, your plans' usage and your GitHub card through tools, and never answers a card. **Stop** ends a reply midway.
- **Web for Zeca:** with **Settings → Chat → Web for Zeca** on, the API chat can search (Brave Search, Tavily or your own SearXNG) and read pages. A page is read by a separate call with no tools, and only its answer reaches Zeca. At most 200 a day, each in the local trace.
- **Cards that warn:** once a chat conversation read content from outside (a page, `curl`, `gh`, an MCP tool), every card after it says so in red.
- **Answer cards from your phone (preview):** **Settings → Phone** pairs a phone by QR and sends it the cards waiting; approving needs biometrics on the phone, denying does not, and every answer is checked against the exact card, its time and a counter that never repeats. Messages go through a relay you run (a Docker image is in `packaging/relay`) or straight over your LAN or tailnet, sealed so the relay sees no command or project. Off until you turn it on. The Vults phone app is not out yet; until it is, the command-line test client in the docs plays the phone.

- **A Claude Code session's cost on its card:** a working session shows *≈$1.37*, what its tokens would cost at API prices, from Claude Code's own status line. A subscription is not charged it; the card's note says so.
- **Codex usage has its own switch:** **Settings → General → Codex usage** turns the island's Codex meters on or off. Each read asks OpenAI through the codex you logged into, so it goes through the app's network gate like the connectors, and is off on a new install; if you upgrade with Codex installed, it stays on. The docs' Safety page now lists every request Vults makes, and CI keeps that list true.
- **Open terminal beyond KDE:** the session's window comes forward in any X11 session (Xfce, Cinnamon, MATE, i3…), on its workspace, and for XWayland windows elsewhere; with several windows, the one titled after the project wins. On KDE a minimized window comes back.
- **OpenCode from Settings:** **Settings → Agents → OpenCode** installs the plugin that brings its sessions to the island, with the same diff, backup and click as the hooks. A plugin file of yours with that name is never overwritten.
- **Approve OpenCode from the island:** OpenCode's permissions get a card, like Claude Code's and Codex's. OpenCode's own prompt stays up meanwhile; the first answer wins. **Allow** is OpenCode's *Allow once*, and **Always allow** stays a Vults rule. A plugin from the previous version shows **Update available**.
- **OpenCode's questions on the island:** when OpenCode asks you something with choices, the island opens on a question card, as for Claude Code. OpenCode's own prompt stays up; the first answer wins.
- **Qwen Code:** **Settings → Agents → Qwen Code** installs its hooks in `~/.qwen/settings.json`, and its permissions get a card on the island, like Claude Code's. Its questions stay in its terminal.
- **Told once:** desktop notifications come only in *Panel*, where the island is out of sight. In *Island* and *Quiet* the island already shows it all (a card opens it with its sound), so nothing is repeated on the desktop, not even after 20 seconds.
- **Open on hover:** with **Settings → General → Open on hover** on, resting the pointer on the island opens it without a click; opened that way it folds as soon as the pointer leaves, unless you clicked in it. Off by default.
- **A project is a breed:** all the sessions of a project are one species, its flock, and the project keeps that species from one start to the next, so you learn which bird is which. The projects on the wire get different species while the pool has some left.
- **Choose a project's bird:** right-click a session, **This project's bird…**, and pick any vulture but the king, whatever the flock draws from; every session of the project is that species from then on. Also in **Settings → Projects**. **Automatic** gives it back to the draw.
- **A local activity history:** each finished agent turn is kept on this computer, as counts only (length, agent, project folder name, steps, commands, lines changed, your answers; never a prompt, command or path), for the weekly recap and the activity grid coming in **Settings → Activity**. `history` in the settings turns it off.
- **Settings → Activity:** a week's recap (time with your agents, turns, lines, commands, answers, failures, the busiest day) with **‹ ›** through the last 12 weeks, the year as a GitHub-style grid, and the history's switch and **Clear history…**.
- **Last week, on Monday:** from 08:00 on Monday the island tells last week in one line, once (time, turns, the top project), with **Open Activity** for the whole week. A waiting card comes first; *Paused* shows none.
- **Your GitHub calendar in Activity:** the grid's **GitHub** tab shows your contribution calendar, through the `gh` you are logged into, when the GitHub connector is on (asked when the tab opens, kept an hour).
- **Share your week:** **Save as image…** in **Settings → Activity** draws the week as a picture, with **Hide project names**, and saves it where you pick in the desktop's own dialog. Nothing is uploaded.
- **Keys on the island:** opened with **Ctrl+Alt+Space**, the island takes the keyboard until it folds: **↑ ↓** walk the sessions, **Enter** opens the terminal, **M** the quick actions, **Y / N** or a number answer the card on screen, **C** the chat, **Esc** gives the keys back. A line under the island says so. Opened with a click, it takes no keys.
- **One gate for what the app asks on its own:** a connector's checks (GitHub's today) go through the app's network gate: only while switched on, at most 3,000 in any 24 hours, each kept as a local line. One over that waits and says so in **Settings → Connectors**.
- **Zeca's turns traced:** each chat turn and each tool it asks to run is kept for 30 days in `vults.sqlite`: the model, the tokens, the prompt cache and the cost when the engine says it (Claude Code does). Never a prompt or a reply; nothing leaves the computer.
- **No card lost to its size:** a permission request with hundreds of long fields used to pass the size the app reads in one line, and its card never showed; the hook now cuts it down until it fits, and such a request is asked in the agent's own terminal, never on a card that could not show all of it.
- **Every answer on record:** each answer to a card, the flock's and Zeca's chat's (by you, an *Always* rule, or the clock), with the exact command it was about, and each agent config the installer writes go to an audit log in `vults.sqlite`, kept 90 days and never changed; tokens and secrets in commands are redacted first. Nothing leaves the computer; **Clear history** removes it too.
- **One local database:** the activity history moves from `history.jsonl` and `days.json` into `vults.sqlite` in the same folder, on first start; nothing is lost and nothing leaves the computer. **Clear history** still removes all of it.
- **Provenance on every download:** each package of a release carries a signed build attestation, checked with `gh attestation verify <file> --repo rogeriojunior31/Vults` (see *Getting started*).
- **Vults speaks Portuguese, Spanish and Chinese:** the island, Settings, the tray, the notifications, the Activity page and the share image in Brazilian Portuguese, Spanish and Simplified Chinese, besides English. It follows your desktop's language; **Settings → General → Language** picks another.

### Fixes

- The chat's Claude Code and Codex, and the Codex usage read, use the folders your shell sets (`CODEX_HOME`, `CLAUDE_CONFIG_DIR`…) when the app was started from the desktop, not the usual ones.
- Codex's usage meter shows its new count half a minute after a window resets, no longer up to five minutes later.
- The audit log and Zeca's traces hide more keys by their shape: Groq (`gsk_`), xAI (`xai-`), PyPI (`pypi-`), DigitalOcean (`dop_v1_`), SendGrid (`SG.`), Stripe restricted keys (`rk_live_`) and the other Slack tokens (`xoxo-`, `xoxr-`, `xoxs-`).
- **Agent configs written safely in every case:** a config that is a hard link keeps all its names (it was split from the others before), one saved with a BOM or Windows line ends keeps them, and two writes at once never share a temp file.
- **Agent folders moved by their variable:** with `CLAUDE_CONFIG_DIR`, `CODEX_HOME`, `QWEN_HOME` or `GEMINI_CLI_HOME` set, **Settings → Agents** installs the hooks in the file the agent really reads (and Codex's trust in its own `config.toml`), no longer in the usual folder it never looks at. A variable your shell's startup files export is seen too: the app asks your login shell once, when it starts.
- **Codex's trust, read as Codex reads it:** **Settings → Agents** reads Codex's `config.toml` as TOML, so a dotted key, an inline table, a comment after `hooks = false` or a Windows path no longer fool the count, and a hook you turned off in `/hooks` counts as waiting. A `config.toml` that can't be read or parsed shows **Codex's config unreadable**, never "nothing trusted".
- A `"hooks"` value that is not a list of hooks (someone else's) is never replaced: installing is refused and says why.
- **OpenCode 2:** its sessions come back to the island. OpenCode 2 refused the plugin ("Plugin must export a default definition with an id and an effect or setup function"); the new one works with OpenCode 1 and 2, and its permissions get a card on both. OpenCode 2's questions show on the island and are answered in OpenCode. **Settings → Agents** shows **Update available** for the old plugin.
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
