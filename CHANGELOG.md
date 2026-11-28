# Changelog

What changed in each release, newest first. Each release's notes on GitHub come from its section here, so a change a user would notice adds a line under **Unreleased** in the same PR.

## Unreleased

### Fixed

- **Settings**: Dictation's sound and feedback rows sit under Recording as meant; its Windows notes show once instead of in every group; Gitea's empty address no longer reads *undefined*; About's logo hides when its image is missing; Chat & voice no longer names the old Flock & Zeca page.

- **Settings → Chat & voice**: *Who answers Zeca's cards* no longer says destructive commands ask under full access.

- **Zeca's chat**: a folder that no longer exists stops the message with a note, instead of the chat quietly working elsewhere.

- **Zeca's chat, ACP agents**: OpenCode's edit asked with a relative name is no longer refused; Antigravity's commands and file paths show; a crash says why (its last line); a session resumes without replaying (`session/resume`), and an agent that wants its login is told to use it.
- **Linux**: CLIs that only a shell profile puts on the PATH (mise's shims, `~/.local/bin`, `~/.bun/bin`, `~/.cargo/bin`) are found when Vults starts from the desktop.

- **Windows**: OpenCode, Antigravity and the other ACP agents installed as `.exe` or npm `.cmd` are found, so the chat's engine menu offers them.

- **Zeca's chat**: a long task no longer ends after ten minutes with *took too long to answer*; a turn runs as long as it keeps reporting, and only one silent for 15 minutes stops.

- **Zeca's chat**: a saved conversation whose Claude Code session or Codex thread is gone (history cleared, another computer) goes on in a fresh one from what was said, instead of failing with *No conversation found*.

- **Zeca's chat, full access**: it asks nothing any more. Before, a destructive command still asked, and after the first `gh`, `curl` or web read every later command asked for the rest of the conversation. Only a policy of yours that denies still denies.
- **A plan's card**: **Allow and accept edits** now reaches Claude Code; since 0.1.9 the click was dropped on the way and Claude Code asked in its terminal instead.
- **Activity**: a day's bar, the busiest day and the grid count turns side by side once, as the week's total does (they added them up, so parallel agents showed more time per day than in the week).
- **Activity**: a turn still running when Vults quits or updates is kept, ending at its last event, instead of being lost.
- **Microphones**: a 24-bit microphone (and 8, 32 and 64-bit ones) opens for dictation, voice and meetings instead of failing.
- **Windows**: the chat's engine menu finds `claude.exe` and npm's `claude.cmd`, so it no longer says Claude Code or Codex is missing.
- **Windows**: the island hangs below a taskbar or app bar at the top of the screen.
- **Windows**: a VS Code installed under a folder named after Cursor is no longer shown as Cursor.
- **Phone**: the pairing code's text shows only on request, and **Copy** leaves it out of clipboard history and cloud sync.
- **Phone relay**: plain `http://` only for a relay on this computer, a LAN or a tailnet; one address holds at most 64 connections (`VULTS_RELAY_PER_ADDRESS`, 0 behind a proxy).

### Changed

- **Settings**: a long description shows its first sentence, with the rest under **More**; each agent's card shows its state and the step that is due first, with how it works folded and **Remove** last and quiet; History and budget, Approvals and Labs read in titled sections.

- **Settings**, after a design critique of every page: each page reads as title, one sentence, then titled sections (General: Startup, Usage and prices; Island: Behavior, Placement; Sound and notifications: Sounds, Notifications; Flock: Birds); keys drawn as keycaps; disclosures open with a drawn chevron; a switch stays beside its words on narrow windows; Zeca's memory comes before the species gallery, with the second family folded; Agents says when every connected agent is up to date and puts retired agents apart; an ACP engine's unknown sign-in has a neutral dot.

- **Settings → Keyboard shortcuts**: Dictation's keys (its global shortcuts, recording mode and X11 keys) moved there from Dictation, shown once dictation has a voice model; Dictation keeps what is voice.

- **Icons**: every surface draws Lucide's line icons, as t3code does (ISC; see NOTICE); the flock keeps its own drawing.

- **Settings**: **Zeca, chat and agents** groups Zeca, Chat & voice and Agents. Chat & voice reads in sections (agent engines, permissions and setup, API, web, voice); the API providers are a list beside the chosen one's key and model; Agents lists what is connected, then what is here to set up.

- **Settings**: one topic per page, each with titled sections, as t3code arranges them. **Zeca** leads its own group with **Chat & voice**: his portrait, switch, look, species and memory (moved from Chat & voice). **General** keeps startup, usage and updates; **Appearance** (colors and themes, interface), **Island**, **Sound and notifications** and **Keyboard shortcuts** are pages of their own; **Flock** keeps the flock's species pool and rare visitors.

- **Settings → General → Theme**: each theme is a tile with a small picture of Vults in its colors (the island's flock on the wire, the Nest's list, a card), Dark and Light first, then SP Night's three views; arrows move the choice.

- **Zeca's chat**: Qwen Code, Copilot CLI and Kimi Code are offered as engines only once turned on in **Settings → Labs**.

- **Zeca's chat, ACP agents**: an agent follows its own permission settings, as in its terminal: what it asks is a card, and what its settings allow it runs, said in the turn's **Activity**, instead of the turn being stopped.

- **Quick chat**: no name, greeting or explanation; *Ask anything…* in the draft. Once a conversation runs, the portrait gives its room to the messages, and long code, diffs and tool output scroll in their own shorter blocks.

### Added

- **Settings → Chat & voice**: **Agent engines** shows Claude Code, Codex and each installed ACP agent with how it stands, **Sign in** and **Check again**; the API providers are a list with a dot and a line each, instead of a menu.

- **Zeca's chat, where it works**: the folder menu lists recent folders, takes a typed or pasted path, and opens the system's chooser; a folder dropped on a chat that hasn't started becomes where it works; **Chat here** on each project in the Nest's **By project**.

- **Settings → Labs**: beta features, each off until you turn it on there. The first are Qwen Code, Copilot CLI and Kimi Code as chat engines, until each is checked with a live turn.

- **Zeca's chat**: an ACP agent's own models are in the model menu (Antigravity's Gemini, Claude and GPT-OSS; OpenCode's providers), and a model picked mid-conversation applies from the next turn.
- **Zeca's chat**: each message shows when it was said.

- **Zeca's chat**: the agent's plan (Claude Code's todo list, Codex's plan, an ACP agent's plan) shows as a checklist in the turn's **Activity**, its step on the status line; ACP agents' thinking shows too.
- **Zeca's chat**: code blocks are colored by their language.

- **Zeca's chat**: Antigravity runs the chat (`agy acp`, its telemetry off, asking before edits and commands), with **Sign in** for it and for OpenCode; ACP agents' edits show their diffs, and their context fills the meter.

- **Zeca's chat**: an edit in a turn's **Activity** shows its change as colored lines, with the lines added and removed on its row (Claude Code and Codex).
- **Zeca's chat**: the line under the conversation says what the agent is doing now (*Running cargo test…*, *Editing lib.rs…*) and for how long; a finished turn's **Activity** says how long it took.
- **Zeca's chat**: **↑** and **↓** walk through your earlier messages, your last one offers **Edit and send again**, and each of your messages can be copied.
- **Zeca's chat**: a ring beside **Send** shows how full the model's context is (Claude Code and Codex), amber past 80%.
- **Zeca's chat**: a Claude Code or Codex session referenced with **@** brings what was said in it (its words, not its tools; secrets hidden), not only its summary.
- **Zeca's chat**: a turn reads in the order it ran: each stretch of tools sits between the words before and after it, live and when reopened.
- **Nest**: the Chat's side panel has **Memory**, **Changes** (each file the agent changed, with its diffs) and **Files** (find in the conversation's folder); a click puts the path in the draft.
- **Zeca's chat**: **Ctrl+F** finds in the conversation and steps through the matches.
- **Zeca's chat**: replies show `~~strikethrough~~`, task lists and bare addresses; a long conversation no longer redraws every message on each update.

- **Zeca's chat**: each tool in a turn's **Activity** shows the end of what it printed (Claude Code's tool results, Codex's command output), secrets hidden.
- **Zeca's chat**: an agent's question shows its options to click, and typing while a card waits answers it with your words (a no Claude Code reads); full access never answers a question for you.
- **Zeca's chat**: paste an image, or text of 32 KB or more, and it is attached as a file.
- **Zeca's chat**: **/** at the start of the draft lists its commands (`/remember`, `/forget`, `/continue`, `/new`).
- **Zeca's chat**: **@** lists the conversation folder's files and folders, and a folder dropped on the chat goes into the draft by its path instead of being refused.
- **Zeca's chat**: messages written while a reply comes wait in a queue above the draft and go, in order, when the turn ends well.
- **Zeca's chat, easy setup**: the engine menu asks Claude Code and Codex whether they are signed in (a keychain login or an API key counts), **Sign in** opens a terminal with the CLI's own sign-in, and an empty chat says when its engine is not installed or not signed in, with **Sign in** and **Check again**.
- **Zeca's chat**: OpenCode, Qwen Code, Copilot CLI, Kimi Code and every other installed agent that speaks ACP can run the chat, beside Claude Code and Codex.
- **Zeca's chat**: switch between Claude Code, Codex and the API in the middle of a conversation; the other engine takes it over with what was said so far.
- **Zeca's chat**: choose **Your setup** or **Clean** per conversation, from the chat itself; a clean Codex now runs in a home of its own, without your config, MCP servers, skills or `AGENTS.md`.
- **Zeca's chat**: change the model and effort in the middle of a conversation; the next turn uses them and the conversation goes on.
- **Zeca's chat, full access**: the agent runs in its own mode that asks nothing (Claude Code's `bypassPermissions`, Codex's `never`), switched on or off from the next turn without restarting the conversation.
- **A plan's card**: **Change plan…** sends the plan back with your words, and Claude plans again ([Approvals](docs/guide/approvals.md#a-plan)).
- **Antigravity**: when it asks you something (`ask_question`), its bird waits for you with the question's sound and text.
- **Claude Code in VS Code**: a session from the extension, locally or over Remote-SSH, shows as VS Code.
- **Windows**: hook, question and tool pipes open only to you, administrators and SYSTEM; if another program holds the hook's pipe, a notification says agent events can't arrive.
- **Windows**: the Nest's hidden page asks WebView2 to keep its memory low.

## 0.1.9 (2026-11-27)

### Removed

- **Tasks** are gone: the Tasks tab, *Open Tasks*, `/task`, `/delegate`, runs in their own clone, context-pack handoff and the review loop. Vults watches and answers your agents; the code stays their work. `/continue`, costs and the daily budget remain, and past spend from tasks still counts in Activity.

### Windows

- A Windows 11 preview, still being validated: sessions reach the flock from Claude Code, Codex, Antigravity, OpenCode, Qwen Code, Copilot CLI, Cursor, Factory Droid and Kimi Code, with hooks written for the shell each agent uses (cmd or PowerShell, not only Git Bash), and replaced safely while an agent runs them.
- Native session notifications, the animated tray with the Linux menu and presets, the Panel island docked by a taskbar on any edge, surfaces placed again when monitors or the taskbar change, and *Open terminal* bringing the session's window forward.
- The installer installs for you only (no administrator needed) or for every user of the computer (in Program Files, as administrator); `/CURRENTUSER` and `/ALLUSERS` choose it in a silent install.
- Files dropped from Explorer attach on the island again.
- Global shortcuts are configurable and never take a chord another app already holds.
- Local dictation and voice commands: computer audio mute and volume, a player's volume through its own mixer session, opening Start Menu and Microsoft Store apps by name, and media paused during dictation and resumed after. Installers ship and verify Vulkan for GPU transcription.
- `gh`, `git` and agent relays start without a console window; a chat CLI and every diagnostic probe end with their whole process tree.
- Still Linux-only: meeting recording, voice conversation, agent questions over MCP, the specialists' GitHub tools, typing into terminals and most apps, and terminal tabs and panes for *Open terminal*.

### New

- **Allow and accept edits** on a Claude Code plan's card: the plan starts and that session edits files without asking until it ends, as Claude Code's own terminal offers. Its commands still come as cards, and the audit log keeps the choice.
- **Approve Antigravity's shell commands from the island**, in **Settings → Agents → Antigravity** (off by default). A `run_command` that none of Antigravity's own rules covers waits on a card; Allow runs it, Deny stops it, and no answer leaves Antigravity's prompt. Antigravity's hooks now give a card 120 seconds: **Update hooks…** once.
- A Claude Code session's card shows its title beside the project: your `/rename`, else the title Claude Code gave it. Two sessions in one folder are told apart in the card, the Flock search and the chat's session mentions.
- **GitLab and Gitea connectors**, read-only, next to GitHub: GitLab.com merge requests and self-hosted Gitea pull requests with reviewers and changes asked, diff sizes, checks and pipelines of the default branch, and releases. Each has its own switch, budget and trace; the token lives in the keyring.
- Zeca's API Chat and spoken replies can read the cached GitLab and Gitea snapshots, each off by default. A specialist reads only the repositories, host and account its grant names, and reconnecting or changing the account revokes it. No extra request is made.
- **Five themes** in **Settings → General → Theme**: Dark (formerly Black, the default), Light and the SP Night trio, Noite Paulista, Garoa and Pico do Jaragua. Every control follows the theme, selects and focus rings included.
- **A design pass over the island, the Nest and Settings.** An empty Flock says what brings a bird to the wire; Settings › Agents leads with the agents on this computer; removals and full access ask first; one presentation menu in the headers replaces the size switch, full-screen button and sound toggle. In the Nest, waiting permissions form a queue answered from the Flock, each request counts down on the core's deadline, and one that went back to its terminal stays on its card saying so. Approvals splits its audit log into answers, returns and setup changes, by day.

### Fixes

- Allow works on a card for a multi-line command or plan: the click did nothing and the card waited until it went back to the terminal. A card that shows the whole command answers a click; one cut short ("…") still does not, and neither takes an *Always* rule.
- Full access in Zeca's chat answers on Windows: almost every PowerShell command (anything with a variable, parentheses or brackets) read as destructive, so it always asked. PowerShell is now read for what it runs, inside blocks, subexpressions and assignments, and destructive commands, .NET calls that delete or run code, `& $cmd` and overwriting a file still ask. A command with `2>&1;` no longer reads as overwriting a file, on Linux too.
- A plan card shows the plan Claude Code asks to start, not just `ExitPlanMode`: its first line in the target and the whole plan below (up to 2,000 characters, marked when longer).
- A Codex turn reviewed by Codex's own automatic reviewer (`auto_review`, `strict_auto_review`) no longer waits on a card: Codex decides it. Only a turn its rollout proves automatic skips the card.
- Allowing Claude Code's plan from the island now leaves plan mode: the answer carries the plan back, as Claude Code requires, instead of a bare allow it ignored.
- On Windows, a card answered in the agent's terminal goes away: when the agent ends the shell its hook ran under, the hook now ends too instead of waiting out its whole budget.
- A permission card ignores clicks in its first 600 ms: a click meant for the window under it no longer lands on Allow. Its keys still answer at once.
- The plan meters no longer move back when an idle Claude Code session reports its older copy of the limits: for each window, the later reset wins, and for the same reset the higher use.
- Voice transcription no longer crashes with an illegal instruction on older processors: the packages' speech engine is built for a portable CPU baseline (AVX2 and below) instead of the build machine's own, on Linux as on Windows.
- On Windows, agents can ask you a text question (`ask_user`): the Claude Code that Vults starts asks by itself, and Settings > Dictation > Agents sets it up for your own Claude Code or Codex. Type the answer, or hold the dictation key shown on the question and speak, as on Linux.
- On Windows, a specialist with GitHub tools works in the chat instead of refusing every turn.
- On Windows, the secrets Vults keeps in Credential Manager stay on this computer: on a domain account with a roaming profile they no longer follow you to other PCs (a secret saved before keeps roaming until it is saved again).
- On Windows, `/continue` with a Claude Code session says it can't run there (Claude Code has no sandbox on native Windows, and a continued session runs only in one) instead of starting a session whose every command would fail. Codex sessions continue as before.
- On Windows, the corner widget steps behind a full-screen video, game or presentation, as on Linux, and comes back on top when it ends.
- On Windows, Settings names your monitors as they call themselves (`DELL U2720Q`) instead of `\\.\DISPLAY1`.
- On Windows, Zeca in the tray is drawn at the size the tray shows, so it stays sharp at 125 % and 150 % scaling.
- On Windows, a voice command finds an app even when the Start Menu read is cut short by a slow disk or a scan: the names it missed come from the last full read.
- On Windows, an agent's session log made again under the same name is read from the start, so its tokens and cost are not counted from the old file's offset.
- On Windows, media controls and voice media commands work with a player that holds several sessions (two windows, browser tabs): the one playing, or the one Windows shows, is used.
- On Windows, the island's empty transparent window no longer catches clicks at the top of the screen while it starts.
- On Windows, a Gitea token too long for Credential Manager says so instead of "Could not save the Gitea connection."
- On Windows, changing a screen's scale puts the island, the widget and the dictation overlay back in place instead of shifting them off-centre.
- Connecting a headset or unplugging the microphone mid-dictation keeps what you said so far instead of throwing the recording away.
- On Windows, when the privacy settings keep desktop apps off the microphone, dictation says where to turn it back on instead of a bare error.
- On Windows, installing hooks into an agent config you marked read-only works and keeps it read-only, and its backup path reads `C:\…` instead of `\\?\C:\…`.
- On Windows, Settings no longer shows Linux-only rows (X11 recording keys, System Settings shortcuts) while its shortcut editor loads or fails, and **Talk with Zeca** says voice conversation is Linux-only instead of offering its setup.
- On Windows, Qwen Code sends its sessions when your user name has a space, and Codex hooks no longer start a PowerShell on every tool call. **Settings → Agents** offers to update Codex's hooks once (the same diff and click).
- On Windows, the island and the corner widget stay with you on every virtual desktop, and no longer show in Alt+Tab.
- On Windows, folding the island after a chat or an answer gives the keyboard back to the window you were in, so your next keys reach the terminal again.
- On Windows, the dictation overlay moves with the island when you pick another monitor or the screens change.
- On Windows, requests Vults makes for you (connectors, web tools, voice model downloads, the phone's relay) go through the proxy set in Windows Settings, not only one from `HTTPS_PROXY`.
- On Windows, Vults keeps receiving hook events after an update or restart while the old run was still closing; it used to hear none until it was restarted again.
- On Windows, your own Antigravity status line keeps showing behind Vults: it runs in cmd, as it was written.
- When Claude Code can't start a chat turn, Chat shows what it said (a missing Git Bash on Windows, say) instead of asking whether it is logged in.
- Unplugging a microphone as dictation starts no longer crashes the recorder, and Settings still lists the others.
- On Windows, voice commands open Start Menu apps whose shortcut names `%windir%` or `%ProgramFiles%` (PowerShell, Remote Desktop and about half of Windows' own).
- On Windows, dictation types straight into Windows 11 Notepad instead of only copying.
- On Windows, Chat reads a trusted project's Claude Code settings again, and attachments and specialist projects reach the CLI as plain `C:\…` paths.
- On Windows, one folder is one project whatever its spelling (`c:` or `C:`, either slash, a trailing one): it no longer splits into two flocks, an **Always** rule answers it again, `/continue` finds its session, and a file a command wrote no longer shows twice.
- On Windows, the log is in `%LOCALAPPDATA%\Vults\logs`, beside the rest of the app's data, and Settings and files lists where everything lives on Windows.
- On Windows, a Codex specialist finds your Codex login when Vults starts from the Start menu.
- On Windows, a diff card is titled by the file name instead of its whole path, and the Nest's week bars add a repository's worktrees up again.
- On Windows, Chat, Zeca's sleep and the journal work with a `claude` or `codex` installed through npm: their instructions now go in a file (or on stdin) instead of the command line, where a `.cmd` takes no line break and a long memory outgrew the limit.
- On Windows, destructive commands always ask again: Claude Code's PowerShell tool, `powershell -Command` and `cmd /c` were not read, so `Remove-Item -Recurse` could pass under Zeca's **Full access**. PowerShell syntax Vults can't follow now reads as destructive.
- On Windows, your policies now hold: their folders (`C:\…`, any case or slash) were never matched, so a deny or ask did nothing and Zeca's chat with **Full access** went past them. A policy for a project now also denies or asks when a session's folder can't be read.
- On Windows, Chat no longer says Claude is not installed when its CLI is only on a shell profile's PATH (mise, for one): Vults also looks in the usual per-user CLI folders.
- The plan usage meters no longer vanish and come back: a read that carries only one window (Claude's statusLine, say) keeps the other until its reset.
- Two instances starting together no longer fail when one is still migrating the database: start-up retries for a bounded time.

### Also in this release

- **Settings → Connection setup → GitHub → Check access** tells why GitHub access fails: `gh` missing, signed out, an expired or revoked login, the rate limit, single sign-on, a forbidden read or a repository this account cannot see, and for a repository whether you can push. It works with the connector off and never shows `gh`'s output or a token.

- Vults can tell you when a new version is out. The island asks once whether it may look at its releases on GitHub; with your yes it looks at most once a day and shows the version with **Open release page** and **Later**. **Settings → General → Updates** has the switch and **Check now**. Nothing is downloaded or installed, and AUR packages are built without it.

- A session that finishes or fails while you are looking at its terminal no longer plays a sound or brings the island forward; cards always do. **Settings → General → Stay quiet when I am looking**, on by default, turns it off.

- An uncertain or interrupted GitHub merge, Actions rerun or cancel, or branch deletion can be proposed again 15 minutes later, when a fresh read shows it did not take effect; before, it stayed blocked for 90 days.

- Clear history keeps the receipt of a GitHub merge, Actions rerun or cancel, or branch deletion that GitHub may have carried out, so the same write can no longer be sent a second time after clearing.

- Resize constrained side panels from their visible width and report the available accessible range without overwriting the saved width on window shrink.

- Use the shared keyboard-focus color for side-panel resize and fold controls across all five themes.

- Keep side-panel resizing with its initiating pointer, ignore additional touches and end interrupted drags cleanly when the window loses focus.

- Return keyboard focus to the retained Chat draft after recipient recovery, keep Retry focused on failure, and respect focus moved while waiting.

- Preserve keyboard focus on Recent conversation pagination and filter clearing during list refresh, with search as the fallback when a control disappears.

- Improve compact Chat navigation, accessible turn announcements, Recent keyboard focus, recipient and Memory recovery, scoped Flock permission disclosure and folder-copy feedback.

- Keep idle and completed-response sessions in Flock instead of expiring them after 10/30 minutes. Confirm local owner exits separately from activity, preserve pending cards, and distinguish Hermes Desktop runtime cleanup/reset from durable conversation closure.


- Flock metrics now distinguish context from cumulative consumption, model-response from execution time, native from catalog cost, and independent reading ages. Added Qwen usage-ledger, Copilot shutdown, Kimi wire, Droid summary, Cursor preCompact and Antigravity CLI status-line collection, with explicit source limits and expiring estimates.


- Add Hermes Agent session/tool/subagent observation, patch and repository diffs, numeric context/API-response timing, and read-only native session costs. Permissions remain in Hermes; plugin installation previews and checks all owned files.

- Add shared command setup checks, searchable examples and a non-executing phrase check in Dictation. Preserve drafts, support cancellation/retry, reject invalid input and discard stale results without running actions or sending to Chat. Recognition does not certify native availability.

- Preserve repeated words and leading articles in explicit application, project and notification targets. Keep dictation text processing separate from command dispatch.

- Add opt-in local voice commands for selected-player playback, default computer audio and installed applications, including ordered batches with flock navigation and terminal/folder submission receipts. Keep ambiguous, quoted, negated and unsupported player-specific requests local, revalidate targets and cancellation, and report only confirmed completion. Native desktop acceptance remains pending before 0.1.9 publication.

- Include LICENSE and NOTICE in the Linux Debian/RPM package resources alongside the bundled hook.

- Request clipboard-history exclusion for copied dictation on Linux using the existing KDE hint, while keeping plain-text paste and read-back confirmation. Clipboard managers must support the hint; Vults history retention is unchanged.

- Refresh Scene and Sky canvas resolution and sprite caches when the display scale changes, including successive and fractional monitor transitions. Keep CSS layout and zoom unchanged and avoid starting hidden animation.

- Check Zeca as the recipient before preparing questions from Flock, Connections or the command palette. Preserve drafts and running replies on rejection, and cancel preparation after a conversation switch without sending or resetting.

- Bind local memory/continuation commands and chat transcription to Zeca and their originating conversation. Keep rejected or newer drafts editable and cancel obsolete recordings on conversation changes.

- Identify Zeca separately from the chat executor and bind sends and draft saves to their conversation. Cancel queued sends on a switch, reject obsolete window requests and preserve unsent drafts when Zeca is turned off.

- Add species and fixed-accessory editing to custom agent profiles, with full-sprite previews and appearance saved in each revision. Existing profiles keep their species and start without an accessory.

- Add local custom copies of agent profiles with editable names/instructions, retained revisions, stale-edit protection and recoverable drafts. Built-in profiles remain immutable; saving does not start an agent or change access.

- Add a read-only Agent profiles catalog in Flock with built-in roles, instructions, explicit revisions, native read guards and retry. The catalog works with Zeca off and does not change permissions or start agents.

- Add a saved shared Mini/Quick width choice (560/680/760 logical pixels), a slim 38 px Mini and a 600 px selected Quick cap and screen-aware bounds. General and Quick share the same width preference. Default restores the earlier sizes; Room and Full keep their own geometry. Mini reserves a keyboard-accessible list icon for sessions that do not fit, opening the existing Nest Flock page.

- Add Dictation > Talk with Zeca settings for explicit realtime voice consent, destination, model and exact rolling USD budget, with retained drafts, save readback and reload on conflicts.
- Add retained voice conversations with explicit recording/continuation/Stop controls, opt-in local silence completion, confirmed continuous microphone routes and default-off eight-second answers after fully played questions. Reuse Zeca's shared character for voice feedback and keep semantic turn detection behind an explicit developer preview.

- Add Linux `ask_user` text questions for verified Claude Code processes started by Vults, in Quick and Nest. Keep drafts editable, require a separate Send/Enter, audit confirmation before delivery and never resend a consumed reply after failure. Cancel on departure/lock/expiry/disconnect and give permission cards priority.
- Add local dictation into verified question drafts using the independent dictation key, with separate human confirmation. Offer external-agent question setup only through reviewed, default-off installation with backups and preserved third-party settings. Permission cards never take voice.
- Add the gated native realtime voice backend with durable cost reservations, bounded playback, heard-prefix interruption accounting, local command routing and read-only flock tools. Add Nest-owned manual voice turns with explicit Start/Finish/Cancel, local transcription, bounded page contact and editable recovery copied only on request. Physical/live voice qualification remains open; Codex voice consultation stays unavailable pending subscription-authentication qualification.
- Add separately enabled, tool-free meeting summaries with provider/model disclosure, durable rolling attempt limits, cancellation and original-transcript fallback. Show Transcript/Summary/To-dos and keep generated notes inert.
- Add explicit text-only meeting saves and a searchable Saved meetings workspace with Transcript/Summary/To-dos, source-local voice names, confirmed deletion and native Markdown/SRT/VTT export. Freeze the first saved snapshot, keep audio temporary and include saved meetings in global history erasure. Subtitle export requires one source clock; exported files remain independent copies.
- Keep automatic meeting speaker identification off by default behind an explicit development preview while independent speech-quality qualification remains pending. Add checked optional model installation and offline evaluation tooling.
- Keep Ctrl+Alt+V dedicated to independent dictation and show its passive status capsule at the bottom of the selected screen, with inline fallback when unavailable.

- Add Linux Dictation > Meetings with explicit source/model/language review and consent for recording up to four hours, followed by local transcription in bounded chunks. Stream both temporary tracks to mono 16 kHz PCM16 (less than 1 GiB at the cap), keep hour-aware clocks and preserve cleanup/cancellation. Show local microphone/system transcripts with estimated segment times, source selection, progress, cancellation and explicit discard. Keep unsaved text in the current page only, without clipboard insertion or dictation History. Summaries require their own explicit consent and generation action.

- Bound meeting audio transport waits, retain private WAVs under one cleanup lock through local transcription, recover interrupted audio at startup and publish results only after cleanup. Cancel on page disconnect/navigation, window close/minimize, screen lock or voice preference change. Decode bounded WAV chunks off the dictation path and give interactive Whisper work priority over meeting processing.

- Restore the last Dictation configuration section when returning through its tab, link Voice commands directly to Command settings, localize voice-model descriptions and put History search before compact, foldable daily figures. Reconcile the Styles guide with the running feature and add a real-provider/speech validation checklist.

- Consolidate dictation configuration under the Dictation tab with separate Voice, Keyboard shortcuts, Command settings, Vocabulary, Styles, Paste, Privacy and Diagnostics pages. Keep one shared navigation and a narrow page picker; move shared transcription controls there and link from Chat. Configuration retains its privileged webview and existing consent checks; transcript work pages stay in Nest.

- Resolve optional dictation style and output language at key-down: local text for terminals/editors, Polished for mail, Natural for chats and Prompt for recognized flock sessions, with durable per-app overrides. Add separate output-language and session-English preferences, preserve heard language in History, and retain local text when focus/preferences are unavailable. Add Settings-owned consent with provider/data/quota disclosure, style/language/session-English controls and per-app overrides. The Dictation tab opens the single Styles editor, and History labels known applied styles and output languages. Pending/failure/retry and keyboard focus survive redraws; opening Styles makes no model request.

- Add the pure dictation clean-up prompt/reply contract for Natural, Polished, Prompt and explicit translation. Protect local edits with opaque marks, escape metadata, retain question/imperative intent in examples and reject malformed or implausible replies. Add authored response cassettes and reviewed message snapshots; visible Styles controls are delivered with their separate consent. Add an independent default-off cleanup switch and tool-free API/local provider calls, durable rolling quota, content-free traces, cancellation and bounded full-local-text fallback.

- Add local voice commands on the separate Ctrl+Alt+Z shortcut, including with Zeca off: navigation, project preferences and cached flock reads, with saved aliases, bounded Unicode matching, atomic sequences and 90-second clarification/repeat/undo. Show capture/results in the existing island/Nest feedback and retain command history without replay or dictation statistics. Never paste command text or dispatch card-answer/management or merge intents. Add the Voice commands work page, shared phrase guide and Settings controls. Keep unknown speech local by default; optional handoff uses the current idle Chat with no unsent draft, through its existing model, approval and network owners. Settings changes cancel old command tickets without clearing ordinary Copy again.
- Fix Windows CI compilation by keeping the Nest ownership mock test on targets with Tauri's test feature and giving the non-Linux diagnostic status an explicit string type.

- Reconcile the current dictation statistics and Diagnostics guides, Settings reference and Nest navigation in English and Brazilian Portuguese. Add a local validation checklist, explain the separate lifetimes of usage/timings/session counters, and retain real-desktop and Small/Vulkan latency acceptance as pending.

- Add Diagnostics in Dictation settings: an explicit six-second local microphone test with speech bands, estimated dBFS noise floor and input verdict; current audio/desktop/GPU/VAD state; memory-only loop/retry/Copied counters; and retained stage p50/p95. Cancel tests on departure, lock, voice start or lost page contact, without transcription, history, clipboard or permission requests.

- Add Activity > Dictation statistics with 7-day, 30-day and lifetime usage, estimated time saved, speed, daily bars and top apps. Keep usage after transcript erasure, label legacy app attribution gaps, retain period filters and include dictated words in Week even without agent turns.

- Record local dictation stage timings and per-stage p50/p95 over the newest 50 retained deliveries. Keep legacy/unused measurements unknown, exclude cancelled/failed/empty work and replay, and erase timings with transcript retention. Activity statistics and Settings diagnostics present these measurements.

- Add Dictionary and Snippets work pages for saved dictation words, ordered replacements and literal snippets, with explicit deletion, retry and retained drafts. Try a sentence uses the real local pipeline with highlighted changes, cancellation and optional terminal context, without audio, models, clipboard or history. Add the two opt-in vocabulary controls in Dictation settings, retaining the measured Turbo/English restriction.

- Add optional local project vocabulary for dictation: cache package/dependency/module/file names and branch per known Git working tree, refresh on session start, and select from captured session context or an unambiguous editor/terminal title. Freeze context across preview/final, prioritize personal dictionary words under the existing 224-token cap and model/language gate, and leave the preference off by default. Configure it in Settings > Dictation > Your words.

- Add optional per-recording dictionary context for Whisper preview/final dictation, off by default and capped at 224 tokens. Qualify only Turbo with fixed English from offline synthetic WER and silence/noise comparisons; keep other pairs on their existing prompts, preserve Chat and recheck Base retries. Configure vocabulary in Dictation settings; human-speech validation remains pending.

- Apply saved Your words rules to independent final dictation: protected literal snippets, ordered replacements and eligible dictionary spelling before terminal-context spoken code. Keep per-recording snapshots, original/written history and replay behavior; preserve control-character barriers and reject oversized processing before delivery. Dictionary and Snippets work pages own editing.

- Store dictation dictionary words, ordered replacements and literal snippets locally behind Nest-only commands. Reject case-insensitive duplicate words/triggers, update replacement sources without reordering, and preserve preferences when clearing history.

- Reconcile the dictation guide, settings reference and architecture with the working N3 build. Document supported automatic-insertion destinations and manual-paste fallbacks, current controls and their privacy boundaries in English and Brazilian Portuguese. Keep later prototype features distinct from available behavior.

- Add Music while you speak in Dictation settings: lower playing media to at most 25%, pause it or leave it alone. Restore each untouched player's own state after capture/cancellation; preserve user changes and skip crashed/restarted players. Local MPRIS controls stay separate from Now playing and never delay microphone capture.

- Add optional cancel-by-voice for independent dictation, on by default on X11 and Wayland. Whole or trailing English/Portuguese/Spanish cancel phrases discard the final transcript before delivery, report Cancelled and save no history or totals. Active recordings keep their choice; replay and Chat bypass spoken cancellation.

- Add spoken-code punctuation for captured terminals and sessions, with a saved switch in Dictation settings. Join English, Portuguese and Spanish punctuation terms inside identifiers while preserving prose/unknown targets, case and control bytes. Retained history keeps heard and written text; replay never reformats or submits it.

- Add saved dictation processing choices for repetition removal and the one-time Base fallback. Preview and final text use each recording's choices; defaults stay on, silence cleanup remains active, and disabling fallback skips its extra speech check. Chat keeps its existing processing.

- Add a saved microphone choice for independent dictation, with explicit device refresh and loading/error/empty feedback. Disconnected preferences stay selected, capture keeps its existing missing-input fallback, and Chat keeps using its default microphone.

- Add saved dictation gesture choices: hold with optional double tap, Space-lock and Esc cancellation. Defaults stay enabled, active turns keep their original choices, and the global Cancel shortcut always works. Space/Esc controls explain their X11 requirement and are unavailable on Wayland.

- Add **Change in KDE…** to Dictation settings, opening the installed desktop shortcut editor with failure/retry feedback. Current key labels keep following desktop binding events.

- Move current dictation controls into Settings > Dictation, linked from History, General and Chat & voice. Keep shared model/language setup in Chat & voice and add confirmed history erasure with failure/retry feedback. Existing choices and permission ownership are preserved.

- Keep inactive Nest tabs hidden even when a later pane style uses flex or grid, preventing Dictation from appearing below Chat.

- Put the Nest and Compact Chat drafts in a single rounded row, with attachment on the left and microphone/Send on the right, preserving growing drafts and the separate file strip.

- Add the Nest Dictation history page with local search, measured daily figures, heard/written text, copy/reuse and confirmed deletion. Retention is visible in General until the dedicated settings page arrives; active dictations now update history without storing audio or previous clipboard contents.

- Prepare the local dictation history store with retention, independent daily/app totals and transcript erasure. Live recordings still keep their last text only in memory until History and privacy controls are connected.

- Add an optional dictation start cue that respects sound volume, mute and Do Not Disturb. It plays once per new recording and stays silent on surface changes, snapshots, failures and cancellations.

- Show localized Unicode word totals after confirmed dictation delivery. Caption expiry preserves the total; active recordings, failures and cancellations never reuse a previous count. Correction totals await their later processing features.

- Add optional two-row dictation captions for clipped lines and confirmed delivery text. They yield to cards and limited space, take no clicks, and expire at the original two-second deadline across surface changes. General's dictation controls hold the switch until the dedicated settings page arrives.

- Show real microphone bands and the shared local GPU preview during global dictation. Cancellation and capture end discard the preview; reduced motion keeps the text without moving bands.

- Give Mini and Quick a reserved neutral dictation row with phase icons. Waiting cards and screen lock hide feedback immediately; the row follows zoom and either dock anchor within the fixed overlay.

- Open terminal targets the WezTerm instance recorded by the hook, without loading Lua configuration, starting a server or falling back to a different instance. Direct WezTerm dictation stays copy-only because its input overlays do not have a distinct pane identity.

- Add exact tmux dictation delivery for flock sessions in XTerm/UXTerm or Kitty, with live permission/question and deferred-card guards, literal single-line input without Enter, and distinct In session or Copied/waiting feedback. Other terminal transports remain copy-only.

- Add opt-in dictation paste on Plasma Wayland through a keyboard-only desktop portal grant. Settings shows permission state and explicit retries; restore tokens stay only in the OS keyring. Focus checkpoints and a native text lease protect the destination and newer copies. Terminal paste chords remain disabled; exact tmux delivery has its separate checks.

- Add off-by-default dictation paste in supported native X11 text editors, with focus checks, honest Pasted feedback and conditional clipboard-text restoration that preserves newer copies. Settings explains the clipboard limits. Other apps stay copy-only; terminal delivery uses the separate exact tmux path.

- On X11, Space can lock a held dictation and Esc cancels recording or transcription. Other non-modifier keys discard the initial hold/tap; actual shortcut chords and repeats are ignored, and the listener closes when the turn ends. Wayland keeps the existing double-tap and separate cancel shortcut.

- Fix global dictation showing Copied while Wayland kept the previous clipboard. Background copying now uses native clipboard support and confirms the text by reading it back; unavailable access is reported as an error.

- Global dictation observes the foreground app on X11 and Plasma Wayland and names it in copied feedback, with a terminal paste hint. Unsupported or late observations stay unknown; this context does not authorize insertion.

- Local global dictation copies text without opening Chat, including with Zeca off. Hold or double-tap to record, cancel with its own shortcut, and copy the last result again; desktop bindings are shown in Settings and follow remapping. Additional session transports remain later N3 steps.

- Chat voice preview and final transcripts reduce long repetition loops and omit known standalone silence artifacts. Technical text in parentheses and brackets is preserved.

- Chat voice filters brief noise and keeps short pauses within a phrase while preserving its 1.5-second end-of-speech pause.

- Chat voice uses improved audio conversion while keeping its one-minute recording limit; capture failures are reported instead of silently returning incomplete audio.

- Reopening the Nest from the tray keeps the last tab and Activity subsection during the app session; explicit routes still take priority, and Zeca off returns Chat to Flock.

- Reopening a minimized Nest restores its window. Minimization returns approval shortcuts to the island and does not overwrite the saved Room size.

- The island's clickable region follows viewport and scale changes, including the bottom panel anchor, and its first measurement is replayed when the native bridge connects.

- Connections keeps its repository filter, subsection and selected item when the Nest is closed and reopened during the same app session. Pending merge confirmations are discarded.

- Unsent Chat text, attachments and folder stay together when switching between the island and Nest; returning from Nest's Chat reopens Chat in Quick. Drafts are held only until the app exits.
- Switching between Room and Full keeps the selected Activity subsection, including History, Cost and Audit.

## 0.1.8 (2026-10-12)

### New

- **Kimi Code takes tasks:** **Start with another agent…** in the Tasks tab starts Kimi Code over the Agent Client Protocol when it is installed: each write and command asks with an island card, even when your Kimi config would run everything unasked.
- **Kimi Code:** **Settings → Agents → Kimi Code** adds its hooks to `~/.kimi-code/config.toml`, your comments and layout kept, and its sessions join the flock, each step, an edit's lines and a failed command shown; Kimi asks its permissions in its own prompt.
- **Factory Droid:** **Settings → Agents → Factory Droid** adds its hooks to `~/.factory/settings.json`, and its sessions join the flock, each step and an edit's lines shown; Droid asks its permissions in its own prompt.
- **Cursor:** **Settings → Agents → Cursor** adds its hooks to `~/.cursor/hooks.json` (other tools' hooks there are kept), and its sessions join the flock, each step and failure shown; Cursor asks its permissions in its own prompt. A Cursor session no longer shows a second time as Claude Code.
- **Copilot CLI:** **Settings → Agents → Copilot CLI** writes its hooks file, `~/.copilot/hooks/vults.json`, and its sessions join the flock with a card for each permission, an edit's lines counted; with no click, Copilot asks in its own prompt.
- **A larger island:** the open island's words and buttons are about 14% larger (about 500 px wide on screen); the birds stay crisp at their pixel size, only spaced wider.
- **Who answers Zeca's cards is your choice** (ADR 0002): in **Settings → Chat & voice**, **Asks first** (as before), **Your policies** (your accepted policies answer the chat's calls too) or **Full access** (everything runs with no card, but destructive commands and what a policy denies or asks first still ask, and cards come back once a conversation reads from outside). The chat says each answer given this way and the audit log keeps it; a chip beside the model shows the choice and can give access back.
- **A better chat, on the island and in the Nest:** **Copy** takes a whole reply; a failed turn offers **Try again**; **↑** in an empty composer brings back your last message; *Thinking…* counts the seconds of a long turn; an error reads as one, and what happened along the way as a quiet pill; a card's **Allow** sits beside **Deny**, with *Always in this chat* quieter under them; in the Nest the conversation reads in one column as wide as the composer, a little roomier, each reply signed with Zeca and its engine, code blocks with their language, and the composer's keys said under it.
- **Activity by project:** Quick's Activity shows the week's time per project, the most first, as the prototype draws it (overlapping turns in a project count once).
- **The island chat's engine pill:** one button, *● Claude Code · Default model ▾*, as the prototype has it; its menu holds the engine, the model and the effort (only the engine once a conversation started).
- **Quick's gear follows outside changes:** Presence and the fold changed from the tray or Settings show at once in the gear's view; the chat's folder menu scrolls.
- **The chat's paperclip picks files:** it opens the system's file chooser; the files go the way of a drop, Zeca swallowing them beside the zone while a vulture carries them in, and join your next message at once (no *Ask about it* step; **×** on the chip takes one off).
- **Room resizes:** the Nest's window resizes from its edges and corners (it has no system frame, so the page asks the compositor), opens large the first time, and comes back at the size you left it.
- **Quick's tabs, as the prototype draws them:** Flock, Chat, Activity, Connections and the gear. The chat in a compact form (the engine and conversations up top, a paperclip and the microphone in the composer, the folder and model under it); Activity's four figures and a bar a day; one Connections tab with a chip per connector and a card per row; the gear sets **Presence** and **Fold the island** in place and opens any other page in the Nest. The chips of who needs you stay under the tabs. **Drop a file** is the chat's paperclip now, and Tasks live in the Nest.
- **Quick opens on the flock:** at rest the island shows every session on the wire, its name under its bird, and a chip for each one that needs a look; click a vult or a chip to open its session, as the prototype draws it: **‹ Flock** to go back, a dot per session, the project, agent and state, and its answers across the card. A permission card still opens by itself on its session.
- **The Nest wears the island's frame:** no system title bar, the prototype's rounded frame; its header moves the window. Fixed: the Nest and Settings no longer show stacked in the window.
- **One surface at a time:** while the Nest is open (Room or Full) the island steps aside, pill and birds and all, and comes back when the Nest closes or goes back to Quick; a card brings it up unless you are in the Nest, which answers it there. The chat moves to the Nest with Room or Full; island chat words follow the app's language. **Back to the island** goes to Quick.
- **The Nest's new frame:** the Living Flock prototype's header (the flock's count, sound, search, the size switch, full screen), tabs with icons and counts with **Settings** at the right, and a footer with Zeca, **Zeca's look**, **Open terminal** and **Back to the island**. Settings shows the same frame.
- **The Nest's Flock, as the prototype draws it:** the flock across the window over a card per session; the session in front with its card (what a waiting one asks, answered right there) and its **Changes** (each edit's diff), **Activity**, **Context** and **Zeca**; in Full, your plans' usage.
- **The Nest's Chat, as the prototype draws it:** a title bar with the conversation's title, the engine, folder and model, and the panels for **Recent** and **Memory**; a new conversation waits with the composer in the middle.
- **The Nest's Activity, as the prototype draws it:** the week's four figures in one strip, a bar a day with a day to inspect, a small recap to save, and History, Cost, Traces and Audit as the prototype's lists.
- **The Nest's Connections, as the prototype draws it:** a chip per connector, when it last synced and **↻ Refresh**, the filters as tabs, rows by group with their state, and the row picked in detail with **Open in GitHub ↗**.
- **Settings, as the prototype draws it:** the sidebar in three groups (Experience, Work, System) with icons, a kicker over each page's title, rows in quiet cards, the prototype's switches and buttons.
- **Quick, as the prototype draws it:** the island's header with its mark, sound, search and the size switch, the tabs as icons with counts and Settings at the end, and the flock twice its size with each session's name under its bird.
- **Tasks and the pill, to match:** the Tasks tab in the same cards and buttons; Mini spreads the flock over its wire, with a hairline per plan for its usage.
- **Zeca's memory:** **Settings → Chat & voice → Memory**, off until you turn it on. A journal of each agent turn, each card's answer and the chat's lines, secrets redacted, kept 90 days on this computer; only what joins a conversation reaches its model.
- **He remembers in every conversation:** what he knows about you, what he learned and the project's facts join the conversation when it starts, fixed for its whole length; each message also carries the date and your sessions, and up to three passages from earlier days that answer it.
- **Only you write it:** `/remember <fact>` and `/forget <words>` in the chat; what Zeca proposes (his tool, or what he read in a finished conversation with **Suggest what to remember** on) waits in the **Memory** beside the Nest's Chat, with its place and a diff, until you accept it. Every change can be undone. A fact said again in other words is never kept twice.
- **Roosts:** the Nest's **Activity → By project** groups your sessions by repository, with each worktree's branch, your pull requests and checks from the GitHub card, and what Zeca remembers of the project.
- **One project memory for Claude and Codex:** a roost whose `AGENTS.md` its `CLAUDE.md` does not import offers to add `@AGENTS.md`, with the diff first, a click, and a dated backup outside the repository.
- **Daily summary, sleep and recall by meaning** (each off until you turn it on): a past day summed up by Claude Code's smallest model; a daily sleep that proposes merges and leaves a debrief; and, with an Ollama on this computer, recall that finds summaries by meaning. Each is a request on your behalf, with its own budget and local trace.
- **Zeca's tools in Claude Code:** the chat's Claude Code gets his own MCP server (`flock_list`, `recall`, `journal_day`, `roost_memory`, `memory_propose`) through `--mcp-config`, never a change to your Claude config; each call your rules don't allow asks you with a card.
- **Delegate:** `/delegate <goal>` makes a task and starts it on the agent the router picks: the project's rule, the other agent when one's plan is above 85%, and, after 10 finished tasks, the one that finishes that kind of task best there. The card says why.
- **The palette:** **Ctrl+K** in the open island or the Nest jumps to a session, starts `/task`, `/delegate`, `/continue` or `/remember`, or opens a tab. It never answers a card.
- **Settings, regrouped:** **Chat & voice** and **Flock & Zeca** take the old Chat and Flock pages' places, and General lists every shortcut, the sizes and the Nest's tabs included. Settings now opens in the Nest's window, behind the gear and under the same tabs, as a page of its own: the only one that may change an agent's config, a key, a policy, a paired phone or a request made on your behalf, and one that never shows an agent's text.
- **Connections, filtered:** the Nest's GitHub card shows **All**, **Waiting for your review** or **Checks failed**, each with its count. **Add a connection** lists the connectors Vults has that are off, and **By project** opens a roost's pull requests and its default branch's checks on GitHub.
- **The Nest's Tasks by roost:** each roost's tasks under its name, and the review loop's rounds as marks on the card.
- **A card's one line:** a permission card says in one line when no policy can allow it (a destructive action) or when your policy asks first.
- **Mini, the pill without text:** only the flock, up to eight vults along the wire; its edge pulses amber while someone needs you. A vult looks at you under the pointer (and comes down if it was flying), a click opens the island on its session, a right-click opens its quick actions.
- **Four sizes:** a switch at the top of the island and the Nest, **Mini** (the pill), **Quick** (the island), **Room** (the Nest in a window) and **Full** (the Nest on the whole screen, **Esc** goes back), keeps what you were looking at; **Ctrl+Alt+1**, **2** and **3** switch to Mini, Quick and Room from any window, and **Alt+1** to **5** pick the Nest's tabs. It takes the place of the island's fold button.
- **The Nest, renewed:** five tabs, **Flock**, **Tasks**, **Chat**, **Activity** and **Connections**, and the gear for Settings. It opens from the tray (**Nest…**), and with Zeca off too, without its Chat. **Flock** lists every session with its repository and branch, and a session that waits shows its card there; **Connections** shows the GitHub card. **Activity** holds the week's recap and **By project**, plus four records that only read what this computer keeps: **History** (your agents' finished turns), **Cost** (your plans' windows, the last two weeks' costs against your budget, and each roost's), **Traces** (Zeca's own turns and tools) and **Audit** (who answered each card, and how).
- **More agents take tasks:** **Start with another agent…** in the Tasks tab starts OpenCode, Qwen Code or Copilot CLI (the ones installed) on a task, through the Agent Client Protocol: each write, command and fetch asks with an island card, answered for that once only, and nothing reaches out of the task's clone.
- **Costs and a daily budget:** each task shows what its runs cost and their tokens, each roost in the Nest's **Activity → By project** its cost today and in all, and the **Tasks** tab today's, from what the agents report (at API prices on a subscription). Set a daily budget in the Nest's **Activity → Cost**: a notification at 80% and when it is spent, and then Vults starts no new session until tomorrow. It never stops or blocks an agent.
- **Continue a session:** `/continue <words>` in the chat gives your words to a Claude Code or Codex session in that folder that stopped in the last ten minutes, resumed here; one still open in its terminal is refused.
- **Cross review:** **Review with Codex** (or with Claude Code) has the other agent review a task's work against its criteria; what it finds goes back to the task's session to fix, up to three rounds.
- **Codex takes tasks too:** **Start with Codex** runs a task in its own clone and sandbox, its approvals as island cards; **Hand off to Codex** (or to Claude Code) passes a task's context pack to the other agent.
- **Start fresh with a handoff:** a task whose session has grown long can **Start fresh**: the old session writes a context pack (objective, constraints, done, in progress, decisions, files, failures, next), Vults adds what it knows for certain, and a new session starts from it alone.
- **Start a task:** **Start with Claude Code** runs Claude Code on a task in a clone and branch of its own, in Claude Code's sandbox, still asking before each command; nothing it does can touch your repository, and only its branch comes back. It shows in the flock, its cards come to the island, and **Stop** ends it. Two tasks run side by side.
- **Tasks:** `/task` in the chat keeps a task in one message: its goal, then a line for each criterion. The Nest's new **Tasks** tab lists them with their roost, who works on them, where they stand and what they cost; with the GitHub connector on, **Open an issue** makes one from a task through your `gh`.
- **Policies:** **Settings → Approvals → Policies** lets you write what to allow, deny or always ask, in a project folder or everywhere, see it as a diff and accept it with a click; it then answers those cards for you (allows for Claude Code's calls; denies and asks for every agent). A destructive action always asks, a deny beats an allow (and an older **Always allow**), an allow reaches only the folder the session started in and needs Vults to know what the action does, and every answer goes to the audit log with the policy that gave it. **Revoke** turns it off at once.
- **What each agent can do:** **Settings → Agents** shows, for each agent, whether its CLI is installed and its version, whether its hooks are in (and, for Codex, trusted), and what it can do with Vults: cards, questions, diffs, how a turn could be stopped, its sandbox, and whether Vults can start it.

### Changed

- **Gemini CLI is no longer supported:** Antigravity took its place. Hooks an earlier Vults installed in Gemini's settings send nothing now; **Settings → Agents → Gemini CLI** removes them (with the diff, a dated backup and a click). Gemini CLI no longer takes tasks either; the Google Gemini chat provider stays.

### Fixes

- **OpenCode's edit and write cards** show what **Allow** would change, with the lines it adds and removes, and a finished edit shows its diff: the plugin passed on only the file's name.
- **A Claude Code chat's cost** counted once per turn: from its second turn on, the trace of a chat through Claude Code carried the whole conversation's cost again.
- Going back to 0.1.7 now keeps a copy of your settings: the file is at version 22.
- With Zeca's memory off, nothing of it reaches a model or a window.
- **The chat no longer doubles its words:** each piece of a reply reached the island and the Nest twice; questions and cards in the chat are shown once.

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
